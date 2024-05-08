//! The Varvara computer system
use crate::uxn::Device;
use zerocopy::AsBytes;

/// Handle to the Varvara system
#[derive(Default)]
pub struct Varvara {
    system: System,
}

#[derive(Default)]
struct System {
    red: u16,
    green: u16,
    blue: u16,
}

impl System {
    fn deo(&mut self, target: u8, value: u8) {
        match target & 0x0F {
            0x08 => self.red.as_bytes_mut()[1] = value,
            0x09 => self.red.as_bytes_mut()[0] = value,
            0x0a => self.green.as_bytes_mut()[0] = value,
            0x0b => self.green.as_bytes_mut()[1] = value,
            0x0c => self.blue.as_bytes_mut()[0] = value,
            0x0d => self.blue.as_bytes_mut()[1] = value,

            _ => panic!("unimplemented system call: {target}"),
        }
    }
    fn dei(&mut self, target: u8) -> u8 {
        match target & 0x0F {
            0x08 => self.red.to_be_bytes()[0],
            0x09 => self.red.to_be_bytes()[1],
            0x0a => self.green.to_be_bytes()[0],
            0x0b => self.green.to_be_bytes()[1],
            0x0c => self.blue.to_be_bytes()[0],
            0x0d => self.blue.to_be_bytes()[1],

            _ => panic!("unimplemented system call: {target}"),
        }
    }
}

impl Device for Varvara {
    fn deo(&mut self, target: u8, value: u8) {
        match target & 0xF0 {
            0x00 => self.system.deo(target, value),
            _ => panic!("unimplemented device {target:#2x}"),
        }
    }
    fn dei(&mut self, target: u8) -> u8 {
        match target & 0xF0 {
            0x00 => self.system.dei(target),
            _ => panic!("unimplemented device {target:#2x}"),
        }
    }
}
