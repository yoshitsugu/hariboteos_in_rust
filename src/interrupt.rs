use crate::asm::{in8, out8};
use crate::fifo::FIFO_BUF;

const PIC0_ICW1: u32 = 0x0020;
pub const PIC0_OCW2: u32 = 0x0020;
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

const PORT_KEYDAT: u32 = 0x60;
const PORT_KEYSTA: u32 = 0x0064;
const PORT_KEYCMD: u32 = 0x0064;
const KEYSTA_SEND_NOTREADY: u8 = 0x02;
const KEYCMD_WRITE_MODE: u8 = 0x60;
const KBC_MODE: u8 = 0x47;
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

fn wait_kbc_sendready() {
    // キーボードコントローラがデータ送信可能になるのを待つ
    loop {
        if (in8(PORT_KEYSTA) & KEYSTA_SEND_NOTREADY) == 0 {
            break;
        }
    }
    return;
}

fn init_keyboard() {
    wait_kbc_sendready();
    out8(PORT_KEYCMD, KEYCMD_WRITE_MODE);
    wait_kbc_sendready();
    out8(PORT_KEYDAT, KBC_MODE);
}

pub fn enable_mouse() {
    wait_kbc_sendready();
    out8(PORT_KEYCMD, KEYCMD_SENDTO_MOUSE);
    wait_kbc_sendready();
    out8(PORT_KEYDAT, MOUSECMD_ENABLE);
}

const KEYBOARD_OFFSET: u32 = 256;
const MOUSE_OFFSET: u32 = 512;

pub extern "C" fn inthandler21() {
    out8(PIC0_OCW2, 0x61); // IRQ-01 受付終了
    let key = in8(PORT_KEYDAT);
    FIFO_BUF.lock().put(key as u32 + KEYBOARD_OFFSET).unwrap();
}

pub extern "C" fn inthandler2c() {
    out8(PIC1_OCW2, 0x64); // IRQ-12受付完了をPIC1に通知
    out8(PIC0_OCW2, 0x62); // IRQ-02受付完了をPIC0に通知
    let data = in8(PORT_KEYDAT);
    FIFO_BUF.lock().put(data as u32 + MOUSE_OFFSET).unwrap();
}
