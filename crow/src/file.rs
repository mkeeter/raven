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
