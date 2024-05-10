//! The Varvara computer system
mod console;
mod system;

use uxn::{Device, Uxn};

/// Handle to the Varvara system
pub struct Varvara {
    system: system::System,
    console: console::Console,
    rx: std::sync::mpsc::Receiver<Event>,
}

impl Default for Varvara {
    fn default() -> Self {
        Self::new()
    }
}

enum Event {
    Console(u8),
}

impl Device for Varvara {
    fn deo(&mut self, vm: &mut Uxn, target: u8) {
        match target & 0xF0 {
            0x00 => self.system.deo(vm, target),
            0x10 => self.console.deo(vm, target),
            _ => panic!("unimplemented device {target:#2x}"),
        }
    }
    fn dei(&mut self, vm: &mut Uxn, target: u8) {
        match target & 0xF0 {
            0x00 => self.system.dei(vm, target),
            0x10 => self.console.dei(vm, target),
            _ => panic!("unimplemented device {target:#2x}"),
        }
    }
}

impl Varvara {
    pub fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let console = console::Console::new(tx.clone());
        Self {
            console,
            system: system::System::default(),
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
            }
        }
    }
}
