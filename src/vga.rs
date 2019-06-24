use core::fmt;
use lazy_static::lazy_static;

use crate::asm;
use crate::fonts::{FONTS, FONT_HEIGHT, FONT_WIDTH};

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
    asm::store_eflags(eflags);
}

pub fn init_screen(buf: usize) {
    use Color::*;
    let xsize = *SCREEN_WIDTH as isize;
    let ysize = *SCREEN_HEIGHT as isize;

    boxfill(buf, DarkCyan, 0, 0, xsize - 1, ysize - 29);
    boxfill(buf, LightGray, 0, ysize - 28, xsize - 1, ysize - 28);
    boxfill(buf, White, 0, ysize - 27, xsize - 1, ysize - 27);
    boxfill(buf, LightGray, 0, ysize - 26, xsize - 1, ysize - 1);

    boxfill(buf, White, 3, ysize - 24, 59, ysize - 24);
    boxfill(buf, White, 2, ysize - 24, 2, ysize - 4);
    boxfill(buf, DarkGray, 3, ysize - 4, 59, ysize - 4);
    boxfill(buf, DarkGray, 59, ysize - 23, 59, ysize - 5);
    boxfill(buf, Black, 2, ysize - 3, 59, ysize - 3);
    boxfill(buf, Black, 60, ysize - 24, 60, ysize - 3);

    boxfill(buf, DarkGray, xsize - 47, ysize - 24, xsize - 4, ysize - 24);
    boxfill(buf, DarkGray, xsize - 47, ysize - 23, xsize - 47, ysize - 4);
    boxfill(buf, White, xsize - 47, ysize - 3, xsize - 4, ysize - 3);
    boxfill(buf, White, xsize - 3, ysize - 24, xsize - 3, ysize - 3);
}

pub fn boxfill(buf: usize, color: Color, x0: isize, y0: isize, x1: isize, y1: isize) {
    for y in y0..=y1 {
        for x in x0..=x1 {
            let ptr = unsafe { &mut *((buf as isize + y * *SCREEN_WIDTH as isize + x) as *mut u8) };
            *ptr = color as u8;
        }
    }
}

pub fn print_char(buf: usize, char: u8, color: Color, startx: isize, starty: isize) {
    let font = FONTS[char as usize];
    let color = color as u8;
    let offset = startx + starty * *SCREEN_WIDTH as isize;
    for y in 0..FONT_HEIGHT {
        for x in 0..FONT_WIDTH {
            if font[y][x] {
                let cell = (y * *SCREEN_WIDTH as usize + x) as isize;
                let ptr = unsafe { &mut *((buf as isize + cell + offset) as *mut u8) };
                *ptr = color;
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
    color: Color,
}

impl ScreenWriter {
    pub fn new(buf_addr: Option<usize>, color: Color, x: usize, y: usize) -> ScreenWriter {
        ScreenWriter {
            buf_addr: buf_addr,
            initial_x: x,
            x,
            y,
            color,
        }
    }

    fn newline(&mut self) {
        self.x = self.initial_x;
        self.y = self.y + FONT_HEIGHT;
    }
}

impl fmt::Write for ScreenWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let str_bytes = s.as_bytes();
        let height = *SCREEN_HEIGHT as usize;
        let width = *SCREEN_WIDTH as usize;
        for i in 0..str_bytes.len() {
            if str_bytes[i] == b'\n' {
                self.newline();
                return Ok(());
            }
            let buf_addr = if let Some(b) = self.buf_addr {
                b
            } else {
                *VRAM_ADDR
            };
            if self.x + FONT_WIDTH < width && self.y + FONT_HEIGHT < height {
                print_char(
                    buf_addr,
                    str_bytes[i],
                    self.color,
                    self.x as isize,
                    self.y as isize,
                );
            } else if self.y + FONT_HEIGHT * 2 < height {
                // 1行ずらせば入る場合は1行ずらしてから表示
                self.newline();
                print_char(
                    buf_addr,
                    str_bytes[i],
                    self.color,
                    self.x as isize,
                    self.y as isize,
                );
            }
            // 次の文字用の位置に移動
            if self.x + FONT_WIDTH < width {
                self.x = self.x + FONT_WIDTH;
            } else if self.y + FONT_HEIGHT < height {
                self.newline();
            } else {
                self.x = width;
                self.y = height;
            }
        }
        Ok(())
    }
}
