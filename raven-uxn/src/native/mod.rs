use crate::{Device, Uxn};

#[cfg(not(target_arch = "aarch64"))]
compile_error!("no native implementation for this platform");

////////////////////////////////////////////////////////////////////////////////
// Stubs for DEO calls

#[no_mangle]
extern "C" fn deo_entry(vm: &mut Uxn, dev: &mut DeviceHandle) -> bool {
    vm.deo::<0b000>(dev.0, 0).is_some()
}

#[no_mangle]
extern "C" fn deo_2_entry(vm: &mut Uxn, dev: &mut DeviceHandle) -> bool {
    vm.deo::<0b001>(dev.0, 0).is_some()
}

#[no_mangle]
extern "C" fn deo_r_entry(vm: &mut Uxn, dev: &mut DeviceHandle) -> bool {
    vm.deo::<0b010>(dev.0, 0).is_some()
}
#[no_mangle]
extern "C" fn deo_2r_entry(vm: &mut Uxn, dev: &mut DeviceHandle) -> bool {
    vm.deo::<0b011>(dev.0, 0).is_some()
}

#[no_mangle]
extern "C" fn deo_k_entry(vm: &mut Uxn, dev: &mut DeviceHandle) -> bool {
    vm.deo::<0b100>(dev.0, 0).is_some()
}

#[no_mangle]
extern "C" fn deo_2k_entry(vm: &mut Uxn, dev: &mut DeviceHandle) -> bool {
    vm.deo::<0b101>(dev.0, 0).is_some()
}

#[no_mangle]
extern "C" fn deo_kr_entry(vm: &mut Uxn, dev: &mut DeviceHandle) -> bool {
    vm.deo::<0b110>(dev.0, 0).is_some()
}

#[no_mangle]
extern "C" fn deo_2kr_entry(vm: &mut Uxn, dev: &mut DeviceHandle) -> bool {
    vm.deo::<0b111>(dev.0, 0).is_some()
}

////////////////////////////////////////////////////////////////////////////////
// Stubs for DEI calls

#[no_mangle]
extern "C" fn dei_entry(vm: &mut Uxn, dev: &mut DeviceHandle) -> bool {
    vm.dei::<0b000>(dev.0, 0).is_some()
}

#[no_mangle]
extern "C" fn dei_2_entry(vm: &mut Uxn, dev: &mut DeviceHandle) -> bool {
    vm.dei::<0b001>(dev.0, 0).is_some()
}

#[no_mangle]
extern "C" fn dei_r_entry(vm: &mut Uxn, dev: &mut DeviceHandle) -> bool {
    vm.dei::<0b010>(dev.0, 0).is_some()
}
#[no_mangle]
extern "C" fn dei_2r_entry(vm: &mut Uxn, dev: &mut DeviceHandle) -> bool {
    vm.dei::<0b011>(dev.0, 0).is_some()
}

#[no_mangle]
extern "C" fn dei_k_entry(vm: &mut Uxn, dev: &mut DeviceHandle) -> bool {
    vm.dei::<0b100>(dev.0, 0).is_some()
}

#[no_mangle]
extern "C" fn dei_2k_entry(vm: &mut Uxn, dev: &mut DeviceHandle) -> bool {
    vm.dei::<0b101>(dev.0, 0).is_some()
}

#[no_mangle]
extern "C" fn dei_kr_entry(vm: &mut Uxn, dev: &mut DeviceHandle) -> bool {
    vm.dei::<0b110>(dev.0, 0).is_some()
}

#[no_mangle]
extern "C" fn dei_2kr_entry(vm: &mut Uxn, dev: &mut DeviceHandle) -> bool {
    vm.dei::<0b111>(dev.0, 0).is_some()
}

////////////////////////////////////////////////////////////////////////////////

struct DeviceHandle<'a>(&'a mut dyn Device);

