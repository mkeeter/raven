; x0 - stack pointer (&mut [u8; 256])
; x1 - stack index (u8)
; x2 - return stack pointer (&mut [u8; 256])
; x3 - return stack index (u8)
; x4 - RAM pointer (&mut [u8; 65536])
; x5 - program counter (u16), offset of the next value in RAM
; x6 - VM pointer (&mut Uxn)
; x7 - Device handle pointer (&DeviceHandle)
; x8 - Jump table pointer
; x9-15 - scratch registers
;
; We do not use any callee-saved registers (besides x29 / x30)
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
    ; We have to write our stack index pointers back into the &mut Uxn
    ldp x11, x12, [sp, 0x10] ; restore stack index pointers
    strb w1, [x11]   ; modify stack index pointer
    strb w3, [x12]   ; modify return stack index pointer

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

    ; The DEO operation may have changed stack pointers, so reload them here 
    ldp x11, x12, [sp, 0x10]
    ldrb w1, [x11]
    ldrb w3, [x12]
.endm

.macro opcode, op
    .global \op
    \op:
.endm

; x0 - *EntryHandle
; x1 - pc
; x2 - table
.global _aarch64_entry
_aarch64_entry:
    sub sp, sp, #0x200  ; make room in the stack
    stp   x29, x30, [sp, 0x0]   ; store stack and frame pointer
    mov   x29, sp

    // Unpack from EntryHandle into registers
    mov x5, x1 ; move PC (before overwriting x1)
    mov x8, x2 ; jump table (before overwriting x2)
    ldr x1, [x0, 0x8]  ; stack index pointer
    ldr x2, [x0, 0x10] ; ret data pointer
    ldr x3, [x0, 0x18] ; ret index pointer
    ldr x4, [x0, 0x20] ; RAM pointer
    ldr x6, [x0, 0x28] ; *mut Uxn
    ldr x7, [x0, 0x30] ; *mut DeviceHandle
    ldr x0, [x0, 0x00] ; stack data pointer (overwriting *EntryHandle)

    ; Convert from index pointers to index values in w1 / w3
    stp x1, x3, [sp, 0x10]      ; save stack index pointers
    ldrb w1, [x1]               ; load stack index
    ldrb w3, [x3]               ; load ret index

    ; Jump into the instruction list
    next

opcode _BRK
    ; Write index values back through index pointers
    ldp x9, x10, [sp, 0x10]     ; restore stack index pointers
    strb w1, [x9]               ; save stack index
    strb w3, [x10]              ; save ret index

    ldp   x29, x30, [sp, 0x0]   ; Restore stack and frame pointer
    add sp, sp, #0x200  ; restore stack pointer

    mov x0, x5 ; return PC from function
    ret

opcode _INC
    ldrb w9, [x0, x1]
    add w9, w9, #1
    strb w9, [x0, x1]
    next

opcode _POP
    pop
    next

opcode _NIP
    ldrb w9, [x0, x1]   ; get the top byte
    pop
    strb w9, [x0, x1]   ; overwrite the previous byte
    next

opcode _SWP
    ldrb w10, [x0, x1]   ; get the top byte
    peek w11, x9, 1      ; get the second-from-top byte
    strb w10, [x0, x9]   ; do the swap!
    strb w11, [x0, x1]
    next

opcode _ROT
    ; a b c -- b c a
    ldrb w10, [x0, x1] ; c
    peek w12, x11, 1
    peek w14, x13, 2

    strb w14, [x0, x1]
    strb w12, [x0, x13]
    strb w10, [x0, x11]

    next

opcode _DUP
    ldrb w10, [x0, x1]   ; get the top byte
    push w10
    next

opcode _OVR
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

opcode _EQU
    compare_op eq

opcode _NEQ
    compare_op ne

opcode _GTH
    compare_op hi

opcode _LTH
    compare_op lo

opcode _JMP
    ldrsb x9, [x0, x1]
    pop
    add x5, x5, x9
    and x5, x5, 0xffff
    next

opcode _JCN
    ldrsb w9, [x0, x1]
    pop
    ldrb w10, [x0, x1]
    pop
    cmp w10, #0
    csel w10, wzr, w9, eq ; choose the jump or not
    add x5, x5, x10 ; jump or not
    and x5, x5, 0xffff
    next

opcode _JSR
    ldrsb w9, [x0, x1]
    pop
    lsr w10, w5, 8
    rpush w10
    rpush w5
    add x5, x5, x9
    and x5, x5, 0xffff
    next

