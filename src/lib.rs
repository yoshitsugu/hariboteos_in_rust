#![no_std]
#![feature(asm)]
#![feature(start)]
#![feature(naked_functions)]

use core::default::Default;
use core::panic::PanicInfo;

mod asm;
mod descriptor_table;
mod fifo;
mod fonts;
mod interrupt;
mod keyboard;
mod memory;
mod mouse;
mod sheet;
mod timer;
mod tss;
mod vga;

static mut SHEET_BG_ADDR: usize = 0;
static mut SHEET_MANAGER_ADDR: usize = 0;

#[no_mangle]
#[start]
pub extern "C" fn haribote_os() {
    use asm::{cli, farjmp, load_tr, sti, stihlt};
    use descriptor_table::{SegmentDescriptor, ADR_GDT, AR_TSS32};
    use fifo::Fifo;
    use keyboard::{KEYBOARD_OFFSET, KEYTABLE};
    use memory::{MemMan, MEMMAN_ADDR};
    use mouse::{Mouse, MouseDec, MOUSE_CURSOR_HEIGHT, MOUSE_CURSOR_WIDTH};
    use sheet::SheetManager;
    use timer::TIMER_MANAGER;
    use tss::TSS;
    use vga::{
        boxfill, init_palette, init_screen, make_textbox, make_window, Color, ScreenWriter,
        SCREEN_HEIGHT, SCREEN_WIDTH,
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

    let timer_index_ts = TIMER_MANAGER.lock().alloc().unwrap();
    TIMER_MANAGER
        .lock()
        .init_timer(timer_index_ts, fifo_addr, 2);
    TIMER_MANAGER.lock().set_time(timer_index_ts, 2);
    let timer_index1 = TIMER_MANAGER.lock().alloc().unwrap();
    TIMER_MANAGER.lock().init_timer(timer_index1, fifo_addr, 10);
    TIMER_MANAGER.lock().set_time(timer_index1, 1000);
    let timer_index2 = TIMER_MANAGER.lock().alloc().unwrap();
    TIMER_MANAGER.lock().init_timer(timer_index2, fifo_addr, 3);
    TIMER_MANAGER.lock().set_time(timer_index2, 300);
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
        SHEET_BG_ADDR = shi_bg;
    }
    let shi_mouse = sheet_manager.alloc().unwrap();
    let shi_win = sheet_manager.alloc().unwrap();
    let scrnx = *SCREEN_WIDTH as i32;
    let scrny = *SCREEN_HEIGHT as i32;
    let buf_bg_addr = memman.alloc_4k((scrnx * scrny) as u32).unwrap() as usize;
    let buf_win_addr = memman.alloc_4k((160 * 52) as u32).unwrap() as usize;
    let buf_mouse = [0u8; MOUSE_CURSOR_WIDTH * MOUSE_CURSOR_HEIGHT];
    let buf_mouse_addr =
        &buf_mouse as *const [u8; MOUSE_CURSOR_HEIGHT * MOUSE_CURSOR_WIDTH] as usize;
    sheet_manager.set_buf(shi_bg, buf_bg_addr, scrnx, scrny, None);
    sheet_manager.set_buf(shi_win, buf_win_addr, 160, 52, None);
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

    make_window(buf_win_addr, 160, 52, "window");
    make_textbox(buf_win_addr, 160, 8, 28, 144, 16, Color::White);

    sheet_manager.slide(shi_mouse, mx, my);
    sheet_manager.slide(shi_win, 80, 72);
    sheet_manager.updown(shi_bg, Some(0));
    sheet_manager.updown(shi_win, Some(1));
    sheet_manager.updown(shi_mouse, Some(2));

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

    let mut tss_a: TSS = Default::default();
    tss_a.ldtr = 0;
    tss_a.iomap = 0x40000000;
    let mut tss_b: TSS = Default::default();
    tss_b.ldtr = 0;
    tss_b.iomap = 0x40000000;
    let gdt = unsafe { &mut *((ADR_GDT + 3 * 8) as *mut SegmentDescriptor) };
    *gdt = SegmentDescriptor::new(103, &tss_a as *const TSS as i32, AR_TSS32);
    let gdt = unsafe { &mut *((ADR_GDT + 4 * 8) as *mut SegmentDescriptor) };
    *gdt = SegmentDescriptor::new(103, &tss_b as *const TSS as i32, AR_TSS32);
    load_tr(3 * 8);
    let task_b_esp = memman.alloc_4k(64 * 1024).unwrap() + 64 * 1024;
    tss_b.eip = task_b_main as i32;
    tss_b.eflags = 0x00000202; /* IF = 1; */
    tss_b.eax = 0;
    tss_b.ecx = 0;
    tss_b.edx = 0;
    tss_b.ebx = 0;
    tss_b.esp = task_b_esp as i32;
    tss_b.ebp = 0;
    tss_b.esi = 0;
    tss_b.edi = 0;
    tss_b.es = 1 * 8;
    tss_b.cs = 2 * 8;
    tss_b.ss = 1 * 8;
    tss_b.ds = 1 * 8;
    tss_b.fs = 1 * 8;
    tss_b.gs = 1 * 8;

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
            if i == 2 {
                farjmp(0, 4 * 8);
                TIMER_MANAGER.lock().set_time(timer_index_ts, 2);
            } else if KEYBOARD_OFFSET <= i && i <= 511 {
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
                            160,
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
                            160,
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
                    boxfill(buf_win_addr, 160, cursor_c, cursor_x, 28, cursor_x + 8, 43);
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
            } else if i == 10 {
                write_with_bg!(
                    sheet_manager,
                    shi_bg,
                    *SCREEN_WIDTH as isize,
                    *SCREEN_HEIGHT as isize,
                    0,
                    64,
                    Color::White,
                    Color::DarkCyan,
                    7,
                    "10[sec]"
                );
            } else if i == 3 {
                write_with_bg!(
                    sheet_manager,
                    shi_bg,
                    *SCREEN_WIDTH as isize,
                    *SCREEN_HEIGHT as isize,
                    0,
                    80,
                    Color::White,
                    Color::DarkCyan,
                    6,
                    "3[sec]"
                );
            } else {
                if i != 0 {
                    TIMER_MANAGER.lock().init_timer(timer_index3, fifo_addr, 0);
                    cursor_c = Color::Black;
                } else {
                    TIMER_MANAGER.lock().init_timer(timer_index3, fifo_addr, 1);
                    cursor_c = Color::White;
                }
                TIMER_MANAGER.lock().set_time(timer_index3, 50);
                boxfill(buf_win_addr, 160, cursor_c, cursor_x, 28, cursor_x + 8, 43);
                sheet_manager.refresh(shi_win, cursor_x as i32, 28, cursor_x as i32 + 8, 44)
            }
        } else {
            stihlt();
        }
    }
}

