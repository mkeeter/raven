// x86-64 Uxn interpreter - System V AMD64 ABI (Linux)
//
// Register allocation (all callee-saved to survive DEI/DEO calls):
//   rbx - stack data pointer (&mut [u8; 256])
//   r12 - stack index (u8, zero-extended)
//   r13 - return stack data pointer (&mut [u8; 256])
//   r14 - return stack index (u8, zero-extended)
//   r15 - RAM pointer (&mut [u8; 65536])
//   rbp - program counter (u16, zero-extended)
//
// The VM pointer and device handle pointer are stored on the stack frame.
// The jump table pointer is also stored on the stack frame and loaded into
// a scratch register for dispatch.
//
// Stack frame layout (offsets from rsp after prologue):
//   [rsp+0x58]  DeviceHandle pointer (from caller)
//   [rsp+0x50]  VM DeviceHandle pointer (from caller)
//   [rsp+0x48]  return address (from caller)
//   [rsp+0x40]  saved rbx
//   [rsp+0x38]  saved rbp
//   [rsp+0x30]  saved r12
//   [rsp+0x28]  saved r13
//   [rsp+0x20]  saved r14
//   [rsp+0x18]  saved r15
//   [rsp+0x10]  saved stack_index pointer (from entry arg)
//   [rsp+0x08]  saved ret_index pointer (from entry arg)
//   [rsp+0x00]  alignment
//
// Scratch registers (not preserved across instructions, but saved in precall):
//   rax, rcx, rdx, rsi, rdi, r8, r9, r10, r11

// Advance PC, fetch opcode, dispatch via jump table
.macro next
    movzx eax, byte ptr [r15 + rbp]
    inc rbp
    and rbp, 0xffff
    lea rcx, [rip + JUMP_TABLE]
    jmp qword ptr [rcx + rax*8]
.endm

// Stack operations (work stack: rbx=data, r12=index)
.macro stk_pop
    dec r12
    and r12, 0xff
.endm

.macro rpop
    dec r14
    and r14, 0xff
.endm

// push reg8 onto work stack
.macro stk_push reg
    inc r12
    and r12, 0xff
    mov byte ptr [rbx + r12], \reg
.endm

// push reg8 onto return stack
.macro rpush reg
    inc r14
    and r14, 0xff
    mov byte ptr [r13 + r14], \reg
.endm

// peek: load byte from work stack at offset n below top into reg (zero-extended)
// uses r11 as scratch for address computation
.macro peek reg, n
    peek_ \reg, \n, r11
.endm

.macro peek_ reg, n, tmp
    lea \tmp, [r12 - \n]
    and \tmp, 0xff
    movzx \reg, byte ptr [rbx + \tmp]
.endm

// rpeek: load byte from return stack at offset n below top
.macro rpeek reg, n
    rpeek_ \reg, \n, r11
.endm

.macro rpeek_ reg, n, tmp
    lea \tmp, [r14 - \n]
    and \tmp, 0xff
    movzx \reg, byte ptr [r13 + \tmp]
.endm

// Save all interpreter state to the stack frame and set up args for C call
// C calling convention: arg1=rdi (VM ptr), arg2=rsi (DeviceHandle ptr)
.macro precall
    // Write stack indices back through the pointers saved at entry
    mov rax, qword ptr [rsp + 0x08]   // stack_index pointer
    mov byte ptr [rax], r12b
    mov rax, qword ptr [rsp + 0x10]   // ret_index pointer
    mov byte ptr [rax], r14b

    // Set up args: VM ptr and DeviceHandle ptr
    mov rdi, qword ptr [rsp + 0x50]
    mov rsi, qword ptr [rsp + 0x58]
.endm

// Restore all interpreter state after a C call
.macro postcall
    // Reload stack indices (DEI/DEO may have modified them)
    mov rax, qword ptr [rsp + 0x08]
    movzx r12, byte ptr [rax]
    mov rax, qword ptr [rsp + 0x10]
    movzx r14, byte ptr [rax]
.endm

// Entry point - called from Rust
// Signature (System V AMD64):
//   rdi = stack data ptr
//   rsi = stack_index ptr  (pointer to u8)
//   rdx = ret data ptr
//   rcx = ret_index ptr    (pointer to u8)
//   r8  = RAM ptr
//   r9  = pc (u16, zero-extended in 32-bit arg)
//   [rsp+8]  = VM ptr
//   [rsp+16] = DeviceHandle ptr
ENTRY interpreter_entry
    // Prologue: save callee-saved registers and build frame
    // We need 0x90 bytes of local space (aligned to 16 after 8-byte ret addr)
    push rbx
    push rbp
    push r12
    push r13
    push r14
    push r15
    sub rsp, 0x18

    // Save entry-time stack index pointers (rsi=stack_idx_ptr, rcx=ret_idx_ptr)
    mov qword ptr [rsp + 0x08], rsi
    mov qword ptr [rsp + 0x10], rcx

    // Load interpreter registers from arguments
    mov rbx, rdi                    // stack data ptr
    movzx r12, byte ptr [rsi]       // stack index value
    mov r13, rdx                    // ret stack data ptr
    movzx r14, byte ptr [rcx]       // ret stack index value
    mov r15, r8                     // RAM ptr
    movzx rbp, r9w                  // PC (u16)

    next

_BRK:
    // Write stack indices back through saved pointers
    mov rax, qword ptr [rsp + 0x08]
    mov byte ptr [rax], r12b
    mov rax, qword ptr [rsp + 0x10]
    mov byte ptr [rax], r14b

    // Return PC in rax
    movzx eax, bp

    // Epilogue
    add rsp, 0x18
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbp
    pop rbx
    ret

_INC:
    movzx eax, byte ptr [rbx + r12]
    inc al
    mov byte ptr [rbx + r12], al
    next

_POP:
    stk_pop
    next

_NIP:
    movzx eax, byte ptr [rbx + r12]   // top byte
    stk_pop
    mov byte ptr [rbx + r12], al      // overwrite second byte
    next

_SWP:
    peek ecx, 1                       // a (peek first; r11 is new address)
    movzx eax, byte ptr [rbx + r12]   // b (loaded after peek)
    mov byte ptr [rbx + r12], cl      // store a at top
    mov byte ptr [rbx + r11], al      // store b at second
    next

_ROT:
    // a b c -- b c a  (c=top)
    movzx r8d, byte ptr [rbx + r12]   // c → r8d
    peek ecx, 1                       // b → ecx
    mov byte ptr [rbx + r11], r8b     // second = c
    peek edx, 2                       // a → edx
    mov byte ptr [rbx + r11], cl      // third = b
    mov byte ptr [rbx + r12], dl      // top = a
    next

_DUP:
    movzx eax, byte ptr [rbx + r12]
    stk_push al
    next

_OVR:
    peek eax, 1
    stk_push al
    next

.macro compare_op setcc_op
    movzx eax, byte ptr [rbx + r12]   // top (b)
    stk_pop
    movzx ecx, byte ptr [rbx + r12]   // second (a)
    cmp ecx, eax
    \setcc_op al
    mov byte ptr [rbx + r12], al
    next
.endm

_EQU:
    compare_op sete

_NEQ:
    compare_op setne

_GTH:
    compare_op seta

_LTH:
    compare_op setb

_JMP:
    movsx rax, byte ptr [rbx + r12]
    stk_pop
    add rbp, rax
    and rbp, 0xffff
    next

_JCN:
    movsx eax, byte ptr [rbx + r12]   // offset (signed)
    stk_pop
    movzx ecx, byte ptr [rbx + r12]   // condition
    stk_pop
    test ecx, ecx
    jz 1f
    add rbp, rax
    and rbp, 0xffff
1:
    next

_JSR:
    movsx eax, byte ptr [rbx + r12]   // offset (signed)
    stk_pop
    mov ecx, ebp
    shr ecx, 8
    rpush cl                           // push high byte of PC
    rpush bpl                          // push low byte of PC
    add rbp, rax
    and rbp, 0xffff
    next

_STH:
    movzx eax, byte ptr [rbx + r12]
    stk_pop
    rpush al
    next

_LDZ:
    movzx eax, byte ptr [rbx + r12]
    stk_pop
    movzx eax, byte ptr [r15 + rax]
    stk_push al
    next

_STZ:
    movzx eax, byte ptr [rbx + r12]   // zero-page address
    stk_pop
    movzx ecx, byte ptr [rbx + r12]   // value
    stk_pop
    mov byte ptr [r15 + rax], cl
    next

_LDR:
    movsx rax, byte ptr [rbx + r12]   // signed offset
    lea rax, [rbp + rax]
    and rax, 0xffff
    movzx eax, byte ptr [r15 + rax]
    mov byte ptr [rbx + r12], al      // overwrite (no pop, just replace)
    next

