use log::info;
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

struct OpenFile {
    name: std::path::PathBuf,
    file: std::fs::File,
}

pub struct File {
    f: Option<OpenFile>,
}

impl File {
    pub fn new() -> Self {
        Self { f: None }
    }

    pub fn dei(&mut self, vm: &mut Uxn, target: u8) {
        info!("file dei: {target:2x}");
    }

    pub fn deo(&mut self, vm: &mut Uxn, target: u8) {
        info!("file deo: {target:2x}");
    }
}
