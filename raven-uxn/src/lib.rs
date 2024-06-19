//! Uxn virtual machine
#![cfg_attr(not(test), no_std)]
#![warn(missing_docs)]
#![forbid(unsafe_code)]

const fn keep(flags: u8) -> bool {
    (flags & (1 << 2)) != 0
}
const fn short(flags: u8) -> bool {
    (flags & (1 << 0)) != 0
}
const fn ret(flags: u8) -> bool {
    (flags & (1 << 1)) != 0
}

/// Size of a device in port memory
pub const DEV_SIZE: usize = 16;

/// Simple circular stack, with room for 256 items
#[derive(Debug)]
pub struct Stack {
    data: [u8; 256],

    /// The index points to the last occupied slot, and increases on `push`
    ///
    /// If the buffer is empty or full, it points to `u8::MAX`.
    index: u8,
}

/// Virtual stack, which is aware of `keep` and `short` modes
///
/// This type expects the user to perform all of their `pop()` calls first,
/// followed by any `push(..)` calls.  `pop()` will either adjust the true index
/// or a virtual index, depending on whether `keep` is set.
struct StackView<'a, const FLAGS: u8> {
    stack: &'a mut Stack,

    /// Virtual index, used in `keep` mode
    offset: u8,
}

impl<'a, const FLAGS: u8> StackView<'a, FLAGS> {
    fn new(stack: &'a mut Stack) -> Self {
        Self { stack, offset: 0 }
    }

    /// Pops a single value from the stack
    ///
    /// Returns a [`Value::Short`] if `self.short` is set, and a [`Value::Byte`]
    /// otherwise.
    ///
    /// If `self.keep` is set, then only the view offset ([`StackView::offset`])
    /// is changed; otherwise, the stack index ([`Stack::index`]) is changed.
    #[inline]
    fn pop(&mut self) -> Value {
        if short(FLAGS) {
            Value::Short(self.pop_short())
        } else {
            Value::Byte(self.pop_byte())
        }
    }

    fn pop_byte(&mut self) -> u8 {
        if keep(FLAGS) {
            let v = self.stack.peek_byte_at(self.offset);
            self.offset = self.offset.wrapping_add(1);
            v
        } else {
            self.stack.pop_byte()
        }
    }

    fn pop_short(&mut self) -> u16 {
        if keep(FLAGS) {
            let v = self.stack.peek_short_at(self.offset);
            self.offset = self.offset.wrapping_add(2);
            v
        } else {
            self.stack.pop_short()
        }
    }

    fn push(&mut self, v: Value) {
        self.stack.push(v);
    }

    fn reserve(&mut self, n: u8) {
        self.stack.reserve(n);
    }

    /// Replaces the top item on the stack with the given value
    fn emplace(&mut self, v: Value) {
        match v {
            Value::Short(v) => {
                self.stack.emplace_short(v);
            }
            Value::Byte(v) => {
                self.stack.emplace_byte(v);
            }
        }
    }

    fn push_byte(&mut self, v: u8) {
        self.stack.push_byte(v);
    }

    fn push_short(&mut self, v: u16) {
        self.stack.push_short(v);
    }
}