_STR:
    movsx rax, byte ptr [rbx + r12]   // signed offset
    stk_pop
    movzx ecx, byte ptr [rbx + r12]   // value
    stk_pop
    lea rax, [rbp + rax]
    and rax, 0xffff
    mov byte ptr [r15 + rax], cl
    next

_LDA:
    movzx eax, byte ptr [rbx + r12]   // low byte of address
    stk_pop
    movzx ecx, byte ptr [rbx + r12]   // high byte
    shl ecx, 8
    or eax, ecx                        // full 16-bit address
    movzx eax, byte ptr [r15 + rax]
    mov byte ptr [rbx + r12], al
    next

_STA:
    movzx eax, byte ptr [rbx + r12]   // addr low
    stk_pop
    movzx ecx, byte ptr [rbx + r12]   // addr high
    stk_pop
    shl ecx, 8
    or eax, ecx
    movzx edx, byte ptr [rbx + r12]   // value
    stk_pop
    mov byte ptr [r15 + rax], dl
    next

_DEI:
    precall
    CALL dei_entry
    postcall
    next

_DEO:
    precall
    CALL deo_entry
    postcall
    next

.macro binary_op insn
    movzx eax, byte ptr [rbx + r12]   // top (b)
    stk_pop
    movzx ecx, byte ptr [rbx + r12]   // second (a)
    \insn ecx, eax
    mov byte ptr [rbx + r12], cl
    next
.endm

_ADD:
    binary_op add

_SUB:
    binary_op sub

_MUL:
    binary_op imul

_DIV:
    movzx ecx, byte ptr [rbx + r12]   // b (divisor), top
    stk_pop
    movzx eax, byte ptr [rbx + r12]   // a (dividend), second
    movzx eax, al
    test cl, cl
    jz 1f
    div cl
    jmp 2f
1:
    xor eax, eax                       // div by zero → 0
2:
    mov byte ptr [rbx + r12], al
    next

_AND:
    binary_op and

_ORA:
    binary_op or

_EOR:
    binary_op xor

_SFT:
    movzx eax, byte ptr [rbx + r12]   // shift amount
    stk_pop
    movzx edx, byte ptr [rbx + r12]   // value in dl
    mov ecx, eax
    and ecx, 0xf                       // right-shift count in cl
    shr dl, cl
    shr eax, 4                         // left-shift count
    mov ecx, eax
    shl dl, cl
    mov byte ptr [rbx + r12], dl
    next

_JCI:
    movzx eax, byte ptr [r15 + rbp]   // offset high byte
    inc rbp
    and rbp, 0xffff
    movzx ecx, byte ptr [r15 + rbp]   // offset low byte
    inc rbp
    and rbp, 0xffff
    shl eax, 8
    or eax, ecx                        // 16-bit offset
    movzx edx, byte ptr [rbx + r12]   // condition
    stk_pop
    test edx, edx
    jz 1f
    // sign-extend 16-bit offset to 64 bits and add
    movsx rax, ax
    add rbp, rax
    and rbp, 0xffff
1:
    next

_INC2:
    peek ecx, 1                       // high byte (peek first; r11 is addr)
    movzx eax, byte ptr [rbx + r12]   // low byte (loaded after peek)
    shl ecx, 8
    or eax, ecx
    inc eax
    and eax, 0xffff
    mov byte ptr [rbx + r12], al
    shr eax, 8
    mov byte ptr [rbx + r11], al
    next

_POP2:
    sub r12, 2
    and r12, 0xff
    next

_NIP2:
    movzx eax, byte ptr [rbx + r12]   // b_lo (top)
    stk_pop
    movzx ecx, byte ptr [rbx + r12]   // b_hi (second)
    stk_pop
    mov byte ptr [rbx + r12], al      // b_lo at new top (a_lo position)
    lea rdx, [r12 - 1]
    and rdx, 0xff
    mov byte ptr [rbx + rdx], cl      // b_hi below (a_hi position)
    next

_SWP2:
    peek ecx, 2                        // a_lo (peek first; rax clobbered)
    movzx eax, byte ptr [rbx + r12]   // b_lo (loaded after peek)
    mov byte ptr [rbx + r12], cl      // store a_lo at b_lo's position
    mov byte ptr [rbx + r11], al      // store b_lo at a_lo's position

    peek_ ecx, 3, r10                        // a_hi (peek first; r11 clobbered)
    peek eax, 1                        // b_hi (peek second; ecx still = a_hi)
    mov byte ptr [rbx + r11], cl       // store a_hi at b_hi's position
    mov byte ptr [rbx + r10], al      // store b_hi at a_hi's position
    next

_ROT2:
    // a_hi a_lo b_hi b_lo c_hi c_lo -- b_hi b_lo c_hi c_lo a_hi a_lo
    peek_ ecx, 2, r10                 // b_lo (peek first; rax clobbered)
    movzx r8d, byte ptr [rbx + r12]   // c_lo (loaded after peek)
    peek edx, 4                       // a_lo (clobbers rax; ecx=b_lo, r8d=c_lo valid)
    mov byte ptr [rbx + r12], dl      // store a_lo at top
    mov byte ptr [rbx + r10], r8b     // store c_lo at second short's lo
    mov byte ptr [rbx + r11], cl      // store b_lo at third short's lo

    peek_ r8d, 1, r10                 // c_hi (peek into eax first)
    peek_ ecx, 3, r9                  // b_hi (clobbers rax; r8d=c_hi valid)
    peek edx, 5                       // a_hi (clobbers rax; ecx=b_hi, r8d=c_hi valid)
    mov byte ptr [rbx + r10], dl      // store a_hi
    mov byte ptr [rbx + r9], r8b      // store c_hi
    mov byte ptr [rbx + r11], cl      // store b_hi
    next

_DUP2:
    peek ecx, 1                        // hi (peek first; rax clobbered)
    movzx eax, byte ptr [rbx + r12]   // lo (loaded after peek)
    stk_push cl                        // push hi
    stk_push al                        // push lo
    next

_OVR2:
    peek ecx, 3                        // a_hi (peek first; rax clobbered)
    peek eax, 2                        // a_lo (peek second; ecx=a_hi still valid)
    stk_push cl                        // push a_hi
    stk_push al                        // push a_lo
    next

.macro compare_op2 setcc_op
    movzx eax, byte ptr [rbx + r12]
    stk_pop
    movzx ecx, byte ptr [rbx + r12]
    stk_pop
    shl ecx, 8
    or eax, ecx                        // b (top short)
    movzx ecx, byte ptr [rbx + r12]
    stk_pop
    movzx edx, byte ptr [rbx + r12]
    shl edx, 8
    or ecx, edx                        // a (second short)
    cmp ecx, eax
    \setcc_op al
    movzx eax, al
    mov byte ptr [rbx + r12], al
    next
.endm

_EQU2:
    compare_op2 sete

_NEQ2:
    compare_op2 setne

_GTH2:
    compare_op2 seta

_LTH2:
    compare_op2 setb

_JMP2:
    movzx eax, byte ptr [rbx + r12]   // low byte
    stk_pop
    movzx ecx, byte ptr [rbx + r12]   // high byte
    stk_pop
    shl ecx, 8
    or eax, ecx
    mov rbp, rax
    next

_JCN2:
    movzx eax, byte ptr [rbx + r12]   // addr low
    stk_pop
    movzx ecx, byte ptr [rbx + r12]   // addr high
    stk_pop
    movzx edx, byte ptr [rbx + r12]   // condition
    stk_pop
    shl ecx, 8
    or eax, ecx
    test edx, edx
    jz 1f
    mov rbp, rax
1:
    next

_JSR2:
    movzx eax, byte ptr [rbx + r12]   // addr low
    stk_pop
    movzx ecx, byte ptr [rbx + r12]   // addr high
    stk_pop
    shl ecx, 8
    or eax, ecx
    mov edx, ebp
    shr edx, 8
    rpush dl
    rpush bpl
    mov rbp, rax
    next

_STH2:
    movzx eax, byte ptr [rbx + r12]   // low byte
    stk_pop
    movzx ecx, byte ptr [rbx + r12]   // high byte
    stk_pop
    rpush cl
    rpush al
    next

_LDZ2:
    movzx eax, byte ptr [rbx + r12]   // zero-page address
    stk_pop
    movzx ecx, byte ptr [r15 + rax]
    stk_push cl
    inc eax
    and eax, 0xffff
    movzx ecx, byte ptr [r15 + rax]
    stk_push cl
    next

