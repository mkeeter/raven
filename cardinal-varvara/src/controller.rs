use crate::{Event, EventData};
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
    /// Keys that are currently held down
    down: HashSet<Key>,

    /// Current button state
    buttons: u8,
}

/// Key input to the controller device
#[allow(missing_docs)]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum Key {
    Shift,
    Ctrl,
    Alt,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    Char(u8),
}

impl Controller {
    /// Builds a new controller with no keys held
    pub fn new() -> Self {
        Self::default()
    }

    /// Sends a single character event
    pub fn char(&mut self, vm: &mut Uxn, c: u8) -> Event {
        let p = vm.dev::<ControllerPorts>();
        Event {
            vector: p.vector.get(),
            data: Some(EventData {
                addr: ControllerPorts::KEY,
                value: c,
                clear: true,
            }),
        }
    }

    /// Send the given key event, returning an event if needed
    pub fn pressed(
        &mut self,
        vm: &mut Uxn,
        k: Key,
        repeat: bool,
    ) -> Option<Event> {
        if let Key::Char(k) = k {
            Some(self.char(vm, k))
        } else {
            self.down.insert(k);
            self.check_buttons(vm, repeat)
        }
    }

    /// Indicate that the given key has been released
    ///
    /// This may change our button state and return an event
    pub fn released(&mut self, vm: &mut Uxn, k: Key) -> Option<Event> {
        if !matches!(k, Key::Char(..)) {
            self.down.remove(&k);
            self.check_buttons(vm, false)
        } else {
            None
        }
    }

    fn check_buttons(&mut self, vm: &mut Uxn, repeat: bool) -> Option<Event> {
        let mut buttons = 0;
        for (i, k) in [
            Key::Ctrl,
            Key::Alt,
            Key::Shift,
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

        // We'll return this event in case we don't have a keypress event;
        // otherwise, the keypress event will call the vector (at least once)
        if buttons != self.buttons || repeat {
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