impl Default for Stack {
    fn default() -> Self {
        Self {
            data: [0u8; 256],
            index: u8::MAX,
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum Value {
    Short(u16),
    Byte(u8),
}

impl Value {
    #[inline]
    fn wrapping_add(&self, i: u8) -> Self {
        match self {
            Value::Short(v) => Value::Short(v.wrapping_add(u16::from(i))),
            Value::Byte(v) => Value::Byte(v.wrapping_add(i)),
        }
    }
    #[inline]
    fn wrapping_shr(&self, i: u32) -> Self {
        match self {
            Value::Short(v) => Value::Short(v.wrapping_shr(i)),
            Value::Byte(v) => Value::Byte(v.wrapping_shr(i)),
        }
    }
    #[inline]
    fn wrapping_shl(&self, i: u32) -> Self {
        match self {
            Value::Short(v) => Value::Short(v.wrapping_shl(i)),
            Value::Byte(v) => Value::Byte(v.wrapping_shl(i)),
        }
    }
}

impl From<Value> for u16 {
    fn from(v: Value) -> u16 {
        match v {
            Value::Short(v) => v,
            Value::Byte(v) => u16::from(v),
        }
    }
}

impl Stack {
    #[inline]
    fn pop_byte(&mut self) -> u8 {
        let out = self.data[usize::from(self.index)];
        self.index = self.index.wrapping_sub(1);
        out
    }

    #[inline]
    fn pop_short(&mut self) -> u16 {
        let lo = self.pop_byte();
        let hi = self.pop_byte();
        u16::from_le_bytes([lo, hi])
    }

    #[inline]
    fn push_byte(&mut self, v: u8) {
        self.index = self.index.wrapping_add(1);
        self.data[usize::from(self.index)] = v;
    }

    #[inline]
    fn emplace_byte(&mut self, v: u8) {
        self.data[usize::from(self.index)] = v;
    }

    #[inline]
    fn emplace_short(&mut self, v: u16) {
        let [lo, hi] = v.to_le_bytes();
        self.data[usize::from(self.index.wrapping_sub(1))] = hi;
        self.data[usize::from(self.index)] = lo;
    }

    #[inline]
    fn reserve(&mut self, n: u8) {
        self.index = self.index.wrapping_add(n);
    }

    #[inline]
    fn push_short(&mut self, v: u16) {
        let [lo, hi] = v.to_le_bytes();
        self.push_byte(hi);
        self.push_byte(lo);
    }

    #[inline]
    fn push(&mut self, v: Value) {
        match v {
            Value::Short(v) => self.push_short(v),
            Value::Byte(v) => self.push_byte(v),
        }
    }

    /// Peeks at a byte from the data stack
    #[inline]
    pub fn peek_byte_at(&self, offset: u8) -> u8 {
        self.data[usize::from(self.index.wrapping_sub(offset))]
    }

    #[inline]
    fn peek_short_at(&self, offset: u8) -> u16 {
        let lo = self.peek_byte_at(offset);
        let hi = self.peek_byte_at(offset.wrapping_add(1));
        u16::from_le_bytes([lo, hi])
    }

    /// Returns the number of items in the stack
    #[inline]
    pub fn len(&self) -> u8 {
        self.index.wrapping_add(1)
    }

    /// Checks whether the stack is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Sets the number of items in the stack
    #[inline]
    pub fn set_len(&mut self, n: u8) {
        self.index = n.wrapping_sub(1);
    }
}

/// The virtual machine itself
pub struct Uxn<'a> {
    /// Device memory
    dev: [u8; 256],
    /// 64 KiB of VM memory
    ram: &'a mut [u8; 65536],
    /// 256-byte data stack
    stack: Stack,
    /// 256-byte return stack
    ret: Stack,
}

macro_rules! op_cmp {
    ($self:ident, $flags:ident, $f:expr) => {{
        let mut s = $self.stack_view::<{ $flags }>();
        #[allow(clippy::redundant_closure_call)]
        let v = if short($flags) {
            let b = s.pop_short();
            let a = s.pop_short();
            ($f)(a, b)
        } else {
            let b = s.pop_byte();
            let a = s.pop_byte();
            ($f)(a, b)
        };
        s.push_byte(v as u8);
    }};
}

macro_rules! op_bin {
    ($self:ident, $flags:ident, $f:expr) => {{
        let mut s = $self.stack_view::<{ $flags }>();
        #[allow(clippy::redundant_closure_call)]
        if short($flags) {
            let b = s.pop_short();
            let a = s.pop_short();
            let f: fn(u16, u16) -> u16 = $f;
            s.push_short(f(a, b));
        } else {
            let b = s.pop_byte();
            let a = s.pop_byte();
            let f: fn(u8, u8) -> u8 = $f;
            s.push_byte(f(a, b));
        };
    }};
}

impl<'a> Uxn<'a> {
    /// Build a new `Uxn`, loading the given ROM at the start address
    ///
    /// # Panics
    /// If `rom` cannot fit in memory
    pub fn new<'b>(rom: &'b [u8], ram: &'a mut [u8; 65536]) -> Self {
        let out = Self {
            dev: [0u8; 256],
            ram,
            stack: Stack::default(),
            ret: Stack::default(),
        };
        out.ram[0x100..][..rom.len()].copy_from_slice(rom);
        out
    }

    /// Reads a byte from RAM at the program counter
    #[inline]
    fn next(&mut self, pc: &mut u16) -> u8 {
        let out = self.ram[usize::from(*pc)];
        *pc = pc.wrapping_add(1);
        out
    }

    /// Reads a word from RAM at the program counter
    #[inline]
    fn next2(&mut self, pc: &mut u16) -> u16 {
        let hi = self.next(pc);
        let lo = self.next(pc);
        u16::from_le_bytes([lo, hi])
    }

    #[inline]
    fn ram_write(&mut self, addr: u16, v: Value) {
        match v {
            Value::Short(v) => {
                let [lo, hi] = v.to_le_bytes();
                self.ram[usize::from(addr)] = hi;
                self.ram[usize::from(addr.wrapping_add(1))] = lo;
            }
            Value::Byte(v) => {
                self.ram[usize::from(addr)] = v;
            }
        }
    }

    fn ram_read<const FLAGS: u8>(&self, addr: u16) -> Value {
        if short(FLAGS) {
            let hi = self.ram[usize::from(addr)];
            let lo = self.ram[usize::from(addr.wrapping_add(1))];
            Value::Short(u16::from_le_bytes([lo, hi]))
        } else {
            let v = self.ram[usize::from(addr)];
            Value::Byte(v)
        }
    }

    #[inline]
    fn stack_view<const FLAGS: u8>(&mut self) -> StackView<FLAGS> {
        let stack = if ret(FLAGS) {
            &mut self.ret
        } else {
            &mut self.stack
        };
        StackView::new(stack)
    }

    #[inline]
    fn ret_stack_view<const FLAGS: u8>(&mut self) -> StackView<FLAGS> {
        let stack = if ret(FLAGS) {
            &mut self.stack
        } else {
            &mut self.ret
        };
        StackView::new(stack)
    }

    #[inline]
    fn check_dev_size<D: Ports>() {
        struct AssertDevSize<D>(D);
        impl<D> AssertDevSize<D> {
            const ASSERT: () = if core::mem::size_of::<D>() != DEV_SIZE {
                panic!("dev must be 16 bytes");
            };
        }
        AssertDevSize::<D>::ASSERT
    }

    /// Converts raw ports memory into a [`Ports`] object
    #[inline]
    pub fn dev<D: Ports>(&self) -> &D {
        self.dev_at(D::BASE)
    }

    /// Returns a reference to a device located at `pos`
    #[inline]
    pub fn dev_at<D: Ports>(&self, pos: u8) -> &D {
        Self::check_dev_size::<D>();
        D::ref_from(&self.dev[pos as usize..][..DEV_SIZE]).unwrap()
    }

    /// Returns a reference to a device located at `pos`
    #[inline]
    pub fn dev_mut_at<D: Ports>(&mut self, pos: u8) -> &mut D {
        Self::check_dev_size::<D>();
        D::mut_from(&mut self.dev[pos as usize..][..DEV_SIZE]).unwrap()
    }

    /// Returns a mutable reference to the given [`Ports`] object
    #[inline]
    pub fn dev_mut<D: Ports>(&mut self) -> &mut D {
        self.dev_mut_at(D::BASE)
    }

    /// Writes to the given address in device memory
    #[inline]
    pub fn write_dev_mem(&mut self, addr: u8, value: u8) {
        self.dev[usize::from(addr)] = value;
    }

    /// Mutably borrows the entire RAM array
    #[inline]
    pub fn ram_mut(&mut self) -> &mut [u8; 65536] {
        self.ram
    }

    /// Shared borrow of the entire RAM array
    #[inline]
    pub fn ram(&mut self) -> &[u8; 65536] {
        self.ram
    }

    /// Reads a byte from RAM
    #[inline]
    pub fn ram_read_byte(&self, addr: u16) -> u8 {
        self.ram[addr as usize]
    }

    /// Writes a byte to RAM
    #[inline]
    pub fn ram_write_byte(&mut self, addr: u16, v: u8) {
        self.ram[addr as usize] = v;
    }

    /// Reads a word from RAM
    ///
    /// If the address is at the top of RAM, the second byte will wrap to 0
    #[inline]
    pub fn ram_read_word(&self, addr: u16) -> u16 {
        let hi = self.ram[addr as usize];
        let lo = self.ram[addr.wrapping_add(1) as usize];
        u16::from_le_bytes([lo, hi])
    }

    /// Shared borrow of the working stack
    #[inline]
    pub fn stack(&self) -> &Stack {
        &self.stack
    }

    /// Mutable borrow of the working stack
    #[inline]
    pub fn stack_mut(&mut self) -> &mut Stack {
        &mut self.stack
    }

    /// Shared borrow of the return stack
    #[inline]
    pub fn ret(&self) -> &Stack {
        &self.ret
    }

    /// Mutable borrow of the return stack
    #[inline]
    pub fn ret_mut(&mut self) -> &mut Stack {
        &mut self.ret
    }

    /// Runs the VM starting at the given address until it terminates
    #[inline]
    pub fn run<D: Device>(&mut self, dev: &mut D, mut pc: u16) {
        loop {
            let op = self.next(&mut pc);
            let Some(next) = self.op(op, dev, pc) else {
                break;
            };
            pc = next;
        }
    }

    /// Executes a single operation
    #[inline]
    fn op<D: Device>(&mut self, op: u8, dev: &mut D, pc: u16) -> Option<u16> {
        match op {
            0x00 => op::brk(self, dev, pc),
            0x01 => op::inc::<0b000>(self, dev, pc),
            0x02 => op::pop::<0b000>(self, dev, pc),
            0x03 => op::nip::<0b000>(self, dev, pc),
            0x04 => op::swp::<0b000>(self, dev, pc),
            0x05 => op::rot::<0b000>(self, dev, pc),
            0x06 => op::dup::<0b000>(self, dev, pc),
            0x07 => op::ovr::<0b000>(self, dev, pc),
            0x08 => op::equ::<0b000>(self, dev, pc),
            0x09 => op::neq::<0b000>(self, dev, pc),
            0x0a => op::gth::<0b000>(self, dev, pc),
            0x0b => op::lth::<0b000>(self, dev, pc),
            0x0c => op::jmp::<0b000>(self, dev, pc),
            0x0d => op::jcn::<0b000>(self, dev, pc),
            0x0e => op::jsr::<0b000>(self, dev, pc),
            0x0f => op::sth::<0b000>(self, dev, pc),
            0x10 => op::ldz::<0b000>(self, dev, pc),
            0x11 => op::stz::<0b000>(self, dev, pc),
            0x12 => op::ldr::<0b000>(self, dev, pc),
            0x13 => op::str::<0b000>(self, dev, pc),
            0x14 => op::lda::<0b000>(self, dev, pc),
            0x15 => op::sta::<0b000>(self, dev, pc),
            0x16 => op::dei::<0b000>(self, dev, pc),
            0x17 => op::deo::<0b000>(self, dev, pc),
            0x18 => op::add::<0b000>(self, dev, pc),
            0x19 => op::sub::<0b000>(self, dev, pc),
            0x1a => op::mul::<0b000>(self, dev, pc),
            0x1b => op::div::<0b000>(self, dev, pc),
            0x1c => op::and::<0b000>(self, dev, pc),
            0x1d => op::ora::<0b000>(self, dev, pc),
            0x1e => op::eor::<0b000>(self, dev, pc),
            0x1f => op::sft::<0b000>(self, dev, pc),
            0x20 => op::jci(self, dev, pc),
            0x21 => op::inc::<0b001>(self, dev, pc),
            0x22 => op::pop::<0b001>(self, dev, pc),
            0x23 => op::nip::<0b001>(self, dev, pc),
            0x24 => op::swp::<0b001>(self, dev, pc),
            0x25 => op::rot::<0b001>(self, dev, pc),
            0x26 => op::dup::<0b001>(self, dev, pc),
            0x27 => op::ovr::<0b001>(self, dev, pc),
            0x28 => op::equ::<0b001>(self, dev, pc),
            0x29 => op::neq::<0b001>(self, dev, pc),
            0x2a => op::gth::<0b001>(self, dev, pc),
            0x2b => op::lth::<0b001>(self, dev, pc),
            0x2c => op::jmp::<0b001>(self, dev, pc),
            0x2d => op::jcn::<0b001>(self, dev, pc),
            0x2e => op::jsr::<0b001>(self, dev, pc),
            0x2f => op::sth::<0b001>(self, dev, pc),
            0x30 => op::ldz::<0b001>(self, dev, pc),
            0x31 => op::stz::<0b001>(self, dev, pc),
            0x32 => op::ldr::<0b001>(self, dev, pc),
            0x33 => op::str::<0b001>(self, dev, pc),
            0x34 => op::lda::<0b001>(self, dev, pc),
            0x35 => op::sta::<0b001>(self, dev, pc),
            0x36 => op::dei::<0b001>(self, dev, pc),
            0x37 => op::deo::<0b001>(self, dev, pc),
            0x38 => op::add::<0b001>(self, dev, pc),
            0x39 => op::sub::<0b001>(self, dev, pc),
            0x3a => op::mul::<0b001>(self, dev, pc),
            0x3b => op::div::<0b001>(self, dev, pc),
            0x3c => op::and::<0b001>(self, dev, pc),
            0x3d => op::ora::<0b001>(self, dev, pc),
            0x3e => op::eor::<0b001>(self, dev, pc),
            0x3f => op::sft::<0b001>(self, dev, pc),
            0x40 => op::jmi(self, dev, pc),
            0x41 => op::inc::<0b010>(self, dev, pc),
            0x42 => op::pop::<0b010>(self, dev, pc),
            0x43 => op::nip::<0b010>(self, dev, pc),
            0x44 => op::swp::<0b010>(self, dev, pc),
            0x45 => op::rot::<0b010>(self, dev, pc),
            0x46 => op::dup::<0b010>(self, dev, pc),
            0x47 => op::ovr::<0b010>(self, dev, pc),
            0x48 => op::equ::<0b010>(self, dev, pc),
            0x49 => op::neq::<0b010>(self, dev, pc),
            0x4a => op::gth::<0b010>(self, dev, pc),
            0x4b => op::lth::<0b010>(self, dev, pc),
            0x4c => op::jmp::<0b010>(self, dev, pc),
            0x4d => op::jcn::<0b010>(self, dev, pc),
            0x4e => op::jsr::<0b010>(self, dev, pc),
            0x4f => op::sth::<0b010>(self, dev, pc),
            0x50 => op::ldz::<0b010>(self, dev, pc),
            0x51 => op::stz::<0b010>(self, dev, pc),
            0x52 => op::ldr::<0b010>(self, dev, pc),
            0x53 => op::str::<0b010>(self, dev, pc),
            0x54 => op::lda::<0b010>(self, dev, pc),
            0x55 => op::sta::<0b010>(self, dev, pc),
            0x56 => op::dei::<0b010>(self, dev, pc),
            0x57 => op::deo::<0b010>(self, dev, pc),
            0x58 => op::add::<0b010>(self, dev, pc),
            0x59 => op::sub::<0b010>(self, dev, pc),
            0x5a => op::mul::<0b010>(self, dev, pc),
            0x5b => op::div::<0b010>(self, dev, pc),
            0x5c => op::and::<0b010>(self, dev, pc),
            0x5d => op::ora::<0b010>(self, dev, pc),
            0x5e => op::eor::<0b010>(self, dev, pc),
            0x5f => op::sft::<0b010>(self, dev, pc),
            0x60 => op::jsi(self, dev, pc),
            0x61 => op::inc::<0b011>(self, dev, pc),
            0x62 => op::pop::<0b011>(self, dev, pc),
            0x63 => op::nip::<0b011>(self, dev, pc),
            0x64 => op::swp::<0b011>(self, dev, pc),
            0x65 => op::rot::<0b011>(self, dev, pc),
            0x66 => op::dup::<0b011>(self, dev, pc),
            0x67 => op::ovr::<0b011>(self, dev, pc),
            0x68 => op::equ::<0b011>(self, dev, pc),
            0x69 => op::neq::<0b011>(self, dev, pc),
            0x6a => op::gth::<0b011>(self, dev, pc),
            0x6b => op::lth::<0b011>(self, dev, pc),
            0x6c => op::jmp::<0b011>(self, dev, pc),
            0x6d => op::jcn::<0b011>(self, dev, pc),
            0x6e => op::jsr::<0b011>(self, dev, pc),
            0x6f => op::sth::<0b011>(self, dev, pc),
            0x70 => op::ldz::<0b011>(self, dev, pc),
            0x71 => op::stz::<0b011>(self, dev, pc),
            0x72 => op::ldr::<0b011>(self, dev, pc),
            0x73 => op::str::<0b011>(self, dev, pc),
            0x74 => op::lda::<0b011>(self, dev, pc),
            0x75 => op::sta::<0b011>(self, dev, pc),
            0x76 => op::dei::<0b011>(self, dev, pc),
            0x77 => op::deo::<0b011>(self, dev, pc),
            0x78 => op::add::<0b011>(self, dev, pc),
            0x79 => op::sub::<0b011>(self, dev, pc),
            0x7a => op::mul::<0b011>(self, dev, pc),
            0x7b => op::div::<0b011>(self, dev, pc),
            0x7c => op::and::<0b011>(self, dev, pc),
            0x7d => op::ora::<0b011>(self, dev, pc),
            0x7e => op::eor::<0b011>(self, dev, pc),
            0x7f => op::sft::<0b011>(self, dev, pc),
            0x80 => op::lit::<0b100>(self, dev, pc),
            0x81 => op::inc::<0b100>(self, dev, pc),
            0x82 => op::pop::<0b100>(self, dev, pc),
            0x83 => op::nip::<0b100>(self, dev, pc),
            0x84 => op::swp::<0b100>(self, dev, pc),
            0x85 => op::rot::<0b100>(self, dev, pc),
            0x86 => op::dup::<0b100>(self, dev, pc),
            0x87 => op::ovr::<0b100>(self, dev, pc),
            0x88 => op::equ::<0b100>(self, dev, pc),
            0x89 => op::neq::<0b100>(self, dev, pc),
            0x8a => op::gth::<0b100>(self, dev, pc),
            0x8b => op::lth::<0b100>(self, dev, pc),
            0x8c => op::jmp::<0b100>(self, dev, pc),
            0x8d => op::jcn::<0b100>(self, dev, pc),
            0x8e => op::jsr::<0b100>(self, dev, pc),
            0x8f => op::sth::<0b100>(self, dev, pc),
            0x90 => op::ldz::<0b100>(self, dev, pc),
            0x91 => op::stz::<0b100>(self, dev, pc),
            0x92 => op::ldr::<0b100>(self, dev, pc),
            0x93 => op::str::<0b100>(self, dev, pc),
            0x94 => op::lda::<0b100>(self, dev, pc),
            0x95 => op::sta::<0b100>(self, dev, pc),
            0x96 => op::dei::<0b100>(self, dev, pc),
            0x97 => op::deo::<0b100>(self, dev, pc),
            0x98 => op::add::<0b100>(self, dev, pc),
            0x99 => op::sub::<0b100>(self, dev, pc),
            0x9a => op::mul::<0b100>(self, dev, pc),
            0x9b => op::div::<0b100>(self, dev, pc),
            0x9c => op::and::<0b100>(self, dev, pc),
            0x9d => op::ora::<0b100>(self, dev, pc),
            0x9e => op::eor::<0b100>(self, dev, pc),
            0x9f => op::sft::<0b100>(self, dev, pc),
            0xa0 => op::lit::<0b101>(self, dev, pc),
            0xa1 => op::inc::<0b101>(self, dev, pc),
            0xa2 => op::pop::<0b101>(self, dev, pc),
            0xa3 => op::nip::<0b101>(self, dev, pc),
            0xa4 => op::swp::<0b101>(self, dev, pc),
            0xa5 => op::rot::<0b101>(self, dev, pc),
            0xa6 => op::dup::<0b101>(self, dev, pc),
            0xa7 => op::ovr::<0b101>(self, dev, pc),
            0xa8 => op::equ::<0b101>(self, dev, pc),
            0xa9 => op::neq::<0b101>(self, dev, pc),
            0xaa => op::gth::<0b101>(self, dev, pc),
            0xab => op::lth::<0b101>(self, dev, pc),
            0xac => op::jmp::<0b101>(self, dev, pc),
            0xad => op::jcn::<0b101>(self, dev, pc),
            0xae => op::jsr::<0b101>(self, dev, pc),
            0xaf => op::sth::<0b101>(self, dev, pc),
            0xb0 => op::ldz::<0b101>(self, dev, pc),
            0xb1 => op::stz::<0b101>(self, dev, pc),
            0xb2 => op::ldr::<0b101>(self, dev, pc),
            0xb3 => op::str::<0b101>(self, dev, pc),
            0xb4 => op::lda::<0b101>(self, dev, pc),
            0xb5 => op::sta::<0b101>(self, dev, pc),
            0xb6 => op::dei::<0b101>(self, dev, pc),
            0xb7 => op::deo::<0b101>(self, dev, pc),
            0xb8 => op::add::<0b101>(self, dev, pc),
            0xb9 => op::sub::<0b101>(self, dev, pc),
            0xba => op::mul::<0b101>(self, dev, pc),
            0xbb => op::div::<0b101>(self, dev, pc),
            0xbc => op::and::<0b101>(self, dev, pc),
            0xbd => op::ora::<0b101>(self, dev, pc),
            0xbe => op::eor::<0b101>(self, dev, pc),
            0xbf => op::sft::<0b101>(self, dev, pc),
            0xc0 => op::lit::<0b110>(self, dev, pc),
            0xc1 => op::inc::<0b110>(self, dev, pc),
            0xc2 => op::pop::<0b110>(self, dev, pc),
            0xc3 => op::nip::<0b110>(self, dev, pc),
            0xc4 => op::swp::<0b110>(self, dev, pc),
            0xc5 => op::rot::<0b110>(self, dev, pc),
            0xc6 => op::dup::<0b110>(self, dev, pc),
            0xc7 => op::ovr::<0b110>(self, dev, pc),
            0xc8 => op::equ::<0b110>(self, dev, pc),
            0xc9 => op::neq::<0b110>(self, dev, pc),
            0xca => op::gth::<0b110>(self, dev, pc),
            0xcb => op::lth::<0b110>(self, dev, pc),
            0xcc => op::jmp::<0b110>(self, dev, pc),
            0xcd => op::jcn::<0b110>(self, dev, pc),
            0xce => op::jsr::<0b110>(self, dev, pc),
            0xcf => op::sth::<0b110>(self, dev, pc),
            0xd0 => op::ldz::<0b110>(self, dev, pc),
            0xd1 => op::stz::<0b110>(self, dev, pc),
            0xd2 => op::ldr::<0b110>(self, dev, pc),
            0xd3 => op::str::<0b110>(self, dev, pc),
            0xd4 => op::lda::<0b110>(self, dev, pc),
            0xd5 => op::sta::<0b110>(self, dev, pc),
            0xd6 => op::dei::<0b110>(self, dev, pc),
            0xd7 => op::deo::<0b110>(self, dev, pc),
            0xd8 => op::add::<0b110>(self, dev, pc),
            0xd9 => op::sub::<0b110>(self, dev, pc),
            0xda => op::mul::<0b110>(self, dev, pc),
            0xdb => op::div::<0b110>(self, dev, pc),
            0xdc => op::and::<0b110>(self, dev, pc),
            0xdd => op::ora::<0b110>(self, dev, pc),
            0xde => op::eor::<0b110>(self, dev, pc),
            0xdf => op::sft::<0b110>(self, dev, pc),
            0xe0 => op::lit::<0b111>(self, dev, pc),
            0xe1 => op::inc::<0b111>(self, dev, pc),
            0xe2 => op::pop::<0b111>(self, dev, pc),
            0xe3 => op::nip::<0b111>(self, dev, pc),
            0xe4 => op::swp::<0b111>(self, dev, pc),
            0xe5 => op::rot::<0b111>(self, dev, pc),
            0xe6 => op::dup::<0b111>(self, dev, pc),
            0xe7 => op::ovr::<0b111>(self, dev, pc),
            0xe8 => op::equ::<0b111>(self, dev, pc),
            0xe9 => op::neq::<0b111>(self, dev, pc),
            0xea => op::gth::<0b111>(self, dev, pc),
            0xeb => op::lth::<0b111>(self, dev, pc),
            0xec => op::jmp::<0b111>(self, dev, pc),
            0xed => op::jcn::<0b111>(self, dev, pc),
            0xee => op::jsr::<0b111>(self, dev, pc),
            0xef => op::sth::<0b111>(self, dev, pc),
            0xf0 => op::ldz::<0b111>(self, dev, pc),
            0xf1 => op::stz::<0b111>(self, dev, pc),
            0xf2 => op::ldr::<0b111>(self, dev, pc),
            0xf3 => op::str::<0b111>(self, dev, pc),
            0xf4 => op::lda::<0b111>(self, dev, pc),
            0xf5 => op::sta::<0b111>(self, dev, pc),
            0xf6 => op::dei::<0b111>(self, dev, pc),
            0xf7 => op::deo::<0b111>(self, dev, pc),
            0xf8 => op::add::<0b111>(self, dev, pc),
            0xf9 => op::sub::<0b111>(self, dev, pc),
            0xfa => op::mul::<0b111>(self, dev, pc),
            0xfb => op::div::<0b111>(self, dev, pc),
            0xfc => op::and::<0b111>(self, dev, pc),
            0xfd => op::ora::<0b111>(self, dev, pc),
            0xfe => op::eor::<0b111>(self, dev, pc),
            0xff => op::sft::<0b111>(self, dev, pc),
        }
    }
}

mod op {
    use super::*;

