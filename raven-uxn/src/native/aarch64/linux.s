// Platform-specific macro to load the page-aligned jump table
.macro load_jump_table, x
    adrp x8, JUMP_TABLE
.endm

.macro CALL, name
    bl \name
.endm

.macro ENTRY, name
    .global \name
    \name:
.endm
