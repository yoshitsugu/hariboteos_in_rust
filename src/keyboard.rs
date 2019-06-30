use crate::asm::{in8, out8};
use crate::fifo::FIFO_BUF;
use crate::interrupt::{PIC0_OCW2, PORT_KEYCMD, PORT_KEYDAT};

pub const KEYBOARD_OFFSET: u32 = 256;
const PORT_KEYSTA: u32 = 0x0064;
const KEYCMD_WRITE_MODE: u8 = 0x60;
const KEYSTA_SEND_NOTREADY: u8 = 0x02;
const KBC_MODE: u8 = 0x47;

pub static KEYTABLE: [u8; 84] = [
    0, 0, b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'0', b'-', b'^', 0, 0, b'Q', b'W',
    b'E', b'R', b'T', b'Y', b'U', b'I', b'O', b'P', b'@', b'[', 0, 0, b'A', b'S', b'D', b'F', b'G',
    b'H', b'J', b'K', b'L', b';', b':', 0, 0, b']', b'Z', b'X', b'C', b'V', b'B', b'N', b'M', b',',
    b'.', b'/', 0, b'*', 0, b' ', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, b'7', b'8', b'9', b'-',
    b'4', b'5', b'6', b'+', b'1', b'2', b'3', b'0', b'.',
];

pub fn wait_kbc_sendready() {
    // キーボードコントローラがデータ送信可能になるのを待つ
    loop {
        if (in8(PORT_KEYSTA) & KEYSTA_SEND_NOTREADY) == 0 {
            break;
        }
    }
    return;
}

pub fn init_keyboard() {
    wait_kbc_sendready();
    out8(PORT_KEYCMD, KEYCMD_WRITE_MODE);
    wait_kbc_sendready();
    out8(PORT_KEYDAT, KBC_MODE);
}

pub extern "C" fn inthandler21() {
    out8(PIC0_OCW2, 0x61); // IRQ-01 受付終了
    let key = in8(PORT_KEYDAT);
    FIFO_BUF.lock().put(key as u32 + KEYBOARD_OFFSET).unwrap();
}
