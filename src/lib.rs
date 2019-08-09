#![no_std]
#![feature(asm)]
#![feature(start)]
#![feature(naked_functions)]

mod asm;
mod console;
mod descriptor_table;
mod fifo;
mod file;
mod fonts;
mod interrupt;
mod keyboard;
mod memory;
mod mouse;
mod mt;
mod sheet;
mod timer;
mod vga;
mod window;

use core::fmt::Write;
use core::panic::PanicInfo;

use asm::{cli, end_app, out8, sti};
use console::{console_task, Console, CONSOLE_BACKSPACE, CONSOLE_ENTER};
use fifo::Fifo;
use interrupt::PORT_KEYDAT;
use keyboard::{wait_kbc_sendready, KEYBOARD_OFFSET, KEYCMD_LED, KEYTABLE0, KEYTABLE1, LOCK_KEYS};
use memory::{MemMan, MEMMAN_ADDR};
use mouse::{Mouse, MouseDec, MOUSE_CURSOR_HEIGHT, MOUSE_CURSOR_WIDTH};
use mt::{TaskManager, TASK_MANAGER_ADDR};
use sheet::{SheetFlag, SheetManager};
use vga::{
    init_palette, init_screen, make_textbox, make_window, to_color, Color, ScreenWriter,
    SCREEN_HEIGHT, SCREEN_WIDTH,
};
use window::*;

pub static mut SHEET_MANAGER_ADDR: usize = 0;

