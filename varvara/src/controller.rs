use crate::Event;
use minifb::Key;
use std::{collections::HashSet, mem::offset_of};
use uxn::{Ports, Uxn};
use zerocopy::{AsBytes, BigEndian, FromBytes, FromZeroes, U16};

#[derive(AsBytes, FromZeroes, FromBytes)]
#[repr(C)]
pub struct ControllerPorts {
    vector: U16<BigEndian>,
    button: u8,
    key: u8,
    _pad: [u8; 12],
}

impl Ports for ControllerPorts {
    const BASE: u8 = 0x80;
}

impl ControllerPorts {
    const KEY: u8 = Self::BASE | offset_of!(Self, key) as u8;
}

#[derive(Default)]
pub struct Controller {
    /// Non-character keys that are held down
    down: HashSet<Key>,

    /// Current button state
    buttons: u8,
}

impl Controller {
    /// Send the given key event, returning a vector [`Event`]
    #[must_use]
    pub fn pressed(&mut self, vm: &mut Uxn, k: Key) -> Option<Event> {
        if matches!(
            k,
            Key::LeftShift
                | Key::RightShift
                | Key::LeftCtrl
                | Key::LeftAlt
                | Key::Up
                | Key::Down
                | Key::Left
                | Key::Right
                | Key::LeftSuper
                | Key::RightSuper
                | Key::Home
        ) {
            self.down.insert(k);
        }

        let event = self.check_buttons(vm);

        let shift = self.down.contains(&Key::LeftShift)
            | self.down.contains(&Key::RightShift);

        // US keyboard decoding
        let c = match (k, shift) {
            (Key::Key0, false) => b'0',
            (Key::Key0, true) => b')',
            (Key::Key1, false) => b'1',
            (Key::Key1, true) => b'!',
            (Key::Key2, false) => b'2',
            (Key::Key2, true) => b'@',
            (Key::Key3, false) => b'3',
            (Key::Key3, true) => b'#',
            (Key::Key4, false) => b'4',
            (Key::Key4, true) => b'$',
            (Key::Key5, false) => b'5',
            (Key::Key5, true) => b'5',
            (Key::Key6, false) => b'6',
            (Key::Key6, true) => b'^',
            (Key::Key7, false) => b'7',
            (Key::Key7, true) => b'&',
            (Key::Key8, false) => b'8',
            (Key::Key8, true) => b'*',
            (Key::Key9, false) => b'9',
            (Key::Key9, true) => b'(',
            (Key::A, false) => b'a',
            (Key::A, true) => b'A',
            (Key::B, false) => b'b',
            (Key::B, true) => b'B',
            (Key::C, false) => b'c',
            (Key::C, true) => b'C',
            (Key::D, false) => b'd',
            (Key::D, true) => b'D',
            (Key::E, false) => b'e',
            (Key::E, true) => b'E',
            (Key::F, false) => b'f',
            (Key::F, true) => b'F',
            (Key::G, false) => b'g',
            (Key::G, true) => b'G',
            (Key::H, false) => b'h',
            (Key::H, true) => b'H',
            (Key::I, false) => b'i',
            (Key::I, true) => b'I',
            (Key::J, false) => b'j',
            (Key::J, true) => b'J',
            (Key::K, false) => b'k',
            (Key::K, true) => b'K',
            (Key::L, false) => b'l',
            (Key::L, true) => b'L',
            (Key::M, false) => b'm',
            (Key::M, true) => b'M',
            (Key::N, false) => b'n',
            (Key::N, true) => b'N',
            (Key::O, false) => b'o',
            (Key::O, true) => b'O',
            (Key::P, false) => b'p',
            (Key::P, true) => b'P',
            (Key::Q, false) => b'q',
            (Key::Q, true) => b'Q',
            (Key::R, false) => b'r',
            (Key::R, true) => b'R',
            (Key::S, false) => b's',
            (Key::S, true) => b'S',
            (Key::T, false) => b't',
            (Key::T, true) => b'T',
            (Key::U, false) => b'u',
            (Key::U, true) => b'U',
            (Key::V, false) => b'v',
            (Key::V, true) => b'V',
            (Key::W, false) => b'w',
            (Key::W, true) => b'W',
            (Key::X, false) => b'x',
            (Key::X, true) => b'X',
            (Key::Y, false) => b'y',
            (Key::Y, true) => b'Y',
            (Key::Z, false) => b'z',
            (Key::Z, true) => b'Z',
            (Key::Apostrophe, false) => b'\'',
            (Key::Apostrophe, true) => b'\"',
            (Key::Backquote, false) => b'`',
            (Key::Backquote, true) => b'~',
            (Key::Backslash, false) => b'\\',
            (Key::Backslash, true) => b'|',
            (Key::Comma, false) => b',',
            (Key::Comma, true) => b'<',
            (Key::Equal, false) => b'=',
            (Key::Equal, true) => b'+',
            (Key::LeftBracket, false) => b'[',
            (Key::LeftBracket, true) => b'{',
            (Key::Minus, false) => b'-',
            (Key::Minus, true) => b'_',
            (Key::Period, false) => b'.',
            (Key::Period, true) => b'>',
            (Key::RightBracket, false) => b']',
            (Key::RightBracket, true) => b'}',
            (Key::Semicolon, false) => b';',
            (Key::Semicolon, true) => b':',
            (Key::Slash, false) => b'/',
            (Key::Slash, true) => b'?',
            (Key::Space, _) => b' ',
            (Key::Tab, _) => b'\t',
            (Key::NumPad0, _) => b'0',
            (Key::NumPad1, _) => b'1',
            (Key::NumPad2, _) => b'2',
            (Key::NumPad3, _) => b'3',
            (Key::NumPad4, _) => b'4',
            (Key::NumPad5, _) => b'5',
            (Key::NumPad6, _) => b'6',
            (Key::NumPad7, _) => b'7',
            (Key::NumPad8, _) => b'8',
            (Key::NumPad9, _) => b'9',
            (Key::NumPadDot, _) => b'.',
            (Key::NumPadSlash, _) => b'/',
            (Key::NumPadAsterisk, _) => b'*',
            (Key::NumPadMinus, _) => b'-',
            (Key::NumPadPlus, _) => b'+',
            _ => return event,
        };
        let p = vm.dev::<ControllerPorts>();
        Some(Event {
            vector: p.vector.get(),
            data: Some((ControllerPorts::KEY, c)),
        })
    }

    pub fn released(&mut self, vm: &mut Uxn, k: Key) -> Option<Event> {
        self.down.remove(&k);
        self.check_buttons(vm)
    }

    fn check_buttons(&mut self, vm: &mut Uxn) -> Option<Event> {
        let mut buttons = 0;
        for (i, k) in [
            Key::LeftCtrl,
            Key::LeftAlt,
            Key::LeftShift,
            Key::Home,
            Key::Up,
            Key::Down,
            Key::Left,
            Key::Right,
        ]
        .iter()
        .enumerate()
        {
            if self.down.contains(k) {
                buttons |= 1 << i;
            }
        }
        if self.down.contains(&Key::Left)
            && (self.down.contains(&Key::LeftSuper)
                || self.down.contains(&Key::RightSuper))
        {
            buttons |= 0x08;
        }

        // We'll return this event in case we don't have a keypress event;
        // otherwise, the keypress event will call the vector (at least once)
        if buttons != self.buttons {
            let p = vm.dev_mut::<ControllerPorts>();
            self.buttons = buttons;
            p.button = buttons;
            Some(Event {
                vector: p.vector.get(),
                data: None,
            })
        } else {
            None
        }
    }
}
