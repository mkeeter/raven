use uxn::Uxn;
use varvara::{Key, MouseState, Varvara, AUDIO_CHANNELS, AUDIO_SAMPLE_RATE};

use std::sync::{mpsc, Arc, Mutex};

use anyhow::Result;
use cpal::traits::StreamTrait;
use eframe::egui;
use log::{error, info};

/// Injected events from the [`Stage::rx`] queue
#[derive(Debug)]
pub enum Event {
    LoadRom(Vec<u8>),
    SetMuted(bool),
    Console(u8),
}

pub struct Stage<'a> {
    vm: Uxn<'a>,
    dev: Varvara,

    /// Scale factor to adjust window size
    scale: f32,

    /// Current window size
    ///
    /// When the ROM writes to `Screen/width` or `Screen/height`, the window is
    /// resized and this value is updated accordingly.
    size: (u16, u16),

    /// Time (in seconds) at which we should draw the next frame
    next_frame: f64,

    scroll: (f32, f32),
    cursor_pos: Option<(f32, f32)>,

    texture: egui::TextureHandle,

    /// Event injector
    event_rx: mpsc::Receiver<Event>,

    /// Callback when the size is changed by the ROM
    resized: Option<Box<dyn FnMut(u16, u16)>>,
}

impl<'a> Stage<'a> {
    pub fn new(
        vm: Uxn<'a>,
        dev: Varvara,
        size: (u16, u16),
        scale: f32,
        event_rx: mpsc::Receiver<Event>,
        ctx: &egui::Context,
    ) -> Self {
        let image = egui::ColorImage::new(
            [usize::from(size.0), usize::from(size.1)],
            egui::Color32::BLACK,
        );

        let texture =
            ctx.load_texture("frame", image, egui::TextureOptions::NEAREST);

        Stage {
            vm,
            dev,

            scale,
            size,
            next_frame: 0.0,

            event_rx,
            resized: None,

            scroll: (0.0, 0.0),
            cursor_pos: None,

            texture,
        }
    }

    /// Sets a callback that is triggered when the screen is resized
    pub fn set_resize_callback(&mut self, f: Box<dyn FnMut(u16, u16)>) {
        self.resized = Some(f);
    }

    fn load_rom(&mut self, data: &[u8]) -> Result<()> {
        let data = self.vm.reset(data);
        self.dev.reset(data);
        self.vm.run(&mut self.dev, 0x100);
        let out = self.dev.output(&self.vm);
        out.check()?;
        Ok(())
    }
}

