use crate::Event;
use std::{collections::VecDeque, io::Read, mem::offset_of, sync::mpsc};
use uxn::{Ports, Uxn};
use zerocopy::{AsBytes, BigEndian, FromBytes, FromZeroes, U16};

pub struct Console {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
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

/// Spawns a worker thread that listens on `stdin` and emits characters
pub fn worker() -> mpsc::Receiver<u8> {
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
    rx
}

impl Console {
    pub fn new() -> Self {
        Self {
            stdout: vec![],
            stderr: vec![],
        }
    }

    pub fn deo(&mut self, vm: &mut Uxn, target: u8) {
        let v = vm.dev::<ConsolePorts>();
        match target {
            ConsolePorts::WRITE => {
                self.stdout.push(v.write);
            }
            ConsolePorts::ERROR => {
                self.stderr.push(v.error);
            }
            _ => (),
        }
    }
    pub fn dei(&mut self, _vm: &mut Uxn, _target: u8) {
        // Nothing to do here; data is pre-populated in `vm.dev` memory
    }

    pub fn update(&mut self, vm: &mut Uxn, c: u8, queue: &mut VecDeque<Event>) {
        let p = vm.dev_mut::<ConsolePorts>();
        p.read = c;
        p.type_ = 1; // TODO arguments
        let vector = p.vector.get();
        queue.push_back(Event { vector, data: None })
    }

    /// Takes the `stderr` buffer, leaving it empty
    pub fn stdout(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.stdout)
    }

    /// Takes the `stderr` buffer, leaving it empty
    pub fn stderr(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.stderr)
    }
}
