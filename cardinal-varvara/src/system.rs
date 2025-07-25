use log::warn;
use std::mem::offset_of;
use uxn::{Ports, Uxn};
use zerocopy::{AsBytes, BigEndian, FromBytes, FromZeroes, U16};

pub struct System {
    exit: Option<i32>,
    banks: [Box<[u8; 65536]>; 15],
}

impl Default for System {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(FromZeroes, FromBytes, AsBytes)]
#[repr(C)]
struct Fill {
    length: U16<BigEndian>,
    bank: U16<BigEndian>,
    addr: U16<BigEndian>,
    value: u8,
}

#[derive(FromZeroes, FromBytes, AsBytes)]
#[repr(C)]
struct Cpy {
    length: U16<BigEndian>,
    src_bank: U16<BigEndian>,
    src_addr: U16<BigEndian>,
    dst_bank: U16<BigEndian>,
    dst_addr: U16<BigEndian>,
}

#[derive(AsBytes, FromZeroes, FromBytes)]
#[repr(C)]
pub struct SystemPorts {
    _unused_0: u8,
    _unused_1: u8,
    expansion: U16<BigEndian>,
    wst: u8,
    rst: u8,
    metadata: U16<BigEndian>,
    red: U16<BigEndian>,
    green: U16<BigEndian>,
    blue: U16<BigEndian>,
    debug: u8,
    state: u8,
}

impl Ports for SystemPorts {
    const BASE: u8 = 0x00;
}

impl SystemPorts {
    const EXPANSION: u8 = (offset_of!(Self, expansion) + 1) as u8;
    const WST: u8 = offset_of!(Self, wst) as u8;
    const RST: u8 = offset_of!(Self, rst) as u8;
    const DEBUG: u8 = offset_of!(Self, debug) as u8;
    const STATE: u8 = offset_of!(Self, state) as u8;

    /// Looks up the color for the given index
    pub fn color(&self, i: u8) -> u32 {
        let i = 3 - i;
        let r = u32::from(self.red.get() >> (i * 4)) & 0xF;
        let g = u32::from(self.green.get() >> (i * 4)) & 0xF;
        let b = u32::from(self.blue.get() >> (i * 4)) & 0xF;
        let color = 0x0F000000 | (r << 16) | (g << 8) | b;
        color | (color << 4)
    }
}

mod expansion {
    pub const FILL: u8 = 0x00;
    pub const CPYL: u8 = 0x01;
    pub const CPYR: u8 = 0x02;
}

impl System {
    pub fn new() -> Self {
        let banks = [(); 15].map(|_| Box::new([0u8; 65536]));
        Self { banks, exit: None }
    }

    /// Resets the peripheral, loading the given data into expansion memory
    pub fn reset(&mut self, mut mem: &[u8]) {
        for b in &mut self.banks {
            let n = mem.len().min(b.len());
            b[..n].copy_from_slice(&mem[..n]);
            mem = &mem[n..];
            b[n..].fill(0u8);
        }
        self.exit = None;
    }

    pub fn deo(&mut self, vm: &mut Uxn, target: u8) {
        let v = vm.dev::<SystemPorts>();
        match target {
            SystemPorts::EXPANSION => {
                let addr = v.expansion.get();
                let op = vm.ram_read_byte(addr);
                match op {
                    expansion::FILL => {
                        let mut f = Fill::new_zeroed();
                        for (i, b) in f.as_bytes_mut().iter_mut().enumerate() {
                            *b = vm.ram_read_byte(
                                addr.wrapping_add(1).wrapping_add(i as u16),
                            );
                        }
                        let bank = f.bank.get();
                        let addr = f.addr.get();
                        for i in 0..f.length.get() {
                            let j = addr.wrapping_add(i);
                            match usize::from(bank).checked_sub(1) {
                                None => vm.ram_write_byte(j, f.value),
                                Some(b) => {
                                    self.banks[b][usize::from(j)] = f.value
                                }
                            }
                        }
                    }
                    expansion::CPYL | expansion::CPYR => {
                        let mut c = Cpy::new_zeroed();
                        for (i, b) in c.as_bytes_mut().iter_mut().enumerate() {
                            *b = vm.ram_read_byte(
                                addr.wrapping_add(1).wrapping_add(i as u16),
                            );
                        }
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
                            let v = match usize::from(c.src_bank.get())
                                .checked_sub(1)
                            {
                                None => vm.ram_read_byte(src_addr),
                                Some(b) => self.banks[b][usize::from(src_addr)],
                            };

                            let dst_addr = offset(i, c.dst_addr);
                            match usize::from(c.dst_bank.get()).checked_sub(1) {
                                None => vm.ram_write_byte(dst_addr, v),
                                Some(b) => {
                                    self.banks[b][usize::from(dst_addr)] = v
                                }
                            }
                        }
                    }
                    _ => warn!("invalid expansion opcode {op}"),
                }
            }
            SystemPorts::WST => {
                let wst = v.wst;
                vm.stack_mut().set_len(wst)
            }
            SystemPorts::RST => {
                let rst = v.rst;
                vm.ret_mut().set_len(rst)
            }
            SystemPorts::DEBUG => {
                for (name, st) in [("WST", vm.stack()), ("RST", vm.ret())] {
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
            SystemPorts::STATE => {
                if v.state != 0 {
                    self.exit = Some((v.state & !0x80) as i32);
                }
            }
            _ => (),
        }
    }

    pub fn dei(&mut self, vm: &mut Uxn, target: u8) {
        match target & 0x0F {
            SystemPorts::WST => {
                let wst = vm.stack().len();
                vm.dev_mut::<SystemPorts>().wst = wst;
            }
            SystemPorts::RST => {
                let rst = vm.stack().len();
                vm.dev_mut::<SystemPorts>().rst = rst;
            }
            _ => (),
        }
    }

    /// Returns `true` if the exit flag is set
    pub fn should_exit(&self) -> bool {
        self.exit.is_some()
    }

    /// Clears and returns the exit code (if present)
    pub fn exit(&mut self) -> Option<i32> {
        self.exit.take()
    }
}
