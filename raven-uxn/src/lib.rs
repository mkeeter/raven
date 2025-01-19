//! Uxn virtual machine
#![cfg_attr(not(test), no_std)]
#![warn(missing_docs)]
#![cfg_attr(not(feature = "native"), forbid(unsafe_code))]

#[cfg(feature = "native")]
mod native;

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
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Stack {
    data: [u8; 256],

    /// The index points to the last occupied slot, and increases on `push`
    ///
    /// If the buffer is empty or full, it points to `u8::MAX`.
    index: u8,
}

/// Uxn evaluation backend
#[derive(Copy, Clone, Debug)]
pub enum Backend {
    /// Use a bytecode interpreter
    Interpreter,

    #[cfg(feature = "native")]
    /// Use hand-written threaded assembly
    Native,
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
    fn shr(&self, i: u32) -> Self {
        match self {
            Value::Short(v) => Value::Short(v.checked_shr(i).unwrap_or(0)),
            Value::Byte(v) => Value::Byte(v.checked_shr(i).unwrap_or(0)),
        }
    }
    #[inline]
    fn shl(&self, i: u32) -> Self {
        match self {
            Value::Short(v) => Value::Short(v.checked_shl(i).unwrap_or(0)),
            Value::Byte(v) => Value::Byte(v.checked_shl(i).unwrap_or(0)),
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

    /// Preferred evaluation backend
    backend: Backend,
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
        s.push_byte(u8::from(v));
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
    /// Build a new `Uxn` with zeroed memory
    pub fn new(ram: &'a mut [u8; 65536], backend: Backend) -> Self {
        Self {
            dev: [0u8; 256],
            ram,
            stack: Stack::default(),
            ret: Stack::default(),
            backend,
        }
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

    #[inline]
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

    /// Reads a word from RAM
    ///
    /// If the address is at the top of RAM, the second byte will wrap to 0
    #[inline]
    pub fn ram_read_word(&self, addr: u16) -> u16 {
        let hi = self.ram[usize::from(addr)];
        let lo = self.ram[usize::from(addr.wrapping_add(1))];
        u16::from_le_bytes([lo, hi])
    }

    /// Writes to the given address in device memory
    #[inline]
    pub fn write_dev_mem(&mut self, addr: u8, value: u8) {
        self.dev[usize::from(addr)] = value;
    }

    /// Runs the VM starting at the given address until it terminates
    #[inline]
    pub fn run<D: Device>(&mut self, dev: &mut D, mut pc: u16) -> u16 {
        match self.backend {
            Backend::Interpreter => loop {
                let op = self.next(&mut pc);
                let Some(next) = self.op(op, dev, pc) else {
                    break pc;
                };
                pc = next;
            },
            #[cfg(feature = "native")]
            Backend::Native => native::entry(self, dev, pc),
        }
    }

    /// Runs until the program terminates or we hit a stop condition
    ///
    /// Returns the new program counter if the program terminated, or `None` if
    /// the stop condition was reached.
    ///
    /// This function always uses the interpreter, ignoring
    /// [`self.backend`](Self::backend).
    #[inline]
    pub fn run_until<D: Device, F: Fn(&Self, &D, usize) -> bool>(
        &mut self,
        dev: &mut D,
        mut pc: u16,
        stop: F,
    ) -> Option<u16> {
        for i in 0.. {
            let op = self.next(&mut pc);
            let Some(next) = self.op(op, dev, pc) else {
                return Some(pc);
            };
            pc = next;
            if stop(self, dev, i) {
                return None;
            }
        }
        unreachable!()
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
        D::ref_from(&self.dev[usize::from(pos)..][..DEV_SIZE]).unwrap()
    }

    /// Returns a reference to a device located at `pos`
    #[inline]
    pub fn dev_mut_at<D: Ports>(&mut self, pos: u8) -> &mut D {
        Self::check_dev_size::<D>();
        D::mut_from(&mut self.dev[usize::from(pos)..][..DEV_SIZE]).unwrap()
    }

    /// Returns a mutable reference to the given [`Ports`] object
    #[inline]
    pub fn dev_mut<D: Ports>(&mut self) -> &mut D {
        self.dev_mut_at(D::BASE)
    }

    /// Reads a byte from RAM
    #[inline]
    pub fn ram_read_byte(&self, addr: u16) -> u8 {
        self.ram[usize::from(addr)]
    }

    /// Writes a byte to RAM
    #[inline]
    pub fn ram_write_byte(&mut self, addr: u16, v: u8) {
        self.ram[usize::from(addr)] = v;
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

    /// Resets system memory and loads the given ROM
    ///
    /// Returns trailing ROM data (or an empty slice), which should be loaded
    /// into extension memory.
    #[must_use]
    pub fn reset<'b>(&mut self, rom: &'b [u8]) -> &'b [u8] {
        self.dev.fill(0);
        self.ram.fill(0);
        self.stack = Stack::default();
        self.ret = Stack::default();
        let n = (self.ram.len() - 0x100).min(rom.len());
        self.ram[0x100..][..n].copy_from_slice(&rom[..n]);
        &rom[n..]
    }

    /// Asserts that the given [`Ports`] object is of size [`DEV_SIZE`]
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

    /// Executes a single operation
    #[inline]
    fn op<D: Device>(&mut self, op: u8, dev: &mut D, pc: u16) -> Option<u16> {
        match op {
            op::BRK => self.brk(pc),
            op::INC => self.inc::<0b000>(pc),
            op::POP => self.pop::<0b000>(pc),
            op::NIP => self.nip::<0b000>(pc),
            op::SWP => self.swp::<0b000>(pc),
            op::ROT => self.rot::<0b000>(pc),
            op::DUP => self.dup::<0b000>(pc),
            op::OVR => self.ovr::<0b000>(pc),
            op::EQU => self.equ::<0b000>(pc),
            op::NEQ => self.neq::<0b000>(pc),
            op::GTH => self.gth::<0b000>(pc),
            op::LTH => self.lth::<0b000>(pc),
            op::JMP => self.jmp::<0b000>(pc),
            op::JCN => self.jcn::<0b000>(pc),
            op::JSR => self.jsr::<0b000>(pc),
            op::STH => self.sth::<0b000>(pc),
            op::LDZ => self.ldz::<0b000>(pc),
            op::STZ => self.stz::<0b000>(pc),
            op::LDR => self.ldr::<0b000>(pc),
            op::STR => self.str::<0b000>(pc),
            op::LDA => self.lda::<0b000>(pc),
            op::STA => self.sta::<0b000>(pc),
            op::DEI => self.dei::<0b000>(dev, pc),
            op::DEO => self.deo::<0b000>(dev, pc),
            op::ADD => self.add::<0b000>(pc),
            op::SUB => self.sub::<0b000>(pc),
            op::MUL => self.mul::<0b000>(pc),
            op::DIV => self.div::<0b000>(pc),
            op::AND => self.and::<0b000>(pc),
            op::ORA => self.ora::<0b000>(pc),
            op::EOR => self.eor::<0b000>(pc),
            op::SFT => self.sft::<0b000>(pc),
            op::JCI => self.jci(pc),
            op::INC2 => self.inc::<0b001>(pc),
            op::POP2 => self.pop::<0b001>(pc),
            op::NIP2 => self.nip::<0b001>(pc),
            op::SWP2 => self.swp::<0b001>(pc),
            op::ROT2 => self.rot::<0b001>(pc),
            op::DUP2 => self.dup::<0b001>(pc),
            op::OVR2 => self.ovr::<0b001>(pc),
            op::EQU2 => self.equ::<0b001>(pc),
            op::NEQ2 => self.neq::<0b001>(pc),
            op::GTH2 => self.gth::<0b001>(pc),
            op::LTH2 => self.lth::<0b001>(pc),
            op::JMP2 => self.jmp::<0b001>(pc),
            op::JCN2 => self.jcn::<0b001>(pc),
            op::JSR2 => self.jsr::<0b001>(pc),
            op::STH2 => self.sth::<0b001>(pc),
            op::LDZ2 => self.ldz::<0b001>(pc),
            op::STZ2 => self.stz::<0b001>(pc),
            op::LDR2 => self.ldr::<0b001>(pc),
            op::STR2 => self.str::<0b001>(pc),
            op::LDA2 => self.lda::<0b001>(pc),
            op::STA2 => self.sta::<0b001>(pc),
            op::DEI2 => self.dei::<0b001>(dev, pc),
            op::DEO2 => self.deo::<0b001>(dev, pc),
            op::ADD2 => self.add::<0b001>(pc),
            op::SUB2 => self.sub::<0b001>(pc),
            op::MUL2 => self.mul::<0b001>(pc),
            op::DIV2 => self.div::<0b001>(pc),
            op::AND2 => self.and::<0b001>(pc),
            op::ORA2 => self.ora::<0b001>(pc),
            op::EOR2 => self.eor::<0b001>(pc),
            op::SFT2 => self.sft::<0b001>(pc),
            op::JMI => self.jmi(pc),
            op::INCr => self.inc::<0b010>(pc),
            op::POPr => self.pop::<0b010>(pc),
            op::NIPr => self.nip::<0b010>(pc),
            op::SWPr => self.swp::<0b010>(pc),
            op::ROTr => self.rot::<0b010>(pc),
            op::DUPr => self.dup::<0b010>(pc),
            op::OVRr => self.ovr::<0b010>(pc),
            op::EQUr => self.equ::<0b010>(pc),
            op::NEQr => self.neq::<0b010>(pc),
            op::GTHr => self.gth::<0b010>(pc),
            op::LTHr => self.lth::<0b010>(pc),
            op::JMPr => self.jmp::<0b010>(pc),
            op::JCNr => self.jcn::<0b010>(pc),
            op::JSRr => self.jsr::<0b010>(pc),
            op::STHr => self.sth::<0b010>(pc),
            op::LDZr => self.ldz::<0b010>(pc),
            op::STZr => self.stz::<0b010>(pc),
            op::LDRr => self.ldr::<0b010>(pc),
            op::STRr => self.str::<0b010>(pc),
            op::LDAr => self.lda::<0b010>(pc),
            op::STAr => self.sta::<0b010>(pc),
            op::DEIr => self.dei::<0b010>(dev, pc),
            op::DEOr => self.deo::<0b010>(dev, pc),
            op::ADDr => self.add::<0b010>(pc),
            op::SUBr => self.sub::<0b010>(pc),
            op::MULr => self.mul::<0b010>(pc),
            op::DIVr => self.div::<0b010>(pc),
            op::ANDr => self.and::<0b010>(pc),
            op::ORAr => self.ora::<0b010>(pc),
            op::EORr => self.eor::<0b010>(pc),
            op::SFTr => self.sft::<0b010>(pc),
            op::JSI => self.jsi(pc),
            op::INC2r => self.inc::<0b011>(pc),
            op::POP2r => self.pop::<0b011>(pc),
            op::NIP2r => self.nip::<0b011>(pc),
            op::SWP2r => self.swp::<0b011>(pc),
            op::ROT2r => self.rot::<0b011>(pc),
            op::DUP2r => self.dup::<0b011>(pc),
            op::OVR2r => self.ovr::<0b011>(pc),
            op::EQU2r => self.equ::<0b011>(pc),
            op::NEQ2r => self.neq::<0b011>(pc),
            op::GTH2r => self.gth::<0b011>(pc),
            op::LTH2r => self.lth::<0b011>(pc),
            op::JMP2r => self.jmp::<0b011>(pc),
            op::JCN2r => self.jcn::<0b011>(pc),
            op::JSR2r => self.jsr::<0b011>(pc),
            op::STH2r => self.sth::<0b011>(pc),
            op::LDZ2r => self.ldz::<0b011>(pc),
            op::STZ2r => self.stz::<0b011>(pc),
            op::LDR2r => self.ldr::<0b011>(pc),
            op::STR2r => self.str::<0b011>(pc),
            op::LDA2r => self.lda::<0b011>(pc),
            op::STA2r => self.sta::<0b011>(pc),
            op::DEI2r => self.dei::<0b011>(dev, pc),
            op::DEO2r => self.deo::<0b011>(dev, pc),
            op::ADD2r => self.add::<0b011>(pc),
            op::SUB2r => self.sub::<0b011>(pc),
            op::MUL2r => self.mul::<0b011>(pc),
            op::DIV2r => self.div::<0b011>(pc),
            op::AND2r => self.and::<0b011>(pc),
            op::ORA2r => self.ora::<0b011>(pc),
            op::EOR2r => self.eor::<0b011>(pc),
            op::SFT2r => self.sft::<0b011>(pc),
            op::LIT => self.lit::<0b100>(pc),
            op::INCk => self.inc::<0b100>(pc),
            op::POPk => self.pop::<0b100>(pc),
            op::NIPk => self.nip::<0b100>(pc),
            op::SWPk => self.swp::<0b100>(pc),
            op::ROTk => self.rot::<0b100>(pc),
            op::DUPk => self.dup::<0b100>(pc),
            op::OVRk => self.ovr::<0b100>(pc),
            op::EQUk => self.equ::<0b100>(pc),
            op::NEQk => self.neq::<0b100>(pc),
            op::GTHk => self.gth::<0b100>(pc),
            op::LTHk => self.lth::<0b100>(pc),
            op::JMPk => self.jmp::<0b100>(pc),
            op::JCNk => self.jcn::<0b100>(pc),
            op::JSRk => self.jsr::<0b100>(pc),
            op::STHk => self.sth::<0b100>(pc),
            op::LDZk => self.ldz::<0b100>(pc),
            op::STZk => self.stz::<0b100>(pc),
            op::LDRk => self.ldr::<0b100>(pc),
            op::STRk => self.str::<0b100>(pc),
            op::LDAk => self.lda::<0b100>(pc),
            op::STAk => self.sta::<0b100>(pc),
            op::DEIk => self.dei::<0b100>(dev, pc),
            op::DEOk => self.deo::<0b100>(dev, pc),
            op::ADDk => self.add::<0b100>(pc),
            op::SUBk => self.sub::<0b100>(pc),
            op::MULk => self.mul::<0b100>(pc),
            op::DIVk => self.div::<0b100>(pc),
            op::ANDk => self.and::<0b100>(pc),
            op::ORAk => self.ora::<0b100>(pc),
            op::EORk => self.eor::<0b100>(pc),
            op::SFTk => self.sft::<0b100>(pc),
            op::LIT2 => self.lit::<0b101>(pc),
            op::INC2k => self.inc::<0b101>(pc),
            op::POP2k => self.pop::<0b101>(pc),
            op::NIP2k => self.nip::<0b101>(pc),
            op::SWP2k => self.swp::<0b101>(pc),
            op::ROT2k => self.rot::<0b101>(pc),
            op::DUP2k => self.dup::<0b101>(pc),
            op::OVR2k => self.ovr::<0b101>(pc),
            op::EQU2k => self.equ::<0b101>(pc),
            op::NEQ2k => self.neq::<0b101>(pc),
            op::GTH2k => self.gth::<0b101>(pc),
            op::LTH2k => self.lth::<0b101>(pc),
            op::JMP2k => self.jmp::<0b101>(pc),
            op::JCN2k => self.jcn::<0b101>(pc),
            op::JSR2k => self.jsr::<0b101>(pc),
            op::STH2k => self.sth::<0b101>(pc),
            op::LDZ2k => self.ldz::<0b101>(pc),
            op::STZ2k => self.stz::<0b101>(pc),
            op::LDR2k => self.ldr::<0b101>(pc),
            op::STR2k => self.str::<0b101>(pc),
            op::LDA2k => self.lda::<0b101>(pc),
            op::STA2k => self.sta::<0b101>(pc),
            op::DEI2k => self.dei::<0b101>(dev, pc),
            op::DEO2k => self.deo::<0b101>(dev, pc),
            op::ADD2k => self.add::<0b101>(pc),
            op::SUB2k => self.sub::<0b101>(pc),
            op::MUL2k => self.mul::<0b101>(pc),
            op::DIV2k => self.div::<0b101>(pc),
            op::AND2k => self.and::<0b101>(pc),
            op::ORA2k => self.ora::<0b101>(pc),
            op::EOR2k => self.eor::<0b101>(pc),
            op::SFT2k => self.sft::<0b101>(pc),
            op::LITr => self.lit::<0b110>(pc),
            op::INCkr => self.inc::<0b110>(pc),
            op::POPkr => self.pop::<0b110>(pc),
            op::NIPkr => self.nip::<0b110>(pc),
            op::SWPkr => self.swp::<0b110>(pc),
            op::ROTkr => self.rot::<0b110>(pc),
            op::DUPkr => self.dup::<0b110>(pc),
            op::OVRkr => self.ovr::<0b110>(pc),
            op::EQUkr => self.equ::<0b110>(pc),
            op::NEQkr => self.neq::<0b110>(pc),
            op::GTHkr => self.gth::<0b110>(pc),
            op::LTHkr => self.lth::<0b110>(pc),
            op::JMPkr => self.jmp::<0b110>(pc),
            op::JCNkr => self.jcn::<0b110>(pc),
            op::JSRkr => self.jsr::<0b110>(pc),
            op::STHkr => self.sth::<0b110>(pc),
            op::LDZkr => self.ldz::<0b110>(pc),
            op::STZkr => self.stz::<0b110>(pc),
            op::LDRkr => self.ldr::<0b110>(pc),
            op::STRkr => self.str::<0b110>(pc),
            op::LDAkr => self.lda::<0b110>(pc),
            op::STAkr => self.sta::<0b110>(pc),
            op::DEIkr => self.dei::<0b110>(dev, pc),
            op::DEOkr => self.deo::<0b110>(dev, pc),
            op::ADDkr => self.add::<0b110>(pc),
            op::SUBkr => self.sub::<0b110>(pc),
            op::MULkr => self.mul::<0b110>(pc),
            op::DIVkr => self.div::<0b110>(pc),
            op::ANDkr => self.and::<0b110>(pc),
            op::ORAkr => self.ora::<0b110>(pc),
            op::EORkr => self.eor::<0b110>(pc),
            op::SFTkr => self.sft::<0b110>(pc),
            op::LIT2r => self.lit::<0b111>(pc),
            op::INC2kr => self.inc::<0b111>(pc),
            op::POP2kr => self.pop::<0b111>(pc),
            op::NIP2kr => self.nip::<0b111>(pc),
            op::SWP2kr => self.swp::<0b111>(pc),
            op::ROT2kr => self.rot::<0b111>(pc),
            op::DUP2kr => self.dup::<0b111>(pc),
            op::OVR2kr => self.ovr::<0b111>(pc),
            op::EQU2kr => self.equ::<0b111>(pc),
            op::NEQ2kr => self.neq::<0b111>(pc),
            op::GTH2kr => self.gth::<0b111>(pc),
            op::LTH2kr => self.lth::<0b111>(pc),
            op::JMP2kr => self.jmp::<0b111>(pc),
            op::JCN2kr => self.jcn::<0b111>(pc),
            op::JSR2kr => self.jsr::<0b111>(pc),
            op::STH2kr => self.sth::<0b111>(pc),
            op::LDZ2kr => self.ldz::<0b111>(pc),
            op::STZ2kr => self.stz::<0b111>(pc),
            op::LDR2kr => self.ldr::<0b111>(pc),
            op::STR2kr => self.str::<0b111>(pc),
            op::LDA2kr => self.lda::<0b111>(pc),
            op::STA2kr => self.sta::<0b111>(pc),
            op::DEI2kr => self.dei::<0b111>(dev, pc),
            op::DEO2kr => self.deo::<0b111>(dev, pc),
            op::ADD2kr => self.add::<0b111>(pc),
            op::SUB2kr => self.sub::<0b111>(pc),
            op::MUL2kr => self.mul::<0b111>(pc),
            op::DIV2kr => self.div::<0b111>(pc),
            op::AND2kr => self.and::<0b111>(pc),
            op::ORA2kr => self.ora::<0b111>(pc),
            op::EOR2kr => self.eor::<0b111>(pc),
            op::SFT2kr => self.sft::<0b111>(pc),
        }
    }

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
    pub fn brk(&mut self, _: u16) -> Option<u16> {
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
    pub fn jci(&mut self, mut pc: u16) -> Option<u16> {
        let dt = self.next2(&mut pc);
        if self.stack.pop_byte() != 0 {
            pc = pc.wrapping_add(dt);
        }
        Some(pc)
    }

    /// Jump Instant
    ///
    /// JMI  -- Moves the PC to a relative address at a distance equal to the
    /// next short in memory. This opcode has no modes.
    #[inline]
    pub fn jmi(&mut self, mut pc: u16) -> Option<u16> {
        let dt = self.next2(&mut pc);
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
    pub fn jsi(&mut self, mut pc: u16) -> Option<u16> {
        let dt = self.next2(&mut pc);
        self.ret.push(Value::Short(pc));
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
    pub fn lit<const FLAGS: u8>(&mut self, mut pc: u16) -> Option<u16> {
        let v = if short(FLAGS) {
            Value::Short(self.next2(&mut pc))
        } else {
            Value::Byte(self.next(&mut pc))
        };
        self.stack_view::<FLAGS>().push(v);
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
    pub fn inc<const FLAGS: u8>(&mut self, pc: u16) -> Option<u16> {
        let mut s = self.stack_view::<FLAGS>();
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
    pub fn pop<const FLAGS: u8>(&mut self, pc: u16) -> Option<u16> {
        self.stack_view::<FLAGS>().pop();
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
    pub fn nip<const FLAGS: u8>(&mut self, pc: u16) -> Option<u16> {
        let mut s = self.stack_view::<FLAGS>();
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
    pub fn swp<const FLAGS: u8>(&mut self, pc: u16) -> Option<u16> {
        let mut s = self.stack_view::<FLAGS>();
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
    pub fn rot<const FLAGS: u8>(&mut self, pc: u16) -> Option<u16> {
        let mut s = self.stack_view::<FLAGS>();
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
    pub fn dup<const FLAGS: u8>(&mut self, pc: u16) -> Option<u16> {
        let mut s = self.stack_view::<FLAGS>();
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
    pub fn ovr<const FLAGS: u8>(&mut self, pc: u16) -> Option<u16> {
        let mut s = self.stack_view::<FLAGS>();
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
    pub fn equ<const FLAGS: u8>(&mut self, pc: u16) -> Option<u16> {
        op_cmp!(self, FLAGS, |a, b| a == b);
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
    pub fn neq<const FLAGS: u8>(&mut self, pc: u16) -> Option<u16> {
        op_cmp!(self, FLAGS, |a, b| a != b);
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
    pub fn gth<const FLAGS: u8>(&mut self, pc: u16) -> Option<u16> {
        op_cmp!(self, FLAGS, |a, b| a > b);
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
    pub fn lth<const FLAGS: u8>(&mut self, pc: u16) -> Option<u16> {
        op_cmp!(self, FLAGS, |a, b| a < b);
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
    pub fn jmp<const FLAGS: u8>(&mut self, pc: u16) -> Option<u16> {
        let mut s = self.stack_view::<FLAGS>();
        Some(Self::jump_offset(pc, s.pop()))
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
    pub fn jcn<const FLAGS: u8>(&mut self, pc: u16) -> Option<u16> {
        let mut s = self.stack_view::<FLAGS>();
        let dst = s.pop();
        let cond = s.pop_byte();
        Some(if cond != 0 {
            Self::jump_offset(pc, dst)
        } else {
            pc
        })
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
    pub fn jsr<const FLAGS: u8>(&mut self, pc: u16) -> Option<u16> {
        self.ret_stack_view::<FLAGS>().push(Value::Short(pc));
        let mut s = self.stack_view::<FLAGS>();
        Some(Self::jump_offset(pc, s.pop()))
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
    pub fn sth<const FLAGS: u8>(&mut self, pc: u16) -> Option<u16> {
        let v = self.stack_view::<FLAGS>().pop();
        self.ret_stack_view::<FLAGS>().push(v);
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
    pub fn ldz<const FLAGS: u8>(&mut self, pc: u16) -> Option<u16> {
        let addr = self.stack_view::<FLAGS>().pop_byte();
        let v = self.ram_read::<FLAGS>(u16::from(addr));
        self.stack_view::<FLAGS>().push(v);
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
    pub fn stz<const FLAGS: u8>(&mut self, pc: u16) -> Option<u16> {
        let mut s = self.stack_view::<FLAGS>();
        let addr = s.pop_byte();
        let v = s.pop();
        self.ram_write(u16::from(addr), v);
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
    pub fn ldr<const FLAGS: u8>(&mut self, pc: u16) -> Option<u16> {
        let offset = self.stack_view::<FLAGS>().pop_byte() as i8;
        let addr = pc.wrapping_add_signed(i16::from(offset));
        let v = self.ram_read::<FLAGS>(addr);
        self.stack_view::<FLAGS>().push(v);
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
    pub fn str<const FLAGS: u8>(&mut self, pc: u16) -> Option<u16> {
        let mut s = self.stack_view::<FLAGS>();
        let offset = s.pop_byte() as i8;
        let addr = pc.wrapping_add_signed(i16::from(offset));
        let v = s.pop();
        self.ram_write(addr, v);
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
    pub fn lda<const FLAGS: u8>(&mut self, pc: u16) -> Option<u16> {
        let addr = self.stack_view::<FLAGS>().pop_short();
        let v = self.ram_read::<FLAGS>(addr);
        self.stack_view::<FLAGS>().push(v);
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
    pub fn sta<const FLAGS: u8>(&mut self, pc: u16) -> Option<u16> {
        let mut s = self.stack_view::<FLAGS>();
        let addr = s.pop_short();
        let v = s.pop();
        self.ram_write(addr, v);
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
        &mut self,
        dev: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        let mut s = self.stack_view::<FLAGS>();
        let i = s.pop_byte();

        // For compatibility with the C implementation, we'll
        // pre-emtively push a dummy value here, then use `emplace` to
        // replace it afterwards.  This is because the C implementation
        // `uxn.c` reserves stack space before calling `emu_deo/dei`,
        // which affects the behavior of `System.rst/wst`
        let v = if short(FLAGS) {
            s.reserve(2);
            dev.dei(self, i);
            let hi = self.dev[usize::from(i)];
            let j = i.wrapping_add(1);
            dev.dei(self, j);
            let lo = self.dev[usize::from(j)];
            Value::Short(u16::from_le_bytes([lo, hi]))
        } else {
            s.reserve(1);
            dev.dei(self, i);
            Value::Byte(self.dev[usize::from(i)])
        };
        self.stack_view::<FLAGS>().emplace(v);
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
        &mut self,
        dev: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        let mut s = self.stack_view::<FLAGS>();
        let i = s.pop_byte();
        let mut run = true;
        match s.pop() {
            Value::Short(v) => {
                let [lo, hi] = v.to_le_bytes();
                let j = i.wrapping_add(1);
                self.dev[usize::from(i)] = hi;
                run &= dev.deo(self, i);
                self.dev[usize::from(j)] = lo;
                run &= dev.deo(self, j);
            }
            Value::Byte(v) => {
                self.dev[usize::from(i)] = v;
                run &= dev.deo(self, i);
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
    pub fn add<const FLAGS: u8>(&mut self, pc: u16) -> Option<u16> {
        op_bin!(self, FLAGS, |a, b| a.wrapping_add(b));
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
    pub fn sub<const FLAGS: u8>(&mut self, pc: u16) -> Option<u16> {
        op_bin!(self, FLAGS, |a, b| a.wrapping_sub(b));
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
    pub fn mul<const FLAGS: u8>(&mut self, pc: u16) -> Option<u16> {
        op_bin!(self, FLAGS, |a, b| a.wrapping_mul(b));
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
    pub fn div<const FLAGS: u8>(&mut self, pc: u16) -> Option<u16> {
        op_bin!(self, FLAGS, |a, b| if b != 0 { a / b } else { 0 });
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
    pub fn and<const FLAGS: u8>(&mut self, pc: u16) -> Option<u16> {
        op_bin!(self, FLAGS, |a, b| a & b);
        Some(pc)
    }

    /// Or
    ///
    /// ```text
    /// ORA a b -- a|b
    /// ```
    /// Pushes the result of the bitwise operation `OR`, to the top of the stack.
    #[inline]
    pub fn ora<const FLAGS: u8>(&mut self, pc: u16) -> Option<u16> {
        op_bin!(self, FLAGS, |a, b| a | b);
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
    pub fn eor<const FLAGS: u8>(&mut self, pc: u16) -> Option<u16> {
        op_bin!(self, FLAGS, |a, b| a ^ b);
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
    pub fn sft<const FLAGS: u8>(&mut self, pc: u16) -> Option<u16> {
        let mut s = self.stack_view::<FLAGS>();
        let shift = s.pop_byte();
        let shr = u32::from(shift & 0xF);
        let shl = u32::from(shift >> 4);
        let v = s.pop();
        s.push(v.shr(shr).shl(shl));
        Some(pc)
    }
}

/// Trait for a Uxn-compatible device
pub trait Device {
    /// Performs the `DEI` operation for the given target
    ///
    /// This function must write its output byte to `vm.dev[target]`; the CPU
    /// evaluation loop will then copy this value to the stack.
    fn dei(&mut self, vm: &mut Uxn, target: u8);

    /// Performs the `DEO` operation on the given target
    ///
    /// The input byte will be written to `vm.dev[target]` before this function
    /// is called, and can be read by the function.
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
    use alloc::{boxed::Box, vec};

    /// Helper type for building a RAM array of the appropriate size
    ///
    /// This is only available if the `"alloc"` feature is enabled
    pub struct UxnRam(Box<[u8; 65536]>);

    impl UxnRam {
        /// Builds a new zero-initialized RAM
        pub fn new() -> Self {
            UxnRam(vec![0u8; 65536].into_boxed_slice().try_into().unwrap())
        }

        /// Leaks memory, setting it to a static lifetime
        pub fn leak(self) -> &'static mut [u8; 65536] {
            Box::leak(self.0)
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

////////////////////////////////////////////////////////////////////////////////

/// Opcode names and constants
#[allow(non_upper_case_globals, missing_docs)]
pub mod op {
    pub const BRK: u8 = 0x0;
    pub const INC: u8 = 0x1;
    pub const POP: u8 = 0x2;
    pub const NIP: u8 = 0x3;
    pub const SWP: u8 = 0x4;
    pub const ROT: u8 = 0x5;
    pub const DUP: u8 = 0x6;
    pub const OVR: u8 = 0x7;
    pub const EQU: u8 = 0x8;
    pub const NEQ: u8 = 0x9;
    pub const GTH: u8 = 0xa;
    pub const LTH: u8 = 0xb;
    pub const JMP: u8 = 0xc;
    pub const JCN: u8 = 0xd;
    pub const JSR: u8 = 0xe;
    pub const STH: u8 = 0x0f;
    pub const LDZ: u8 = 0x10;
    pub const STZ: u8 = 0x11;
    pub const LDR: u8 = 0x12;
    pub const STR: u8 = 0x13;
    pub const LDA: u8 = 0x14;
    pub const STA: u8 = 0x15;
    pub const DEI: u8 = 0x16;
    pub const DEO: u8 = 0x17;
    pub const ADD: u8 = 0x18;
    pub const SUB: u8 = 0x19;
    pub const MUL: u8 = 0x1a;
    pub const DIV: u8 = 0x1b;
    pub const AND: u8 = 0x1c;
    pub const ORA: u8 = 0x1d;
    pub const EOR: u8 = 0x1e;
    pub const SFT: u8 = 0x1f;
    pub const JCI: u8 = 0x20;
    pub const INC2: u8 = 0x21;
    pub const POP2: u8 = 0x22;
    pub const NIP2: u8 = 0x23;
    pub const SWP2: u8 = 0x24;
    pub const ROT2: u8 = 0x25;
    pub const DUP2: u8 = 0x26;
    pub const OVR2: u8 = 0x27;
    pub const EQU2: u8 = 0x28;
    pub const NEQ2: u8 = 0x29;
    pub const GTH2: u8 = 0x2a;
    pub const LTH2: u8 = 0x2b;
    pub const JMP2: u8 = 0x2c;
    pub const JCN2: u8 = 0x2d;
    pub const JSR2: u8 = 0x2e;
    pub const STH2: u8 = 0x2f;
    pub const LDZ2: u8 = 0x30;
    pub const STZ2: u8 = 0x31;
    pub const LDR2: u8 = 0x32;
    pub const STR2: u8 = 0x33;
    pub const LDA2: u8 = 0x34;
    pub const STA2: u8 = 0x35;
    pub const DEI2: u8 = 0x36;
    pub const DEO2: u8 = 0x37;
    pub const ADD2: u8 = 0x38;
    pub const SUB2: u8 = 0x39;
    pub const MUL2: u8 = 0x3a;
    pub const DIV2: u8 = 0x3b;
    pub const AND2: u8 = 0x3c;
    pub const ORA2: u8 = 0x3d;
    pub const EOR2: u8 = 0x3e;
    pub const SFT2: u8 = 0x3f;
    pub const JMI: u8 = 0x40;
    pub const INCr: u8 = 0x41;
    pub const POPr: u8 = 0x42;
    pub const NIPr: u8 = 0x43;
    pub const SWPr: u8 = 0x44;
    pub const ROTr: u8 = 0x45;
    pub const DUPr: u8 = 0x46;
    pub const OVRr: u8 = 0x47;
    pub const EQUr: u8 = 0x48;
    pub const NEQr: u8 = 0x49;
    pub const GTHr: u8 = 0x4a;
    pub const LTHr: u8 = 0x4b;
    pub const JMPr: u8 = 0x4c;
    pub const JCNr: u8 = 0x4d;
    pub const JSRr: u8 = 0x4e;
    pub const STHr: u8 = 0x4f;
    pub const LDZr: u8 = 0x50;
    pub const STZr: u8 = 0x51;
    pub const LDRr: u8 = 0x52;
    pub const STRr: u8 = 0x53;
    pub const LDAr: u8 = 0x54;
    pub const STAr: u8 = 0x55;
    pub const DEIr: u8 = 0x56;
    pub const DEOr: u8 = 0x57;
    pub const ADDr: u8 = 0x58;
    pub const SUBr: u8 = 0x59;
    pub const MULr: u8 = 0x5a;
    pub const DIVr: u8 = 0x5b;
    pub const ANDr: u8 = 0x5c;
    pub const ORAr: u8 = 0x5d;
    pub const EORr: u8 = 0x5e;
    pub const SFTr: u8 = 0x5f;
    pub const JSI: u8 = 0x60;
    pub const INC2r: u8 = 0x61;
    pub const POP2r: u8 = 0x62;
    pub const NIP2r: u8 = 0x63;
    pub const SWP2r: u8 = 0x64;
    pub const ROT2r: u8 = 0x65;
    pub const DUP2r: u8 = 0x66;
    pub const OVR2r: u8 = 0x67;
    pub const EQU2r: u8 = 0x68;
    pub const NEQ2r: u8 = 0x69;
    pub const GTH2r: u8 = 0x6a;
    pub const LTH2r: u8 = 0x6b;
    pub const JMP2r: u8 = 0x6c;
    pub const JCN2r: u8 = 0x6d;
    pub const JSR2r: u8 = 0x6e;
    pub const STH2r: u8 = 0x6f;
    pub const LDZ2r: u8 = 0x70;
    pub const STZ2r: u8 = 0x71;
    pub const LDR2r: u8 = 0x72;
    pub const STR2r: u8 = 0x73;
    pub const LDA2r: u8 = 0x74;
    pub const STA2r: u8 = 0x75;
    pub const DEI2r: u8 = 0x76;
    pub const DEO2r: u8 = 0x77;
    pub const ADD2r: u8 = 0x78;
    pub const SUB2r: u8 = 0x79;
    pub const MUL2r: u8 = 0x7a;
    pub const DIV2r: u8 = 0x7b;
    pub const AND2r: u8 = 0x7c;
    pub const ORA2r: u8 = 0x7d;
    pub const EOR2r: u8 = 0x7e;
    pub const SFT2r: u8 = 0x7f;
    pub const LIT: u8 = 0x80;
    pub const INCk: u8 = 0x81;
    pub const POPk: u8 = 0x82;
    pub const NIPk: u8 = 0x83;
    pub const SWPk: u8 = 0x84;
    pub const ROTk: u8 = 0x85;
    pub const DUPk: u8 = 0x86;
    pub const OVRk: u8 = 0x87;
    pub const EQUk: u8 = 0x88;
    pub const NEQk: u8 = 0x89;
    pub const GTHk: u8 = 0x8a;
    pub const LTHk: u8 = 0x8b;
    pub const JMPk: u8 = 0x8c;
    pub const JCNk: u8 = 0x8d;
    pub const JSRk: u8 = 0x8e;
    pub const STHk: u8 = 0x8f;
    pub const LDZk: u8 = 0x90;
    pub const STZk: u8 = 0x91;
    pub const LDRk: u8 = 0x92;
    pub const STRk: u8 = 0x93;
    pub const LDAk: u8 = 0x94;
    pub const STAk: u8 = 0x95;
    pub const DEIk: u8 = 0x96;
    pub const DEOk: u8 = 0x97;
    pub const ADDk: u8 = 0x98;
    pub const SUBk: u8 = 0x99;
    pub const MULk: u8 = 0x9a;
    pub const DIVk: u8 = 0x9b;
    pub const ANDk: u8 = 0x9c;
    pub const ORAk: u8 = 0x9d;
    pub const EORk: u8 = 0x9e;
    pub const SFTk: u8 = 0x9f;
    pub const LIT2: u8 = 0xa0;
    pub const INC2k: u8 = 0xa1;
    pub const POP2k: u8 = 0xa2;
    pub const NIP2k: u8 = 0xa3;
    pub const SWP2k: u8 = 0xa4;
    pub const ROT2k: u8 = 0xa5;
    pub const DUP2k: u8 = 0xa6;
    pub const OVR2k: u8 = 0xa7;
    pub const EQU2k: u8 = 0xa8;
    pub const NEQ2k: u8 = 0xa9;
    pub const GTH2k: u8 = 0xaa;
    pub const LTH2k: u8 = 0xab;
    pub const JMP2k: u8 = 0xac;
    pub const JCN2k: u8 = 0xad;
    pub const JSR2k: u8 = 0xae;
    pub const STH2k: u8 = 0xaf;
    pub const LDZ2k: u8 = 0xb0;
    pub const STZ2k: u8 = 0xb1;
    pub const LDR2k: u8 = 0xb2;
    pub const STR2k: u8 = 0xb3;
    pub const LDA2k: u8 = 0xb4;
    pub const STA2k: u8 = 0xb5;
    pub const DEI2k: u8 = 0xb6;
    pub const DEO2k: u8 = 0xb7;
    pub const ADD2k: u8 = 0xb8;
    pub const SUB2k: u8 = 0xb9;
    pub const MUL2k: u8 = 0xba;
    pub const DIV2k: u8 = 0xbb;
    pub const AND2k: u8 = 0xbc;
    pub const ORA2k: u8 = 0xbd;
    pub const EOR2k: u8 = 0xbe;
    pub const SFT2k: u8 = 0xbf;
    pub const LITr: u8 = 0xc0;
    pub const INCkr: u8 = 0xc1;
    pub const POPkr: u8 = 0xc2;
    pub const NIPkr: u8 = 0xc3;
    pub const SWPkr: u8 = 0xc4;
    pub const ROTkr: u8 = 0xc5;
    pub const DUPkr: u8 = 0xc6;
    pub const OVRkr: u8 = 0xc7;
    pub const EQUkr: u8 = 0xc8;
    pub const NEQkr: u8 = 0xc9;
    pub const GTHkr: u8 = 0xca;
    pub const LTHkr: u8 = 0xcb;
    pub const JMPkr: u8 = 0xcc;
    pub const JCNkr: u8 = 0xcd;
    pub const JSRkr: u8 = 0xce;
    pub const STHkr: u8 = 0xcf;
    pub const LDZkr: u8 = 0xd0;
    pub const STZkr: u8 = 0xd1;
    pub const LDRkr: u8 = 0xd2;
    pub const STRkr: u8 = 0xd3;
    pub const LDAkr: u8 = 0xd4;
    pub const STAkr: u8 = 0xd5;
    pub const DEIkr: u8 = 0xd6;
    pub const DEOkr: u8 = 0xd7;
    pub const ADDkr: u8 = 0xd8;
    pub const SUBkr: u8 = 0xd9;
    pub const MULkr: u8 = 0xda;
    pub const DIVkr: u8 = 0xdb;
    pub const ANDkr: u8 = 0xdc;
    pub const ORAkr: u8 = 0xdd;
    pub const EORkr: u8 = 0xde;
    pub const SFTkr: u8 = 0xdf;
    pub const LIT2r: u8 = 0xe0;
    pub const INC2kr: u8 = 0xe1;
    pub const POP2kr: u8 = 0xe2;
    pub const NIP2kr: u8 = 0xe3;
    pub const SWP2kr: u8 = 0xe4;
    pub const ROT2kr: u8 = 0xe5;
    pub const DUP2kr: u8 = 0xe6;
    pub const OVR2kr: u8 = 0xe7;
    pub const EQU2kr: u8 = 0xe8;
    pub const NEQ2kr: u8 = 0xe9;
    pub const GTH2kr: u8 = 0xea;
    pub const LTH2kr: u8 = 0xeb;
    pub const JMP2kr: u8 = 0xec;
    pub const JCN2kr: u8 = 0xed;
    pub const JSR2kr: u8 = 0xee;
    pub const STH2kr: u8 = 0xef;
    pub const LDZ2kr: u8 = 0xf0;
    pub const STZ2kr: u8 = 0xf1;
    pub const LDR2kr: u8 = 0xf2;
    pub const STR2kr: u8 = 0xf3;
    pub const LDA2kr: u8 = 0xf4;
    pub const STA2kr: u8 = 0xf5;
    pub const DEI2kr: u8 = 0xf6;
    pub const DEO2kr: u8 = 0xf7;
    pub const ADD2kr: u8 = 0xf8;
    pub const SUB2kr: u8 = 0xf9;
    pub const MUL2kr: u8 = 0xfa;
    pub const DIV2kr: u8 = 0xfb;
    pub const AND2kr: u8 = 0xfc;
    pub const ORA2kr: u8 = 0xfd;
    pub const EOR2kr: u8 = 0xfe;
    pub const SFT2kr: u8 = 0xff;

    pub const NAMES: [&str; 256] = [
        "BRK", "INC", "POP", "NIP", "SWP", "ROT", "DUP", "OVR", "EQU", "NEQ",
        "GTH", "LTH", "JMP", "JCN", "JSR", "STH", "LDZ", "STZ", "LDR", "STR",
        "LDA", "STA", "DEI", "DEO", "ADD", "SUB", "MUL", "DIV", "AND", "ORA",
        "EOR", "SFT", "JCI", "INC2", "POP2", "NIP2", "SWP2", "ROT2", "DUP2",
        "OVR2", "EQU2", "NEQ2", "GTH2", "LTH2", "JMP2", "JCN2", "JSR2", "STH2",
        "LDZ2", "STZ2", "LDR2", "STR2", "LDA2", "STA2", "DEI2", "DEO2", "ADD2",
        "SUB2", "MUL2", "DIV2", "AND2", "ORA2", "EOR2", "SFT2", "JMI", "INCr",
        "POPr", "NIPr", "SWPr", "ROTr", "DUPr", "OVRr", "EQUr", "NEQr", "GTHr",
        "LTHr", "JMPr", "JCNr", "JSRr", "STHr", "LDZr", "STZr", "LDRr", "STRr",
        "LDAr", "STAr", "DEIr", "DEOr", "ADDr", "SUBr", "MULr", "DIVr", "ANDr",
        "ORAr", "EORr", "SFTr", "JSI", "INC2r", "POP2r", "NIP2r", "SWP2r",
        "ROT2r", "DUP2r", "OVR2r", "EQU2r", "NEQ2r", "GTH2r", "LTH2r", "JMP2r",
        "JCN2r", "JSR2r", "STH2r", "LDZ2r", "STZ2r", "LDR2r", "STR2r", "LDA2r",
        "STA2r", "DEI2r", "DEO2r", "ADD2r", "SUB2r", "MUL2r", "DIV2r", "AND2r",
        "ORA2r", "EOR2r", "SFT2r", "LIT", "INCk", "POPk", "NIPk", "SWPk",
        "ROTk", "DUPk", "OVRk", "EQUk", "NEQk", "GTHk", "LTHk", "JMPk", "JCNk",
        "JSRk", "STHk", "LDZk", "STZk", "LDRk", "STRk", "LDAk", "STAk", "DEIk",
        "DEOk", "ADDk", "SUBk", "MULk", "DIVk", "ANDk", "ORAk", "EORk", "SFTk",
        "LIT2", "INC2k", "POP2k", "NIP2k", "SWP2k", "ROT2k", "DUP2k", "OVR2k",
        "EQU2k", "NEQ2k", "GTH2k", "LTH2k", "JMP2k", "JCN2k", "JSR2k", "STH2k",
        "LDZ2k", "STZ2k", "LDR2k", "STR2k", "LDA2k", "STA2k", "DEI2k", "DEO2k",
        "ADD2k", "SUB2k", "MUL2k", "DIV2k", "AND2k", "ORA2k", "EOR2k", "SFT2k",
        "LITr", "INCkr", "POPkr", "NIPkr", "SWPkr", "ROTkr", "DUPkr", "OVRkr",
        "EQUkr", "NEQkr", "GTHkr", "LTHkr", "JMPkr", "JCNkr", "JSRkr", "STHkr",
        "LDZkr", "STZkr", "LDRkr", "STRkr", "LDAkr", "STAkr", "DEIkr", "DEOkr",
        "ADDkr", "SUBkr", "MULkr", "DIVkr", "ANDkr", "ORAkr", "EORkr", "SFTkr",
        "LIT2r", "INC2kr", "POP2kr", "NIP2kr", "SWP2kr", "ROT2kr", "DUP2kr",
        "OVR2kr", "EQU2kr", "NEQ2kr", "GTH2kr", "LTH2kr", "JMP2kr", "JCN2kr",
        "JSR2kr", "STH2kr", "LDZ2kr", "STZ2kr", "LDR2kr", "STR2kr", "LDA2kr",
        "STA2kr", "DEI2kr", "DEO2kr", "ADD2kr", "SUB2kr", "MUL2kr", "DIV2kr",
        "AND2kr", "ORA2kr", "EOR2kr", "SFT2kr",
    ];
}

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
        let mode = (u8::from(keep) << 7)
            | (u8::from(ret) << 6)
            | (u8::from(short) << 5);
        let out = match s {
            "BRK" => op::BRK,
            "JCI" => op::JCI,
            "JMI" => op::JMI,
            "JSI" => op::JSI,
            "LIT" => op::LIT | mode,

            "INC" => op::INC | mode,
            "POP" => op::POP | mode,
            "NIP" => op::NIP | mode,
            "SWP" => op::SWP | mode,
            "ROT" => op::ROT | mode,
            "DUP" => op::DUP | mode,
            "OVR" => op::OVR | mode,
            "EQU" => op::EQU | mode,
            "NEQ" => op::NEQ | mode,
            "GTH" => op::GTH | mode,
            "LTH" => op::LTH | mode,
            "JMP" => op::JMP | mode,
            "JCN" => op::JCN | mode,
            "JSR" => op::JSR | mode,
            "STH" => op::STH | mode,
            "LDZ" => op::LDZ | mode,
            "STZ" => op::STZ | mode,
            "LDR" => op::LDR | mode,
            "STR" => op::STR | mode,
            "LDA" => op::LDA | mode,
            "STA" => op::STA | mode,
            "DEI" => op::DEI | mode,
            "DEO" => op::DEO | mode,
            "ADD" => op::ADD | mode,
            "SUB" => op::SUB | mode,
            "MUL" => op::MUL | mode,
            "DIV" => op::DIV | mode,
            "AND" => op::AND | mode,
            "ORA" => op::ORA | mode,
            "EOR" => op::EOR | mode,
            "SFT" => op::SFT | mode,
            _ => return Err(s),
        };
        Ok(out)
    }

    fn parse_and_test(s: &str) {
        let mut ram = UxnRam::new();
        let mut vm = Uxn::new(&mut ram, Backend::Interpreter);
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
                vm.ram[0] = op.unwrap();
                vm.run(&mut dev, 0);
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
            #1234 DUP2k  ( 12 34 12 34 12 34 )
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
            #0120 #0010 DIV2  ( 00 12 )
            #0120 #0010 DIV2k ( 01 20 00 10 00 12 )
            #34 #10 SFT        ( 68 )
            #34 #01 SFT        ( 1a )
            #34 #33 SFTk       ( 34 33 30 )
            #1248 #34 SFT2k    ( 12 48 34 09 20 )
            #1248 #34 SFT2     ( 09 20 )
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
