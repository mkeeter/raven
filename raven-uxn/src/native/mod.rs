use crate::{Device, Uxn};

#[cfg(target_arch = "aarch64")]
mod aarch64;

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

#[repr(C)]
pub(crate) struct EntryHandle {
    stack_data: *mut u8,
    stack_index: *mut u8,
    ret_data: *mut u8,
    ret_index: *mut u8,
    ram: *mut u8,
    vm: *mut core::ffi::c_void,  // *Uxn
    dev: *mut core::ffi::c_void, // *DeviceHandle
}

struct DeviceHandle<'a>(&'a mut dyn Device);

pub fn entry(vm: &mut Uxn, dev: &mut dyn Device, pc: u16) -> u16 {
    let mut h = DeviceHandle(dev);
    let mut e = EntryHandle {
        stack_data: vm.stack.data.as_mut_ptr(),
        stack_index: &mut vm.stack.index as *mut _,
        ret_data: vm.ret.data.as_mut_ptr(),
        ret_index: &mut vm.ret.index as *mut _,
        ram: (*vm.ram).as_mut_ptr(),
        vm: vm as *mut _ as *mut _,
        dev: &mut h as *mut _ as *mut _,
    };

    // SAFETY: do you trust me?
    unsafe { aarch64::aarch64_entry(&mut e as *mut _, pc, JUMP_TABLE.as_ptr()) }
}

