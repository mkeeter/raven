use crate::Event;
use raven::{Ports, Uxn};
use std::{
    io::{Read, Write},
    mem::offset_of,
    sync::mpsc,
};
use zerocopy::{AsBytes, BigEndian, FromBytes, FromZeroes, U16};

pub struct Console {
    rx: mpsc::Receiver<u8>,
}

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
    const WRITE: u8 = Self::BASE | offset_of!(Self, write) as u8;
    const ERROR: u8 = Self::BASE | offset_of!(Self, error) as u8;
}

impl Console {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
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

    /// Checks whether a callback is ready
    #[must_use]
    pub fn event(&mut self, vm: &mut Uxn, c: u8) -> Event {
        let p = vm.dev_mut::<ConsolePorts>();
        p.read = c;
        p.type_ = 1;
        let vector = p.vector.get();
        Event { vector, data: None }
    }

    pub fn deo(&mut self, vm: &mut Uxn, target: u8) {
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
    pub fn dei(&mut self, _vm: &mut Uxn, _target: u8) {
        // Nothing to do here; data is pre-populated in `vm.dev` memory
    }

    #[cfg(feature = "gui")]
    #[must_use]
    pub fn poll(&mut self, vm: &mut Uxn) -> Option<Event> {
        self.rx.try_recv().map(|c| self.event(vm, c)).ok()
    }

    #[cfg(not(feature = "gui"))]
    #[must_use]
    pub fn block(&mut self, vm: &mut Uxn) -> Option<Event> {
        self.rx.try_recv().map(|c| self.event(vm, c)).ok()
    }
}
