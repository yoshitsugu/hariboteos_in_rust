use crate::asm;

const COLOR_PALETTE: [[u8; 3]; 16] = [
	[0x00, 0x00, 0x00],	/*  0:黒 */
	[0xff, 0x00, 0x00],	/*  1:明るい赤 */
	[0x00, 0xff, 0x00],	/*  2:明るい緑 */
	[0xff, 0xff, 0x00],	/*  3:明るい黄色 */
	[0x00, 0x00, 0xff],	/*  4:明るい青 */
	[0xff, 0x00, 0xff],	/*  5:明るい紫 */
	[0x00, 0xff, 0xff],	/*  6:明るい水色 */
	[0xff, 0xff, 0xff],	/*  7:白 */
	[0xc6, 0xc6, 0xc6],	/*  8:明るい灰色 */
	[0x84, 0x00, 0x00],	/*  9:暗い赤 */
	[0x00, 0x84, 0x00],	/* 10:暗い緑 */
	[0x84, 0x84, 0x00],	/* 11:暗い黄色 */
	[0x00, 0x00, 0x84],	/* 12:暗い青 */
	[0x84, 0x00, 0x84],	/* 13:暗い紫 */
	[0x00, 0x84, 0x84],	/* 14:暗い水色 */
	[0x84, 0x84, 0x84]	/* 15:暗い灰色 */
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

pub fn set_palette() {
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

pub fn boxfill8(ptr: *mut u8, offset: isize, color: Color, x0: isize, y0: isize, x1: isize, y1: isize) {
    for y in y0..=y1 {
        for x in x0..=x1 {
            let ptr = unsafe { &mut *(ptr.offset(y * offset + x)) };
            *ptr = color as u8;
        }
    }
}