opcode _STH
    ldrb w9, [x0, x1]
    pop
    rpush w9
    next

opcode _LDZ
    ldrb w9, [x0, x1]
    pop
    ldrb w9, [x4, x9]
    push w9
    next

opcode _STZ
    ldrb w9, [x0, x1]
    pop
    ldrb w10, [x0, x1]
    pop
    strb w10, [x4, x9]
    next

opcode _LDR
    ldrsb w9, [x0, x1]
    add x9, x5, x9
    and x9, x9, #0xffff
    ldrb w9, [x4, x9] ; read from RAM
    strb w9, [x0, x1] ; push to stack
    next

opcode _STR
    ldrsb w9, [x0, x1]
    pop
    ldrb w10, [x0, x1]
    pop
    add x9, x5, x9
    and x9, x9, #0xffff
    strb w10, [x4, x9] ; write to RAM
    next

opcode _LDA
    ldrb w9, [x0, x1]
    pop
    ldrb w10, [x0, x1]
    orr w12, w9, w10, lsl #8
    ldrb w12, [x4, x12]
    strb w12, [x0, x1]
    next

opcode _STA
    ldrb w9, [x0, x1]
    pop
    ldrb w10, [x0, x1]
    pop
    orr w12, w9, w10, lsl #8
    ldrb w10, [x0, x1]
    pop
    strb w10, [x4, x12]
    next

opcode _DEI
    precall
    bl _dei_entry
    postcall
    next

opcode _DEO
    precall
    bl _deo_entry ; todo check return value for early exit?
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

opcode _ADD
    binary_op add

opcode _SUB
    binary_op sub

opcode _MUL
    binary_op mul

opcode _DIV
    binary_op udiv

opcode _AND
    binary_op and

opcode _ORA
    binary_op orr

opcode _EOR
    binary_op eor

opcode _SFT
    ldrb w10, [x0, x1]
    pop
    ldrb w11, [x0, x1]
    lsr w12, w10, 4
    and w10, w10, #0xf
    lsr w11, w11, w10
    lsl w11, w11, w12
    strb w11, [x0, x1]
    next

opcode _JCI
    ldrb w9, [x4, x5]
    add x5, x5, #1
    and x5, x5, #0xffff
    ldrb w10, [x4, x5]
    add x5, x5, #1
    and x5, x5, #0xffff
    orr w12, w10, w9, lsl #8 ; build the jump offset
    ldrb w10, [x0, x1] ; read conditional byte
    pop
    cmp w10, #0
    csel w10, wzr, w12, eq ; choose the jump or not
    add x5, x5, x10 ; jump or not
    and x5, x5, 0xffff
    next

opcode _INC2
    ldrb w10, [x0, x1]  ; get the top byte
    peek w11, x9, 1     ; get the second-from-top byte
    orr w12, w10, w11, lsl #8
    add w12, w12, #1
    and w12, w12, #0xffff
    strb w12, [x0, x1]
    lsr w12, w12, 8
    strb w12, [x0, x9]
    next

opcode _POP2
    sub x1, x1, #2
    and x1, x1, #0xff
    next

opcode _NIP2
    ldrb w9, [x0, x1]
    pop
    ldrb w10, [x0, x1]
    pop
    strb w9, [x0, x1]
    sub x11, x1, #1
    and x11, x11, #0xff
    strb w10, [x0, x11]
    next

opcode _SWP2
    ldrb w11, [x0, x1]   ; get the top byte
    peek w12, x9, 2       ; get the second-from-top byte
    strb w11, [x0, x9]   ; do the swap!
    strb w12, [x0, x1]

    peek w11, x9, 1
    peek w12, x10, 3
    strb w11, [x0, x10]
    strb w12, [x0, x9]

    next

opcode _ROT2
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

opcode _DUP2
    ldrb w11, [x0, x1]
    peek w10, x10, 1
    push w10
    push w11
    next

opcode _OVR2
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

opcode _EQU2
    compare_op2 eq

opcode _NEQ2
    compare_op2 ne

opcode _GTH2
    compare_op2 hi

opcode _LTH2
    compare_op2 lo

opcode _JMP2
    ldrb w9, [x0, x1]
    pop
    ldrb w10, [x0, x1]
    pop
    orr w5, w9, w10, lsl #8 ; update program counter
    next

opcode _JCN2
    ldrb w9, [x0, x1]
    pop
    ldrb w10, [x0, x1]
    pop
    ldrb w11, [x0, x1]
    pop
    orr w9, w9, w10, lsl #8 ; update program counter
    cmp w11, #0
    csel w5, w5, w9, eq ; choose the jump or not
    next

