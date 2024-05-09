//! Uxn virtual machine

fn keep(flags: u8) -> bool {
    (flags & (1 << 2)) != 0
}
fn short(flags: u8) -> bool {
    (flags & (1 << 0)) != 0
}
fn ret(flags: u8) -> bool {
    (flags & (1 << 1)) != 0
}

/// Simple circular stack, with room for 256 items
#[derive(Debug)]
pub(crate) struct Stack {
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
    fn pop(&mut self) -> Value {
        self.pop_type(short(FLAGS))
    }

    fn pop_type(&mut self, short: bool) -> Value {
        if keep(FLAGS) {
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

    fn reserve(&mut self, n: u8) {
        self.stack.reserve(n);
    }

    /// Replaces the top item on the stack with the given value
    fn emplace(&mut self, v: Value) {
        match v {
            Value::Short(..) => {
                self.pop_short();
            }
            Value::Byte(..) => {
                self.pop_byte();
            }
        }
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
    fn reserve(&mut self, n: u8) {
        self.index = self.index.wrapping_add(n);
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

    pub fn peek_byte_at(&self, offset: u8) -> u8 {
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

    /// Returns the number of items in the stack
    pub fn len(&self) -> u8 {
        self.index.wrapping_add(1)
    }

    /// Sets the number of items in the stack
    pub fn set_len(&mut self, n: u8) {
        self.index = n.wrapping_sub(1);
    }
}

/// The virtual machine itself
pub struct Uxn {
    /// Device memory
    dev: [u8; 256],
    /// 64 KiB of VM memory
    pub(crate) ram: Box<[u8; 65536]>,
    /// 256-byte data stack
    pub(crate) stack: Stack,
    /// 256-byte return stack
    pub(crate) ret: Stack,
}

impl Default for Uxn {
    fn default() -> Self {
        Self {
            dev: [0u8; 256],
            ram: Box::new([0u8; 65536]),
            stack: Stack::default(),
            ret: Stack::default(),
        }
    }
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

impl Uxn {
    /// Build a new `Uxn`, loading the given ROM at the start address
    ///
    /// # Panics
    /// If `rom` cannot fit in memory
    pub fn new(rom: &[u8]) -> Self {
        let mut out = Self {
            dev: [0u8; 256],
            ram: Box::new([0u8; 65536]),
            stack: Stack::default(),
            ret: Stack::default(),
        };
        out.ram[0x100..][..rom.len()].copy_from_slice(rom);
        out
    }

    /// Reads a byte from RAM at the program counter
    fn next(&mut self, pc: &mut u16) -> u8 {
        let out = self.ram[usize::from(*pc)];
        *pc = pc.wrapping_add(1);
        out
    }

    /// Reads a word from RAM at the program counter
    fn next2(&mut self, pc: &mut u16) -> u16 {
        let hi = self.next(pc);
        let lo = self.next(pc);
        u16::from_be_bytes([hi, lo])
    }

    fn stack_view<const FLAGS: u8>(&mut self) -> StackView<FLAGS> {
        let stack = if ret(FLAGS) {
            &mut self.ret
        } else {
            &mut self.stack
        };
        StackView::new(stack)
    }

    fn ret_stack_view<const FLAGS: u8>(&mut self) -> StackView<FLAGS> {
        let stack = if ret(FLAGS) {
            &mut self.stack
        } else {
            &mut self.ret
        };
        StackView::new(stack)
    }

    /// Reads a byte from device memory
    pub fn dev_read(&self, addr: u8) -> u8 {
        self.dev[addr as usize]
    }

    /// Writes a byte to device memory
    pub fn dev_write(&mut self, addr: u8, v: u8) {
        self.dev[addr as usize] = v;
    }

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
    fn op<D: Device>(&mut self, op: u8, dev: &mut D, pc: u16) -> Option<u16> {
        type FnOp = fn(&mut Uxn, &mut dyn Device, u16) -> Option<u16>;
        const OPCODES: [FnOp; 256] = [
            op::brk,
            op::inc::<0b000>,
            op::pop::<0b000>,
            op::nip::<0b000>,
            op::swp::<0b000>,
            op::rot::<0b000>,
            op::dup::<0b000>,
            op::ovr::<0b000>,
            op::equ::<0b000>,
            op::neq::<0b000>,
            op::gth::<0b000>,
            op::lth::<0b000>,
            op::jmp::<0b000>,
            op::jcn::<0b000>,
            op::jsr::<0b000>,
            op::sth::<0b000>,
            op::ldz::<0b000>,
            op::stz::<0b000>,
            op::ldr::<0b000>,
            op::str::<0b000>,
            op::lda::<0b000>,
            op::sta::<0b000>,
            op::dei::<0b000>,
            op::deo::<0b000>,
            op::add::<0b000>,
            op::sub::<0b000>,
            op::mul::<0b000>,
            op::div::<0b000>,
            op::and::<0b000>,
            op::ora::<0b000>,
            op::eor::<0b000>,
            op::sft::<0b000>,
            op::jci,
            op::inc::<0b001>,
            op::pop::<0b001>,
            op::nip::<0b001>,
            op::swp::<0b001>,
            op::rot::<0b001>,
            op::dup::<0b001>,
            op::ovr::<0b001>,
            op::equ::<0b001>,
            op::neq::<0b001>,
            op::gth::<0b001>,
            op::lth::<0b001>,
            op::jmp::<0b001>,
            op::jcn::<0b001>,
            op::jsr::<0b001>,
            op::sth::<0b001>,
            op::ldz::<0b001>,
            op::stz::<0b001>,
            op::ldr::<0b001>,
            op::str::<0b001>,
            op::lda::<0b001>,
            op::sta::<0b001>,
            op::dei::<0b001>,
            op::deo::<0b001>,
            op::add::<0b001>,
            op::sub::<0b001>,
            op::mul::<0b001>,
            op::div::<0b001>,
            op::and::<0b001>,
            op::ora::<0b001>,
            op::eor::<0b001>,
            op::sft::<0b001>,
            op::jmi,
            op::inc::<0b010>,
            op::pop::<0b010>,
            op::nip::<0b010>,
            op::swp::<0b010>,
            op::rot::<0b010>,
            op::dup::<0b010>,
            op::ovr::<0b010>,
            op::equ::<0b010>,
            op::neq::<0b010>,
            op::gth::<0b010>,
            op::lth::<0b010>,
            op::jmp::<0b010>,
            op::jcn::<0b010>,
            op::jsr::<0b010>,
            op::sth::<0b010>,
            op::ldz::<0b010>,
            op::stz::<0b010>,
            op::ldr::<0b010>,
            op::str::<0b010>,
            op::lda::<0b010>,
            op::sta::<0b010>,
            op::dei::<0b010>,
            op::deo::<0b010>,
            op::add::<0b010>,
            op::sub::<0b010>,
            op::mul::<0b010>,
            op::div::<0b010>,
            op::and::<0b010>,
            op::ora::<0b010>,
            op::eor::<0b010>,
            op::sft::<0b010>,
            op::jsi,
            op::inc::<0b011>,
            op::pop::<0b011>,
            op::nip::<0b011>,
            op::swp::<0b011>,
            op::rot::<0b011>,
            op::dup::<0b011>,
            op::ovr::<0b011>,
            op::equ::<0b011>,
            op::neq::<0b011>,
            op::gth::<0b011>,
            op::lth::<0b011>,
            op::jmp::<0b011>,
            op::jcn::<0b011>,
            op::jsr::<0b011>,
            op::sth::<0b011>,
            op::ldz::<0b011>,
            op::stz::<0b011>,
            op::ldr::<0b011>,
            op::str::<0b011>,
            op::lda::<0b011>,
            op::sta::<0b011>,
            op::dei::<0b011>,
            op::deo::<0b011>,
            op::add::<0b011>,
            op::sub::<0b011>,
            op::mul::<0b011>,
            op::div::<0b011>,
            op::and::<0b011>,
            op::ora::<0b011>,
            op::eor::<0b011>,
            op::sft::<0b011>,
            op::lit::<0b100>,
            op::inc::<0b100>,
            op::pop::<0b100>,
            op::nip::<0b100>,
            op::swp::<0b100>,
            op::rot::<0b100>,
            op::dup::<0b100>,
            op::ovr::<0b100>,
            op::equ::<0b100>,
            op::neq::<0b100>,
            op::gth::<0b100>,
            op::lth::<0b100>,
            op::jmp::<0b100>,
            op::jcn::<0b100>,
            op::jsr::<0b100>,
            op::sth::<0b100>,
            op::ldz::<0b100>,
            op::stz::<0b100>,
            op::ldr::<0b100>,
            op::str::<0b100>,
            op::lda::<0b100>,
            op::sta::<0b100>,
            op::dei::<0b100>,
            op::deo::<0b100>,
            op::add::<0b100>,
            op::sub::<0b100>,
            op::mul::<0b100>,
            op::div::<0b100>,
            op::and::<0b100>,
            op::ora::<0b100>,
            op::eor::<0b100>,
            op::sft::<0b100>,
            op::lit::<0b101>,
            op::inc::<0b101>,
            op::pop::<0b101>,
            op::nip::<0b101>,
            op::swp::<0b101>,
            op::rot::<0b101>,
            op::dup::<0b101>,
            op::ovr::<0b101>,
            op::equ::<0b101>,
            op::neq::<0b101>,
            op::gth::<0b101>,
            op::lth::<0b101>,
            op::jmp::<0b101>,
            op::jcn::<0b101>,
            op::jsr::<0b101>,
            op::sth::<0b101>,
            op::ldz::<0b101>,
            op::stz::<0b101>,
            op::ldr::<0b101>,
            op::str::<0b101>,
            op::lda::<0b101>,
            op::sta::<0b101>,
            op::dei::<0b101>,
            op::deo::<0b101>,
            op::add::<0b101>,
            op::sub::<0b101>,
            op::mul::<0b101>,
            op::div::<0b101>,
            op::and::<0b101>,
            op::ora::<0b101>,
            op::eor::<0b101>,
            op::sft::<0b101>,
            op::lit::<0b110>,
            op::inc::<0b110>,
            op::pop::<0b110>,
            op::nip::<0b110>,
            op::swp::<0b110>,
            op::rot::<0b110>,
            op::dup::<0b110>,
            op::ovr::<0b110>,
            op::equ::<0b110>,
            op::neq::<0b110>,
            op::gth::<0b110>,
            op::lth::<0b110>,
            op::jmp::<0b110>,
            op::jcn::<0b110>,
            op::jsr::<0b110>,
            op::sth::<0b110>,
            op::ldz::<0b110>,
            op::stz::<0b110>,
            op::ldr::<0b110>,
            op::str::<0b110>,
            op::lda::<0b110>,
            op::sta::<0b110>,
            op::dei::<0b110>,
            op::deo::<0b110>,
            op::add::<0b110>,
            op::sub::<0b110>,
            op::mul::<0b110>,
            op::div::<0b110>,
            op::and::<0b110>,
            op::ora::<0b110>,
            op::eor::<0b110>,
            op::sft::<0b110>,
            op::lit::<0b111>,
            op::inc::<0b111>,
            op::pop::<0b111>,
            op::nip::<0b111>,
            op::swp::<0b111>,
            op::rot::<0b111>,
            op::dup::<0b111>,
            op::ovr::<0b111>,
            op::equ::<0b111>,
            op::neq::<0b111>,
            op::gth::<0b111>,
            op::lth::<0b111>,
            op::jmp::<0b111>,
            op::jcn::<0b111>,
            op::jsr::<0b111>,
            op::sth::<0b111>,
            op::ldz::<0b111>,
            op::stz::<0b111>,
            op::ldr::<0b111>,
            op::str::<0b111>,
            op::lda::<0b111>,
            op::sta::<0b111>,
            op::dei::<0b111>,
            op::deo::<0b111>,
            op::add::<0b111>,
            op::sub::<0b111>,
            op::mul::<0b111>,
            op::div::<0b111>,
            op::and::<0b111>,
            op::ora::<0b111>,
            op::eor::<0b111>,
            op::sft::<0b111>,
        ];
        OPCODES[op as usize](self, dev, pc)
    }
}

mod op {
    use super::*;
    /// Break
    /// ```text
    /// BRK --
    /// ```
    ///
    /// Ends the evaluation of the current vector. This opcode has no modes.
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
    pub fn jci(vm: &mut Uxn, _: &mut dyn Device, mut pc: u16) -> Option<u16> {
        let dt = vm.next2(&mut pc);
        if vm.stack.pop_byte() != 0 {
            pc = pc.wrapping_add(dt);
        }
        Some(pc)
    }

    /// Jump Instant
    ///
    /// JMI  -- Moves the PC to a relative address at a distance equal to the next
    /// short in memory. This opcode has no modes.
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
    pub fn jmp<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        let mut s = vm.stack_view::<FLAGS>();
        Some(if short(FLAGS) {
            s.pop_short()
        } else {
            pc.wrapping_add(u16::from(s.pop_byte()))
        })
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
    pub fn jcn<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        mut pc: u16,
    ) -> Option<u16> {
        let mut s = vm.stack_view::<FLAGS>();
        let dst = s.pop();
        let cond = s.pop_byte();
        if cond != 0 {
            pc = match dst {
                Value::Short(dst) => dst,
                Value::Byte(offset) => pc.wrapping_add(u16::from(offset)),
            };
        }
        Some(pc)
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
    ///
    pub fn jsr<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        vm.ret.push(Value::Short(pc));
        let mut s = vm.stack_view::<FLAGS>();
        Some(match s.pop() {
            Value::Short(v) => v,
            Value::Byte(v) => pc.wrapping_add(u16::from(v)),
        })
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
    pub fn ldz<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        let addr = vm.stack_view::<FLAGS>().pop_byte();
        let v = if short(FLAGS) {
            let hi = vm.ram[usize::from(addr)];
            let lo = vm.ram[usize::from(addr.wrapping_add(1))];
            Value::Short(u16::from_be_bytes([hi, lo]))
        } else {
            let v = vm.ram[usize::from(addr)];
            Value::Byte(v)
        };
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
    pub fn stz<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        let mut s = vm.stack_view::<FLAGS>();
        let addr = s.pop_byte();
        match s.pop() {
            Value::Short(v) => {
                let [hi, lo] = v.to_be_bytes();
                vm.ram[usize::from(addr)] = hi;
                vm.ram[usize::from(addr.wrapping_add(1))] = lo;
            }
            Value::Byte(v) => {
                vm.ram[usize::from(addr)] = v;
            }
        }
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
    pub fn ldr<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        let offset = vm.stack_view::<FLAGS>().pop_byte() as i8;

        // TODO: make this more obviously infallible
        let addr = if offset < 0 {
            pc.wrapping_sub(i16::from(offset).abs().try_into().unwrap())
        } else {
            pc.wrapping_add(u16::try_from(offset).unwrap())
        };

        let v = if short(FLAGS) {
            let hi = vm.ram[usize::from(addr)];
            let lo = vm.ram[usize::from(addr.wrapping_add(1))];
            Value::Short(u16::from_be_bytes([hi, lo]))
        } else {
            let v = vm.ram[usize::from(addr)];
            Value::Byte(v)
        };
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
    pub fn str<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        let mut s = vm.stack_view::<FLAGS>();
        let offset = s.pop_byte() as i8;
        let addr = if offset < 0 {
            pc.wrapping_sub(i16::from(offset).abs().try_into().unwrap())
        } else {
            pc.wrapping_add(u16::try_from(offset).unwrap())
        };
        match s.pop() {
            Value::Short(v) => {
                let [hi, lo] = v.to_be_bytes();
                vm.ram[usize::from(addr)] = hi;
                vm.ram[usize::from(addr.wrapping_add(1))] = lo;
            }
            Value::Byte(v) => {
                vm.ram[usize::from(addr)] = v;
            }
        }
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
    pub fn lda<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        let addr = vm.stack_view::<FLAGS>().pop_short();
        let v = if short(FLAGS) {
            let hi = vm.ram[usize::from(addr)];
            let lo = vm.ram[usize::from(addr.wrapping_add(1))];
            Value::Short(u16::from_be_bytes([hi, lo]))
        } else {
            let v = vm.ram[usize::from(addr)];
            Value::Byte(v)
        };
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
    pub fn sta<const FLAGS: u8>(
        vm: &mut Uxn,
        _: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        let mut s = vm.stack_view::<FLAGS>();
        let addr = s.pop_short();
        match s.pop() {
            Value::Short(v) => {
                let [hi, lo] = v.to_be_bytes();
                vm.ram[usize::from(addr)] = hi;
                vm.ram[usize::from(addr.wrapping_add(1))] = lo;
            }
            Value::Byte(v) => {
                vm.ram[usize::from(addr)] = v;
            }
        }
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
            dev.dei(vm, i.wrapping_add(1));
            let lo = vm.dev[j as usize];
            Value::Short(u16::from_be_bytes([hi, lo]))
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
    pub fn deo<const FLAGS: u8>(
        vm: &mut Uxn,
        dev: &mut dyn Device,
        pc: u16,
    ) -> Option<u16> {
        let mut s = vm.stack_view::<FLAGS>();
        let i = s.pop_byte();
        match s.pop() {
            Value::Short(v) => {
                let [hi, lo] = v.to_be_bytes();
                let j = i.wrapping_add(1);
                vm.dev[i as usize] = hi;
                dev.deo(vm, i);
                vm.dev[j as usize] = lo;
                dev.deo(vm, j);
            }
            Value::Byte(v) => {
                vm.dev[i as usize] = v;
                dev.deo(vm, i);
            }
        }
        Some(pc)
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
    fn deo(&mut self, vm: &mut Uxn, target: u8);
}

/// Device which does nothing
pub struct EmptyDevice;
impl Device for EmptyDevice {
    fn dei(&mut self, _vm: &mut Uxn, _target: u8) {
        // nothing to do here
    }
    fn deo(&mut self, _vm: &mut Uxn, _target: u8) {
        // nothing to do here
    }
}

#[cfg(test)]
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
        let mut vm = Uxn::default();
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
                        "failed to execute {:?}: got {actual:2x?}, expected {expected:2x?}",
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
