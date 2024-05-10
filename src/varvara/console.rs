use crate::{
    uxn::{Device, Uxn},
    varvara::Event,
};
use std::{
    io::{Read, Write},
    sync::mpsc,
};

pub struct Console;

mod port {
    pub const VECTOR_0: u8 = 0x10;
    pub const VECTOR_1: u8 = 0x11;
    pub const READ: u8 = 0x12;
    pub const _EXEC: u8 = 0x13;
    pub const _MODE: u8 = 0x14;
    pub const _DEAD: u8 = 0x15;
    pub const _EXIT: u8 = 0x16;
    pub const TYPE: u8 = 0x17;
    pub const WRITE: u8 = 0x18;
    pub const ERROR: u8 = 0x19;
}

impl Device for Console {
    fn deo(&mut self, vm: &mut Uxn, target: u8) {
        match target {
            port::WRITE => {
                let v = vm.dev_read(target);
                let mut out = std::io::stdout().lock();
                out.write_all(&[v]).unwrap();
                out.flush().unwrap();
            }
            port::ERROR => {
                let v = vm.dev_read(target);
                let mut out = std::io::stderr().lock();
                out.write_all(&[v]).unwrap();
                out.flush().unwrap();
            }
            _ => (),
        }
    }
    fn dei(&mut self, _vm: &mut Uxn, _target: u8) {
        // Nothing to do here; data is pre-populated in `vm.dev` memory
    }
}

impl Console {
    pub fn new(tx: mpsc::Sender<Event>) -> Self {
        std::thread::spawn(move || {
            let mut i = std::io::stdin().lock();
            let mut buf = [0u8; 32];
            loop {
                let n = i.read(&mut buf).unwrap();
                for &c in &buf[..n] {
                    if tx.send(Event::Console(c)).is_err() {
                        return;
                    }
                }
            }
        });
        Self
    }

    /// Reads the `vector` value from VM device memory
    fn vector(&self, vm: &Uxn) -> u16 {
        let hi = vm.dev_read(port::VECTOR_0);
        let lo = vm.dev_read(port::VECTOR_1);
        u16::from_be_bytes([hi, lo])
    }

    /// Checks whether a callback is ready
    pub fn event(&mut self, vm: &mut Uxn, c: u8) -> u16 {
        vm.dev_write(port::READ, c);
        vm.dev_write(port::TYPE, 1);
        self.vector(vm)
    }
}
