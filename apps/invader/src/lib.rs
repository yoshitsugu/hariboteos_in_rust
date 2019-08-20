#![no_std]
#![feature(asm)]
#![feature(start)]
#![feature(naked_functions)]

use core::mem::swap;
use core::panic::PanicInfo;

const CHARSET: [u8; 16 * 8] = [
    /* invader(0) */
    0x00, 0x00, 0x00, 0x43, 0x5f, 0x5f, 0x5f, 0x7f, 0x1f, 0x1f, 0x1f, 0x1f, 0x00, 0x20, 0x3f, 0x00,
    /* invader(1) */
    0x00, 0x0f, 0x7f, 0xff, 0xcf, 0xcf, 0xcf, 0xff, 0xff, 0xe0, 0xff, 0xff, 0xc0, 0xc0, 0xc0, 0x00,
    /* invader(2) */
    0x00, 0xf0, 0xfe, 0xff, 0xf3, 0xf3, 0xf3, 0xff, 0xff, 0x07, 0xff, 0xff, 0x03, 0x03, 0x03, 0x00,
    /* invader(3) */
    0x00, 0x00, 0x00, 0xc2, 0xfa, 0xfa, 0xfa, 0xfe, 0xf8, 0xf8, 0xf8, 0xf8, 0x00, 0x04, 0xfc, 0x00,
    /* fighter(0) */
    0x00, 0x00, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x43, 0x47, 0x4f, 0x5f, 0x7f, 0x7f, 0x00,
    /* fighter(1) */
    0x18, 0x7e, 0xff, 0xc3, 0xc3, 0xc3, 0xc3, 0xff, 0xff, 0xff, 0xe7, 0xe7, 0xe7, 0xe7, 0xff, 0x00,
    /* fighter(2) */
    0x00, 0x00, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0xc2, 0xe2, 0xf2, 0xfa, 0xfe, 0xfe, 0x00,
    /* laser */
    0x00, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00,
];
const MAX_SHEETS: usize = 256;
const INVADER_STR: &[u8; 32] = b" abcd abcd abcd abcd abcd      \0";

extern "C" {
    fn _api_openwin(
        buf_addr: usize,
        xsize: usize,
        ysize: usize,
        col_inv: i8,
        title_addr: usize,
    ) -> usize;
    fn _api_boxfilwin(win: usize, x0: i32, y0: i32, x1: i32, y1: i32, col: u8);
    fn _api_putstrwin(win: usize, x: i32, y: i32, col: i32, len: usize, str_ptr: usize);
    fn _api_refreshwin(win: usize, x0: i32, y0: i32, x1: i32, y1: i32);
    fn _api_alloctimer() -> usize;
    fn _api_inittimer(timer_index: usize, data: i32);
    fn _api_settimer(timer_index: usize, time: i32);
    fn _api_getkey(mode: i32) -> u8;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Point {
    x: usize,
    y: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Left,
    Right,
}

struct MyState {
    high: usize,
    score: usize,
    x: usize,
    score_unit: usize,
}

impl MyState {
    fn new() -> MyState {
        MyState {
            high: 0,
            score: 0,
            x: 18,
            score_unit: 1,
        }
    }
}

struct LaserState {
    point: Point,
    wait: usize,
}

impl LaserState {
    fn new() -> LaserState {
        LaserState {
            point: Point { x: 0, y: 0 },
            wait: 0,
        }
    }
}

struct InvaderState {
    point: Point,
    direction: Direction,
    wait: usize,
    init_wait: usize,
    line: usize,
    map: [u8; 32 * 6],
}

impl InvaderState {
    fn new() -> InvaderState {
        let mut is = InvaderState {
            point: Point { x: 7, y: 1 },
            direction: Direction::Right,
            wait: 20,
            init_wait: 20,
            line: 6,
            map: [0; 32 * 6],
        };
        for i in 0..6 {
            for j in 0..27 {
                is.map[32 * i + j] = INVADER_STR[j];
            }
        }
        is
    }
}

struct KeyFlag {
    left: bool,
    right: bool,
    space: bool,
}

impl KeyFlag {
    fn new() -> KeyFlag {
        KeyFlag {
            left: false,
            right: false,
            space: false,
        }
    }
}

struct DisplayScore {
    s: [u8; 10],
    p: usize,
}

impl DisplayScore {
    fn new() -> DisplayScore {
        DisplayScore { s: [0; 10], p: 0 }
    }
}

impl core::fmt::Write for DisplayScore {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let str_bytes = s.as_bytes();
        for i in 0..str_bytes.len() {
            if self.p >= 10 {
                break;
            }
            self.s[self.p] = str_bytes[i];
            self.p += 1;
        }
        Ok(())
    }
}

