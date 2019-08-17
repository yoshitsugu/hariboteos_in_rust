use core::fmt;
use core::fmt::Write;
use lazy_static::lazy_static;

use crate::asm;
use crate::fonts::{HANKAKU, HANKAKU_HEIGHT, HANKAKU_WIDTH};
use crate::mt::{LangMode, TaskManager, TASK_MANAGER_ADDR};
use crate::NIHONGO_ADDR;

const COLOR_PALETTE: [[u8; 3]; 16] = [
    [0x00, 0x00, 0x00], /*  0:黒 */
    [0xff, 0x00, 0x00], /*  1:明るい赤 */
    [0x00, 0xff, 0x00], /*  2:明るい緑 */
    [0xff, 0xff, 0x00], /*  3:明るい黄色 */
    [0x00, 0x00, 0xff], /*  4:明るい青 */
    [0xff, 0x00, 0xff], /*  5:明るい紫 */
    [0x00, 0xff, 0xff], /*  6:明るい水色 */
    [0xff, 0xff, 0xff], /*  7:白 */
    [0xc6, 0xc6, 0xc6], /*  8:明るい灰色 */
    [0x84, 0x00, 0x00], /*  9:暗い赤 */
    [0x00, 0x84, 0x00], /* 10:暗い緑 */
    [0x84, 0x84, 0x00], /* 11:暗い黄色 */
    [0x00, 0x00, 0x84], /* 12:暗い青 */
    [0x84, 0x00, 0x84], /* 13:暗い紫 */
    [0x00, 0x84, 0x84], /* 14:暗い水色 */
    [0x84, 0x84, 0x84], /* 15:暗い灰色 */
];

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    LightRed = 1,
    LightGreen = 2,
    LightYellow = 3,
    LightBlue = 4,
    LightPurple = 5,
    LightCyan = 6,
    White = 7,
    LightGray = 8,
    DarkRed = 9,
    DarkGreen = 10,
    DarkYellow = 11,
    DarkBlue = 12,
    DarkPurple = 13,
    DarkCyan = 14,
    DarkGray = 15,
}

pub fn to_color(n: i8) -> Option<Color> {
    use Color::*;
    match n {
        0 => Some(Black),
        1 => Some(LightRed),
        2 => Some(LightGreen),
        3 => Some(LightYellow),
        4 => Some(LightBlue),
        5 => Some(LightPurple),
        6 => Some(LightCyan),
        7 => Some(White),
        8 => Some(LightGray),
        9 => Some(DarkRed),
        10 => Some(DarkGreen),
        11 => Some(DarkYellow),
        12 => Some(DarkBlue),
        13 => Some(DarkPurple),
        14 => Some(DarkCyan),
        15 => Some(DarkGray),
        _ => None,
    }
}

pub const MAX_BLOCK_SIZE: usize = 16;

lazy_static! {
    pub static ref SCREEN_WIDTH: i16 = unsafe { *(0x0ff4 as *const i16) };
    pub static ref SCREEN_HEIGHT: i16 = unsafe { *(0x0ff6 as *const i16) };
    pub static ref VRAM_ADDR: usize = unsafe { *(0xff8 as *const usize) };
}

pub fn init_palette() {
    let eflags = asm::load_eflags();
    asm::cli();
    asm::out8(0x03c8, 0);
    for i in 0..16 {
        // 書き込むときは上位2ビットを0にしないといけない。See: http://oswiki.osask.jp/?VGA#o2d4bfd3
        asm::out8(0x03c9, COLOR_PALETTE[i][0] / 4);
        asm::out8(0x03c9, COLOR_PALETTE[i][1] / 4);
        asm::out8(0x03c9, COLOR_PALETTE[i][2] / 4);
    }
    let mut color_palette2: [u8; 216 * 3] = [0; 216 * 3];
    for b in 0..6 {
        for g in 0..6 {
            for r in 0..6 {
                color_palette2[(r + g * 6 + b * 36) * 3 + 0] = r as u8 * 51;
                color_palette2[(r + g * 6 + b * 36) * 3 + 1] = g as u8 * 51;
                color_palette2[(r + g * 6 + b * 36) * 3 + 2] = b as u8 * 51;
            }
        }
    }
    for i in 0..216 {
        asm::out8(0x03c9, color_palette2[i * 3] / 4);
        asm::out8(0x03c9, color_palette2[i * 3 + 1] / 4);
        asm::out8(0x03c9, color_palette2[i * 3 + 2] / 4);
    }
    asm::store_eflags(eflags);
}