    /// Computes a jump, either relative (signed) or absolute
    #[inline]
    fn jump_offset(pc: u16, v: Value) -> u16 {
        match v {
            Value::Short(dst) => dst,
            Value::Byte(offset) => {
                let offset = i16::from(offset as i8);
                pc.wrapping_add_signed(offset)
            }
        }
    }

    /// Break
    /// ```text
    /// BRK --
    /// ```
    ///
    /// Ends the evaluation of the current vector. This opcode has no modes.
    #[inline]
    pub fn brk(_: &mut Uxn, _: &mut dyn Device, _: u16) -> Option<u16> {
        None
    }

    /// Jump Conditional Instant
    ///
    /// ```text
    /// JCI cond8 --
    /// ```
    ///
    /// Pops a byte from the working stack and if it is not zero, moves
    /// the `PC` to a relative address at a distance equal to the next short in
    /// memory, otherwise moves `PC+2`. This opcode has no modes.
    #[inline]
    pub fn jci(vm: &mut Uxn, _: &mut dyn Device, mut pc: u16) -> Option<u16> {
        let dt = vm.next2(&mut pc);
        if vm.stack.pop_byte() != 0 {
            pc = pc.wrapping_add(dt);
        }
        Some(pc)
    }

    /// Jump Instant
    ///
    /// JMI  -- Moves the PC to a relative address at a distance equal to the
    /// next short in memory. This opcode has no modes.
    #[inline]
    pub fn jmi(vm: &mut Uxn, _: &mut dyn Device, mut pc: u16) -> Option<u16> {
        let dt = vm.next2(&mut pc);
        Some(pc.wrapping_add(dt))
    }

