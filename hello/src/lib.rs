#![no_std]
#![feature(asm)]
#![feature(start)]
#![feature(naked_functions)]

use core::panic::PanicInfo;

#[no_mangle]
#[start]
pub extern "C" fn hrmain() {
    put_string(b"hello".as_ptr() as usize);
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
fn put_string(string_ptr: usize) {
    unsafe {
        asm!("MOV EDX,2
              MOV EBX,[$0]
              INT 0x40" : : "r"(&string_ptr) : : "intel");
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
