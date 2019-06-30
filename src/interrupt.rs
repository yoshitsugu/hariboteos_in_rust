use crate::asm::out8;
use crate::keyboard::{init_keyboard, wait_kbc_sendready};

const PIC0_ICW1: u32 = 0x0020;
pub const PIC0_OCW2: u32 = 0x0020;
const PIC0_IMR: u32 = 0x0021;
const PIC0_ICW2: u32 = 0x0021;
const PIC0_ICW3: u32 = 0x0021;
const PIC0_ICW4: u32 = 0x0021;
const PIC1_ICW1: u32 = 0x00a0;
pub const PIC1_OCW2: u32 = 0x00a0;
const PIC1_IMR: u32 = 0x00a1;
const PIC1_ICW2: u32 = 0x00a1;
const PIC1_ICW3: u32 = 0x00a1;
const PIC1_ICW4: u32 = 0x00a1;

pub const PORT_KEYCMD: u32 = 0x0064;
pub const PORT_KEYDAT: u32 = 0x60;
const KEYCMD_SENDTO_MOUSE: u8 = 0xd4;
const MOUSECMD_ENABLE: u8 = 0xf4;

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
    out8(PIC0_IMR, 0xf8); // PITとPIC1とキーボードを許可(11111000)
    out8(PIC1_IMR, 0xef); // マウスを許可(11101111)
    init_keyboard();
}

pub fn enable_mouse() {
    wait_kbc_sendready();
    out8(PORT_KEYCMD, KEYCMD_SENDTO_MOUSE);
    wait_kbc_sendready();
    out8(PORT_KEYDAT, MOUSECMD_ENABLE);
}
