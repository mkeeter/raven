//! The Varvara computer system
mod console;
mod system;

#[cfg(feature = "screen")]
mod screen;

use uxn::{Device, Ports, Uxn};

/// Handle to the Varvara system
pub struct Varvara {
    system: system::System,
    console: console::Console,
    #[cfg(feature = "screen")]
    screen: screen::Screen,
    rx: std::sync::mpsc::Receiver<Event>,
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
            screen::ScreenPorts::BASE => self.screen.deo(vm, target),
            _ => panic!("unimplemented device {target:#2x}"),
        }
    }
    fn dei(&mut self, vm: &mut Uxn, target: u8) {
        match target & 0xF0 {
            system::SystemPorts::BASE => self.system.dei(vm, target),
            console::ConsolePorts::BASE => self.console.dei(vm, target),
            #[cfg(feature = "screen")]
            screen::ScreenPorts::BASE => self.screen.dei(vm, target),
            _ => panic!("unimplemented device {target:#2x}"),
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
            screen: screen::Screen::new(tx.clone()),
            rx,
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
                    let vector = self.screen.event(vm);
                    vm.run(self, vector);
                    if !self.screen.update(vm) {
                        break;
                    }
                }
            }
        }
    }
}
