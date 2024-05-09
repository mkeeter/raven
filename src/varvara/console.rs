use crate::uxn::{Device, Uxn};
use std::io::{Read, Write};

pub struct Console {
    rx: std::sync::mpsc::Receiver<u8>,
}

impl Default for Console {
    fn default() -> Self {
        Self::new()
    }
}

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
            0x02 => (), // read
            _ => panic!("unimplemented console DEI call: {target:#2x}"),
        }
    }
}

impl Console {
    fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut i = std::io::stdin().lock();
            let mut buf = [0u8; 32];
            loop {
                let n = i.read(&mut buf).unwrap();
                for &c in &buf[..n] {
                    if tx.send(c).is_err() {
                        return;
                    }
                }
            }
        });
        Self { rx }
    }

    /// Reads the `vector` value from VM device memory
    fn vector(&self, vm: &Uxn) -> u16 {
        let hi = vm.dev_read(0x10);
        let lo = vm.dev_read(0x11);
        u16::from_be_bytes([hi, lo])
    }

    /// Checks whether a callback is ready
    pub fn ready(&mut self, vm: &mut Uxn) -> Option<u16> {
        // TODO error handling?
        if let Ok(c) = self.rx.try_recv() {
            vm.dev_write(0x12, c);
            vm.dev_write(0x17, 1);
            Some(self.vector(vm))
        } else {
            None
        }
    }
}