pub fn entry(vm: &mut Uxn, dev: &mut dyn Device, pc: u16) -> u16 {
    let mut h = DeviceHandle(dev);
    let r: usize;

    // SAFETY: do you trust me?
    unsafe {
        core::arch::asm!(
            "bl aarch64_entry",
            inout("x0") vm.stack.data.as_mut_ptr() as usize => r,
            in("x1") &mut vm.stack.index as *mut _,
            in("x2") vm.ret.data.as_mut_ptr(),
            in("x3") &mut vm.ret.index as *mut _,
            in("x4") (*vm.ram).as_mut_ptr(),
            in("x5") pc,
            in("x6") vm as *mut _,
            in("x7") &mut h as *mut _,
        );
    }
    r as u16
}

core::arch::global_asm!(include_str!("aarch64.s"));

#[cfg(all(feature = "alloc", test))]
mod test {
    use crate::{op::*, Backend, EmptyDevice, Uxn, UxnRam};

    fn run_and_compare(cmd: &[u8]) {
        run_and_compare_all(cmd, false, false);
    }

    fn run_and_compare_r(cmd: &[u8]) {
        run_and_compare_all(cmd, false, true);
    }

    fn run_and_compare_with_ram_r(cmd: &[u8]) {
        run_and_compare_all(cmd, true, true);
    }

    /// Tests the given command string, along with its `keep` variant
    ///
    /// If `test_r` is set, also tests `ret` variants
    fn run_and_compare_all(cmd: &[u8], fill_ram: bool, test_r: bool) {
        run_and_compare_inner(cmd, fill_ram);

        // Test with the keep flag set
        let mut cmd_k = cmd.to_vec();
        *cmd_k.last_mut().unwrap() |= 0b100 << 5;
        run_and_compare_inner(&cmd_k, fill_ram);

        // Test with the return flag set
        if test_r {
            let mut cmd_r = cmd.to_vec();
            *cmd_r.last_mut().unwrap() |= 0b010 << 5;
            for c in cmd_r.iter_mut() {
                if *c == LIT {
                    *c = LITr;
                } else if *c == LIT2 {
                    *c = LIT2r;
                }
            }
            run_and_compare_inner(&cmd_k, fill_ram);

            let mut cmd_kr = cmd_r.to_vec();
            *cmd_kr.last_mut().unwrap() |= 0b100 << 5;
            run_and_compare_inner(&cmd_kr, fill_ram);
        }
    }

    /// Tests all 8 variants of a binary opcode
    fn op_binary(op: u8) {
        assert!(op & (0b011 << 6) == 0);
        run_and_compare_r(&[LIT, 0x56, LIT, 0x98, op]);
        run_and_compare_r(&[LIT, 0x23, LIT, 0x23, op]);
        run_and_compare_r(&[LIT, 0x00, LIT, 0x23, op]);
        run_and_compare_r(&[LIT, 0x98, LIT, 0x34, op]);
        run_and_compare_r(&[LIT, 0x98, LIT, 0x00, op]);

        let op2 = op | (0b001 << 5);
        run_and_compare_r(&[LIT2, 0x56, 0x12, LIT2, 0x43, 0x98, op2]);
        run_and_compare_r(&[LIT2, 0x00, 0x00, LIT2, 0x43, 0x98, op2]);
        run_and_compare_r(&[LIT2, 0x56, 0x12, LIT2, 0x00, 0x00, op2]);
        run_and_compare_r(&[LIT2, 0x12, 0x34, LIT2, 0x12, 0x34, op2]);
        run_and_compare_r(&[LIT2, 0x43, 0x12, LIT2, 0x56, 0x98, op2]);
    }

