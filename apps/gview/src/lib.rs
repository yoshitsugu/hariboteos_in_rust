#![no_std]
#![feature(asm)]
#![feature(start)]
#![feature(naked_functions)]

use core::panic::PanicInfo;

extern "C" {
    fn _api_putchar(chr: u8);
    fn _api_putstr0(string_ptr: usize);
    fn _api_initmalloc();
    fn _api_malloc(size: usize) -> usize;
    fn _api_free(addr: usize, size: usize);
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

    fn _info_BMP(env: *const DllStrpicenv, info: *const i32, size: usize, fp: *const u8) -> i32;
    fn _decode0_BMP(
        env: *const DllStrpicenv,
        size: usize,
        fp: *const u8,
        b_type: i32,
        buf: *const i32,
        skip: i32,
    ) -> i32;

    fn info_JPEG(env: *const DllStrpicenv, info: *const i32, size: usize, fp: *const u8) -> i32;
    fn decode0_JPEG(
        env: *const DllStrpicenv,
        size: usize,
        fp: *const u8,
        b_type: i32,
        buf: *const i32,
        skip: i32,
    ) -> i32;
}

#[repr(C)]
struct DllStrpicenv {
    work: [i32; 64 * 1024 / 4],
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(C)]
struct Rgb {
    b: u8,
    g: u8,
    r: u8,
    t: u8,
}

impl Rgb {
    fn new() -> Rgb {
        Rgb {
            b: 0,
            g: 0,
            r: 0,
            t: 0,
        }
    }
}

struct Prim {
    num: [u8; 5],
    ptr: usize,
}

#[no_mangle]
#[start]
pub extern "C" fn hrmain() {
    unsafe { _api_initmalloc() };
    let filebuf: [u8; 512 * 1024] = [0; 512 * 1024];
    let mut winbuf: [u8; 1040 * 805] = [0; 1040 * 805];
    let s: [u8; 32] = [0; 32];
    let env = DllStrpicenv {
        work: [0; 64 * 1024 / 4],
    };
    let info: [i32; 8] = [0; 8];

    // ファイル名の先頭まですすめる
    unsafe { _api_cmdline(s.as_ptr() as usize, 30) };
    let mut si = 0;
    while s[si] > b' ' {
        si += 1;
    }
    while s[si] == b' ' {
        si += 1;
    }

    // ファイル読み込み
    let fi = unsafe { _api_fopen(s[si..s.len()].as_ptr() as usize) };
    if fi == 0 {
        error(b"FILE NOT FOUND\n\0");
        return;
    }
    let size = unsafe { _api_fsize(fi, 0) };
    if size > 512 * 1024 {
        error(b"FILE TOO LARGE\n\0");
        return;
    }
    unsafe { _api_fread(filebuf.as_ptr() as usize, size as usize, fi) };
    unsafe { _api_fclose(fi) };

    // ファイルタイプチェック
    if unsafe {
        _info_BMP(
            &env as *const DllStrpicenv,
            info.as_ptr(),
            size,
            filebuf.as_ptr(),
        )
    } == 0
    {
        if unsafe {
            info_JPEG(
                &env as *const DllStrpicenv,
                info.as_ptr(),
                size,
                filebuf.as_ptr(),
            )
        } == 0
        {
            error(b"FILE TYPE UNKNOWN\n\0");
            return;
        }
    }
    /* どちらかのinfo関数が成功すると、以下の情報がinfoに入っている */
    /*	info[0] : ファイルタイプ (1:BMP, 2:JPEG) */
    /*	info[1] : カラー情報 */
    /*	info[2] : xsize */
    /*	info[3] : ysize */

    if info[2] > 1024 || info[3] > 768 {
        error(b"PICTURE TOO LARGE\n\0");
        return;
    }

    // ウィンドウの準備
    let mut xsize = info[2] + 16;
    if xsize < 136 {
        xsize = 136;
    }
    let win = unsafe {
        _api_openwin(
            winbuf.as_ptr() as usize,
            xsize as usize,
            (info[3] + 37 - 1) as usize,
            -1,
            b"gview".as_ptr() as usize,
        )
    };

    // ファイル内容を画像データに変換
    let mut decode_result = 0;

    let picbuf_addr = unsafe { _api_malloc(1024 * 768 * 4) };
    let picbuf = unsafe { &mut *(picbuf_addr as *mut [i32; 1024 * 768]) };
    *picbuf = [0; 1024 * 768];

    if info[0] == 1 {
        decode_result = unsafe {
            _decode0_BMP(
                &env as *const DllStrpicenv,
                size,
                filebuf.as_ptr(),
                4,
                picbuf.as_ptr(),
                0,
            )
        };
    } else {
        decode_result = unsafe {
            decode0_JPEG(
                &env as *const DllStrpicenv,
                size,
                filebuf.as_ptr(),
                4,
                picbuf.as_ptr(),
                0,
            )
        };
    }

    if decode_result != 0 {
        error(b"DECODE ERROR\n\0");
        return;
    }

    for i in 0..info[3] {
        let p = ((i + 29) * xsize + (xsize - info[2]) / 2) as usize;
        let q = (i * info[2]) as usize;
        for j in 0..info[2] {
            let rgb = unsafe { *(&picbuf[q + j as usize] as *const i32 as *const Rgb ) };
            winbuf[p + j as usize] = rgb2pal(rgb.r as i32, rgb.g as i32, rgb.b as i32, j as i32, i as i32);
        }
    }
    unsafe {
        _api_refreshwin(
            win,
            (xsize - info[2]) / 2,
            29,
            (xsize - info[2]) / 2 + info[2],
            29 + info[3],
        )
    }

    loop {
        let k = unsafe { _api_getkey(1) };
        if k == b'Q' || k == b'q' {
            break;
        }
    }
    end();
}

fn error(msg: &[u8]) {
    unsafe { _api_putstr0(msg.as_ptr() as usize) };
    end();
}

fn rgb2pal(r: i32, g: i32, b: i32, x: i32, y: i32) -> u8 {
    let table: [i32; 4] = [3, 1, 0, 2];
    let x = x & 1;
    let y = y & 1;
    let i = table[(x + y * 2) as usize];
    let mut r = (r * 21) / 256;
    let mut g = (g * 21) / 256;
    let mut b = (b * 21) / 256;
    r = (r + i) / 4;
    g = (g + i) / 4;
    b = (b + i) / 4;
    return (16 + r + g * 6 + b * 36) as u8;
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
