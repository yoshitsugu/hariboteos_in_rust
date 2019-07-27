#![no_std]
#![feature(asm)]
#![feature(start)]
#![feature(naked_functions)]

use core::panic::PanicInfo;

#[no_mangle]
#[start]
pub extern "C" fn haribote_os() {
    unsafe { *(0x00102600 as *mut u8) = 0 };
    put_char(b'h');
    put_char(b'e');
    put_char(b'l');
    put_char(b'l');
    put_char(b'o');
    end()
}

#[naked]
fn put_char(c: u8) {
    unsafe {
        asm!("MOV EDX,1
              MOV AL,[$0]
              INT 0x40" : : "r"(&c) : : "intel");
    }
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
        unsafe {
            asm!("HLT");
        }
    }
}
