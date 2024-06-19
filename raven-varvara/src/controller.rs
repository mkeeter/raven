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
    LeftShift,
    RightShift,
    LeftCtrl,
    LeftAlt,
    Up,
    Down,
    Left,
    Right,
    LeftSuper,
    RightSuper,
    Home,
    Char(u8),
}

impl Controller {
    /// Builds a new controller with no keys held
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks whether either shift key is held
    pub fn shift_held(&self) -> bool {
        self.down.contains(&Key::LeftShift)
            | self.down.contains(&Key::RightShift)
    }

    /// Send the given key event, appending an event to the queue if needed
    pub fn pressed(&mut self, vm: &mut Uxn, k: Key, queue: &mut Vec<Event>) {
        self.down.insert(k);

        let e = match k {
            Key::Char(c) => {
                let p = vm.dev::<ControllerPorts>();
                Some(Event {
                    vector: p.vector.get(),
                    data: Some(EventData {
                        addr: ControllerPorts::KEY,
                        value: c,
                        clear: true,
                    }),
                })
            }
            _ => self.check_buttons(vm),
        };
        queue.extend(e);
    }

    /// Indicate that the given key has been released
    ///
    /// This may change our button state and push an [`Event`] to the queue
    pub fn released(&mut self, vm: &mut Uxn, k: Key, queue: &mut Vec<Event>) {
        self.down.remove(&k);
        queue.extend(self.check_buttons(vm));
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
