use crate::Event;
use std::{
    io::{Read, Write},
    sync::mpsc,
};
use uxn::{Device, Ports, Uxn};
use zerocopy::{AsBytes, BigEndian, FromBytes, FromZeroes, U16};

pub struct Console;

#[derive(AsBytes, FromZeroes, FromBytes)]
#[repr(C)]
pub struct ConsolePorts {
    vector: U16<BigEndian>,
    read: u8,
    _exec: u8,
    _mode: u8,
    _dead: u8,
    _exit: u8,
    type_: u8,
    write: u8,
    error: u8,
    _pad: [u8; 6],
}

impl Ports for ConsolePorts {
    const BASE: u8 = 0x10;
}

impl ConsolePorts {
    const WRITE: u8 = Self::BASE | std::mem::offset_of!(Self, write) as u8;
    const ERROR: u8 = Self::BASE | std::mem::offset_of!(Self, error) as u8;
}

impl Device for Console {
    fn deo(&mut self, vm: &mut Uxn, target: u8) {
        let v = vm.dev::<ConsolePorts>();
        match target {
            ConsolePorts::WRITE => {
                let mut out = std::io::stdout().lock();
                out.write_all(&[v.write]).unwrap();
                out.flush().unwrap();
            }
            ConsolePorts::ERROR => {
                let mut out = std::io::stderr().lock();
                out.write_all(&[v.write]).unwrap();
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

    /// Checks whether a callback is ready
    pub fn event(&mut self, vm: &mut Uxn, c: u8) -> u16 {
        let p = vm.dev_mut::<ConsolePorts>();
        p.read = c;
        p.type_ = 1;
        p.vector.get()
    }
}
