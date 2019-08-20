#![no_std]
#![feature(asm)]
#![feature(start)]
#![feature(naked_functions)]

use core::fmt;
use core::panic::PanicInfo;

const INVALID: i32 = -0x7fffffff;

extern "C" {
    fn _api_putstr0(string_ptr: usize);
    fn _api_cmdline(buf_addr: usize, maxsize: usize);
}

const RESULT_LENGTH: usize = 30;

struct CalcResult {
    r: [u8; RESULT_LENGTH],
    p: usize,
}

impl CalcResult {
    fn new() -> CalcResult {
        CalcResult {
            r: [0; RESULT_LENGTH],
            p: 0,
        }
    }
}

impl fmt::Write for CalcResult {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let str_bytes = s.as_bytes();
        for i in 0..str_bytes.len() {
            if self.p >= RESULT_LENGTH {
                break;
            }
            self.r[self.p] = str_bytes[i];
            self.p += 1;
        }
        Ok(())
    }
}

#[no_mangle]
#[start]
pub extern "C" fn hrmain() {
    use core::fmt::Write;
    let buf: [u8; 30] = [0; 30];
    unsafe { _api_cmdline(buf.as_ptr() as usize, 30) };
    let mut pi = 0;
    let mut p = buf[pi];
    while p > b' ' {
        pi += 1;
        p = buf[pi];
    }
    let (i, _) = getnum(&buf[pi..(buf.len())], 0, 9);
    if i == INVALID {
        unsafe { _api_putstr0(b"error!\n\0".as_ptr() as usize) };
    } else {
        let mut r = CalcResult::new();
        write!(r, "= {} = 0x{:x}\n\0", i, i).unwrap();
        unsafe { _api_putstr0(r.r.as_ptr() as usize) };
    }
    end();
}

fn skipspace(buf: &[u8], i: usize) -> usize {
    let mut i = i;
    while i < (buf.len() - 1) && buf[i] == b' ' {
        i += 1;
    }
    i
}

fn getnum(buf: &[u8], bi: usize, priority: i32) -> (i32, usize) {
    let mut i;
    let mut bi = skipspace(buf, bi);

    // 単項演算子
    if buf[bi] == b'+' {
        bi = skipspace(buf, bi + 1);
        let r = getnum(buf, bi, 0);
        i = r.0;
        bi = r.1;
    } else if buf[bi] == b'-' {
        bi = skipspace(buf, bi + 1);
        let r = getnum(buf, bi, 0);
        i = r.0;
        bi = r.1;
        if i != INVALID {
            i = -i;
        }
    } else if buf[bi] == b'~' {
        bi = skipspace(buf, bi + 1);
        let r = getnum(buf, bi, 0);
        i = r.0;
        bi = r.1;
        if i != INVALID {
            i = !i;
        }
    } else if buf[bi] == b'(' {
        bi = skipspace(buf, bi + 1);
        let r = getnum(buf, bi, 9);
        i = r.0;
        bi = r.1;
        if buf[bi] == b')' {
            bi = skipspace(buf, bi + 1);
        } else {
            i = INVALID;
        }
    } else if b'0' <= buf[bi] && buf[bi] <= b'9' {
        i = (buf[bi] - b'0') as i32;
        if bi < (buf.len() - 1) {
            bi += 1;
            while bi < (buf.len() - 1) && b'0' <= buf[bi] && buf[bi] <= b'9' {
                i *= 10;
                i += (buf[bi] - b'0') as i32;
                bi += 1;
            }
        }
    } else {
        i = INVALID;
    }

    // 二項演算子
    loop {
        if i == INVALID {
            break;
        }
        bi = skipspace(buf, bi);
        if buf[bi] == b'+' && priority > 2 {
            bi = skipspace(buf, bi + 1);
            let r = getnum(buf, bi, 2);
            bi = r.1;
            if r.0 != INVALID {
                i += r.0;
            } else {
                i = INVALID;
            }
        } else if buf[bi] == b'-' && priority > 2 {
            bi = skipspace(buf, bi + 1);
            let r = getnum(buf, bi, 2);
            bi = r.1;
            if r.0 != INVALID {
                i -= r.0;
            } else {
                i = INVALID;
            }
        } else if buf[bi] == b'*' && priority > 1 {
            bi = skipspace(buf, bi + 1);
            let r = getnum(buf, bi, 1);
            bi = r.1;
            if r.0 != INVALID {
                i *= r.0;
            } else {
                i = INVALID;
            }
        } else if buf[bi] == b'/' && priority > 1 {
            bi = skipspace(buf, bi + 1);
            let r = getnum(buf, bi, 1);
            bi = r.1;
            if r.0 != INVALID && r.0 != 0 {
                i /= r.0;
            } else {
                i = INVALID;
            }
        } else if buf[bi] == b'%' && priority > 1 {
            bi = skipspace(buf, bi + 1);
            let r = getnum(buf, bi, 1);
            bi = r.1;
            if r.0 != INVALID && r.0 != 0 {
                i %= r.0;
            } else {
                i = INVALID;
            }
        } else if buf[bi] == b'<' && buf[bi + 1] == b'<' && priority > 3 {
            bi = skipspace(buf, bi + 2);
            let r = getnum(buf, bi, 3);
            bi = r.1;
            if r.0 != INVALID && r.0 != 0 {
                i <<= r.0;
            } else {
                i = INVALID;
            }
        } else if buf[bi] == b'>' && buf[bi + 1] == b'>' && priority > 3 {
            bi = skipspace(buf, bi + 2);
            let r = getnum(buf, bi, 3);
            bi = r.1;
            if r.0 != INVALID && r.0 != 0 {
                i >>= r.0;
            } else {
                i = INVALID;
            }
        } else if buf[bi] == b'&' && priority > 4 {
            bi = skipspace(buf, bi + 1);
            let r = getnum(buf, bi, 4);
            bi = r.1;
            if r.0 != INVALID {
                i &= r.0;
            } else {
                i = INVALID;
            }
        } else if buf[bi] == b'^' && priority > 5 {
            bi = skipspace(buf, bi + 1);
            let r = getnum(buf, bi, 5);
            bi = r.1;
            if r.0 != INVALID {
                i ^= r.0;
            } else {
                i = INVALID;
            }
        } else if buf[bi] == b'|' && priority > 6 {
            bi = skipspace(buf, bi + 1);
            let r = getnum(buf, bi, 6);
            bi = r.1;
            if r.0 != INVALID {
                i |= r.0;
            } else {
                i = INVALID;
            }
        } else {
            break;
        }
    }
    (i, bi)
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