#[no_mangle]
#[start]
pub extern "C" fn hrmain() {
    let mut buf: [u8; 336 * 261] = [0; 336 * 261];
    let win = unsafe {
        _api_openwin(
            buf.as_ptr() as usize,
            336,
            261,
            -1,
            b"invader".as_ptr() as usize,
        )
    };
    unsafe { _api_boxfilwin(win, 6, 27, 329, 254, 0) };
    let timer_index = unsafe { _api_alloctimer() };
    unsafe { _api_inittimer(timer_index, 128) };
    putstr(win, &mut buf, 22, 0, 7, b"HIGH:00000000");
    let mut m: MyState = MyState::new();
    let mut i: InvaderState = InvaderState::new();
    let mut l: LaserState = LaserState::new();
    let mut k: KeyFlag = KeyFlag::new();
    startup(&mut m, &mut i, &mut k, &mut l, win, &mut buf, timer_index);
    unsafe { _api_getkey(1) };
    end();
}

fn init(
    m: &mut MyState,
    i: &mut InvaderState,
    k: &mut KeyFlag,
    l: &mut LaserState,
    win: usize,
    buf: &mut [u8; 336 * 261],
    timer_index: usize,
    high: usize,
) {
    swap(m, &mut MyState::new());
    m.high = high;
    swap(i, &mut InvaderState::new());
    swap(k, &mut KeyFlag::new());
    swap(l, &mut LaserState::new());
    putstr(win, buf, 4, 0, 7, b"SCORE:00000000");
    putstr(win, buf, m.x, 13, 6, b"efg");
    wait(100, timer_index, k);
    for n in 0..6 {
        putstr(
            win,
            buf,
            i.point.x + 1,
            i.point.y + n,
            2,
            &i.map[(n * 32)..((n + 1) * 32)],
        );
    }
    wait(100, timer_index, k);
}

fn startup(
    m: &mut MyState,
    i: &mut InvaderState,
    k: &mut KeyFlag,
    l: &mut LaserState,
    win: usize,
    buf: &mut [u8; 336 * 261],
    timer_index: usize,
) {
    let mut high = 0;
    loop {
        init(m, i, k, l, win, buf, timer_index, high);
        main_loop(m, i, k, l, win, buf, timer_index);
        high = m.high;
    }
}

fn main_loop(
    m: &mut MyState,
    i: &mut InvaderState,
    k: &mut KeyFlag,
    l: &mut LaserState,
    win: usize,
    buf: &mut [u8; 336 * 261],
    timer_index: usize,
) {
    loop {
        if l.wait != 0 {
            l.wait -= 1;
            k.space = false;
        }
        wait(4, timer_index, k);

        // 自機の処理
        if k.left && m.x > 0 {
            m.x -= 1;
            putstr(win, buf, m.x, 13, 6, b"efg \0");
            k.left = false;
        }
        if k.right && m.x < 36 {
            m.x += 1;
            if m.x == 35 {
                putstr(win, buf, m.x, 13, 6, b" efg\0");
            } else {
                putstr(win, buf, m.x - 1, 13, 6, b"  efg\0");
            }
            k.right = false;
        }
        if k.space && l.wait == 0 {
            l.wait = 15;
            l.point.x = m.x + 1;
            l.point.y = 13;
        }

        // インベーダ移動
        if i.wait != 0 {
            i.wait -= 1;
        } else {
            i.wait = i.init_wait;
            if (i.point.x > 12 && i.direction == Direction::Right)
                || (i.point.x < 1 && i.direction == Direction::Left)
            {
                if i.point.y + i.line == 13 {
                    // GAME OVER
                    break;
                }
                i.direction = if i.direction == Direction::Right {
                    Direction::Left
                } else {
                    Direction::Right
                };
                putstr(
                    win,
                    buf,
                    i.point.x + 1,
                    i.point.y,
                    0,
                    b"                         \0",
                );
                i.point.y += 1;
            } else {
                i.point.x = if i.direction == Direction::Right {
                    i.point.x + 1
                } else {
                    i.point.x - 1
                };
            }

            for n in 0..i.line {
                putstr(
                    win,
                    buf,
                    i.point.x,
                    i.point.y + n,
                    2,
                    &i.map[(n * 32)..((n + 1) * 32)],
                );
            }
        }

        // レーザー処理
        if l.point.y > 0 {
            if l.point.y < 13 {
                if i.point.x < l.point.x
                    && l.point.x < (i.point.x + 25)
                    && i.point.y <= l.point.y
                    && l.point.y < i.point.y + i.line
                {
                    let n = l.point.y - i.point.y;
                    putstr(
                        win,
                        buf,
                        i.point.x,
                        l.point.y,
                        2,
                        &i.map[(n * 32)..((n + 1) * 32)],
                    );
                } else {
                    putstr(win, buf, l.point.x, l.point.y, 0, b" ");
                }
            }
            l.point.y -= 1;
            if l.point.y > 0 {
                putstr(win, buf, l.point.x, l.point.y, 3, b"h");
            } else {
                if m.score_unit >= 10 {
                    m.score_unit -= 10;
                }
                if m.score_unit == 0 {
                    m.score_unit = 1;
                }
            }
            if i.point.x < l.point.x
                && l.point.x < (i.point.x + 25)
                && i.point.y <= l.point.y
                && l.point.y < i.point.y + i.line
            {
                let mut p = (l.point.y - i.point.y) * 32 + (l.point.x - i.point.x);
                if i.map[p] != b' ' {
                    m.score += m.score_unit;
                    m.score_unit += 1;
                    let mut s = DisplayScore::new();
                    use core::fmt::Write;
                    write!(s, "{:<08}", m.score).unwrap();
                    putstr(win, buf, 10, 0, 7, &s.s);
                    if m.high < m.score {
                        m.high = m.score;
                        putstr(win, buf, 27, 0, 7, &s.s);
                    }
                    p -= 1;
                    while i.map[p] != b' ' {
                        p -= 1;
                    }
                    for pi in 1..5 {
                        i.map[p + pi] = b' ';
                    }
                    let n = l.point.y - i.point.y;
                    putstr(
                        win,
                        buf,
                        i.point.x,
                        l.point.y,
                        2,
                        &i.map[(n * 32)..((n + 1) * 32)],
                    );
                    let mut alive = false;
                    while i.line > 0 {
                        p = (i.line - 1) * 32;
                        while i.map[p] != 0 {
                            if i.map[p] != b' ' {
                                alive = true;
                                break;
                            }
                            p += 1;
                        }
                        if alive {
                            break;
                        }
                        i.line -= 1;
                    }
                    l.point.y = 0;
                    if !alive {
                        wait(100, timer_index, k);
                        let init_wait = i.init_wait;
                        swap(i, &mut InvaderState::new());
                        i.init_wait -= init_wait / 3;
                        l.wait = 0;
                        for n in 0..6 {
                            putstr(
                                win,
                                buf,
                                i.point.x + 1,
                                i.point.y + n,
                                2,
                                &i.map[(n * 32)..((n + 1) * 32)],
                            );
                        }
                        swap(k, &mut KeyFlag::new());
                        wait(100, timer_index, k)
                    }
                }
            }
        }
    }
    putstr(win, buf, 15, 6, 1, b"GAME OVER");
    wait(0, timer_index, k);
    for n in 1..14 {
        putstr(
            win,
            buf,
            0,
            n,
            0,
            b"                                        ",
        );
    }
}