impl eframe::App for Stage<'_> {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(e) = self.event_rx.try_recv() {
            match e {
                Event::LoadRom(data) => {
                    if let Err(e) = self.load_rom(&data) {
                        error!("could not load rom: {e:?}");
                    }
                }
                Event::SetMuted(m) => {
                    self.dev.audio_set_muted(m);
                }
                Event::Console(b) => {
                    self.dev.console(&mut self.vm, b);
                }
            }
        }

        // Repaint at vsync rate (60 FPS)
        ctx.request_repaint();
        ctx.input(|i| {
            while i.time >= self.next_frame {
                // Screen callback (limited to 60 FPS).  We want to err on the
                // side of redrawing early, rather than missing frames.
                self.next_frame += 0.0166667;
                self.dev.redraw(&mut self.vm);
            }

            if i.raw.dropped_files.len() == 1 {
                let target = &i.raw.dropped_files[0];
                let r = if let Some(path) = &target.path {
                    let data =
                        std::fs::read(path).expect("failed to read file");
                    info!("loading {} bytes from {path:?}", data.len());
                    self.load_rom(&data)
                } else if let Some(data) = &target.bytes {
                    self.load_rom(data)
                } else {
                    Ok(())
                };
                if let Err(e) = r {
                    error!("could not load ROM: {e:?}");
                }
            }

            let shift_held = i.modifiers.shift;
            for e in i.events.iter() {
                match e {
                    egui::Event::Text(s) => {
                        // The Text event doesn't handle Ctrl + characters, so
                        // we do everything through the Key event, with the
                        // exception of quotes (which don't have an associated
                        // key; https://github.com/emilk/egui/pull/4683)
                        //
                        // Similarly, the Key event doesn't always decode
                        // events with Shift and an attached key.  This is all
                        // terribly messy; my apologies.
                        const RAW_CHARS: [u8; 16] = [
                            b'"', b'\'', b'{', b'}', b'_', b')', b'(', b'*',
                            b'&', b'^', b'%', b'$', b'#', b'@', b'!', b'~',
                        ];
                        for c in s.bytes() {
                            if RAW_CHARS.contains(&c) {
                                self.dev.char(&mut self.vm, c);
                            }
                        }
                    }
                    egui::Event::Key {
                        key,
                        pressed,
                        repeat,
                        ..
                    } => {
                        if let Some(k) = decode_key(*key, shift_held) {
                            if *pressed {
                                self.dev.pressed(&mut self.vm, k, *repeat);
                            } else {
                                self.dev.released(&mut self.vm, k);
                            }
                        }
                    }
                    egui::Event::Scroll(s) => {
                        self.scroll.0 += s.x;
                        self.scroll.1 -= s.y;
                    }
                    _ => (),
                }
            }
            for (b, k) in [
                (i.modifiers.ctrl, Key::Ctrl),
                (i.modifiers.alt, Key::Alt),
                (i.modifiers.shift, Key::Shift),
            ] {
                if b {
                    self.dev.pressed(&mut self.vm, k, false)
                } else {
                    self.dev.released(&mut self.vm, k)
                }
            }

            let ptr = &i.pointer;
            if let Some(p) = ptr.latest_pos() {
                self.cursor_pos = Some((p.x / self.scale, p.y / self.scale));
            }

            let buttons = [
                egui::PointerButton::Primary,
                egui::PointerButton::Middle,
                egui::PointerButton::Secondary,
            ]
            .into_iter()
            .enumerate()
            .map(|(i, b)| (ptr.button_down(b) as u8) << i)
            .fold(0, |a, b| a | b);
            let m = MouseState {
                pos: self.cursor_pos.unwrap_or((0.0, 0.0)),
                scroll: std::mem::take(&mut self.scroll),
                buttons,
            };
            self.dev.mouse(&mut self.vm, m);
            i.time
        });

        // Handle audio callback
        self.dev.audio(&mut self.vm);

        let out = self.dev.output(&self.vm);

        // Update our GUI based on current state
        if out.hide_mouse {
            ctx.set_cursor_icon(egui::CursorIcon::None);
        }
        if self.size != out.size {
            info!("resizing window to {:?}", out.size);
            self.size = out.size;
            let size = egui::Vec2::new(out.size.0 as f32, out.size.1 as f32)
                * self.scale;
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
            if let Some(f) = self.resized.as_mut() {
                f(out.size.0, out.size.1);
            }
        }

        // TODO reduce allocation here?
        let mut image = egui::ColorImage::new(
            [out.size.0 as usize, out.size.1 as usize],
            egui::Color32::BLACK,
        );
        for (i, o) in out.frame.chunks(4).zip(image.pixels.iter_mut()) {
            *o = egui::Color32::from_rgba_unmultiplied(i[2], i[1], i[0], i[3]);
        }
        self.texture.set(image, egui::TextureOptions::NEAREST);

        egui::CentralPanel::default().show(ctx, |ui| {
            let mut mesh = egui::Mesh::with_texture(self.texture.id());
            mesh.add_rect_with_uv(
                egui::Rect {
                    min: egui::Pos2::new(0.0, 0.0),
                    max: egui::Pos2::new(
                        out.size.0 as f32 * self.scale,
                        out.size.1 as f32 * self.scale,
                    ),
                },
                egui::Rect {
                    min: egui::Pos2::new(0.0, 0.0),
                    max: egui::Pos2::new(1.0, 1.0),
                },
                egui::Color32::WHITE,
            );
            ui.painter().add(egui::Shape::mesh(mesh));
        });

        // Update stdout / stderr / exiting
        out.check().expect("failed to print output?");
    }
}

