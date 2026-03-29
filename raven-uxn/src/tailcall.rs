use crate::{Backend, Device, Stack, UxnCore};

type TailFn = for<'a> extern "rust-preserve-none" fn(
    &'a mut [u8; 256],
    u8,
    &'a mut [u8; 256],
    u8,
    &'a mut [u8; 256],
    &'a mut [u8; 65536],
    Backend,
    u16,
    &mut dyn Device,
) -> (UxnCore<'a>, u16);

#[cfg_attr(target_arch = "wasm32", target_feature(enable = "tail-call"))]
pub fn entry<'a>(
    core: UxnCore<'a>,
    dev: &mut dyn Device,
    pc: u16,
) -> (UxnCore<'a>, u16) {
    dispatch(
        core.stack.data,
        core.stack.index,
        core.ret.data,
        core.ret.index,
        core.dev,
        core.ram,
        core.backend,
        pc,
        dev,
    )
}

#[repr(C, align(64))] // align to cache line
struct FunctionTable([TailFn; 256]);

const TABLE: FunctionTable = FunctionTable([
    brk,
    inc::<0b000>,
    pop::<0b000>,
    nip::<0b000>,
    swp::<0b000>,
    rot::<0b000>,
    dup::<0b000>,
    ovr::<0b000>,
    equ::<0b000>,
    neq::<0b000>,
    gth::<0b000>,
    lth::<0b000>,
    jmp::<0b000>,
    jcn::<0b000>,
    jsr::<0b000>,
    sth::<0b000>,
    ldz::<0b000>,
    stz::<0b000>,
    ldr::<0b000>,
    str::<0b000>,
    lda::<0b000>,
    sta::<0b000>,
    dei::<0b000>,
    deo::<0b000>,
    add::<0b000>,
    sub::<0b000>,
    mul::<0b000>,
    div::<0b000>,
    and::<0b000>,
    ora::<0b000>,
    eor::<0b000>,
    sft::<0b000>,
    jci,
    inc::<0b001>,
    pop::<0b001>,
    nip::<0b001>,
    swp::<0b001>,
    rot::<0b001>,
    dup::<0b001>,
    ovr::<0b001>,
    equ::<0b001>,
    neq::<0b001>,
    gth::<0b001>,
    lth::<0b001>,
    jmp::<0b001>,
    jcn::<0b001>,
    jsr::<0b001>,
    sth::<0b001>,
    ldz::<0b001>,
    stz::<0b001>,
    ldr::<0b001>,
    str::<0b001>,
    lda::<0b001>,
    sta::<0b001>,
    dei::<0b001>,
    deo::<0b001>,
    add::<0b001>,
    sub::<0b001>,
    mul::<0b001>,
    div::<0b001>,
    and::<0b001>,
    ora::<0b001>,
    eor::<0b001>,
    sft::<0b001>,
    jmi,
    inc::<0b010>,
    pop::<0b010>,
    nip::<0b010>,
    swp::<0b010>,
    rot::<0b010>,
    dup::<0b010>,
    ovr::<0b010>,
    equ::<0b010>,
    neq::<0b010>,
    gth::<0b010>,
    lth::<0b010>,
    jmp::<0b010>,
    jcn::<0b010>,
    jsr::<0b010>,
    sth::<0b010>,
    ldz::<0b010>,
    stz::<0b010>,
    ldr::<0b010>,
    str::<0b010>,
    lda::<0b010>,
    sta::<0b010>,
    dei::<0b010>,
    deo::<0b010>,
    add::<0b010>,
    sub::<0b010>,
    mul::<0b010>,
    div::<0b010>,
    and::<0b010>,
    ora::<0b010>,
    eor::<0b010>,
    sft::<0b010>,
    jsi,
    inc::<0b011>,
    pop::<0b011>,
    nip::<0b011>,
    swp::<0b011>,
    rot::<0b011>,
    dup::<0b011>,
    ovr::<0b011>,
    equ::<0b011>,
    neq::<0b011>,
    gth::<0b011>,
    lth::<0b011>,
    jmp::<0b011>,
    jcn::<0b011>,
    jsr::<0b011>,
    sth::<0b011>,
    ldz::<0b011>,
    stz::<0b011>,
    ldr::<0b011>,
    str::<0b011>,
    lda::<0b011>,
    sta::<0b011>,
    dei::<0b011>,
    deo::<0b011>,
    add::<0b011>,
    sub::<0b011>,
    mul::<0b011>,
    div::<0b011>,
    and::<0b011>,
    ora::<0b011>,
    eor::<0b011>,
    sft::<0b011>,
    lit::<0b100>,
    inc::<0b100>,
    pop::<0b100>,
    nip::<0b100>,
    swp::<0b100>,
    rot::<0b100>,
    dup::<0b100>,
    ovr::<0b100>,
    equ::<0b100>,
    neq::<0b100>,
    gth::<0b100>,
    lth::<0b100>,
    jmp::<0b100>,
    jcn::<0b100>,
    jsr::<0b100>,
    sth::<0b100>,
    ldz::<0b100>,
    stz::<0b100>,
    ldr::<0b100>,
    str::<0b100>,
    lda::<0b100>,
    sta::<0b100>,
    dei::<0b100>,
    deo::<0b100>,
    add::<0b100>,
    sub::<0b100>,
    mul::<0b100>,
    div::<0b100>,
    and::<0b100>,
    ora::<0b100>,
    eor::<0b100>,
    sft::<0b100>,
    lit::<0b101>,
    inc::<0b101>,
    pop::<0b101>,
    nip::<0b101>,
    swp::<0b101>,
    rot::<0b101>,
    dup::<0b101>,
    ovr::<0b101>,
    equ::<0b101>,
    neq::<0b101>,
    gth::<0b101>,
    lth::<0b101>,
    jmp::<0b101>,
    jcn::<0b101>,
    jsr::<0b101>,
    sth::<0b101>,
    ldz::<0b101>,
    stz::<0b101>,
    ldr::<0b101>,
    str::<0b101>,
    lda::<0b101>,
    sta::<0b101>,
    dei::<0b101>,
    deo::<0b101>,
    add::<0b101>,
    sub::<0b101>,
    mul::<0b101>,
    div::<0b101>,
    and::<0b101>,
    ora::<0b101>,
    eor::<0b101>,
    sft::<0b101>,
    lit::<0b110>,
    inc::<0b110>,
    pop::<0b110>,
    nip::<0b110>,
    swp::<0b110>,
    rot::<0b110>,
    dup::<0b110>,
    ovr::<0b110>,
    equ::<0b110>,
    neq::<0b110>,
    gth::<0b110>,
    lth::<0b110>,
    jmp::<0b110>,
    jcn::<0b110>,
    jsr::<0b110>,
    sth::<0b110>,
    ldz::<0b110>,
    stz::<0b110>,
    ldr::<0b110>,
    str::<0b110>,
    lda::<0b110>,
    sta::<0b110>,
    dei::<0b110>,
    deo::<0b110>,
    add::<0b110>,
    sub::<0b110>,
    mul::<0b110>,
    div::<0b110>,
    and::<0b110>,
    ora::<0b110>,
    eor::<0b110>,
    sft::<0b110>,
    lit::<0b111>,
    inc::<0b111>,
    pop::<0b111>,
    nip::<0b111>,
    swp::<0b111>,
    rot::<0b111>,
    dup::<0b111>,
    ovr::<0b111>,
    equ::<0b111>,
    neq::<0b111>,
    gth::<0b111>,
    lth::<0b111>,
    jmp::<0b111>,
    jcn::<0b111>,
    jsr::<0b111>,
    sth::<0b111>,
    ldz::<0b111>,
    stz::<0b111>,
    ldr::<0b111>,
    str::<0b111>,
    lda::<0b111>,
    sta::<0b111>,
    dei::<0b111>,
    deo::<0b111>,
    add::<0b111>,
    sub::<0b111>,
    mul::<0b111>,
    div::<0b111>,
    and::<0b111>,
    ora::<0b111>,
    eor::<0b111>,
    sft::<0b111>,
]);