    fn run_and_compare_inner(cmd: &[u8], fill_ram: bool) {
        let op = cmd.last().unwrap();
        let op_name = NAMES[*op as usize];

        let mut cmd = cmd.to_vec();
        if fill_ram {
            cmd.push(BRK);
        }

        let mut dev = EmptyDevice;
        let mut ram_native = UxnRam::new();
        let mut ram_interp = UxnRam::new();
        if fill_ram {
            for i in 0..ram_native.len() {
                ram_native[i] = i as u8;
                ram_interp[i] = i as u8;
            }
        }
        let mut vm_native = Uxn::new(&mut ram_native, Backend::Native);
        let r = vm_native.reset(&cmd);
        assert!(r.is_empty());

        let mut vm_interp = Uxn::new(&mut ram_interp, Backend::Interpreter);
        let r = vm_interp.reset(&cmd);
        assert!(r.is_empty());

        let pc_native = vm_native.run(&mut dev, 0x100);
        let pc_interp = vm_interp.run(&mut dev, 0x100);
        assert_eq!(pc_native, pc_interp, "{op_name}: pc mismatch");

        assert_eq!(
            vm_native.dev, vm_interp.dev,
            "{op_name}: dev memory mismatch"
        );
        assert_eq!(vm_native.ram, vm_interp.ram, "{op_name}: ram mismatch");
        assert_eq!(
            vm_native.stack.index, vm_interp.stack.index,
            "{op_name}: stack index mismatch"
        );
        assert_eq!(
            vm_native.stack.data, vm_interp.stack.data,
            "{op_name}: stack data mismatch"
        );
        assert_eq!(
            vm_native.ret.index, vm_interp.ret.index,
            "{op_name}: ret index mismatch"
        );
        assert_eq!(
            vm_native.ret.data, vm_interp.ret.data,
            "{op_name}: ret index mismatch"
        );
    }

    #[test]
    fn brk() {
        run_and_compare(&[BRK]);
    }

    #[test]
    fn inc() {
        run_and_compare_r(&[LIT, 0x1, INC]);
    }

    #[test]
    fn pop() {
        run_and_compare_r(&[LIT, 0x1, POP]);
        run_and_compare_r(&[POP]);
    }

    #[test]
    fn nip() {
        run_and_compare_r(&[LIT2, 0x12, 0x34, NIP]);
        run_and_compare_r(&[NIP]);
    }

    #[test]
    fn swp() {
        run_and_compare_r(&[LIT2, 0x12, 0x34, SWP]);
    }

    #[test]
    fn rot() {
        run_and_compare_r(&[LIT2, 0x12, 0x34, LIT, 0x45, ROT]);
    }

    #[test]
    fn dup() {
        run_and_compare_r(&[LIT, 0x45, DUP]);
    }

    #[test]
    fn ovr() {
        run_and_compare_r(&[LIT2, 0x12, 0x34, OVR]);
    }

    #[test]
    fn equ() {
        op_binary(EQU);
    }

    #[test]
    fn neq() {
        op_binary(NEQ);
    }

    #[test]
    fn gth() {
        op_binary(GTH);
    }

    #[test]
    fn lth() {
        op_binary(LTH);
    }

    #[test]
    fn jmp() {
        run_and_compare_r(&[LIT, 0x12, JMP]);
        run_and_compare_r(&[LIT, 0xf2, JMP]);
    }

    #[test]
    fn jcn() {
        run_and_compare_r(&[LIT2, 0x1, 0x12, JCN]);
        run_and_compare_r(&[LIT2, 0x1, 0xf2, JCN]);
        run_and_compare_r(&[LIT2, 0x0, 0x12, JCN]);
        run_and_compare_r(&[LIT2, 0x0, 0xf2, JCN]);
    }

    #[test]
    fn jsr() {
        run_and_compare_r(&[LIT, 0x12, JSR]);
        run_and_compare_r(&[LIT, 0xf2, JSR]);
    }

    #[test]
    fn sth() {
        run_and_compare_r(&[LIT, 0x12, STH]);
        run_and_compare_r(&[STH]);
    }

    #[test]
    fn ldz() {
        run_and_compare_with_ram_r(&[LIT, 0x12, LDZ]);
        run_and_compare_with_ram_r(&[LIT, 0xff, LDZ]);
    }

    #[test]
    fn stz() {
        run_and_compare_r(&[LIT2, 0x12, 0x34, STZ]);
    }

    #[test]
    fn ldr() {
        run_and_compare_with_ram_r(&[LIT, 0x12, LDR]);
        run_and_compare_with_ram_r(&[LIT, 0xf2, LDR]);
    }

