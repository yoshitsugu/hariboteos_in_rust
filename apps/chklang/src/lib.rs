#![no_std]
#![feature(asm)]
#![feature(start)]
#![feature(naked_functions)]

use core::panic::PanicInfo;

extern "C" {
    fn _api_putstr0(string_ptr: usize);
    fn _api_getlang() -> usize;
}

#[no_mangle]
#[start]
pub extern "C" fn hrmain() {
    let langmode = unsafe { _api_getlang() };
    // 日本語シフトJISモード
    let s1: [u8; 23] = [
        0x93, 0xfa, 0x96, 0x7b, 0x8c, 0xea, 0x83, 0x56, 0x83, 0x74, 0x83, 0x67, 0x4a, 0x49, 0x53,
        0x83, 0x82, 0x81, 0x5b, 0x83, 0x68, 0x0a, 0x00,
    ];
    // 日本語EUCモード
    let s2: [u8; 17] = [
        0xc6, 0xfc, 0xcb, 0xdc, 0xb8, 0xec, 0x45, 0x55, 0x43, 0xa5, 0xe2, 0xa1, 0xbc, 0xa5, 0xc9,
        0x0a, 0x00,
    ];
    match langmode {
        0 => unsafe {
            _api_putstr0(b"English ASCII mode\n\0".as_ptr() as usize);
        },
        1 => unsafe {
            _api_putstr0(s1.as_ptr() as usize);
        },
        2 => unsafe {
            _api_putstr0(s2.as_ptr() as usize);
        },
        _ => unsafe {
            // Nothing to do
        },
    }
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