pub fn init_screen(buf: usize) {
    use Color::*;
    let xsize = *SCREEN_WIDTH as isize;
    let ysize = *SCREEN_HEIGHT as isize;

    boxfill(buf, xsize, DarkCyan, 0, 0, xsize - 1, ysize - 29);
    boxfill(buf, xsize, LightGray, 0, ysize - 28, xsize - 1, ysize - 28);
    boxfill(buf, xsize, White, 0, ysize - 27, xsize - 1, ysize - 27);
    boxfill(buf, xsize, LightGray, 0, ysize - 26, xsize - 1, ysize - 1);

    boxfill(buf, xsize, White, 3, ysize - 24, 59, ysize - 24);
    boxfill(buf, xsize, White, 2, ysize - 24, 2, ysize - 4);
    boxfill(buf, xsize, DarkGray, 3, ysize - 4, 59, ysize - 4);
    boxfill(buf, xsize, DarkGray, 59, ysize - 23, 59, ysize - 5);
    boxfill(buf, xsize, Black, 2, ysize - 3, 59, ysize - 3);
    boxfill(buf, xsize, Black, 60, ysize - 24, 60, ysize - 3);

    boxfill(
        buf,
        xsize,
        DarkGray,
        xsize - 47,
        ysize - 24,
        xsize - 4,
        ysize - 24,
    );
    boxfill(
        buf,
        xsize,
        DarkGray,
        xsize - 47,
        ysize - 23,
        xsize - 47,
        ysize - 4,
    );
    boxfill(
        buf,
        xsize,
        White,
        xsize - 47,
        ysize - 3,
        xsize - 4,
        ysize - 3,
    );
    boxfill(
        buf,
        xsize,
        White,
        xsize - 3,
        ysize - 24,
        xsize - 3,
        ysize - 3,
    );
}

pub fn boxfill(buf: usize, xsize: isize, color: Color, x0: isize, y0: isize, x1: isize, y1: isize) {
    for y in y0..=y1 {
        for x in x0..=x1 {
            let ptr = unsafe { &mut *((buf as isize + y * xsize + x) as *mut u8) };
            *ptr = color as u8;
        }
    }
}

pub fn print_char(buf: usize, xsize: usize, chr: u8, color: Color, startx: isize, starty: isize) {
    let task_manager = unsafe { &mut *(TASK_MANAGER_ADDR as *mut TaskManager) };
    let task_index = task_manager.now_index();
    let task = &task_manager.tasks_data[task_index];
    let offset = startx + starty * xsize as isize;
    if task.lang_mode == LangMode::En {
        let font = HANKAKU[chr as usize];
        for y in 0..HANKAKU_HEIGHT {
            for x in 0..HANKAKU_WIDTH {
                if font[y][x] {
                    let cell = (y * xsize + x) as isize;
                    let ptr = unsafe { &mut *((buf as isize + cell + offset) as *mut Color) };
                    *ptr = color;
                }
            }
        }
    } else if task.lang_mode == LangMode::JpJis {
        let nihongo_addr = unsafe { *(NIHONGO_ADDR as *const usize) };
        let nihongos = unsafe { *(nihongo_addr as *const [[u8; HANKAKU_HEIGHT]; 256]) };
        let nihongo = nihongos[chr as usize];
        for y in 0..HANKAKU_HEIGHT {
            let n = nihongo[y];
            for x in 0..HANKAKU_WIDTH {
                if (n & (1 << (7 - x))) != 0 {
                    let cell = (y * xsize + x) as isize;
                    let ptr = unsafe { &mut *((buf as isize + cell + offset) as *mut Color) };
                    *ptr = color;
                }
            }
        }
    }
}

// 本では画像としてレンダリングできるサイズ可変になっているが、Rustでのとりまわしが面倒だったので一旦16固定にしている。
// const generics ( https://github.com/rust-lang/rfcs/blob/master/text/2000-const-generics.md )が使えれば解決しそう？
pub fn putblock(
    buf: usize,
    bxsize: isize,
    image: [[Color; MAX_BLOCK_SIZE]; MAX_BLOCK_SIZE],
    ixsize: isize,
    iysize: isize,
    px0: isize,
    py0: isize,
) {
    for y in 0..iysize {
        for x in 0..ixsize {
            let ptr = unsafe { &mut *((buf as isize + (py0 + y) * bxsize + (px0 + x)) as *mut u8) };
            *ptr = image[y as usize][x as usize] as u8;
        }
    }
}

pub struct ScreenWriter {
    buf_addr: Option<usize>,
    initial_x: usize,
    x: usize,
    y: usize,
    xsize: usize,
    ysize: usize,
    color: Color,
}

impl ScreenWriter {
    pub fn new(
        buf_addr: Option<usize>,
        color: Color,
        x: usize,
        y: usize,
        xsize: usize,
        ysize: usize,
    ) -> ScreenWriter {
        ScreenWriter {
            buf_addr: buf_addr,
            initial_x: x,
            x,
            y,
            xsize,
            ysize,
            color,
        }
    }