_STZ2:
    movzx eax, byte ptr [rbx + r12]   // address
    stk_pop
    movzx ecx, byte ptr [rbx + r12]   // high byte
    stk_pop
    movzx edx, byte ptr [rbx + r12]   // low byte
    stk_pop
    mov byte ptr [r15 + rax], dl
    inc eax
    and eax, 0xffff
    mov byte ptr [r15 + rax], cl
    next

_LDR2:
    movsx rax, byte ptr [rbx + r12]
    lea rax, [rbp + rax]
    and rax, 0xffff
    movzx ecx, byte ptr [r15 + rax]
    mov byte ptr [rbx + r12], cl
    inc rax
    and rax, 0xffff
    movzx ecx, byte ptr [r15 + rax]
    stk_push cl
    next

_STR2:
    movsx rax, byte ptr [rbx + r12]   // signed offset
    stk_pop
    movzx ecx, byte ptr [rbx + r12]   // high value byte
    stk_pop
    movzx edx, byte ptr [rbx + r12]   // low value byte
    stk_pop
    lea rax, [rbp + rax]
    and rax, 0xffff
    mov byte ptr [r15 + rax], dl
    inc rax
    and rax, 0xffff
    mov byte ptr [r15 + rax], cl
    next

_LDA2:
    peek ecx, 1                        // addr high (peek first; rax clobbered)
    movzx eax, byte ptr [rbx + r12]   // addr low (loaded after peek)
    shl ecx, 8
    or eax, ecx
    movzx ecx, byte ptr [r15 + rax]
    mov byte ptr [rbx + r11], cl
    inc eax
    and eax, 0xffff
    movzx ecx, byte ptr [r15 + rax]
    mov byte ptr [rbx + r12], cl
    next

_STA2:
    movzx eax, byte ptr [rbx + r12]   // addr low
    stk_pop
    movzx ecx, byte ptr [rbx + r12]   // addr high
    stk_pop
    shl ecx, 8
    or eax, ecx
    movzx ecx, byte ptr [rbx + r12]   // low value
    stk_pop
    movzx edx, byte ptr [rbx + r12]   // high value
    stk_pop
    mov byte ptr [r15 + rax], dl
    inc eax
    and eax, 0xffff
    mov byte ptr [r15 + rax], cl
    next

_DEI2:
    precall
    CALL dei_2_entry
    postcall
    next

_DEO2:
    precall
    CALL deo_2_entry
    postcall
    next

.macro binary_op2 insn
    movzx eax, byte ptr [rbx + r12]
    stk_pop
    movzx ecx, byte ptr [rbx + r12]
    stk_pop
    shl ecx, 8
    or eax, ecx                        // b (top short)

    movzx ecx, byte ptr [rbx + r12]
    peek edx, 1
    shl edx, 8
    or ecx, edx                        // a (second short)

    \insn ecx, eax                     // result in ecx
    mov byte ptr [rbx + r12], cl      // store result_hi at current pos
    shr ecx, 8
    mov byte ptr [rbx + r11], cl      // store result_lo at current pos
    next
.endm

_ADD2:
    binary_op2 add

_SUB2:
    binary_op2 sub

_MUL2:
    binary_op2 imul

_DIV2:
    movzx eax, byte ptr [rbx + r12]
    stk_pop
    movzx ecx, byte ptr [rbx + r12]
    stk_pop
    shl ecx, 8
    or eax, ecx                        // b (divisor, top short)

    movzx ecx, byte ptr [rbx + r12]
    stk_pop
    movzx edx, byte ptr [rbx + r12]
    shl edx, 8
    or ecx, edx                        // a (dividend, second short)

    // 16-bit unsigned divide: a / b
    push rax                           // save divisor (b) onto x86 stack
    mov eax, ecx                       // dividend (a) in eax
    movzx eax, ax
    xor edx, edx
    pop rcx                            // restore divisor into ecx
    movzx ecx, cx
    test cx, cx
    jz 1f
    div cx                             // ax = a / b
    jmp 2f
1:
    xor eax, eax                       // div by zero → 0
2:
    movzx r8d, al                      // save result_lo
    shr eax, 8
    mov byte ptr [rbx + r12], al      // store result_hi at current pos
    stk_push r8b                       // push result_lo on top
    next

_AND2:
    binary_op2 and

_ORA2:
    binary_op2 or

_EOR2:
    binary_op2 xor

_SFT2:
    movzx eax, byte ptr [rbx + r12]   // shift amount
    stk_pop
    movzx r8d, byte ptr [rbx + r12]   // value_lo in r8d
    stk_pop
    movzx edx, byte ptr [rbx + r12]   // value_hi in edx
    shl edx, 8
    or r8d, edx                        // value (16-bit) in r8d

    mov ecx, eax
    and ecx, 0xf                       // right shift count in cl
    shr r8d, cl
    shr eax, 4                         // left shift count
    mov ecx, eax
    shl r8d, cl                        // result in r8d

    mov edx, r8d
    shr edx, 8
    mov byte ptr [rbx + r12], dl       // result_hi at current pos
    stk_push r8b                       // result_lo becomes top
    next

_JMI:
    movzx eax, byte ptr [r15 + rbp]
    inc rbp
    and rbp, 0xffff
    movzx ecx, byte ptr [r15 + rbp]
    inc rbp
    and rbp, 0xffff
    shl eax, 8
    or eax, ecx                        // 16-bit offset
    movsx rax, ax                      // sign-extend
    add rbp, rax
    and rbp, 0xffff
    next

// ============================================================
// r-mode variants: swap rbx<->r13 and r12<->r14 for stack ops
// ============================================================

_INCr:
    movzx eax, byte ptr [r13 + r14]
    inc al
    mov byte ptr [r13 + r14], al
    next

_POPr:
    rpop
    next

_NIPr:
    movzx eax, byte ptr [r13 + r14]
    rpop
    mov byte ptr [r13 + r14], al
    next

_SWPr:
    rpeek ecx, 1                       // a (rpeek first; rax clobbered)
    movzx eax, byte ptr [r13 + r14]   // b (loaded after rpeek)
    mov byte ptr [r13 + r14], cl      // store a at top
    mov byte ptr [r13 + r11], al      // store b at second
    next

_ROTr:
    rpeek_ ecx, 1, r10                // b → ecx (rpeek first; rax clobbered)
    movzx r8d, byte ptr [r13 + r14]   // c → eax
    rpeek edx, 2                      // a → edx (clobbers rax; ecx=b, r8d=c still valid)
    mov byte ptr [r13 + r14], dl      // top = a
    mov byte ptr [r13 + r10], r8b     // second = c
    mov byte ptr [r13 + r11], cl      // third = b
    next

_DUPr:
    movzx eax, byte ptr [r13 + r14]
    rpush al
    next

_OVRr:
    rpeek eax, 1
    rpush al
    next

.macro compare_opr setcc_op
    movzx eax, byte ptr [r13 + r14]
    rpop
    movzx ecx, byte ptr [r13 + r14]
    cmp ecx, eax
    \setcc_op al
    mov byte ptr [r13 + r14], al
    next
.endm

_EQUr:
    compare_opr sete

_NEQr:
    compare_opr setne

_GTHr:
    compare_opr seta

_LTHr:
    compare_opr setb

_JMPr:
    movsx rax, byte ptr [r13 + r14]
    rpop
    add bp, ax
    next

_JCNr:
    movsx eax, byte ptr [r13 + r14]
    rpop
    movzx ecx, byte ptr [r13 + r14]
    rpop
    test ecx, ecx
    jz 1f
    add bp, ax
1:
    next

_JSRr:
    movsx eax, byte ptr [r13 + r14]
    rpop
    mov ecx, ebp
    shr ecx, 8
    stk_push cl
    stk_push bpl
    add bp, ax
    next

_STHr:
    movzx eax, byte ptr [r13 + r14]
    rpop
    stk_push al
    next

_LDZr:
    movzx eax, byte ptr [r13 + r14]
    rpop
    movzx eax, byte ptr [r15 + rax]
    rpush al
    next

_STZr:
    movzx eax, byte ptr [r13 + r14]
    rpop
    movzx ecx, byte ptr [r13 + r14]
    rpop
    mov byte ptr [r15 + rax], cl
    next

_LDRr:
    movsx rax, byte ptr [r13 + r14]
    lea rax, [rbp + rax]
    and rax, 0xffff
    movzx eax, byte ptr [r15 + rax]
    mov byte ptr [r13 + r14], al
    next

_STRr:
    movsx rax, byte ptr [r13 + r14]
    rpop
    movzx ecx, byte ptr [r13 + r14]
    rpop
    lea rax, [rbp + rax]
    and rax, 0xffff
    mov byte ptr [r15 + rax], cl
    next

_LDAr:
    movzx eax, byte ptr [r13 + r14]
    rpop
    movzx ecx, byte ptr [r13 + r14]
    shl ecx, 8
    or eax, ecx
    movzx eax, byte ptr [r15 + rax]
    mov byte ptr [r13 + r14], al
    next