opcode _JSR2
    ldrb w9, [x0, x1]
    pop
    ldrb w10, [x0, x1]
    pop
    lsr w11, w5, 8
    rpush w11
    rpush w5
    orr w5, w9, w10, lsl #8 ; update program counter
    next

opcode _STH2
    ldrb w9, [x0, x1]
    pop
    ldrb w10, [x0, x1]
    pop
    rpush w10
    rpush w9
    next

opcode _LDZ2
    ldrb w9, [x0, x1]
    pop
    ldrb w10, [x4, x9]
    push w10
    add w9, w9, #1
    and w9, w9, #0xFFFF
    ldrb w10, [x4, x9]
    push w10
    next

opcode _STZ2
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

opcode _LDR2
    ldrsb w9, [x0, x1]
    add x9, x5, x9
    and x9, x9, #0xffff
    ldrb w10, [x4, x9] ; read from RAM
    strb w10, [x0, x1] ; push to stack
    add x9, x9, #1
    and x9, x9, #0xffff
    ldrb w10, [x4, x9] ; read from RAM
    push w10
    next

opcode _STR2
    ldrsb w9, [x0, x1]
    pop
    ldrsb w10, [x0, x1]
    pop
    ldrsb w11, [x0, x1]
    pop
    add x9, x5, x9
    and x9, x9, #0xffff
    strb w11, [x4, x9] ; write to RAM
    add x9, x9, #1
    and x9, x9, #0xffff
    strb w10, [x4, x9] ; write to RAM
    next

opcode _LDA2
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

opcode _STA2
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

opcode _DEI2
    precall
    bl _dei_2_entry
    postcall
    next

opcode _DEO2
    precall
    bl _deo_2_entry ; todo check return value for early exit?
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

opcode _ADD2
    binary_op2 add

opcode _SUB2
    binary_op2 sub

opcode _MUL2
    binary_op2 mul

opcode _DIV2
    binary_op2 udiv

opcode _AND2
    binary_op2 and

opcode _ORA2
    binary_op2 orr

opcode _EOR2
    binary_op2 eor

opcode _SFT2
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

opcode _JMI
    ldrb w9, [x4, x5]
    add x5, x5, #1
    and x5, x5, #0xffff
    ldrb w10, [x4, x5]
    add x5, x5, #1
    and x5, x5, #0xffff
    orr w12, w10, w9, lsl #8 ; build the jump offset
    add x5, x5, x12 ; do the jump
    and x5, x5, 0xffff
    next

opcode _INCr
    ldrb w9, [x2, x3]
    add w9, w9, #1
    strb w9, [x2, x3]
    next

opcode _POPr
    sub x3, x3, #1
    and x3, x3, #0xff
    next

opcode _NIPr
    ldrb w9, [x2, x3]   ; get the top byte
    rpop
    strb w9, [x2, x3]   ; overwrite the previous byte
    next

opcode _SWPr
    ldrb w10, [x2, x3]  ; get the top byte
    rpeek w11, x9, 1    ; get the second-from-top byte
    strb w10, [x2, x9]  ; do the swap!
    strb w11, [x2, x3]
    next

opcode _ROTr
    ldrb w10, [x2, x3]
    rpeek w12, x11, 1
    rpeek w14, x13, 2

    strb w14, [x2, x3]
    strb w12, [x2, x13]
    strb w10, [x2, x11]
    next

opcode _DUPr
    ldrb w10, [x2, x3]   ; get the top byte
    rpush w10
    next

opcode _OVRr
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

opcode _EQUr
    compare_opr eq

opcode _NEQr
    compare_opr ne

opcode _GTHr
    compare_opr hi

opcode _LTHr
    compare_opr lo

opcode _JMPr
    ldrsb x9, [x2, x3]
    rpop
    add x5, x5, x9
    and x5, x5, 0xffff
    next

opcode _JCNr
    ldrsb w9, [x2, x3]
    rpop
    ldrb w10, [x2, x3]
    rpop
    cmp w10, #0
    csel w10, wzr, w9, eq ; choose the jump or not
    add x5, x5, x10 ; jump or not
    and x5, x5, 0xffff
    next

opcode _JSRr
    ldrsb w9, [x2, x3]
    rpop
    lsr w10, w5, 8
    push w10
    push w5
    add x5, x5, x9
    and x5, x5, 0xffff
    next

opcode _STHr
    ldrb w9, [x2, x3]
    rpop
    push w9
    next

