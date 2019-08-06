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
    fn _api_putstrwin(
        sheet_index: usize,
        x: i32,
        y: i32,
        color: i32,
        len: usize,
        string_addr: usize,
    );
    fn _api_beep(tone: i32);
}

const SHEET_UNREFRESH_OFFSET: usize = 256;

#[no_mangle]
#[start]
pub extern "C" fn hrmain() {
    let timer_index = alloc_timer();
    unsafe {
        _api_inittimer(timer_index.clone(), 128);
    }
    let mut i = 20000000;
    while i >= 20000 {
        unsafe {
            _api_beep(i);
            _api_settimer(timer_index, 1);
        }
        if get_key(1) != 128 {
            break;
        }
        i -= i / 100;
    }
    unsafe { _api_beep(0) };
    end();
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