fn putstr(win: usize, buf: &mut [u8; 336 * 261], x: usize, y: usize, col: i32, string: &[u8]) {
    let mut x = x * 8 + 8;
    let y = y * 16 + 29;
    let x0 = x;
    let i = string.len();
    unsafe {
        _api_boxfilwin(
            win + MAX_SHEETS,
            x as i32,
            y as i32,
            (x + i * 8) as i32,
            (y + 15) as i32,
            0,
        )
    };
    let mut q = buf.as_ptr() as usize + y * 336;
    for ci in 0..string.len() {
        let c = string[ci];
        if c == 0 {
            break;
        }
        if c != b' ' {
            if b'a' <= c && c <= b'h' {
                let p = CHARSET.as_ptr() as usize + 16 * (c - b'a') as usize;
                q += x;
                for i in 0..16 {
                    let pv = unsafe { *((p + i) as *const u8) };
                    for j in 0..8 {
                        if (pv & 1 << (7 - j)) != 0 {
                            let q_ptr = unsafe { &mut *((q + j) as *mut i8) };
                            *q_ptr = col as i8;
                        }
                    }
                    q += 336;
                }
                q -= 336 * 16 + x;
            } else {
                unsafe {
                    _api_putstrwin(
                        win + MAX_SHEETS,
                        x as i32,
                        y as i32,
                        col,
                        1,
                        [c, 0].as_ptr() as usize,
                    )
                };
            }
        }
        x += 8
    }
    unsafe {
        // 縁を再描画
        _api_boxfilwin(win + MAX_SHEETS, 2, 27, 5, 254, 8);
        _api_boxfilwin(win + MAX_SHEETS, 1, 27, 1, 254, 7);
        _api_boxfilwin(win + MAX_SHEETS, 0, 27, 0, 254, 8);
        _api_boxfilwin(win + MAX_SHEETS, 330, 27, 333, 254, 8);
        _api_boxfilwin(win + MAX_SHEETS, 334, 27, 334, 254, 15);
        _api_boxfilwin(win + MAX_SHEETS, 335, 27, 335, 254, 0);
        _api_refreshwin(
            win,
            (x0 - 8) as i32,
            y as i32,
            (x + 8) as i32,
            (y + 16) as i32,
        );
    }
}

fn wait(i: i32, timer_index: usize, keyflag: &mut KeyFlag) {
    let mut i = i;
    if i > 0 {
        unsafe { _api_settimer(timer_index, i) };
        i = 128;
    } else {
        i = 0x0a; // Enter
    }
    loop {
        let k = unsafe { _api_getkey(1) };
        if i == k as i32 {
            break;
        }
        keyflag.left = k == b'4';
        keyflag.right = k == b'6';
        keyflag.space = k == b' ';
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