opcode _LDZr
    ldrb w9, [x2, x3]
    rpop
    ldrb w9, [x4, x9]
    rpush w9
    next

opcode _STZr
    ldrb w9, [x2, x3]
    rpop
    ldrb w10, [x2, x3]
    rpop
    strb w10, [x4, x9]
    next

opcode _LDRr
    ldrsb w9, [x2, x3]
    add x9, x5, x9
    and x9, x9, #0xffff
    ldrb w9, [x4, x9] ; read from RAM
    strb w9, [x2, x3] ; push to stack
    next

opcode _STRr
    ldrsb w9, [x2, x3]
    rpop
    ldrb w10, [x2, x3]
    rpop
    add x9, x5, x9
    and x9, x9, #0xffff
    strb w10, [x4, x9] ; write to RAM
    next

opcode _LDAr
    ldrb w9, [x2, x3]
    rpop
    ldrb w10, [x2, x3]
    orr w12, w9, w10, lsl #8
    ldrb w12, [x4, x12]
    strb w12, [x2, x3]
    next

opcode _STAr
    ldrb w9, [x2, x3]
    rpop
    ldrb w10, [x2, x3]
    rpop
    orr w12, w9, w10, lsl #8
    ldrb w10, [x2, x3]
    rpop
    strb w10, [x4, x12]
    next

opcode _DEIr
    precall
    bl _dei_r_entry
    postcall
    next

opcode _DEOr
    precall
    bl _deo_r_entry ; todo check return value for early exit?
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

opcode _ADDr
    binary_opr add

opcode _SUBr
    binary_opr sub

opcode _MULr
    binary_opr mul

opcode _DIVr
    binary_opr udiv

opcode _ANDr
    binary_opr and

opcode _ORAr
    binary_opr orr

opcode _EORr
    binary_opr eor

opcode _SFTr
    ldrb w10, [x2, x3]
    rpop
    ldrb w11, [x2, x3]
    lsr w12, w10, 4
    and w10, w10, #0xf
    lsr w11, w11, w10
    lsl w11, w11, w12
    strb w11, [x2, x3]
    next

opcode _JSI
    ldrb w9, [x4, x5]
    add x5, x5, #1
    and x5, x5, #0xffff
    ldrb w10, [x4, x5]
    add x5, x5, #1
    and x5, x5, #0xffff

    orr w12, w10, w9, lsl #8 ; build the jump offset

    ; Store PC + 2 to the return stack
    lsr w9, w5, 8
    rpush w9
    rpush w5

    add x5, x5, x12 ; do the jump
    and x5, x5, 0xffff
    next

opcode _INC2r
    ldrb w10, [x2, x3]
    rpeek w11, x9, 1
    orr w12, w10, w11, lsl #8
    add w12, w12, #1
    and w12, w12, #0xffff
    strb w12, [x2, x3]
    lsr w12, w12, 8
    strb w12, [x2, x9]
    next

opcode _POP2r
    sub x3, x3, #2
    and x3, x3, #0xff
    next

opcode _NIP2r
    ldrb w9, [x2, x3]
    rpop
    ldrb w10, [x2, x3]
    rpop
    strb w9, [x2, x3]
    sub x11, x1, #1
    and x11, x11, #0xff
    strb w10, [x2, x31]
    next

opcode _SWP2r
    ldrb w11, [x2, x3]  ; get the top byte
    rpeek w12, x9, 2    ; get the second-from-top byte
    strb w11, [x2, x9]  ; do the swap!
    strb w12, [x2, x3]

    rpeek w11, x9, 1
    rpeek w12, x10, 2

    strb w11, [x2, x10]
    strb w12, [x2, x9]

    next

opcode _ROT2r
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

opcode _DUP2r
    ldrb w11, [x2, x3]
    sub w9, w3, #1
    and w9, w9, #0xff
    ldrb w10, [x2, x9]
    rpush w10
    rpush w11
    next

opcode _OVR2r
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

opcode _EQU2r
    compare_op2r eq

opcode _NEQ2r
    compare_op2r ne

opcode _GTH2r
    compare_op2r hi

opcode _LTH2r
    compare_op2r lo

opcode _JMP2r
    ldrb w9, [x2, x3]
    rpop
    ldrb w10, [x2, x3]
    rpop
    orr w5, w9, w10, lsl #8 ; update program counter
    next

opcode _JCN2r
    ldrb w9, [x2, x3]
    rpop
    ldrb w10, [x2, x3]
    rpop
    ldrb w11, [x2, x3]
    rpop
    orr w9, w9, w10, lsl #8 ; update program counter
    cmp w11, #0
    csel w5, w5, w9, eq ; choose the jump or not
    next

