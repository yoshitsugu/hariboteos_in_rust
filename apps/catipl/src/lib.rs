#![no_std]
#![feature(asm)]
#![feature(start)]
#![feature(naked_functions)]

use core::fmt;
use core::panic::PanicInfo;

extern "C" {
    fn _api_putchar(chr: u8);
    fn _api_putstr0(string_ptr: usize);
    fn _api_fopen(string_addr: usize) -> usize;
    fn _api_fseek(fhandler_addr: usize, offset: i32, mode: i32);
    fn _api_fsize(fhandler_addr: usize, mode: i32) -> i32;
    fn _api_fread(buf_addr: usize, maxsize: usize, fhandler_addr: usize) -> i32;
}

const STRING_SIZE: usize = 30;

struct FileInfo {
    string: [u8; STRING_SIZE],
    ptr: usize,
}

#[no_mangle]
#[start]
pub extern "C" fn hrmain() {
    use core::fmt::Write;
    let fh_addr = unsafe { _api_fopen(b"ipl.asm".as_ptr() as usize) };
    if fh_addr == 0 {
        return;
    }
    unsafe { _api_fseek(fh_addr, 20, 0) };
    let filesize = unsafe { _api_fsize(fh_addr, 0) };
    let filepos = unsafe { _api_fsize(fh_addr, 1) };
    let mut file_info = FileInfo {
        string: [0; STRING_SIZE],
        ptr: 0,
    };
    write!(file_info, "size: {}, pos: {}\n", filesize, filepos).unwrap();
    unsafe { _api_putstr0(file_info.string.as_ptr() as usize) };
    let buf = [0];
    for i in 0..10 {
        if unsafe { _api_fread(buf.as_ptr() as usize, 1, fh_addr) } == 0 {
            break;
        }
        unsafe { _api_putchar(buf[0] as u8) };
    }
    end();
}

impl fmt::Write for FileInfo {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let str_bytes = s.as_bytes();
        for i in 0..str_bytes.len() {
            if self.ptr >= STRING_SIZE {
                break;
            }
            self.string[self.ptr] = str_bytes[i];
            self.ptr += 1;
        }
        Ok(())
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
        unsafe { asm!("HLT") }
    }
}