    #[test]
    fn str() {
        run_and_compare_r(&[LIT2, 0x12, 0x12, STR]);
        run_and_compare_r(&[LIT2, 0x34, 0xf2, STR]);
    }

    #[test]
    fn lda() {
        run_and_compare_with_ram_r(&[LIT2, 0x12, 0x34, LDA]);
        run_and_compare_with_ram_r(&[LIT2, 0x35, 0xff, LDA]);
    }

    #[test]
    fn sta() {
        run_and_compare_r(&[LIT, 0x56, LIT2, 0x12, 0x34, STA]);
        run_and_compare_r(&[LIT, 0x78, LIT2, 0x35, 0xff, STA]);
    }

    #[test]
    fn deo() {
        run_and_compare_r(&[LIT2, 0x56, 0x34, DEO]);
        run_and_compare_r(&[LIT2, 0x64, 0x34, DEO]);
    }

    #[test]
    fn dei() {
        run_and_compare_r(&[LIT2, 0x56, 0x34, DEI]);
        run_and_compare_r(&[LIT2, 0x64, 0x34, DEI]);
    }

    #[test]
    fn add() {
        op_binary(ADD)
    }

    #[test]
    fn sub() {
        op_binary(SUB)
    }

    #[test]
    fn mul() {
        op_binary(MUL)
    }

    #[test]
    fn div() {
        op_binary(DIV)
    }

    #[test]
    fn and() {
        op_binary(AND)
    }

    #[test]
    fn ora() {
        op_binary(ORA)
    }

    #[test]
    fn eor() {
        op_binary(EOR)
    }

    #[test]
    fn sft() {
        run_and_compare_r(&[LIT2, 0x56, 0x12, SFT]);
        run_and_compare_r(&[LIT2, 0x06, 0x12, SFT]);
        run_and_compare_r(&[LIT2, 0x10, 0x12, SFT]);
    }

    #[test]
    fn jci() {
        run_and_compare(&[LIT, 0x0, JCI, 0x12, 0x34]);
        run_and_compare(&[LIT, 0x0, JCI, 0xf2, 0x34]);
        run_and_compare(&[LIT, 0x1, JCI, 0x12, 0x34]);
        run_and_compare(&[LIT, 0x1, JCI, 0xf2, 0x34]);
        run_and_compare(&[LIT, 0x0, JCI, 0x12, 0xf4]);
        run_and_compare(&[LIT, 0x0, JCI, 0xf2, 0xf4]);
        run_and_compare(&[LIT, 0x1, JCI, 0x12, 0xf4]);
        run_and_compare(&[LIT, 0x1, JCI, 0xf2, 0xf4]);
    }

    #[test]
    fn inc2() {
        run_and_compare_r(&[LIT2, 0x1, 0x34, INC2]);
    }

    #[test]
    fn pop2() {
        run_and_compare_r(&[LIT2, 0x1, 0x34, POP2]);
        run_and_compare_r(&[LIT, 0x34, POP2]);
    }

    #[test]
    fn nip2() {
        run_and_compare_r(&[LIT2, 0x1, 0x34, LIT2, 0x45, 0x67, NIP2]);
    }

    #[test]
    fn swp2() {
        run_and_compare_r(&[LIT2, 0x1, 0x34, LIT2, 0x45, 0x67, SWP2]);
    }

    #[test]
    fn rot2() {
        run_and_compare_r(&[
            LIT2, 0x1, 0x34, LIT2, 0x45, 0x67, LIT2, 0xf4, 0xe0, ROT2,
        ]);
    }

    #[test]
    fn dup2() {
        run_and_compare_r(&[LIT2, 0x1, 0x34, DUP2]);
    }

    #[test]
    fn ovr2() {
        run_and_compare_r(&[LIT2, 0x1, 0x34, LIT2, 0x45, 0x67, OVR2]);
    }

    #[test]
    fn jmp2() {
        run_and_compare_r(&[LIT2, 0x12, 0x34, JMP2]);
        run_and_compare_r(&[LIT2, 0xf2, 0xff, JMP2]);
    }

