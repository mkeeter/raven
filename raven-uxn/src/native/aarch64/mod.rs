use crate::native::DeviceHandle;
use crate::Uxn;

pub fn entry(vm: &mut Uxn, dev: &mut DeviceHandle, pc: u16) -> u16 {
    // SAFETY: do you trust me?
    unsafe {
        aarch64_entry(
            vm.stack.data.as_mut_ptr(),
            &mut vm.stack.index as *mut _,
            vm.ret.data.as_mut_ptr(),
            &mut vm.ret.index as *mut _,
            (*vm.ram).as_mut_ptr(),
            pc,
            vm as *mut _,
            dev as *mut _,
        )
    }
}

#[cfg(target_os = "macos")]
core::arch::global_asm!(concat!(
    include_str!("macos.s"),
    include_str!("impl.s")
));

#[cfg(target_os = "linux")]
core::arch::global_asm!(concat!(
    include_str!("linux.s"),
    include_str!("impl.s")
));

extern "C" {
    #[allow(improper_ctypes)]
    fn aarch64_entry(
        stack: *mut u8,
        stack_index: *mut u8,
        ret: *mut u8,
        ret_index: *mut u8,
        ram: *mut u8,
        pc: u16,
        vm: *mut Uxn,
        dev: *mut DeviceHandle,
    ) -> u16;
}
