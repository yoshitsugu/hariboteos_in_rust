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
mod vga;

#[no_mangle]
#[start]
pub extern "C" fn haribote_os() {
    use asm::{cli, sti, stihlt};
    use core::fmt::Write;
    use interrupt::{enable_mouse, KEYBUF, MOUSEBUF};
    use vga::{Color, Screen, ScreenWriter};

    descriptor_table::init();
    interrupt::init();
    sti();
    interrupt::allow_input();
    let mut screen = Screen::new();
    screen.init();
    enable_mouse();
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
            (Screen::new()).boxfill8(Color::DarkCyan, 32, 0, 48, 16);
            let mut writer = ScreenWriter::new(Screen::new(), vga::Color::White, 32, 0);
            write!(writer, "{:x}", i).unwrap();
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
