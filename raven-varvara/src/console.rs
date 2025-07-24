impl Default for Console {
    fn default() -> Self {
        Self::new()
    }
}
use crate::{Event, EventData};
use std::mem::offset_of;
use uxn::{Ports, Uxn};
use zerocopy::{AsBytes, BigEndian, FromBytes, FromZeroes, U16};

pub struct Console {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    stdout_listeners: Vec<Box<dyn FnMut(u8) + Send>>,
    stderr_listeners: Vec<Box<dyn FnMut(u8) + Send>>,
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
///
/// # Panics
/// If threads are not available on the system (e.g. in WebAssembly)
pub fn spawn_worker<F, E>(mut tx: F)
where
    F: FnMut(u8) -> Result<(), E> + Send + 'static,
{
    use std::io::Read;
    std::thread::spawn(move || {
        let mut i = std::io::stdin().lock();
        let mut buf = [0u8; 32];
        loop {
            let n = i.read(&mut buf).unwrap();
            for &c in &buf[..n] {
                if tx(c).is_err() {
                    return;
                }
            }
        }
    });
}

impl Console {
    pub fn new() -> Self {
        Self {
            stdout: vec![],
            stderr: vec![],
            stdout_listeners: vec![],
            stderr_listeners: vec![],
        }
    }

    /// Register a callback to receive bytes written to stderr
    pub fn register_stderr_listener<F>(&mut self, listener: F)
    where
        F: FnMut(u8) + Send + 'static,
    {
        self.stderr_listeners.push(Box::new(listener));
    }
    /// Register a callback to receive bytes written to stdout
    pub fn register_stdout_listener<F>(&mut self, listener: F)
    where
        F: FnMut(u8) + Send + 'static,
    {
        self.stdout_listeners.push(Box::new(listener));
    }
    pub fn deo(&mut self, vm: &mut Uxn, target: u8) {
        let v = vm.dev::<ConsolePorts>();
        match target {
            ConsolePorts::WRITE => {
                self.stdout.push(v.write);
                for listener in &mut self.stdout_listeners {
                    listener(v.write);
                }
            }
            ConsolePorts::ERROR => {
                self.stderr.push(v.error);
                for listener in &mut self.stderr_listeners {
                    listener(v.error);
                }
            }
            _ => (),
        }
    }
    pub fn dei(&mut self, _vm: &mut Uxn, _target: u8) {
        // Nothing to do here; data is pre-populated in `vm.dev` memory
    }

    /// Sets the appropriate type value if there are arguments to be parsed
    ///
    /// This should be called before running the reset vector
    pub fn set_has_args(&mut self, vm: &mut Uxn, has_args: bool) {
        if has_args {
            let p = vm.dev_mut::<ConsolePorts>();
            p.type_ = 1;
        }
    }

    /// Sets the current character type
    ///
    /// This should be called before sending a console event
    pub fn set_type(&mut self, vm: &mut Uxn, ty: Type) {
        let p = vm.dev_mut::<ConsolePorts>();
        p.type_ = ty as u8;
    }

    /// Returns an event that sets the given character and calls the vector
    ///
    /// Note that this function does not set the type, which should be
    /// configured by calling [`Self::set_type`] before firing the vector.
    pub fn update(&self, vm: &Uxn, c: u8) -> Event {
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
