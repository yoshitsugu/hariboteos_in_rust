use crate::asm::out8;
use crate::interrupt::PIC0_OCW2;

pub static mut COUNTER: u32 = 0;

const PIT_CTRL: u32 = 0x0043;
const PIT_CNT0: u32 = 0x0040;

pub fn init_pit() {
    out8(PIT_CTRL, 0x34);
    out8(PIT_CNT0, 0x9c);
    out8(PIT_CNT0, 0x2e);
    unsafe {
        COUNTER = 0;
    }
}

pub extern "C" fn inthandler20() {
    out8(PIC0_OCW2, 0x60); // IRQ-00受付完了をPICに通知
    unsafe {
        COUNTER += 1;
    }
}
