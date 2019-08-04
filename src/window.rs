use crate::console::{CONSOLE_CURSOR_OFF, CONSOLE_CURSOR_ON};
use crate::fifo::Fifo;
use crate::mt::TaskManager;
use crate::sheet::SheetManager;
use crate::vga::{boxfill, toggle_title_color, Color};

pub fn window_on(
    sheet_manager: &mut SheetManager,
    task_manager: &TaskManager,
    sheet_index: usize,
    shi_win: usize,
    cursor_c: Color,
) -> Color {
    let sheet = sheet_manager.sheets_data[sheet_index];
    let mut cursor_c = cursor_c;
    toggle_title_color(sheet.buf_addr, sheet.width as usize, true);
    sheet_manager.refresh(sheet_index, 3, 3, sheet.width, 21);
    if sheet_index == shi_win {
        cursor_c = Color::Black;
        {
            let mut sheet_win = &mut sheet_manager.sheets_data[sheet_index];
            sheet_win.cursor = true;
        }
    } else {
        if sheet.cursor {
            let task = task_manager.tasks_data[sheet.task_index];
            let fifo = unsafe { &mut *(task.fifo_addr as *mut Fifo) };
            fifo.put(CONSOLE_CURSOR_ON).unwrap();
        }
    }
    cursor_c
}

pub fn window_off(
    sheet_manager: &mut SheetManager,
    task_manager: &TaskManager,
    sheet_index: usize,
    shi_win: usize,
    cursor_c: Color,
    cursor_x: i32,
) -> Color {
    let sheet_win = sheet_manager.sheets_data[sheet_index];
    let sheet = sheet_manager.sheets_data[sheet_index];
    let mut cursor_c = cursor_c;
    toggle_title_color(sheet.buf_addr, sheet.width as usize, false);
    sheet_manager.refresh(sheet_index, 3, 3, sheet.width, 21);
    if sheet_index == shi_win {
        cursor_c = Color::White;
        {
            let mut sheet_win = &mut sheet_manager.sheets_data[sheet_index];
            sheet_win.cursor = false;
        }
        boxfill(
            sheet_win.buf_addr,
            sheet_win.width as isize,
            Color::White,
            cursor_x as isize,
            28,
            cursor_x as isize + 7,
            43,
        );
    } else {
        if sheet.cursor {
            let task = task_manager.tasks_data[sheet.task_index];
            let fifo = unsafe { &mut *(task.fifo_addr as *mut Fifo) };
            fifo.put(CONSOLE_CURSOR_OFF).unwrap();
        }
    }
    cursor_c
}
