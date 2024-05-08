//! Uxn virtual machine

/// Opcode mode flags
#[derive(Copy, Clone, Debug)]
struct Mode {
    /// `2` mode
    ///
    /// Operate on shorts (`u16`), instead of bytes
    short: bool,

    /// `k` mode
    ///
    /// Operate without consuming items
    keep: bool,

    /// `r` mode
    ///
    /// Operate on the return stack
    ret: bool,
}

impl From<LitMode> for Mode {
    fn from(mode: LitMode) -> Mode {
        Mode {
            short: mode.short,
            keep: false,
            ret: mode.ret,
        }
    }
}

/// Opcode mode flags for literal opcodes (where `keep` is always true)
#[derive(Copy, Clone, Debug)]
struct LitMode {
    /// `2` mode
    ///
    /// Operate on shorts (`u16`), instead of bytes
    short: bool,

    /// `r` mode
    ///
    /// Operate on the return stack
    ret: bool,
}

/// Uxn opcode
#[derive(Copy, Clone, Debug)]
enum Op {
    /// Break
    ///
    /// ```text
    /// BRK --
    /// ```
    ///
    /// Ends the evaluation of the current vector. This opcode has no modes.
    Brk,

    /// Jump Conditional Instant
    ///
    /// ```text
    /// JCI cond8 --
    /// ```
    ///
    /// Pops a byte from the working stack and if it is not zero, moves
    /// the `PC` to a relative address at a distance equal to the next short in
    /// memory, otherwise moves `PC+2`. This opcode has no modes.
    Jci,

    /// Jump Instant
    ///
    /// JMI  -- Moves the PC to a relative address at a distance equal to the next
    /// short in memory. This opcode has no modes.
    Jmi,

    /// Jump Stash Return Instant
    ///
    /// ```text
    /// JSI  --
    /// ```
    ///
    /// Pushes `PC+2` to the return-stack and moves the `PC` to a relative
    /// address at a distance equal to the next short in memory. This opcode has
    /// no modes.
    Jsi,

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
    Lit(LitMode),

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
    Inc(Mode),

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
    Pop(Mode),

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
    Nip(Mode),

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
    Swp(Mode),

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
    Rot(Mode),

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
    Dup(Mode),

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
    Ovr(Mode),

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
    Equ(Mode),

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
    Neq(Mode),

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
    Gth(Mode),

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
    Lth(Mode),

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
    Jmp(Mode),

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
    Jcn(Mode),

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
    ///
    Jsr(Mode),

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
    Sth(Mode),

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
    Ldz(Mode),

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
    Stz(Mode),

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
    Ldr(Mode),

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
    Str(Mode),

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
    Lda(Mode),

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
    Sta(Mode),

    /// Device Input
    ///
    /// ```text
    /// DEI device8 -- value
    /// ```
    ///
    /// Pushes a value from the device page, to the top of the stack. The target
    /// device might capture the reading to trigger an I/O event.
    Dei(Mode),

    /// Device Output
    ///
    /// ```text
    /// DEO val device8 --
    /// ```
    ///
    /// Writes a value to the device page. The target device might capture the
    /// writing to trigger an I/O event.
    Deo(Mode),

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
    Add(Mode),

    /// Subtract
    ///
    /// ```text
    /// SUB a b -- a-b
    /// ```
    ///
    /// Pushes the difference of the first value minus the second, to the top of
    /// the stack.
    Sub(Mode),

    /// Multiply
    ///
    /// ```text
    /// MUL a b -- a*b
    /// ```
    ///
    /// Pushes the product of the first and second values at the top of the
    /// stack.
    Mul(Mode),

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
    Div(Mode),

    /// And
    ///
    /// ```text
    /// AND a b -- a&b
    /// ```
    ///
    /// Pushes the result of the bitwise operation `AND`, to the top of the
    /// stack.
    And(Mode),

    /// Or
    ///
    /// ```text
    /// ORA a b -- a|b
    /// ```
    /// Pushes the result of the bitwise operation `OR`, to the top of the stack.
    Ora(Mode),

