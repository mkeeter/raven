use std::io::Read;
use std::path::PathBuf;

use uxn::{Uxn, UxnRam};
use varvara::{
    Input, Key, MouseState, Varvara, AUDIO_CHANNELS, AUDIO_SAMPLE_RATE,
};

use anyhow::{Context, Result};
use clap::Parser;
use cpal::traits::StreamTrait;
use minifb::{MouseButton, MouseMode, Scale, Window, WindowOptions};

/// Uxn runner
#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    rom: PathBuf,
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

fn decode_key(k: minifb::Key, shift: bool) -> Option<Key> {
    let c = match (k, shift) {
        (minifb::Key::LeftShift, _) => Key::LeftShift,
        (minifb::Key::RightShift, _) => Key::RightShift,
        (minifb::Key::LeftCtrl, _) => Key::LeftCtrl,
        (minifb::Key::LeftAlt, _) => Key::LeftAlt,
        (minifb::Key::Up, _) => Key::Up,
        (minifb::Key::Down, _) => Key::Down,
        (minifb::Key::Left, _) => Key::Left,
        (minifb::Key::Right, _) => Key::Right,
        (minifb::Key::LeftSuper, _) => Key::LeftSuper,
        (minifb::Key::RightSuper, _) => Key::RightSuper,
        (minifb::Key::Home, _) => Key::Home,
        (minifb::Key::Key0, false) => Key::Char(b'0'),
        (minifb::Key::Key0, true) => Key::Char(b')'),
        (minifb::Key::Key1, false) => Key::Char(b'1'),
        (minifb::Key::Key1, true) => Key::Char(b'!'),
        (minifb::Key::Key2, false) => Key::Char(b'2'),
        (minifb::Key::Key2, true) => Key::Char(b'@'),
        (minifb::Key::Key3, false) => Key::Char(b'3'),
        (minifb::Key::Key3, true) => Key::Char(b'#'),
        (minifb::Key::Key4, false) => Key::Char(b'4'),
        (minifb::Key::Key4, true) => Key::Char(b'$'),
        (minifb::Key::Key5, false) => Key::Char(b'5'),
        (minifb::Key::Key5, true) => Key::Char(b'5'),
        (minifb::Key::Key6, false) => Key::Char(b'6'),
        (minifb::Key::Key6, true) => Key::Char(b'^'),
        (minifb::Key::Key7, false) => Key::Char(b'7'),
        (minifb::Key::Key7, true) => Key::Char(b'&'),
        (minifb::Key::Key8, false) => Key::Char(b'8'),
        (minifb::Key::Key8, true) => Key::Char(b'*'),
        (minifb::Key::Key9, false) => Key::Char(b'9'),
        (minifb::Key::Key9, true) => Key::Char(b'('),
        (minifb::Key::A, false) => Key::Char(b'a'),
        (minifb::Key::A, true) => Key::Char(b'A'),
        (minifb::Key::B, false) => Key::Char(b'b'),
        (minifb::Key::B, true) => Key::Char(b'B'),
        (minifb::Key::C, false) => Key::Char(b'c'),
        (minifb::Key::C, true) => Key::Char(b'C'),
        (minifb::Key::D, false) => Key::Char(b'd'),
        (minifb::Key::D, true) => Key::Char(b'D'),
        (minifb::Key::E, false) => Key::Char(b'e'),
        (minifb::Key::E, true) => Key::Char(b'E'),
        (minifb::Key::F, false) => Key::Char(b'f'),
        (minifb::Key::F, true) => Key::Char(b'F'),
        (minifb::Key::G, false) => Key::Char(b'g'),
        (minifb::Key::G, true) => Key::Char(b'G'),
        (minifb::Key::H, false) => Key::Char(b'h'),
        (minifb::Key::H, true) => Key::Char(b'H'),
        (minifb::Key::I, false) => Key::Char(b'i'),
        (minifb::Key::I, true) => Key::Char(b'I'),
        (minifb::Key::J, false) => Key::Char(b'j'),
        (minifb::Key::J, true) => Key::Char(b'J'),
        (minifb::Key::K, false) => Key::Char(b'k'),
        (minifb::Key::K, true) => Key::Char(b'K'),
        (minifb::Key::L, false) => Key::Char(b'l'),
        (minifb::Key::L, true) => Key::Char(b'L'),
        (minifb::Key::M, false) => Key::Char(b'm'),
        (minifb::Key::M, true) => Key::Char(b'M'),
        (minifb::Key::N, false) => Key::Char(b'n'),
        (minifb::Key::N, true) => Key::Char(b'N'),
        (minifb::Key::O, false) => Key::Char(b'o'),
        (minifb::Key::O, true) => Key::Char(b'O'),
        (minifb::Key::P, false) => Key::Char(b'p'),
        (minifb::Key::P, true) => Key::Char(b'P'),
        (minifb::Key::Q, false) => Key::Char(b'q'),
        (minifb::Key::Q, true) => Key::Char(b'Q'),
        (minifb::Key::R, false) => Key::Char(b'r'),
        (minifb::Key::R, true) => Key::Char(b'R'),
        (minifb::Key::S, false) => Key::Char(b's'),
        (minifb::Key::S, true) => Key::Char(b'S'),
        (minifb::Key::T, false) => Key::Char(b't'),
        (minifb::Key::T, true) => Key::Char(b'T'),
        (minifb::Key::U, false) => Key::Char(b'u'),
        (minifb::Key::U, true) => Key::Char(b'U'),
        (minifb::Key::V, false) => Key::Char(b'v'),
        (minifb::Key::V, true) => Key::Char(b'V'),
        (minifb::Key::W, false) => Key::Char(b'w'),
        (minifb::Key::W, true) => Key::Char(b'W'),
        (minifb::Key::X, false) => Key::Char(b'x'),
        (minifb::Key::X, true) => Key::Char(b'X'),
        (minifb::Key::Y, false) => Key::Char(b'y'),
        (minifb::Key::Y, true) => Key::Char(b'Y'),
        (minifb::Key::Z, false) => Key::Char(b'z'),
        (minifb::Key::Z, true) => Key::Char(b'Z'),
        (minifb::Key::Apostrophe, false) => Key::Char(b'\''),
        (minifb::Key::Apostrophe, true) => Key::Char(b'\"'),
        (minifb::Key::Backquote, false) => Key::Char(b'`'),
        (minifb::Key::Backquote, true) => Key::Char(b'~'),
        (minifb::Key::Backslash, false) => Key::Char(b'\\'),
        (minifb::Key::Backslash, true) => Key::Char(b'|'),
        (minifb::Key::Comma, false) => Key::Char(b','),
        (minifb::Key::Comma, true) => Key::Char(b'<'),
        (minifb::Key::Equal, false) => Key::Char(b'='),
        (minifb::Key::Equal, true) => Key::Char(b'+'),
        (minifb::Key::LeftBracket, false) => Key::Char(b'['),
        (minifb::Key::LeftBracket, true) => Key::Char(b'{'),
        (minifb::Key::Minus, false) => Key::Char(b'-'),
        (minifb::Key::Minus, true) => Key::Char(b'_'),
        (minifb::Key::Period, false) => Key::Char(b'.'),
        (minifb::Key::Period, true) => Key::Char(b'>'),
        (minifb::Key::RightBracket, false) => Key::Char(b']'),
        (minifb::Key::RightBracket, true) => Key::Char(b'}'),
        (minifb::Key::Semicolon, false) => Key::Char(b';'),
        (minifb::Key::Semicolon, true) => Key::Char(b':'),
        (minifb::Key::Slash, false) => Key::Char(b'/'),
        (minifb::Key::Slash, true) => Key::Char(b'?'),
        (minifb::Key::Space, _) => Key::Char(b' '),
        (minifb::Key::Tab, _) => Key::Char(b'\t'),
        (minifb::Key::NumPad0, _) => Key::Char(b'0'),
        (minifb::Key::NumPad1, _) => Key::Char(b'1'),
        (minifb::Key::NumPad2, _) => Key::Char(b'2'),
        (minifb::Key::NumPad3, _) => Key::Char(b'3'),
        (minifb::Key::NumPad4, _) => Key::Char(b'4'),
        (minifb::Key::NumPad5, _) => Key::Char(b'5'),
        (minifb::Key::NumPad6, _) => Key::Char(b'6'),
        (minifb::Key::NumPad7, _) => Key::Char(b'7'),
        (minifb::Key::NumPad8, _) => Key::Char(b'8'),
        (minifb::Key::NumPad9, _) => Key::Char(b'9'),
        (minifb::Key::NumPadDot, _) => Key::Char(b'.'),
        (minifb::Key::NumPadSlash, _) => Key::Char(b'/'),
        (minifb::Key::NumPadAsterisk, _) => Key::Char(b'*'),
        (minifb::Key::NumPadMinus, _) => Key::Char(b'-'),
        (minifb::Key::NumPadPlus, _) => Key::Char(b'+'),
        _ => return None,
    };
    Some(c)
}