extern "C" {
    fn BRK();
    fn INC();
    fn POP();
    fn NIP();
    fn SWP();
    fn ROT();
    fn DUP();
    fn OVR();
    fn EQU();
    fn NEQ();
    fn GTH();
    fn LTH();
    fn JMP();
    fn JCN();
    fn JSR();
    fn STH();
    fn LDZ();
    fn STZ();
    fn LDR();
    fn STR();
    fn LDA();
    fn STA();
    fn DEI();
    fn DEO();
    fn ADD();
    fn SUB();
    fn MUL();
    fn DIV();
    fn AND();
    fn ORA();
    fn EOR();
    fn SFT();
    fn JCI();
    fn INC2();
    fn POP2();
    fn NIP2();
    fn SWP2();
    fn ROT2();
    fn DUP2();
    fn OVR2();
    fn EQU2();
    fn NEQ2();
    fn GTH2();
    fn LTH2();
    fn JMP2();
    fn JCN2();
    fn JSR2();
    fn STH2();
    fn LDZ2();
    fn STZ2();
    fn LDR2();
    fn STR2();
    fn LDA2();
    fn STA2();
    fn DEI2();
    fn DEO2();
    fn ADD2();
    fn SUB2();
    fn MUL2();
    fn DIV2();
    fn AND2();
    fn ORA2();
    fn EOR2();
    fn SFT2();
    fn JMI();
    fn INCr();
    fn POPr();
    fn NIPr();
    fn SWPr();
    fn ROTr();
    fn DUPr();
    fn OVRr();
    fn EQUr();
    fn NEQr();
    fn GTHr();
    fn LTHr();
    fn JMPr();
    fn JCNr();
    fn JSRr();
    fn STHr();
    fn LDZr();
    fn STZr();
    fn LDRr();
    fn STRr();
    fn LDAr();
    fn STAr();
    fn DEIr();
    fn DEOr();
    fn ADDr();
    fn SUBr();
    fn MULr();
    fn DIVr();
    fn ANDr();
    fn ORAr();
    fn EORr();
    fn SFTr();
    fn JSI();
    fn INC2r();
    fn POP2r();
    fn NIP2r();
    fn SWP2r();
    fn ROT2r();
    fn DUP2r();
    fn OVR2r();
    fn EQU2r();
    fn NEQ2r();
    fn GTH2r();
    fn LTH2r();
    fn JMP2r();
    fn JCN2r();
    fn JSR2r();
    fn STH2r();
    fn LDZ2r();
    fn STZ2r();
    fn LDR2r();
    fn STR2r();
    fn LDA2r();
    fn STA2r();
    fn DEI2r();
    fn DEO2r();
    fn ADD2r();
    fn SUB2r();
    fn MUL2r();
    fn DIV2r();
    fn AND2r();
    fn ORA2r();
    fn EOR2r();
    fn SFT2r();
    fn LIT();
    fn INCk();
    fn POPk();
    fn NIPk();
    fn SWPk();
    fn ROTk();
    fn DUPk();
    fn OVRk();
    fn EQUk();
    fn NEQk();
    fn GTHk();
    fn LTHk();
    fn JMPk();
    fn JCNk();
    fn JSRk();
    fn STHk();
    fn LDZk();
    fn STZk();
    fn LDRk();
    fn STRk();
    fn LDAk();
    fn STAk();
    fn DEIk();
    fn DEOk();
    fn ADDk();
    fn SUBk();
    fn MULk();
    fn DIVk();
    fn ANDk();
    fn ORAk();
    fn EORk();
    fn SFTk();
    fn LIT2();
    fn INC2k();
    fn POP2k();
    fn NIP2k();
    fn SWP2k();
    fn ROT2k();
    fn DUP2k();
    fn OVR2k();
    fn EQU2k();
    fn NEQ2k();
    fn GTH2k();
    fn LTH2k();
    fn JMP2k();
    fn JCN2k();
    fn JSR2k();
    fn STH2k();
    fn LDZ2k();
    fn STZ2k();
    fn LDR2k();
    fn STR2k();
    fn LDA2k();
    fn STA2k();
    fn DEI2k();
    fn DEO2k();
    fn ADD2k();
    fn SUB2k();
    fn MUL2k();
    fn DIV2k();
    fn AND2k();
    fn ORA2k();
    fn EOR2k();
    fn SFT2k();
    fn LITr();
    fn INCkr();
    fn POPkr();
    fn NIPkr();
    fn SWPkr();
    fn ROTkr();
    fn DUPkr();
    fn OVRkr();
    fn EQUkr();
    fn NEQkr();
    fn GTHkr();
    fn LTHkr();
    fn JMPkr();
    fn JCNkr();
    fn JSRkr();
    fn STHkr();
    fn LDZkr();
    fn STZkr();
    fn LDRkr();
    fn STRkr();
    fn LDAkr();
    fn STAkr();
    fn DEIkr();
    fn DEOkr();
    fn ADDkr();
    fn SUBkr();
    fn MULkr();
    fn DIVkr();
    fn ANDkr();
    fn ORAkr();
    fn EORkr();
    fn SFTkr();
    fn LIT2r();
    fn INC2kr();
    fn POP2kr();
    fn NIP2kr();
    fn SWP2kr();
    fn ROT2kr();
    fn DUP2kr();
    fn OVR2kr();
    fn EQU2kr();
    fn NEQ2kr();
    fn GTH2kr();
    fn LTH2kr();
    fn JMP2kr();
    fn JCN2kr();
    fn JSR2kr();
    fn STH2kr();
    fn LDZ2kr();
    fn STZ2kr();
    fn LDR2kr();
    fn STR2kr();
    fn LDA2kr();
    fn STA2kr();
    fn DEI2kr();
    fn DEO2kr();
    fn ADD2kr();
    fn SUB2kr();
    fn MUL2kr();
    fn DIV2kr();
    fn AND2kr();
    fn ORA2kr();
    fn EOR2kr();
    fn SFT2kr();
}