    /// Jump Stash Return Instant
    ///
    /// ```text
    /// JSI  --
    /// ```
    ///
    /// Pushes `PC+2` to the return-stack and moves the `PC` to a relative
    /// address at a distance equal to the next short in memory. This opcode has
    /// no modes.
    #[inline]
    pub fn jsi(vm: &mut Uxn, _: &mut dyn Device, mut pc: u16) -> Option<u16> {
        let dt = vm.next2(&mut pc);
        vm.ret.push(Value::Short(pc));
        Some(pc.wrapping_add(dt))
    }

    /// Literal
    ///
    /// ```text
    /// LIT -- a
    /// ```
    ///
    /// Pushes the next bytes in memory, and moves the `PC+2`. The `LIT` opcode
    /// always has the keep mode active. Notice how the `0x00` opcode, with the
    /// keep bit toggled, is the location of the literal opcodes.
    ///
    /// ```text
    /// LIT 12          ( 12 )
    /// LIT2 abcd       ( ab cd )
    /// ```
    #[inline]
    pub fn lit<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        mut pc: u16,
    ) -> Option<u16> {
        let v = if short(FLAGS) {
            Value::Short(vm.next2(&mut pc))
        } else {
            Value::Byte(vm.next(&mut pc))
        };
        vm.stack_view::<FLAGS>().push(v);
        Some(pc)
    }