    fn newline(&mut self) {
        self.x = self.initial_x;
        self.y = self.y + HANKAKU_HEIGHT;
    }
}

impl fmt::Write for ScreenWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let str_bytes = s.as_bytes();
        let height = self.ysize;
        let width = self.xsize;
        for i in 0..str_bytes.len() {
            if str_bytes[i] == b'\n' {
                self.newline();
                continue;
            }
            let buf_addr = if let Some(b) = self.buf_addr {
                b
            } else {
                *VRAM_ADDR
            };
            if self.x + HANKAKU_WIDTH <= width && self.y + HANKAKU_HEIGHT <= height {
                print_char(
                    buf_addr,
                    self.xsize,
                    str_bytes[i],
                    self.color,
                    self.x as isize,
                    self.y as isize,
                );
            } else if self.y + HANKAKU_HEIGHT * 2 < height {
                // 1行ずらせば入る場合は1行ずらしてから表示
                self.newline();
                print_char(
                    buf_addr,
                    self.xsize,
                    str_bytes[i],
                    self.color,
                    self.x as isize,
                    self.y as isize,
                );
            }
            // 次の文字用の位置に移動
            if self.x + HANKAKU_WIDTH < width {
                self.x = self.x + HANKAKU_WIDTH;
            } else if self.y + HANKAKU_HEIGHT < height {
                self.newline();
            } else {
                self.x = width;
                self.y = height;
            }
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! write_with_bg {
    ($sheet_manager: expr, $sheet_addr: expr, $width: expr, $height: expr, $x: expr, $y: expr, $fg: expr, $bg: expr, $length: expr, $($arg: tt)* ) => {{
        let buf_addr = $sheet_manager.get_buf_addr($sheet_addr);
        crate::vga::boxfill(buf_addr, $width as isize, $bg, $x as isize, $y as isize, $x as isize + 8 * $length as isize - 1, $y as isize + 15);
        let mut writer = crate::vga::ScreenWriter::new(
                    Some(buf_addr),
                    $fg,
                    $x as usize,
                    $y as usize,
                    $width as usize,
                    $height as usize);
        use core::fmt::Write;
        write!(writer, $($arg)*).unwrap();
        $sheet_manager.refresh($sheet_addr, $x as i32, $y as i32, $x as i32 + $length as i32 * 8, $height as i32);
    }}
}

pub fn make_window(buf: usize, xsize: isize, ysize: isize, title: &str, active: bool) {
    boxfill(buf, xsize, Color::LightGray, 0, 0, xsize - 1, 0);
    boxfill(buf, xsize, Color::White, 1, 1, xsize - 2, 1);
    boxfill(buf, xsize, Color::LightGray, 0, 0, 0, ysize - 1);
    boxfill(buf, xsize, Color::White, 1, 1, 1, ysize - 2);
    boxfill(
        buf,
        xsize,
        Color::DarkGray,
        xsize - 2,
        1,
        xsize - 2,
        ysize - 2,
    );
    boxfill(buf, xsize, Color::Black, xsize - 1, 0, xsize - 1, ysize - 1);
    boxfill(buf, xsize, Color::LightGray, 2, 2, xsize - 3, ysize - 3);
    boxfill(
        buf,
        xsize,
        Color::DarkGray,
        1,
        ysize - 2,
        xsize - 2,
        ysize - 2,
    );
    boxfill(buf, xsize, Color::Black, 0, ysize - 1, xsize - 1, ysize - 1);
    make_wtitle(buf, xsize, ysize, title, active);
}

pub fn make_wtitle(buf: usize, xsize: isize, ysize: isize, title: &str, active: bool) {
    let closebtn: [&[u8; 16]; 14] = [
        b"OOOOOOOOOOOOOOO@",
        b"OQQQQQQQQQQQQQ$@",
        b"OQQQQQQQQQQQQQ$@",
        b"OQQQ@@QQQQ@@QQ$@",
        b"OQQQQ@@QQ@@QQQ$@",
        b"OQQQQQ@@@@QQQQ$@",
        b"OQQQQQQ@@QQQQQ$@",
        b"OQQQQQ@@@@QQQQ$@",
        b"OQQQQ@@QQ@@QQQ$@",
        b"OQQQ@@QQQQ@@QQ$@",
        b"OQQQQQQQQQQQQQ$@",
        b"OQQQQQQQQQQQQQ$@",
        b"O$$$$$$$$$$$$$$@",
        b"@@@@@@@@@@@@@@@@",
    ];
    let title_bg: Color;
    let title_fg: Color;
    if active {
        title_bg = Color::DarkBlue;
        title_fg = Color::White;
    } else {
        title_bg = Color::DarkGray;
        title_fg = Color::LightGray;
    }
    boxfill(buf, xsize, title_bg, 3, 3, xsize - 4, 20);
    let mut writer = ScreenWriter::new(Some(buf), title_fg, 24, 4, xsize as usize, ysize as usize);
    write!(writer, "{}", title).unwrap();
    for y in 0..14 {
        let y = y as usize;
        for x in 0..16 {
            let x = x as usize;
            let c = closebtn[y][x];
            let color: Color;
            if c == b'@' {
                color = Color::Black
            } else if c == b'$' {
                color = Color::DarkGray;
            } else if c == b'Q' {
                color = Color::LightGray;
            } else {
                color = Color::White
            }
            let ptr = unsafe {
                &mut *((buf + (5 + y) * xsize as usize + (xsize as usize - 21 + x)) as *mut Color)
            };
            *ptr = color;
        }
    }
}

pub fn make_textbox(
    buf: usize,
    bxsize: isize,
    x0: isize,
    y0: isize,
    sx: isize,
    sy: isize,
    c: Color,
) {
    let x1 = x0 + sx;
    let y1 = y0 + sy;
    boxfill(buf, bxsize, Color::DarkGray, x0 - 2, y0 - 3, x1 + 1, y0 - 3);
    boxfill(buf, bxsize, Color::DarkGray, x0 - 3, y0 - 3, x0 - 3, y1 + 1);
    boxfill(buf, bxsize, Color::White, x0 - 3, y1 + 2, x1 + 1, y1 + 2);
    boxfill(buf, bxsize, Color::White, x1 + 2, y0 - 3, x1 + 2, y1 + 2);
    boxfill(buf, bxsize, Color::Black, x0 - 1, y0 - 2, x1 + 0, y0 - 2);
    boxfill(buf, bxsize, Color::Black, x0 - 2, y0 - 2, x0 - 2, y1 + 0);
    boxfill(
        buf,
        bxsize,
        Color::LightGray,
        x0 - 2,
        y1 + 1,
        x1 + 0,
        y1 + 1,
    );
    boxfill(
        buf,
        bxsize,
        Color::LightGray,
        x1 + 1,
        y0 - 2,
        x1 + 1,
        y1 + 1,
    );
    boxfill(buf, bxsize, c, x0 - 1, y0 - 1, x1 + 0, y1 + 0);
}

pub fn draw_line(buf_addr: usize, bxsize: i32, x0: i32, y0: i32, x1: i32, y1: i32, color: i32) {
    let mut dx = x1 - x0;
    let mut dy = y1 - y0;
    let mut x = x0 << 10;
    let mut y = y0 << 10;
    let length: i32;
    if dx < 0 {
        dx = -dx;
    }
    if dy < 0 {
        dy = -dy;
    }
    if dx >= dy {
        length = dx + 1;
        if x0 > x1 {
            dx = -1024;
        } else {
            dx = 1024;
        }
        if y0 <= y1 {
            dy = ((y1 - y0 + 1) << 10) / length;
        } else {
            dy = ((y1 - y0 - 1) << 10) / length;
        }
    } else {
        length = dy + 1;
        if y0 > y1 {
            dy = -1024;
        } else {
            dy = 1024;
        }
        if x0 <= x1 {
            dx = ((x1 - x0 + 1) << 10) / length;
        } else {
            dx = ((x1 - x0 - 1) << 10) / length;
        }
    }

    for _ in 0..length {
        let ptr = unsafe { &mut *((buf_addr as i32 + (y >> 10) * bxsize + (x >> 10)) as *mut u8) };
        *ptr = color as u8;
        x += dx;
        y += dy;
    }
}

pub fn toggle_title_color(buf_addr: usize, xsize: usize, to_active: bool) {
    let bg_active = Color::DarkBlue;
    let fg_active = Color::White;
    let bg_inactive = Color::DarkGray;
    let fg_inactive = Color::LightGray;
    let bg_from = if to_active { bg_inactive } else { bg_active };
    let fg_from = if to_active { fg_inactive } else { fg_active };
    let bg_to = if to_active { bg_active } else { bg_inactive };
    let fg_to = if to_active { fg_active } else { fg_inactive };
    for y in 3..=20 {
        for x in 3..=(xsize - 4) {
            let c = unsafe { &mut *((buf_addr + y * xsize + x) as *mut Color) };
            if *c == fg_from && x <= xsize - 22 {
                *c = fg_to;
            } else if *c == bg_from {
                *c = bg_to;
            }
        }
    }
}
