#![no_std]
#![feature(asm)]
#![feature(start)]
#![feature(naked_functions)]

use core::cmp::{max, min};
use core::mem::replace;
use core::panic::PanicInfo;

extern "C" {
    fn _api_putchar(chr: u8);

    fn _api_putstr0(string_ptr: usize);
    fn _api_openwin(
        buf_addr: usize,
        xsize: usize,
        ysize: usize,
        col_inv: i8,
        title_addr: usize,
    ) -> usize;
    fn _api_boxfilwin(win: usize, x0: i32, y0: i32, x1: i32, y1: i32, col: u8);
    fn _api_cmdline(buf_addr: usize, maxsize: usize);
    fn _api_getkey(mode: i32) -> u8;
    fn _api_fopen(string_addr: usize) -> usize;
    fn _api_fread(buf_addr: usize, maxsize: usize, fhandler_addr: usize) -> i32;
}

const MIN_WIDTH: usize = 20;
const MAX_WIDTH: usize = 126;
const DEFAULT_WIDTH: usize = 30;
const MIN_HEIGHT: usize = 1;
const MAX_HEIGHT: usize = 45;
const DEFAULT_HEIGHT: usize = 10;
const MIN_TAB: usize = 1;
const DEFAULT_TAB: usize = 4;

struct Viewer {
    width: usize,
    height: usize,
    tab: usize,
    filename_start: usize,
    filename_end: usize,
}

impl Viewer {
    fn new() -> Viewer {
        Viewer {
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            tab: DEFAULT_TAB,
            filename_start: 0,
            filename_end: 0,
        }
    }
}

#[no_mangle]
#[start]
pub extern "C" fn hrmain() {
    let mut buf: [u8; 30] = [0; 30];
    let winbuf: [u8; 1024 * 757] = [0; 1024 * 757];
    unsafe { _api_cmdline(buf.as_ptr() as usize, 30) };
    let mut v = Viewer::new();
    if let Err(e) = parse_options(&buf, &mut v) {
        unsafe { _api_putstr0(e.as_bytes().as_ptr() as usize) };
        end();
        return;
    }

    let win = unsafe {
        _api_openwin(
            winbuf.as_ptr() as usize,
            v.width * 8 + 16,
            v.height * 16 + 37,
            -1,
            b"tview".as_ptr() as usize,
        )
    };
    unsafe {
        _api_boxfilwin(
            win,
            6,
            27,
            v.width as i32 * 8 + 9,
            v.height as i32 * 16 + 30,
            7,
        )
    };

    let mut filename: [u8; 30] = [0; 30];
    (&mut filename[0..(v.filename_end - v.filename_start)])
        .copy_from_slice(&buf[v.filename_start..v.filename_end]);
    let fi = unsafe { _api_fopen(filename.as_ptr() as usize) };
    if fi == 0 {
        unsafe { _api_putstr0(b" FILE OPEN ERROR\n\0".as_ptr() as usize) };
        end();
        return;
    }
    loop {
        if unsafe { _api_getkey(1) } == 0x0a {
            break; // Enterならbreak
        }
    }
    v = Viewer::new();
    end();
}

fn parse_options(buf: &[u8], v: &mut Viewer) -> Result<(), &'static str> {
    let mut bi = 0;
    let mut p = buf[bi];
    while p > b' ' {
        bi += 1;
        p = buf[bi];
    }
    while buf[bi] != 0 {
        bi = skipspace(buf, bi);
        if buf[bi] == b'-' {
            if buf[bi + 1] == b'w' {
                let r = strtol(buf, bi + 2);
                v.width = min(max(r.0, MIN_WIDTH), MAX_WIDTH);
                bi = r.1;
            } else if buf[bi + 1] == b'h' {
                let r = strtol(buf, bi + 2);
                v.height = min(max(r.0, MIN_HEIGHT), MAX_HEIGHT);
                bi = r.1;
            } else if buf[bi + 1] == b't' {
                let r = strtol(buf, bi + 2);
                v.tab = if r.0 < MIN_TAB { MIN_TAB } else { r.0 };
                bi = r.1;
            } else {
                return Err(" INVALID OPTION\n >tview file [-w30 -h10 -t4]\n\0");
            }
        } else {
            if v.filename_start != 0 {
                return Err(" FILE NAME DUPLICATE\n >tview file [-w30 -h10 -t4]\n\0");
            }
            v.filename_start = bi;
            while (bi < (buf.len() - 1)) && (buf[bi] > b' ') {
                bi += 1;
            }
            v.filename_end = bi;
            if v.filename_start == v.filename_end {
                return Err(" FILE NAME NOT FOUND\n >tview file [-w30 -h10 -t4]\n\0");
            }
        }
    }
    Ok(())
}

fn skipspace(buf: &[u8], i: usize) -> usize {
    let mut i = i;
    while (i < (buf.len() - 1)) && (buf[i] == b' ') {
        i += 1;
    }
    i
}

fn strtol(buf: &[u8], i: usize) -> (usize, usize) {
    let mut n = 0;
    let mut i = i;
    while (i < (buf.len() - 1)) && (b'0' <= buf[i]) && (buf[i] <= b'9') {
        n *= 10;
        n += (buf[i] - b'0') as usize;
        i += 1;
    }
    (n, i)
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
