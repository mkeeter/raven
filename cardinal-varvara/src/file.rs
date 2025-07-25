use log::{error, trace, warn};
use std::{
    collections::{HashSet, VecDeque},
    io::{Read, Write},
    mem::offset_of,
};
use uxn::{Ports, Uxn, DEV_SIZE};
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
        match String::from_utf8(out) {
            Ok(s) => Some(s),
            Err(e) => {
                error!("could not read filename from VM: {e}");
                None
            }
        }
    }

    /// Checks whether the given value is in the file ports memory space
    pub fn matches(t: u8) -> bool {
        (Self::BASE..Self::BASE + 0x20).contains(&t)
    }

    fn dev<'a>(vm: &'a Uxn, i: usize) -> &'a Self {
        let pos = Self::BASE + (i * DEV_SIZE) as u8;
        vm.dev_at(pos)
    }

    fn dev_mut<'a>(vm: &'a mut Uxn, i: usize) -> &'a mut Self {
        let pos = Self::BASE + (i * DEV_SIZE) as u8;
        vm.dev_mut_at(pos)
    }
}

impl FilePorts {
    const NAME_H: u8 = offset_of!(Self, name) as u8;
    const NAME_L: u8 = Self::NAME_H + 1;
    const LENGTH_H: u8 = offset_of!(Self, length) as u8;
    const LENGTH_L: u8 = Self::LENGTH_H + 1;
    const READ_H: u8 = offset_of!(Self, read) as u8;
    const READ_L: u8 = Self::READ_H + 1;
    const WRITE_H: u8 = offset_of!(Self, write) as u8;
    const WRITE_L: u8 = Self::WRITE_H + 1;
    const APPEND: u8 = offset_of!(Self, append) as u8;
    const DELETE: u8 = offset_of!(Self, delete) as u8;
}

#[cfg_attr(target_os = "windows", allow(clippy::large_enum_variant))]
enum Handle {
    File {
        path: std::path::PathBuf,
        file: std::fs::File,
    },
    Dir {
        path: std::path::PathBuf,
        dir: std::fs::ReadDir, // weirdly huge (616 bytes) on Windows!

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

impl Default for File {
    fn default() -> Self {
        Self::new()
    }
}

impl File {
    pub fn new() -> Self {
        Self {
            f: None,
            buf: vec![],
            missing_files: HashSet::new(),
        }
    }

    /// Decodes a port address into an `(index, offset)` tuple
    fn decode_target(target: u8) -> (usize, u8) {
        let i = usize::from(target - FilePorts::BASE) / DEV_SIZE;
        (i, target & 0xF)
    }

    pub fn deo(&mut self, vm: &mut Uxn, target: u8) {
        let (i, target) = Self::decode_target(target);
        match target {
            FilePorts::DELETE => self.delete(vm, i),
            FilePorts::APPEND => (), // Ignored, this sets the append flag
            FilePorts::NAME_H | FilePorts::NAME_L => {
                self.f = None;
            }
            FilePorts::LENGTH_H | FilePorts::LENGTH_L => {
                // Ignored, this sets the buffer length
            }
            FilePorts::READ_H => (), // ignored, action is on READ_L
            FilePorts::READ_L => self.read(vm, i),
            FilePorts::WRITE_H => (), // ignored, action is on WRITE_L
            FilePorts::WRITE_L => self.write(vm, i),

            _ => warn!("unknown file deo: {target:2x}"),
        }
    }

    /// Checks that the given path is local and does not escape our working dir
    ///
    /// Note that this simply checks depth; symlinks must be examined separately
    fn is_path_local(path: &std::path::Path) -> bool {
        let mut depth = 0;
        for component in path.components() {
            match component {
                std::path::Component::Prefix(..)
                | std::path::Component::RootDir => {
                    error!("path {path:?} is not relative");
                    return false;
                }
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir => {
                    if depth == 0 {
                        error!("path {path:?} escapes working directory");
                        return false;
                    } else {
                        depth -= 1;
                    }
                }
                std::path::Component::Normal(..) => {
                    depth += 1;
                }
            }
        }
        true
    }

    fn delete(&mut self, vm: &mut Uxn, index: usize) {
        // Close the file, if it happens to be open
        self.f = None;

        // Set the return flag to -1
        FilePorts::dev_mut(vm, index).success.set(u16::MAX);

        let ports = FilePorts::dev(vm, index);
        let Some(filename) = ports.filename(vm) else {
            return;
        };
        let path = std::path::PathBuf::from(&filename);
        if !Self::is_path_local(&path) {
            return;
        }
        if std::fs::remove_file(&path).is_ok() {
            FilePorts::dev_mut(vm, index).success.set(0);
        };
    }

    fn write(&mut self, vm: &mut Uxn, index: usize) {
        // Clear the success flag
        let ports = FilePorts::dev_mut(vm, index);
        ports.success.set(0);

        let ports = FilePorts::dev(vm, index);
        if !matches!(self.f, Some(Handle::Write { .. })) {
            let Some(filename) = ports.filename(vm) else {
                return;
            };
            let path = std::path::PathBuf::from(&filename);
            if !Self::is_path_local(&path) {
                error!("path {path:?} escapes working directory");
                return;
            }

            let file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
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
                trace!("opened {path:?} as file for writing");
                self.f = Some(Handle::Write { path, file });
            }
        }

        self.buf.resize(usize::from(ports.length.get()), 0u8);
        self.buf.fill(0u8);
        let Some(Handle::Write { path, file }) = self.f.as_mut() else {
            unreachable!();
        };

        // Copy data out of the VM
        self.buf.resize(usize::from(ports.length.get()), 0u8);
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
        let ports = FilePorts::dev_mut(vm, index);
        ports.success.set(n as u16);
    }

    fn read(&mut self, vm: &mut Uxn, index: usize) {
        // Clear the success flag
        let ports = FilePorts::dev_mut(vm, index);
        ports.success.set(0);

        if !matches!(self.f, Some(Handle::File { .. } | Handle::Dir { .. })) {
            let ports = FilePorts::dev(vm, index);
            let Some(filename) = ports.filename(vm) else {
                return;
            };
            let path = std::path::PathBuf::from(&filename);
            if !path.exists() {
                if self.missing_files.insert(filename.to_owned()) {
                    error!("{filename:?} is missing");
                }
                return;
            }
            let path = std::path::PathBuf::from(&filename);
            if !Self::is_path_local(&path) {
                error!("path {path:?} escapes working directory");
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
                trace!("opened {path:?} as dir for reading");
                self.f = Some(Handle::Dir {
                    path,
                    dir,
                    scratch: Default::default(),
                });
            } else {
                trace!("opened {path:?} as file for reading");
                self.f = Some(Handle::File { path, file });
            }
        }

        let ports = FilePorts::dev_mut(vm, index);
        self.buf.resize(usize::from(ports.length.get()), 0u8);
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
        for &b in &self.buf {
            vm.ram_write_byte(addr, b);
            addr = addr.wrapping_add(1);
        }
    }
}
