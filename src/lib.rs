#![no_std]
#![feature(asm)]
#![feature(start)]
#![feature(naked_functions)]

use core::panic::PanicInfo;

mod asm;
mod descriptor_table;
mod fifo;
mod fonts;
mod interrupt;
mod keyboard;
mod memory;
mod mouse;
mod mt;
mod sheet;
mod timer;
mod vga;

static mut SHEET_MANAGER_ADDR: usize = 0;

#[no_mangle]
#[start]
pub extern "C" fn haribote_os() {
    use asm::{cli, sti, stihlt};
    use fifo::Fifo;
    use keyboard::{KEYBOARD_OFFSET, KEYTABLE};
    use memory::{MemMan, MEMMAN_ADDR};
    use mouse::{Mouse, MouseDec, MOUSE_CURSOR_HEIGHT, MOUSE_CURSOR_WIDTH};
    use mt::{TaskManager, TASK_MANAGER_ADDR};
    use sheet::SheetManager;
    use timer::TIMER_MANAGER;
    use vga::{
        boxfill, init_palette, init_screen, make_textbox, make_window, Color, SCREEN_HEIGHT,
        SCREEN_WIDTH,
    };

    let fifo = Fifo::new(128);
    let fifo_addr = &fifo as *const Fifo as usize;

    descriptor_table::init();
    interrupt::init();
    sti();
    interrupt::allow_input();
    keyboard::init_keyboard(fifo_addr);
    timer::init_pit();
    init_palette();
    mouse::enable_mouse(fifo_addr);

    let timer_index3 = TIMER_MANAGER.lock().alloc().unwrap();
    TIMER_MANAGER.lock().init_timer(timer_index3, fifo_addr, 1);
    TIMER_MANAGER.lock().set_time(timer_index3, 50);

    let memtotal = memory::memtest(0x00400000, 0xbfffffff);
    let memman = unsafe { &mut *(MEMMAN_ADDR as *mut MemMan) };
    *memman = MemMan::new();
    memman.free(0x00001000, 0x0009e000).unwrap();
    memman.free(0x00400000, 2).unwrap();
    memman.free(0x00400000, memtotal - 0x00400000).unwrap();

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
    let shi_win = sheet_manager.alloc().unwrap();
    let scrnx = *SCREEN_WIDTH as i32;
    let scrny = *SCREEN_HEIGHT as i32;
    let buf_bg_addr = memman.alloc_4k((scrnx * scrny) as u32).unwrap() as usize;
    let buf_win_addr = memman.alloc_4k((144 * 52) as u32).unwrap() as usize;
    let buf_mouse = [0u8; MOUSE_CURSOR_WIDTH * MOUSE_CURSOR_HEIGHT];
    let buf_mouse_addr =
        &buf_mouse as *const [u8; MOUSE_CURSOR_HEIGHT * MOUSE_CURSOR_WIDTH] as usize;
    sheet_manager.set_buf(shi_bg, buf_bg_addr, scrnx, scrny, None);
    sheet_manager.set_buf(shi_win, buf_win_addr, 144, 52, None);
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

    make_window(buf_win_addr, 144, 52, "window");
    make_textbox(buf_win_addr, 144, 8, 28, 128, 16, Color::White);

    write_with_bg!(
        sheet_manager,
        shi_bg,
        *SCREEN_WIDTH as isize,
        *SCREEN_HEIGHT as isize,
        0,
        32,
        Color::White,
        Color::DarkCyan,
        27,
        "total: {:>2}MB  free: {:>6}KB",
        memtotal / (1024 * 1024),
        memman.total() / 1024
    );

    let task_manager_addr = memman
        .alloc_4k(core::mem::size_of::<TaskManager>() as u32)
        .unwrap();
    unsafe {
        TASK_MANAGER_ADDR = task_manager_addr as usize;
    }
    let task_manager = unsafe { &mut *(task_manager_addr as *mut TaskManager) };
    *task_manager = TaskManager::new();
    let task_a_index = task_manager.init(memman).unwrap();
    {
        let mut fifo_mut = unsafe { &mut *(fifo_addr as *mut Fifo) };
        fifo_mut.task_index = Some(task_a_index);
    }
    task_manager.run(task_a_index, 1, 2);

    let mut sheet_win_b: [usize; 3] = [0; 3];
    let mut task_b: [usize; 3] = [0; 3];

    const B_WIN_HEIGHT: usize = 52;
    const B_WIN_WIDTH: usize = 144;

    for i in 0..(3 as usize) {
        sheet_win_b[i] = sheet_manager.alloc().unwrap();
        let buf_win_b_addr = memman
            .alloc_4k((B_WIN_WIDTH * B_WIN_HEIGHT) as u32)
            .unwrap();
        sheet_manager.set_buf(
            sheet_win_b[i],
            buf_win_b_addr as usize,
            B_WIN_WIDTH as i32,
            B_WIN_HEIGHT as i32,
            None,
        );
        make_window(
            buf_win_b_addr as usize,
            B_WIN_WIDTH as i32,
            B_WIN_HEIGHT as i32,
            "",
        );
        // titleを動的に作成したいので、ここでwrite
        use core::fmt::Write;
        let mut writer = vga::ScreenWriter::new(
            Some(buf_win_b_addr as usize),
            Color::White,
            24,
            4,
            B_WIN_WIDTH,
            B_WIN_HEIGHT,
        );
        write!(writer, "window_b{}", i).unwrap();

        task_b[i] = task_manager.alloc().unwrap();
        let mut task_b_mut = &mut task_manager.tasks_data[task_b[i]];
        let task_b_esp = memman.alloc_4k(64 * 1024).unwrap() + 64 * 1024 - 8;
        task_b_mut.tss.esp = task_b_esp as i32;
        task_b_mut.tss.eip = task_b_main as i32;
        task_b_mut.tss.es = 1 * 8;
        task_b_mut.tss.cs = 2 * 8;
        task_b_mut.tss.ss = 1 * 8;
        task_b_mut.tss.ds = 1 * 8;
        task_b_mut.tss.fs = 1 * 8;
        task_b_mut.tss.gs = 1 * 8;
        // 第1引数にsheet_win_b[i]を読みこみ
        let ptr = unsafe { &mut *((task_b_mut.tss.esp + 4) as *mut usize) };
        *ptr = sheet_win_b[i];
        // task_manager.run(task_b[i], 2, (i + 1) as i32);
    }

    sheet_manager.slide(shi_mouse, mx, my);
    sheet_manager.slide(shi_win, 8, 56);
    sheet_manager.slide(sheet_win_b[0], 168, 56);
    sheet_manager.slide(sheet_win_b[1], 8, 116);
    sheet_manager.slide(sheet_win_b[2], 168, 116);
    sheet_manager.updown(shi_bg, Some(0));
    sheet_manager.updown(shi_win, Some(1));
    sheet_manager.updown(sheet_win_b[0], Some(2));
    sheet_manager.updown(sheet_win_b[1], Some(3));
    sheet_manager.updown(sheet_win_b[2], Some(4));
    sheet_manager.updown(shi_mouse, Some(5));

    // カーソル
    let min_cursor_x = 8;
    let max_cursor_x = 144;
    let mut cursor_x = min_cursor_x;
    let mut cursor_c = Color::White;

    loop {
        cli();
        if fifo.status() != 0 {
            let i = fifo.get().unwrap();
            sti();
            if KEYBOARD_OFFSET <= i && i <= 511 {
                let key = i - KEYBOARD_OFFSET;
                write_with_bg!(
                    sheet_manager,
                    shi_bg,
                    *SCREEN_WIDTH as isize,
                    *SCREEN_HEIGHT as isize,
                    0,
                    0,
                    Color::White,
                    Color::DarkCyan,
                    2,
                    "{:x}",
                    key
                );
                if key < KEYTABLE.len() as u32 {
                    if KEYTABLE[key as usize] != 0 && cursor_x < max_cursor_x {
                        write_with_bg!(
                            sheet_manager,
                            shi_win,
                            144,
                            52,
                            cursor_x,
                            28,
                            Color::Black,
                            Color::White,
                            1,
                            "{}",
                            KEYTABLE[key as usize] as char,
                        );
                        cursor_x += 8;
                    }
                    // バックスペース
                    if key == 0x0e && cursor_x > min_cursor_x {
                        write_with_bg!(
                            sheet_manager,
                            shi_win,
                            144,
                            52,
                            cursor_x,
                            28,
                            Color::Black,
                            Color::White,
                            1,
                            " "
                        );
                        cursor_x -= 8;
                    }
                    boxfill(buf_win_addr, 144, cursor_c, cursor_x, 28, cursor_x + 8, 43);
                    sheet_manager.refresh(shi_win, cursor_x as i32, 28, cursor_x as i32 + 8, 44)
                }
            } else if 512 <= i && i <= 767 {
                if mouse_dec.decode((i - 512) as u8).is_some() {
                    write_with_bg!(
                        sheet_manager,
                        shi_bg,
                        *SCREEN_WIDTH as isize,
                        *SCREEN_HEIGHT as isize,
                        32,
                        0,
                        Color::White,
                        Color::DarkCyan,
                        15,
                        "[{}{}{} {:>4},{:>4}]",
                        if (mouse_dec.btn.get() & 0x01) != 0 {
                            'L'
                        } else {
                            'l'
                        },
                        if (mouse_dec.btn.get() & 0x04) != 0 {
                            'C'
                        } else {
                            'c'
                        },
                        if (mouse_dec.btn.get() & 0x02) != 0 {
                            'R'
                        } else {
                            'r'
                        },
                        mouse_dec.x.get(),
                        mouse_dec.y.get(),
                    );
                    let (new_x, new_y) = sheet_manager.get_new_point(
                        shi_mouse,
                        mouse_dec.x.get(),
                        mouse_dec.y.get(),
                    );
                    sheet_manager.slide(shi_mouse, new_x, new_y);
                    // 左クリックをおしていた場合
                    if (mouse_dec.btn.get() & 0x01) != 0 {
                        sheet_manager.slide(shi_win, new_x - 80, new_y - 8);
                    }
                }
            } else {
                if i != 0 {
                    TIMER_MANAGER.lock().init_timer(timer_index3, fifo_addr, 0);
                    cursor_c = Color::Black;
                } else {
                    TIMER_MANAGER.lock().init_timer(timer_index3, fifo_addr, 1);
                    cursor_c = Color::White;
                }
                TIMER_MANAGER.lock().set_time(timer_index3, 50);
                boxfill(buf_win_addr, 144, cursor_c, cursor_x, 28, cursor_x + 8, 43);
                sheet_manager.refresh(shi_win, cursor_x as i32, 28, cursor_x as i32 + 8, 44)
            }
        } else {
            task_manager.sleep(task_a_index);
            sti();
        }
    }
}