    /// Increment
    ///
    /// ```text
    /// INC a -- a+1
    /// ```
    ///
    /// Increments the value at the top of the stack, by 1.
    ///
    /// ```text
    /// #01 INC         ( 02 )
    /// #0001 INC2      ( 00 02 )
    /// #0001 INC2k     ( 00 01 00 02 )
    /// ```
    #[inline]
    pub fn inc<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        let mut s = vm.stack_view::<FLAGS>();
        let v = s.pop();
        s.push(v.wrapping_add(1));
        Some(pc)
    }

    /// Pop
    ///
    /// ```text
    /// POP a --
    /// ```
    ///
    /// Removes the value at the top of the stack.
    ///
    /// ```text
    /// #1234 POP    ( 12 )
    /// #1234 POP2   ( )
    /// #1234 POP2k  ( 12 34 )
    /// ```
    #[inline]
    pub fn pop<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        vm.stack_view::<FLAGS>().pop();
        Some(pc)
    }

    /// Nip
    ///
    /// ```text
    /// NIP a b -- b
    /// ```
    ///
    /// Removes the second value from the stack. This is practical to convert a
    /// short into a byte.
    ///
    /// ```text
    /// #1234 NIP          ( 34 )
    /// #1234 #5678 NIP2   ( 56 78 )
    /// #1234 #5678 NIP2k  ( 12 34 56 78 56 78 )
    /// ```
    #[inline]
    pub fn nip<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        let mut s = vm.stack_view::<FLAGS>();
        let v = s.pop();
        let _ = s.pop();
        s.push(v);
        Some(pc)
    }

    /// Swap
    ///
    /// ```text
    /// SWP a b -- b a
    /// ```
    ///
    /// Exchanges the first and second values at the top of the stack.
    ///
    /// ```text
    /// #1234 SWP          ( 34 12 )
    /// #1234 SWPk         ( 12 34 34 12 )
    /// #1234 #5678 SWP2   ( 56 78 12 34 )
    /// #1234 #5678 SWP2k  ( 12 34 56 78 56 78 12 34 )
    /// ```
    #[inline]
    pub fn swp<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        let mut s = vm.stack_view::<FLAGS>();
        let b = s.pop();
        let a = s.pop();
        s.push(b);
        s.push(a);
        Some(pc)
    }

    /// Rotate
    ///
    /// ```text
    /// ROT a b c -- b c a
    /// ```
    ///
    /// Rotates three values at the top of the stack, to the left, wrapping
    /// around.
    ///
    /// ```text
    /// #1234 #56 ROT            ( 34 56 12 )
    /// #1234 #56 ROTk           ( 12 34 56 34 56 12 )
    /// #1234 #5678 #9abc ROT2   ( 56 78 9a bc 12 34 )
    /// #1234 #5678 #9abc ROT2k  ( 12 34 56 78 9a bc 56 78 9a bc 12 34 )
    /// ```
    #[inline]
    pub fn rot<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        let mut s = vm.stack_view::<FLAGS>();
        let c = s.pop();
        let b = s.pop();
        let a = s.pop();
        s.push(b);
        s.push(c);
        s.push(a);
        Some(pc)
    }

    /// Duplicate
    ///
    /// ```text
    /// DUP a -- a a
    /// ```
    ///
    /// Duplicates the value at the top of the stack.
    ///
    /// ```text
    /// #1234 DUP   ( 12 34 34 )
    /// #12 DUPk    ( 12 12 12 )
    /// #1234 DUP2  ( 12 34 12 34 )
    /// ```
    #[inline]
    pub fn dup<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        let mut s = vm.stack_view::<FLAGS>();
        let v = s.pop();
        s.push(v);
        s.push(v);
        Some(pc)
    }

    /// Over
    ///
    /// ```text
    /// OVR a b -- a b a
    /// ```
    ///
    /// Duplicates the second value at the top of the stack.
    ///
    /// ```text
    /// #1234 OVR          ( 12 34 12 )
    /// #1234 OVRk         ( 12 34 12 34 12 )
    /// #1234 #5678 OVR2   ( 12 34 56 78 12 34 )
    /// #1234 #5678 OVR2k  ( 12 34 56 78 12 34 56 78 12 34 )
    /// ```
    #[inline]
    pub fn ovr<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        let mut s = vm.stack_view::<FLAGS>();
        let b = s.pop();
        let a = s.pop();
        s.push(a);
        s.push(b);
        s.push(a);
        Some(pc)
    }

    /// Equal
    ///
    /// ```text
    /// EQU a b -- bool8
    /// ```
    ///
    /// Pushes `01` to the stack if the two values at the top of the stack are
    /// equal, `00` otherwise.
    ///
    /// ```text
    /// #1212 EQU          ( 01 )
    /// #1234 EQUk         ( 12 34 00 )
    /// #abcd #ef01 EQU2   ( 00 )
    /// #abcd #abcd EQU2k  ( ab cd ab cd 01 )
    /// ```
    #[inline]
    pub fn equ<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        op_cmp!(vm, FLAGS, |a, b| a == b);
        Some(pc)
    }

    /// Not Equal
    ///
    /// ```text
    /// NEQ a b -- bool8
    /// ```
    ///
    /// Pushes `01` to the stack if the two values at the top of the stack are
    /// not equal, `00` otherwise.
    ///
    /// ```text
    /// #1212 NEQ          ( 00 )
    /// #1234 NEQk         ( 12 34 01 )
    /// #abcd #ef01 NEQ2   ( 01 )
    /// #abcd #abcd NEQ2k  ( ab cd ab cd 00 )
    /// ```
    #[inline]
    pub fn neq<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        op_cmp!(vm, FLAGS, |a, b| a != b);
        Some(pc)
    }

    /// Greater Than
    ///
    /// ```text
    /// GTH a b -- bool8
    /// ```
    ///
    /// Pushes `01` to the stack if the second value at the top of the stack is
    /// greater than the value at the top of the stack, `00` otherwise.
    ///
    /// ```text
    /// #1234 GTH          ( 00 )
    /// #3412 GTHk         ( 34 12 01 )
    /// #3456 #1234 GTH2   ( 01 )
    /// #1234 #3456 GTH2k  ( 12 34 34 56 00 )
    /// ```
    #[inline]
    pub fn gth<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        op_cmp!(vm, FLAGS, |a, b| a > b);
        Some(pc)
    }

    /// Lesser Than
    ///
    /// ```text
    /// LTH a b -- bool8
    /// ```
    ///
    /// Pushes `01` to the stack if the second value at the top of the stack is
    /// lesser than the value at the top of the stack, `00` otherwise.
    ///
    /// ```text
    /// #0101 LTH          ( 00 )
    /// #0100 LTHk         ( 01 00 00 )
    /// #0001 #0000 LTH2   ( 00 )
    /// #0001 #0000 LTH2k  ( 00 01 00 00 00 )
    /// ```
    #[inline]
    pub fn lth<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        op_cmp!(vm, FLAGS, |a, b| a < b);
        Some(pc)
    }

    /// Jump
    ///
    /// ```text
    /// JMP addr --
    /// ```
    ///
    /// Moves the PC by a relative distance equal to the signed byte on the top
    /// of the stack, or to an absolute address in short mode.
    ///
    /// ```text
    /// ,&skip-rel JMP BRK &skip-rel #01  ( 01 )
    /// ```
    #[inline]
    pub fn jmp<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        let mut s = vm.stack_view::<FLAGS>();
        Some(jump_offset(pc, s.pop()))
    }

    /// Jump Conditional
    ///
    /// ```text
    /// JCN cond8 addr --
    /// ```
    ///
    /// If the byte preceeding the address is not `00`, moves the `PC` by a
    /// signed value equal to the byte on the top of the stack, or to an
    /// absolute address in short mode.
    ///
    /// ```text
    /// #abcd #01 ,&pass JCN SWP &pass POP  ( ab )
    /// #abcd #00 ,&fail JCN SWP &fail POP  ( cd )
    /// ```
    #[inline]
    pub fn jcn<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        let mut s = vm.stack_view::<FLAGS>();
        let dst = s.pop();
        let cond = s.pop_byte();
        Some(if cond != 0 { jump_offset(pc, dst) } else { pc })
    }

    /// Jump Stash Return
    ///
    /// ```text
    /// JSR addr -- | ret16
    /// ```
    ///
    /// Pushes the `PC` to the return-stack and moves the `PC` by a signed value
    /// equal to the byte on the top of the stack, or to an absolute address in
    /// short mode.
    ///
    /// ```text
    /// ,&routine JSR                     ( | PC* )
    /// ,&get JSR #01 BRK &get #02 JMP2r  ( 02 01 )
    /// ```
    #[inline]
    pub fn jsr<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        vm.ret.push(Value::Short(pc));
        let mut s = vm.stack_view::<FLAGS>();
        Some(jump_offset(pc, s.pop()))
    }

    /// Stash
    ///
    /// ```text
    /// STH a -- | a
    /// ```
    ///
    /// Moves the value at the top of the stack to the return stack. Note that
    /// with the `r`-mode, the stacks are exchanged and the value is moved from
    /// the return stack to the working stack.
    ///
    /// ```text
    /// #12 STH       ( | 12 )
    /// LITr 34 STHr  ( 34 )
    /// ```
    #[inline]
    pub fn sth<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        let v = vm.stack_view::<FLAGS>().pop();
        vm.ret_stack_view::<FLAGS>().push(v);
        Some(pc)
    }

    /// Load Zero-Page
    ///
    /// ```text
    /// LDZ addr8 -- value
    /// ```
    /// Pushes the value at an address within the first 256 bytes of memory, to
    /// the top of the stack.
    ///
    /// ```text
    /// |00 @cell $2 |0100 .cell LDZ ( 00 )
    /// ```
    #[inline]
    pub fn ldz<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        let addr = vm.stack_view::<FLAGS>().pop_byte();
        let v = vm.ram_read::<FLAGS>(u16::from(addr));
        vm.stack_view::<FLAGS>().push(v);
        Some(pc)
    }

    /// Store Zero-Page
    ///
    /// ```text
    /// STZ val addr8 --
    /// ```
    /// Writes a value to an address within the first 256 bytes of memory.
    ///
    /// ```text
    /// |00 @cell $2 |0100 #abcd .cell STZ2  { ab cd }
    /// ```
    #[inline]
    pub fn stz<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        let mut s = vm.stack_view::<FLAGS>();
        let addr = s.pop_byte();
        let v = s.pop();
        vm.ram_write(u16::from(addr), v);
        Some(pc)
    }

    /// Load Relative
    ///
    /// ```text
    /// LDR addr8 -- value
    /// ```
    ///
    /// Pushes a value at a relative address in relation to the PC, within a
    /// range between -128 and +127 bytes, to the top of the stack.
    ///
    /// ```text
    /// ,cell LDR2 BRK @cell abcd  ( ab cd )
    /// ```
    #[inline]
    pub fn ldr<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        let offset = vm.stack_view::<FLAGS>().pop_byte() as i8;
        let addr = pc.wrapping_add_signed(i16::from(offset));
        let v = vm.ram_read::<FLAGS>(addr);
        vm.stack_view::<FLAGS>().push(v);
        Some(pc)
    }

    /// Store Relative
    ///
    /// ```text
    /// STR val addr8 --
    /// ```
    ///
    /// Writes a value to a relative address in relation to the PC, within a
    /// range between -128 and +127 bytes.
    ///
    /// ```text
    /// #1234 ,cell STR2 BRK @cell $2  ( )
    /// ```
    #[inline]
    pub fn str<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        let mut s = vm.stack_view::<FLAGS>();
        let offset = s.pop_byte() as i8;
        let addr = pc.wrapping_add_signed(i16::from(offset));
        let v = s.pop();
        vm.ram_write(addr, v);
        Some(pc)
    }

    /// Load Absolute
    ///
    /// ```text
    /// LDA addr16 -- value
    /// ```
    ///
    /// Pushes the value at a absolute address, to the top of the stack.
    ///
    /// ```text
    /// ;cell LDA BRK @cell abcd ( ab )
    /// ```
    #[inline]
    pub fn lda<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        let addr = vm.stack_view::<FLAGS>().pop_short();
        let v = vm.ram_read::<FLAGS>(addr);
        vm.stack_view::<FLAGS>().push(v);
        Some(pc)
    }

    /// Store Absolute
    ///
    /// ```text
    /// STA val addr16 --
    /// ```
    ///
    /// Writes a value to a absolute address.
    ///
    /// ```text
    /// #abcd ;cell STA BRK @cell $1 ( ab )
    /// ```
    #[inline]
    pub fn sta<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        let mut s = vm.stack_view::<FLAGS>();
        let addr = s.pop_short();
        let v = s.pop();
        vm.ram_write(addr, v);
        Some(pc)
    }

    /// Device Input
    ///
    /// ```text
    /// DEI device8 -- value
    /// ```
    ///
    /// Pushes a value from the device page, to the top of the stack. The target
    /// device might capture the reading to trigger an I/O event.
    #[inline]
    pub fn dei<const FLAGS: u8>(
        vm: &mut Uxn,
        dev: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        let mut s = vm.stack_view::<FLAGS>();
        let i = s.pop_byte();

        // For compatibility with the C implementation, we'll
        // pre-emtively push a dummy value here, then use `emplace` to
        // replace it afterwards.  This is because the C implementation
        // `uxn.c` reserves stack space before calling `emu_deo/dei`,
        // which affects the behavior of `System.rst/wst`
        let v = if short(FLAGS) {
            s.reserve(2);
            dev.dei(vm, i);
            let hi = vm.dev[i as usize];
            let j = i.wrapping_add(1);
            dev.dei(vm, j);
            let lo = vm.dev[j as usize];
            Value::Short(u16::from_le_bytes([lo, hi]))
        } else {
            s.reserve(1);
            dev.dei(vm, i);
            Value::Byte(vm.dev[i as usize])
        };
        vm.stack_view::<FLAGS>().emplace(v);
        Some(pc)
    }

    /// Device Output
    ///
    /// ```text
    /// DEO val device8 --
    /// ```
    ///
    /// Writes a value to the device page. The target device might capture the
    /// writing to trigger an I/O event.
    #[inline]
    pub fn deo<const FLAGS: u8>(
        vm: &mut Uxn,
        dev: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        let mut s = vm.stack_view::<FLAGS>();
        let i = s.pop_byte();
        let mut run = true;
        match s.pop() {
            Value::Short(v) => {
                let [lo, hi] = v.to_le_bytes();
                let j = i.wrapping_add(1);
                vm.dev[i as usize] = hi;
                run &= dev.deo(vm, i);
                vm.dev[j as usize] = lo;
                run &= dev.deo(vm, j);
            }
            Value::Byte(v) => {
                vm.dev[i as usize] = v;
                run &= dev.deo(vm, i);
            }
        }
        if run {
            Some(pc)
        } else {
            None
        }
    }

    /// Add
    ///
    /// ```text
    /// ADD a b -- a+b
    /// ```
    /// Pushes the sum of the two values at the top of the stack.
    ///
    /// ```text
    /// #1a #2e ADD       ( 48 )
    /// #02 #5d ADDk      ( 02 5d 5f )
    /// #0001 #0002 ADD2  ( 00 03 )
    /// ```
    #[inline]
    pub fn add<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        op_bin!(vm, FLAGS, |a, b| a.wrapping_add(b));
        Some(pc)
    }

    /// Subtract
    ///
    /// ```text
    /// SUB a b -- a-b
    /// ```
    ///
    /// Pushes the difference of the first value minus the second, to the top of
    /// the stack.
    #[inline]
    pub fn sub<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        op_bin!(vm, FLAGS, |a, b| a.wrapping_sub(b));
        Some(pc)
    }

    /// Multiply
    ///
    /// ```text
    /// MUL a b -- a*b
    /// ```
    ///
    /// Pushes the product of the first and second values at the top of the
    /// stack.
    #[inline]
    pub fn mul<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        op_bin!(vm, FLAGS, |a, b| a.wrapping_mul(b));
        Some(pc)
    }

    /// Divide
    ///
    /// ```text
    /// DIV a b -- a/b
    /// ```
    ///
    /// Pushes the quotient of the first value over the second, to the top of
    /// the stack. A division by zero pushes zero on the stack. The rounding
    /// direction is toward zero.
    ///
    /// ```text
    /// #10 #02 DIV       ( 08 )
    /// #10 #03 DIVk      ( 10 03 05 )
    /// #0010 #0000 DIV2  ( 00 00 )
    /// ```
    #[inline]
    pub fn div<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        op_bin!(vm, FLAGS, |a, b| if b != 0 { a / b } else { 0 });
        Some(pc)
    }

    /// And
    ///
    /// ```text
    /// AND a b -- a&b
    /// ```
    ///
    /// Pushes the result of the bitwise operation `AND`, to the top of the
    /// stack.
    #[inline]
    pub fn and<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        op_bin!(vm, FLAGS, |a, b| a & b);
        Some(pc)
    }

    /// Or
    ///
    /// ```text
    /// ORA a b -- a|b
    /// ```
    /// Pushes the result of the bitwise operation `OR`, to the top of the stack.
    #[inline]
    pub fn ora<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        op_bin!(vm, FLAGS, |a, b| a | b);
        Some(pc)
    }

    /// Exclusive Or
    ///
    /// ```text
    /// EOR a b -- a^b
    /// ```
    ///
    /// Pushes the result of the bitwise operation `XOR`, to the top of the
    /// stack.
    #[inline]
    pub fn eor<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        op_bin!(vm, FLAGS, |a, b| a ^ b);
        Some(pc)
    }

    /// Shift
    ///
    /// ```text
    /// SFT a shift8 -- c
    /// ```
    ///
    /// Shifts the bits of the second value of the stack to the left or right,
    /// depending on the control value at the top of the stack. The high nibble of
    /// the control value indicates how many bits to shift left, and the low nibble
    /// how many bits to shift right. The rightward shift is done first.
    ///
    /// ```text
    /// #34 #10 SFT        ( 68 )
    /// #34 #01 SFT        ( 1a )
    /// #34 #33 SFTk       ( 34 33 30 )
    /// #1248 #34 SFT2k    ( 12 48 34 09 20 )
    /// ```
    #[inline]
    pub fn sft<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        let mut s = vm.stack_view::<FLAGS>();
        let shift = s.pop_byte();
        let shr = u32::from(shift & 0xF);
        let shl = u32::from(shift >> 4);
        let v = s.pop();
        s.push(v.wrapping_shr(shr).wrapping_shl(shl));
        Some(pc)
    }
}

