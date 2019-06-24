use crate::vga::{Color, putblock};
use core::cell::{Cell, RefCell};

#[derive(Debug)]
pub struct MouseDec {
    pub buf: RefCell<[u8; 3]>,
    pub phase: Cell<MouseDecPhase>,
    pub x: Cell<i32>,
    pub y: Cell<i32>,
    pub btn: Cell<i32>,
}

#[derive(Debug, Clone, Copy)]
pub enum MouseDecPhase {
    START,
    FIRST,
    SECOND,
    THIRD,
}

impl MouseDec {
    pub fn new() -> MouseDec {
        MouseDec {
            buf: RefCell::new([0; 3]),
            phase: Cell::new(MouseDecPhase::START),
            x: Cell::new(0),
            y: Cell::new(0),
            btn: Cell::new(0),
        }
    }

    pub fn decode(&self, data: u8) -> Option<()> {
        use MouseDecPhase::*;
        match self.phase.get() {
            START => {
                if data == 0xfa {
                    self.phase.set(FIRST)
                }
                None
            }
            FIRST => {
                if (data & 0xc8) == 0x08 {
                    let mut buf = self.buf.borrow_mut();
                    buf[0] = data;
                    self.phase.set(SECOND);
                }
                None
            }
            SECOND => {
                let mut buf = self.buf.borrow_mut();
                buf[1] = data;
                self.phase.set(THIRD);
                None
            }
            THIRD => {
                let mut buf = self.buf.borrow_mut();
                buf[2] = data;
                self.phase.set(FIRST);
                self.btn.set((buf[0] & 0x07) as i32);
                self.x.set(buf[1] as i32);
                self.y.set(buf[2] as i32);
                if (buf[0] & 0x10) != 0 {
                    self.x.set((buf[1] as u32 | 0xffffff00) as i32);
                }
                if (buf[0] & 0x20) != 0 {
                    self.y.set((buf[2] as u32 | 0xffffff00) as i32);
                }
                self.y.set(-self.y.get());
                Some(())
            }
        }
    }
}

pub const MOUSE_CURSOR_WIDTH: usize = 16;
pub const MOUSE_CURSOR_HEIGHT: usize = 16;

#[derive(Debug)]
pub struct Mouse {
    cursor: [[Color; MOUSE_CURSOR_WIDTH]; MOUSE_CURSOR_HEIGHT],
    buf_mouse_addr: usize,
}

impl Mouse {
    pub fn new(buf_mouse_addr: usize) -> Mouse {
        let cursor_icon: [[u8; MOUSE_CURSOR_WIDTH]; MOUSE_CURSOR_HEIGHT] = [
            *b"**************..",
            *b"*OOOOOOOOOOO*...",
            *b"*OOOOOOOOOO*....",
            *b"*OOOOOOOOO*.....",
            *b"*OOOOOOOO*......",
            *b"*OOOOOOO*.......",
            *b"*OOOOOOO*.......",
            *b"*OOOOOOOO*......",
            *b"*OOOO**OOO*.....",
            *b"*OOO*..*OOO*....",
            *b"*OO*....*OOO*...",
            *b"*O*......*OOO*..",
            *b"**........*OOO*.",
            *b"*..........*OOO*",
            *b"............*OO*",
            *b".............***",
        ];

        let mut cursor: [[Color; MOUSE_CURSOR_WIDTH]; MOUSE_CURSOR_HEIGHT] =
            [[Color::DarkCyan; MOUSE_CURSOR_WIDTH]; MOUSE_CURSOR_HEIGHT];
        for y in 0..MOUSE_CURSOR_HEIGHT {
            for x in 0..MOUSE_CURSOR_WIDTH {
                match cursor_icon[y][x] {
                    b'*' => cursor[y][x] = Color::Black,
                    b'O' => cursor[y][x] = Color::White,
                    _ => (),
                }
            }
        }

        Mouse {
            cursor,
            buf_mouse_addr,
        }
    }

    pub fn render(&self) {
        putblock(
            self.buf_mouse_addr,
            MOUSE_CURSOR_WIDTH as isize,
            self.cursor,
            MOUSE_CURSOR_WIDTH as isize,
            MOUSE_CURSOR_HEIGHT as isize,
            0,
            0,
        );
    }
}
