// Platform-specific macro to load the page-aligned jump table
.macro load_jump_table x
    adrp x8, JUMP_TABLE
.endm