extern "rust-preserve-none" fn dispatch<'a>(
    stack_data: &'a mut [u8; 256],
    stack_index: u8,
    rstack_data: &'a mut [u8; 256],
    rstack_index: u8,
    dev: &'a mut [u8; 256],
    ram: &'a mut [u8; 65536],
    backend: Backend,
    mut pc: u16,
    vdev: &mut dyn Device,
) -> (UxnCore<'a>, u16) {
    let mut core = UxnCore {
        stack: Stack {
            data: stack_data,
            index: stack_index,
        },
        ret: Stack {
            data: rstack_data,
            index: rstack_index,
        },
        dev,
        ram,
        backend,
    };
    let op = core.next(&mut pc);
    become TABLE.0[op as usize](
        core.stack.data,
        core.stack.index,
        core.ret.data,
        core.ret.index,
        core.dev,
        core.ram,
        core.backend,
        pc,
        vdev,
    )
}

macro_rules! tail_fn {
    ($name:ident $(::<$flags:ident>)?) => {
        tail_fn!($name $(::<$flags>)?[][vdev: &mut dyn Device]);
    };
    ($name:ident $(::<$flags:ident>)?($($arg:ident: $ty:ty),*)) => {
        tail_fn!($name $(::<$flags>)?[$($arg: $ty),*][]);
    };
    ($name:ident $(::<$flags:ident>)?[$($arg0:ident: $ty0:ty),*][$($arg1:ident: $ty1:ty),*]) => {
        extern "rust-preserve-none" fn $name<'a, $(const $flags: u8)?>(
            stack_data: &'a mut [u8; 256],
            stack_index: u8,
            rstack_data: &'a mut [u8; 256],
            rstack_index: u8,
            dev: &'a mut [u8; 256],
            ram: &'a mut [u8; 65536],
            backend: Backend,
            pc: u16,
            $($arg0: $ty0),*
            $($arg1: $ty1),*
        ) -> (UxnCore<'a>, u16) {
            let mut core = UxnCore {
                stack: Stack {
                    data: stack_data,
                    index: stack_index,
                },
                ret: Stack {
                    data: rstack_data,
                    index: rstack_index,
                },
                dev,
                ram,
                backend,
            };
            match core.$name::<$($flags)?>(pc, $($arg0),*) {
                Some(pc) => {
                    become dispatch(
                        core.stack.data,
                        core.stack.index,
                        core.ret.data,
                        core.ret.index,
                        core.dev,
                        core.ram,
                        core.backend,
                        pc,
                        $($arg0),*
                        $($arg1),*
                    )
                }
                None => (core, pc),
            }
        }
    };
}