_STAr:
    movzx eax, byte ptr [r13 + r14]
    rpop
    movzx ecx, byte ptr [r13 + r14]
    rpop
    shl ecx, 8
    or eax, ecx
    movzx edx, byte ptr [r13 + r14]
    rpop
    mov byte ptr [r15 + rax], dl
    next

_DEIr:
    precall
    CALL dei_r_entry
    postcall
    next

_DEOr:
    precall
    CALL deo_r_entry
    postcall
    next

.macro binary_opr insn
    movzx eax, byte ptr [r13 + r14]
    rpop
    movzx ecx, byte ptr [r13 + r14]
    \insn ecx, eax
    mov byte ptr [r13 + r14], cl
    next
.endm

_ADDr:
    binary_opr add

_SUBr:
    binary_opr sub

_MULr:
    binary_opr imul

_DIVr:
    movzx ecx, byte ptr [r13 + r14]   // b (divisor), top
    rpop
    movzx eax, byte ptr [r13 + r14]   // a (dividend), second
    movzx eax, al
    test cl, cl
    jz 1f
    div cl
    jmp 2f
1:
    xor eax, eax
2:
    mov byte ptr [r13 + r14], al
    next

_ANDr:
    binary_opr and

_ORAr:
    binary_opr or

_EORr:
    binary_opr xor

_SFTr:
    movzx eax, byte ptr [r13 + r14]   // shift amount
    rpop
    movzx edx, byte ptr [r13 + r14]   // value in dl
    mov ecx, eax
    and ecx, 0xf                       // right shift count in cl
    shr dl, cl
    shr eax, 4                         // left shift count
    mov ecx, eax
    shl dl, cl
    mov byte ptr [r13 + r14], dl
    next

_JSI:
    movzx eax, byte ptr [r15 + rbp]
    inc rbp
    and rbp, 0xffff
    movzx ecx, byte ptr [r15 + rbp]
    inc rbp
    and rbp, 0xffff
    shl eax, 8
    or eax, ecx                        // 16-bit offset
    mov edx, ebp
    shr edx, 8
    rpush dl
    rpush bpl
    movsx rax, ax
    add rbp, rax
    and rbp, 0xffff
    next

_INC2r:
    rpeek ecx, 1                      // high byte (rpeek first; r11 is addr)
    movzx eax, byte ptr [r13 + r14]   // low byte (loaded after rpeek)
    shl ecx, 8
    or eax, ecx
    inc eax
    and eax, 0xffff
    mov byte ptr [r13 + r14], al
    shr eax, 8
    mov byte ptr [r13 + r11], al
    next

_POP2r:
    sub r14, 2
    and r14, 0xff
    next

_NIP2r:
    movzx eax, byte ptr [r13 + r14]   // b_lo (top)
    rpop
    movzx ecx, byte ptr [r13 + r14]   // b_hi (second)
    rpop
    mov byte ptr [r13 + r14], al      // b_lo at new top
    lea rdx, [r14 - 1]
    and rdx, 0xff
    mov byte ptr [r13 + rdx], cl      // b_hi below
    next

_SWP2r:
    rpeek ecx, 2                       // a_lo (rpeek first; rax clobbered)
    movzx eax, byte ptr [r13 + r14]   // b_lo (loaded after rpeek)
    mov byte ptr [r13 + r14], cl      // store a_lo at b_lo's position
    mov byte ptr [r13 + r11], al      // store b_lo at a_lo's position

    rpeek_ ecx, 3, r10                       // a_hi (rpeek first; rax clobbered)
    rpeek eax, 1                       // b_hi (rpeek second; ecx=a_hi still valid)
    mov byte ptr [r13 + r11], cl      // store a_hi at b_hi's position
    mov byte ptr [r13 + r10], al      // store b_hi at a_hi's position
    next

_ROT2r:
    rpeek_ ecx, 2, r10                       // b_lo (rpeek first; rax clobbered)
    movzx eax, byte ptr [r13 + r14]   // c_lo (loaded after rpeek)
    movzx r8d, al                      // save c_lo (next rpeek clobbers eax)
    rpeek edx, 4                       // a_lo
    mov byte ptr [r13 + r14], dl      // store a_lo at top
    mov byte ptr [r13 + r10], r8b     // store c_lo
    mov byte ptr [r13 + r11], cl      // store b_lo

    rpeek_ r8d, 1, r10                       // c_hi (rpeek into eax first)
    rpeek_ ecx, 3, r9                       // b_hi (clobbers rax; r8d=c_hi valid)
    rpeek edx, 5                       // a_hi
    mov byte ptr [r13 + r10], dl      // store a_hi
    mov byte ptr [r13 + r9], r8b     // store c_hi
    mov byte ptr [r13 + r11], cl      // store b_hi
    next

_DUP2r:
    rpeek ecx, 1                       // hi (rpeek first; rax clobbered)
    movzx eax, byte ptr [r13 + r14]   // lo (loaded after rpeek)
    rpush cl                           // push hi
    rpush al                           // push lo
    next

_OVR2r:
    rpeek ecx, 3                       // a_hi (rpeek first; rax clobbered)
    rpeek eax, 2                       // a_lo (rpeek second; ecx=a_hi still valid)
    rpush cl                           // push a_hi
    rpush al                           // push a_lo
    next

.macro compare_op2r setcc_op
    movzx eax, byte ptr [r13 + r14]
    rpop
    movzx ecx, byte ptr [r13 + r14]
    rpop
    shl ecx, 8
    or eax, ecx
    movzx ecx, byte ptr [r13 + r14]
    rpop
    movzx edx, byte ptr [r13 + r14]
    shl edx, 8
    or ecx, edx
    cmp ecx, eax
    \setcc_op al
    movzx eax, al
    mov byte ptr [r13 + r14], al
    next
.endm

_EQU2r:
    compare_op2r sete

_NEQ2r:
    compare_op2r setne

_GTH2r:
    compare_op2r seta

_LTH2r:
    compare_op2r setb

_JMP2r:
    movzx eax, byte ptr [r13 + r14]
    rpop
    movzx ecx, byte ptr [r13 + r14]
    rpop
    shl ecx, 8
    or eax, ecx
    mov rbp, rax
    next

_JCN2r:
    movzx eax, byte ptr [r13 + r14]
    rpop
    movzx ecx, byte ptr [r13 + r14]
    rpop
    movzx edx, byte ptr [r13 + r14]
    rpop
    shl ecx, 8
    or eax, ecx
    test edx, edx
    jz 1f
    mov rbp, rax
1:
    next

_JSR2r:
    movzx eax, byte ptr [r13 + r14]
    rpop
    movzx ecx, byte ptr [r13 + r14]
    rpop
    shl ecx, 8
    or eax, ecx
    mov edx, ebp
    shr edx, 8
    stk_push dl
    stk_push bpl
    mov rbp, rax
    next

_STH2r:
    movzx eax, byte ptr [r13 + r14]
    rpop
    movzx ecx, byte ptr [r13 + r14]
    rpop
    stk_push cl
    stk_push al
    next

_LDZ2r:
    movzx eax, byte ptr [r13 + r14]
    rpop
    movzx ecx, byte ptr [r15 + rax]
    rpush cl
    inc eax
    and eax, 0xffff
    movzx ecx, byte ptr [r15 + rax]
    rpush cl
    next

_STZ2r:
    movzx eax, byte ptr [r13 + r14]
    rpop
    movzx ecx, byte ptr [r13 + r14]
    rpop
    movzx edx, byte ptr [r13 + r14]
    rpop
    mov byte ptr [r15 + rax], dl
    inc eax
    and eax, 0xffff
    mov byte ptr [r15 + rax], cl
    next

_LDR2r:
    movsx rax, byte ptr [r13 + r14]
    lea rax, [rbp + rax]
    and rax, 0xffff
    movzx ecx, byte ptr [r15 + rax]
    mov byte ptr [r13 + r14], cl
    inc rax
    and rax, 0xffff
    movzx ecx, byte ptr [r15 + rax]
    rpush cl
    next

_STR2r:
    movsx rax, byte ptr [r13 + r14]
    rpop
    movzx ecx, byte ptr [r13 + r14]
    rpop
    movzx edx, byte ptr [r13 + r14]
    rpop
    lea rax, [rbp + rax]
    and rax, 0xffff
    mov byte ptr [r15 + rax], dl
    inc rax
    and rax, 0xffff
    mov byte ptr [r15 + rax], cl
    next

