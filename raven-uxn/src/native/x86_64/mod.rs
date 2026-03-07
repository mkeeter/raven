#[cfg(target_os = "linux")]
core::arch::global_asm!(concat!(
    include_str!("linux.s"),
    include_str!("impl.s"),
    include_str!("../jump_table.s"),
));
