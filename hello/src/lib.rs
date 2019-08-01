#![no_std]
#![feature(asm)]
#![feature(start)]
#![feature(naked_functions)]

use core::panic::PanicInfo;

#[no_mangle]
#[start]
pub extern "C" fn hrmain() {
    let buf: [u8; 150 * 50] = [0; 150 * 50];
    let sheet_index = open_window(
        buf.as_ptr() as usize,
        150,
        50,
        -1,
        b"hello".as_ptr() as usize,
    );
    box_fil_window(sheet_index as usize, 8, 36, 141, 43, 3);
    put_str_window(
        sheet_index as usize,
        28,
        28,
        0,
        12,
        b"hello again".as_ptr() as usize,
    );
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

#[no_mangle]
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

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe {
            asm!("HLT");
        }
    }
}