pub extern "C" fn task_b_main(sheet_win: usize) {
    use asm::{cli, hlt, sti};
    use fifo::Fifo;
    use sheet::SheetManager;
    use timer::TIMER_MANAGER;
    use vga::Color;

    let fifo = Fifo::new(128);
    let fifo_addr = &fifo as *const Fifo as usize;

    let sheet_manager_addr = unsafe { SHEET_MANAGER_ADDR };
    let sheet_manager = unsafe { &mut *(sheet_manager_addr as *mut SheetManager) };

    let timer_index_sp = TIMER_MANAGER.lock().alloc().unwrap();
    TIMER_MANAGER
        .lock()
        .init_timer(timer_index_sp, fifo_addr, 8);
    TIMER_MANAGER.lock().set_time(timer_index_sp, 800);
    let mut count = 0;
    loop {
        count += 1;
        cli();
        if fifo.status() == 0 {
            sti();
        } else {
            let i = fifo.get().unwrap();
            sti();
            if i == 8 {
                write_with_bg!(
                    sheet_manager,
                    sheet_win,
                    144,
                    52,
                    24,
                    28,
                    Color::Black,
                    Color::LightGray,
                    11,
                    "{:>11}",
                    count
                );
            }
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use vga::{ScreenWriter, SCREEN_HEIGHT, SCREEN_WIDTH};
    let mut writer = ScreenWriter::new(
        None,
        vga::Color::LightRed,
        0,
        0,
        *SCREEN_WIDTH as usize,
        *SCREEN_HEIGHT as usize,
    );
    use core::fmt::Write;
    write!(writer, "[ERR] {:?}", info).unwrap();
    loop {
        asm::hlt()
    }
}
