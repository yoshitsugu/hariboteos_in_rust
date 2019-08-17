#![no_std]
#![feature(asm)]
#![feature(start)]
#![feature(naked_functions)]

use core::panic::PanicInfo;

extern "C" {
    fn _api_putstr0(string_ptr: usize);
}

#[no_mangle]
#[start]
pub extern "C" fn hrmain() {
    let iroha: [u8; 9] = [0xb2, 0xdb, 0xca, 0xc6, 0xce, 0xcd, 0xc4, 0x0a, 0x00];
    unsafe { _api_putstr0(iroha.as_ptr() as usize) };
    end();
}

#[naked]
fn end() {
    unsafe {
        asm!("MOV EDX,4
              INT 0x40" : : : : "intel");
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("HLT") }
    }
}