/// Reopens the window, based on the screen size
pub fn open_window(size: (u16, u16), hide_mouse: bool) -> Window {
    let (width, height) = size;
    let mut window = Window::new(
        "Varvara",
        width as usize,
        height as usize,
        WindowOptions {
            scale: Scale::X2,
            ..WindowOptions::default()
        },
    )
    .unwrap();
    window.set_target_fps(120);
    if hide_mouse {
        window.set_cursor_visibility(false);
    }
    window
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

    let mut ram = UxnRam::new();
    let mut vm = Uxn::new(&rom, &mut ram);
    let mut dev = Varvara::new();

    let _audio = audio_setup(&dev);

    // Run the reset vector
    vm.run(&mut dev, 0x100);

    let out = dev.output(&vm);
    out.print()?;
    if let Some(e) = out.exit {
        std::process::exit(e);
    }

    let mut window = open_window(dev.screen_size(), false);

    let mut frame = 0;
    let mut hide_mouse = false;
    let console_rx = varvara::console_worker();
    while window.is_open() {
        // We run at 120 Hz, so call the screen vector every other frame
        if frame % 2 == 0 {
            dev.redraw(&mut vm);
        }
        frame += 1;

        let mouse = mouse_state(&window);
        let shift_held = dev.shift_held(); // TODO find this from the window?
        let pressed: Vec<Key> = window
            .get_keys_pressed(minifb::KeyRepeat::Yes)
            .into_iter()
            .flat_map(|k| decode_key(k, shift_held))
            .collect();
        let released: Vec<Key> = window
            .get_keys_released()
            .into_iter()
            .flat_map(|k| decode_key(k, shift_held))
            .collect();

        let input = Input {
            mouse,
            pressed,
            released,
            console: console_rx.try_recv().ok(),
        };
        let prev_size = dev.screen_size();

        // Handle device events
        let out = dev.update(&mut vm, input);

        // Update our GUI based on current state
        if out.hide_mouse && !hide_mouse {
            hide_mouse = true;
            window.set_cursor_visibility(false);
        }
        if prev_size != out.size {
            window = open_window(out.size, hide_mouse);
        }

        out.print()?;
        if let Some(e) = out.exit {
            std::process::exit(e);
        }

        // Redraw
        window
            .update_with_buffer(
                out.frame,
                out.size.0 as usize,
                out.size.1 as usize,
            )
            .unwrap();
    }

    Ok(())
}
