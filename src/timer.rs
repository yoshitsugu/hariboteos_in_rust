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

pub extern "C" fn inthandler20() {
    out8(PIC0_OCW2, 0x60); // IRQ-00受付完了をPICに通知
    let mut tm = TIMER_MANAGER.lock();
    tm.count += 1;
    if tm.next > tm.count {
        return;
    }
    let mut timeout_count = 0;
    for i in 0..tm.counting {
        timeout_count = i;
        let timer_index = tm.timers[i as usize];
        let t = tm.timers_data[timer_index];
        if t.timeout > tm.count {
            break;
        }
        {
            let mut t_mut = &mut tm.timers_data[timer_index];
            t_mut.flag = TimerFlag::USED;
        }
        let fifo = unsafe { &*(t.fifo_addr as *const Fifo) };
        fifo.put(t.data).unwrap();
    }
    tm.counting -= timeout_count;
    for i in 0..tm.counting {
        tm.timers[i as usize] = tm.timers[(timeout_count + i) as usize];
    }
    if tm.counting > 0 {
        tm.next = tm.timers_data[tm.timers[0]].timeout;
    } else {
        tm.next = 0xffffffff;
    }
}

const MAX_TIMER: usize = 500;

#[derive(Debug, Clone, Copy)]
pub struct Timer {
    pub timeout: u32,
    pub flag: TimerFlag,
    pub fifo_addr: u32,
    pub data: u8,
}

impl Timer {
    pub fn new() -> Timer {
        Timer {
            timeout: 0,
            flag: TimerFlag::AVAILABLE,
            fifo_addr: 0,
            data: 0,
        }
    }
}

pub struct TimerManager {
    pub count: u32,
    pub next: u32,
    pub counting: u32,
    pub timers: [usize; MAX_TIMER],
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
        TimerManager {
            count: 0,
            next: 0,
            counting: 0,
            timers: [0; MAX_TIMER],
            timers_data: [Timer::new(); MAX_TIMER],
        }
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
        let mut timer = &mut self.timers_data[timer_index];
        timer.timeout = timeout + self.count;
        timer.flag = TimerFlag::COUNTING;
        let eflags = load_eflags();
        cli();
        let mut insert_index: usize = 0;
        for i in 0..self.counting {
            insert_index = i as usize;
            let t = self.timers_data[self.timers[i as usize]];
            if t.timeout >= t.timeout {
                break;
            }
        }
        let mut j = self.counting as usize;
        while j > insert_index {
            self.timers[j] = self.timers[j - 1];
            j -= 1;
        }
        self.counting += 1;
        self.timers[insert_index] = timer_index;
        self.next = self.timers_data[self.timers[0]].timeout;
        store_eflags(eflags);
    }

    pub fn init_timer(&mut self, timer_index: usize, fifo: &Fifo, data: u8) {
        let mut timer = &mut self.timers_data[timer_index];
        timer.fifo_addr = fifo as *const Fifo as u32;
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
