//! The Varvara computer system
use log::warn;

mod console;
mod system;

#[cfg(feature = "gui")]
mod screen;

#[cfg(feature = "gui")]
mod mouse;

#[cfg(feature = "gui")]
mod window;

mod datetime;
mod file;

use uxn::{Device, Ports, Uxn};

/// Handle to the Varvara system
pub struct Varvara {
    system: system::System,
    console: console::Console,
    #[cfg(feature = "gui")]
    window: window::Window,
    datetime: datetime::Datetime,

    rx: std::sync::mpsc::Receiver<Event>,

    already_warned: [bool; 16],
}

impl Default for Varvara {
    fn default() -> Self {
        Self::new()
    }
}

enum Event {
    Console(u8),
    #[cfg(feature = "gui")]
    Window,
}

impl Device for Varvara {
    fn deo(&mut self, vm: &mut Uxn, target: u8) {
        match target & 0xF0 {
            system::SystemPorts::BASE => self.system.deo(vm, target),
            console::ConsolePorts::BASE => self.console.deo(vm, target),
            #[cfg(feature = "gui")]
            screen::ScreenPorts::BASE => self.window.screen.deo(vm, target),
            datetime::DatetimePorts::BASE => self.datetime.deo(vm, target),
            mouse::MousePorts::BASE => self.window.set_mouse(),

            t => self.warn_missing(t),
        }
    }
    fn dei(&mut self, vm: &mut Uxn, target: u8) {
        match target & 0xF0 {
            system::SystemPorts::BASE => self.system.dei(vm, target),
            console::ConsolePorts::BASE => self.console.dei(vm, target),
            #[cfg(feature = "gui")]
            screen::ScreenPorts::BASE => self.window.screen.dei(vm, target),
            datetime::DatetimePorts::BASE => self.datetime.dei(vm, target),
            mouse::MousePorts::BASE => self.window.set_mouse(),

            t => self.warn_missing(t),
        }
    }
}

impl Varvara {
    pub fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            console: console::Console::new(tx.clone()),
            system: system::System::default(),
            #[cfg(feature = "gui")]
            window: window::Window::new(tx.clone()),
            datetime: datetime::Datetime,
            rx,
            already_warned: [false; 16],
        }
    }

    fn warn_missing(&mut self, t: u8) {
        if !self.already_warned[(t >> 4) as usize] {
            warn!("unimplemented device {t:#02x}");
            self.already_warned[(t >> 4) as usize] = true;
        }
    }

    /// Runs in a wait-loop
    pub fn run(&mut self, vm: &mut Uxn) {
        while let Ok(e) = self.rx.recv() {
            match e {
                Event::Console(c) => {
                    let vector = self.console.event(vm, c);
                    vm.run(self, vector);
                }
                #[cfg(feature = "gui")]
                Event::Window => {
                    for v in self.window.event(vm) {
                        vm.run(self, v);
                    }
                }
            }

            #[cfg(feature = "gui")]
            if !self.window.update(vm) {
                break;
            }
        }
    }
}