pub fn audio_setup(
    data: [Arc<Mutex<varvara::StreamData>>; 4],
) -> Option<(cpal::Device, [cpal::Stream; 4])> {
    use cpal::traits::{DeviceTrait, HostTrait};
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("no output device available");
    let supported_configs_range = device
        .supported_output_configs()
        .expect("error while querying configs");

    let Some(supported_config) = supported_configs_range
        .filter(|c| usize::from(c.channels()) == AUDIO_CHANNELS)
        .filter(|c| c.sample_format() == cpal::SampleFormat::F32)
        .find_map(|c| {
            c.try_with_sample_rate(cpal::SampleRate(AUDIO_SAMPLE_RATE))
        })
    else {
        error!(
            "could not find supported audio config ({} channels, {} Hz, f32)",
            AUDIO_CHANNELS, AUDIO_SAMPLE_RATE
        );
        error!("available configs:");
        for c in device.supported_output_configs().unwrap() {
            if c.min_sample_rate() == c.max_sample_rate() {
                error!(
                    "  channels: {}, sample_rate: {} Hz, {}",
                    c.channels(),
                    c.min_sample_rate().0,
                    c.sample_format(),
                );
            } else {
                error!(
                    "  channels: {}, sample_rate: {} - {} Hz, {}",
                    c.channels(),
                    c.min_sample_rate().0,
                    c.max_sample_rate().0,
                    c.sample_format(),
                );
            }
        }
        return None;
    };
    let config = supported_config.config();

    let streams = data.map(|d| {
        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _opt: &cpal::OutputCallbackInfo| {
                    d.lock().unwrap().next(data);
                },
                move |err| {
                    panic!("{err}");
                },
                None,
            )
            .expect("could not build stream");
        stream.play().unwrap();
        stream
    });
    Some((device, streams))
}