pub extern "C" fn task_b_main() {
    use asm::{cli, farjmp, hlt, sti};
    use fifo::Fifo;
    use sheet::SheetManager;
    use timer::TIMER_MANAGER;
    use vga::{Color, SCREEN_HEIGHT, SCREEN_WIDTH};

    let fifo = Fifo::new(128);
    let fifo_addr = &fifo as *const Fifo as usize;

    let shi_bg = unsafe { SHEET_BG_ADDR };
    let sheet_manager_addr = unsafe { SHEET_MANAGER_ADDR };
    let sheet_manager = unsafe { &mut *(sheet_manager_addr as *mut SheetManager) };

    let timer_index_ts = TIMER_MANAGER.lock().alloc().unwrap();
    TIMER_MANAGER
        .lock()
        .init_timer(timer_index_ts, fifo_addr, 2);
    TIMER_MANAGER.lock().set_time(timer_index_ts, 2);
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
            if i == 2 {
                farjmp(0, 3 * 8);
                TIMER_MANAGER.lock().set_time(timer_index_ts, 2);
            } else if i == 8 {
                write_with_bg!(
                    sheet_manager,
                    shi_bg,
                    *SCREEN_WIDTH as isize,
                    *SCREEN_HEIGHT as isize,
                    10,
                    144,
                    Color::White,
                    Color::DarkCyan,
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
