#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use uxn::{Backend, EmptyDevice, Uxn, UxnRam};

#[derive(Arbitrary)]
struct UxnProgram<'a>(&'a [u8]);

impl std::fmt::Debug for UxnProgram<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)?;
        let mut lit_count = 0;
        for &op in self.0.iter() {
            if lit_count > 0 {
                write!(f, " {}", op)?;
                lit_count -= 1;
            } else {
                write!(f, " {}", uxn::op::NAMES[usize::from(op)])?;
                if matches!(op, uxn::op::LIT | uxn::op::LITr) {
                    lit_count = 1;
                } else if matches!(
                    op,
                    uxn::op::LIT2
                        | uxn::op::LIT2r
                        | uxn::op::JCI
                        | uxn::op::JMI
                        | uxn::op::JSI
                ) {
                    lit_count = 2;
                }
            }
        }
        Ok(())
    }
}

fuzz_target!(|data: UxnProgram| {
    let mut ram_v = UxnRam::new();
    let mut vm_v = Uxn::new(&mut ram_v, Backend::Interpreter);

    let mut ram_n = UxnRam::new();
    let mut vm_n = Uxn::new(&mut ram_n, Backend::Native);

    // Don't load any programs that require auxiliary memory
    if !vm_v.reset(data.0).is_empty() {
        return;
    }
    assert!(vm_n.reset(data.0).is_empty());

    // Use the VM-backed evaluator, halting if we take more than 65K cycles
    let Some(pc_v) =
        vm_v.run_until(&mut EmptyDevice, 0x100, |_uxn, _dev, i| i > 65536)
    else {
        return;
    };
    let pc_n = vm_n.run(&mut EmptyDevice, 0x100);

    let mut failed = false;

    if pc_v != pc_n {
        println!("PC mismatch: {pc_v:#04x} != {pc_n:#04x}");
        failed = true;
    }
    for i in 0..=65535 {
        let a = vm_v.ram_read_byte(i);
        let b = vm_n.ram_read_byte(i);
        if a != b {
            println!("RAM mismatch at {i:#04x}: {a:#02x} != {b:#02x}");
            failed = true;
        }
    }
    if vm_v.ret() != vm_n.ret() {
        println!(
            "return mismatch:\n  bytecode: {:?}\n    native: {:?}",
            vm_v.ret(),
            vm_n.ret()
        );
        failed = true;
    }
    if vm_v.stack() != vm_n.stack() {
        println!(
            "stack mismatch:\n  bytecode: {:?}\n    native: {:?}",
            vm_v.stack(),
            vm_n.stack()
        );
        failed = true;
    }
    if failed {
        panic!("mismatch found"); // debug impl pretty-prints the program
    }
});