const JUMP_TABLE: [unsafe extern "C" fn(); 256] = [
    (BRK as unsafe extern "C" fn()),
    (INC as unsafe extern "C" fn()),
    (POP as unsafe extern "C" fn()),
    (NIP as unsafe extern "C" fn()),
    (SWP as unsafe extern "C" fn()),
    (ROT as unsafe extern "C" fn()),
    (DUP as unsafe extern "C" fn()),
    (OVR as unsafe extern "C" fn()),
    (EQU as unsafe extern "C" fn()),
    (NEQ as unsafe extern "C" fn()),
    (GTH as unsafe extern "C" fn()),
    (LTH as unsafe extern "C" fn()),
    (JMP as unsafe extern "C" fn()),
    (JCN as unsafe extern "C" fn()),
    (JSR as unsafe extern "C" fn()),
    (STH as unsafe extern "C" fn()),
    (LDZ as unsafe extern "C" fn()),
    (STZ as unsafe extern "C" fn()),
    (LDR as unsafe extern "C" fn()),
    (STR as unsafe extern "C" fn()),
    (LDA as unsafe extern "C" fn()),
    (STA as unsafe extern "C" fn()),
    (DEI as unsafe extern "C" fn()),
    (DEO as unsafe extern "C" fn()),
    (ADD as unsafe extern "C" fn()),
    (SUB as unsafe extern "C" fn()),
    (MUL as unsafe extern "C" fn()),
    (DIV as unsafe extern "C" fn()),
    (AND as unsafe extern "C" fn()),
    (ORA as unsafe extern "C" fn()),
    (EOR as unsafe extern "C" fn()),
    (SFT as unsafe extern "C" fn()),
    (JCI as unsafe extern "C" fn()),
    (INC2 as unsafe extern "C" fn()),
    (POP2 as unsafe extern "C" fn()),
    (NIP2 as unsafe extern "C" fn()),
    (SWP2 as unsafe extern "C" fn()),
    (ROT2 as unsafe extern "C" fn()),
    (DUP2 as unsafe extern "C" fn()),
    (OVR2 as unsafe extern "C" fn()),
    (EQU2 as unsafe extern "C" fn()),
    (NEQ2 as unsafe extern "C" fn()),
    (GTH2 as unsafe extern "C" fn()),
    (LTH2 as unsafe extern "C" fn()),
    (JMP2 as unsafe extern "C" fn()),
    (JCN2 as unsafe extern "C" fn()),
    (JSR2 as unsafe extern "C" fn()),
    (STH2 as unsafe extern "C" fn()),
    (LDZ2 as unsafe extern "C" fn()),
    (STZ2 as unsafe extern "C" fn()),
    (LDR2 as unsafe extern "C" fn()),
    (STR2 as unsafe extern "C" fn()),
    (LDA2 as unsafe extern "C" fn()),
    (STA2 as unsafe extern "C" fn()),
    (DEI2 as unsafe extern "C" fn()),
    (DEO2 as unsafe extern "C" fn()),
    (ADD2 as unsafe extern "C" fn()),
    (SUB2 as unsafe extern "C" fn()),
    (MUL2 as unsafe extern "C" fn()),
    (DIV2 as unsafe extern "C" fn()),
    (AND2 as unsafe extern "C" fn()),
    (ORA2 as unsafe extern "C" fn()),
    (EOR2 as unsafe extern "C" fn()),
    (SFT2 as unsafe extern "C" fn()),
    (JMI as unsafe extern "C" fn()),
    (INCr as unsafe extern "C" fn()),
    (POPr as unsafe extern "C" fn()),
    (NIPr as unsafe extern "C" fn()),
    (SWPr as unsafe extern "C" fn()),
    (ROTr as unsafe extern "C" fn()),
    (DUPr as unsafe extern "C" fn()),
    (OVRr as unsafe extern "C" fn()),
    (EQUr as unsafe extern "C" fn()),
    (NEQr as unsafe extern "C" fn()),
    (GTHr as unsafe extern "C" fn()),
    (LTHr as unsafe extern "C" fn()),
    (JMPr as unsafe extern "C" fn()),
    (JCNr as unsafe extern "C" fn()),
    (JSRr as unsafe extern "C" fn()),
    (STHr as unsafe extern "C" fn()),
    (LDZr as unsafe extern "C" fn()),
    (STZr as unsafe extern "C" fn()),
    (LDRr as unsafe extern "C" fn()),
    (STRr as unsafe extern "C" fn()),
    (LDAr as unsafe extern "C" fn()),
    (STAr as unsafe extern "C" fn()),
    (DEIr as unsafe extern "C" fn()),
    (DEOr as unsafe extern "C" fn()),
    (ADDr as unsafe extern "C" fn()),
    (SUBr as unsafe extern "C" fn()),
    (MULr as unsafe extern "C" fn()),
    (DIVr as unsafe extern "C" fn()),
    (ANDr as unsafe extern "C" fn()),
    (ORAr as unsafe extern "C" fn()),
    (EORr as unsafe extern "C" fn()),
    (SFTr as unsafe extern "C" fn()),
    (JSI as unsafe extern "C" fn()),
    (INC2r as unsafe extern "C" fn()),
    (POP2r as unsafe extern "C" fn()),
    (NIP2r as unsafe extern "C" fn()),
    (SWP2r as unsafe extern "C" fn()),
    (ROT2r as unsafe extern "C" fn()),
    (DUP2r as unsafe extern "C" fn()),
    (OVR2r as unsafe extern "C" fn()),
    (EQU2r as unsafe extern "C" fn()),
    (NEQ2r as unsafe extern "C" fn()),
    (GTH2r as unsafe extern "C" fn()),
    (LTH2r as unsafe extern "C" fn()),
    (JMP2r as unsafe extern "C" fn()),
    (JCN2r as unsafe extern "C" fn()),
    (JSR2r as unsafe extern "C" fn()),
    (STH2r as unsafe extern "C" fn()),
    (LDZ2r as unsafe extern "C" fn()),
    (STZ2r as unsafe extern "C" fn()),
    (LDR2r as unsafe extern "C" fn()),
    (STR2r as unsafe extern "C" fn()),
    (LDA2r as unsafe extern "C" fn()),
    (STA2r as unsafe extern "C" fn()),
    (DEI2r as unsafe extern "C" fn()),
    (DEO2r as unsafe extern "C" fn()),
    (ADD2r as unsafe extern "C" fn()),
    (SUB2r as unsafe extern "C" fn()),
    (MUL2r as unsafe extern "C" fn()),
    (DIV2r as unsafe extern "C" fn()),
    (AND2r as unsafe extern "C" fn()),
    (ORA2r as unsafe extern "C" fn()),
    (EOR2r as unsafe extern "C" fn()),
    (SFT2r as unsafe extern "C" fn()),
    (LIT as unsafe extern "C" fn()),
    (INCk as unsafe extern "C" fn()),
    (POPk as unsafe extern "C" fn()),
    (NIPk as unsafe extern "C" fn()),
    (SWPk as unsafe extern "C" fn()),
    (ROTk as unsafe extern "C" fn()),
    (DUPk as unsafe extern "C" fn()),
    (OVRk as unsafe extern "C" fn()),
    (EQUk as unsafe extern "C" fn()),
    (NEQk as unsafe extern "C" fn()),
    (GTHk as unsafe extern "C" fn()),
    (LTHk as unsafe extern "C" fn()),
    (JMPk as unsafe extern "C" fn()),
    (JCNk as unsafe extern "C" fn()),
    (JSRk as unsafe extern "C" fn()),
    (STHk as unsafe extern "C" fn()),
    (LDZk as unsafe extern "C" fn()),
    (STZk as unsafe extern "C" fn()),
    (LDRk as unsafe extern "C" fn()),
    (STRk as unsafe extern "C" fn()),
    (LDAk as unsafe extern "C" fn()),
    (STAk as unsafe extern "C" fn()),
    (DEIk as unsafe extern "C" fn()),
    (DEOk as unsafe extern "C" fn()),
    (ADDk as unsafe extern "C" fn()),
    (SUBk as unsafe extern "C" fn()),
    (MULk as unsafe extern "C" fn()),
    (DIVk as unsafe extern "C" fn()),
    (ANDk as unsafe extern "C" fn()),
    (ORAk as unsafe extern "C" fn()),
    (EORk as unsafe extern "C" fn()),
    (SFTk as unsafe extern "C" fn()),
    (LIT2 as unsafe extern "C" fn()),
    (INC2k as unsafe extern "C" fn()),
    (POP2k as unsafe extern "C" fn()),
    (NIP2k as unsafe extern "C" fn()),
    (SWP2k as unsafe extern "C" fn()),
    (ROT2k as unsafe extern "C" fn()),
    (DUP2k as unsafe extern "C" fn()),
    (OVR2k as unsafe extern "C" fn()),
    (EQU2k as unsafe extern "C" fn()),
    (NEQ2k as unsafe extern "C" fn()),
    (GTH2k as unsafe extern "C" fn()),
    (LTH2k as unsafe extern "C" fn()),
    (JMP2k as unsafe extern "C" fn()),
    (JCN2k as unsafe extern "C" fn()),
    (JSR2k as unsafe extern "C" fn()),
    (STH2k as unsafe extern "C" fn()),
    (LDZ2k as unsafe extern "C" fn()),
    (STZ2k as unsafe extern "C" fn()),
    (LDR2k as unsafe extern "C" fn()),
    (STR2k as unsafe extern "C" fn()),
    (LDA2k as unsafe extern "C" fn()),
    (STA2k as unsafe extern "C" fn()),
    (DEI2k as unsafe extern "C" fn()),
    (DEO2k as unsafe extern "C" fn()),
    (ADD2k as unsafe extern "C" fn()),
    (SUB2k as unsafe extern "C" fn()),
    (MUL2k as unsafe extern "C" fn()),
    (DIV2k as unsafe extern "C" fn()),
    (AND2k as unsafe extern "C" fn()),
    (ORA2k as unsafe extern "C" fn()),
    (EOR2k as unsafe extern "C" fn()),
    (SFT2k as unsafe extern "C" fn()),
    (LITr as unsafe extern "C" fn()),
    (INCkr as unsafe extern "C" fn()),
    (POPkr as unsafe extern "C" fn()),
    (NIPkr as unsafe extern "C" fn()),
    (SWPkr as unsafe extern "C" fn()),
    (ROTkr as unsafe extern "C" fn()),
    (DUPkr as unsafe extern "C" fn()),
    (OVRkr as unsafe extern "C" fn()),
    (EQUkr as unsafe extern "C" fn()),
    (NEQkr as unsafe extern "C" fn()),
    (GTHkr as unsafe extern "C" fn()),
    (LTHkr as unsafe extern "C" fn()),
    (JMPkr as unsafe extern "C" fn()),
    (JCNkr as unsafe extern "C" fn()),
    (JSRkr as unsafe extern "C" fn()),
    (STHkr as unsafe extern "C" fn()),
    (LDZkr as unsafe extern "C" fn()),
    (STZkr as unsafe extern "C" fn()),
    (LDRkr as unsafe extern "C" fn()),
    (STRkr as unsafe extern "C" fn()),
    (LDAkr as unsafe extern "C" fn()),
    (STAkr as unsafe extern "C" fn()),
    (DEIkr as unsafe extern "C" fn()),
    (DEOkr as unsafe extern "C" fn()),
    (ADDkr as unsafe extern "C" fn()),
    (SUBkr as unsafe extern "C" fn()),
    (MULkr as unsafe extern "C" fn()),
    (DIVkr as unsafe extern "C" fn()),
    (ANDkr as unsafe extern "C" fn()),
    (ORAkr as unsafe extern "C" fn()),
    (EORkr as unsafe extern "C" fn()),
    (SFTkr as unsafe extern "C" fn()),
    (LIT2r as unsafe extern "C" fn()),
    (INC2kr as unsafe extern "C" fn()),
    (POP2kr as unsafe extern "C" fn()),
    (NIP2kr as unsafe extern "C" fn()),
    (SWP2kr as unsafe extern "C" fn()),
    (ROT2kr as unsafe extern "C" fn()),
    (DUP2kr as unsafe extern "C" fn()),
    (OVR2kr as unsafe extern "C" fn()),
    (EQU2kr as unsafe extern "C" fn()),
    (NEQ2kr as unsafe extern "C" fn()),
    (GTH2kr as unsafe extern "C" fn()),
    (LTH2kr as unsafe extern "C" fn()),
    (JMP2kr as unsafe extern "C" fn()),
    (JCN2kr as unsafe extern "C" fn()),
    (JSR2kr as unsafe extern "C" fn()),
    (STH2kr as unsafe extern "C" fn()),
    (LDZ2kr as unsafe extern "C" fn()),
    (STZ2kr as unsafe extern "C" fn()),
    (LDR2kr as unsafe extern "C" fn()),
    (STR2kr as unsafe extern "C" fn()),
    (LDA2kr as unsafe extern "C" fn()),
    (STA2kr as unsafe extern "C" fn()),
    (DEI2kr as unsafe extern "C" fn()),
    (DEO2kr as unsafe extern "C" fn()),
    (ADD2kr as unsafe extern "C" fn()),
    (SUB2kr as unsafe extern "C" fn()),
    (MUL2kr as unsafe extern "C" fn()),
    (DIV2kr as unsafe extern "C" fn()),
    (AND2kr as unsafe extern "C" fn()),
    (ORA2kr as unsafe extern "C" fn()),
    (EOR2kr as unsafe extern "C" fn()),
    (SFT2kr as unsafe extern "C" fn()),
];

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
        let mut vm_native = Uxn::new(&cmd, &mut ram_native, Backend::Native);
        let mut vm_interp =
            Uxn::new(&cmd, &mut ram_interp, Backend::Interpreter);

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