fn decode_key(k: egui::Key, shift: bool) -> Option<Key> {
    let c = match (k, shift) {
        (egui::Key::ArrowUp, _) => Key::Up,
        (egui::Key::ArrowDown, _) => Key::Down,
        (egui::Key::ArrowLeft, _) => Key::Left,
        (egui::Key::ArrowRight, _) => Key::Right,
        (egui::Key::Home, _) => Key::Home,
        (egui::Key::Num0, false) => Key::Char(b'0'),
        (egui::Key::Num0, true) => Key::Char(b')'),
        (egui::Key::Num1, false) => Key::Char(b'1'),
        (egui::Key::Num1, true) => Key::Char(b'!'),
        (egui::Key::Num2, false) => Key::Char(b'2'),
        (egui::Key::Num2, true) => Key::Char(b'@'),
        (egui::Key::Num3, false) => Key::Char(b'3'),
        (egui::Key::Num3, true) => Key::Char(b'#'),
        (egui::Key::Num4, false) => Key::Char(b'4'),
        (egui::Key::Num4, true) => Key::Char(b'$'),
        (egui::Key::Num5, false) => Key::Char(b'5'),
        (egui::Key::Num5, true) => Key::Char(b'5'),
        (egui::Key::Num6, false) => Key::Char(b'6'),
        (egui::Key::Num6, true) => Key::Char(b'^'),
        (egui::Key::Num7, false) => Key::Char(b'7'),
        (egui::Key::Num7, true) => Key::Char(b'&'),
        (egui::Key::Num8, false) => Key::Char(b'8'),
        (egui::Key::Num8, true) => Key::Char(b'*'),
        (egui::Key::Num9, false) => Key::Char(b'9'),
        (egui::Key::Num9, true) => Key::Char(b'('),
        (egui::Key::A, false) => Key::Char(b'a'),
        (egui::Key::A, true) => Key::Char(b'A'),
        (egui::Key::B, false) => Key::Char(b'b'),
        (egui::Key::B, true) => Key::Char(b'B'),
        (egui::Key::C, false) => Key::Char(b'c'),
        (egui::Key::C, true) => Key::Char(b'C'),
        (egui::Key::D, false) => Key::Char(b'd'),
        (egui::Key::D, true) => Key::Char(b'D'),
        (egui::Key::E, false) => Key::Char(b'e'),
        (egui::Key::E, true) => Key::Char(b'E'),
        (egui::Key::F, false) => Key::Char(b'f'),
        (egui::Key::F, true) => Key::Char(b'F'),
        (egui::Key::G, false) => Key::Char(b'g'),
        (egui::Key::G, true) => Key::Char(b'G'),
        (egui::Key::H, false) => Key::Char(b'h'),
        (egui::Key::H, true) => Key::Char(b'H'),
        (egui::Key::I, false) => Key::Char(b'i'),
        (egui::Key::I, true) => Key::Char(b'I'),
        (egui::Key::J, false) => Key::Char(b'j'),
        (egui::Key::J, true) => Key::Char(b'J'),
        (egui::Key::K, false) => Key::Char(b'k'),
        (egui::Key::K, true) => Key::Char(b'K'),
        (egui::Key::L, false) => Key::Char(b'l'),
        (egui::Key::L, true) => Key::Char(b'L'),
        (egui::Key::M, false) => Key::Char(b'm'),
        (egui::Key::M, true) => Key::Char(b'M'),
        (egui::Key::N, false) => Key::Char(b'n'),
        (egui::Key::N, true) => Key::Char(b'N'),
        (egui::Key::O, false) => Key::Char(b'o'),
        (egui::Key::O, true) => Key::Char(b'O'),
        (egui::Key::P, false) => Key::Char(b'p'),
        (egui::Key::P, true) => Key::Char(b'P'),
        (egui::Key::Q, false) => Key::Char(b'q'),
        (egui::Key::Q, true) => Key::Char(b'Q'),
        (egui::Key::R, false) => Key::Char(b'r'),
        (egui::Key::R, true) => Key::Char(b'R'),
        (egui::Key::S, false) => Key::Char(b's'),
        (egui::Key::S, true) => Key::Char(b'S'),
        (egui::Key::T, false) => Key::Char(b't'),
        (egui::Key::T, true) => Key::Char(b'T'),
        (egui::Key::U, false) => Key::Char(b'u'),
        (egui::Key::U, true) => Key::Char(b'U'),
        (egui::Key::V, false) => Key::Char(b'v'),
        (egui::Key::V, true) => Key::Char(b'V'),
        (egui::Key::W, false) => Key::Char(b'w'),
        (egui::Key::W, true) => Key::Char(b'W'),
        (egui::Key::X, false) => Key::Char(b'x'),
        (egui::Key::X, true) => Key::Char(b'X'),
        (egui::Key::Y, false) => Key::Char(b'y'),
        (egui::Key::Y, true) => Key::Char(b'Y'),
        (egui::Key::Z, false) => Key::Char(b'z'),
        (egui::Key::Z, true) => Key::Char(b'Z'),
        // TODO missing Key::Quote
        (egui::Key::Backtick, false) => Key::Char(b'`'),
        (egui::Key::Backtick, true) => Key::Char(b'~'),
        (egui::Key::Backslash, _) => Key::Char(b'\\'),
        (egui::Key::Pipe, _) => Key::Char(b'|'),
        (egui::Key::Comma, false) => Key::Char(b','),
        (egui::Key::Comma, true) => Key::Char(b'<'),
        (egui::Key::Equals, _) => Key::Char(b'='),
        (egui::Key::Plus, _) => Key::Char(b'+'),
        (egui::Key::OpenBracket, false) => Key::Char(b'['),
        (egui::Key::OpenBracket, true) => Key::Char(b'{'),
        (egui::Key::Minus, false) => Key::Char(b'-'),
        (egui::Key::Minus, true) => Key::Char(b'_'),
        (egui::Key::Period, false) => Key::Char(b'.'),
        (egui::Key::Period, true) => Key::Char(b'>'),
        (egui::Key::CloseBracket, false) => Key::Char(b']'),
        (egui::Key::CloseBracket, true) => Key::Char(b'}'),
        (egui::Key::Semicolon, _) => Key::Char(b';'),
        (egui::Key::Colon, _) => Key::Char(b':'),
        (egui::Key::Slash, _) => Key::Char(b'/'),
        (egui::Key::Questionmark, _) => Key::Char(b'?'),
        (egui::Key::Space, _) => Key::Char(b' '),
        (egui::Key::Tab, _) => Key::Char(b'\t'),
        (egui::Key::Enter, _) => Key::Char(b'\r'),
        (egui::Key::Backspace, _) => Key::Char(0x08),
        _ => return None,
    };
    Some(c)
}

#[cfg_attr(target_arch = "wasm32", path = "web.rs")]
#[cfg_attr(not(target_arch = "wasm32"), path = "native.rs")]
mod core;

fn main() -> anyhow::Result<()> {
    let out = core::run();
    match &out {
        Ok(()) => info!("core::run() completed successfully"),
        Err(e) => error!("core::run() failed: {e:?}"),
    };
    out
}
