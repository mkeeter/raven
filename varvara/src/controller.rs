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

pub struct Controller;

impl Controller {
    pub fn event(&mut self, key: Option<u8>) -> Option<u16> {
        None
    }
}