opcode _JSR2r
    ldrb w9, [x2, x3]
    rpop
    ldrb w10, [x2, x3]
    rpop
    lsr w11, w5, 8
    push w11
    push w5
    orr w5, w9, w10, lsl #8 ; update program counter
    next

opcode _STH2r
    ldrb w9, [x2, x3]
    rpop
    ldrb w10, [x2, x3]
    rpop
    push w10
    push w9
    next

opcode _LDZ2r
    ldrb w9, [x2, x3]
    rpop
    ldrb w10, [x4, x9]
    rpush w10
    add w9, w9, #1
    and w9, w9, #0xFFFF
    ldrb w10, [x4, x9]
    rpush w10
    next

opcode _STZ2r
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

opcode _LDR2r
    ldrsb w9, [x2, x3]
    add x9, x5, x9
    and x9, x9, #0xffff
    ldrb w10, [x4, x9] ; read from RAM
    strb w10, [x2, x3] ; push to stack
    add x9, x9, #1
    and x9, x9, #0xffff
    ldrb w10, [x4, x9] ; read from RAM
    rpush w10
    next

opcode _STR2r
    ldrsb w9, [x2, x3]
    rpop
    ldrsb w10, [x2, x3]
    rpop
    ldrsb w11, [x2, x3]
    rpop
    add x9, x5, x9
    and x9, x9, #0xffff
    strb w11, [x4, x9] ; write to RAM
    add x9, x9, #1
    and x9, x9, #0xffff
    strb w10, [x4, x9] ; write to RAM
    next

opcode _LDA2r
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

opcode _STA2r
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

opcode _DEI2r
    precall
    bl _dei_2r_entry
    postcall
    next

opcode _DEO2r
    precall
    bl _deo_2r_entry ; todo check return value for early exit?
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

opcode _ADD2r
    binary_op2r add

opcode _SUB2r
    binary_op2r sub

opcode _MUL2r
    binary_op2r mul

opcode _DIV2r
    binary_op2r udiv

opcode _AND2r
    binary_op2r and

opcode _ORA2r
    binary_op2r orr

opcode _EOR2r
    binary_op2r eor

opcode _SFT2r
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

opcode _LIT
    ldrb w9, [x4, x5]
    add x5, x5, #1
    and x5, x5, #0xffff
    push w9
    next

opcode _INCk
    ldrb w9, [x0, x1]
    add w9, w9, #1
    push w9
    next

opcode _POPk
    next

opcode _NIPk
    ldrb w9, [x0, x1]
    push w9
    next

opcode _SWPk
    ldrb w10, [x0, x1]   ; get the top byte
    peek w11, x9, 1      ; get the second-from-top byte
    push w10
    push w11
    next

opcode _ROTk
    ldrb w13, [x0, x1]
    peek w10, x11, 1
    push w10
    push w13
    peek w10, x11, 4
    push w10
    next

opcode _DUPk
    ldrb w11, [x0, x1]
    push w11
    push w11
    next

opcode _OVRk
    peek w10, x9, 1 ; get the second-from-top
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

opcode _EQUk
    compare_opk eq

opcode _NEQk
    compare_opk ne

opcode _GTHk
    compare_opk hi

opcode _LTHk
    compare_opk lo

opcode _JMPk
    ldrsb x9, [x0, x1]
    add x5, x5, x9
    and x5, x5, 0xffff
    next

opcode _JCNk
    ldrsb w9, [x0, x1]
    peek w10, x10, 1
    cmp w10, #0
    csel w10, wzr, w9, eq ; choose the jump or not
    add x5, x5, x10 ; jump or not
    and x5, x5, 0xffff
    next

opcode _JSRk
    ldrsb w9, [x0, x1]
    lsr w10, w5, 8
    rpush w10
    rpush w5
    add x5, x5, x9
    and x5, x5, 0xffff
    next

opcode _STHk
    ldrb w9, [x0, x1]
    rpush w9
    next

opcode _LDZk
    ldrb w9, [x0, x1]
    ldrb w9, [x4, x9]
    push w9
    next

opcode _STZk
    ldrb w9, [x0, x1]
    peek w10, x10, 1
    strb w10, [x4, x9]
    next

opcode _LDRk
    ldrsb w9, [x0, x1]
    add x9, x5, x9
    and x9, x9, #0xffff
    ldrb w9, [x4, x9] ; read from RAM
    push w9
    next

