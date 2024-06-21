use uxn::Uxn;
use varvara::{Key, MouseState, Varvara, AUDIO_CHANNELS, AUDIO_SAMPLE_RATE};

use cpal::traits::StreamTrait;
use eframe::egui;
use log::warn;

pub struct Stage<'a> {
    vm: Uxn<'a>,
    dev: Varvara,

    /// Time (in seconds) at which we should draw the next frame
    next_frame: f64,

    #[cfg(not(target_arch = "wasm32"))]
    console_rx: std::sync::mpsc::Receiver<u8>,

    scroll: (f32, f32),
    cursor_pos: Option<(f32, f32)>,

    texture: egui::TextureHandle,
}

impl<'a> Stage<'a> {
    pub fn new(
        vm: Uxn<'a>,
        mut dev: Varvara,
        ctx: &egui::Context,
    ) -> Stage<'a> {
        let out = dev.output(&vm);

        let size = out.size;
        let image = egui::ColorImage::new(
            [size.0 as usize, size.1 as usize],
            egui::Color32::BLACK,
        );

        let texture =
            ctx.load_texture("frame", image, egui::TextureOptions::NEAREST);

        Stage {
            vm,
            dev,

            next_frame: 0.0,

            #[cfg(not(target_arch = "wasm32"))]
            console_rx: varvara::console_worker(),

            scroll: (0.0, 0.0),
            cursor_pos: None,

            texture,
        }
    }
}

impl eframe::App for Stage<'_> {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let time = ctx.input(|i| {
            for e in i.events.iter() {
                match e {
                    egui::Event::Text(s) => {
                        for c in s.bytes() {
                            self.dev.char(&mut self.vm, c);
                        }
                    }
                    egui::Event::Key { key, pressed, .. } => {
                        if let Some(k) = decode_key(*key) {
                            if *pressed {
                                self.dev.pressed(&mut self.vm, k);
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
                    self.dev.pressed(&mut self.vm, k)
                } else {
                    self.dev.released(&mut self.vm, k)
                }
            }

            let ptr = &i.pointer;
            if let Some(p) = ptr.latest_pos() {
                self.cursor_pos = Some((p.x, p.y));
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

        // Listen for console characters
        #[cfg(not(target_arch = "wasm32"))]
        if let Ok(c) = self.console_rx.try_recv() {
            self.dev.console(&mut self.vm, c);
        }

        // Handle audio callback
        self.dev.audio(&mut self.vm);

        // Screen callback (limited to 60 FPS)
        if time >= self.next_frame {
            self.dev.redraw(&mut self.vm);
            self.next_frame = time + 0.01666666666;
        }
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(
            self.next_frame - time,
        ));

        let prev_size = self.dev.screen_size();
        let out = self.dev.output(&self.vm);

        // Update our GUI based on current state
        if out.hide_mouse {
            ctx.set_cursor_icon(egui::CursorIcon::None);
        }
        if prev_size != out.size {
            warn!("can't programmatically resize window");
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
                    max: egui::Pos2::new(out.size.0 as f32, out.size.1 as f32),
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

pub fn audio_setup(dev: &Varvara) -> (cpal::Device, [cpal::Stream; 4]) {
    use cpal::traits::{DeviceTrait, HostTrait};
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("no output device available");
    let mut supported_configs_range = device
        .supported_output_configs()
        .expect("error while querying configs");

    let supported_config = supported_configs_range
        .find_map(|c| {
            c.try_with_sample_rate(cpal::SampleRate(AUDIO_SAMPLE_RATE))
        })
        .filter(|c| c.channels() == AUDIO_CHANNELS)
        .expect("no supported config?");
    let config = supported_config.config();

    let streams = [0, 1, 2, 3].map(|i| {
        let d = dev.audio_stream(i);
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
    (device, streams)
}

fn decode_key(k: egui::Key) -> Option<Key> {
    let c = match k {
        egui::Key::ArrowUp => Key::Up,
        egui::Key::ArrowDown => Key::Down,
        egui::Key::ArrowLeft => Key::Left,
        egui::Key::ArrowRight => Key::Right,
        egui::Key::Home => Key::Home,

        // TODO are these also sent as text events?
        egui::Key::Tab => Key::Char(b'\t'),
        egui::Key::Backspace => Key::Char(0x08),
        egui::Key::Enter => Key::Char(b'\r'),
        _ => return None,
    };
    Some(c)
}
