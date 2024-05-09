//! The Varvara computer system
mod console;
mod system;

use crate::uxn::{Device, Uxn};

/// Handle to the Varvara system
#[derive(Default)]
pub struct Varvara {
    system: system::System,
    console: console::Console,
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
