//! # Registers
//! - `x0`: stack pointer (`&mut [u8; 256]`)
//! - `x1`: stack index (`u8`)
//! - `x2`: return stack pointer (`&mut [u8; 256]`)
//! - `x3`: return stack index (`u8`)
//! - `x4`: program counter (`u16`)
//! - `x5`: RAM pointer (`&mut [u8; 65536]`)
//! - `x6`: VM pointer (`&mut Uxn`)
//! - `x7`: Device handle pointer (`&mut DeviceHandle`)
//! - `x8`: Jump table pointer
#![allow(dead_code)]

use crate::native::EntryHandle;

core::arch::global_asm!(include_str!("aarch64.s"));

extern "C" {
    pub fn aarch64_entry(
        h: *const EntryHandle,
        pc: u16,
        table: *const unsafe extern "C" fn(),
    ) -> u16;
}