    #[test]
    fn jcn2() {
        run_and_compare_r(&[LIT, 0x1, LIT2, 0x12, 0x34, JCN2]);
        run_and_compare_r(&[LIT, 0x1, LIT2, 0xf2, 0x34, JCN2]);
        run_and_compare_r(&[LIT, 0x0, LIT2, 0x12, 0x34, JCN2]);
        run_and_compare_r(&[LIT, 0x0, LIT2, 0xf2, 0x34, JCN2]);
    }

    #[test]
    fn sth2() {
        run_and_compare_r(&[LIT2, 0x12, 0x34, STH2]);
        run_and_compare_r(&[LIT2, 0xf2, 0x34, STH2]);
        run_and_compare_r(&[LIT2, 0x12, 0x34, STH2]);
        run_and_compare_r(&[LIT2, 0xf2, 0x34, STH2]);
    }

    #[test]
    fn jsr2() {
        run_and_compare_r(&[LIT2, 0x12, 0x34, JSR2]);
        run_and_compare_r(&[LIT2, 0xf2, 0x34, JSR2]);
        run_and_compare_r(&[LIT2, 0x12, 0x34, JSR2]);
        run_and_compare_r(&[LIT2, 0xf2, 0x34, JSR2]);
    }

    #[test]
    fn ldz2() {
        run_and_compare_with_ram_r(&[LIT, 0x12, LDZ2]);
    }

    #[test]
    fn stz2() {
        run_and_compare_r(&[LIT2, 0x12, 0x34, LIT, 0x56, STZ2]);
    }

    #[test]
    fn ldr2() {
        run_and_compare_with_ram_r(&[LIT, 0x12, LDR2]);
        run_and_compare_with_ram_r(&[LIT, 0xf2, LDR2]);
    }

    #[test]
    fn str2() {
        run_and_compare_r(&[LIT2, 0x12, 0x45, LIT, 0x12, STR2]);
        run_and_compare_r(&[LIT2, 0x34, 0x56, LIT, 0xf2, STR2]);
    }

    #[test]
    fn lda2() {
        run_and_compare_with_ram_r(&[LIT2, 0x12, 0x34, LDA2]);
        run_and_compare_with_ram_r(&[LIT2, 0x35, 0xff, LDA2]);
    }

    #[test]
    fn sta2() {
        run_and_compare_r(&[LIT2, 0x56, 0x14, LIT2, 0x12, 0x34, STA2]);
        run_and_compare_r(&[LIT2, 0x78, 0x90, LIT2, 0x35, 0xff, STA2]);
    }

    #[test]
    fn deo2() {
        run_and_compare_r(&[LIT2, 0x56, 0x12, LIT, 0x78, DEO2]);
        run_and_compare_r(&[LIT2, 0x64, 0x45, LIT, 0x56, DEO2]);
    }

    #[test]
    fn dei2() {
        run_and_compare_r(&[LIT2, 0x56, 0x12, LIT, 0x78, DEI2]);
        run_and_compare_r(&[LIT2, 0x64, 0x45, LIT, 0x56, DEI2]);
    }

    #[test]
    fn sft2() {
        run_and_compare_r(&[LIT2, 0x56, 0x12, LIT, 0x34, SFT2]);
        run_and_compare_r(&[LIT2, 0x56, 0x12, LIT, 0x04, SFT2]);
        run_and_compare_r(&[LIT2, 0x56, 0x12, LIT, 0x30, SFT2]);
    }

    #[test]
    fn jmi() {
        // NOTE: testing `keep` mode is meaningless here, because the last
        // instruction in the tape isn't the opcode under test
        run_and_compare(&[JMI, 0x56, 0x12]);
        run_and_compare(&[JMI, 0xf6, 0x12]);
        run_and_compare(&[JMI, 0x16, 0xf2]);
    }

    #[test]
    fn jsi() {
        run_and_compare(&[JSI, 0x56, 0x12]);
        run_and_compare(&[JSI, 0xf6, 0x12]);
        run_and_compare(&[JSI, 0x26, 0xf2]);
    }
}
