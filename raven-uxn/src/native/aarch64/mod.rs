#[cfg(target_os = "macos")]
core::arch::global_asm!(concat!(
    include_str!("macos.s"),
    include_str!("impl.s")
    include_str!("../jump_table.s"),
));

#[cfg(target_os = "linux")]
core::arch::global_asm!(concat!(
    include_str!("linux.s"),
    include_str!("impl.s")
    include_str!("../jump_table.s"),
));
