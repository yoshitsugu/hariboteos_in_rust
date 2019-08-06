#![no_std]
#![feature(asm)]
#![feature(start)]
#![feature(naked_functions)]

use core::fmt;
use core::panic::PanicInfo;

extern "C" {
    fn _api_initmalloc();
    fn _api_malloc(size: usize) -> usize;
    fn _api_free(addr: usize, size: usize);
    fn _api_linewin(sheet_index: usize, x0: i32, y0: i32, x1: i32, y1: i32, color: i32);
    fn _api_inittimer(timer_index: usize, data: i32);
    fn _api_settimer(timer_index: usize, timer: i32);
    fn _api_boxfilwin(sheet_index: usize, x0: i32, y0: i32, x1: i32, y1: i32, color: i32);
    fn _api_refreshwin(sheet_index: usize, x0: i32, y0: i32, x1: i32, y1: i32);
    fn _api_putstrwin(
        sheet_index: usize,
        x: i32,
        y: i32,
        color: i32,
        len: usize,
        string_addr: usize,
    );
    fn _api_getkey(mode: i32);
    fn _api_beep(tone: i32);
}

const SHEET_UNREFRESH_OFFSET: usize = 256;

#[no_mangle]
#[start]
pub extern "C" fn hrmain() {
    unsafe { _api_initmalloc() };
    let buf_addr = unsafe { _api_malloc(144 * 164) };
    let sheet_index = open_window(buf_addr, 144, 164, -1, b"color".as_ptr() as usize);
    // for y in 0..128 {
    //     for x in 0..128 {
    //         let r = x * 2;
    //         let g = y * 2;
    //         let b = 0;
    //         let ptr = unsafe { &mut *((buf_addr + x + 8 + (y + 28) * 144) as *mut u8) };
    //         *ptr = (16 + (r / 43) + (g / 43) * 6 + (b / 43) * 36) as u8;
    //     }
    // }
    for y in 0..128 {
        for x in 0..128 {
            let ptr = unsafe { &mut *((buf_addr + x + 8 + (y + 28) * 144) as *mut u8) };
            *ptr = rgb2pal(x as i32 * 2, y as i32 * 2, 0, x as i32, y as i32);
        }
    }

    unsafe {
        _api_refreshwin(sheet_index as usize, 8, 28, 136, 156);
    }
    unsafe { _api_getkey(1) };
    end();
}

fn rgb2pal(r: i32, g: i32, b: i32, x: i32, y: i32) -> u8 {
    let table: [i32; 4] = [3, 1, 0, 2];
    let x = x & 1;
    let y = y & 1;
    let i = table[(x + y * 2) as usize];
    let r = (r * 21) / 256;
    let g = (g * 21) / 256;
    let b = (b * 21) / 256;
    let r = (r + i) / 4;
    let g = (g + i) / 4;
    let b = (b + i) / 4;
    return (16 + r + g * 6 + b * 36) as u8;
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

fn open_window(
    buf_addr: usize,
    xsize: usize,
    ysize: usize,
    background_color: i8,
    title_addr: usize,
) -> i32 {
    let mut ret: i32;
    unsafe {
        asm!("
		MOV		EDX,5
		INT		0x40
        " : "={EAX}"(ret) : "{EBX}"(buf_addr), "{ESI}"(xsize), "{EDI}"(ysize), "{EAX}"(background_color as i32), "{ECX}"(title_addr) : : "intel");
    }
    ret
}

fn get_key(mode: i32) -> usize {
    let mut key: usize;
    unsafe {
        asm!("
		MOV		EDX,15
		INT		0x40
        " : "={EAX}"(key) : "{EAX}"(mode) : : "intel");
    }
    key
}

fn alloc_timer() -> usize {
    let mut timer_index: usize;
    unsafe {
        asm!("
		MOV		EDX,16
		INT		0x40
        " : "={EAX}"(timer_index) : : : "intel");
    }
    timer_index
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("HLT") }
    }
}
