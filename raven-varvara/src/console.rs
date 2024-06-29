use crate::{Event, EventData};
use std::mem::offset_of;
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

#[derive(Copy, Clone, Debug)]
pub enum Type {
    #[allow(unused)]
    NoQueue = 0,
    Stdin = 1,
    Argument = 2,
    ArgumentSpacer = 3,
    ArgumentEnd = 4,
}

impl Ports for ConsolePorts {
    const BASE: u8 = 0x10;
}

impl ConsolePorts {
    const READ: u8 = Self::BASE | offset_of!(Self, read) as u8;
    const WRITE: u8 = Self::BASE | offset_of!(Self, write) as u8;
    const ERROR: u8 = Self::BASE | offset_of!(Self, error) as u8;
}

/// Spawns a worker thread that listens on `stdin` and emits characters
#[cfg(not(target_arch = "wasm32"))]
pub fn worker() -> std::sync::mpsc::Receiver<u8> {
    use std::io::Read;
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
    rx
}

impl Console {
    pub fn new() -> Self {
        Self {
            stdout: vec![],
            stderr: vec![],
        }
    }

    pub fn deo<U: Uxn>(&mut self, vm: &mut U, target: u8) {
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
    pub fn dei<U>(&mut self, _vm: &mut U, _target: u8) {
        // Nothing to do here; data is pre-populated in `vm.dev` memory
    }

    /// Sets the current character type
    ///
    /// This should be called before sending a console event
    pub fn set_type<U: Uxn>(&mut self, vm: &mut U, ty: Type) {
        let p = vm.dev_mut::<ConsolePorts>();
        p.type_ = ty as u8;
    }

    /// Returns an event that sets the given character and calls the vector
    ///
    /// Note that this function does not set the type, which should be
    /// configured by calling [`Self::set_type`] before firing the vector.
    pub fn update<U: Uxn>(&self, vm: &U, c: u8) -> Event {
        let p = vm.dev::<ConsolePorts>();
        let vector = p.vector.get();
        Event {
            vector,
            data: Some(EventData {
                addr: ConsolePorts::READ,
                value: c,
                clear: false,
            }),
        }
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
