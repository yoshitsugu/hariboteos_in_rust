use crate::asm::{cli, sti};
use crate::fifo::Fifo;
use crate::keyboard::KEYBOARD_OFFSET;
use crate::memory::{MemMan, MEMMAN_ADDR};
use crate::mt::{TaskManager, TASK_MANAGER_ADDR};
use crate::sheet::SheetManager;
use crate::timer::TIMER_MANAGER;
use crate::vga::{boxfill, Color};
use crate::{write_with_bg, SHEET_MANAGER_ADDR};

pub const CONSOLE_CURSOR_ON: u32 = 2;
pub const CONSOLE_CURSOR_OFF: u32 = 3;
pub const CONSOLE_BACKSPACE: u32 = 8;
pub const CONSOLE_ENTER: u32 = 10;
const MIN_CURSOR_X: isize = 16;
const MIN_CURSOR_Y: isize = 28;
const MAX_CURSOR_X: isize = 8 + 240;
const MAX_CURSOR_Y: isize = 28 + 112;

pub extern "C" fn console_task(sheet_index: usize, memtotal: usize) {
    let task_manager = unsafe { &mut *(TASK_MANAGER_ADDR as *mut TaskManager) };
    let task_index = task_manager.now_index();

    let fifo = Fifo::new(128, Some(task_index));
    let fifo_addr = &fifo as *const Fifo as usize;
    {
        let mut task = &mut task_manager.tasks_data[task_index];
        task.fifo_addr = fifo_addr;
    }

    let mut cursor_x: isize = MIN_CURSOR_X;
    let mut cursor_y: isize = MIN_CURSOR_Y;
    let mut cursor_c = Color::Black;
    let mut cursor_on = false;

    // コマンドを保持するための配列
    let mut cmdline: [u8; 30] = [0; 30];

    let sheet_manager_addr = unsafe { SHEET_MANAGER_ADDR };
    let sheet_manager = unsafe { &mut *(sheet_manager_addr as *mut SheetManager) };

    let timer_index = TIMER_MANAGER.lock().alloc().unwrap();
    TIMER_MANAGER.lock().init_timer(timer_index, fifo_addr, 1);
    TIMER_MANAGER.lock().set_time(timer_index, 50);
    let sheet = sheet_manager.sheets_data[sheet_index];

    // プロンプト表示
    write_with_bg!(
        sheet_manager,
        sheet_index,
        sheet.width,
        sheet.height,
        8,
        cursor_y,
        Color::White,
        Color::Black,
        1,
        ">"
    );

    loop {
        cli();
        if fifo.status() == 0 {
            task_manager.sleep(task_index);
            sti();
        } else {
            let i = fifo.get().unwrap();
            sti();
            if i <= 1 {
                if i != 0 {
                    TIMER_MANAGER.lock().init_timer(timer_index, fifo_addr, 0);
                    cursor_c = if cursor_on {
                        Color::White
                    } else {
                        Color::Black
                    };
                } else {
                    TIMER_MANAGER.lock().init_timer(timer_index, fifo_addr, 1);
                    cursor_c = Color::Black;
                }
                TIMER_MANAGER.lock().set_time(timer_index, 50);
            } else if KEYBOARD_OFFSET <= i && i <= 511 {
                let key = (i - KEYBOARD_OFFSET) as u8;
                if key != 0 {
                    // バックスペース
                    if key == CONSOLE_BACKSPACE as u8 {
                        if cursor_x > MIN_CURSOR_X {
                            write_with_bg!(
                                sheet_manager,
                                sheet_index,
                                sheet.width,
                                sheet.height,
                                cursor_x,
                                cursor_y,
                                Color::White,
                                Color::Black,
                                1,
                                " "
                            );
                            cursor_x -= 8;
                        }
                    } else if key == CONSOLE_ENTER as u8 {
                        write_with_bg!(
                            sheet_manager,
                            sheet_index,
                            sheet.width,
                            sheet.height,
                            cursor_x,
                            cursor_y,
                            Color::White,
                            Color::Black,
                            1,
                            " "
                        );
                        cursor_y = newline(cursor_y, sheet_manager, sheet_index);
                        cursor_y =
                            exec_cmd(cmdline, cursor_y, sheet_manager, sheet_index, memtotal);
                        cmdline = [0; 30];
                        // プロンプト表示
                        write_with_bg!(
                            sheet_manager,
                            sheet_index,
                            sheet.width,
                            sheet.height,
                            8,
                            cursor_y,
                            Color::White,
                            Color::Black,
                            1,
                            ">"
                        );
                        cursor_x = 16;
                    } else {
                        if cursor_x < MAX_CURSOR_X {
                            cmdline[cursor_x as usize / 8 - 2] = key;
                            write_with_bg!(
                                sheet_manager,
                                sheet_index,
                                sheet.width,
                                sheet.height,
                                cursor_x,
                                cursor_y,
                                Color::White,
                                Color::Black,
                                1,
                                "{}",
                                key as char,
                            );
                            cursor_x += 8;
                        }
                    }
                }
            } else if i == CONSOLE_CURSOR_ON {
                cursor_on = true;
            } else if i == CONSOLE_CURSOR_OFF {
                cursor_on = false;
            }
            if cursor_on {
                boxfill(
                    sheet.buf_addr,
                    sheet.width as isize,
                    cursor_c,
                    cursor_x,
                    cursor_y,
                    cursor_x + 7,
                    cursor_y + 15,
                );
                sheet_manager.refresh(
                    sheet_index,
                    cursor_x as i32,
                    cursor_y as i32,
                    cursor_x as i32 + 8,
                    cursor_y as i32 + 16,
                );
            }
        }
    }
}

