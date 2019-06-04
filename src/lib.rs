#![no_std]
#![feature(asm)]
#![feature(start)]

use core::panic::PanicInfo;

#[no_mangle]
fn hlt() {
    unsafe {
        asm!("hlt");
    }
}

#[no_mangle]
fn show_white(i: u32) {
    let a: u8 = 15;
    let ptr = unsafe { &mut *(i as *mut u8) };
    *ptr = a 
}

#[no_mangle]
#[start]
pub extern "C" fn haribote_os() -> ! {
    for i in 0xa000..0xaffff {
      show_white(i);
    }
    loop {
        hlt()
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // println!("{}", info);
    loop {
        hlt()
    }
}
