#![no_std]
#![feature(asm)]
#![feature(start)]
#![feature(naked_functions)]

use core::panic::PanicInfo;

extern "C" {
    fn _api_putchar(chr: u8);
    fn _api_putstr0(string_ptr: usize);
    fn _api_fopen(string_addr: usize) -> usize;
    fn _api_fread(buf_addr: usize, maxsize: usize, fhandler_addr: usize) -> i32;
    fn _api_cmdline(buf_addr: usize, maxsize: usize) -> usize;
}

#[no_mangle]
#[start]
pub extern "C" fn hrmain() {
    let cmdline: [u8; 30] = [0; 30];
    unsafe { _api_cmdline(cmdline.as_ptr() as usize, 30) };
    let mut filename_index = 0;
    while cmdline[filename_index] > b' ' {
        filename_index += 1;
    }
    while cmdline[filename_index] == b' ' {
        filename_index += 1;
    }

    let fh_addr = unsafe { _api_fopen(cmdline.as_ptr() as usize + filename_index) };
    if fh_addr != 0 {
        loop {
            let b: u8 = 0;
            if unsafe { _api_fread(&b as *const u8 as usize, 1, fh_addr) } == 0 {
                break;
            }
            unsafe { _api_putchar(b) };
        }
    } else {
        unsafe { _api_putstr0(b"File not found".as_ptr() as usize) };
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
