use crate::asm::out8;

const PIC0_ICW1: u32 = 0x0020;
const PIC0_OCW2: u32 = 0x0020;
const PIC0_IMR: u32 = 0x0021;
const PIC0_ICW2: u32 = 0x0021;
const PIC0_ICW3: u32 = 0x0021;
const PIC0_ICW4: u32 = 0x0021;
const PIC1_ICW1: u32 = 0x00a0;
const PIC1_OCW2: u32 = 0x00a0;
const PIC1_IMR: u32 = 0x00a1;
const PIC1_ICW2: u32 = 0x00a1;
const PIC1_ICW3: u32 = 0x00a1;
const PIC1_ICW4: u32 = 0x00a1;

pub fn init() {
    out8(PIC0_IMR, 0xff); // 全ての割り込みを受け付けない
    out8(PIC1_IMR, 0xff); // 全ての割り込みを受け付けない

    out8(PIC0_ICW1, 0x11); // エッジトリガモード
    out8(PIC0_ICW2, 0x20); // IRQ0-7は、INT20-27で受ける
    out8(PIC0_ICW3, 1 << 2); // PIC1はIRQ2にて接続
    out8(PIC0_ICW4, 0x01); // ノンバッファモード

    out8(PIC1_ICW1, 0x11); // エッジトリガモード
    out8(PIC1_ICW2, 0x28); // IRQ8-15は、INT28-2fで受ける
    out8(PIC1_ICW3, 2); // PIC1はIRQ2にて接続
    out8(PIC1_ICW4, 0x01); // ノンバッファモード

    out8(PIC0_IMR, 0xfb); // 11111011 PIC1以外は全て禁止
    out8(PIC1_IMR, 0xff); // 11111111 全ての割り込みを受け付けない
}

pub fn allow_input() {
    out8(PIC0_IMR, 0xf9); // PIC1とキーボードを許可(11111001)
    out8(PIC1_IMR, 0xef); // マウスを許可(11101111)
}

pub extern "C" fn inthandler21() {
    use crate::vga::{Color, Screen, ScreenWriter};
    let mut screen = Screen::new();
    screen.boxfill8(Color::Black, 0, 0, 32 * 8 - 1, 15);
    let mut writer = ScreenWriter::new(Screen::new(), Color::White, 0, 0);
    use core::fmt::Write;
    write!(writer, "INT 21 (IRQ-1) : PS/2 keyboard").unwrap();
    loop {
        crate::asm::hlt();
    }
}

pub extern "C" fn inthandler2c() {
    use crate::vga::{Color, Screen, ScreenWriter};
    let mut screen = Screen::new();
    screen.boxfill8(Color::Black, 0, 0, 32 * 8 - 1, 15);
    let mut writer = ScreenWriter::new(Screen::new(), Color::White, 0, 0);
    use core::fmt::Write;
    write!(writer, "INT 2C (IRQ-12) : PS/2 mouse").unwrap();
    loop {
        crate::asm::hlt();
    }
}
