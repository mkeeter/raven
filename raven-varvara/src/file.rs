use log::{error, trace, warn};
use std::{
    collections::{HashSet, VecDeque},
    io::{Read, Write},
    mem::offset_of,
};
use uxn::{Ports, Uxn};
use zerocopy::{AsBytes, BigEndian, FromBytes, FromZeroes, U16};

#[derive(AsBytes, FromZeroes, FromBytes)]
#[repr(C)]
pub struct FilePorts {
    _vector: U16<BigEndian>,
    success: U16<BigEndian>,
    stat: U16<BigEndian>,
    delete: u8,
    append: u8,
    name: U16<BigEndian>,
    length: U16<BigEndian>,
    read: U16<BigEndian>,
    write: U16<BigEndian>,
}

impl Ports for FilePorts {
    const BASE: u8 = 0xa0;
}

impl FilePorts {
    /// Gets the filename from the memory address
    ///
    /// Logs an error and returns `None` if anything goes wrong
    fn filename(&self, vm: &Uxn) -> Option<String> {
        // TODO return a slice here instead?
        let mut addr = self.name.get();
        let mut out = vec![];
        while out.last() != Some(&0) {
            out.push(vm.ram_read_byte(addr));
            addr = addr.wrapping_add(1);
        }
        out.pop();
        String::from_utf8(out).ok()
    }
}

impl FilePorts {
    const NAME_H: u8 = Self::BASE | offset_of!(Self, name) as u8;
    const NAME_L: u8 = Self::NAME_H + 1;
    const LENGTH_H: u8 = Self::BASE | offset_of!(Self, length) as u8;
    const LENGTH_L: u8 = Self::LENGTH_H + 1;
    const READ_H: u8 = Self::BASE | offset_of!(Self, read) as u8;
    const READ_L: u8 = Self::READ_H + 1;
    const WRITE_H: u8 = Self::BASE | offset_of!(Self, write) as u8;
    const WRITE_L: u8 = Self::WRITE_H + 1;
}

enum Handle {
    File {
        path: std::path::PathBuf,
        file: std::fs::File,
    },
    Dir {
        path: std::path::PathBuf,
        dir: std::fs::ReadDir,

        /// Buffer of left-over characters to write
        scratch: VecDeque<u8>,
    },
    Write {
        path: std::path::PathBuf,
        file: std::fs::File,
    },
}

pub struct File {
    f: Option<Handle>,

    /// Scratch buffer
    buf: Vec<u8>,

    /// Log of missing files, to avoid spamming warnings
    missing_files: HashSet<String>,
}

impl File {
    pub fn new() -> Self {
        Self {
            f: None,
            buf: vec![],
            missing_files: HashSet::new(),
        }
    }

    pub fn deo(&mut self, vm: &mut Uxn, target: u8) {
        match target {
            FilePorts::READ_H => (), // ignored, action is on READ_L
            FilePorts::READ_L => self.read(vm),
            FilePorts::WRITE_H => (), // ignored, action is on WRITE_L
            FilePorts::WRITE_L => self.write(vm),
            FilePorts::NAME_H | FilePorts::NAME_L => {
                self.f = None;
            }
            FilePorts::LENGTH_H | FilePorts::LENGTH_L => {
                // Ignored, this sets the buffer length
            }
            _ => warn!("unknown file deo: {target:2x}"),
        }
    }

    fn write(&mut self, vm: &mut Uxn) {
        // Clear the success flag
        let ports = vm.dev_mut::<FilePorts>();
        ports.success.set(0);

        let ports = vm.dev::<FilePorts>();
        if !matches!(self.f, Some(Handle::Write { .. })) {
            let Some(filename) = ports.filename(vm) else {
                error!("could not read filename");
                return;
            };
            let path = std::path::PathBuf::from(&filename);
            if !path.exists() {
                if self.missing_files.insert(filename.to_owned()) {
                    error!("{filename:?} is missing");
                }
                return;
            }
            let path = match path.canonicalize() {
                Ok(p) => p,
                Err(e) => {
                    error!("could not canonicalize path {filename:?}: {e}");
                    return;
                }
            };
            let pwd = match std::env::current_dir() {
                Ok(f) => f,
                Err(e) => {
                    error!("could not get pwd: {e}");
                    return;
                }
            };
            if !path.starts_with(&pwd) {
                warn!(
                    "requested path {path:?} is outside of
                     working directory {pwd:?}"
                );
                return;
            }

            let file = std::fs::OpenOptions::new()
                .append(ports.append == 0x1)
                .open(&path);
            let file = match file {
                Ok(f) => f,
                Err(e) => {
                    error!("could not open {path:?}: {e}");
                    return;
                }
            };
            let m = match file.metadata() {
                Ok(m) => m,
                Err(e) => {
                    error!("could not check metadata for {path:?}: {e}");
                    return;
                }
            };
            if m.is_symlink() {
                warn!("{path:?} is a symlink; skipping");
                return;
            } else if m.is_dir() {
                warn!("{path:?} is a directory; skipping");
                return;
            } else {
                trace!("opened {path:?} as file");
                self.f = Some(Handle::Write { path, file });
            }
        }

        self.buf.resize(ports.length.get() as usize, 0u8);
        let Some(Handle::Write { path, file }) = self.f.as_mut() else {
            unreachable!();
        };

        // Copy data out of the VM
        self.buf.resize(ports.length.get() as usize, 0u8);
        let mut addr = ports.write.get();
        for b in self.buf.iter_mut() {
            *b = vm.ram_read_byte(addr);
            addr = addr.wrapping_add(1);
        }

        let n = match file.write(&self.buf) {
            Ok(n) => n,
            Err(e) => {
                error!("could not write to {path:?}: {e}");
                return;
            }
        };
        if n != self.buf.len() {
            error!("could not write all bytes to file");
            return;
        }
        let ports = vm.dev_mut::<FilePorts>();
        ports.success.set(n as u16);
    }