_LDA2r:
    movzx eax, byte ptr [r13 + r14]
    rpop
    movzx ecx, byte ptr [r13 + r14]
    shl ecx, 8
    or eax, ecx
    movzx ecx, byte ptr [r15 + rax]
    mov byte ptr [r13 + r14], cl
    inc eax
    and eax, 0xffff
    movzx ecx, byte ptr [r15 + rax]
    rpush cl
    next

_STA2r:
    movzx eax, byte ptr [r13 + r14]
    rpop
    movzx ecx, byte ptr [r13 + r14]
    rpop
    shl ecx, 8
    or eax, ecx
    movzx ecx, byte ptr [r13 + r14]
    rpop
    movzx edx, byte ptr [r13 + r14]
    rpop
    mov byte ptr [r15 + rax], dl
    inc eax
    and eax, 0xffff
    mov byte ptr [r15 + rax], cl
    next

_DEI2r:
    precall
    CALL dei_2r_entry
    postcall
    next

_DEO2r:
    precall
    CALL deo_2r_entry
    postcall
    next

.macro binary_op2r insn
    movzx eax, byte ptr [r13 + r14]
    rpop
    movzx ecx, byte ptr [r13 + r14]
    rpop
    shl ecx, 8
    or eax, ecx                        // b

    movzx ecx, byte ptr [r13 + r14]
    rpop
    movzx edx, byte ptr [r13 + r14]
    shl edx, 8
    or ecx, edx                        // a

    \insn ecx, eax                     // result in ecx
    movzx r8d, cl                      // save result_lo
    shr ecx, 8
    mov byte ptr [r13 + r14], cl      // store result_hi at current pos
    rpush r8b                          // push result_lo on top
    next
.endm

_ADD2r:
    binary_op2r add

_SUB2r:
    binary_op2r sub

_MUL2r:
    binary_op2r imul

_DIV2r:
    movzx eax, byte ptr [r13 + r14]
    rpop
    movzx ecx, byte ptr [r13 + r14]
    rpop
    shl ecx, 8
    or eax, ecx                        // b (divisor)

    movzx ecx, byte ptr [r13 + r14]
    rpop
    movzx edx, byte ptr [r13 + r14]
    shl edx, 8
    or ecx, edx                        // a (dividend)

    push rax
    mov eax, ecx
    movzx eax, ax
    xor edx, edx
    pop rcx
    movzx ecx, cx
    test cx, cx
    jz 1f
    div cx                             // ax = a / b
    jmp 2f
1:
    xor eax, eax
2:
    movzx r8d, al                      // save result_lo
    shr eax, 8
    mov byte ptr [r13 + r14], al      // store result_hi at current pos
    rpush r8b                          // push result_lo on top
    next

_AND2r:
    binary_op2r and

_ORA2r:
    binary_op2r or

_EOR2r:
    binary_op2r xor

_SFT2r:
    movzx eax, byte ptr [r13 + r14]   // shift amount
    rpop
    movzx r8d, byte ptr [r13 + r14]   // value_lo in r8d
    rpop
    movzx edx, byte ptr [r13 + r14]   // value_hi in edx
    shl edx, 8
    or r8d, edx                        // value (16-bit) in r8d

    mov ecx, eax
    and ecx, 0xf                       // right shift count in cl
    shr r8d, cl
    shr eax, 4                         // left shift count
    mov ecx, eax
    shl r8d, cl                        // result in r8d

    mov edx, r8d
    shr edx, 8
    mov byte ptr [r13 + r14], dl       // result_hi at current pos
    rpush r8b                          // result_lo becomes top
    next

// ============================================================
// k-mode (keep) variants
// ============================================================

_LIT:
    movzx eax, byte ptr [r15 + rbp]
    inc rbp
    and rbp, 0xffff
    stk_push al
    next

_INCk:
    movzx eax, byte ptr [rbx + r12]
    inc al
    stk_push al
    next

_POPk:
    next

_NIPk:
    movzx eax, byte ptr [rbx + r12]
    stk_push al
    next

_SWPk:
    peek ecx, 1                        // a (peek first; rax clobbered)
    movzx eax, byte ptr [rbx + r12]   // b (loaded after peek)
    stk_push al                        // push b
    stk_push cl                        // push a (now a is on top)
    next

_ROTk:
    // a b c -- a b c b c a  (push b, c, a)
    peek ecx, 1                        // b (peek first; rax clobbered)
    movzx eax, byte ptr [rbx + r12]   // c (loaded after peek)
    stk_push cl                        // push b
    stk_push al                        // push c
    peek eax, 4                        // a (r12 now +2, so peek 4 = orig peek 2 = a)
    stk_push al                        // push a (on top)
    next

_DUPk:
    movzx eax, byte ptr [rbx + r12]
    stk_push al
    stk_push al
    next

_OVRk:
    // a b -- a b a b a  (push a, b, a)
    peek ecx, 1                        // a (peek first; rax clobbered)
    movzx eax, byte ptr [rbx + r12]   // b (loaded after peek)
    stk_push cl                        // push a
    stk_push al                        // push b
    stk_push cl                        // push a again
    next

.macro compare_opk setcc_op
    peek ecx, 1                        // a (peek first; rax clobbered)
    movzx eax, byte ptr [rbx + r12]   // b (loaded after peek)
    cmp ecx, eax                       // a vs b
    \setcc_op al
    movzx eax, al
    stk_push al
    next
.endm

_EQUk:
    compare_opk sete

_NEQk:
    compare_opk setne

_GTHk:
    compare_opk seta

_LTHk:
    compare_opk setb

_JMPk:
    movsx rax, byte ptr [rbx + r12]
    add bp, ax
    next

_JCNk:
    peek ecx, 1                        // condition (peek first; rax clobbered)
    movsx eax, byte ptr [rbx + r12]   // offset (signed, loaded after peek)
    test ecx, ecx
    jz 1f
    add bp, ax
1:
    next

_JSRk:
    movsx eax, byte ptr [rbx + r12]
    mov ecx, ebp
    shr ecx, 8
    rpush cl
    rpush bpl
    add bp, ax
    next

_STHk:
    movzx eax, byte ptr [rbx + r12]
    rpush al
    next

_LDZk:
    movzx eax, byte ptr [rbx + r12]
    movzx eax, byte ptr [r15 + rax]
    stk_push al
    next

_STZk:
    peek ecx, 1                        // val (peek first; rax clobbered)
    movzx eax, byte ptr [rbx + r12]   // addr (loaded after peek)
    mov byte ptr [r15 + rax], cl      // store val at addr
    next

_LDRk:
    movsx rax, byte ptr [rbx + r12]
    lea rax, [rbp + rax]
    and rax, 0xffff
    movzx eax, byte ptr [r15 + rax]
    stk_push al
    next

_STRk:
    peek ecx, 1                        // val (peek first; rax clobbered)
    movsx rax, byte ptr [rbx + r12]   // offset (signed, loaded after peek)
    lea rax, [rbp + rax]
    and rax, 0xffff
    mov byte ptr [r15 + rax], cl
    next

_LDAk:
    peek ecx, 1                        // addr_hi (peek first; rax clobbered)
    movzx eax, byte ptr [rbx + r12]   // addr_lo (loaded after peek)
    shl ecx, 8
    or eax, ecx
    movzx eax, byte ptr [r15 + rax]
    stk_push al
    next

_STAk:
    peek ecx, 1                        // addr_hi (peek first; rax clobbered)
    movzx eax, byte ptr [rbx + r12]   // addr_lo (loaded after peek)
    shl ecx, 8
    or eax, ecx                        // full addr in eax
    mov r8d, eax                       // save addr (next peek clobbers rax)
    peek ecx, 2                        // val (clobbers rax)
    mov byte ptr [r15 + r8], cl       // store val at addr
    next

_DEIk:
    precall
    CALL dei_k_entry
    postcall
    next

_DEOk:
    precall
    CALL deo_k_entry
    postcall
    next

.macro binary_opk insn
    peek ecx, 1                        // a (peek first; rax clobbered but that's ok)
    movzx eax, byte ptr [rbx + r12]   // b (loaded after peek, not via rax)
    \insn ecx, eax                     // a OP b
    stk_push cl
    next
.endm

_ADDk:
    binary_opk add

_SUBk:
    binary_opk sub

_MULk:
    binary_opk imul

_DIVk:
    peek eax, 1                        // a (dividend; peek first, rax clobbered)
    movzx ecx, byte ptr [rbx + r12]   // b (divisor; loaded after peek)
    test cl, cl
    jz 1f
    div cl
    jmp 2f
1:
    xor eax, eax
2:
    stk_push al
    next

_ANDk:
    binary_opk and

_ORAk:
    binary_opk or

_EORk:
    binary_opk xor