tail_fn!(brk);
tail_fn!(jci);
tail_fn!(jmi);
tail_fn!(jsi);
tail_fn!(inc::<FLAGS>);
tail_fn!(pop::<FLAGS>);
tail_fn!(nip::<FLAGS>);
tail_fn!(swp::<FLAGS>);
tail_fn!(rot::<FLAGS>);
tail_fn!(dup::<FLAGS>);
tail_fn!(ovr::<FLAGS>);
tail_fn!(equ::<FLAGS>);
tail_fn!(neq::<FLAGS>);
tail_fn!(gth::<FLAGS>);
tail_fn!(lth::<FLAGS>);
tail_fn!(jmp::<FLAGS>);
tail_fn!(jcn::<FLAGS>);
tail_fn!(jsr::<FLAGS>);
tail_fn!(sth::<FLAGS>);
tail_fn!(ldz::<FLAGS>);
tail_fn!(stz::<FLAGS>);
tail_fn!(ldr::<FLAGS>);
tail_fn!(str::<FLAGS>);
tail_fn!(lda::<FLAGS>);
tail_fn!(sta::<FLAGS>);
tail_fn!(add::<FLAGS>);
tail_fn!(sub::<FLAGS>);
tail_fn!(mul::<FLAGS>);
tail_fn!(div::<FLAGS>);
tail_fn!(and::<FLAGS>);
tail_fn!(ora::<FLAGS>);
tail_fn!(eor::<FLAGS>);
tail_fn!(sft::<FLAGS>);
tail_fn!(lit::<FLAGS>);
tail_fn!(dei::<FLAGS>(dev: &mut dyn Device));
tail_fn!(deo::<FLAGS>(dev: &mut dyn Device));
