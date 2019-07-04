#[derive(Default)]
#[repr(C, packed)]
pub struct TSS {
    pub backlink: i32,
    pub esp0: i32,
    pub ss0: i32,
    pub esp1: i32,
    pub ss1: i32,
    pub esp2: i32,
    pub ss2: i32,
    pub cr3: i32,
    pub eip: i32,
    pub eflags: i32,
    pub eax: i32,
    pub ecx: i32,
    pub edx: i32,
    pub ebx: i32,
    pub esp: i32,
    pub ebp: i32,
    pub esi: i32,
    pub edi: i32,
    pub es: i32,
    pub cs: i32,
    pub ss: i32,
    pub ds: i32,
    pub fs: i32,
    pub gs: i32,
    pub ldtr: i32,
    pub iomap: i32,
}

use crate::timer::TIMER_MANAGER;

pub static mut MT_TIMER_INDEX: usize = 0;
pub static mut MT_TR: i32 = 3 * 8;

pub fn mt_init() {
    let timer_index_ts = TIMER_MANAGER.lock().alloc().unwrap();
    TIMER_MANAGER.lock().set_time(timer_index_ts, 2);
    unsafe {
        MT_TIMER_INDEX = timer_index_ts;
    }
}

pub fn mt_taskswitch() {
    if unsafe { MT_TR } == 3 * 8 {
        unsafe {
            MT_TR = 4 * 8;
        }
        TIMER_MANAGER.lock().set_time(unsafe { MT_TIMER_INDEX }, 2);
        crate::asm::farjmp(0, 4 * 8);
    } else {
        unsafe {
            MT_TR = 3 * 8;
        }
        TIMER_MANAGER.lock().set_time(unsafe { MT_TIMER_INDEX }, 2);
        crate::asm::farjmp(0, 3 * 8);
    }
}