opcode _STRk
    ldrsb w9, [x0, x1]
    peek w10, x10, 1
    add x9, x5, x9
    and x9, x9, #0xffff
    strb w10, [x4, x9] ; write to RAM
    next

opcode _LDAk
    ldrb w9, [x0, x1]
    sub w10, w1, #1
    and w10, w10, #0xff
    ldrb w10, [x0, x10]
    orr w10, w9, w10, lsl #8    ; build address
    ldrb w10, [x4, x10]         ; load byte from RAM
    push w10
    next

opcode _STAk
    ldrb w9, [x0, x1]
    peek w10, x10, 1
    orr w12, w9, w10, lsl #8
    peek w10, x10, 2
    strb w10, [x4, x12]
    next

opcode _DEIk
    precall
    bl _dei_k_entry
    postcall
    next

opcode _DEOk
    precall
    bl _deo_k_entry ; todo check return value for early exit?
    postcall
    next

.macro binary_opk op
    peek w11, x9, 1
    ldrb w10, [x0, x1]
    \op w10, w11, w10
    push w10
    next
.endm

opcode _ADDk
    binary_opk add

opcode _SUBk
    binary_opk sub

opcode _MULk
    binary_opk mul

opcode _DIVk
    binary_opk udiv

opcode _ANDk
    binary_opk and

opcode _ORAk
    binary_opk orr

opcode _EORk
    binary_opk eor

opcode _SFTk
    ldrb w10, [x0, x1]
    peek w11, x9, 1
    lsr w12, w10, 4
    and w10, w10, #0xf
    lsr w11, w11, w10
    lsl w11, w11, w12
    push w11
    next

opcode _LIT2
    ldrb w9, [x4, x5]
    add x5, x5, #1
    and x5, x5, #0xffff
    push w9
    ldrb w9, [x4, x5]
    add x5, x5, #1
    and x5, x5, #0xffff
    push w9
    next

opcode _INC2k
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

opcode _POP2k
    next

opcode _NIP2k
    ldrb w9, [x0, x1]
    peek w10, x11, 1
    push w10
    push w9
    next

opcode _SWP2k
    peek w11, x9, 1
    push w11
    peek w11, x9, 1
    push w11
    peek w11, x9, 5
    push w11
    peek w11, x9, 5
    push w11
    next

opcode _ROT2k
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

opcode _DUP2k
    ldrb w11, [x0, x1]
    sub w9, w1, #1
    and w9, w9, #0xff
    ldrb w10, [x0, x9]

    push w10
    push w11
    push w10
    push w11
    next

opcode _OVR2k
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

opcode _EQU2k
    compare_op2k eq

opcode _NEQ2k
    compare_op2k ne

opcode _GTH2k
    compare_op2k hi

opcode _LTH2k
    compare_op2k lo

opcode _JMP2k
    ldrb w9, [x0, x1]
    peek w10, x10, 1
    orr w5, w9, w10, lsl #8 ; update program counter
    next

opcode _JCN2k
    ldrb w9, [x0, x1]
    peek w10, x12, 1
    peek w11, x12, 2

    orr w9, w9, w10, lsl #8 ; update program counter
    cmp w11, #0
    csel w5, w5, w9, eq ; choose the jump or not
    next

opcode _JSR2k
    ldrb w9, [x0, x1]
    peek w10, x10, 1

    lsr w11, w5, 8
    rpush w11
    rpush w5

    orr w5, w9, w10, lsl #8 ; update program counter
    next

opcode _STH2k
    ldrb w9, [x0, x1]
    peek w10, x10, 1
    rpush w10
    rpush w9
    next

opcode _LDZ2k
    ldrb w9, [x0, x1]
    ldrb w10, [x4, x9]
    push w10
    add w9, w9, #1
    and w9, w9, #0xFFFF
    ldrb w10, [x4, x9]
    push w10
    next

opcode _STZ2k
    ldrb w9, [x0, x1]
    peek w10, x10, 1
    peek w11, x11, 2

    strb w11, [x4, x9]
    add w9, w9, #1
    and w9, w9, #0xFFFF
    strb w10, [x4, x9]
    next

opcode _LDR2k
    ldrsb w9, [x0, x1]
    add x9, x5, x9
    and x9, x9, #0xffff
    ldrb w10, [x4, x9] ; read from RAM
    push w10
    add x9, x9, #1
    and x9, x9, #0xffff
    ldrb w10, [x4, x9] ; read from RAM
    push w10
    next