_SFTk:
    movzx r9d, byte ptr [rbx + r12]   // shift amount in r9d (peek clobbers rax)
    peek edx, 1                        // value in dl
    mov ecx, r9d
    and ecx, 0xf                       // right shift count in cl
    shr dl, cl
    shr r9d, 4                         // left shift count
    mov ecx, r9d
    shl dl, cl
    stk_push dl
    next

_LIT2:
    movzx eax, byte ptr [r15 + rbp]
    inc rbp
    and rbp, 0xffff
    stk_push al
    movzx eax, byte ptr [r15 + rbp]
    inc rbp
    and rbp, 0xffff
    stk_push al
    next

_INC2k:
    peek ecx, 1                        // high byte (peek first; rax clobbered)
    movzx eax, byte ptr [rbx + r12]   // low byte (loaded after peek)
    shl ecx, 8
    or eax, ecx
    inc eax
    and eax, 0xffff
    // push high then low (stack grows upward, push increments first)
    mov ecx, eax
    shr ecx, 8
    stk_push cl
    stk_push al
    next

_POP2k:
    next

_NIP2k:
    peek ecx, 1                        // b_hi (peek first; rax clobbered)
    movzx eax, byte ptr [rbx + r12]   // b_lo (loaded after peek)
    stk_push cl                        // push b_hi
    stk_push al                        // push b_lo
    next

_SWP2k:
    peek eax, 1
    stk_push al
    peek eax, 1
    stk_push al
    peek eax, 5
    stk_push al
    peek eax, 5
    stk_push al
    next

_ROT2k:
    peek eax, 3
    stk_push al
    peek eax, 3
    stk_push al
    peek eax, 3
    stk_push al
    peek eax, 3
    stk_push al
    peek eax, 9
    stk_push al
    peek eax, 9
    stk_push al
    next

_DUP2k:
    peek ecx, 1                        // hi (peek first; rax clobbered)
    movzx eax, byte ptr [rbx + r12]   // lo (loaded after peek)
    stk_push cl                        // push hi (1st copy)
    stk_push al                        // push lo (1st copy)
    stk_push cl                        // push hi (2nd copy)
    stk_push al                        // push lo (2nd copy)
    next

_OVR2k:
    // a b -- a b a b a  (6 new pushes: a_hi, a_lo, b_hi, b_lo, a_hi, a_lo)
    peek ecx, 1                        // b_hi (peek first; rax clobbered)
    peek edx, 2                        // a_lo (clobbers rax; ecx=b_hi fine)
    peek esi, 3                        // a_hi (clobbers rax; ecx,edx fine)
    movzx eax, byte ptr [rbx + r12]   // b_lo (loaded last, after all peeks)
    stk_push sil                       // push a_hi
    stk_push dl                        // push a_lo
    stk_push cl                        // push b_hi
    stk_push al                        // push b_lo
    stk_push sil                       // push a_hi
    stk_push dl                        // push a_lo
    next

.macro compare_op2k setcc_op
    peek ecx, 1                        // b_hi (peek first; rax clobbered)
    movzx eax, byte ptr [rbx + r12]   // b_lo (loaded after peek)
    shl ecx, 8
    or eax, ecx                        // b
    mov r8d, eax                       // save b (next peeks clobber rax)
    peek ecx, 2                        // a_lo
    peek edx, 3                        // a_hi
    shl edx, 8
    or ecx, edx                        // a
    cmp ecx, r8d                       // a vs b
    \setcc_op al
    movzx eax, al
    stk_push al
    next
.endm

_EQU2k:
    compare_op2k sete

_NEQ2k:
    compare_op2k setne

_GTH2k:
    compare_op2k seta

_LTH2k:
    compare_op2k setb

_JMP2k:
    peek ecx, 1                        // hi (peek first; rax clobbered)
    movzx eax, byte ptr [rbx + r12]   // lo (loaded after peek)
    shl ecx, 8
    or eax, ecx                        // addr
    mov rbp, rax
    next

_JCN2k:
    peek ecx, 1                        // addr_hi (peek first; rax clobbered)
    movzx eax, byte ptr [rbx + r12]   // addr_lo (loaded after peek)
    shl ecx, 8
    or eax, ecx                        // addr in eax
    peek edx, 2                        // condition
    test edx, edx
    jz 1f
    mov rbp, rax
1:
    next

_JSR2k:
    peek ecx, 1                        // hi (peek first; rax clobbered)
    movzx eax, byte ptr [rbx + r12]   // lo (loaded after peek)
    shl ecx, 8
    or eax, ecx                        // addr
    mov edx, ebp
    shr edx, 8
    rpush dl
    rpush bpl
    mov rbp, rax
    next

_STH2k:
    peek ecx, 1                        // hi (peek first; rax clobbered)
    movzx eax, byte ptr [rbx + r12]   // lo (loaded after peek)
    rpush cl                           // push hi
    rpush al                           // push lo
    next

_LDZ2k:
    movzx eax, byte ptr [rbx + r12]
    movzx ecx, byte ptr [r15 + rax]
    stk_push cl
    inc eax
    and eax, 0xffff
    movzx ecx, byte ptr [r15 + rax]
    stk_push cl
    next

_STZ2k:
    peek ecx, 1                        // val_lo (peek first; rax clobbered)
    peek edx, 2                        // val_hi (clobbers rax; ecx=val_lo fine)
    movzx eax, byte ptr [rbx + r12]   // addr (loaded after peeks)
    mov byte ptr [r15 + rax], dl      // store val_hi at addr
    inc eax
    and eax, 0xffff
    mov byte ptr [r15 + rax], cl      // store val_lo at addr+1
    next

_LDR2k:
    movsx rax, byte ptr [rbx + r12]
    lea rax, [rbp + rax]
    and rax, 0xffff
    movzx ecx, byte ptr [r15 + rax]
    stk_push cl
    inc rax
    and rax, 0xffff
    movzx ecx, byte ptr [r15 + rax]
    stk_push cl
    next

_STR2k:
    peek ecx, 1                        // val_lo (peek first; rax clobbered)
    peek edx, 2                        // val_hi (clobbers rax; ecx=val_lo fine)
    movsx rax, byte ptr [rbx + r12]   // offset (signed, loaded after peeks)
    lea rax, [rbp + rax]
    and rax, 0xffff
    mov byte ptr [r15 + rax], dl
    inc rax
    and rax, 0xffff
    mov byte ptr [r15 + rax], cl
    next

_LDA2k:
    peek ecx, 1                        // addr_hi (peek first; rax clobbered)
    movzx eax, byte ptr [rbx + r12]   // addr_lo (loaded after peek)
    shl ecx, 8
    or eax, ecx
    movzx ecx, byte ptr [r15 + rax]
    stk_push cl
    inc eax
    and eax, 0xffff
    movzx ecx, byte ptr [r15 + rax]
    stk_push cl
    next

_STA2k:
    peek ecx, 1                        // addr_hi (peek first; rax clobbered)
    movzx eax, byte ptr [rbx + r12]   // addr_lo (loaded after peek)
    shl ecx, 8
    or eax, ecx                        // full addr in eax
    mov r8d, eax                       // save addr (next peeks clobber rax)
    peek ecx, 2                        // val_lo (clobbers rax; r8d=addr fine)
    peek edx, 3                        // val_hi (clobbers rax; ecx=val_lo fine)
    mov byte ptr [r15 + r8], dl       // store val_hi at addr
    inc r8d
    and r8d, 0xffff
    mov byte ptr [r15 + r8], cl       // store val_lo at addr+1
    next

_DEI2k:
    precall
    CALL dei_2k_entry
    postcall
    next

_DEO2k:
    precall
    CALL deo_2k_entry
    postcall
    next

.macro binary_op2k insn
    peek ecx, 1                        // b_hi (peek first; rax clobbered)
    movzx eax, byte ptr [rbx + r12]   // b_lo (loaded after peek)
    shl ecx, 8
    or eax, ecx                        // b
    mov r8d, eax                       // save b (next peeks clobber rax/eax)

    peek ecx, 2                        // a_lo
    peek edx, 3                        // a_hi
    shl edx, 8
    or ecx, edx                        // a

    \insn ecx, r8d                     // a OP b → ecx
    movzx r8d, cl                      // save result_lo
    shr ecx, 8
    stk_push cl                        // push result_hi first
    stk_push r8b                       // push result_lo on top
    next
.endm

_ADD2k:
    binary_op2k add

_SUB2k:
    binary_op2k sub

_MUL2k:
    binary_op2k imul

