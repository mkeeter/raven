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

impl Mode {
    fn offset(&self, v: u8) -> u8 {
        if self.short {
            v.wrapping_mul(2)
        } else {
            v
        }
    }
}

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

struct Stack {
    data: [u8; 256],

    /// The index points to the last occupied slot, and increases on `push`
    ///
    /// If the buffer is empty or full, it points to `u8::MAX`.
    index: u8,
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

impl Stack {
    /// Drops `n` bytes from the stack
    fn drop(&mut self, n: u8) {
        self.index = self.index.wrapping_sub(n);
    }
    fn pop_byte(&mut self) -> u8 {
        let out = self.data[usize::from(self.index)];
        self.index = self.index.wrapping_sub(1);
        out
    }
    fn peek_short(&mut self) -> u16 {
        self.peek_short_at(0)
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
    fn peek(&self, short: bool) -> Value {
        self.peek_at(0, short)
    }
    fn peek_byte(&self) -> u8 {
        self.peek_byte_at(0)
    }
    fn peekpop_byte(&mut self, keep: bool) -> u8 {
        if keep {
            self.peek_byte()
        } else {
            self.pop_byte()
        }
    }
    fn peekpop_byte_at(&mut self, offset: u8, keep: bool) -> u8 {
        if keep {
            self.peek_byte_at(offset)
        } else {
            self.pop_byte()
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
    fn peekpop(&mut self, mode: Mode) -> Value {
        if mode.keep {
            self.peek(mode.short)
        } else {
            self.pop(mode.short)
        }
    }
}

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

impl Vm {
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
                if mode.short {
                    let v = self.next2();
                    self.stack_mut(mode.ret).push(Value::Short(v));
                } else {
                    let v = self.next();
                    self.stack_mut(mode.ret).push(Value::Byte(v));
                }
            }
            Op::Inc(mode) => {
                let s = self.stack_mut(mode.ret);
                let v = s.peekpop(mode);
                s.push(v.wrapping_add(1));
            }
            Op::Pop(mode) => {
                let s = self.stack_mut(mode.ret);
                s.peekpop(mode);
            }
            Op::Nip(mode) => {
                let s = self.stack_mut(mode.ret);
                let v = s.peek(mode.short);
                if !mode.keep {
                    s.drop(mode.offset(2));
                }
                s.push(v);
            }
            Op::Swp(mode) => {
                let s = self.stack_mut(mode.ret);
                let a = s.peek_at(0, mode.short);
                let b = s.peek_at(mode.offset(1), mode.short);
                if !mode.keep {
                    s.drop(mode.offset(2));
                }
                s.push(a);
                s.push(b);
            }
            Op::Rot(mode) => {
                let s = self.stack_mut(mode.ret);
                let c = s.peek_at(0, mode.short);
                let b = s.peek_at(mode.offset(1), mode.short);
                let a = s.peek_at(mode.offset(2), mode.short);
                if !mode.keep {
                    s.drop(mode.offset(3));
                }
                s.push(b);
                s.push(c);
                s.push(a);
            }
            Op::Dup(mode) => {
                let s = self.stack_mut(mode.ret);
                let v = s.peek(mode.short);
                s.push(v);
                if mode.keep {
                    s.push(v);
                }
            }
            Op::Ovr(mode) => {
                let s = self.stack_mut(mode.ret);
                let b = s.peek_at(0, mode.short);
                let a = s.peek_at(mode.offset(1), mode.short);
                s.push(a);
                if mode.keep {
                    s.push(b);
                    s.push(a);
                }
            }
            Op::Equ(mode) => self.op_cmp(mode, |a, b| a == b),
            Op::Neq(mode) => self.op_cmp(mode, |a, b| a != b),
            Op::Gth(mode) => self.op_cmp(mode, |a, b| b > a),
            Op::Lth(mode) => self.op_cmp(mode, |a, b| b < a),
            Op::Jmp(mode) => {
                let s = self.stack_mut(mode.ret);
                self.pc = match s.peekpop(mode) {
                    Value::Short(v) => v,
                    Value::Byte(v) => self.pc.wrapping_add(u16::from(v)),
                }
            }
            Op::Jcn(mode) => {
                let s = self.stack_mut(mode.ret);
                let dst = s.peekpop(mode);
                let cond = if mode.keep {
                    s.peek_byte_at(mode.offset(1))
                } else {
                    s.pop_byte()
                };
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
                let s = self.stack_mut(mode.ret);
                self.pc = match s.peekpop(mode) {
                    Value::Short(v) => v,
                    Value::Byte(v) => self.pc.wrapping_add(u16::from(v)),
                }
            }
            Op::Sth(mode) => {
                let src = self.stack_mut(!mode.ret);
                let v = src.peekpop(mode);
                let dst = self.stack_mut(mode.ret);
                dst.push(v);
            }
            Op::Ldz(mode) => {
                let s = self.stack_mut(mode.ret);
                let addr = s.peekpop_byte(mode.keep);
                let v = if mode.short {
                    let hi = self.ram[usize::from(addr)];
                    let lo = self.ram[usize::from(addr.wrapping_add(1))];
                    Value::Short(u16::from_be_bytes([hi, lo]))
                } else {
                    let v = self.ram[usize::from(addr)];
                    Value::Byte(v)
                };
                let s = self.stack_mut(mode.ret);
                s.push(v);
            }
            Op::Stz(mode) => {
                let s = self.stack_mut(mode.ret);
                let addr = s.peekpop_byte(mode.keep);
                if mode.short {
                    let v = if mode.keep {
                        s.peek_short_at(1)
                    } else {
                        s.pop_short()
                    };
                    let [hi, lo] = v.to_be_bytes();
                    self.ram[usize::from(addr)] = hi;
                    self.ram[usize::from(addr.wrapping_add(1))] = lo;
                } else {
                    let v = if mode.keep {
                        s.peek_byte_at(1)
                    } else {
                        s.pop_byte()
                    };
                    self.ram[usize::from(addr)] = v;
                }
            }
            Op::Ldr(mode) => {
                let s = self.stack_mut(mode.ret);
                let offset = s.peekpop_byte(mode.keep) as i8;

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
                let s = self.stack_mut(mode.ret);
                s.push(v);
            }
            Op::Str(mode) => {
                let pc = self.pc;
                let s = self.stack_mut(mode.ret);
                let offset = s.peekpop_byte(mode.keep) as i8;
                let addr = if offset < 0 {
                    pc.wrapping_sub(i16::from(offset).abs().try_into().unwrap())
                } else {
                    pc.wrapping_add(u16::try_from(offset).unwrap())
                };

                let v = if mode.keep {
                    s.peek_at(1, mode.short)
                } else {
                    s.pop(mode.short)
                };
                match v {
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
                let s = self.stack_mut(mode.ret);
                let addr = if mode.keep {
                    s.peek_short()
                } else {
                    s.pop_short()
                };

                let v = if mode.short {
                    let hi = self.ram[usize::from(addr)];
                    let lo = self.ram[usize::from(addr.wrapping_add(1))];
                    Value::Short(u16::from_be_bytes([hi, lo]))
                } else {
                    let v = self.ram[usize::from(addr)];
                    Value::Byte(v)
                };
                let s = self.stack_mut(mode.ret);
                s.push(v);
            }
            Op::Sta(mode) => {
                let s = self.stack_mut(mode.ret);
                let addr = if mode.keep {
                    s.peek_short()
                } else {
                    s.pop_short()
                };

                let v = if mode.keep {
                    s.peek_at(1, mode.short)
                } else {
                    s.pop(mode.short)
                };
                match v {
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
                // TODO pass in devs, instead of stealing them here?
                let s = self.stack_mut(mode.ret);
                let i = s.peekpop_byte(mode.keep);
                // Short mode is weird here, but I think this matches the C
                // reference implementation.
                let a = dev.dei(self, i);
                self.stack_mut(mode.ret).push_byte(a);
                if mode.short {
                    let i = i.wrapping_add(1);
                    let a = dev.dei(self, i);
                    self.stack_mut(mode.ret).push_byte(a);
                }
            }
            Op::Deo(mode) => {
                // TODO pass in devs, instead of stealing them here?
                let s = self.stack_mut(mode.ret);
                let i = s.peekpop_byte(mode.keep);
                let v = s.peekpop_byte_at(1, mode.keep);
                // Short mode is weird here, but I think this matches the C
                // reference implementation.
                dev.deo(self, i, v);
                if mode.short {
                    let i = i.wrapping_add(1);
                    let s = self.stack_mut(mode.ret);
                    let v = s.peekpop_byte_at(2, mode.keep);
                    dev.deo(self, i, v);
                }
            }
            Op::Add(mode) => {
                self.op_bin(mode, |a, b| a.wrapping_add(b));
            }
            Op::Sub(mode) => {
                self.op_bin(mode, |a, b| a.wrapping_sub(b));
            }
            Op::Mul(mode) => {
                self.op_bin(mode, |a, b| a.wrapping_mul(b));
            }
            Op::Div(mode) => {
                self.op_bin(mode, |a, b| if b != 0 { a / b } else { 0 });
            }
            Op::And(mode) => {
                self.op_bin(mode, |a, b| a & b);
            }
            Op::Ora(mode) => {
                self.op_bin(mode, |a, b| a | b);
            }
            Op::Eor(mode) => {
                self.op_bin(mode, |a, b| a ^ b);
            }
            Op::Sft(mode) => {
                let s = self.stack_mut(mode.ret);
                let shift = s.peekpop_byte(mode.keep);
                let shr = u32::from(shift & 0xF);
                let shl = u32::from(shift >> 4);
                let v = if mode.keep {
                    s.peek_at(1, mode.short)
                } else {
                    s.pop(mode.short)
                };
                s.push(v.wrapping_shr(shr).wrapping_shl(shl));
            }
        }

        false
    }

    fn op_cmp<F: Fn(u16, u16) -> bool>(&mut self, mode: Mode, f: F) {
        let s = self.stack_mut(mode.ret);
        let v = if mode.short {
            let (a, b) = (s.peek_short_at(0), s.peek_short_at(2));
            if !mode.keep {
                s.drop(4);
            }
            f(a, b)
        } else {
            let (a, b) = (s.peek_byte_at(0), s.peek_byte_at(1));
            if !mode.keep {
                s.drop(2);
            }
            f(u16::from(a), u16::from(b))
        };
        s.push_byte(v as u8);
    }

    fn op_bin<F: Fn(u16, u16) -> u16>(&mut self, mode: Mode, f: F) {
        let s = self.stack_mut(mode.ret);
        if mode.short {
            let (a, b) = (s.peek_short_at(2), s.peek_short_at(0));
            if !mode.keep {
                s.drop(4);
            }
            s.push_short(f(a, b));
        } else {
            let (a, b) = (s.peek_byte_at(1), s.peek_byte_at(0));
            if !mode.keep {
                s.drop(2);
            }
            s.push_byte(f(u16::from(a), u16::from(b)) as u8);
        }
    }

    fn stack_mut(&mut self, ret: bool) -> &mut Stack {
        if ret {
            &mut self.ret
        } else {
            &mut self.stack
        }
    }
}

pub trait Device {
    fn dei(&mut self, vm: &mut Vm, target: u8) -> u8;
    fn deo(&mut self, vm: &mut Vm, target: u8, value: u8);
}

#[cfg(test)]
mod test {
    use super::*;

    struct EmptyDevice;
    impl Device for EmptyDevice {
        fn dei(&mut self, _vm: &mut Vm, _target: u8) -> u8 {
            0
        }
        fn deo(&mut self, _vm: &mut Vm, _target: u8, _value: u8) {
            // nothing to do here
        }
    }

    fn parse_and_test(s: &str) {
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
