#![no_std]
#![feature(asm)]
#![feature(start)]
#![feature(naked_functions)]

use core::fmt;
use core::panic::PanicInfo;

const MAX: usize = 1000;

extern "C" {
    fn _api_putstr0(string_ptr: usize);
}

struct Prim {
    num: [u8; 5],
    ptr: usize,
}

#[no_mangle]
#[start]
pub extern "C" fn hrmain() {
    let mut flag: [bool; MAX] = [false; MAX];
    use core::fmt::Write;
 
    for i in 2..MAX {
        if !flag[i] {
            let mut prim = Prim {
                num: [0; 5],
                ptr: 0,
            };
            write!(prim, "{} ", i).unwrap();
            unsafe {
                _api_putstr0(prim.num.as_ptr() as usize);
            }
            let mut j = i * 2;
            while j < MAX {
                flag[j] = true;
                j += i;
            }
        }
    }
    end();
}

impl fmt::Write for Prim {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let str_bytes = s.as_bytes();
        for i in 0..str_bytes.len() {
            if self.ptr >= 5 {
                break;
            }
            self.num[self.ptr] = str_bytes[i];
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