opcode _STR2k
    ldrsb w9, [x0, x1]
    peek w10, x10, 1
    peek w11, x11, 2

    add x9, x5, x9
    and x9, x9, #0xffff
    strb w11, [x4, x9] ; write to RAM
    add x9, x9, #1
    and x9, x9, #0xffff
    strb w10, [x4, x9] ; write to RAM
    next

opcode _LDA2k
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

opcode _STA2k
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

opcode _DEI2k
    precall
    bl _dei_2k_entry
    postcall
    next

opcode _DEO2k
    precall
    bl _deo_2k_entry ; todo check return value for early exit?
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

opcode _ADD2k
    binary_op2k add

opcode _SUB2k
    binary_op2k sub

opcode _MUL2k
    binary_op2k mul

opcode _DIV2k
    binary_op2k udiv

opcode _AND2k
    binary_op2k and

opcode _ORA2k
    binary_op2k orr

opcode _EOR2k
    binary_op2k eor

opcode _SFT2k
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

opcode _LITr
    ldrb w9, [x4, x5]
    add x5, x5, #1
    and x5, x5, #0xffff
    rpush w9
    next

opcode _INCkr
    ldrb w9, [x2, x3]
    add w9, w9, #1
    rpush w9
    next

opcode _POPkr
    next

opcode _NIPkr
    ldrb w9, [x2, x3]
    rpush w9
    next

opcode _SWPkr
    ldrb w10, [x2, x3]   ; get the top byte
    rpeek w11, x9, 1
    rpush w10
    rpush w11
    next

opcode _ROTkr
    ldrb w13, [x2, x3]
    rpeek w10, x11, 1
    rpush w10
    rpush w13
    rpeek w10, x11, 4
    rpush w10
    next

opcode _DUPkr
    ldrb w11, [x2, x3]
    rpush w11
    rpush w11
    next

opcode _OVRkr
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

opcode _EQUkr
    compare_opkr eq

opcode _NEQkr
    compare_opkr ne

opcode _GTHkr
    compare_opkr hi

opcode _LTHkr
    compare_opkr lo

opcode _JMPkr
    ldrsb x9, [x2, x3]
    add x5, x5, x9
    and x5, x5, 0xffff
    next

opcode _JCNkr
    ldrsb w9, [x2, x3]
    rpeek w10, x10, 1
    cmp w10, #0
    csel w10, wzr, w9, eq ; choose the jump or not
    add x5, x5, x10 ; jump or not
    and x5, x5, 0xffff
    next

opcode _JSRkr
    ldrsb w9, [x2, x3]
    lsr w10, w5, 8
    push w10
    push w5
    add x5, x5, x9
    and x5, x5, 0xffff
    next

opcode _STHkr
    ldrb w9, [x2, x3]
    push w9
    next

opcode _LDZkr
    ldrb w9, [x2, x3]
    ldrb w9, [x4, x9]
    rpush w9
    next

opcode _STZkr
    ldrb w9, [x2, x3]
    rpeek w10, x10, 1
    strb w10, [x4, x9]
    next

opcode _LDRkr
    ldrsb w9, [x2, x3]
    add x9, x5, x9
    and x9, x9, #0xffff
    ldrb w9, [x4, x9] ; read from RAM
    rpush w9
    next

opcode _STRkr
    ldrsb w9, [x2, x3]
    rpeek w10, x10, 1
    add x9, x5, x9
    and x9, x9, #0xffff
    strb w10, [x4, x9] ; write to RAM
    next

opcode _LDAkr
    ldrb w9, [x2, x3]
    sub w10, w3, #1
    and w10, w10, #0xff
    ldrb w10, [x2, x10]
    orr w10, w9, w10, lsl #8    ; build address
    ldrb w10, [x4, x10]         ; load byte from RAM
    rpush w10
    next

opcode _STAkr
    ldrb w9, [x2, x3]
    rpeek w10, x10, 1
    orr w12, w9, w10, lsl #8
    rpeek w10, x10, 2
    strb w10, [x4, x12]
    next

opcode _DEIkr
    precall
    bl _dei_kr_entry
    postcall
    next

opcode _DEOkr
    precall
    bl _deo_kr_entry ; todo check return value for early exit?
    postcall
    next

.macro binary_opkr op
    ldrb w10, [x2, x3]
    rpeek w11, x9, 1
    \op w10, w11, w10
    rpush w10
    next
.endm

opcode _ADDkr
    binary_opkr add

opcode _SUBkr
    binary_opkr sub

opcode _MULkr
    binary_opkr mul

