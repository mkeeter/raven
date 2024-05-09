use crate::uxn::{Device, Uxn};
use std::io::Write;

#[derive(Default)]
pub struct Console {}

impl Device for Console {
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
