use crate::uxn::{Device, Uxn};
use zerocopy::{BigEndian, FromBytes, FromZeroes, U16};

pub struct System {
    banks: [Box<[u8]>; 15],
}

impl Default for System {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(FromZeroes, FromBytes)]
struct Fill {
    length: U16<BigEndian>,
    bank: U16<BigEndian>,
    addr: U16<BigEndian>,
    value: u8,
}

#[derive(FromZeroes, FromBytes)]
struct Cpy {
    length: U16<BigEndian>,
    src_bank: U16<BigEndian>,
    src_addr: U16<BigEndian>,
    dst_bank: U16<BigEndian>,
    dst_addr: U16<BigEndian>,
}

mod port {
    pub const _UNUSED_0: u8 = 0x00;
    pub const _UNUSED_1: u8 = 0x01;
    pub const EXPANSION_0: u8 = 0x02;
    pub const EXPANSION_1: u8 = 0x03;
    pub const WST: u8 = 0x04;
    pub const RST: u8 = 0x05;
    pub const METADATA_0: u8 = 0x06;
    pub const METADATA_1: u8 = 0x07;
    pub const RED_0: u8 = 0x08;
    pub const RED_1: u8 = 0x09;
    pub const GREEN_0: u8 = 0x0a;
    pub const GREEN_1: u8 = 0x0b;
    pub const BLUE_0: u8 = 0x0c;
    pub const BLUE_1: u8 = 0x0d;
    pub const DEBUG: u8 = 0x0e;
    pub const STATE: u8 = 0x0f;
}

mod expansion {
    pub const FILL: u8 = 0x00;
    pub const CPYL: u8 = 0x01;
    pub const CPYR: u8 = 0x02;
}

impl Device for System {
    fn deo(&mut self, vm: &mut Uxn, target: u8) {
        let v = vm.dev_read(target);
        match target {
            port::EXPANSION_0 => (), // triggers on subsequent byte
            port::EXPANSION_1 => {
                let hi = vm.dev_read(0x2);
                let addr = u16::from_be_bytes([hi, v]);
                let op = vm.ram[addr as usize];
                match op {
                    expansion::FILL => {
                        let f = Fill::read_from(
                            &vm.ram[addr.wrapping_add(1) as usize..]
                                [..std::mem::size_of::<Fill>()],
                        )
                        .unwrap();
                        let bank = f.bank.get();
                        let addr = f.addr.get();
                        for i in 0..f.length.get() {
                            let ram = match bank {
                                0 => &mut vm.ram,
                                b => &mut self.banks[b as usize - 1],
                            };
                            ram[addr.wrapping_add(i) as usize] = f.value;
                        }
                    }
                    expansion::CPYL | expansion::CPYR => {
                        let c = Cpy::read_from(
                            &vm.ram[addr.wrapping_add(1) as usize..]
                                [..std::mem::size_of::<Cpy>()],
                        )
                        .unwrap();
                        let offset = |i, addr: zerocopy::U16<zerocopy::BE>| {
                            if op == expansion::CPYL {
                                addr.get().wrapping_add(i)
                            } else {
                                addr.get()
                                    .wrapping_add(c.length.get())
                                    .wrapping_sub(1)
                                    .wrapping_sub(i)
                            }
                        };

                        for i in 0..c.length.get() {
                            let src_addr = offset(i, c.src_addr);
                            let src = match c.src_bank.get() {
                                0 => &vm.ram,
                                b => &self.banks[b as usize - 1],
                            };
                            let v = src[src_addr as usize];

                            let dst_addr = offset(i, c.dst_addr);
                            let dst = match c.dst_bank.get() {
                                0 => &mut vm.ram,
                                b => &mut self.banks[b as usize - 1],
                            };
                            dst[dst_addr as usize] = v;
                        }
                    }
                    _ => panic!("invalid expansion opcode {op}"),
                }
            }
            port::WST => vm.stack.set_len(v),
            port::RST => vm.ret.set_len(v),
            port::METADATA_0 | port::METADATA_1 => (),
            port::RED_0 | port::RED_1 => (), // red
            port::GREEN_0 | port::GREEN_1 => (), // green
            port::BLUE_0 | port::BLUE_1 => (), // blue
            port::DEBUG => {
                for (name, st) in [("WST", &vm.stack), ("RST", &vm.ret)] {
                    print!("{name} ");
                    let n = st.len();
                    for i in (0..8).rev() {
                        print!("{:02x}", st.peek_byte_at(i));
                        if i == n {
                            print!("|")
                        } else {
                            print!(" ");
                        }
                    }
                    println!("<");
                }
            }
            port::STATE => {
                if v & 0x80 != 0 {
                    std::process::exit((v & !0x80) as i32);
                }
            }
            _ => unreachable!(),
        }
    }
    fn dei(&mut self, vm: &mut Uxn, target: u8) {
        match target & 0x0F {
            port::WST => vm.dev_write(target, vm.stack.len()),
            port::RST => vm.dev_write(target, vm.ret.len()),
            _ => (),
        }
    }
}

impl System {
    fn new() -> Self {
        let banks = [(); 15]
            .map(|_| vec![0u8; usize::from(u16::MAX)].into_boxed_slice());
        Self { banks }
    }
}
