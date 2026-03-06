// Platform-specific macros for Linux x86-64 (no symbol prefix needed)
.macro CALL name
    call \name
.endm

.macro ENTRY name
    .global \name
    \name:
.endm