    /// Exclusive Or
    ///
    /// ```text
    /// EOR a b -- a^b
    /// ```
    ///
    /// Pushes the result of the bitwise operation `XOR`, to the top of the
    /// stack.
    Eor(Mode),

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
    Sft(Mode),
}

/// Every `u8` is a valid opcode
impl From<u8> for Op {
    fn from(i: u8) -> Op {
        let short = (i & (1 << 5)) != 0;
        let ret = (i & (1 << 6)) != 0;
        let keep = (i & (1 << 7)) != 0;
        let mode = Mode { short, keep, ret };
        match i & 0b11111 {
            0x00 => match i {
                0x00 => Op::Brk,
                0x20 => Op::Jci,
                0x40 => Op::Jmi,
                0x60 => Op::Jsi,
                _ => Op::Lit(LitMode { short, ret }),
            },
            0x01 => Op::Inc(mode),
            0x02 => Op::Pop(mode),
            0x03 => Op::Nip(mode),
            0x04 => Op::Swp(mode),
            0x05 => Op::Rot(mode),
            0x06 => Op::Dup(mode),
            0x07 => Op::Ovr(mode),
            0x08 => Op::Equ(mode),
            0x09 => Op::Neq(mode),
            0x0a => Op::Gth(mode),
            0x0b => Op::Lth(mode),
            0x0c => Op::Jmp(mode),
            0x0d => Op::Jcn(mode),
            0x0e => Op::Jsr(mode),
            0x0f => Op::Sth(mode),
            0x10 => Op::Ldz(mode),
            0x11 => Op::Stz(mode),
            0x12 => Op::Ldr(mode),
            0x13 => Op::Str(mode),
            0x14 => Op::Lda(mode),
            0x15 => Op::Sta(mode),
            0x16 => Op::Dei(mode),
            0x17 => Op::Deo(mode),
            0x18 => Op::Add(mode),
            0x19 => Op::Sub(mode),
            0x1a => Op::Mul(mode),
            0x1b => Op::Div(mode),
            0x1c => Op::And(mode),
            0x1d => Op::Ora(mode),
            0x1e => Op::Eor(mode),
            0x1f => Op::Sft(mode),
            _ => unreachable!(),
        }
    }
}

/// Simple circular stack, with room for 256 items
#[derive(Debug)]
struct Stack {
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
struct StackView<'a> {
    stack: &'a mut Stack,
    keep: bool,
    short: bool,

    /// Virtual index, used in `keep` mode
    offset: u8,
}

impl<'a> StackView<'a> {
    fn new(stack: &'a mut Stack, mode: Mode) -> Self {
        Self {
            stack,
            keep: mode.keep,
            short: mode.short,
            offset: 0,
        }
    }

    /// Pops a single value from the stack
    ///
    /// Returns a [`Value::Short`] if `self.short` is set, and a [`Value::Byte`]
    /// otherwise.
    ///
    /// If `self.keep` is set, then only the view offset ([`StackView::offset`])
    /// is changed; otherwise, the stack index ([`Stack::index`]) is changed.
    fn pop(&mut self) -> Value {
        self.pop_type(self.short)
    }

    fn pop_type(&mut self, short: bool) -> Value {
        if self.keep {
            let v = self.stack.peek_at(self.offset, short);
            self.offset = self.offset.wrapping_add(if short { 2 } else { 1 });
            v
        } else {
            self.stack.pop(short)
        }
    }

    fn pop_byte(&mut self) -> u8 {
        let Value::Byte(out) = self.pop_type(false) else {
            unreachable!();
        };
        out
    }

    fn pop_short(&mut self) -> u16 {
        let Value::Short(out) = self.pop_type(true) else {
            unreachable!();
        };
        out
    }