    fn read(&mut self, vm: &mut Uxn) {
        // Clear the success flag
        let ports = vm.dev_mut::<FilePorts>();
        ports.success.set(0);

        if !matches!(self.f, Some(Handle::File { .. } | Handle::Dir { .. })) {
            let ports = vm.dev::<FilePorts>();
            let Some(filename) = ports.filename(vm) else {
                error!("could not read filename");
                return;
            };
            let path = std::path::PathBuf::from(&filename);
            if !path.exists() {
                if self.missing_files.insert(filename.to_owned()) {
                    error!("{filename:?} is missing");
                }
                return;
            }
            let path = match path.canonicalize() {
                Ok(p) => p,
                Err(e) => {
                    error!("could not canonicalize path {filename:?}: {e}");
                    return;
                }
            };
            let pwd = match std::env::current_dir() {
                Ok(f) => f,
                Err(e) => {
                    error!("could not get pwd: {e}");
                    return;
                }
            };
            if !path.starts_with(&pwd) {
                warn!(
                    "requested path {path:?} is outside of
                     working directory {pwd:?}"
                );
                return;
            }

            let file = match std::fs::File::open(&path) {
                Ok(f) => f,
                Err(e) => {
                    error!("could not open {path:?}: {e}");
                    return;
                }
            };
            let m = match file.metadata() {
                Ok(m) => m,
                Err(e) => {
                    error!("could not check metadata for {path:?}: {e}");
                    return;
                }
            };
            if m.is_symlink() {
                warn!("{path:?} is a symlink; skipping");
                return;
            } else if m.is_dir() {
                let dir = match std::fs::read_dir(&path) {
                    Ok(d) => d,
                    Err(e) => {
                        error!("could not open dir for {path:?}: {e}");
                        return;
                    }
                };
                self.f = Some(Handle::Dir {
                    path,
                    dir,
                    scratch: Default::default(),
                });
            } else {
                trace!("opened {path:?} as file");
                self.f = Some(Handle::File { path, file });
            }
        }

        let ports = vm.dev_mut::<FilePorts>();
        self.buf.resize(ports.length.get() as usize, 0u8);
        let n = match self.f.as_mut().unwrap() {
            Handle::Write { .. } => unreachable!(),
            Handle::File { path, file } => match file.read(&mut self.buf) {
                Ok(n) => n,
                Err(e) => {
                    error!("failed to read file at {path:?}: {e}");
                    return;
                }
            },
            Handle::Dir { path, dir, scratch } => {
                let mut n = 0;
                while n != self.buf.len() {
                    // Send any pending characters
                    while n < self.buf.len() {
                        let Some(c) = scratch.pop_front() else {
                            break;
                        };
                        self.buf[n] = c;
                        n += 1;
                    }
                    // Preload new data into the buffer
                    if n < self.buf.len() && scratch.is_empty() {
                        let Some(next) = dir.next() else {
                            break;
                        };
                        match next {
                            Ok(d) => {
                                let m = match d.metadata() {
                                    Ok(m) => m,
                                    Err(e) => {
                                        error!(
                                            "could not get entry metadata: {e}"
                                        );
                                        return;
                                    }
                                };
                                let size = if m.is_dir() {
                                    "----".to_owned()
                                } else if m.len() < u16::MAX as u64 {
                                    format!("{:04x}", m.len())
                                } else {
                                    "????".to_owned()
                                };
                                scratch.extend(size.bytes());
                                scratch.push_back(b' ');
                                let name = d.file_name();
                                scratch.extend(name.as_encoded_bytes());
                                scratch.push_back(b'\n');
                            }
                            Err(e) => {
                                error!(
                                    "error while iterating over {path:?}: {e}"
                                );
                                return;
                            }
                        }
                    }
                }
                n
            }
        };

        ports.success.set(n as u16);
        let mut addr = ports.read.get();
        for &b in &self.buf[..n] {
            vm.ram_write_byte(addr, b);
            addr = addr.wrapping_add(1);
        }
    }
}