/// Trait for a Uxn-compatible device
pub trait Device {
    /// Performs the `DEI` operation for the given target
    ///
    /// The output byte (if any) must be written to `vm.dev[target]`, and can be
    /// read after this function returns.
    fn dei(&mut self, vm: &mut Uxn, target: u8);

    /// Performs the `DEO` operation on the given target
    ///
    /// The input byte (if any) will be read from `vm.dev[target]`, and must be
    /// stored before this function is called.
    ///
    /// Returns `true` if the CPU should keep running, `false` if it should
    /// exit.
    #[must_use]
    fn deo(&mut self, vm: &mut Uxn, target: u8) -> bool;
}

/// Trait for a type which can be cast to a device ports `struct`
pub trait Ports:
    zerocopy::AsBytes + zerocopy::FromBytes + zerocopy::FromZeroes
{
    /// Base address of the port, of the form `0xA0`
    const BASE: u8;
}

/// Device which does nothing
pub struct EmptyDevice;
impl Device for EmptyDevice {
    fn dei(&mut self, _vm: &mut Uxn, _target: u8) {
        // nothing to do here
    }
    fn deo(&mut self, _vm: &mut Uxn, _target: u8) -> bool {
        // nothing to do here, keep running
        true
    }
}

#[cfg(feature = "alloc")]
mod ram {
    extern crate alloc;
    use alloc::boxed::Box;