_DIV2k:
    peek ecx, 1                        // b_hi (peek first; rax clobbered)
    movzx eax, byte ptr [rbx + r12]   // b_lo (loaded after peek)
    shl ecx, 8
    or eax, ecx                        // b
    mov r8d, eax                       // save b (next peeks clobber rax)

    peek ecx, 2                        // a_lo
    peek edx, 3                        // a_hi
    shl edx, 8
    or ecx, edx                        // a

    push r8                            // save b (divisor) onto x86 stack
    mov eax, ecx                       // dividend in eax
    movzx eax, ax
    xor edx, edx
    pop rcx                            // restore divisor into ecx
    movzx ecx, cx
    test cx, cx
    jz 1f
    div cx                             // ax = a / b
    jmp 2f
1:
    xor eax, eax
2:
    movzx r8d, al                      // save result_lo
    shr eax, 8
    stk_push al                        // push result_hi first
    stk_push r8b                       // push result_lo on top
    next

_AND2k:
    binary_op2k and

_ORA2k:
    binary_op2k or

_EOR2k:
    binary_op2k xor

_SFT2k:
    movzx r9d, byte ptr [rbx + r12]   // shift amount in r9d (peek clobbers rax)
    peek r8d, 1                        // value_lo in r8d
    peek edx, 2                        // value_hi in edx
    shl edx, 8
    or r8d, edx                        // value (16-bit) in r8d

    mov ecx, r9d
    and ecx, 0xf                       // right shift count in cl
    shr r8d, cl
    shr r9d, 4                         // left shift count
    mov ecx, r9d
    shl r8d, cl                        // result in r8d

    mov edx, r8d
    shr edx, 8
    stk_push dl                        // push result_hi first
    stk_push r8b                       // push result_lo (becomes top)
    next

// ============================================================
// kr-mode variants
// ============================================================

_LITr:
    movzx eax, byte ptr [r15 + rbp]
    inc rbp
    and rbp, 0xffff
    rpush al
    next

_INCkr:
    movzx eax, byte ptr [r13 + r14]
    inc al
    rpush al
    next

_POPkr:
    next

_NIPkr:
    movzx eax, byte ptr [r13 + r14]
    rpush al
    next

_SWPkr:
    rpeek ecx, 1                       // a (rpeek first; rax clobbered)
    movzx eax, byte ptr [r13 + r14]   // b (loaded after rpeek)
    rpush al                           // push b
    rpush cl                           // push a (now a is on top)
    next

_ROTkr:
    // a b c -- a b c b c a (push b, c, a)
    rpeek ecx, 1                       // b (rpeek first; rax clobbered)
    movzx eax, byte ptr [r13 + r14]   // c (loaded after rpeek)
    rpush cl                           // push b
    rpush al                           // push c
    rpeek eax, 4                       // a (r14 now +2, so rpeek 4 = orig rpeek 2 = a)
    rpush al                           // push a (on top)
    next

_DUPkr:
    movzx eax, byte ptr [r13 + r14]
    rpush al
    rpush al
    next

_OVRkr:
    // a b -- a b a b a  (push a, b, a)
    rpeek ecx, 1                       // a (rpeek first; rax clobbered)
    movzx eax, byte ptr [r13 + r14]   // b (loaded after rpeek)
    rpush cl                           // push a
    rpush al                           // push b
    rpush cl                           // push a again
    next

.macro compare_opkr setcc_op
    rpeek ecx, 1                       // a (rpeek first; rax clobbered)
    movzx eax, byte ptr [r13 + r14]   // b (loaded after rpeek)
    cmp ecx, eax                       // a vs b
    \setcc_op al
    movzx eax, al
    rpush al
    next
.endm

_EQUkr:
    compare_opkr sete

_NEQkr:
    compare_opkr setne

_GTHkr:
    compare_opkr seta

_LTHkr:
    compare_opkr setb

_JMPkr:
    movsx rax, byte ptr [r13 + r14]
    add rbp, rax
    and rbp, 0xffff
    next

_JCNkr:
    rpeek ecx, 1                       // condition (rpeek first; rax clobbered)
    movsx eax, byte ptr [r13 + r14]   // offset (signed, loaded after rpeek)
    test ecx, ecx
    jz 1f
    add rbp, rax
    and rbp, 0xffff
1:
    next

_JSRkr:
    movsx eax, byte ptr [r13 + r14]
    mov ecx, ebp
    shr ecx, 8
    stk_push cl
    stk_push bpl
    add rbp, rax
    and rbp, 0xffff
    next

_STHkr:
    movzx eax, byte ptr [r13 + r14]
    stk_push al
    next

_LDZkr:
    movzx eax, byte ptr [r13 + r14]
    movzx eax, byte ptr [r15 + rax]
    rpush al
    next

_STZkr:
    rpeek ecx, 1                       // val (rpeek first; rax clobbered)
    movzx eax, byte ptr [r13 + r14]   // addr (loaded after rpeek)
    mov byte ptr [r15 + rax], cl
    next

_LDRkr:
    movsx rax, byte ptr [r13 + r14]
    lea rax, [rbp + rax]
    and rax, 0xffff
    movzx eax, byte ptr [r15 + rax]
    rpush al
    next

_STRkr:
    rpeek ecx, 1                       // val (rpeek first; rax clobbered)
    movsx rax, byte ptr [r13 + r14]   // offset (signed, loaded after rpeek)
    lea rax, [rbp + rax]
    and rax, 0xffff
    mov byte ptr [r15 + rax], cl
    next

_LDAkr:
    rpeek ecx, 1                       // addr_hi (rpeek first; rax clobbered)
    movzx eax, byte ptr [r13 + r14]   // addr_lo (loaded after rpeek)
    shl ecx, 8
    or eax, ecx                        // full addr
    movzx eax, byte ptr [r15 + rax]
    rpush al
    next

_STAkr:
    rpeek ecx, 1                       // addr_hi (rpeek first; rax clobbered)
    movzx eax, byte ptr [r13 + r14]   // addr_lo (loaded after rpeek)
    shl ecx, 8
    or eax, ecx                        // full addr
    mov r8d, eax                       // save addr
    rpeek ecx, 2                       // val (clobbers rax)
    mov byte ptr [r15 + r8], cl
    next

_DEIkr:
    precall
    CALL dei_kr_entry
    postcall
    next

_DEOkr:
    precall
    CALL deo_kr_entry
    postcall
    next

.macro binary_opkr insn
    rpeek ecx, 1                       // a (rpeek first; rax clobbered but ok)
    movzx eax, byte ptr [r13 + r14]   // b (loaded after rpeek)
    \insn ecx, eax                     // a OP b
    rpush cl
    next
.endm

_ADDkr:
    binary_opkr add

_SUBkr:
    binary_opkr sub

_MULkr:
    binary_opkr imul

_DIVkr:
    rpeek eax, 1                       // a (dividend; rpeek first, rax clobbered)
    movzx ecx, byte ptr [r13 + r14]   // b (divisor; loaded after rpeek)
    test cl, cl
    jz 1f
    div cl
    jmp 2f
1:
    xor eax, eax
2:
    rpush al
    next

_ANDkr:
    binary_opkr and

_ORAkr:
    binary_opkr or

_EORkr:
    binary_opkr xor

_SFTkr:
    movzx r9d, byte ptr [r13 + r14]   // shift amount in r9d (rpeek clobbers rax)
    rpeek edx, 1                       // value in dl
    mov ecx, r9d
    and ecx, 0xf                       // right shift count in cl
    shr dl, cl
    shr r9d, 4                         // left shift count
    mov ecx, r9d
    shl dl, cl
    rpush dl
    next

_LIT2r:
    movzx eax, byte ptr [r15 + rbp]
    inc rbp
    and rbp, 0xffff
    rpush al
    movzx eax, byte ptr [r15 + rbp]
    inc rbp
    and rbp, 0xffff
    rpush al
    next

_INC2kr:
    rpeek ecx, 1                        // high byte (rpeek first; rax clobbered)
    movzx eax, byte ptr [r13 + r14]    // low byte (loaded after rpeek)
    shl ecx, 8
    or eax, ecx
    inc eax
    and eax, 0xffff
    mov ecx, eax
    shr ecx, 8
    rpush cl
    rpush al
    next

_POP2kr:
    next

_NIP2kr:
    rpeek ecx, 1                       // b_hi (rpeek first; rax clobbered)
    movzx eax, byte ptr [r13 + r14]   // b_lo (loaded after rpeek)
    rpush cl                           // push b_hi
    rpush al                           // push b_lo
    next

_SWP2kr:
    rpeek eax, 1
    rpush al
    rpeek eax, 1
    rpush al
    rpeek eax, 5
    rpush al
    rpeek eax, 5
    rpush al
    next

_ROT2kr:
    rpeek eax, 3
    rpush al
    rpeek eax, 3
    rpush al
    rpeek eax, 3
    rpush al
    rpeek eax, 3
    rpush al
    rpeek eax, 9
    rpush al
    rpeek eax, 9
    rpush al
    next