opcode _DIVkr
    binary_opkr udiv

opcode _ANDkr
    binary_opkr and

opcode _ORAkr
    binary_opkr orr

opcode _EORkr
    binary_opkr eor

opcode _SFTkr
    ldrb w10, [x2, x3]
    rpeek w11, x9, 1
    lsr w12, w10, 4
    and w10, w10, #0xf
    lsr w11, w11, w10
    lsl w11, w11, w12
    rpush w11
    next

opcode _LIT2r
    ldrb w9, [x4, x5]
    add x5, x5, #1
    and x5, x5, #0xffff
    rpush w9
    ldrb w9, [x4, x5]
    add x5, x5, #1
    and x5, x5, #0xffff
    rpush w9
    next

opcode _INC2kr
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

opcode _POP2kr
    next

opcode _NIP2kr
    ldrb w9, [x2, x3]
    rpeek w10, x11, 1
    rpush w10
    rpush w9
    next

opcode _SWP2kr
    rpeek w11, x9, 1
    rpush w11
    rpeek w11, x9, 1
    rpush w11
    rpeek w11, x9, 5
    rpush w11
    rpeek w11, x9, 5
    rpush w11
    next

opcode _ROT2kr
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

opcode _DUP2kr
    ldrb w11, [x2, x3]
    sub w9, w3, #1
    and w9, w9, #0xff
    ldrb w10, [x2, x9]

    rpush w10
    rpush w11
    rpush w10
    rpush w11
    next

opcode _OVR2kr
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

opcode _EQU2kr
    compare_op2kr eq

opcode _NEQ2kr
    compare_op2kr ne

opcode _GTH2kr
    compare_op2kr hi

opcode _LTH2kr
    compare_op2kr lo

opcode _JMP2kr
    ldrb w9, [x2, x3]
    rpeek w10, x10, 1
    orr w5, w9, w10, lsl #8 ; update program counter
    next

opcode _JCN2kr
    ldrb w9, [x2, x3]
    rpeek w10, x12, 1
    rpeek w11, x12, 2
    orr w9, w9, w10, lsl #8 ; update program counter
    cmp w11, #0
    csel w5, w5, w9, eq ; choose the jump or not
    next

opcode _JSR2kr
    ldrb w9, [x2, x3]
    rpeek w10, x10, 1
    lsr w11, w5, 8
    push w11
    push w5
    orr w5, w9, w10, lsl #8 ; update program counter
    next

opcode _STH2kr
    ldrb w9, [x2, x3]
    rpeek w10, x11, 1
    push w10
    push w9
    next

opcode _LDZ2kr
    ldrb w9, [x2, x3]
    ldrb w10, [x4, x9]
    rpush w10
    add w9, w9, #1
    and w9, w9, #0xFFFF
    ldrb w10, [x4, x9]
    rpush w10
    next

opcode _STZ2kr
    ldrb w9, [x2, x3]
    rpeek w10, x10, 1
    rpeek w11, x11, 2

    strb w11, [x4, x9]
    add w9, w9, #1
    and w9, w9, #0xFFFF
    strb w10, [x4, x9]
    next

opcode _LDR2kr
    ldrsb w9, [x2, x3]
    add x9, x5, x9
    and x9, x9, #0xffff
    ldrb w10, [x4, x9] ; read from RAM
    rpush w10
    add x9, x9, #1
    and x9, x9, #0xffff
    ldrb w10, [x4, x9] ; read from RAM
    rpush w10
    next

opcode _STR2kr
    ldrsb w9, [x2, x3]
    rpeek w10, x10, 1
    rpeek w11, x11, 2

    add x9, x5, x9
    and x9, x9, #0xffff
    strb w11, [x4, x9] ; write to RAM
    add x9, x9, #1
    and x9, x9, #0xffff
    strb w10, [x4, x9] ; write to RAM
    next

opcode _LDA2kr
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

opcode _STA2kr
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

opcode _DEI2kr
    precall
    bl _dei_2kr_entry
    postcall
    next

opcode _DEO2kr
    precall
    bl _deo_2kr_entry ; todo check return value for early exit?
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

opcode _ADD2kr
    binary_op2kr add

opcode _SUB2kr
    binary_op2kr sub

opcode _MUL2kr
    binary_op2kr mul

opcode _DIV2kr
    binary_op2kr udiv

opcode _AND2kr
    binary_op2kr and

opcode _ORA2kr
    binary_op2kr orr

opcode _EOR2kr
    binary_op2kr eor

opcode _SFT2kr
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
