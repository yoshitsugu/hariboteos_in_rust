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
        col_inv: i8,
        title_addr: usize,
    ) -> usize;
    fn _api_boxfilwin(win: usize, x0: i32, y0: i32, x1: i32, y1: i32, col: i8);
    fn _api_linewin(win: usize, x0: i32, y0: i32, x1: i32, y1: i32, col: i8);
    fn _api_getkey(mode: i32) -> u8;
}

#[no_mangle]
#[start]
pub extern "C" fn hrmain() {
    let buf: [u8; 216 * 237] = [0; 216 * 237];
    let points: [(i32, i32); 16] = [
        (204, 129),
        (195, 90),
        (172, 58),
        (137, 38),
        (98, 34),
        (61, 46),
        (31, 73),
        (15, 110),
        (15, 148),
        (31, 185),
        (61, 212),
        (98, 224),
        (137, 220),
        (172, 200),
        (195, 168),
        (204, 129),
    ];
    let win = unsafe {
        _api_openwin(
            buf.as_ptr() as usize,
            216,
            237,
            -1,
            b"bball".as_ptr() as usize,
        )
    };
    unsafe {
        _api_boxfilwin(win, 8, 29, 207, 228, 0);
    }
    for i in 0..=14 {
        for j in (i + 1)..=15 {
            let mut d = j - i;
            if d >= 8 {
                d = 15 - d;
            }
            if d != 0 {
                unsafe {
                    _api_linewin(
                        win,
                        points[i].0,
                        points[i].1,
                        points[j].0,
                        points[j].1,
                        (8 - d) as i8,
                    );
                }
            }
        }
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
