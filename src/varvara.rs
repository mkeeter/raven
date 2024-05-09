//! The Varvara computer system
use crate::uxn::{Device, Uxn};
use std::io::Write;

/// Handle to the Varvara system
#[derive(Default)]
pub struct Varvara {
    system: System,
    console: Console,
}

#[derive(Default)]
struct System {}

impl System {
    fn deo(&mut self, _vm: &mut Uxn, target: u8) {
        match target & 0x0F {
            0x08..=0x0d => (), // colors
            _ => panic!("unimplemented system DEO call: {target}"),
        }
    }
    fn dei(&mut self, _vm: &mut Uxn, target: u8) {
        match target & 0x0F {
            0x08..=0x0d => (), // colors
            _ => panic!("unimplemented system DEI call: {target}"),
        }
    }
}

#[derive(Default)]
struct Console {}

impl Console {
    fn deo(&mut self, vm: &mut Uxn, target: u8) {
        match target & 0x0F {
            0x00..=0x01 => (), // vector
            0x08 => {
                let v = vm.dev_read(target);
                let mut out = std::io::stdout().lock();
                out.write_all(&[v]).unwrap();
                out.flush().unwrap();
            }
            0x09 => {
                let v = vm.dev_read(target);
                let mut out = std::io::stderr().lock();
                out.write_all(&[v]).unwrap();
                out.flush().unwrap();
            }

            _ => panic!("unimplemented console DEO call: {target:#2x}"),
        }
    }
    fn dei(&mut self, _vm: &mut Uxn, target: u8) {
        match target & 0x0F {
            0x07 => (), // TODO
            _ => panic!("unimplemented console DEI call: {target:#2x}"),
        }
    }
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
