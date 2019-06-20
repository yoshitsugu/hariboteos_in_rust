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
mod vga;

#[no_mangle]
#[start]
pub extern "C" fn haribote_os() {
    use asm::{cli, sti, stihlt};
    use core::fmt::Write;
    use interrupt::{enable_mouse, KEYBUF, MOUSEBUF};
    use memory::{MemMan, MEMMAN_ADDR};
    use mouse::{Mouse, MouseDec, MOUSE_CURSOR_HEIGHT, MOUSE_CURSOR_WIDTH};
    use vga::{Color, Screen, ScreenWriter};

    descriptor_table::init();
    interrupt::init();
    sti();
    interrupt::allow_input();
    let mut screen = Screen::new();
    screen.init();
    let mouse_dec = MouseDec::new();
    let mouse = Mouse::new(
        (screen.scrnx as i32 - MOUSE_CURSOR_WIDTH as i32) / 2,
        (screen.scrny as i32 - MOUSE_CURSOR_HEIGHT as i32 - 28) / 2,
    );
    mouse.render();
    enable_mouse();
    let memtotal = memory::memtest(0x00400000, 0xbfffffff);
    let memman = unsafe { &mut *(MEMMAN_ADDR as *mut MemMan) };
    *memman = MemMan::new();
    memman.free(0x00001000, 0x0009e000).unwrap();
    memman.free(0x00400000, 2).unwrap();
    memman.free(0x00400000, memtotal - 0x00400000).unwrap();
    (Screen::new()).boxfill8(Color::DarkCyan, 0, 32, 100, 48);
    let mut writer = ScreenWriter::new(Screen::new(), vga::Color::White, 0, 32);
    write!(
        writer,
        "total: {}MB  free: {}KB",
        memtotal / (1024 * 1024),
        memman.total() / 1024
    )
    .unwrap();
    loop {
        cli();
        if KEYBUF.lock().status() != 0 {
            let key = KEYBUF.lock().get().unwrap();
            sti();
            (Screen::new()).boxfill8(Color::DarkCyan, 0, 0, 16, 16);
            let mut writer = ScreenWriter::new(Screen::new(), vga::Color::White, 0, 0);
            write!(writer, "{:x}", key).unwrap();
        } else if MOUSEBUF.lock().status() != 0 {
            let i = MOUSEBUF.lock().get().unwrap();
            sti();
            if mouse_dec.decode(i).is_some() {
                (Screen::new()).boxfill8(Color::DarkCyan, 32, 0, 32 + 15 * 8 - 1, 16);
                let mut writer = ScreenWriter::new(Screen::new(), vga::Color::White, 32, 0);
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
                mouse.move_and_render(mouse_dec.x.get(), mouse_dec.y.get());
            }
        } else {
            stihlt();
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use vga::{Screen, ScreenWriter};
    let mut screen = Screen::new();
    screen.init();
    let mut writer = ScreenWriter::new(screen, vga::Color::LightRed, 0, 0);
    use core::fmt::Write;
    write!(writer, "[ERR] {:?}", info).unwrap();
    loop {
        asm::hlt()
    }
}
