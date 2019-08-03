#![no_std]
#![feature(asm)]
#![feature(start)]
#![feature(naked_functions)]

use core::panic::PanicInfo;

// use rand::prelude::*;

extern "C" {
    fn _api_initmalloc();
    fn _api_malloc(size: usize) -> usize;
    fn _api_free(addr: usize, size: usize);
    fn _api_linewin(sheet_index: usize, x0: i32, y0: i32, x1: i32, y1: i32, color: i32);
}

const SHEET_UNREFRESH_OFFSET: usize = 256;

#[no_mangle]
#[start]
pub extern "C" fn hrmain() {
    unsafe {
        _api_initmalloc();
    }
    let buf_addr = unsafe { _api_malloc(160 * 100) };
    let sheet_index = open_window(buf_addr, 160, 100, -1, b"lines".as_ptr() as usize) as usize;
    let sheet_index_nonrefresh = sheet_index + SHEET_UNREFRESH_OFFSET;
    for i in 0..8 {
        unsafe { _api_linewin(sheet_index_nonrefresh, 8, 26, 77, i * 9 + 26, i) }
        unsafe {
            _api_linewin(sheet_index_nonrefresh, 88, 26, i * 9 + 88, 89, i);
        }
    }
    refresh_window(sheet_index, 6, 26, 154, 90);
    loop {
        if get_key(1) == 0x0a {
            break;
        }
    }
    close_window(sheet_index);
    // let sheet_index = open_window(buf_addr, 150, 100, -1, b"star1".as_ptr() as usize) as usize;
    // box_fil_window(
    //     sheet_index + SHEET_UNREFRESH_OFFSET,
    //     6,
    //     26,
    //     143,
    //     93,
    //     0, /* 黒 */
    // );
    // let mut rng = StdRng::seed_from_u64(123);
    // for i in 0..50 {
    //     let x = (rng.next_u32() % 137 + 6) as i32;
    //     let y = (rng.next_u32() % 67 + 26) as i32;
    //     point_window(sheet_index + SHEET_UNREFRESH_OFFSET, x, y, 3 /* 黄 */);
    // }
    // refresh_window(sheet_index, 6, 26, 144, 94);
    unsafe {
        _api_free(buf_addr, 160 * 100);
    }
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

fn put_str_window(sheet_index: usize, x: i32, y: i32, color: i32, len: usize, string_addr: usize) {
    unsafe {
        asm!("
		MOV		EDX,6
		INT		0x40
        " : : "{EBX}"(sheet_index), "{ESI}"(x), "{EDI}"(y), "{EAX}"(color), "{ECX}"(len), "{EBP}"(string_addr) : : "intel");
    }
}

fn box_fil_window(sheet_index: usize, x0: i32, y0: i32, x1: i32, y1: i32, color: i32) {
    unsafe {
        asm!("
		MOV		EDX,7
		INT		0x40
        " : : "{EBX}"(sheet_index), "{EAX}"(x0), "{ECX}"(y0), "{ESI}"(x1), "{EDI}"(y1), "{EBP}"(color) : : "intel");
    }
}

fn point_window(sheet_index: usize, x: i32, y: i32, color: i32) {
    unsafe {
        asm!("
		MOV		EDX,11
		INT		0x40
        " : : "{EBX}"(sheet_index), "{ESI}"(x), "{EDI}"(y), "{EAX}"(color) : : "intel");
    }
}

fn refresh_window(sheet_index: usize, x0: i32, y0: i32, x1: i32, y1: i32) {
    unsafe {
        asm!("
		MOV		EDX,12
		INT		0x40
        " : : "{EBX}"(sheet_index), "{EAX}"(x0), "{ECX}"(y0), "{ESI}"(x1), "{EDI}"(y1) : : "intel");
    }
}

fn close_window(sheet_index: usize) {
    unsafe {
        asm!("
		MOV		EDX,14
		INT		0x40
        " : : "{EBX}"(sheet_index) : : "intel");
    }
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

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe {
            asm!("HLT");
        }
    }
}