    fn push(&mut self, v: Value) {
        self.stack.push(v);
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
    fn wrapping_add(&self, i: u8) -> Self {
        match self {
            Value::Short(v) => Value::Short(v.wrapping_add(u16::from(i))),
            Value::Byte(v) => Value::Byte(v.wrapping_add(i)),
        }
    }
    fn wrapping_shr(&self, i: u32) -> Self {
        match self {
            Value::Short(v) => Value::Short(v.wrapping_shr(i)),
            Value::Byte(v) => Value::Byte(v.wrapping_shr(i)),
        }
    }
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
    fn pop_byte(&mut self) -> u8 {
        let out = self.data[usize::from(self.index)];
        self.index = self.index.wrapping_sub(1);
        out
    }
    fn pop_short(&mut self) -> u16 {
        let lo = self.pop_byte();
        let hi = self.pop_byte();
        u16::from_be_bytes([hi, lo])
    }
    fn push_byte(&mut self, v: u8) {
        self.index = self.index.wrapping_add(1);
        self.data[usize::from(self.index)] = v;
    }
    fn push_short(&mut self, v: u16) {
        let [hi, lo] = v.to_be_bytes();
        self.push_byte(hi);
        self.push_byte(lo);
    }
    fn push(&mut self, v: Value) {
        match v {
            Value::Short(v) => self.push_short(v),
            Value::Byte(v) => self.push_byte(v),
        }
    }
    fn pop(&mut self, short: bool) -> Value {
        if short {
            Value::Short(self.pop_short())
        } else {
            Value::Byte(self.pop_byte())
        }
    }
    fn peek_byte_at(&self, offset: u8) -> u8 {
        self.data[usize::from(self.index.wrapping_sub(offset))]
    }
    fn peek_short_at(&self, offset: u8) -> u16 {
        let lo = self.peek_byte_at(offset);
        let hi = self.peek_byte_at(offset.wrapping_add(1));
        u16::from_be_bytes([hi, lo])
    }
    fn peek_at(&self, offset: u8, short: bool) -> Value {
        if short {
            Value::Short(self.peek_short_at(offset))
        } else {
            Value::Byte(self.peek_byte_at(offset))
        }
    }
}

/// The virtual machine itself
pub struct Vm {
    ram: Box<[u8]>,
    stack: Stack,
    ret: Stack,
    pc: u16,
}

impl Default for Vm {
    fn default() -> Self {
        Self {
            ram: vec![0u8; usize::from(u16::MAX)].into_boxed_slice(),
            stack: Stack::default(),
            ret: Stack::default(),
            pc: 0x0,
        }
    }
}

macro_rules! op_cmp {
    ($self:ident, $mode:ident, $f:expr) => {{
        let mut s = $self.stack_view($mode);
        #[allow(clippy::redundant_closure_call)]
        let v = if $mode.short {
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
    ($self:ident, $mode:ident, $f:expr) => {{
        let mut s = $self.stack_view($mode);
        #[allow(clippy::redundant_closure_call)]
        if $mode.short {
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

impl Vm {
    /// Build a new `Vm`, loading the given ROM at the start address
    ///
    /// # Panics
    /// If `rom` cannot fit in memory
    pub fn new(rom: &[u8]) -> Self {
        let mut out = Self {
            ram: vec![0u8; usize::from(u16::MAX)].into_boxed_slice(),
            stack: Stack::default(),
            ret: Stack::default(),
            pc: 0x100,
        };
        out.ram[0x100..][..rom.len()].copy_from_slice(rom);
        out
    }

    /// Reads a byte from RAM at the program counter
    fn next(&mut self) -> u8 {
        let out = self.ram[usize::from(self.pc)];
        self.pc = self.pc.wrapping_add(1);
        out
    }

    /// Reads a word from RAM at the program counter
    fn next2(&mut self) -> u16 {
        let lo = self.next();
        let hi = self.next();
        u16::from_le_bytes([lo, hi])
    }

    /// Executes the opcode at the program counter
    pub fn step<D: Device>(&mut self, dev: &mut D) -> bool {
        let i = self.next();
        let op = Op::from(i);
        self.run_op(op, dev)
    }

    fn run_op<D: Device>(&mut self, op: Op, dev: &mut D) -> bool {
        match op {
            Op::Brk => return true,
            Op::Jci => {
                let dt = self.next2();
                if self.stack.pop_byte() != 0 {
                    self.pc = self.pc.wrapping_add(dt);
                }
            }
            Op::Jmi => {
                let dt = self.next2();
                self.pc = self.pc.wrapping_add(dt);
            }
            Op::Jsi => {
                let dt = self.next2();
                self.ret.push(Value::Short(self.pc));
                self.pc = self.pc.wrapping_add(dt);
            }
            Op::Lit(mode) => {
                let v = if mode.short {
                    Value::Short(self.next2())
                } else {
                    Value::Byte(self.next())
                };
                self.stack_view(Mode::from(mode)).push(v);
            }
            Op::Inc(mode) => {
                let mut s = self.stack_view(mode);
                let v = s.pop();
                s.push(v.wrapping_add(1));
            }
            Op::Pop(mode) => {
                self.stack_view(mode).pop();
            }
            Op::Nip(mode) => {
                let mut s = self.stack_view(mode);
                let v = s.pop();
                let _ = s.pop();
                s.push(v);
            }
            Op::Swp(mode) => {
                let mut s = self.stack_view(mode);
                let b = s.pop();
                let a = s.pop();
                s.push(b);
                s.push(a);
            }
            Op::Rot(mode) => {
                let mut s = self.stack_view(mode);
                let c = s.pop();
                let b = s.pop();
                let a = s.pop();
                s.push(b);
                s.push(c);
                s.push(a);
            }
            Op::Dup(mode) => {
                let mut s = self.stack_view(mode);
                let v = s.pop();
                s.push(v);
                s.push(v);
            }
            Op::Ovr(mode) => {
                let mut s = self.stack_view(mode);
                let b = s.pop();
                let a = s.pop();
                s.push(a);
                s.push(b);
                s.push(a);
            }
            Op::Equ(mode) => op_cmp!(self, mode, |a, b| a == b),
            Op::Neq(mode) => op_cmp!(self, mode, |a, b| a != b),
            Op::Gth(mode) => op_cmp!(self, mode, |a, b| a > b),
            Op::Lth(mode) => op_cmp!(self, mode, |a, b| a < b),
            Op::Jmp(mode) => {
                let mut s = self.stack_view(mode);
                self.pc = match s.pop() {
                    Value::Short(v) => v,
                    Value::Byte(v) => self.pc.wrapping_add(u16::from(v)),
                }
            }
            Op::Jcn(mode) => {
                let mut s = self.stack_view(mode);
                let dst = s.pop();
                let cond = s.pop_byte();
                if cond != 0 {
                    self.pc = match dst {
                        Value::Short(dst) => dst,
                        Value::Byte(offset) => {
                            self.pc.wrapping_add(u16::from(offset))
                        }
                    };
                }
            }
            Op::Jsr(mode) => {
                self.ret.push(Value::Short(self.pc));
                let mut s = self.stack_view(mode);
                self.pc = match s.pop() {
                    Value::Short(v) => v,
                    Value::Byte(v) => self.pc.wrapping_add(u16::from(v)),
                }
            }
            Op::Sth(mode) => {
                let v = self
                    .stack_view(Mode {
                        ret: !mode.ret,
                        ..mode
                    })
                    .pop();
                self.stack_view(mode).push(v)
            }
            Op::Ldz(mode) => {
                let addr = self.stack_view(mode).pop_byte();
                let v = if mode.short {
                    let hi = self.ram[usize::from(addr)];
                    let lo = self.ram[usize::from(addr.wrapping_add(1))];
                    Value::Short(u16::from_be_bytes([hi, lo]))
                } else {
                    let v = self.ram[usize::from(addr)];
                    Value::Byte(v)
                };
                self.stack_view(mode).push(v)
            }
            Op::Stz(mode) => {
                let mut s = self.stack_view(mode);
                let addr = s.pop_byte();
                match s.pop() {
                    Value::Short(v) => {
                        let [hi, lo] = v.to_be_bytes();
                        self.ram[usize::from(addr)] = hi;
                        self.ram[usize::from(addr.wrapping_add(1))] = lo;
                    }
                    Value::Byte(v) => {
                        self.ram[usize::from(addr)] = v;
                    }
                }
            }
            Op::Ldr(mode) => {
                let offset = self.stack_view(mode).pop_byte() as i8;

                // TODO: make this more obviously infallible
                let addr = if offset < 0 {
                    self.pc.wrapping_sub(
                        i16::from(offset).abs().try_into().unwrap(),
                    )
                } else {
                    self.pc.wrapping_add(u16::try_from(offset).unwrap())
                };

                let v = if mode.short {
                    let hi = self.ram[usize::from(addr)];
                    let lo = self.ram[usize::from(addr.wrapping_add(1))];
                    Value::Short(u16::from_be_bytes([hi, lo]))
                } else {
                    let v = self.ram[usize::from(addr)];
                    Value::Byte(v)
                };
                self.stack_view(mode).push(v);
            }
            Op::Str(mode) => {
                let pc = self.pc;
                let mut s = self.stack_view(mode);
                let offset = s.pop_byte() as i8;
                let addr = if offset < 0 {
                    pc.wrapping_sub(i16::from(offset).abs().try_into().unwrap())
                } else {
                    pc.wrapping_add(u16::try_from(offset).unwrap())
                };
                match s.pop() {
                    Value::Short(v) => {
                        let [hi, lo] = v.to_be_bytes();
                        self.ram[usize::from(addr)] = hi;
                        self.ram[usize::from(addr.wrapping_add(1))] = lo;
                    }
                    Value::Byte(v) => {
                        self.ram[usize::from(addr)] = v;
                    }
                }
            }
            Op::Lda(mode) => {
                let addr = self.stack_view(mode).pop_short();
                let v = if mode.short {
                    let hi = self.ram[usize::from(addr)];
                    let lo = self.ram[usize::from(addr.wrapping_add(1))];
                    Value::Short(u16::from_be_bytes([hi, lo]))
                } else {
                    let v = self.ram[usize::from(addr)];
                    Value::Byte(v)
                };
                self.stack_view(mode).push(v);
            }
            Op::Sta(mode) => {
                let mut s = self.stack_view(mode);
                let addr = s.pop_short();
                match s.pop() {
                    Value::Short(v) => {
                        let [hi, lo] = v.to_be_bytes();
                        self.ram[usize::from(addr)] = hi;
                        self.ram[usize::from(addr.wrapping_add(1))] = lo;
                    }
                    Value::Byte(v) => {
                        self.ram[usize::from(addr)] = v;
                    }
                }
            }
            Op::Dei(mode) => {
                let i = self.stack_view(mode).pop_byte();
                let v = if mode.short {
                    // ORDER??
                    let lo = dev.dei(i);
                    let hi = dev.dei(i.wrapping_add(1));
                    Value::Short(u16::from_be_bytes([hi, lo]))
                } else {
                    let lo = dev.dei(i);
                    Value::Byte(lo)
                };
                self.stack_view(mode).push(v);
            }
            Op::Deo(mode) => {
                let mut s = self.stack_view(mode);
                let i = s.pop_byte();
                match s.pop() {
                    Value::Short(v) => {
                        let [hi, lo] = v.to_be_bytes();
                        // ORDER??
                        dev.deo(i, lo);
                        dev.deo(i.wrapping_add(1), hi);
                    }
                    Value::Byte(v) => {
                        dev.deo(i, v);
                    }
                }
            }
            Op::Add(mode) => {
                op_bin!(self, mode, |a, b| a.wrapping_add(b));
            }
            Op::Sub(mode) => {
                op_bin!(self, mode, |a, b| a.wrapping_sub(b));
            }
            Op::Mul(mode) => {
                op_bin!(self, mode, |a, b| a.wrapping_mul(b));
            }
            Op::Div(mode) => {
                op_bin!(self, mode, |a, b| if b != 0 { a / b } else { 0 });
            }
            Op::And(mode) => {
                op_bin!(self, mode, |a, b| a & b);
            }
            Op::Ora(mode) => {
                op_bin!(self, mode, |a, b| a | b);
            }
            Op::Eor(mode) => {
                op_bin!(self, mode, |a, b| a ^ b);
            }
            Op::Sft(mode) => {
                let mut s = self.stack_view(mode);
                let shift = s.pop_byte();
                let shr = u32::from(shift & 0xF);
                let shl = u32::from(shift >> 4);
                let v = s.pop();
                s.push(v.wrapping_shr(shr).wrapping_shl(shl));
            }
        }

        false
    }

    fn stack_view(&mut self, mode: Mode) -> StackView {
        let stack = if mode.ret {
            &mut self.ret
        } else {
            &mut self.stack
        };
        StackView::new(stack, mode)
    }
}

/// Trait for a Uxn-compatible device
pub trait Device {
    /// Performs the `DEI` operation, reading a byte from the device
    fn dei(&mut self, target: u8) -> u8;
    /// Performs the `DEO` operation, writing a byte to the device
    fn deo(&mut self, target: u8, value: u8);
}

#[cfg(test)]
mod test {
    use super::*;

    /// Simple parser for textual opcodes
    impl<'a> TryFrom<&'a str> for Op {
        type Error = &'a str;
        fn try_from(s: &str) -> Result<Op, &str> {
            let (s, ret) =
                s.strip_suffix('r').map(|s| (s, true)).unwrap_or((s, false));
            let (s, keep) =
                s.strip_suffix('k').map(|s| (s, true)).unwrap_or((s, false));
            let (s, short) =
                s.strip_suffix('2').map(|s| (s, true)).unwrap_or((s, false));
            let mode = Mode { ret, keep, short };
            let out = match s {
                "BRK" => Op::Brk,
                "JCI" => Op::Jci,
                "JMI" => Op::Jmi,
                "JSI" => Op::Jsi,
                "LIT" => Op::Lit(LitMode { ret, short }),

                "INC" => Op::Inc(mode),
                "POP" => Op::Pop(mode),
                "NIP" => Op::Nip(mode),
                "SWP" => Op::Swp(mode),
                "ROT" => Op::Rot(mode),
                "DUP" => Op::Dup(mode),
                "OVR" => Op::Ovr(mode),
                "EQU" => Op::Equ(mode),
                "NEQ" => Op::Neq(mode),
                "GTH" => Op::Gth(mode),
                "LTH" => Op::Lth(mode),
                "JMP" => Op::Jmp(mode),
                "JCN" => Op::Jcn(mode),
                "JSR" => Op::Jsr(mode),
                "STH" => Op::Sth(mode),
                "LDZ" => Op::Ldz(mode),
                "STZ" => Op::Stz(mode),
                "LDR" => Op::Ldr(mode),
                "STR" => Op::Str(mode),
                "LDA" => Op::Lda(mode),
                "STA" => Op::Sta(mode),
                "DEI" => Op::Dei(mode),
                "DEO" => Op::Deo(mode),
                "ADD" => Op::Add(mode),
                "SUB" => Op::Sub(mode),
                "MUL" => Op::Mul(mode),
                "DIV" => Op::Div(mode),
                "AND" => Op::And(mode),
                "ORA" => Op::Ora(mode),
                "EOR" => Op::Eor(mode),
                "SFT" => Op::Sft(mode),
                _ => return Err(s),
            };
            Ok(out)
        }
    }

    struct EmptyDevice;
    impl Device for EmptyDevice {
        fn dei(&mut self, _target: u8) -> u8 {
            0
        }
        fn deo(&mut self, _target: u8, _value: u8) {
            // nothing to do here
        }
    }

    fn parse_and_test(s: &str) {
        println!("\n{s}");
        let mut vm = Vm::default();
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
                vm.run_op(op.unwrap(), &mut dev);
                let mut actual = vec![];
                while vm.stack.index != u8::MAX {
                    actual.push(vm.stack.pop_byte());
                }
                actual.reverse();
                if actual != expected {
                    panic!(
                        "failed to execute {:?}: got {actual:2x?}, expected {expected:2x?}",
                        s.trim()
                    );
                }
                break;
            } else {
                let o = Op::try_from(i).unwrap();
                op = Some(o);
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
