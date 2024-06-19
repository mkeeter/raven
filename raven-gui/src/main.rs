use std::io::Read;
use std::path::PathBuf;

use uxn::{Uxn, UxnRam};
use varvara::{Input, Varvara, AUDIO_CHANNELS, AUDIO_SAMPLE_RATE};

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

    mouse_hidden: bool,
    changes: Input,
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

        let mouse_hidden = out.hide_mouse;
        Stage {
            vm,
            dev,

            mouse_hidden,
            changes: Default::default(),
            console_rx,

            texture,
        }
    }
}

impl eframe::App for Stage<'_> {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // TODO mouse and keyboard handling

        let mut input = std::mem::take(&mut self.changes);

        // Listen for console characters
        input.console = self.console_rx.try_recv().ok();

        let prev_size = self.dev.screen_size();
        let out = self.dev.update(&mut self.vm, input);

        // Update our GUI based on current state
        if out.hide_mouse && !self.mouse_hidden {
            ctx.set_cursor_icon(egui::CursorIcon::None);
            self.mouse_hidden = true;
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

/*
fn mouse_state(window: &Window) -> MouseState {
    let pos = window.get_mouse_pos(MouseMode::Clamp).unwrap();
    let scroll = window.get_scroll_wheel().unwrap_or((0.0, 0.0));
    let buttons = [MouseButton::Left, MouseButton::Middle, MouseButton::Right]
        .into_iter()
        .enumerate()
        .map(|(i, b)| (window.get_mouse_down(b) as u8) << i)
        .fold(0, |a, b| a | b);
    MouseState {
        pos,
        scroll,
        buttons,
    }
}
*/

/*
fn decode_key(k: miniquad::KeyCode, shift: bool) -> Option<Key> {
    let c = match (k, shift) {
        (miniquad::KeyCode::LeftShift, _) => Key::LeftShift,
        (miniquad::KeyCode::RightShift, _) => Key::RightShift,
        (miniquad::KeyCode::LeftControl, _) => Key::LeftCtrl,
        (miniquad::KeyCode::LeftAlt, _) => Key::LeftAlt,
        (miniquad::KeyCode::Up, _) => Key::Up,
        (miniquad::KeyCode::Down, _) => Key::Down,
        (miniquad::KeyCode::Left, _) => Key::Left,
        (miniquad::KeyCode::Right, _) => Key::Right,
        (miniquad::KeyCode::LeftSuper, _) => Key::LeftSuper,
        (miniquad::KeyCode::RightSuper, _) => Key::RightSuper,
        (miniquad::KeyCode::Home, _) => Key::Home,
        (miniquad::KeyCode::Key0, false) => Key::Char(b'0'),
        (miniquad::KeyCode::Key0, true) => Key::Char(b')'),
        (miniquad::KeyCode::Key1, false) => Key::Char(b'1'),
        (miniquad::KeyCode::Key1, true) => Key::Char(b'!'),
        (miniquad::KeyCode::Key2, false) => Key::Char(b'2'),
        (miniquad::KeyCode::Key2, true) => Key::Char(b'@'),
        (miniquad::KeyCode::Key3, false) => Key::Char(b'3'),
        (miniquad::KeyCode::Key3, true) => Key::Char(b'#'),
        (miniquad::KeyCode::Key4, false) => Key::Char(b'4'),
        (miniquad::KeyCode::Key4, true) => Key::Char(b'$'),
        (miniquad::KeyCode::Key5, false) => Key::Char(b'5'),
        (miniquad::KeyCode::Key5, true) => Key::Char(b'5'),
        (miniquad::KeyCode::Key6, false) => Key::Char(b'6'),
        (miniquad::KeyCode::Key6, true) => Key::Char(b'^'),
        (miniquad::KeyCode::Key7, false) => Key::Char(b'7'),
        (miniquad::KeyCode::Key7, true) => Key::Char(b'&'),
        (miniquad::KeyCode::Key8, false) => Key::Char(b'8'),
        (miniquad::KeyCode::Key8, true) => Key::Char(b'*'),
        (miniquad::KeyCode::Key9, false) => Key::Char(b'9'),
        (miniquad::KeyCode::Key9, true) => Key::Char(b'('),
        (miniquad::KeyCode::A, false) => Key::Char(b'a'),
        (miniquad::KeyCode::A, true) => Key::Char(b'A'),
        (miniquad::KeyCode::B, false) => Key::Char(b'b'),
        (miniquad::KeyCode::B, true) => Key::Char(b'B'),
        (miniquad::KeyCode::C, false) => Key::Char(b'c'),
        (miniquad::KeyCode::C, true) => Key::Char(b'C'),
        (miniquad::KeyCode::D, false) => Key::Char(b'd'),
        (miniquad::KeyCode::D, true) => Key::Char(b'D'),
        (miniquad::KeyCode::E, false) => Key::Char(b'e'),
        (miniquad::KeyCode::E, true) => Key::Char(b'E'),
        (miniquad::KeyCode::F, false) => Key::Char(b'f'),
        (miniquad::KeyCode::F, true) => Key::Char(b'F'),
        (miniquad::KeyCode::G, false) => Key::Char(b'g'),
        (miniquad::KeyCode::G, true) => Key::Char(b'G'),
        (miniquad::KeyCode::H, false) => Key::Char(b'h'),
        (miniquad::KeyCode::H, true) => Key::Char(b'H'),
        (miniquad::KeyCode::I, false) => Key::Char(b'i'),
        (miniquad::KeyCode::I, true) => Key::Char(b'I'),
        (miniquad::KeyCode::J, false) => Key::Char(b'j'),
        (miniquad::KeyCode::J, true) => Key::Char(b'J'),
        (miniquad::KeyCode::K, false) => Key::Char(b'k'),
        (miniquad::KeyCode::K, true) => Key::Char(b'K'),
        (miniquad::KeyCode::L, false) => Key::Char(b'l'),
        (miniquad::KeyCode::L, true) => Key::Char(b'L'),
        (miniquad::KeyCode::M, false) => Key::Char(b'm'),
        (miniquad::KeyCode::M, true) => Key::Char(b'M'),
        (miniquad::KeyCode::N, false) => Key::Char(b'n'),
        (miniquad::KeyCode::N, true) => Key::Char(b'N'),
        (miniquad::KeyCode::O, false) => Key::Char(b'o'),
        (miniquad::KeyCode::O, true) => Key::Char(b'O'),
        (miniquad::KeyCode::P, false) => Key::Char(b'p'),
        (miniquad::KeyCode::P, true) => Key::Char(b'P'),
        (miniquad::KeyCode::Q, false) => Key::Char(b'q'),
        (miniquad::KeyCode::Q, true) => Key::Char(b'Q'),
        (miniquad::KeyCode::R, false) => Key::Char(b'r'),
        (miniquad::KeyCode::R, true) => Key::Char(b'R'),
        (miniquad::KeyCode::S, false) => Key::Char(b's'),
        (miniquad::KeyCode::S, true) => Key::Char(b'S'),
        (miniquad::KeyCode::T, false) => Key::Char(b't'),
        (miniquad::KeyCode::T, true) => Key::Char(b'T'),
        (miniquad::KeyCode::U, false) => Key::Char(b'u'),
        (miniquad::KeyCode::U, true) => Key::Char(b'U'),
        (miniquad::KeyCode::V, false) => Key::Char(b'v'),
        (miniquad::KeyCode::V, true) => Key::Char(b'V'),
        (miniquad::KeyCode::W, false) => Key::Char(b'w'),
        (miniquad::KeyCode::W, true) => Key::Char(b'W'),
        (miniquad::KeyCode::X, false) => Key::Char(b'x'),
        (miniquad::KeyCode::X, true) => Key::Char(b'X'),
        (miniquad::KeyCode::Y, false) => Key::Char(b'y'),
        (miniquad::KeyCode::Y, true) => Key::Char(b'Y'),
        (miniquad::KeyCode::Z, false) => Key::Char(b'z'),
        (miniquad::KeyCode::Z, true) => Key::Char(b'Z'),
        (miniquad::KeyCode::Apostrophe, false) => Key::Char(b'\''),
        (miniquad::KeyCode::Apostrophe, true) => Key::Char(b'\"'),
        (miniquad::KeyCode::GraveAccent, false) => Key::Char(b'`'),
        (miniquad::KeyCode::GraveAccent, true) => Key::Char(b'~'),
        (miniquad::KeyCode::Backslash, false) => Key::Char(b'\\'),
        (miniquad::KeyCode::Backslash, true) => Key::Char(b'|'),
        (miniquad::KeyCode::Comma, false) => Key::Char(b','),
        (miniquad::KeyCode::Comma, true) => Key::Char(b'<'),
        (miniquad::KeyCode::Equal, false) => Key::Char(b'='),
        (miniquad::KeyCode::Equal, true) => Key::Char(b'+'),
        (miniquad::KeyCode::LeftBracket, false) => Key::Char(b'['),
        (miniquad::KeyCode::LeftBracket, true) => Key::Char(b'{'),
        (miniquad::KeyCode::Minus, false) => Key::Char(b'-'),
        (miniquad::KeyCode::Minus, true) => Key::Char(b'_'),
        (miniquad::KeyCode::Period, false) => Key::Char(b'.'),
        (miniquad::KeyCode::Period, true) => Key::Char(b'>'),
        (miniquad::KeyCode::RightBracket, false) => Key::Char(b']'),
        (miniquad::KeyCode::RightBracket, true) => Key::Char(b'}'),
        (miniquad::KeyCode::Semicolon, false) => Key::Char(b';'),
        (miniquad::KeyCode::Semicolon, true) => Key::Char(b':'),
        (miniquad::KeyCode::Slash, false) => Key::Char(b'/'),
        (miniquad::KeyCode::Slash, true) => Key::Char(b'?'),
        (miniquad::KeyCode::Space, _) => Key::Char(b' '),
        (miniquad::KeyCode::Tab, _) => Key::Char(b'\t'),
        (miniquad::KeyCode::Backspace, _) => Key::Char(0x08),
        (miniquad::KeyCode::Enter, _) => Key::Char(b'\r'),
        _ => return None,
    };
    Some(c)
}
*/

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
