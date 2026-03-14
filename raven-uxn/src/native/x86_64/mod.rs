#[cfg(any(target_os = "linux", target_os = "windows"))]
core::arch::global_asm!(concat!(
    include_str!("impl.s"),
    include_str!("../jump_table.s"),
));

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
compile_error!(
    "unsupported target OS for x86-64 interpreter; \
     you may want to diable the 'native' feature"
);
