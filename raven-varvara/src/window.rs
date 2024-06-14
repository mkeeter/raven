use crate::{
    controller::{Controller, ControllerPorts, Key},
    mouse::{Mouse, MousePorts, MouseState},
    screen::{Screen, ScreenPorts},
    Event,
};
use minifb::{
    MouseButton, MouseMode, Scale, Window as FbWindow, WindowOptions,
};
use std::collections::VecDeque;
use uxn::{Ports, Uxn};

pub struct Window {
    pub screen: Screen,
    pub mouse: Mouse,
    pub controller: Controller,

    has_mouse: bool,
    has_controller: bool,
    window: FbWindow,
    frame: u64,
}

const APP_NAME: &str = "Varvara";
impl Window {
    pub fn new() -> Self {
        const WIDTH: u16 = 512;
        const HEIGHT: u16 = 320;
        let screen = Screen::new(WIDTH, HEIGHT);
        let mouse = Mouse::new();

        let mut window = FbWindow::new(
            APP_NAME,
            WIDTH as usize,
            HEIGHT as usize,
            WindowOptions {
                scale: Scale::X2,
                ..WindowOptions::default()
            },
        )
        .unwrap();
        window.set_target_fps(120);

        Self {
            screen,
            mouse,
            controller: Controller::default(),
            frame: 0,

            has_mouse: false,
            has_controller: false,
            window,
        }
    }

    /// Sets `self.has_mouse` to true and hides the cursor
    fn set_mouse(&mut self) {
        if !self.has_mouse {
            self.has_mouse = true;
            self.window.set_cursor_visibility(false);
        }
    }

    fn mouse_state(window: &FbWindow) -> MouseState {
        let pos = window.get_mouse_pos(MouseMode::Clamp).unwrap();
        let scroll = window.get_scroll_wheel().unwrap_or((0.0, 0.0));
        let buttons =
            [MouseButton::Left, MouseButton::Middle, MouseButton::Right]
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

    pub fn update(&mut self, vm: &mut Uxn, queue: &mut VecDeque<Event>) {
        // The screen vector should be called every other frame, since we do
        // updates at ~120 FPS
        if self.frame & 1 == 1 {
            let vector = self.screen.update(vm);
            queue.push_back(Event { vector, data: None });
        };
        self.frame = self.frame.wrapping_add(1);

        // The mouse vector should be called if it changed
        if self.has_mouse {
            let state = Self::mouse_state(&self.window);
            if let Some(vector) = self.mouse.update(vm, state) {
                queue.push_back(Event { vector, data: None });
            }
        }

        if self.has_controller {
            for k in self.window.get_keys_pressed(minifb::KeyRepeat::Yes) {
                if let Some(k) =
                    Self::decode_key(k, self.controller.shift_held())
                {
                    queue.extend(self.controller.pressed(vm, k));
                }
            }
            for k in self.window.get_keys_released() {
                if let Some(k) =
                    Self::decode_key(k, self.controller.shift_held())
                {
                    queue.extend(self.controller.released(vm, k));
                }
            }
        }
    }

    /// Redraws the window and handles miscellaneous polling
    ///
    /// Returns `true` if the window is still open; `false` otherwise
    pub fn redraw(&mut self, vm: &Uxn) -> bool {
        if self.screen.resized() {
            self.reopen();
        }
        let (buffer, width, height) = self.screen.redraw(vm);
        self.window
            .update_with_buffer(buffer, width as usize, height as usize)
            .unwrap();
        self.window.is_open()
    }

    /// Reopens the window, based on the screen size
    pub fn reopen(&mut self) {
        let (width, height) = self.screen.size();
        self.window = FbWindow::new(
            APP_NAME,
            width as usize,
            height as usize,
            WindowOptions {
                scale: Scale::X2,
                ..WindowOptions::default()
            },
        )
        .unwrap();
        self.window.set_target_fps(120);
        if self.has_mouse {
            self.window.set_cursor_visibility(false);
        }
    }

    /// Triggers a DEO operation on a child component
    ///
    /// Returns `true` if the operation was handled, `false` otherwise
    pub fn deo(&mut self, vm: &mut Uxn, target: u8) -> bool {
        match target & 0xF0 {
            ScreenPorts::BASE => self.screen.deo(vm, target),
            MousePorts::BASE => self.set_mouse(),
            ControllerPorts::BASE => self.has_controller = true,

            _ => return false,
        }
        true
    }

    /// Triggers a DEI operation on a child component
    ///
    /// Returns `true` if the operation was handled, `false` otherwise
    pub fn dei(&mut self, vm: &mut Uxn, target: u8) -> bool {
        match target & 0xF0 {
            ScreenPorts::BASE => self.screen.dei(vm, target),
            MousePorts::BASE => self.set_mouse(),
            ControllerPorts::BASE => self.has_controller = true,

            _ => return false,
        }
        true
    }
}