_DUP2kr:
    rpeek ecx, 1                       // hi (rpeek first; rax clobbered)
    movzx eax, byte ptr [r13 + r14]   // lo (loaded after rpeek)
    rpush cl                           // push hi (1st copy)
    rpush al                           // push lo (1st copy)
    rpush cl                           // push hi (2nd copy)
    rpush al                           // push lo (2nd copy)
    next

_OVR2kr:
    // a b -- a b a b a  (6 new rpushes: a_hi, a_lo, b_hi, b_lo, a_hi, a_lo)
    rpeek ecx, 1                       // b_hi (rpeek first; rax clobbered)
    rpeek edx, 2                       // a_lo (clobbers rax; ecx=b_hi fine)
    rpeek esi, 3                       // a_hi (clobbers rax; ecx,edx fine)
    movzx eax, byte ptr [r13 + r14]   // b_lo (loaded last, after all rpeeks)
    rpush sil                          // push a_hi
    rpush dl                           // push a_lo
    rpush cl                           // push b_hi
    rpush al                           // push b_lo
    rpush sil                          // push a_hi
    rpush dl                           // push a_lo
    next

.macro compare_op2kr setcc_op
    rpeek ecx, 1                       // b_hi (rpeek first; rax clobbered)
    movzx r8d, byte ptr [r13 + r14]   // b_lo (loaded after rpeek)
    shl ecx, 8
    or r8d, ecx                        // b
    rpeek ecx, 2                       // a_lo
    rpeek edx, 3                       // a_hi
    shl edx, 8
    or ecx, edx                        // a
    cmp ecx, r8d                       // a vs b
    \setcc_op al
    rpush al
    next
.endm

_EQU2kr:
    compare_op2kr sete

_NEQ2kr:
    compare_op2kr setne

_GTH2kr:
    compare_op2kr seta

_LTH2kr:
    compare_op2kr setb

_JMP2kr:
    rpeek ecx, 1                       // hi (rpeek first; rax clobbered)
    movzx eax, byte ptr [r13 + r14]   // lo (loaded after rpeek)
    shl ecx, 8
    or eax, ecx                        // addr
    mov rbp, rax
    next

_JCN2kr:
    rpeek ecx, 1                       // addr_hi (rpeek first; rax clobbered)
    movzx eax, byte ptr [r13 + r14]   // addr_lo (loaded after rpeek)
    shl ecx, 8
    or eax, ecx                        // addr in eax
    mov r8d, eax                       // save addr (next rpeek clobbers rax)
    rpeek edx, 2                       // condition (clobbers rax)
    test edx, edx
    jz 1f
    mov rbp, r8
    and rbp, 0xffff
1:
    next

_JSR2kr:
    rpeek ecx, 1                       // hi (rpeek first; rax clobbered)
    movzx eax, byte ptr [r13 + r14]   // lo (loaded after rpeek)
    shl ecx, 8
    or eax, ecx                        // addr
    mov edx, ebp
    shr edx, 8
    stk_push dl
    stk_push bpl
    mov rbp, rax
    next

_STH2kr:
    rpeek ecx, 1                       // hi (rpeek first; rax clobbered)
    movzx eax, byte ptr [r13 + r14]   // lo (loaded after rpeek)
    stk_push cl                        // push hi
    stk_push al                        // push lo
    next

_LDZ2kr:
    movzx eax, byte ptr [r13 + r14]
    movzx ecx, byte ptr [r15 + rax]
    rpush cl
    inc eax
    and eax, 0xffff
    movzx ecx, byte ptr [r15 + rax]
    rpush cl
    next

_STZ2kr:
    rpeek ecx, 1                       // val_lo (rpeek first; rax clobbered)
    rpeek edx, 2                       // val_hi (clobbers rax; ecx=val_lo ok)
    movzx eax, byte ptr [r13 + r14]   // addr (loaded after rpeeks)
    mov byte ptr [r15 + rax], dl
    inc eax
    and eax, 0xffff
    mov byte ptr [r15 + rax], cl
    next

_LDR2kr:
    movsx rax, byte ptr [r13 + r14]
    lea rax, [rbp + rax]
    and rax, 0xffff
    movzx ecx, byte ptr [r15 + rax]
    rpush cl
    inc rax
    and rax, 0xffff
    movzx ecx, byte ptr [r15 + rax]
    rpush cl
    next

_STR2kr:
    rpeek ecx, 1                       // val_lo (rpeek first; rax clobbered)
    rpeek edx, 2                       // val_hi (clobbers rax; ecx=val_lo ok)
    movsx rax, byte ptr [r13 + r14]   // offset (loaded after rpeeks)
    lea rax, [rbp + rax]
    and rax, 0xffff
    mov byte ptr [r15 + rax], dl
    inc rax
    and rax, 0xffff
    mov byte ptr [r15 + rax], cl
    next

_LDA2kr:
    rpeek ecx, 1                       // addr_hi (rpeek first; rax clobbered)
    movzx eax, byte ptr [r13 + r14]   // addr_lo (loaded after rpeek)
    shl ecx, 8
    or eax, ecx
    movzx ecx, byte ptr [r15 + rax]
    rpush cl
    inc eax
    and eax, 0xffff
    movzx ecx, byte ptr [r15 + rax]
    rpush cl
    next

_STA2kr:
    rpeek ecx, 1                       // addr_hi (rpeek first; rax clobbered)
    movzx eax, byte ptr [r13 + r14]   // addr_lo (loaded after rpeek)
    shl ecx, 8
    or eax, ecx                        // full addr in eax
    mov r8d, eax                       // save addr (next rpeeks clobber rax)
    rpeek ecx, 2                       // val_lo (clobbers rax; r8d=addr ok)
    rpeek edx, 3                       // val_hi (clobbers rax; ecx=val_lo, r8d=addr ok)
    mov byte ptr [r15 + r8], dl
    inc r8d
    and r8d, 0xffff
    mov byte ptr [r15 + r8], cl
    next

_DEI2kr:
    precall
    CALL dei_2kr_entry
    postcall
    next

_DEO2kr:
    precall
    CALL deo_2kr_entry
    postcall
    next

.macro binary_op2kr insn
    rpeek ecx, 1                       // b_hi (rpeek first; rax clobbered)
    movzx eax, byte ptr [r13 + r14]   // b_lo (loaded after rpeek)
    shl ecx, 8
    or eax, ecx                        // b
    mov r8d, eax                       // save b (next rpeeks clobber rax/eax)

    rpeek ecx, 2                       // a_lo
    rpeek edx, 3                       // a_hi
    shl edx, 8
    or ecx, edx                        // a

    \insn ecx, r8d                     // a OP b → ecx
    movzx r8d, cl                      // save result_lo
    shr ecx, 8
    rpush cl                           // push result_hi first
    rpush r8b                          // push result_lo on top
    next
.endm

_ADD2kr:
    binary_op2kr add

_SUB2kr:
    binary_op2kr sub

_MUL2kr:
    binary_op2kr imul

_DIV2kr:
    rpeek ecx, 1                       // b_hi (rpeek first; rax clobbered)
    movzx eax, byte ptr [r13 + r14]   // b_lo (loaded after rpeek)
    shl ecx, 8
    or eax, ecx                        // b
    mov r8d, eax                       // save b (next rpeeks clobber rax)

    rpeek ecx, 2                       // a_lo
    rpeek edx, 3                       // a_hi
    shl edx, 8
    or ecx, edx                        // a

    push r8                            // save b (divisor) onto x86 stack
    mov eax, ecx                       // dividend in eax
    movzx eax, ax
    xor edx, edx
    pop rcx                            // restore divisor into ecx
    movzx ecx, cx
    test cx, cx
    jz 1f
    div cx                             // ax = a / b
    jmp 2f
1:
    xor eax, eax
2:
    movzx r8d, al                      // save result_lo
    shr eax, 8
    rpush al                           // push result_hi first
    rpush r8b                          // push result_lo on top
    next

_AND2kr:
    binary_op2kr and

_ORA2kr:
    binary_op2kr or

_EOR2kr:
    binary_op2kr xor

_SFT2kr:
    movzx r9d, byte ptr [r13 + r14]   // shift amount in r9d (rpeek clobbers rax)
    rpeek r8d, 1                       // value_lo in r8d
    rpeek edx, 2                       // value_hi in edx
    shl edx, 8
    or r8d, edx                        // value (16-bit) in r8d

    mov ecx, r9d
    and ecx, 0xf                       // right shift count in cl
    shr r8d, cl
    shr r9d, 4                         // left shift count
    mov ecx, r9d
    shl r8d, cl                        // result in r8d

    mov edx, r8d
    shr edx, 8
    rpush dl                           // push result_hi first
    rpush r8b                          // push result_lo (becomes top)
    next