fn newline(cursor_y: isize, sheet_manager: &mut SheetManager, sheet_index: usize) -> isize {
    let mut cursor_y = cursor_y;
    let sheet = sheet_manager.sheets_data[sheet_index];

    if cursor_y < MAX_CURSOR_Y {
        cursor_y += 16;
    } else {
        for y in MIN_CURSOR_Y..MAX_CURSOR_Y {
            for x in (MIN_CURSOR_X - 8)..MAX_CURSOR_X {
                let x = x as usize;
                let y = y as usize;
                // 下の画素をコピーする
                let ptr =
                    unsafe { &mut *((sheet.buf_addr + x + y * sheet.width as usize) as *mut u8) };
                *ptr = unsafe {
                    *((sheet.buf_addr + x + (y + 16) * sheet.width as usize) as *const u8)
                }
            }
        }
        for y in MAX_CURSOR_Y..(MAX_CURSOR_Y + 16) {
            for x in (MIN_CURSOR_X - 8)..MAX_CURSOR_X {
                let x = x as usize;
                let y = y as usize;
                // 最後の行は黒で埋める
                let ptr =
                    unsafe { &mut *((sheet.buf_addr + x + y * sheet.width as usize) as *mut u8) };
                *ptr = Color::Black as u8;
            }
        }

        sheet_manager.refresh(
            sheet_index,
            (MIN_CURSOR_X - 8) as i32,
            MIN_CURSOR_Y as i32,
            MAX_CURSOR_X as i32,
            (MAX_CURSOR_Y + 16) as i32,
        );
    }
    cursor_y
}

fn exec_cmd(
    cmdline: [u8; 30],
    cursor_y: isize,
    sheet_manager: &mut SheetManager,
    sheet_index: usize,
    memtotal: usize,
) -> isize {
    let sheet = sheet_manager.sheets_data[sheet_index];
    let mut cursor_y = cursor_y;
    let cmd_ind = extract_cmd_index(cmdline);
    let cmd = core::str::from_utf8(&cmdline[cmd_ind.0..cmd_ind.1]).unwrap();
    if cmd == "mem" {
        let memman = unsafe { &mut *(MEMMAN_ADDR as *mut MemMan) };

        write_with_bg!(
            sheet_manager,
            sheet_index,
            sheet.width,
            sheet.height,
            8,
            cursor_y,
            Color::White,
            Color::Black,
            30,
            "total   {}MB",
            memtotal / (1024 * 1024)
        );
        cursor_y = newline(cursor_y, sheet_manager, sheet_index);
        write_with_bg!(
            sheet_manager,
            sheet_index,
            sheet.width,
            sheet.height,
            8,
            cursor_y,
            Color::White,
            Color::Black,
            30,
            "free {}KB",
            memman.total() / 1024
        );
        cursor_y = newline(cursor_y, sheet_manager, sheet_index);
        cursor_y = newline(cursor_y, sheet_manager, sheet_index);
    } else if cmd == "clear" {
        for y in MIN_CURSOR_Y..(MAX_CURSOR_Y + 16) {
            for x in (MIN_CURSOR_X - 8)..MAX_CURSOR_X {
                let x = x as usize;
                let y = y as usize;
                let ptr =
                    unsafe { &mut *((sheet.buf_addr + x + y * sheet.width as usize) as *mut u8) };
                *ptr = Color::Black as u8;
            }
        }
        sheet_manager.refresh(
            sheet_index,
            (MIN_CURSOR_X - 8) as i32,
            MIN_CURSOR_Y as i32,
            MAX_CURSOR_X as i32,
            (MAX_CURSOR_Y + 16) as i32,
        );
        cursor_y = MIN_CURSOR_Y;
    } else {
        write_with_bg!(
            sheet_manager,
            sheet_index,
            sheet.width,
            sheet.height,
            8,
            cursor_y,
            Color::White,
            Color::Black,
            12,
            "Bad Command"
        );
        cursor_y = newline(cursor_y, sheet_manager, sheet_index);
        cursor_y = newline(cursor_y, sheet_manager, sheet_index);
    }
    cursor_y
}

fn extract_cmd_index(cmdline: [u8; 30]) -> (usize, usize) {
    // 空白(32)、0はとばす
    let mut start: isize = -1;
    let mut end: isize = -1;
    for i in 0..cmdline.len() {
        if start < 0 {
            if cmdline[i] != 0 && cmdline[i] != 32 {
                start = i as isize;
                end = i as isize;
            }
        } else {
            end = i as isize;
            if cmdline[i] == 0 || cmdline[i] == 32 {
                break;
            }
        }
    }
    (
        (if start < 0 { 0 } else { start }) as usize,
        (if end < 0 { 0 } else { end }) as usize,
    )
}
