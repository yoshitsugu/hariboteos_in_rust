use crate::console::{CONSOLE_CURSOR_OFF, CONSOLE_CURSOR_ON};
use crate::fifo::Fifo;
use crate::mt::TaskManager;
use crate::sheet::SheetManager;
use crate::vga::{toggle_title_color};

pub fn window_on(sheet_manager: &mut SheetManager, task_manager: &TaskManager, sheet_index: usize) {
    let sheet = sheet_manager.sheets_data[sheet_index];
    toggle_title_color(sheet.buf_addr, sheet.width as usize, true);
    sheet_manager.refresh(sheet_index, 3, 3, sheet.width, 21);
    if sheet.cursor {
        let task = task_manager.tasks_data[sheet.task_index];
        let fifo = unsafe { &mut *(task.fifo_addr as *mut Fifo) };
        fifo.put(CONSOLE_CURSOR_ON).unwrap();
    }
}

pub fn window_off(
    sheet_manager: &mut SheetManager,
    task_manager: &TaskManager,
    sheet_index: usize,
) {
    let sheet = sheet_manager.sheets_data[sheet_index];
    toggle_title_color(sheet.buf_addr, sheet.width as usize, false);
    sheet_manager.refresh(sheet_index, 3, 3, sheet.width, 21);
    if sheet.cursor {
        let task = task_manager.tasks_data[sheet.task_index];
        let fifo = unsafe { &mut *(task.fifo_addr as *mut Fifo) };
        fifo.put(CONSOLE_CURSOR_OFF).unwrap();
    }
}