#[no_mangle]
#[start]
pub extern "C" fn hrmain() {
    descriptor_table::init();
    interrupt::init();
    sti();
    interrupt::allow_input();

    let mut fifo = &mut Fifo::new(128, None);
    let fifo_addr = fifo as *const Fifo as usize;

    keyboard::init_keyboard(fifo_addr);
    timer::init_pit();
    init_palette();
    mouse::enable_mouse(fifo_addr);

    let memtotal = memory::memtest(0x00400000, 0xbfffffff);
    let memman = unsafe { &mut *(MEMMAN_ADDR as *mut MemMan) };
    *memman = MemMan::new();
    memman.free(0x00001000, 0x0009e000).unwrap();
    memman.free(0x00400000, 2).unwrap();
    memman.free(0x00400000, memtotal - 0x00400000).unwrap();

    let task_manager_addr = memman
        .alloc_4k(core::mem::size_of::<TaskManager>() as u32)
        .unwrap();
    unsafe {
        TASK_MANAGER_ADDR = task_manager_addr as usize;
    }
    let task_manager = unsafe { &mut *(task_manager_addr as *mut TaskManager) };
    *task_manager = TaskManager::new();
    let task_a_index = task_manager.init(memman, fifo_addr).unwrap();
    fifo.task_index = Some(task_a_index);

    let sheet_manager_addr = memman
        .alloc_4k(core::mem::size_of::<SheetManager>() as u32)
        .unwrap();
    let sheet_manager = unsafe { &mut *(sheet_manager_addr as *mut SheetManager) };
    let sheet_map_addr = memman
        .alloc_4k(*SCREEN_HEIGHT as u32 * *SCREEN_WIDTH as u32)
        .unwrap();
    *sheet_manager = SheetManager::new(sheet_map_addr as i32);
    let shi_bg = sheet_manager.alloc().unwrap();
    unsafe {
        SHEET_MANAGER_ADDR = sheet_manager_addr as usize;
    }
    let shi_mouse = sheet_manager.alloc().unwrap();
    let scrnx = *SCREEN_WIDTH as i32;
    let scrny = *SCREEN_HEIGHT as i32;
    let buf_bg_addr = memman.alloc_4k((scrnx * scrny) as u32).unwrap() as usize;
    let buf_mouse = [0u8; MOUSE_CURSOR_WIDTH * MOUSE_CURSOR_HEIGHT];
    let buf_mouse_addr =
        &buf_mouse as *const [u8; MOUSE_CURSOR_HEIGHT * MOUSE_CURSOR_WIDTH] as usize;
    sheet_manager.set_buf(shi_bg, buf_bg_addr, scrnx, scrny, None);
    sheet_manager.set_buf(
        shi_mouse,
        buf_mouse_addr,
        MOUSE_CURSOR_WIDTH as i32,
        MOUSE_CURSOR_HEIGHT as i32,
        Some(Color::DarkCyan),
    );

    init_screen(buf_bg_addr);
    let mouse_dec = MouseDec::new();
    let mx = (scrnx as i32 - MOUSE_CURSOR_WIDTH as i32) / 2;
    let my = (scrny as i32 - MOUSE_CURSOR_HEIGHT as i32 - 28) / 2;
    let mouse = Mouse::new(buf_mouse_addr);
    mouse.render();

    task_manager.run(task_a_index, 1, 2);

    const CONSOLE_WIDTH: usize = 256;
    const CONSOLE_HEIGHT: usize = 165;
    const CONSOLE_COUNT: usize = 2;
    let mut console_sheets: [usize; CONSOLE_COUNT] = [0; CONSOLE_COUNT];
    let mut console_bufs: [usize; CONSOLE_COUNT] = [0; CONSOLE_COUNT];
    let mut console_tasks: [usize; CONSOLE_COUNT] = [0; CONSOLE_COUNT];
    let mut console_fifos: [usize; CONSOLE_COUNT] = [0; CONSOLE_COUNT];

    for ci in 0..CONSOLE_COUNT {
        console_sheets[ci] = sheet_manager.alloc().unwrap();
        console_bufs[ci] = memman
            .alloc_4k((CONSOLE_WIDTH * CONSOLE_HEIGHT) as u32)
            .unwrap() as usize;
        sheet_manager.set_buf(
            console_sheets[ci],
            console_bufs[ci],
            CONSOLE_WIDTH as i32,
            CONSOLE_HEIGHT as i32,
            None,
        );
        make_window(
            console_bufs[ci],
            CONSOLE_WIDTH as isize,
            CONSOLE_HEIGHT as isize,
            "console",
            false,
        );
        make_textbox(
            console_bufs[ci],
            CONSOLE_WIDTH as isize,
            8,
            28,
            240,
            128,
            Color::Black,
        );
        console_tasks[ci] = task_manager.alloc().unwrap();
        let mut console_task_mut = &mut task_manager.tasks_data[console_tasks[ci]];

        console_fifos[ci] = memman.alloc_4k(128 * 4).unwrap() as usize;
        let console_fifo = unsafe { &mut *(console_fifos[ci] as *mut Fifo) };
        *console_fifo = Fifo::new(128, Some(console_tasks[ci]));
        console_task_mut.fifo_addr = console_fifos[ci];

        let console_esp = memman.alloc_4k(64 * 1024).unwrap() + 64 * 1024 - 12;
        console_task_mut.tss.esp = console_esp as i32;
        console_task_mut.tss.eip = console_task as i32;
        console_task_mut.tss.es = 1 * 8;
        console_task_mut.tss.cs = 2 * 8;
        console_task_mut.tss.ss = 1 * 8;
        console_task_mut.tss.ds = 1 * 8;
        console_task_mut.tss.fs = 1 * 8;
        console_task_mut.tss.gs = 1 * 8;

        let ptr = unsafe { &mut *((console_task_mut.tss.esp + 4) as *mut usize) };
        *ptr = console_sheets[ci];
        let ptr = unsafe { &mut *((console_task_mut.tss.esp + 8) as *mut usize) };
        *ptr = memtotal as usize;
        task_manager.run(console_tasks[ci], 2, 2);
        {
            let mut sheet_console = &mut sheet_manager.sheets_data[console_sheets[ci]];
            sheet_console.task_index = console_tasks[ci];
            sheet_console.cursor = true;
        }
    }

    sheet_manager.slide(shi_mouse, mx, my);
    sheet_manager.slide(console_sheets[0], 56, 6);
    sheet_manager.slide(console_sheets[1], 8, 2);
    sheet_manager.updown(shi_bg, Some(0));
    sheet_manager.updown(console_sheets[1], Some(1));
    sheet_manager.updown(console_sheets[0], Some(2));
    sheet_manager.updown(shi_mouse, Some(3));

    let mut active_window: usize = console_sheets[0];
    window_on(sheet_manager, task_manager, active_window);

    // シフトキー
    let mut key_shift = (false, false);
    // CapsLock, NumLock, ScreenLock
    let mut lock_keys = *LOCK_KEYS;
    let mut keycmd_wait: i32 = -1;
    // キーボードの状態管理用のFifo
    let keycmd = Fifo::new(32, None);
    keycmd.put(KEYCMD_LED as u32).unwrap();
    keycmd.put(lock_keys.as_bytes() as u32).unwrap();
    // ウィンドウの移動
    let mut moving = false;
    let mut mouse_move_x = 0;
    let mut mouse_move_y = 0;
    let mut tmp_sheet_x = 0;
    let mut new_mx = -1;
    let mut new_my = 0;
    let mut new_wx = 0x7fffffff;
    let mut new_wy = 0;
    let mut target_sheet_index = 0;

    loop {
        // キーボードコントローラに送るデータがあれば送る
        if keycmd.status() > 0 && keycmd_wait < 0 {
            keycmd_wait = keycmd.get().unwrap() as u8 as i32;
            wait_kbc_sendready();
            out8(PORT_KEYDAT, keycmd_wait as u8);
        }
        cli();
        if fifo.status() != 0 {
            let i = fifo.get().unwrap();
            sti();
            let active_sheet = sheet_manager.sheets_data[active_window];
            if active_sheet.flag == SheetFlag::AVAILABLE {
                // ウィンドウが閉じられた
                if let Some(zmax) = sheet_manager.z_max {
                    active_window = sheet_manager.sheets[zmax - 1];
                    window_on(sheet_manager, task_manager, active_window);
                }
            }
            if KEYBOARD_OFFSET <= i && i <= 511 {
                let key = i - KEYBOARD_OFFSET;
                let mut chr = 0 as u8;
                if key < KEYTABLE0.len() as u32 {
                    if key_shift == (false, false) {
                        chr = KEYTABLE0[key as usize];
                    } else {
                        chr = KEYTABLE1[key as usize];
                    }
                }
                if b'A' <= chr && chr <= b'Z' {
                    // アルファベットの場合、ShiftキーとCapsLockの状態で大文字小文字を決める
                    if !lock_keys.caps_lock && key_shift == (false, false)
                        || lock_keys.caps_lock && key_shift != (false, false)
                    {
                        chr += 0x20;
                    }
                }
                if chr != 0 {
                    let ctask = task_manager.tasks_data[active_sheet.task_index];
                    let fifo = unsafe { &*(ctask.fifo_addr as *const Fifo) };
                    fifo.put(chr as u32 + KEYBOARD_OFFSET).unwrap();
                }
                // Enterキー
                if key == 0x1c {
                    let ctask = task_manager.tasks_data[active_sheet.task_index];
                    let fifo = unsafe { &*(ctask.fifo_addr as *const Fifo) };
                    fifo.put(CONSOLE_ENTER + KEYBOARD_OFFSET).unwrap();
                }
                // バックスペース
                if key == 0x0e {
                    let ctask = task_manager.tasks_data[active_sheet.task_index];
                    let fifo = unsafe { &*(ctask.fifo_addr as *const Fifo) };
                    fifo.put(CONSOLE_BACKSPACE + KEYBOARD_OFFSET).unwrap();
                }
                // タブ
                if key == 0x0f {
                    window_off(sheet_manager, task_manager, active_window);
                    let mut j = active_sheet.z.unwrap() - 1;
                    if j == 0 && sheet_manager.z_max.is_some() && sheet_manager.z_max.unwrap() > 0 {
                        j = sheet_manager.z_max.unwrap() - 1;
                    }
                    active_window = sheet_manager.sheets[j];
                    window_on(sheet_manager, task_manager, active_window);
                    sheet_manager.updown(
                        active_window,
                        if let Some(zmax) = sheet_manager.z_max {
                            if zmax > 0 {
                                Some(zmax - 1)
                            } else {
                                Some(0)
                            }
                        } else {
                            None
                        },
                    );
                }
                // 左シフト ON
                if key == 0x2a {
                    key_shift.0 = true;
                }
                // 右シフト ON
                if key == 0x36 {
                    key_shift.1 = true;
                }
                // 左シフト OFF
                if key == 0xaa {
                    key_shift.0 = false;
                }
                // 右シフト OFF
                if key == 0xb6 {
                    key_shift.1 = false;
                }
                // CapsLock
                if key == 0x3a {
                    lock_keys.caps_lock = !lock_keys.caps_lock;
                    keycmd.put(KEYCMD_LED as u32).unwrap();
                    keycmd.put(lock_keys.as_bytes() as u32).unwrap();
                }
                // NumLock
                if key == 0x45 {
                    lock_keys.num_lock = !lock_keys.num_lock;
                    keycmd.put(KEYCMD_LED as u32).unwrap();
                    keycmd.put(lock_keys.as_bytes() as u32).unwrap();
                }
                // ScrollLock
                if key == 0x46 {
                    lock_keys.scroll_lock = !lock_keys.scroll_lock;
                    keycmd.put(KEYCMD_LED as u32).unwrap();
                    keycmd.put(lock_keys.as_bytes() as u32).unwrap();
                }
                // Shift + F1 でアプリケーションを強制終了
                {
                    let mut console_task_mut =
                        &mut task_manager.tasks_data[active_sheet.task_index];
                    if key == 0x3b
                        && (key_shift.0 == true || key_shift.1 == true)
                        && console_task_mut.tss.ss0 != 0
                    {
                        let console =
                            unsafe { &mut *(console_task_mut.console_addr as *mut Console) };
                        let message = b"\nBreak(key) :\n";
                        console.put_string(message.as_ptr() as usize, message.len(), 8);
                        cli();
                        console_task_mut.tss.eax =
                            unsafe { &console_task_mut.tss.esp0 } as *const i32 as i32;
                        console_task_mut.tss.eip = end_app as i32;
                        sti();
                    }
                }
                // F11 で 1 の位置にあるSheetを下げる
                if key == 0x57 && sheet_manager.z_max.is_some() && sheet_manager.z_max.unwrap() > 2
                {
                    let z = sheet_manager.z_max.unwrap();
                    let sheet_index = sheet_manager.sheets[1];
                    sheet_manager.updown(sheet_index, Some(z - 1));
                }
                // キーボードがデータを無事に受け取った
                if key == 0xfa {
                    keycmd_wait = -1;
                }
                // キーボードがデータを無事に受け取れなかった
                if key == 0xfe {
                    wait_kbc_sendready();
                    out8(PORT_KEYDAT, keycmd_wait as u8);
                }
            } else if 512 <= i && i <= 767 {
                if mouse_dec.decode((i - 512) as u8).is_some() {
                    let (new_x, new_y) = sheet_manager.get_new_point(
                        shi_mouse,
                        mouse_dec.x.get(),
                        mouse_dec.y.get(),
                    );
                    new_mx = new_x;
                    new_my = new_y;
                    // 左クリックをおしていた場合
                    if (mouse_dec.btn.get() & 0x01) != 0 {
                        if moving {
                            let x = new_x - mouse_move_x;
                            let y = new_y - mouse_move_y;
                            let sheet = sheet_manager.sheets_data[target_sheet_index];
                            new_wx = (x + tmp_sheet_x + 2) & !3;
                            new_wy = new_wy + y;
                            mouse_move_y = new_y;
                        } else {
                            // Sheetの順番を入れ替え
                            if let Some(z) = sheet_manager.z_max {
                                let mut h = z - 1;
                                while h > 0 {
                                    target_sheet_index = sheet_manager.sheets[h];
                                    let sheet = sheet_manager.sheets_data[target_sheet_index];
                                    let x = new_x - sheet.x;
                                    let y = new_y - sheet.y;

                                    if 0 <= x && x < sheet.width && 0 <= y && y < sheet.height {
                                        let color = unsafe {
                                            *((sheet.buf_addr
                                                + y as usize * sheet.width as usize
                                                + x as usize)
                                                as *const i8)
                                        };
                                        if to_color(color) != sheet.transparent {
                                            sheet_manager.updown(target_sheet_index, Some(z - 1));
                                            if active_window != target_sheet_index {
                                                window_off(
                                                    sheet_manager,
                                                    task_manager,
                                                    active_window,
                                                );
                                                active_window = target_sheet_index;
                                                window_on(
                                                    sheet_manager,
                                                    task_manager,
                                                    active_window,
                                                );
                                            }
                                            if 3 <= x && x < sheet.width - 3 && 3 <= y && y < 21 {
                                                // ウィンドウ移動モードへ
                                                moving = true;
                                                mouse_move_x = new_x;
                                                mouse_move_y = new_y;
                                                tmp_sheet_x = sheet.x;
                                                new_wy = sheet.y;
                                            }
                                            if sheet.width - 21 <= x
                                                && x < sheet.width - 5
                                                && 5 <= y
                                                && y < 19
                                            {
                                                //×ボタンクリック
                                                if sheet.from_app {
                                                    let task =
                                                        task_manager.tasks_data[sheet.task_index];
                                                    let console = unsafe {
                                                        &mut *(task.console_addr as *mut Console)
                                                    };
                                                    let message = b"\nBreak(mouse) :\n";
                                                    console.put_string(
                                                        message.as_ptr() as usize,
                                                        message.len(),
                                                        8,
                                                    );
                                                    cli();
                                                    {
                                                        let mut console_task_mut =
                                                            &mut task_manager.tasks_data
                                                                [sheet.task_index];
                                                        console_task_mut.tss.eax =
                                                            unsafe { &console_task_mut.tss.esp0 }
                                                                as *const i32
                                                                as i32;
                                                        console_task_mut.tss.eip = end_app as i32;
                                                    }
                                                    sti();
                                                }
                                            }
                                            break;
                                        }
                                    }
                                    h -= 1;
                                }
                            }
                        }
                    } else {
                        // 左クリックを押してなかったらウィンドウ移動モードからもどす
                        moving = false;
                        if new_wx != 0x7fffffff {
                            sheet_manager.slide(target_sheet_index, new_wx, new_wy);
                            new_wx = 0x7fffffff;
                        }
                    }
                }
            }
        } else {
            if new_mx >= 0 {
                sti();
                sheet_manager.slide(shi_mouse, new_mx, new_my);
                new_mx = -1;
            } else if new_wx != 0x7fffffff {
                sti();
                sheet_manager.slide(target_sheet_index, new_wx, new_wy);
                new_wx = 0x7fffffff;
            } else {
                task_manager.sleep(task_a_index);
                sti();
            }
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut writer = ScreenWriter::new(
        None,
        Color::LightRed,
        0,
        0,
        *SCREEN_WIDTH as usize,
        *SCREEN_HEIGHT as usize,
    );
    write!(writer, "[ERR] {:?}", info).unwrap();
    loop {
        asm::hlt()
    }
}
