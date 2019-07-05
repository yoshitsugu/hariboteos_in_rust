use core::default::Default;

use crate::asm::load_tr;
use crate::descriptor_table::{SegmentDescriptor, ADR_GDT, AR_TSS32};
use crate::timer::TIMER_MANAGER;

const MAX_TASKS: usize = 1000;
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

#[derive(Debug, Clone, Copy)]
pub struct Task {
    pub select: i32,
    pub flag: TaskFlag,
    pub tss: TSS,
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
            tss: Default::default(),
        }
    }
}

pub struct TaskManager {
    pub running_count: i32,
    pub now_running: i32,
    pub tasks: [usize; MAX_TASKS],
    pub tasks_data: [Task; MAX_TASKS],
}

pub static mut TASK_MANAGER_ADDR: usize = 0;
pub static mut MT_TIMER_INDEX: usize = 1001;

impl TaskManager {
    pub fn new() -> TaskManager {
        TaskManager {
            running_count: 0,
            now_running: 0,
            tasks: [0; MAX_TASKS],
            tasks_data: [Task::new(); MAX_TASKS],
        }
    }

    pub fn init(&mut self) {
        for i in 0..MAX_TASKS {
            let mut task = &mut self.tasks_data[i];
            task.select = (TASK_GDT0 + i as i32) * 8;
            let gdt =
                unsafe { &mut *((ADR_GDT + (TASK_GDT0 + i as i32) * 8) as *mut SegmentDescriptor) };
            *gdt = SegmentDescriptor::new(103, &(task.tss) as *const TSS as i32, AR_TSS32);
        }
        let task_index = self.alloc().unwrap();

        let mut task = &mut self.tasks_data[task_index];
        task.flag = TaskFlag::RUNNING;
        self.running_count = 1;
        self.now_running = 0;
        self.tasks[0] = task_index;
        load_tr(task.select);
        let timer_index_ts = TIMER_MANAGER.lock().alloc().unwrap();
        TIMER_MANAGER.lock().set_time(timer_index_ts, 2);
        unsafe {
            MT_TIMER_INDEX = timer_index_ts;
        }
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

    pub fn run(&mut self, task_index: usize) {
        let mut task = &mut self.tasks_data[task_index];
        task.flag = TaskFlag::RUNNING;
        self.tasks[self.running_count as usize] = task_index;
        self.running_count += 1;
    }

    pub fn switch(&mut self) {
        TIMER_MANAGER.lock().set_time(unsafe { MT_TIMER_INDEX }, 2);
        if self.running_count >= 2 {
            self.now_running += 1;
            if self.now_running == self.running_count {
                self.now_running = 0;
            }

            crate::asm::farjmp(
                0,
                self.tasks_data[self.tasks[self.now_running as usize]].select,
            );
        }
    }
}
