#![no_std]
#![feature(asm)]
#![feature(start)]
#![feature(naked_functions)]

use core::cmp::{max, min};
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
    fn _api_putstrwin(
        sheet_index: usize,
        x: i32,
        y: i32,
        color: i32,
        len: usize,
        string_addr: usize,
    );
    fn _api_boxfilwin(win: usize, x0: i32, y0: i32, x1: i32, y1: i32, col: u8);
    fn _api_refreshwin(sheet_index: usize, x0: i32, y0: i32, x1: i32, y1: i32);
    fn _api_cmdline(buf_addr: usize, maxsize: usize);
    fn _api_getkey(mode: i32) -> u8;
    fn _api_fopen(string_addr: usize) -> usize;
    fn _api_fclose(fhandle: usize);
    fn _api_fsize(fhandle: usize, mode: i32) -> usize;
    fn _api_fread(buf_addr: usize, maxsize: usize, fhandler_addr: usize) -> i32;
    fn _api_getlang() -> usize;
}

const MAX_SHEETS: usize = 256;

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
    lang: usize,
}

impl Viewer {
    fn new() -> Viewer {
        Viewer {
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            tab: DEFAULT_TAB,
            filename_start: 0,
            filename_end: 0,
            lang: 0,
        }
    }
}

#[no_mangle]
#[start]
pub extern "C" fn hrmain() {
    let buf: [u8; 30] = [0; 30];
    let winbuf: [u8; 1024 * 757] = [0; 1024 * 757];
    let mut textbuf: [u8; 240 * 1024] = [0; 240 * 1024];
    unsafe { _api_cmdline(buf.as_ptr() as usize, 30) };
    let mut v = Viewer::new();
    v.lang = unsafe { _api_getlang() };
    if let Err(e) = parse_options(&buf, &mut v) {
        unsafe { _api_putstr0(e.as_bytes().as_ptr() as usize) };
        end();
        return;
    }

    let win = init_window(winbuf.as_ptr() as usize, &v);

    if let Err(e) = load_file(&buf, &mut textbuf, &v) {
        unsafe { _api_putstr0(e.as_bytes().as_ptr() as usize) };
        end();
        return;
    }
    main_loop(win, &textbuf, &mut v);
    end();
}

