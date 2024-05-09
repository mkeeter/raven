use crate::uxn::{Device, Uxn};

#[derive(Default)]
pub struct System {}

impl Device for System {
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