    /// Helper type for building a RAM array of the appropriate size
    ///
    /// This is only available if the `"alloc"` feature is enabled
    pub struct UxnRam(Box<[u8; 65536]>);

    impl UxnRam {
        /// Builds a new zero-initialized RAM
        pub fn new() -> Self {
            UxnRam(Box::new([0u8; 65536]))
        }
    }

    impl Default for UxnRam {
        fn default() -> Self {
            Self::new()
        }
    }

    impl core::ops::Deref for UxnRam {
        type Target = [u8; 65536];
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl core::ops::DerefMut for UxnRam {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
}

#[cfg(feature = "alloc")]
pub use ram::UxnRam;

#[cfg(all(feature = "alloc", test))]
mod test {
    use super::*;

    /// Simple parser for textual opcodes
    fn decode_op(s: &str) -> Result<u8, &str> {
        let (s, ret) =
            s.strip_suffix('r').map(|s| (s, true)).unwrap_or((s, false));
        let (s, keep) =
            s.strip_suffix('k').map(|s| (s, true)).unwrap_or((s, false));
        let (s, short) =
            s.strip_suffix('2').map(|s| (s, true)).unwrap_or((s, false));
        let mode =
            ((keep as u8) << 7) | ((ret as u8) << 6) | ((short as u8) << 5);
        let out = match s {
            "BRK" => 0x00,
            "JCI" => 0x20,
            "JMI" => 0x40,
            "JSI" => 0x60,
            "LIT" => 0x80 | mode,

            "INC" => 0x01 | mode,
            "POP" => 0x02 | mode,
            "NIP" => 0x03 | mode,
            "SWP" => 0x04 | mode,
            "ROT" => 0x05 | mode,
            "DUP" => 0x06 | mode,
            "OVR" => 0x07 | mode,
            "EQU" => 0x08 | mode,
            "NEQ" => 0x09 | mode,
            "GTH" => 0x0a | mode,
            "LTH" => 0x0b | mode,
            "JMP" => 0x0c | mode,
            "JCN" => 0x0d | mode,
            "JSR" => 0x0e | mode,
            "STH" => 0x0f | mode,
            "LDZ" => 0x10 | mode,
            "STZ" => 0x11 | mode,
            "LDR" => 0x12 | mode,
            "STR" => 0x13 | mode,
            "LDA" => 0x14 | mode,
            "STA" => 0x15 | mode,
            "DEI" => 0x16 | mode,
            "DEO" => 0x17 | mode,
            "ADD" => 0x18 | mode,
            "SUB" => 0x19 | mode,
            "MUL" => 0x1a | mode,
            "DIV" => 0x1b | mode,
            "AND" => 0x1c | mode,
            "ORA" => 0x1d | mode,
            "EOR" => 0x1e | mode,
            "SFT" => 0x1f | mode,
            _ => return Err(s),
        };
        Ok(out)
    }

