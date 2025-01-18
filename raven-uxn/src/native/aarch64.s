// x0 - stack pointer (&mut [u8; 256])
// x1 - stack index (u8)
// x2 - return stack pointer (&mut [u8; 256])
// x3 - return stack index (u8)
// x4 - RAM pointer (&mut [u8; 65536])
// x5 - program counter (u16), offset of the next value in RAM
// x6 - VM pointer (&mut Uxn)
// x7 - Device handle pointer (&DeviceHandle)
// x8 - Jump table pointer (loaded in aarch64_entry)
// x9-15 - scratch registers
//
// We do not use any callee-saved registers (besides x29 / x30)
.macro next
    ldrb w9, [x4, x5]
    add x5, x5, #1
    and x5, x5, #0xffff
    ldr x10, [x8, x9, lsl #3]
    br x10
.endm

.macro pop
    sub x1, x1, #1
    and x1, x1, #0xff
.endm

.macro rpop
    sub x3, x3, #1
    and x3, x3, #0xff
.endm

.macro push, reg
    add x1, x1, #1
    and x1, x1, #0xff
    strb \reg, [x0, x1]
.endm

.macro rpush, reg
    add x3, x3, #1
    and x3, x3, #0xff
    strb \reg, [x2, x3]
.endm

.macro peek, w, x, n
    sub \x, x1, \n
    and \x, \x, #0xff
    ldrb \w, [x0, \x]
.endm

.macro rpeek, w, x, n
    sub \x, x3, \n
    and \x, \x, #0xff
    ldrb \w, [x2, \x]
.endm

.macro precall
    // We have to write our stack index pointers back into the &mut Uxn
    ldp x11, x12, [sp, 0x10] // restore stack index pointers
    strb w1, [x11]   // modify stack index pointer
    strb w3, [x12]   // modify return stack index pointer

    stp x0, x1, [sp, #0x20]
    stp x2, x3, [sp, #0x30]
    stp x5, x4, [sp, #0x40]
    stp x6, x7, [sp, #0x50]
    str x8,     [sp, #0x60]

    mov x0, x6
    mov x1, x7
.endm

.macro postcall
    ldp x0, x1, [sp, #0x20]
    ldp x2, x3, [sp, #0x30]
    ldp x5, x4, [sp, #0x40]
    ldp x6, x7, [sp, #0x50]
    ldr x8,     [sp, #0x60]

    // The DEO operation may have changed stack pointers, so reload them here
    ldp x11, x12, [sp, 0x10]
    ldrb w1, [x11]
    ldrb w3, [x12]
.endm

ENTRY aarch64_entry
    sub sp, sp, #0x200          // make room in the stack
    stp   x29, x30, [sp, 0x0]   // store stack and frame pointer
    mov   x29, sp
    load_jump_table x8 // platform-dependent

    // Convert from index pointers to index values in w1 / w3
    stp x1, x3, [sp, 0x10]      // save stack index pointers
    ldrb w1, [x1]               // load stack index
    ldrb w3, [x3]               // load ret index

    // Jump into the instruction list
    next

_BRK:
    // Write index values back through index pointers
    ldp x9, x10, [sp, 0x10]     // restore stack index pointers
    strb w1, [x9]               // save stack index
    strb w3, [x10]              // save ret index

    ldp   x29, x30, [sp, 0x0]   // Restore stack and frame pointer
    add sp, sp, #0x200  // restore stack pointer

    mov x0, x5 // return PC from function
    ret

_INC:
    ldrb w9, [x0, x1]
    add w9, w9, #1
    strb w9, [x0, x1]
    next

_POP:
    pop
    next

_NIP:
    ldrb w9, [x0, x1]   // get the top byte
    pop
    strb w9, [x0, x1]   // overwrite the previous byte
    next

_SWP:
    ldrb w10, [x0, x1]   // get the top byte
    peek w11, x9, 1      // get the second-from-top byte
    strb w10, [x0, x9]   // do the swap!
    strb w11, [x0, x1]
    next

_ROT:
    // a b c -- b c a
    ldrb w10, [x0, x1] // c
    peek w12, x11, 1
    peek w14, x13, 2

    strb w14, [x0, x1]
    strb w12, [x0, x13]
    strb w10, [x0, x11]

    next

_DUP:
    ldrb w10, [x0, x1]   // get the top byte
    push w10
    next

_OVR:
    peek w10, x10, 1
    push w10
    next

.macro compare_op op
    ldrb w10, [x0, x1]
    pop
    ldrb w11, [x0, x1]
    cmp w11, w10
    cset w10, \op
    strb w10, [x0, x1]
    next
.endm

_EQU:
    compare_op eq

_NEQ:
    compare_op ne

_GTH:
    compare_op hi

_LTH:
    compare_op lo

_JMP:
    ldrsb x9, [x0, x1]
    pop
    add x5, x5, x9
    and x5, x5, 0xffff
    next

_JCN:
    ldrsb w9, [x0, x1]
    pop
    ldrb w10, [x0, x1]
    pop
    cmp w10, #0
    csel w10, wzr, w9, eq // choose the jump or not
    add x5, x5, x10 // jump or not
    and x5, x5, 0xffff
    next

_JSR:
    ldrsb w9, [x0, x1]
    pop
    lsr w10, w5, 8
    rpush w10
    rpush w5
    add x5, x5, x9
    and x5, x5, 0xffff
    next

_STH:
    ldrb w9, [x0, x1]
    pop
    rpush w9
    next

_LDZ:
    ldrb w9, [x0, x1]
    pop
    ldrb w9, [x4, x9]
    push w9
    next

_STZ:
    ldrb w9, [x0, x1]
    pop
    ldrb w10, [x0, x1]
    pop
    strb w10, [x4, x9]
    next

_LDR:
    ldrsb w9, [x0, x1]
    add x9, x5, x9
    and x9, x9, #0xffff
    ldrb w9, [x4, x9] // read from RAM
    strb w9, [x0, x1] // push to stack
    next

_STR:
    ldrsb w9, [x0, x1]
    pop
    ldrb w10, [x0, x1]
    pop
    add x9, x5, x9
    and x9, x9, #0xffff
    strb w10, [x4, x9] // write to RAM
    next

_LDA:
    ldrb w9, [x0, x1]
    pop
    ldrb w10, [x0, x1]
    orr w12, w9, w10, lsl #8
    ldrb w12, [x4, x12]
    strb w12, [x0, x1]
    next

_STA:
    ldrb w9, [x0, x1]
    pop
    ldrb w10, [x0, x1]
    pop
    orr w12, w9, w10, lsl #8
    ldrb w10, [x0, x1]
    pop
    strb w10, [x4, x12]
    next

_DEI:
    precall
    CALL dei_entry
    postcall
    next

_DEO:
    precall
    CALL deo_entry // todo check return value for early exit?
    postcall
    next

.macro binary_op op
    ldrb w10, [x0, x1]
    pop
    ldrb w11, [x0, x1]
    \op w10, w11, w10
    strb w10, [x0, x1]
    next
.endm

_ADD:
    binary_op add

_SUB:
    binary_op sub

_MUL:
    binary_op mul

_DIV:
    binary_op udiv

_AND:
    binary_op and

_ORA:
    binary_op orr

_EOR:
    binary_op eor

_SFT:
    ldrb w10, [x0, x1]
    pop
    ldrb w11, [x0, x1]
    lsr w12, w10, 4
    and w10, w10, #0xf
    lsr w11, w11, w10
    lsl w11, w11, w12
    strb w11, [x0, x1]
    next

_JCI:
    ldrb w9, [x4, x5]
    add x5, x5, #1
    and x5, x5, #0xffff
    ldrb w10, [x4, x5]
    add x5, x5, #1
    and x5, x5, #0xffff
    orr w12, w10, w9, lsl #8 // build the jump offset
    ldrb w10, [x0, x1] // read conditional byte
    pop
    cmp w10, #0
    csel w10, wzr, w12, eq // choose the jump or not
    add x5, x5, x10 // jump or not
    and x5, x5, 0xffff
    next

_INC2:
    ldrb w10, [x0, x1]  // get the top byte
    peek w11, x9, 1     // get the second-from-top byte
    orr w12, w10, w11, lsl #8
    add w12, w12, #1
    and w12, w12, #0xffff
    strb w12, [x0, x1]
    lsr w12, w12, 8
    strb w12, [x0, x9]
    next

_POP2:
    sub x1, x1, #2
    and x1, x1, #0xff
    next

_NIP2:
    ldrb w9, [x0, x1]
    pop
    ldrb w10, [x0, x1]
    pop
    strb w9, [x0, x1]
    sub x11, x1, #1
    and x11, x11, #0xff
    strb w10, [x0, x11]
    next

_SWP2:
    ldrb w11, [x0, x1]   // get the top byte
    peek w12, x9, 2       // get the second-from-top byte
    strb w11, [x0, x9]   // do the swap!
    strb w12, [x0, x1]

    peek w11, x9, 1
    peek w12, x10, 3
    strb w11, [x0, x10]
    strb w12, [x0, x9]

    next

_ROT2:
    ldrb w10, [x0, x1]
    peek w12, x11, 2
    peek w14, x13, 4
    strb w14, [x0, x1]
    strb w12, [x0, x13]
    strb w10, [x0, x11]

    peek w10, x15, 1
    peek w12, x11, 3
    peek w14, x13, 5
    strb w14, [x0, x15]
    strb w12, [x0, x13]
    strb w10, [x0, x11]

    next

_DUP2:
    ldrb w11, [x0, x1]
    peek w10, x10, 1
    push w10
    push w11
    next

_OVR2:
    peek w10, x9, 2
    peek w11, x9, 3
    push w11
    push w10
    next

.macro compare_op2 op
    ldrb w10, [x0, x1]
    pop
    ldrb w11, [x0, x1]
    pop
    orr w10, w10, w11, lsl #8
    ldrb w11, [x0, x1]
    pop
    ldrb w12, [x0, x1]
    orr w11, w11, w12, lsl #8
    cmp w11, w10
    cset w10, \op
    strb w10, [x0, x1]
    next
.endm

_EQU2:
    compare_op2 eq

_NEQ2:
    compare_op2 ne

_GTH2:
    compare_op2 hi

_LTH2:
    compare_op2 lo

_JMP2:
    ldrb w9, [x0, x1]
    pop
    ldrb w10, [x0, x1]
    pop
    orr w5, w9, w10, lsl #8 // update program counter
    next

_JCN2:
    ldrb w9, [x0, x1]
    pop
    ldrb w10, [x0, x1]
    pop
    ldrb w11, [x0, x1]
    pop
    orr w9, w9, w10, lsl #8 // update program counter
    cmp w11, #0
    csel w5, w5, w9, eq // choose the jump or not
    next

_JSR2:
    ldrb w9, [x0, x1]
    pop
    ldrb w10, [x0, x1]
    pop
    lsr w11, w5, 8
    rpush w11
    rpush w5
    orr w5, w9, w10, lsl #8 // update program counter
    next

_STH2:
    ldrb w9, [x0, x1]
    pop
    ldrb w10, [x0, x1]
    pop
    rpush w10
    rpush w9
    next

_LDZ2:
    ldrb w9, [x0, x1]
    pop
    ldrb w10, [x4, x9]
    push w10
    add w9, w9, #1
    and w9, w9, #0xFFFF
    ldrb w10, [x4, x9]
    push w10
    next

_STZ2:
    ldrb w9, [x0, x1]
    pop
    ldrb w10, [x0, x1]
    pop
    ldrb w11, [x0, x1]
    pop
    strb w11, [x4, x9]
    add w9, w9, #1
    and w9, w9, #0xFFFF
    strb w10, [x4, x9]
    next

_LDR2:
    ldrsb w9, [x0, x1]
    add x9, x5, x9
    and x9, x9, #0xffff
    ldrb w10, [x4, x9] // read from RAM
    strb w10, [x0, x1] // push to stack
    add x9, x9, #1
    and x9, x9, #0xffff
    ldrb w10, [x4, x9] // read from RAM
    push w10
    next

_STR2:
    ldrsb w9, [x0, x1]
    pop
    ldrsb w10, [x0, x1]
    pop
    ldrsb w11, [x0, x1]
    pop
    add x9, x5, x9
    and x9, x9, #0xffff
    strb w11, [x4, x9] // write to RAM
    add x9, x9, #1
    and x9, x9, #0xffff
    strb w10, [x4, x9] // write to RAM
    next

_LDA2:
    ldrb w9, [x0, x1]
    peek w10, x12, 1
    orr w9, w9, w10, lsl #8

    ldrb w10, [x4, x9]
    strb w10, [x0, x12]
    add w9, w9, #1
    and w9, w9, #0xffff
    ldrb w10, [x4, x9]
    strb w10, [x0, x1]
    next

_STA2:
    ldrb w9, [x0, x1]
    pop
    ldrb w10, [x0, x1]
    pop
    orr w12, w9, w10, lsl #8
    ldrb w10, [x0, x1]
    pop
    ldrb w11, [x0, x1]
    pop
    strb w11, [x4, x12]
    add x12, x12, #1
    and x12, x12, #0xffff
    strb w10, [x4, x12]
    next

_DEI2:
    precall
    CALL dei_2_entry
    postcall
    next

_DEO2:
    precall
    CALL deo_2_entry // todo check return value for early exit?
    postcall
    next

.macro binary_op2 op
    ldrb w10, [x0, x1]
    pop
    ldrb w11, [x0, x1]
    pop
    orr w12, w10, w11, lsl #8

    ldrb w10, [x0, x1]
    pop
    ldrb w11, [x0, x1]
    orr w11, w10, w11, lsl #8

    \op w11, w11, w12
    lsr w12, w11, 8
    strb w12, [x0, x1]
    push w11
    next
.endm

_ADD2:
    binary_op2 add

_SUB2:
    binary_op2 sub

_MUL2:
    binary_op2 mul

_DIV2:
    binary_op2 udiv

_AND2:
    binary_op2 and

_ORA2:
    binary_op2 orr

_EOR2:
    binary_op2 eor

_SFT2:
    ldrb w10, [x0, x1]
    pop
    ldrb w11, [x0, x1]
    pop
    ldrb w12, [x0, x1]
    orr w11, w11, w12, lsl #8

    lsr w12, w10, 4
    and w10, w10, #0xf
    lsr w11, w11, w10
    lsl w11, w11, w12
    lsr w12, w11, 8
    strb w12, [x0, x1]
    push w11
    next

_JMI:
    ldrb w9, [x4, x5]
    add x5, x5, #1
    and x5, x5, #0xffff
    ldrb w10, [x4, x5]
    add x5, x5, #1
    and x5, x5, #0xffff
    orr w12, w10, w9, lsl #8 // build the jump offset
    add x5, x5, x12 // do the jump
    and x5, x5, 0xffff
    next

_INCr:
    ldrb w9, [x2, x3]
    add w9, w9, #1
    strb w9, [x2, x3]
    next

_POPr:
    sub x3, x3, #1
    and x3, x3, #0xff
    next

_NIPr:
    ldrb w9, [x2, x3]   // get the top byte
    rpop
    strb w9, [x2, x3]   // overwrite the previous byte
    next

_SWPr:
    ldrb w10, [x2, x3]  // get the top byte
    rpeek w11, x9, 1    // get the second-from-top byte
    strb w10, [x2, x9]  // do the swap!
    strb w11, [x2, x3]
    next

_ROTr:
    ldrb w10, [x2, x3]
    rpeek w12, x11, 1
    rpeek w14, x13, 2

    strb w14, [x2, x3]
    strb w12, [x2, x13]
    strb w10, [x2, x11]
    next

_DUPr:
    ldrb w10, [x2, x3]   // get the top byte
    rpush w10
    next

_OVRr:
    rpeek w10, x9, 1
    rpush w10
    next

.macro compare_opr op
    ldrb w10, [x2, x3]
    rpop
    ldrb w11, [x2, x3]
    cmp w11, w10
    cset w10, \op
    strb w10, [x2, x3]
    next
.endm

_EQUr:
    compare_opr eq

_NEQr:
    compare_opr ne

_GTHr:
    compare_opr hi

_LTHr:
    compare_opr lo

_JMPr:
    ldrsb x9, [x2, x3]
    rpop
    add x5, x5, x9
    and x5, x5, 0xffff
    next

_JCNr:
    ldrsb w9, [x2, x3]
    rpop
    ldrb w10, [x2, x3]
    rpop
    cmp w10, #0
    csel w10, wzr, w9, eq // choose the jump or not
    add x5, x5, x10 // jump or not
    and x5, x5, 0xffff
    next

_JSRr:
    ldrsb w9, [x2, x3]
    rpop
    lsr w10, w5, 8
    push w10
    push w5
    add x5, x5, x9
    and x5, x5, 0xffff
    next

_STHr:
    ldrb w9, [x2, x3]
    rpop
    push w9
    next

_LDZr:
    ldrb w9, [x2, x3]
    rpop
    ldrb w9, [x4, x9]
    rpush w9
    next

_STZr:
    ldrb w9, [x2, x3]
    rpop
    ldrb w10, [x2, x3]
    rpop
    strb w10, [x4, x9]
    next

_LDRr:
    ldrsb w9, [x2, x3]
    add x9, x5, x9
    and x9, x9, #0xffff
    ldrb w9, [x4, x9] // read from RAM
    strb w9, [x2, x3] // push to stack
    next

_STRr:
    ldrsb w9, [x2, x3]
    rpop
    ldrb w10, [x2, x3]
    rpop
    add x9, x5, x9
    and x9, x9, #0xffff
    strb w10, [x4, x9] // write to RAM
    next

_LDAr:
    ldrb w9, [x2, x3]
    rpop
    ldrb w10, [x2, x3]
    orr w12, w9, w10, lsl #8
    ldrb w12, [x4, x12]
    strb w12, [x2, x3]
    next

_STAr:
    ldrb w9, [x2, x3]
    rpop
    ldrb w10, [x2, x3]
    rpop
    orr w12, w9, w10, lsl #8
    ldrb w10, [x2, x3]
    rpop
    strb w10, [x4, x12]
    next

_DEIr:
    precall
    CALL dei_r_entry
    postcall
    next

_DEOr:
    precall
    CALL deo_r_entry // todo check return value for early exit?
    postcall
    next

.macro binary_opr op
    ldrb w10, [x2, x3]
    rpop
    ldrb w11, [x2, x3]
    \op w10, w11, w10
    strb w10, [x2, x3]
    next
.endm

_ADDr:
    binary_opr add

_SUBr:
    binary_opr sub

_MULr:
    binary_opr mul

_DIVr:
    binary_opr udiv

_ANDr:
    binary_opr and

_ORAr:
    binary_opr orr

_EORr:
    binary_opr eor

_SFTr:
    ldrb w10, [x2, x3]
    rpop
    ldrb w11, [x2, x3]
    lsr w12, w10, 4
    and w10, w10, #0xf
    lsr w11, w11, w10
    lsl w11, w11, w12
    strb w11, [x2, x3]
    next

_JSI:
    ldrb w9, [x4, x5]
    add x5, x5, #1
    and x5, x5, #0xffff
    ldrb w10, [x4, x5]
    add x5, x5, #1
    and x5, x5, #0xffff

    orr w12, w10, w9, lsl #8 // build the jump offset

    // Store PC + 2 to the return stack
    lsr w9, w5, 8
    rpush w9
    rpush w5

    add x5, x5, x12 // do the jump
    and x5, x5, 0xffff
    next

_INC2r:
    ldrb w10, [x2, x3]
    rpeek w11, x9, 1
    orr w12, w10, w11, lsl #8
    add w12, w12, #1
    and w12, w12, #0xffff
    strb w12, [x2, x3]
    lsr w12, w12, 8
    strb w12, [x2, x9]
    next

_POP2r:
    sub x3, x3, #2
    and x3, x3, #0xff
    next

_NIP2r:
    ldrb w9, [x2, x3]
    rpop
    ldrb w10, [x2, x3]
    rpop
    strb w9, [x2, x3]
    sub x11, x1, #1
    and x11, x11, #0xff
    strb w10, [x2, x31]
    next

_SWP2r:
    ldrb w11, [x2, x3]  // get the top byte
    rpeek w12, x9, 2    // get the second-from-top byte
    strb w11, [x2, x9]  // do the swap!
    strb w12, [x2, x3]

    rpeek w11, x9, 1
    rpeek w12, x10, 2

    strb w11, [x2, x10]
    strb w12, [x2, x9]

    next

_ROT2r:
    ldrb w10, [x2, x3]
    rpeek w12, x11, 2
    rpeek w14, x13, 4

    strb w14, [x2, x3]
    strb w12, [x2, x13]
    strb w10, [x2, x11]

    rpeek w10, x15, 1
    rpeek w12, x11, 3
    rpeek w14, x13, 5

    ldrb w14, [x2, x13]
    strb w14, [x2, x15]
    strb w12, [x2, x13]
    strb w10, [x2, x11]

    next

_DUP2r:
    ldrb w11, [x2, x3]
    sub w9, w3, #1
    and w9, w9, #0xff
    ldrb w10, [x2, x9]
    rpush w10
    rpush w11
    next

_OVR2r:
    rpeek w10, x9, 2
    rpeek w11, x9, 3
    rpush w11
    rpush w10
    next

.macro compare_op2r op
    ldrb w10, [x2, x3]
    rpop
    ldrb w11, [x2, x3]
    rpop
    orr w10, w10, w11, lsl #8
    ldrb w11, [x2, x3]
    rpop
    ldrb w12, [x2, x3]
    orr w11, w11, w12, lsl #8
    cmp w11, w10
    cset w10, \op
    strb w10, [x2, x3]
    next
.endm

_EQU2r:
    compare_op2r eq

_NEQ2r:
    compare_op2r ne

_GTH2r:
    compare_op2r hi

_LTH2r:
    compare_op2r lo

_JMP2r:
    ldrb w9, [x2, x3]
    rpop
    ldrb w10, [x2, x3]
    rpop
    orr w5, w9, w10, lsl #8 // update program counter
    next

_JCN2r:
    ldrb w9, [x2, x3]
    rpop
    ldrb w10, [x2, x3]
    rpop
    ldrb w11, [x2, x3]
    rpop
    orr w9, w9, w10, lsl #8 // update program counter
    cmp w11, #0
    csel w5, w5, w9, eq // choose the jump or not
    next

_JSR2r:
    ldrb w9, [x2, x3]
    rpop
    ldrb w10, [x2, x3]
    rpop
    lsr w11, w5, 8
    push w11
    push w5
    orr w5, w9, w10, lsl #8 // update program counter
    next

_STH2r:
    ldrb w9, [x2, x3]
    rpop
    ldrb w10, [x2, x3]
    rpop
    push w10
    push w9
    next

_LDZ2r:
    ldrb w9, [x2, x3]
    rpop
    ldrb w10, [x4, x9]
    rpush w10
    add w9, w9, #1
    and w9, w9, #0xFFFF
    ldrb w10, [x4, x9]
    rpush w10
    next

_STZ2r:
    ldrb w9, [x2, x3]
    rpop
    ldrb w10, [x2, x3]
    rpop
    ldrb w11, [x2, x3]
    rpop
    strb w11, [x4, x9]
    add w9, w9, #1
    and w9, w9, #0xFFFF
    strb w10, [x4, x9]
    next

_LDR2r:
    ldrsb w9, [x2, x3]
    add x9, x5, x9
    and x9, x9, #0xffff
    ldrb w10, [x4, x9] // read from RAM
    strb w10, [x2, x3] // push to stack
    add x9, x9, #1
    and x9, x9, #0xffff
    ldrb w10, [x4, x9] // read from RAM
    rpush w10
    next

_STR2r:
    ldrsb w9, [x2, x3]
    rpop
    ldrsb w10, [x2, x3]
    rpop
    ldrsb w11, [x2, x3]
    rpop
    add x9, x5, x9
    and x9, x9, #0xffff
    strb w11, [x4, x9] // write to RAM
    add x9, x9, #1
    and x9, x9, #0xffff
    strb w10, [x4, x9] // write to RAM
    next

_LDA2r:
    ldrb w9, [x2, x3]
    rpop
    ldrb w10, [x2, x3]
    orr w12, w9, w10, lsl #8
    ldrb w10, [x4, x12]
    strb w10, [x2, x3]
    add w12, w12, #1
    and w12, w12, #0xffff
    ldrb w10, [x4, x12]
    rpush w10
    next

_STA2r:
    ldrb w9, [x2, x3]
    rpop
    ldrb w10, [x2, x3]
    rpop
    orr w12, w9, w10, lsl #8
    ldrb w10, [x2, x3]
    rpop
    ldrb w11, [x2, x3]
    rpop
    strb w11, [x4, x12]
    add x12, x12, #1
    and x12, x12, #0xffff
    strb w10, [x4, x12]
    next

_DEI2r:
    precall
    CALL dei_2r_entry
    postcall
    next

_DEO2r:
    precall
    CALL deo_2r_entry // todo check return value for early exit?
    postcall
    next

.macro binary_op2r op
    ldrb w10, [x2, x3]
    rpop
    ldrb w11, [x2, x3]
    rpop
    orr w12, w10, w11, lsl #8

    ldrb w10, [x2, x3]
    rpop
    ldrb w11, [x2, x3]
    orr w11, w10, w11, lsl #8

    \op w11, w11, w12
    lsr w12, w11, 8
    strb w12, [x2, x3]
    rpush w11
    next
.endm

_ADD2r:
    binary_op2r add

_SUB2r:
    binary_op2r sub

_MUL2r:
    binary_op2r mul

_DIV2r:
    binary_op2r udiv

_AND2r:
    binary_op2r and

_ORA2r:
    binary_op2r orr

_EOR2r:
    binary_op2r eor

_SFT2r:
    ldrb w10, [x2, x3]
    rpop
    ldrb w11, [x2, x3]
    rpop
    ldrb w12, [x2, x3]
    orr w11, w11, w12, lsl #8

    lsr w12, w10, 4
    and w10, w10, #0xf
    lsr w11, w11, w10
    lsl w11, w11, w12
    lsr w12, w11, 8
    strb w12, [x2, x3]
    rpush w11
    next

_LIT:
    ldrb w9, [x4, x5]
    add x5, x5, #1
    and x5, x5, #0xffff
    push w9
    next

_INCk:
    ldrb w9, [x0, x1]
    add w9, w9, #1
    push w9
    next

_POPk:
    next

_NIPk:
    ldrb w9, [x0, x1]
    push w9
    next

_SWPk:
    ldrb w10, [x0, x1]   // get the top byte
    peek w11, x9, 1      // get the second-from-top byte
    push w10
    push w11
    next

_ROTk:
    ldrb w13, [x0, x1]
    peek w10, x11, 1
    push w10
    push w13
    peek w10, x11, 4
    push w10
    next

_DUPk:
    ldrb w11, [x0, x1]
    push w11
    push w11
    next

_OVRk:
    peek w10, x9, 1 // get the second-from-top
    ldrb w11, [x0, x1]
    push w10
    push w11
    push w10
    next

.macro compare_opk op
    ldrb w10, [x0, x1]
    peek w11, x9, 1
    cmp w11, w10
    cset w10, \op
    push w10
    next
.endm

_EQUk:
    compare_opk eq

_NEQk:
    compare_opk ne

_GTHk:
    compare_opk hi

_LTHk:
    compare_opk lo

_JMPk:
    ldrsb x9, [x0, x1]
    add x5, x5, x9
    and x5, x5, 0xffff
    next

_JCNk:
    ldrsb w9, [x0, x1]
    peek w10, x10, 1
    cmp w10, #0
    csel w10, wzr, w9, eq // choose the jump or not
    add x5, x5, x10 // jump or not
    and x5, x5, 0xffff
    next

_JSRk:
    ldrsb w9, [x0, x1]
    lsr w10, w5, 8
    rpush w10
    rpush w5
    add x5, x5, x9
    and x5, x5, 0xffff
    next

_STHk:
    ldrb w9, [x0, x1]
    rpush w9
    next

_LDZk:
    ldrb w9, [x0, x1]
    ldrb w9, [x4, x9]
    push w9
    next

_STZk:
    ldrb w9, [x0, x1]
    peek w10, x10, 1
    strb w10, [x4, x9]
    next

_LDRk:
    ldrsb w9, [x0, x1]
    add x9, x5, x9
    and x9, x9, #0xffff
    ldrb w9, [x4, x9] // read from RAM
    push w9
    next

_STRk:
    ldrsb w9, [x0, x1]
    peek w10, x10, 1
    add x9, x5, x9
    and x9, x9, #0xffff
    strb w10, [x4, x9] // write to RAM
    next

_LDAk:
    ldrb w9, [x0, x1]
    sub w10, w1, #1
    and w10, w10, #0xff
    ldrb w10, [x0, x10]
    orr w10, w9, w10, lsl #8    // build address
    ldrb w10, [x4, x10]         // load byte from RAM
    push w10
    next

_STAk:
    ldrb w9, [x0, x1]
    peek w10, x10, 1
    orr w12, w9, w10, lsl #8
    peek w10, x10, 2
    strb w10, [x4, x12]
    next

_DEIk:
    precall
    CALL dei_k_entry
    postcall
    next

_DEOk:
    precall
    CALL deo_k_entry // todo check return value for early exit?
    postcall
    next

.macro binary_opk op
    peek w11, x9, 1
    ldrb w10, [x0, x1]
    \op w10, w11, w10
    push w10
    next
.endm

_ADDk:
    binary_opk add

_SUBk:
    binary_opk sub

_MULk:
    binary_opk mul

_DIVk:
    binary_opk udiv

_ANDk:
    binary_opk and

_ORAk:
    binary_opk orr

_EORk:
    binary_opk eor

_SFTk:
    ldrb w10, [x0, x1]
    peek w11, x9, 1
    lsr w12, w10, 4
    and w10, w10, #0xf
    lsr w11, w11, w10
    lsl w11, w11, w12
    push w11
    next

_LIT2:
    ldrb w9, [x4, x5]
    add x5, x5, #1
    and x5, x5, #0xffff
    push w9
    ldrb w9, [x4, x5]
    add x5, x5, #1
    and x5, x5, #0xffff
    push w9
    next

_INC2k:
    ldrb w10, [x0, x1]
    peek w11, x9, 1
    orr w12, w10, w11, lsl #8
    add w12, w12, #1
    and w12, w12, #0xffff

    add x10, x1, #1
    and x10, x10, #0xff
    add x1, x10, #1
    and x1, x1, #0xff
    strb w12, [x0, x1]
    lsr w12, w12, 8
    strb w12, [x0, x10]
    next

_POP2k:
    next

_NIP2k:
    ldrb w9, [x0, x1]
    peek w10, x11, 1
    push w10
    push w9
    next

_SWP2k:
    peek w11, x9, 1
    push w11
    peek w11, x9, 1
    push w11
    peek w11, x9, 5
    push w11
    peek w11, x9, 5
    push w11
    next

_ROT2k:
    peek w11, x9, 3
    push w11
    peek w11, x9, 3
    push w11
    peek w11, x9, 3
    push w11
    peek w11, x9, 3
    push w11
    peek w11, x9, 9
    push w11
    peek w11, x9, 9
    push w11
    next

_DUP2k:
    ldrb w11, [x0, x1]
    sub w9, w1, #1
    and w9, w9, #0xff
    ldrb w10, [x0, x9]

    push w10
    push w11
    push w10
    push w11
    next

_OVR2k:
    ldrb w10, [x0, x1]
    peek w11, x9, 1
    peek w12, x9, 2
    peek w13, x9, 3
    push w13
    push w12
    push w11
    push w10
    push w13
    push w12
    next

.macro compare_op2k op
    ldrb w10, [x0, x1]
    peek w11, x9, 1
    orr w10, w10, w11, lsl #8

    peek w11, x9, 2
    peek w12, x9, 3
    orr w11, w11, w12, lsl #8

    cmp w11, w10
    cset w10, \op
    push w10
    next
.endm

_EQU2k:
    compare_op2k eq

_NEQ2k:
    compare_op2k ne

_GTH2k:
    compare_op2k hi

_LTH2k:
    compare_op2k lo

_JMP2k:
    ldrb w9, [x0, x1]
    peek w10, x10, 1
    orr w5, w9, w10, lsl #8 // update program counter
    next

_JCN2k:
    ldrb w9, [x0, x1]
    peek w10, x12, 1
    peek w11, x12, 2

    orr w9, w9, w10, lsl #8 // update program counter
    cmp w11, #0
    csel w5, w5, w9, eq // choose the jump or not
    next

_JSR2k:
    ldrb w9, [x0, x1]
    peek w10, x10, 1

    lsr w11, w5, 8
    rpush w11
    rpush w5

    orr w5, w9, w10, lsl #8 // update program counter
    next

_STH2k:
    ldrb w9, [x0, x1]
    peek w10, x10, 1
    rpush w10
    rpush w9
    next

_LDZ2k:
    ldrb w9, [x0, x1]
    ldrb w10, [x4, x9]
    push w10
    add w9, w9, #1
    and w9, w9, #0xFFFF
    ldrb w10, [x4, x9]
    push w10
    next

_STZ2k:
    ldrb w9, [x0, x1]
    peek w10, x10, 1
    peek w11, x11, 2

    strb w11, [x4, x9]
    add w9, w9, #1
    and w9, w9, #0xFFFF
    strb w10, [x4, x9]
    next

_LDR2k:
    ldrsb w9, [x0, x1]
    add x9, x5, x9
    and x9, x9, #0xffff
    ldrb w10, [x4, x9] // read from RAM
    push w10
    add x9, x9, #1
    and x9, x9, #0xffff
    ldrb w10, [x4, x9] // read from RAM
    push w10
    next

_STR2k:
    ldrsb w9, [x0, x1]
    peek w10, x10, 1
    peek w11, x11, 2

    add x9, x5, x9
    and x9, x9, #0xffff
    strb w11, [x4, x9] // write to RAM
    add x9, x9, #1
    and x9, x9, #0xffff
    strb w10, [x4, x9] // write to RAM
    next

_LDA2k:
    ldrb w9, [x0, x1]
    peek w10, x10, 1
    orr w12, w9, w10, lsl #8
    ldrb w10, [x4, x12]
    push w10
    add w12, w12, #1
    and w12, w12, #0xffff
    ldrb w10, [x4, x12]
    push w10
    next

_STA2k:
    ldrb w9, [x0, x1]
    peek w10, x10, 1
    orr w12, w9, w10, lsl #8

    peek w10, x10, 2
    peek w11, x11, 3

    strb w11, [x4, x12]
    add x12, x12, #1
    and x12, x12, #0xffff
    strb w10, [x4, x12]
    next

_DEI2k:
    precall
    CALL dei_2k_entry
    postcall
    next

_DEO2k:
    precall
    CALL deo_2k_entry // todo check return value for early exit?
    postcall
    next

.macro binary_op2k op
    ldrb w10, [x0, x1]
    peek w11, x9, 1
    orr w12, w10, w11, lsl #8

    peek w10, x9, 2
    peek w11, x9, 3
    orr w11, w10, w11, lsl #8

    \op w11, w11, w12
    lsr w12, w11, 8
    push w12
    push w11
    next
.endm

_ADD2k:
    binary_op2k add

_SUB2k:
    binary_op2k sub

_MUL2k:
    binary_op2k mul

_DIV2k:
    binary_op2k udiv

_AND2k:
    binary_op2k and

_ORA2k:
    binary_op2k orr

_EOR2k:
    binary_op2k eor

_SFT2k:
    ldrb w10, [x0, x1]
    peek w11, x9, 1
    peek w12, x9, 2
    orr w11, w11, w12, lsl #8

    lsr w12, w10, 4
    and w10, w10, #0xf
    lsr w11, w11, w10
    lsl w11, w11, w12
    lsr w12, w11, 8
    push w12
    push w11
    next

_LITr:
    ldrb w9, [x4, x5]
    add x5, x5, #1
    and x5, x5, #0xffff
    rpush w9
    next

_INCkr:
    ldrb w9, [x2, x3]
    add w9, w9, #1
    rpush w9
    next

_POPkr:
    next

_NIPkr:
    ldrb w9, [x2, x3]
    rpush w9
    next

_SWPkr:
    ldrb w10, [x2, x3]   // get the top byte
    rpeek w11, x9, 1
    rpush w10
    rpush w11
    next

_ROTkr:
    ldrb w13, [x2, x3]
    rpeek w10, x11, 1
    rpush w10
    rpush w13
    rpeek w10, x11, 4
    rpush w10
    next

_DUPkr:
    ldrb w11, [x2, x3]
    rpush w11
    rpush w11
    next

_OVRkr:
    rpeek w10, x9, 1
    ldrb w11, [x2, x3]
    rpush w10
    rpush w11
    rpush w10
    next

.macro compare_opkr op
    ldrb w10, [x2, x3]
    rpeek w11, x9, 1
    cmp w11, w10
    cset w10, \op
    rpush w10
    next
.endm

_EQUkr:
    compare_opkr eq

_NEQkr:
    compare_opkr ne

_GTHkr:
    compare_opkr hi

_LTHkr:
    compare_opkr lo

_JMPkr:
    ldrsb x9, [x2, x3]
    add x5, x5, x9
    and x5, x5, 0xffff
    next

_JCNkr:
    ldrsb w9, [x2, x3]
    rpeek w10, x10, 1
    cmp w10, #0
    csel w10, wzr, w9, eq // choose the jump or not
    add x5, x5, x10 // jump or not
    and x5, x5, 0xffff
    next

_JSRkr:
    ldrsb w9, [x2, x3]
    lsr w10, w5, 8
    push w10
    push w5
    add x5, x5, x9
    and x5, x5, 0xffff
    next

_STHkr:
    ldrb w9, [x2, x3]
    push w9
    next

_LDZkr:
    ldrb w9, [x2, x3]
    ldrb w9, [x4, x9]
    rpush w9
    next

_STZkr:
    ldrb w9, [x2, x3]
    rpeek w10, x10, 1
    strb w10, [x4, x9]
    next

_LDRkr:
    ldrsb w9, [x2, x3]
    add x9, x5, x9
    and x9, x9, #0xffff
    ldrb w9, [x4, x9] // read from RAM
    rpush w9
    next

_STRkr:
    ldrsb w9, [x2, x3]
    rpeek w10, x10, 1
    add x9, x5, x9
    and x9, x9, #0xffff
    strb w10, [x4, x9] // write to RAM
    next

_LDAkr:
    ldrb w9, [x2, x3]
    sub w10, w3, #1
    and w10, w10, #0xff
    ldrb w10, [x2, x10]
    orr w10, w9, w10, lsl #8    // build address
    ldrb w10, [x4, x10]         // load byte from RAM
    rpush w10
    next

_STAkr:
    ldrb w9, [x2, x3]
    rpeek w10, x10, 1
    orr w12, w9, w10, lsl #8
    rpeek w10, x10, 2
    strb w10, [x4, x12]
    next

_DEIkr:
    precall
    CALL dei_kr_entry
    postcall
    next

_DEOkr:
    precall
    CALL deo_kr_entry // todo check return value for early exit?
    postcall
    next

.macro binary_opkr op
    ldrb w10, [x2, x3]
    rpeek w11, x9, 1
    \op w10, w11, w10
    rpush w10
    next
.endm

_ADDkr:
    binary_opkr add

_SUBkr:
    binary_opkr sub

_MULkr:
    binary_opkr mul

_DIVkr:
    binary_opkr udiv

_ANDkr:
    binary_opkr and

_ORAkr:
    binary_opkr orr

_EORkr:
    binary_opkr eor

_SFTkr:
    ldrb w10, [x2, x3]
    rpeek w11, x9, 1
    lsr w12, w10, 4
    and w10, w10, #0xf
    lsr w11, w11, w10
    lsl w11, w11, w12
    rpush w11
    next

_LIT2r:
    ldrb w9, [x4, x5]
    add x5, x5, #1
    and x5, x5, #0xffff
    rpush w9
    ldrb w9, [x4, x5]
    add x5, x5, #1
    and x5, x5, #0xffff
    rpush w9
    next

_INC2kr:
    ldrb w10, [x2, x3]
    rpeek w11, x9, 1
    orr w12, w10, w11, lsl #8
    add w12, w12, #1
    and w12, w12, #0xffff

    add x10, x3, #1
    and x10, x10, #0xff
    add x3, x10, #1
    and x3, x3, #0xff
    strb w12, [x2, x3]
    lsr w12, w12, 8
    strb w12, [x2, x10]
    next

_POP2kr:
    next

_NIP2kr:
    ldrb w9, [x2, x3]
    rpeek w10, x11, 1
    rpush w10
    rpush w9
    next

_SWP2kr:
    rpeek w11, x9, 1
    rpush w11
    rpeek w11, x9, 1
    rpush w11
    rpeek w11, x9, 5
    rpush w11
    rpeek w11, x9, 5
    rpush w11
    next

_ROT2kr:
    rpeek w11, x9, 3
    rpush w11
    rpeek w11, x9, 3
    rpush w11
    rpeek w11, x9, 3
    rpush w11
    rpeek w11, x9, 3
    rpush w11
    rpeek w11, x9, 9
    rpush w11
    rpeek w11, x9, 9
    rpush w11
    next

_DUP2kr:
    ldrb w11, [x2, x3]
    sub w9, w3, #1
    and w9, w9, #0xff
    ldrb w10, [x2, x9]

    rpush w10
    rpush w11
    rpush w10
    rpush w11
    next

_OVR2kr:
    ldrb w10, [x2, x3]
    rpeek w11, x9, 1
    rpeek w12, x9, 2
    rpeek w13, x9, 3
    rpush w13
    rpush w12
    rpush w11
    rpush w10
    rpush w13
    rpush w12
    next

.macro compare_op2kr op
    ldrb w10, [x2, x3]
    rpeek w11, x9, 1
    orr w10, w10, w11, lsl #8
    rpeek w11, x9, 2
    rpeek w12, x9, 3
    orr w11, w11, w12, lsl #8
    cmp w11, w10
    cset w10, \op
    rpush w10
    next
.endm

_EQU2kr:
    compare_op2kr eq

_NEQ2kr:
    compare_op2kr ne

_GTH2kr:
    compare_op2kr hi

_LTH2kr:
    compare_op2kr lo

_JMP2kr:
    ldrb w9, [x2, x3]
    rpeek w10, x10, 1
    orr w5, w9, w10, lsl #8 // update program counter
    next

_JCN2kr:
    ldrb w9, [x2, x3]
    rpeek w10, x12, 1
    rpeek w11, x12, 2
    orr w9, w9, w10, lsl #8 // update program counter
    cmp w11, #0
    csel w5, w5, w9, eq // choose the jump or not
    next

_JSR2kr:
    ldrb w9, [x2, x3]
    rpeek w10, x10, 1
    lsr w11, w5, 8
    push w11
    push w5
    orr w5, w9, w10, lsl #8 // update program counter
    next

_STH2kr:
    ldrb w9, [x2, x3]
    rpeek w10, x11, 1
    push w10
    push w9
    next

_LDZ2kr:
    ldrb w9, [x2, x3]
    ldrb w10, [x4, x9]
    rpush w10
    add w9, w9, #1
    and w9, w9, #0xFFFF
    ldrb w10, [x4, x9]
    rpush w10
    next

_STZ2kr:
    ldrb w9, [x2, x3]
    rpeek w10, x10, 1
    rpeek w11, x11, 2

    strb w11, [x4, x9]
    add w9, w9, #1
    and w9, w9, #0xFFFF
    strb w10, [x4, x9]
    next

_LDR2kr:
    ldrsb w9, [x2, x3]
    add x9, x5, x9
    and x9, x9, #0xffff
    ldrb w10, [x4, x9] // read from RAM
    rpush w10
    add x9, x9, #1
    and x9, x9, #0xffff
    ldrb w10, [x4, x9] // read from RAM
    rpush w10
    next

_STR2kr:
    ldrsb w9, [x2, x3]
    rpeek w10, x10, 1
    rpeek w11, x11, 2

    add x9, x5, x9
    and x9, x9, #0xffff
    strb w11, [x4, x9] // write to RAM
    add x9, x9, #1
    and x9, x9, #0xffff
    strb w10, [x4, x9] // write to RAM
    next

_LDA2kr:
    ldrb w9, [x2, x3]
    rpeek w10, x10, 1
    orr w12, w9, w10, lsl #8
    ldrb w10, [x4, x12]
    rpush w10
    add w12, w12, #1
    and w12, w12, #0xffff
    ldrb w10, [x4, x12]
    rpush w10
    next

_STA2kr:
    ldrb w9, [x2, x3]
    rpeek w10, x10, 1
    orr w12, w9, w10, lsl #8

    rpeek w10, x10, 2
    rpeek w11, x11, 3

    strb w11, [x4, x12]
    add x12, x12, #1
    and x12, x12, #0xffff
    strb w10, [x4, x12]
    next

_DEI2kr:
    precall
    CALL dei_2kr_entry
    postcall
    next

_DEO2kr:
    precall
    CALL deo_2kr_entry // todo check return value for early exit?
    postcall
    next

.macro binary_op2kr op
    ldrb w10, [x2, x3]
    rpeek w11, x9, 1
    orr w12, w10, w11, lsl #8

    rpeek w10, x9, 2
    rpeek w11, x9, 3
    orr w11, w10, w11, lsl #8

    \op w11, w11, w12
    lsr w12, w11, 8
    rpush w12
    rpush w11
    next
.endm

_ADD2kr:
    binary_op2kr add

_SUB2kr:
    binary_op2kr sub

_MUL2kr:
    binary_op2kr mul

_DIV2kr:
    binary_op2kr udiv

_AND2kr:
    binary_op2kr and

_ORA2kr:
    binary_op2kr orr

_EOR2kr:
    binary_op2kr eor

_SFT2kr:
    ldrb w10, [x2, x3]
    rpeek w11, x9, 1
    rpeek w12, x9, 2
    orr w11, w11, w12, lsl #8

    lsr w12, w10, 4
    and w10, w10, #0xf
    lsr w11, w11, w10
    lsl w11, w11, w12
    lsr w12, w11, 8
    rpush w12
    rpush w11
    next

.data
.balign 4096
.global JUMP_TABLE
JUMP_TABLE:
    .quad _BRK
    .quad _INC
    .quad _POP
    .quad _NIP
    .quad _SWP
    .quad _ROT
    .quad _DUP
    .quad _OVR
    .quad _EQU
    .quad _NEQ
    .quad _GTH
    .quad _LTH
    .quad _JMP
    .quad _JCN
    .quad _JSR
    .quad _STH
    .quad _LDZ
    .quad _STZ
    .quad _LDR
    .quad _STR
    .quad _LDA
    .quad _STA
    .quad _DEI
    .quad _DEO
    .quad _ADD
    .quad _SUB
    .quad _MUL
    .quad _DIV
    .quad _AND
    .quad _ORA
    .quad _EOR
    .quad _SFT
    .quad _JCI
    .quad _INC2
    .quad _POP2
    .quad _NIP2
    .quad _SWP2
    .quad _ROT2
    .quad _DUP2
    .quad _OVR2
    .quad _EQU2
    .quad _NEQ2
    .quad _GTH2
    .quad _LTH2
    .quad _JMP2
    .quad _JCN2
    .quad _JSR2
    .quad _STH2
    .quad _LDZ2
    .quad _STZ2
    .quad _LDR2
    .quad _STR2
    .quad _LDA2
    .quad _STA2
    .quad _DEI2
    .quad _DEO2
    .quad _ADD2
    .quad _SUB2
    .quad _MUL2
    .quad _DIV2
    .quad _AND2
    .quad _ORA2
    .quad _EOR2
    .quad _SFT2
    .quad _JMI
    .quad _INCr
    .quad _POPr
    .quad _NIPr
    .quad _SWPr
    .quad _ROTr
    .quad _DUPr
    .quad _OVRr
    .quad _EQUr
    .quad _NEQr
    .quad _GTHr
    .quad _LTHr
    .quad _JMPr
    .quad _JCNr
    .quad _JSRr
    .quad _STHr
    .quad _LDZr
    .quad _STZr
    .quad _LDRr
    .quad _STRr
    .quad _LDAr
    .quad _STAr
    .quad _DEIr
    .quad _DEOr
    .quad _ADDr
    .quad _SUBr
    .quad _MULr
    .quad _DIVr
    .quad _ANDr
    .quad _ORAr
    .quad _EORr
    .quad _SFTr
    .quad _JSI
    .quad _INC2r
    .quad _POP2r
    .quad _NIP2r
    .quad _SWP2r
    .quad _ROT2r
    .quad _DUP2r
    .quad _OVR2r
    .quad _EQU2r
    .quad _NEQ2r
    .quad _GTH2r
    .quad _LTH2r
    .quad _JMP2r
    .quad _JCN2r
    .quad _JSR2r
    .quad _STH2r
    .quad _LDZ2r
    .quad _STZ2r
    .quad _LDR2r
    .quad _STR2r
    .quad _LDA2r
    .quad _STA2r
    .quad _DEI2r
    .quad _DEO2r
    .quad _ADD2r
    .quad _SUB2r
    .quad _MUL2r
    .quad _DIV2r
    .quad _AND2r
    .quad _ORA2r
    .quad _EOR2r
    .quad _SFT2r
    .quad _LIT
    .quad _INCk
    .quad _POPk
    .quad _NIPk
    .quad _SWPk
    .quad _ROTk
    .quad _DUPk
    .quad _OVRk
    .quad _EQUk
    .quad _NEQk
    .quad _GTHk
    .quad _LTHk
    .quad _JMPk
    .quad _JCNk
    .quad _JSRk
    .quad _STHk
    .quad _LDZk
    .quad _STZk
    .quad _LDRk
    .quad _STRk
    .quad _LDAk
    .quad _STAk
    .quad _DEIk
    .quad _DEOk
    .quad _ADDk
    .quad _SUBk
    .quad _MULk
    .quad _DIVk
    .quad _ANDk
    .quad _ORAk
    .quad _EORk
    .quad _SFTk
    .quad _LIT2
    .quad _INC2k
    .quad _POP2k
    .quad _NIP2k
    .quad _SWP2k
    .quad _ROT2k
    .quad _DUP2k
    .quad _OVR2k
    .quad _EQU2k
    .quad _NEQ2k
    .quad _GTH2k
    .quad _LTH2k
    .quad _JMP2k
    .quad _JCN2k
    .quad _JSR2k
    .quad _STH2k
    .quad _LDZ2k
    .quad _STZ2k
    .quad _LDR2k
    .quad _STR2k
    .quad _LDA2k
    .quad _STA2k
    .quad _DEI2k
    .quad _DEO2k
    .quad _ADD2k
    .quad _SUB2k
    .quad _MUL2k
    .quad _DIV2k
    .quad _AND2k
    .quad _ORA2k
    .quad _EOR2k
    .quad _SFT2k
    .quad _LITr
    .quad _INCkr
    .quad _POPkr
    .quad _NIPkr
    .quad _SWPkr
    .quad _ROTkr
    .quad _DUPkr
    .quad _OVRkr
    .quad _EQUkr
    .quad _NEQkr
    .quad _GTHkr
    .quad _LTHkr
    .quad _JMPkr
    .quad _JCNkr
    .quad _JSRkr
    .quad _STHkr
    .quad _LDZkr
    .quad _STZkr
    .quad _LDRkr
    .quad _STRkr
    .quad _LDAkr
    .quad _STAkr
    .quad _DEIkr
    .quad _DEOkr
    .quad _ADDkr
    .quad _SUBkr
    .quad _MULkr
    .quad _DIVkr
    .quad _ANDkr
    .quad _ORAkr
    .quad _EORkr
    .quad _SFTkr
    .quad _LIT2r
    .quad _INC2kr
    .quad _POP2kr
    .quad _NIP2kr
    .quad _SWP2kr
    .quad _ROT2kr
    .quad _DUP2kr
    .quad _OVR2kr
    .quad _EQU2kr
    .quad _NEQ2kr
    .quad _GTH2kr
    .quad _LTH2kr
    .quad _JMP2kr
    .quad _JCN2kr
    .quad _JSR2kr
    .quad _STH2kr
    .quad _LDZ2kr
    .quad _STZ2kr
    .quad _LDR2kr
    .quad _STR2kr
    .quad _LDA2kr
    .quad _STA2kr
    .quad _DEI2kr
    .quad _DEO2kr
    .quad _ADD2kr
    .quad _SUB2kr
    .quad _MUL2kr
    .quad _DIV2kr
    .quad _AND2kr
    .quad _ORA2kr
    .quad _EOR2kr
    .quad _SFT2kr
