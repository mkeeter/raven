// Platform-specific macro to load the page-aligned jump table
.macro load_jump_table, x
    adrp \x, JUMP_TABLE@PAGE
.endm

.macro CALL, name
    bl _\name
.endm

.macro ENTRY, name
    .global _\name
    _\name:
.endm
