#![no_std]
#![feature(asm)]
#![feature(start)]
#![feature(naked_functions)]

use core::panic::PanicInfo;

extern "C" {
    fn _api_openwin(
        buf_addr: usize,
        xsize: usize,
        ysize: usize,
        col_inv: u8,
        title_addr: usize,
    ) -> usize;
    fn _api_boxfilwin(win: usize, x0: i32, y0: i32, x1: i32, y1: i32, col: u8);
    fn _api_getkey(mode: i32) -> u8;
}

#[no_mangle]
#[start]
pub extern "C" fn hrmain() {
    let buf: [u8; 150 * 70] = [0; 150 * 70];
    let win = unsafe {
        _api_openwin(
            buf.as_ptr() as usize,
            150,
            70,
            14,
            b"notrec".as_ptr() as usize,
        )
    };
    unsafe {
        _api_boxfilwin(win, 0, 50, 34, 69, 14);
        _api_boxfilwin(win, 115, 50, 149, 69, 14);
        _api_boxfilwin(win, 50, 30, 99, 49, 14);
    }
    loop {
        if unsafe { _api_getkey(1) } == 0x0a {
            break; // Enterならbreak
        }
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