fn init_window(winbuf_addr: usize, v: &Viewer) -> usize {
    let win = unsafe {
        _api_openwin(
            winbuf_addr,
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
    win
}

fn main_loop(win: usize, textbuf: &[u8], v: &mut Viewer) {
    let mut ti = 1;
    let mut xskip = 0;
    let mut spd_x = 1;
    let mut spd_y = 1;
    loop {
        textview(win, ti, v, textbuf, xskip);
        let k = unsafe { _api_getkey(1) };
        if k == b'Q' || k == b'q' {
            break;
        } else if b'A' <= k && k <= b'F' {
            spd_x = 1 << (k - b'A');
        } else if b'a' <= k && k <= b'f' {
            spd_y = 1 << (k - b'a');
        } else if k == b'<' && v.tab > 1 {
            v.tab /= 2;
        } else if k == b'>' && v.tab < 256 {
            v.tab *= 2;
        } else if k == b'4' {
            loop {
                if xskip >= spd_x {
                    xskip -= spd_x;
                }
                if unsafe { _api_getkey(0) } != b'4' {
                    break;
                }
            }
        } else if k == b'6' {
            loop {
                xskip += spd_x;
                if unsafe { _api_getkey(0) } != b'6' {
                    break;
                }
            }
        } else if k == b'8' {
            loop {
                for _ in 0..spd_y {
                    if ti < 2 {
                        break;
                    }
                    ti -= 1;
                    while textbuf[ti - 1] != 0x0a {
                        ti -= 1;
                    }
                }
                if unsafe { _api_getkey(0) } != b'8' {
                    break;
                }
            }
        } else if k == b'2' {
            loop {
                for _ in 0..spd_y {
                    let mut ci = ti;
                    while textbuf[ci] != 0 && textbuf[ci] != 0x0a {
                        ci += 1;
                    }
                    if textbuf[ci] == 0 {
                        break;
                    }
                    ti = ci + 1;
                }
                if unsafe { _api_getkey(0) } != b'2' {
                    break;
                }
            }
        }
    }
}

fn textview(win: usize, ti: usize, v: &Viewer, textbuf: &[u8], xskip: usize) {
    let mut ti = ti;
    unsafe {
        _api_boxfilwin(
            win + MAX_SHEETS,
            8,
            29,
            v.width as i32 * 8 + 7,
            v.height as i32 * 16 + 28,
            7,
        )
    };

    for i in 0..v.height {
        ti = lineview(win, i * 16 + 29, ti, v, textbuf, xskip);
    }
    unsafe {
        _api_refreshwin(
            win,
            8,
            29,
            v.width as i32 * 8 + 8,
            v.height as i32 * 16 + 29,
        )
    }
}

fn lineview(win: usize, y: usize, ti: usize, v: &Viewer, textbuf: &[u8], xskip: usize) -> usize {
    let mut ti = ti;
    let mut x = -(xskip as i32);
    let mut s: [u8; 130] = [0; 130];
    let w = v.width;
    loop {
        let p = if ti < textbuf.len() { textbuf[ti] } else { 0 };
        if p == 0 {
            break;
        }
        if p == 0x0a {
            ti += 1;
            break;
        }
        if v.lang == 0 {
            // ASCII
            if p == 0x09 {
                x = puttab(x, w, xskip, &mut s, v.tab);
            } else {
                if 0 <= x && x <= w as i32 {
                    s[x as usize] = textbuf[ti];
                }
                x += 1;
            }
            ti += 1;
        } else if v.lang == 1 {
            // SJIS
            if p == 0x09 {
                x = puttab(x, w, xskip, &mut s, v.tab);
                ti += 1;
            } else if (0x81 <= p && p <= 0x9f) || (0xe0 <= p && p <= 0xfc) {
                if x == -1 {
                    s[0] = b' ';
                }
                if 0 <= x && x < (w as i32 - 1) {
                    s[x as usize] = p;
                    s[x as usize + 1] = if (ti + 1) < textbuf.len() {
                        textbuf[ti + 1]
                    } else {
                        0
                    };
                }
                if x == (w as i32 - 1) {
                    s[x as usize] = b' ';
                }
                x += 2;
                ti += 2;
            } else {
                if 0 <= x && x < w as i32 {
                    s[x as usize] = p;
                }
                x += 1;
                ti += 1;
            }
        } else if v.lang == 2 {
            // EUC
            if p == 0x09 {
                x = puttab(x, w, xskip, &mut s, v.tab);
                ti += 1;
            } else if 0xa1 <= p && p <= 0xfe {
                if x == -1 {
                    s[0] = b' ';
                }
                if 0 <= x && x < (w as i32 - 1) {
                    s[x as usize] = p;
                    s[x as usize + 1] = if (ti + 1) < textbuf.len() {
                        textbuf[ti + 1]
                    } else {
                        0
                    };
                }
                if x == (w as i32 - 1) {
                    s[x as usize] = b' ';
                }
                x += 2;
                ti += 2;
            } else {
                if 0 <= x && x < w as i32 {
                    s[x as usize] = p;
                }
                x += 1;
                ti += 1;
            }
        }
    }
    if x > w as i32 {
        x = w as i32;
    }
    if x > 0 {
        s[x as usize] = 0;
        unsafe {
            _api_putstrwin(
                win + MAX_SHEETS,
                8,
                y as i32,
                0,
                x as usize,
                s.as_ptr() as usize,
            )
        };
    }
    ti
}

fn puttab(x: i32, w: usize, xskip: usize, s: &mut [u8], tab: usize) -> i32 {
    let mut x = x;
    loop {
        if 0 <= x && x < w as i32 {
            s[x as usize] = b' ';
        }
        x += 1;
        if (x + xskip as i32) % (tab as i32) == 0 {
            break;
        }
    }
    x
}

fn load_file(buf: &[u8], textbuf: &mut [u8; 240 * 1024], v: &Viewer) -> Result<(), &'static str> {
    let mut filename: [u8; 30] = [0; 30];
    (&mut filename[0..(v.filename_end - v.filename_start)])
        .copy_from_slice(&buf[v.filename_start..v.filename_end]);
    let fi = unsafe { _api_fopen(filename.as_ptr() as usize) };
    if fi == 0 {
        return Err(" FILE OPEN ERROR\n\0");
    }
    let size = unsafe { _api_fsize(fi, 0) };
    let mut j = size;
    if size >= (240 * 1024 - 1) {
        j = 240 * 1024 - 2;
    }
    textbuf[0] = 0x0a; // 番兵用の改行
    unsafe { _api_fread(textbuf[1..(240 * 1024)].as_ptr() as usize, j as usize, fi) };
    unsafe { _api_fclose(fi) };
    let mut ti = 1;
    let mut ti2 = 1;
    while ti < textbuf.len() && textbuf[ti] != 0 {
        if textbuf[ti] != 0x0d {
            textbuf[ti2] = textbuf[ti];
            ti2 += 1;
        }
        ti += 1;
    }
    Ok(())
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