    fn parse_and_test(s: &str) {
        let mut ram = UxnRam::new();
        let mut vm = Uxn::new(&[], &mut ram);
        let mut iter = s.split_whitespace();
        let mut op = None;
        let mut dev = EmptyDevice;
        while let Some(i) = iter.next() {
            if let Some(s) = i.strip_prefix('#') {
                match s.len() {
                    2 => {
                        let v = u8::from_str_radix(s, 16).unwrap();
                        vm.stack.push_byte(v);
                    }
                    4 => {
                        let v = u16::from_str_radix(s, 16).unwrap();
                        vm.stack.push_short(v);
                    }
                    _ => panic!("invalid length for literal: {i:?}"),
                }
                continue;
            } else if i == "(" {
                let mut expected: Vec<u8> = vec![];
                for s in iter {
                    if s == ")" {
                        break;
                    } else {
                        expected.push(u8::from_str_radix(s, 16).unwrap());
                    }
                }
                vm.op(op.unwrap(), &mut dev, 0);
                let mut actual = vec![];
                while vm.stack.index != u8::MAX {
                    actual.push(vm.stack.pop_byte());
                }
                actual.reverse();
                if actual != expected {
                    panic!(
                        "failed to execute {:?}: got {actual:2x?}, \
                         expected {expected:2x?}",
                        s.trim()
                    );
                }
                break;
            } else {
                op = Some(decode_op(i).unwrap());
            }
        }
    }

    #[test]
    fn opcodes() {
        const TEST_SUITE: &str = "
            #01 INC         ( 02 )
            #0001 INC2      ( 00 02 )
            #0001 INC2k     ( 00 01 00 02 )
            #1234 POP    ( 12 )
            #1234 POP2   ( )
            #1234 POP2k  ( 12 34 )
            #1234 NIP          ( 34 )
            #1234 #5678 NIP2   ( 56 78 )
            #1234 #5678 NIP2k  ( 12 34 56 78 56 78 )
            #1234 SWP          ( 34 12 )
            #1234 SWPk         ( 12 34 34 12 )
            #1234 #5678 SWP2   ( 56 78 12 34 )
            #1234 #5678 SWP2k  ( 12 34 56 78 56 78 12 34 )
            #1234 #56 ROT            ( 34 56 12 )
            #1234 #56 ROTk           ( 12 34 56 34 56 12 )
            #1234 #5678 #9abc ROT2   ( 56 78 9a bc 12 34 )
            #1234 #5678 #9abc ROT2k  ( 12 34 56 78 9a bc 56 78 9a bc 12 34 )
            #1234 DUP   ( 12 34 34 )
            #12 DUPk    ( 12 12 12 )
            #1234 DUP2  ( 12 34 12 34 )
            #1234 OVR          ( 12 34 12 )
            #1234 OVRk         ( 12 34 12 34 12 )
            #1234 #5678 OVR2   ( 12 34 56 78 12 34 )
            #1234 #5678 OVR2k  ( 12 34 56 78 12 34 56 78 12 34 )
            #1212 EQU          ( 01 )
            #1234 EQUk         ( 12 34 00 )
            #abcd #ef01 EQU2   ( 00 )
            #abcd #abcd EQU2k  ( ab cd ab cd 01 )
            #1212 NEQ          ( 00 )
            #1234 NEQk         ( 12 34 01 )
            #abcd #ef01 NEQ2   ( 01 )
            #abcd #abcd NEQ2k  ( ab cd ab cd 00 )
            #1234 GTH          ( 00 )
            #3412 GTHk         ( 34 12 01 )
            #3456 #1234 GTH2   ( 01 )
            #1234 #3456 GTH2k  ( 12 34 34 56 00 )
            #0101 LTH          ( 00 )
            #0100 LTHk         ( 01 00 00 )
            #0001 #0000 LTH2   ( 00 )
            #0001 #0000 LTH2k  ( 00 01 00 00 00 )
            #1a #2e ADD       ( 48 )
            #02 #5d ADDk      ( 02 5d 5f )
            #0001 #0002 ADD2  ( 00 03 )
            #10 #02 DIV       ( 08 )
            #10 #03 DIVk      ( 10 03 05 )
            #0010 #0000 DIV2  ( 00 00 )
            #34 #10 SFT        ( 68 )
            #34 #01 SFT        ( 1a )
            #34 #33 SFTk       ( 34 33 30 )
            #1248 #34 SFT2k    ( 12 48 34 09 20 )
        ";
        for line in TEST_SUITE.lines() {
            parse_and_test(line);
        }

        #[allow(dead_code)]
        const HARD_TESTS: &str = "
            LIT 12          ( 12 )
            LIT2 abcd       ( ab cd )
            ,&skip-rel JMP BRK &skip-rel #01  ( 01 )
            #abcd #01 ,&pass JCN SWP &pass POP  ( ab )
            #abcd #00 ,&fail JCN SWP &fail POP  ( cd )
            ,&routine JSR                     ( | PC* )
            ,&get JSR #01 BRK &get #02 JMP2r  ( 02 01 )
            #12 STH       ( | 12 )
            LITr 34 STHr  ( 34 )
            |00 @cell $2 |0100 .cell LDZ ( 00 )
            |00 @cell $2 |0100 #abcd .cell STZ2  { ab cd }
            ,cell LDR2 BRK @cell abcd  ( ab cd )
            #1234 ,cell STR2 BRK @cell $2  ( )
            ;cell LDA BRK @cell abcd ( ab )
            #abcd ;cell STA BRK @cell $1 ( ab )
        ";
    }
}
