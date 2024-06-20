use std::io::Read;
use std::path::PathBuf;

use uxn::{Uxn, UxnRam};
use varvara::{Key, MouseState, Varvara, AUDIO_CHANNELS, AUDIO_SAMPLE_RATE};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use cpal::traits::StreamTrait;
use eframe::egui;
use log::{info, warn};

/// Uxn runner
#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Target file to load
    rom: PathBuf,

    /// Arguments to pass into the VM
    #[arg(last = true)]
    args: Vec<String>,
}

struct Stage<'a> {
    vm: Uxn<'a>,
    dev: Varvara,

    next_frame: std::time::Instant,

    scroll: (f32, f32),
    cursor_pos: Option<(f32, f32)>,
    console_rx: std::sync::mpsc::Receiver<u8>,

    texture: egui::TextureHandle,
}

impl<'a> Stage<'a> {
    fn new(vm: Uxn<'a>, mut dev: Varvara, ctx: &egui::Context) -> Stage<'a> {
        let out = dev.output(&vm);
        let console_rx = varvara::console_worker();

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

            next_frame: std::time::Instant::now(),

            scroll: (0.0, 0.0),
            cursor_pos: None,
            console_rx,

            texture,
        }
    }
}

impl eframe::App for Stage<'_> {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let dt = std::time::Duration::from_millis(16);
        let now = std::time::Instant::now();
        ctx.request_repaint_after(dt);

        ctx.input(|i| {
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
        });

        // Listen for console characters
        if let Ok(c) = self.console_rx.try_recv() {
            self.dev.console(&mut self.vm, c);
        }

        // Handle audio callback
        self.dev.audio(&mut self.vm);

        // Screen callback (limited to 60 FPS)
        if now >= self.next_frame {
            println!("{:?}", now - self.next_frame);
            self.dev.redraw(&mut self.vm);
            self.next_frame = now + std::time::Duration::from_millis(15);
        }

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

fn audio_setup(dev: &Varvara) -> (cpal::Device, [cpal::Stream; 4]) {
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

fn main() -> Result<()> {
    let env = env_logger::Env::default()
        .filter_or("UXN_LOG", "info")
        .write_style_or("UXN_LOG", "always");
    env_logger::init_from_env(env);

    let args = Args::parse();
    let mut f = std::fs::File::open(&args.rom)
        .with_context(|| format!("failed to open {:?}", args.rom))?;

    let mut rom = vec![];
    f.read_to_end(&mut rom).context("failed to read file")?;

    let ram = UxnRam::new();
    let mut vm = Uxn::new(&rom, ram.leak());
    let mut dev = Varvara::new();

    let _audio = audio_setup(&dev);

    // Run the reset vector
    let start = std::time::Instant::now();
    vm.run(&mut dev, 0x100);
    info!("startup complete in {:?}", start.elapsed());

    dev.output(&vm).check()?;
    dev.send_args(&mut vm, &args.args).check()?;

    let (width, height) = dev.output(&vm).size;
    let options = eframe::NativeOptions {
        window_builder: Some(Box::new(move |v| {
            v.with_inner_size(egui::Vec2::new(width as f32, height as f32))
                .with_resizable(false)
        })),
        ..Default::default()
    };

    eframe::run_native(
        "Varvara",
        options,
        Box::new(move |cc| Box::new(Stage::new(vm, dev, &cc.egui_ctx))),
    )
    .map_err(|e| anyhow!("got egui error: {e:?}"))
}
