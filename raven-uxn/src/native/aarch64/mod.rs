#[cfg(target_os = "macos")]
core::arch::global_asm!(concat!(
    include_str!("macos.s"),
    include_str!("impl.s"),
    include_str!("../jump_table.s"),
));

#[cfg(target_os = "linux")]
core::arch::global_asm!(concat!(
    include_str!("linux.s"),
    include_str!("impl.s"),
    include_str!("../jump_table.s"),
));

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
compile_error!(
    "unsupported target OS for AArch64 interpreter; \
     you may want to disable the 'native' feature"
);
