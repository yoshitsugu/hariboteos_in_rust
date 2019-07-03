use lazy_static::lazy_static;
use spin::Mutex;

use crate::asm::{cli, load_eflags, out8, store_eflags};
use crate::fifo::Fifo;
use crate::interrupt::PIC0_OCW2;

const PIT_CTRL: u32 = 0x0043;
const PIT_CNT0: u32 = 0x0040;

pub fn init_pit() {
    out8(PIT_CTRL, 0x34);
    out8(PIT_CNT0, 0x9c);
    out8(PIT_CNT0, 0x2e);
}

const MAX_TIMER: usize = 500;

#[derive(Debug, Clone, Copy)]
pub struct Timer {
    pub timeout: u32,
    pub flag: TimerFlag,
    pub data: u8,
    pub fifo_addr: usize,
    pub next: Option<usize>,
}

impl Timer {
    pub fn new() -> Timer {
        Timer {
            timeout: 0,
            flag: TimerFlag::AVAILABLE,
            data: 0,
            fifo_addr: 0,
            next: None,
        }
    }
}

pub struct TimerManager {
    pub count: u32,
    pub next_tick: u32,
    pub t0: Option<usize>,
    pub timers_data: [Timer; MAX_TIMER],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerFlag {
    AVAILABLE,
    USED,
    COUNTING,
}

impl TimerManager {
    pub fn new() -> TimerManager {
        let mut tm = TimerManager {
            count: 0,
            next_tick: 0xffffffff,
            t0: Some(MAX_TIMER - 1),
            timers_data: [Timer::new(); MAX_TIMER],
        };
        // 番兵
        tm.timers_data[MAX_TIMER - 1] = Timer {
            timeout: 0xffffffff,
            flag: TimerFlag::COUNTING,
            data: 0,
            fifo_addr: 0,
            next: None,
        };
        tm
    }

    pub fn alloc(&mut self) -> Result<usize, &'static str> {
        for i in 0..MAX_TIMER {
            if self.timers_data[i].flag == TimerFlag::AVAILABLE {
                self.timers_data[i].flag = TimerFlag::USED;
                return Ok(i);
            }
        }
        Err("CANNOT ASSIGN TIMER")
    }

    pub fn set_time(&mut self, timer_index: usize, timeout: u32) {
        {
            let mut timer = &mut self.timers_data[timer_index];
            timer.timeout = timeout + self.count;
            timer.flag = TimerFlag::COUNTING;
        }
        if self.t0.is_none() {
            return;
        }
        let eflags = load_eflags();
        cli();
        let mut t_index = self.t0.unwrap();
        if &self.timers_data[timer_index].timeout <= &self.timers_data[t_index].timeout {
            // 先頭に入れる
            let mut timer = &mut self.timers_data[timer_index];
            self.t0 = Some(timer_index);
            timer.next = Some(t_index);
            self.next_tick = timer.timeout;
            store_eflags(eflags);
            return;
        }
        let mut old_t_index: usize;
        // 挿入できるインデックスをさがす
        loop {
            old_t_index = t_index;
            if self.timers_data[t_index].next.is_none() {
                store_eflags(eflags);
                break;
            }
            t_index = self.timers_data[t_index].next.unwrap();
            if self.timers_data[timer_index].timeout <= self.timers_data[t_index].timeout {
                {
                    let mut s = &mut self.timers_data[old_t_index];
                    s.next = Some(timer_index);
                }
                {
                    let mut timer = &mut self.timers_data[timer_index];
                    timer.next = Some(t_index);
                }
                store_eflags(eflags);
                return;
            }
        }
    }

    pub fn init_timer(&mut self, timer_index: usize, fifo_addr: usize, data: u8) {
        let mut timer = &mut self.timers_data[timer_index];
        timer.fifo_addr = fifo_addr;
        timer.data = data;
    }

    pub fn free(&mut self, i: usize) {
        let mut timer = &mut self.timers_data[i];
        timer.flag = TimerFlag::AVAILABLE;
    }
}

lazy_static! {
    pub static ref TIMER_MANAGER: Mutex<TimerManager> = Mutex::new(TimerManager::new());
}

pub extern "C" fn inthandler20() {
    out8(PIC0_OCW2, 0x60); // IRQ-00受付完了をPICに通知
    let mut tm = TIMER_MANAGER.lock();
    tm.count += 1;
    if tm.next_tick > tm.count {
        return;
    }
    let mut timer_index = tm.t0;
    loop {
        if timer_index.is_none() {
            return;
        }
        let t_index = timer_index.unwrap();
        if tm.timers_data[t_index].timeout > tm.count {
            break;
        }
        let mut timer = &mut tm.timers_data[t_index];
        timer.flag = TimerFlag::USED;
        let fifo = unsafe { &mut *(timer.fifo_addr as *mut Fifo) };
        fifo.put(timer.data as u32).unwrap();
        timer_index = timer.next;
    }
    tm.t0 = timer_index;
    if let Some(t_index) = timer_index {
        tm.next_tick = tm.timers_data[t_index].timeout;
    }
}
