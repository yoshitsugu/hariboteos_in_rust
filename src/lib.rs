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
mod memory;
mod mouse;
mod sheet;
mod vga;

#[no_mangle]
#[start]
pub extern "C" fn haribote_os() {
    use asm::{cli, sti, stihlt};
    use core::fmt::Write;
    use interrupt::{enable_mouse, KEYBUF, MOUSEBUF};
    use memory::{MemMan, MEMMAN_ADDR};
    use mouse::{Mouse, MouseDec, MOUSE_CURSOR_HEIGHT, MOUSE_CURSOR_WIDTH};
    use sheet::SheetManager;
    use vga::{
        boxfill, init_palette, init_screen, make_window, Color, ScreenWriter, SCREEN_HEIGHT,
        SCREEN_WIDTH,
    };

    descriptor_table::init();
    interrupt::init();
    sti();
    interrupt::allow_input();
    init_palette();
    enable_mouse();
    let memtotal = memory::memtest(0x00400000, 0xbfffffff);
    let memman = unsafe { &mut *(MEMMAN_ADDR as *mut MemMan) };
    *memman = MemMan::new();
    memman.free(0x00001000, 0x0009e000).unwrap();
    memman.free(0x00400000, 2).unwrap();
    memman.free(0x00400000, memtotal - 0x00400000).unwrap();

    let sheet_manager = unsafe {
        &mut *(memman
            .alloc_4k(core::mem::size_of::<SheetManager>() as u32)
            .unwrap() as *mut SheetManager)
    };
    *sheet_manager = SheetManager::new();
    let shi_bg = sheet_manager.alloc().unwrap();
    let shi_mouse = sheet_manager.alloc().unwrap();
    let shi_win = sheet_manager.alloc().unwrap();
    let scrnx = *SCREEN_WIDTH as i32;
    let scrny = *SCREEN_HEIGHT as i32;
    let buf_bg_addr = memman.alloc_4k((scrnx * scrny) as u32).unwrap() as usize;
    let buf_win_addr = memman.alloc_4k((160 * 68) as u32).unwrap() as usize;
    let buf_mouse = [0u8; MOUSE_CURSOR_WIDTH * MOUSE_CURSOR_HEIGHT];
    let buf_mouse_addr =
        &buf_mouse as *const [u8; MOUSE_CURSOR_HEIGHT * MOUSE_CURSOR_WIDTH] as usize;
    sheet_manager.set_buf(shi_bg, buf_bg_addr, scrnx, scrny, None);
    sheet_manager.set_buf(shi_win, buf_win_addr, 160, 68, None);
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

    make_window(buf_win_addr, 160, 68, "window");
    let mut writer = ScreenWriter::new(Some(buf_win_addr), vga::Color::Black, 24, 28, 160, 68);
    write!(writer, "Welcome to\n Haribote OS!").unwrap();

    sheet_manager.slide(shi_mouse, mx, my);
    sheet_manager.slide(shi_win, 80, 72);
    sheet_manager.updown(shi_bg, Some(0));
    sheet_manager.updown(shi_mouse, Some(1));
    sheet_manager.updown(shi_win, Some(2));

    boxfill(
        buf_bg_addr,
        *SCREEN_WIDTH as isize,
        Color::DarkCyan,
        0,
        32,
        100,
        48,
    );
    let mut writer = ScreenWriter::new(
        Some(buf_bg_addr),
        vga::Color::White,
        0,
        32,
        *SCREEN_WIDTH as usize,
        *SCREEN_HEIGHT as usize,
    );
    write!(
        writer,
        "total: {}MB  free: {}KB",
        memtotal / (1024 * 1024),
        memman.total() / 1024
    )
    .unwrap();
    sheet_manager.refresh(shi_bg, 0, 0, scrnx, 48);
    loop {
        cli();
        if KEYBUF.lock().status() != 0 {
            let key = KEYBUF.lock().get().unwrap();
            sti();
            boxfill(
                buf_bg_addr,
                *SCREEN_WIDTH as isize,
                Color::DarkCyan,
                0,
                0,
                16,
                16,
            );
            let mut writer = ScreenWriter::new(
                Some(buf_bg_addr),
                vga::Color::White,
                0,
                0,
                *SCREEN_WIDTH as usize,
                *SCREEN_HEIGHT as usize,
            );
            write!(writer, "{:x}", key).unwrap();
            sheet_manager.refresh(shi_bg, 0, 0, 16, 16);
        } else if MOUSEBUF.lock().status() != 0 {
            let i = MOUSEBUF.lock().get().unwrap();
            sti();
            if mouse_dec.decode(i).is_some() {
                boxfill(
                    buf_bg_addr,
                    *SCREEN_WIDTH as isize,
                    Color::DarkCyan,
                    32,
                    0,
                    32 + 15 * 8,
                    16,
                );
                let mut writer = ScreenWriter::new(
                    Some(buf_bg_addr),
                    vga::Color::White,
                    32,
                    0,
                    *SCREEN_WIDTH as usize,
                    *SCREEN_HEIGHT as usize,
                );
                write!(
                    writer,
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
                )
                .unwrap();
                sheet_manager.refresh(shi_bg, 32, 0, 32 + 15 * 8, 16);
                sheet_manager.slide_by_diff(shi_mouse, mouse_dec.x.get(), mouse_dec.y.get());
            }
        } else {
            stihlt();
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
