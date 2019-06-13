#![no_std]
#![feature(asm)]
#![feature(start)]

use core::panic::PanicInfo;

mod asm;
mod descriptor_table;
mod fonts;
mod interrupt;
mod vga;

#[no_mangle]
#[start]
pub extern "C" fn haribote_os() {
    use asm::{hlt, sti};
    use vga::{Screen, ScreenWriter};

    descriptor_table::init();
    interrupt::init();
    sti();
    interrupt::allow_input();

    let mut screen = Screen::new();
    screen.init();
    let mut writer = ScreenWriter::new(screen, vga::Color::White, 0, 0);
    use core::fmt::Write;
    write!(writer, "ABC\n").unwrap();
    loop {
        hlt()
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
