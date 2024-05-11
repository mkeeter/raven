//! The Varvara computer system
use log::warn;

mod console;
mod system;

#[cfg(feature = "screen")]
mod screen;

#[cfg(feature = "screen")]
mod window;

#[cfg(feature = "file")]
mod file;

use uxn::{Device, Ports, Uxn};

/// Handle to the Varvara system
pub struct Varvara {
    system: system::System,
    console: console::Console,
    #[cfg(feature = "screen")]
    window: window::Window,
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
    #[cfg(feature = "screen")]
    Screen,
}

impl Device for Varvara {
    fn deo(&mut self, vm: &mut Uxn, target: u8) {
        match target & 0xF0 {
            system::SystemPorts::BASE => self.system.deo(vm, target),
            console::ConsolePorts::BASE => self.console.deo(vm, target),
            #[cfg(feature = "screen")]
            screen::ScreenPorts::BASE => self.window.screen.deo(vm, target),

            t => self.warn_missing(t),
        }
    }
    fn dei(&mut self, vm: &mut Uxn, target: u8) {
        match target & 0xF0 {
            system::SystemPorts::BASE => self.system.dei(vm, target),
            console::ConsolePorts::BASE => self.console.dei(vm, target),
            #[cfg(feature = "screen")]
            screen::ScreenPorts::BASE => self.window.screen.dei(vm, target),

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
            #[cfg(feature = "screen")]
            window: window::Window::new(tx.clone()),
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
                #[cfg(feature = "screen")]
                Event::Screen => {
                    let vector = self.window.screen.event(vm);
                    vm.run(self, vector);
                }
            }

            #[cfg(feature = "screen")]
            if !self.window.update(vm) {
                break;
            }
        }
    }
}
