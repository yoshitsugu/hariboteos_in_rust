use core::default::Default;

use crate::asm::{farjmp, load_tr};
use crate::descriptor_table::{SegmentDescriptor, ADR_GDT, AR_LDT, AR_TSS32};
use crate::memory::{MemMan, MEMMAN_ADDR};
use crate::timer::TIMER_MANAGER;

const MAX_TASKS: usize = 1000;
const MAX_TASKS_LV: usize = 100;
const MAX_TASKLEVELS: usize = 10;
const TASK_GDT0: i32 = 3;

#[derive(Debug, Default, Clone, Copy)]
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

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Task {
    pub select: i32,
    pub flag: TaskFlag,
    pub level: usize,
    pub priority: i32,
    pub tss: TSS,
    pub fifo_addr: usize,
    pub console_addr: usize,
    pub ds_base: usize,
    pub console_stack: usize,
    pub ldt: [SegmentDescriptor; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskFlag {
    AVAILABLE,
    USED,
    RUNNING,
}

impl Task {
    fn new() -> Task {
        Task {
            select: 0,
            flag: TaskFlag::AVAILABLE,
            level: 0,
            priority: 2,
            tss: Default::default(),
            fifo_addr: 0,
            console_addr: 0,
            ds_base: 0,
            console_stack: 0,
            ldt: [
                SegmentDescriptor::new(0, 0, 0),
                SegmentDescriptor::new(0, 0, 0),
            ],
        }
    }
}

#[derive(Clone, Copy)]
pub struct TaskLevel {
    pub running_count: usize,
    pub now_running: usize,
    pub tasks: [usize; MAX_TASKS_LV],
}

impl TaskLevel {
    pub fn new() -> TaskLevel {
        TaskLevel {
            running_count: 0,
            now_running: 0,
            tasks: [0; MAX_TASKS_LV],
        }
    }
}

pub struct TaskManager {
    pub now_lv: usize,
    pub lv_change: bool,
    pub level: [TaskLevel; MAX_TASKLEVELS],
    pub tasks_data: [Task; MAX_TASKS],
}

pub static mut TASK_MANAGER_ADDR: usize = 0;
pub static mut MT_TIMER_INDEX: usize = 1001;

impl TaskManager {
    pub fn new() -> TaskManager {
        TaskManager {
            now_lv: 0,
            lv_change: false,
            level: [TaskLevel::new(); MAX_TASKLEVELS],
            tasks_data: [Task::new(); MAX_TASKS],
        }
    }

    pub fn now_index(&self) -> usize {
        let tl = self.level[self.now_lv];
        tl.tasks[tl.now_running]
    }

    pub fn add_task(&mut self, task_index: usize) {
        {
            let mut lv = &mut self.level[self.tasks_data[task_index].level];
            lv.tasks[lv.running_count] = task_index;
            lv.running_count += 1;
        }
        {
            let mut task = &mut self.tasks_data[task_index];
            task.flag = TaskFlag::RUNNING;
        }
    }

    pub fn remove_task(&mut self, task_index: usize) {
        let mut lv = &mut self.level[self.tasks_data[task_index].level];
        let mut task_order = 0;
        for i in 0..lv.running_count {
            task_order = i;
            if lv.tasks[i] == task_index {
                break;
            }
        }
        lv.running_count -= 1;
        if task_order < lv.now_running {
            lv.now_running -= 1;
        }
        if lv.now_running >= lv.running_count {
            lv.now_running = 0;
        }
        let mut task = &mut self.tasks_data[task_index];
        task.flag = TaskFlag::USED;
        for i in task_order..lv.running_count {
            lv.tasks[i] = lv.tasks[i + 1];
        }
    }

    pub fn close_task(&mut self, task_index: usize) {
        self.sleep(task_index);
        let mut task = &mut self.tasks_data[task_index];
        let memman = unsafe { &mut *(MEMMAN_ADDR as *mut MemMan) };
        memman
            .free_4k(task.console_stack as u32, 64 * 1024)
            .unwrap();
        memman.free_4k(task.fifo_addr as u32, 128 * 4).unwrap();
        task.flag = TaskFlag::AVAILABLE;
    }

    pub fn switchsub(&mut self) {
        let mut now_lv = 0;
        for i in 0..MAX_TASKLEVELS {
            now_lv = i;
            if self.level[i].running_count > 0 {
                break;
            }
        }
        self.now_lv = now_lv;
        self.lv_change = false;
    }

    pub fn init(&mut self, memman: &mut MemMan, fifo_addr: usize) -> Result<usize, &'static str> {
        for i in 0..MAX_TASKS {
            let mut task = &mut self.tasks_data[i];
            task.select = (TASK_GDT0 + i as i32) * 8;
            task.tss.ldtr = (TASK_GDT0 + MAX_TASKS as i32 + i as i32) * 8;
            let gdt =
                unsafe { &mut *((ADR_GDT + (TASK_GDT0 + i as i32) * 8) as *mut SegmentDescriptor) };
            *gdt = SegmentDescriptor::new(103, &(task.tss) as *const TSS as i32, AR_TSS32);
            let ldt = unsafe {
                &mut *((ADR_GDT + (TASK_GDT0 + MAX_TASKS as i32 + i as i32) * 8)
                    as *mut SegmentDescriptor)
            };
            *ldt = SegmentDescriptor::new(15, task.ldt.as_ptr() as i32, AR_LDT);
        }
        let task_index = self.alloc()?;
        {
            let mut task = &mut self.tasks_data[task_index];
            task.flag = TaskFlag::RUNNING;
            task.priority = 2;
            task.level = 0;
            task.fifo_addr = fifo_addr;
        }
        self.add_task(task_index);
        self.switchsub();
        let task = self.tasks_data[task_index];
        load_tr(task.select);
        let timer_index_ts = TIMER_MANAGER.lock().alloc()?;
        TIMER_MANAGER
            .lock()
            .set_time(timer_index_ts, task.priority as u32);
        unsafe {
            MT_TIMER_INDEX = timer_index_ts;
        }
        {
            let idle_index = self.alloc()?;
            let mut idle = &mut self.tasks_data[idle_index];
            idle.tss.esp = memman.alloc_4k(64 * 1024)? as i32 + 64 * 1024;
            idle.tss.eip = task_idle as i32;
            idle.tss.es = 1 * 8;
            idle.tss.cs = 2 * 8;
            idle.tss.ss = 1 * 8;
            idle.tss.ds = 1 * 8;
            idle.tss.fs = 1 * 8;
            idle.tss.gs = 1 * 8;
            self.run(idle_index, MAX_TASKLEVELS as i32 - 1, 1);
        }

        Ok(task_index)
    }

    pub fn alloc(&mut self) -> Result<usize, &'static str> {
        for i in 0..MAX_TASKS {
            if self.tasks_data[i].flag == TaskFlag::AVAILABLE {
                let mut task = &mut self.tasks_data[i];
                task.flag = TaskFlag::USED;
                task.tss.eflags = 0x00000202; /* IF = 1; */
                task.tss.iomap = 0x40000000;
                return Ok(i);
            }
        }
        return Err("CANNOT ALLOCATE TASK");
    }

    pub fn run(&mut self, task_index: usize, level_i32: i32, priority: i32) {
        let task = self.tasks_data[task_index];
        let level: usize;
        if level_i32 < 0 {
            level = task.level;
        } else {
            level = level_i32 as usize;
        }
        if priority > 0 {
            let mut task = &mut self.tasks_data[task_index];
            task.priority = priority;
        }
        if task.flag == TaskFlag::RUNNING && task.level != level {
            self.remove_task(task_index);
        }
        // フラグがかわる可能性があるのでtaskをとりなおし
        if self.tasks_data[task_index].flag != TaskFlag::RUNNING {
            let mut task = &mut self.tasks_data[task_index];
            task.level = level;
            self.add_task(task_index);
        }
        self.lv_change = true;
    }

    pub fn switch(&mut self) {
        let mut lv = &mut self.level[self.now_lv];
        let now_task_index = lv.tasks[lv.now_running];
        lv.now_running += 1;
        if lv.now_running == lv.running_count {
            lv.now_running = 0;
        }
        if self.lv_change {
            self.switchsub();
            lv = &mut self.level[self.now_lv];
        }
        let new_task_index = lv.tasks[lv.now_running];
        let new_task = self.tasks_data[new_task_index];
        TIMER_MANAGER
            .lock()
            .set_time(unsafe { MT_TIMER_INDEX }, new_task.priority as u32);
        if new_task_index != now_task_index {
            farjmp(0, new_task.select);
        }
    }

    pub fn sleep(&mut self, task_index: usize) {
        let task = self.tasks_data[task_index];

        if task.flag == TaskFlag::RUNNING {
            let now_index = self.now_index();
            self.remove_task(task_index);
            if task_index == now_index {
                // スリープする対象と今動いているタスクが同じなのでタスクスイッチが必要
                self.switchsub();
                let now_task = self.tasks_data[self.now_index()];
                farjmp(0, now_task.select);
            }
        }
    }
}

pub extern "C" fn task_idle() {
    loop {
        crate::asm::hlt();
    }
}
