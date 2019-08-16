use core::str::from_utf8;

use crate::asm::{cli, in8, out8, sti};
use crate::descriptor_table::{SegmentDescriptor, AR_CODE32_ER, AR_DATA32_RW};
use crate::fifo::Fifo;
use crate::file::*;
use crate::keyboard::KEYBOARD_OFFSET;
use crate::memory::{MemMan, MEMMAN_ADDR};
use crate::mt::{TaskManager, TASK_MANAGER_ADDR};
use crate::sheet::{SheetFlag, SheetManager, MAX_SHEETS};
use crate::timer::TIMER_MANAGER;
use crate::vga::{boxfill, draw_line, make_window, to_color, Color, SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::{
    open_console, open_console_task, write_with_bg, EXIT_CONSOLE, EXIT_OFFSET,
    EXIT_ONLY_CONSOLE_OFFSET, EXIT_TASK_OFFSET, SHEET_MANAGER_ADDR, TASK_A_FIFO_ADDR,
};

pub const CONSOLE_CURSOR_ON: u32 = 2;
pub const CONSOLE_CURSOR_OFF: u32 = 3;
pub const CONSOLE_BACKSPACE: u32 = 8;
pub const CONSOLE_ENTER: u32 = 10;
const MIN_CURSOR_X: isize = 16;
const MIN_CURSOR_Y: isize = 28;
const MAX_CURSOR_X: isize = 8 + 240;
const MAX_CURSOR_Y: isize = 28 + 112;
const MAX_CMD: usize = 30;

const MAX_FILE_HANDLER: usize = 8;

extern "C" {
    fn _start_app(eip: i32, cs: i32, esp: i32, ds: i32, tss_esp_addr: i32);
}

#[no_mangle]
pub extern "C" fn hrb_api(
    edi: i32,
    esi: i32,
    ebp: i32,
    esp: i32,
    ebx: i32,
    edx: i32,
    ecx: i32,
    eax: i32,
) -> usize {
    let memman = unsafe { &mut *(MEMMAN_ADDR as *mut MemMan) };
    let task_manager = unsafe { &mut *(TASK_MANAGER_ADDR as *mut TaskManager) };
    let task_index = task_manager.now_index();
    let task = task_manager.tasks_data[task_index];
    let ds_base = task.ds_base;
    let console = unsafe { &mut *(task.console_addr as *mut Console) };
    let sheet_manager = unsafe { &mut *(console.sheet_manager_addr as *mut SheetManager) };
    let reg = &eax as *const i32 as usize + 4;

    if edx == 1 {
        // 1文字出力
        console.put_string([eax as u8].as_ptr() as usize, 1, None);
    } else if edx == 2 {
        // 0まで出力
        let mut i = 0;
        loop {
            let chr = unsafe { *((ebx as usize + i as usize + ds_base) as *const u8) };
            if chr == 0 {
                break;
            }
            i += 1;
        }
        console.put_string(ebx as usize + ds_base, i, None);
    } else if edx == 3 {
        // 指定した文字数出力
        console.put_string(ebx as usize, ecx as usize, None);
    } else if edx == 4 {
        return unsafe { &(task.tss.esp0) } as *const i32 as usize;
    } else if edx == 5 {
        let sheet_index = sheet_manager.alloc().unwrap();
        {
            let mut new_sheet = &mut sheet_manager.sheets_data[sheet_index];
            new_sheet.set(ebx as usize + ds_base, esi, edi, to_color(eax as i8));
            new_sheet.task_index = task_index;
            new_sheet.from_app = true;
        }
        let title = unsafe { *((ecx as usize + ds_base) as *const [u8; 30]) };
        let mut t = title.iter().take_while(|t| **t != 0);
        let mut i = 0;
        for n in 0..30 {
            i = n;
            if t.next().is_none() {
                break;
            }
        }
        make_window(
            ebx as usize + ds_base,
            esi as isize,
            edi as isize,
            from_utf8(&title[0..i]).unwrap(),
            false,
        );
        sheet_manager.slide(
            sheet_index,
            ((*SCREEN_WIDTH as i32 - esi) / 2) & !3,
            (*SCREEN_HEIGHT as i32 - edi) / 2,
        );
        sheet_manager.updown(sheet_index, sheet_manager.z_max);
        let reg_eax = unsafe { &mut *((reg + 7 * 4) as *mut i32) };
        *reg_eax = sheet_index as i32;
    } else if edx == 6 {
        let mut sheet_index = ebx as usize;
        let mut refresh = true;
        if sheet_index >= MAX_SHEETS {
            refresh = false;
            sheet_index -= MAX_SHEETS;
        }
        let sheet = sheet_manager.sheets_data[sheet_index];
        let string = unsafe { *((ebp as usize + ds_base) as *const [u8; 30]) };
        use crate::vga::ScreenWriter;
        use core::fmt::Write;
        let mut writer = ScreenWriter::new(
            Some(sheet.buf_addr),
            to_color(eax as i8).unwrap(),
            esi as usize,
            edi as usize,
            sheet.width as usize,
            sheet.height as usize,
        );
        write!(writer, "{}", from_utf8(&string[0..(ecx as usize)]).unwrap()).unwrap();
        if refresh {
            sheet_manager.refresh(sheet_index, esi, edi, esi + ecx * 8, edi + 16);
        }
    } else if edx == 7 {
        let mut sheet_index = ebx as usize;
        let mut refresh = true;
        if sheet_index >= MAX_SHEETS {
            refresh = false;
            sheet_index -= MAX_SHEETS;
        }
        let sheet = sheet_manager.sheets_data[sheet_index];
        boxfill(
            sheet.buf_addr,
            sheet.width as isize,
            to_color(ebp as i8).unwrap(),
            eax as isize,
            ecx as isize,
            esi as isize,
            edi as isize,
        );
        if refresh {
            sheet_manager.refresh(sheet_index, eax, ecx, esi + 1, edi + 1);
        }
    } else if edx == 8 {
        let memman = unsafe { &mut *((ebx as usize + ds_base) as *mut MemMan) };
        *memman = MemMan::new();
        let bytes = ecx as u32 & 0xfffffff0;
        memman.free(eax as u32, bytes).unwrap();
    } else if edx == 9 {
        let bytes = (ecx as u32 + 0x0f) & 0xfffffff0;
        let reg_eax = unsafe { &mut *((reg + 7 * 4) as *mut u32) };
        let memman = unsafe { &mut *((ebx as usize + ds_base) as *mut MemMan) };
        *reg_eax = memman.alloc(bytes).unwrap();
    } else if edx == 10 {
        let bytes = (ecx as u32 + 0x0f) & 0xfffffff0;
        let memman = unsafe { &mut *((ebx as usize + ds_base) as *mut MemMan) };
        memman.free(eax as u32, bytes).unwrap();
    } else if edx == 11 {
        let mut sheet_index = ebx as usize;
        let mut refresh = true;
        if sheet_index >= MAX_SHEETS {
            refresh = false;
            sheet_index -= MAX_SHEETS;
        }

        let sheet = sheet_manager.sheets_data[sheet_index];
        let ptr = unsafe {
            &mut *((sheet.buf_addr + sheet.width as usize * edi as usize + esi as usize) as *mut u8)
        };
        *ptr = eax as u8;
        if refresh {
            sheet_manager.refresh(sheet_index, esi, edi, esi + 1, edi + 1);
        }
    } else if edx == 12 {
        let sheet_index = ebx as usize;
        sheet_manager.refresh(sheet_index, eax, ecx, esi, edi);
    } else if edx == 13 {
        let mut sheet_index = ebx as usize;
        let mut refresh = true;
        if sheet_index >= MAX_SHEETS {
            refresh = false;
            sheet_index -= MAX_SHEETS;
        }
        let sheet = sheet_manager.sheets_data[sheet_index];
        draw_line(sheet.buf_addr, sheet.width, eax, ecx, esi, edi, ebp);
        if refresh {
            sheet_manager.refresh(sheet_index, eax, ecx, esi + 1, edi + 1);
        }
    } else if edx == 14 {
        let sheet_index = ebx as usize;
        sheet_manager.free(sheet_index);
    } else if edx == 15 {
        loop {
            cli();
            let fifo = { unsafe { &*(task.fifo_addr as *const Fifo) } };
            if fifo.status() == 0 {
                if eax != 0 {
                    task_manager.sleep(task_index);
                } else {
                    sti();
                    let reg_eax = unsafe { &mut *((reg + 7 * 4) as *mut i32) };
                    *reg_eax = -1;
                    return 0;
                }
            }
            let i = fifo.get().unwrap();
            sti();
            if i <= 1 {
                TIMER_MANAGER
                    .lock()
                    .init_timer(console.timer_index, task.fifo_addr, 1);
                TIMER_MANAGER.lock().set_time(console.timer_index, 50);
            } else if i == 2 {
                console.cursor_c = Color::White
            } else if i == 3 {
                console.cursor_c = Color::Black
            } else if i == 4 {
                TIMER_MANAGER.lock().cancel(console.timer_index);
                cli();
                let task_a_fifo_addr = unsafe { *(TASK_A_FIFO_ADDR as *const usize) };
                let task_a_fifo = unsafe { &mut *(task_a_fifo_addr as *mut Fifo) };
                task_a_fifo
                    .put(console.sheet_index as u32 + EXIT_ONLY_CONSOLE_OFFSET as u32)
                    .unwrap();
                console.sheet_index = 0;
                sti();
            } else if 256 <= i {
                let reg_eax = unsafe { &mut *((reg + 7 * 4) as *mut u32) };
                *reg_eax = i - 256;
                return 0;
            }
        }
    } else if edx == 16 {
        let reg_eax = unsafe { &mut *((reg + 7 * 4) as *mut usize) };
        {
            let mut timer_manager = TIMER_MANAGER.lock();
            let timer_index = timer_manager.alloc().unwrap();
            let mut timer = &mut timer_manager.timers_data[timer_index];
            timer.from_app = true;
            *reg_eax = timer_index;
        }
    } else if edx == 17 {
        TIMER_MANAGER
            .lock()
            .init_timer(ebx as usize, task.fifo_addr, eax + 256);
    } else if edx == 18 {
        TIMER_MANAGER.lock().set_time(ebx as usize, eax as u32);
    } else if edx == 19 {
        TIMER_MANAGER.lock().free(ebx as usize);
    } else if edx == 20 {
        if eax == 0 {
            let i = in8(0x61);
            out8(0x61, i & 0x0d);
        } else {
            let i = 1193180000 / eax;
            out8(0x43, 0xb6);
            out8(0x42, i as u8);
            out8(0x42, (i >> 8) as u8);
            let i = in8(0x61);
            out8(0x61, (i | 0x03) & 0x0f);
        }
    } else if edx == 21 {
        let fhandlers =
            unsafe { &mut *(task.file_handler_addr as *mut [FileHandler; MAX_FILE_HANDLER]) };
        let mut fhandler: Option<&mut FileHandler> = None;
        for i in 0..MAX_FILE_HANDLER {
            if fhandlers[i].buf_addr == 0 {
                fhandler = Some(&mut fhandlers[i]);
                break;
            }
        }
        let mut i = 0;
        loop {
            let chr = unsafe { *((ebx as usize + i as usize + ds_base) as *const u8) };
            if chr == 0 {
                break;
            }
            i += 1;
        }
        let filename = unsafe { *((ebx as usize + ds_base) as *const [u8; 30]) };
        let fat = unsafe { *(task.fat_addr as *const [u32; MAX_FAT]) };
        if let Some(fhandler) = fhandler {
            let finfo = search_file(&filename[0..i]);
            if let Some(finfo) = finfo {
                let reg_eax = unsafe { &mut *((reg + 7 * 4) as *mut usize) };
                *reg_eax = fhandler as *const FileHandler as usize;
                fhandler.buf_addr = memman.alloc_4k(finfo.size).unwrap() as usize;
                fhandler.size = finfo.size as i32;
                fhandler.pos = 0;
                finfo.load_file(fhandler.buf_addr, &fat, ADR_DISKIMG + 0x003e00);
            }
        }
    } else if edx == 22 {
        let mut fh = unsafe { &mut *(eax as *mut FileHandler) };
        memman.free_4k(fh.buf_addr as u32, fh.size as u32).unwrap();
        fh.buf_addr = 0;
    } else if edx == 23 {
        let mut fh = unsafe { &mut *(eax as *mut FileHandler) };
        if ecx == 0 {
            fh.pos = ebx;
        } else if ecx == 1 {
            fh.pos += ebx;
        } else if ecx == 2 {
            fh.pos = fh.size + ebx;
        }
        if fh.pos < 0 {
            fh.pos = 0;
        }
        if fh.pos > fh.size {
            fh.pos = fh.size;
        }
    } else if edx == 24 {
        let fh = unsafe { &mut *(eax as *mut FileHandler) };
        let reg_eax = unsafe { &mut *((reg + 7 * 4) as *mut i32) };
        if ecx == 0 {
            *reg_eax = fh.size;
        } else if ecx == 1 {
            *reg_eax = fh.pos;
        } else if ecx == 2 {
            *reg_eax = fh.pos - fh.size;
        }
    } else if edx == 25 {
        let mut fh = unsafe { &mut *(eax as *mut FileHandler) };
        let mut size: usize = 0;
        for i in 0..(ecx as usize) {
            if fh.pos == fh.size {
                break;
            }
            let ptr = unsafe { &mut *((ebx as usize + ds_base + i) as *mut u8) };
            let buf = unsafe { &*((fh.buf_addr + fh.pos as usize) as *const u8) };
            *ptr = *buf;
            fh.pos += 1;
            size = i + 1;
        }
        let reg_eax = unsafe { &mut *((reg + 7 * 4) as *mut usize) };
        *reg_eax = size;
    } else if edx == 26 {
        let mut i = 0;
        loop {
            let ptr = unsafe { &mut *((ebx as usize + ds_base + i) as *mut u8) };
            let buf = unsafe { &*((task.cmdline_addr + i) as *const u8) };
            *ptr = *buf;
            if *buf == 0 {
                break;
            }
            i += 1;
        }
        let reg_eax = unsafe { &mut *((reg + 7 * 4) as *mut usize) };
        *reg_eax = i;
    }
    0
}

#[repr(C, packed)]
pub struct Console {
    pub cursor_x: isize,
    pub cursor_y: isize,
    pub cursor_c: Color,
    pub cursor_on: bool,
    pub sheet_index: usize,
    pub sheet_manager_addr: usize,
    pub timer_index: usize,
}

impl Console {
    pub fn new(sheet_index: usize, sheet_manager_addr: usize) -> Console {
        Console {
            cursor_x: MIN_CURSOR_X,
            cursor_y: MIN_CURSOR_Y,
            cursor_c: Color::Black,
            cursor_on: false,
            sheet_index,
            sheet_manager_addr,
            timer_index: 0,
        }
    }

    pub fn show_prompt(&mut self) {
        let cx = self.cursor_x;
        self.cursor_x = 8;
        self.put_char(b'>', false);
        self.cursor_x = cx;
    }

    pub fn put_char(&mut self, char_num: u8, move_cursor: bool) {
        if self.sheet_index != 0 {
            let sheet_manager = unsafe { &mut *(self.sheet_manager_addr as *mut SheetManager) };

            let sheet = sheet_manager.sheets_data[self.sheet_index];
            write_with_bg!(
                sheet_manager,
                self.sheet_index,
                sheet.width,
                sheet.height,
                self.cursor_x,
                self.cursor_y,
                Color::White,
                Color::Black,
                1,
                "{}",
                char_num as char,
            );
        }
        if move_cursor {
            self.cursor_x += 8;
        }
    }

    pub fn newline(&mut self) {
        let sheet_manager = unsafe { &mut *(self.sheet_manager_addr as *mut SheetManager) };
        let sheet = sheet_manager.sheets_data[self.sheet_index];

        if self.cursor_y < MAX_CURSOR_Y {
            self.cursor_y += 16;
        } else {
            if self.sheet_index == 0 {
                return;
            }
            for y in MIN_CURSOR_Y..MAX_CURSOR_Y {
                for x in (MIN_CURSOR_X - 8)..MAX_CURSOR_X {
                    let x = x as usize;
                    let y = y as usize;
                    // 下の画素をコピーする
                    let ptr = unsafe {
                        &mut *((sheet.buf_addr + x + y * sheet.width as usize) as *mut u8)
                    };
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
                    let ptr = unsafe {
                        &mut *((sheet.buf_addr + x + y * sheet.width as usize) as *mut u8)
                    };
                    *ptr = Color::Black as u8;
                }
            }

            sheet_manager.refresh(
                self.sheet_index,
                (MIN_CURSOR_X - 8) as i32,
                MIN_CURSOR_Y as i32,
                MAX_CURSOR_X as i32,
                (MAX_CURSOR_Y + 16) as i32,
            );
        }
    }

    fn run_cmd(&mut self, cmdline: [u8; MAX_CMD], memtotal: usize, fat: &[u32; MAX_FAT]) {
        self.cursor_x = 8;
        let cmdline_strs = cmdline.split(|s| *s == 0 || *s == b' ');
        let mut cmdline_strs = cmdline_strs.skip_while(|cmd| cmd.len() == 0);
        let cmd = cmdline_strs.next();
        if cmd.is_none() {
            self.display_error("Bad Command");
            return;
        }
        let cmd = cmd.unwrap();
        let cmd_str = from_utf8(&cmd).unwrap();
        if cmd_str == "mem" && self.sheet_index != 0 {
            self.cmd_mem(memtotal);
        } else if cmd_str == "clear" && self.sheet_index != 0 {
            self.cmd_clear();
        } else if cmd_str == "ls" && self.sheet_index != 0 {
            self.cmd_ls();
        } else if cmd_str == "start" {
            self.cmd_start(cmdline_strs, memtotal as u32);
        } else if cmd_str == "ncst" {
            self.cmd_ncst(cmdline_strs, memtotal as u32);
        } else if cmd_str == "exit" {
            self.cmd_exit(fat);
        } else {
            self.cmd_app(&cmd, fat);
        }
    }

    pub fn cmd_mem(&mut self, memtotal: usize) {
        let sheet_manager = unsafe { &mut *(self.sheet_manager_addr as *mut SheetManager) };
        let sheet = sheet_manager.sheets_data[self.sheet_index];
        let memman = unsafe { &mut *(MEMMAN_ADDR as *mut MemMan) };
        write_with_bg!(
            sheet_manager,
            self.sheet_index,
            sheet.width,
            sheet.height,
            8,
            self.cursor_y,
            Color::White,
            Color::Black,
            30,
            "total   {}MB",
            memtotal / (1024 * 1024)
        );
        self.newline();
        write_with_bg!(
            sheet_manager,
            self.sheet_index,
            sheet.width,
            sheet.height,
            8,
            self.cursor_y,
            Color::White,
            Color::Black,
            30,
            "free {}KB",
            memman.total() / 1024
        );
        self.newline();
        self.newline();
    }

    pub fn cmd_clear(&mut self) {
        let sheet_manager = unsafe { &mut *(self.sheet_manager_addr as *mut SheetManager) };
        let sheet = sheet_manager.sheets_data[self.sheet_index];
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
            self.sheet_index,
            (MIN_CURSOR_X - 8) as i32,
            MIN_CURSOR_Y as i32,
            MAX_CURSOR_X as i32,
            (MAX_CURSOR_Y + 16) as i32,
        );
        self.cursor_y = MIN_CURSOR_Y;
    }

    pub fn cmd_ls(&mut self) {
        let sheet_manager = unsafe { &mut *(self.sheet_manager_addr as *mut SheetManager) };
        let sheet = sheet_manager.sheets_data[self.sheet_index];
        for findex in 0..MAX_FILE_INFO {
            let finfo = unsafe {
                *((ADR_DISKIMG + ADR_FILE_OFFSET + findex * core::mem::size_of::<FileInfo>())
                    as *const FileInfo)
            };
            if finfo.name[0] == 0x00 {
                break;
            }
            let size = finfo.size;
            if finfo.name[0] != 0xe5 {
                if (finfo.ftype & 0x18) == 0 {
                    write_with_bg!(
                        sheet_manager,
                        self.sheet_index,
                        sheet.width,
                        sheet.height,
                        8,
                        self.cursor_y,
                        Color::White,
                        Color::Black,
                        30,
                        "{:>8}.{:>3}   {:>7}",
                        from_utf8(&finfo.name).unwrap(),
                        from_utf8(&finfo.ext).unwrap(),
                        size
                    );
                    self.newline();
                }
            }
        }
        self.newline();
    }

    fn display_error(&mut self, error_message: &'static str) {
        if self.sheet_index != 0 {
            self.put_string(
                error_message.as_bytes().as_ptr() as usize,
                error_message.len(),
                None,
            );
        }
        self.newline();
        self.newline();
    }

    pub fn cmd_start<'a>(&mut self, cmdline_strs: impl Iterator<Item = &'a [u8]>, memtotal: u32) {
        let mut cmd = cmdline_strs.skip_while(|strs| strs.len() == 0);
        let cmd = cmd.next();
        if cmd.is_none() {
            self.display_error("Command Not Found");
            return;
        }
        let cmd = cmd.unwrap();
        let sheet_manager = unsafe { &mut *(self.sheet_manager_addr as *mut SheetManager) };
        let task_manager = unsafe { &mut *(TASK_MANAGER_ADDR as *mut TaskManager) };
        let sheet_index = open_console(sheet_manager, task_manager, memtotal);
        let task = &task_manager.tasks_data[sheet_manager.sheets_data[sheet_index].task_index];
        let fifo = unsafe { &mut *(task.fifo_addr as *mut Fifo) };
        sheet_manager.slide(sheet_index, 32, 4);
        sheet_manager.updown(sheet_index, sheet_manager.z_max);
        for ci in 0..cmd.len() {
            fifo.put(cmd[ci] as u32 + 256).unwrap();
        }
        fifo.put(10 + 256).unwrap(); // Enter
        self.newline();
    }

    pub fn cmd_ncst<'a>(&mut self, cmdline_strs: impl Iterator<Item = &'a [u8]>, memtotal: u32) {
        let task_manager = unsafe { &mut *(TASK_MANAGER_ADDR as *mut TaskManager) };
        let mut cmd = cmdline_strs.skip_while(|strs| strs.len() == 0);
        let cmd = cmd.next();
        if cmd.is_none() {
            self.display_error("Command Not Found");
            return;
        }
        let cmd = cmd.unwrap();
        let task_index = open_console_task(task_manager, 0, memtotal);
        let task = &task_manager.tasks_data[task_index];
        let fifo = unsafe { &mut *(task.fifo_addr as *mut Fifo) };
        for ci in 0..cmd.len() {
            fifo.put(cmd[ci] as u32 + 256).unwrap();
        }
        fifo.put(10 + 256).unwrap(); // Enter
        self.newline();
    }

    pub fn cmd_exit(&mut self, fat: &[u32; MAX_FAT]) {
        let memman = unsafe { &mut *(MEMMAN_ADDR as *mut MemMan) };
        let task_a_fifo_addr = unsafe { *(TASK_A_FIFO_ADDR as *const usize) };
        let task_a_fifo = unsafe { &mut *(task_a_fifo_addr as *mut Fifo) };
        let task_manager = unsafe { &mut *(TASK_MANAGER_ADDR as *mut TaskManager) };
        let task_index = task_manager.now_index();
        TIMER_MANAGER.lock().cancel(self.timer_index);
        memman.free_4k(fat.as_ptr() as u32, 4 * 2880).unwrap();
        cli();
        if self.sheet_index != 0 {
            task_a_fifo
                .put(self.sheet_index as u32 + EXIT_OFFSET as u32)
                .unwrap();
        } else {
            task_a_fifo
                .put(task_index as u32 + EXIT_TASK_OFFSET as u32)
                .unwrap();
        }
        sti();
        loop {
            task_manager.sleep(task_index);
        }
    }

    pub fn cmd_app<'a>(&mut self, filename: &'a [u8], fat: &[u32; MAX_FAT]) {
        let memman = unsafe { &mut *(MEMMAN_ADDR as *mut MemMan) };
        let mut finfo = search_file(filename);
        if finfo.is_none() && filename.len() > 1 && filename[filename.len() - 2] != b'.' {
            let mut filename_ext = [b' '; MAX_CMD + 4];
            let filename_ext = &mut filename_ext[0..(filename.len() + 4)];
            filename_ext[..filename.len()].copy_from_slice(filename);
            filename_ext[filename.len()] = b'.';
            filename_ext[filename.len() + 1] = b'h';
            filename_ext[filename.len() + 2] = b'r';
            filename_ext[filename.len() + 3] = b'b';
            finfo = search_file(filename_ext);
        }
        if finfo.is_none() {
            self.display_error("Bad Command");
            return;
        }
        let finfo = finfo.unwrap();
        let content_addr = memman.alloc_4k(finfo.size).unwrap() as usize;
        finfo.load_file(content_addr, fat, ADR_DISKIMG + 0x003e00);

        let task_manager = unsafe { &mut *(TASK_MANAGER_ADDR as *mut TaskManager) };
        let task_index = task_manager.now_index();

        // kernel.ldを使ってリンクされたファイルのみ実行可能
        let mut app_eip = 0;
        let mut app_mem_addr = 0;
        let mut segment_size = 0;
        let mut esp = 0;
        if finfo.size >= 8 {
            // 4から7バイト目で判定
            let bytes = unsafe { *((content_addr + 4) as *const [u8; 4]) };
            if bytes == *b"Hari" {
                app_eip = 0x1b;
                segment_size = unsafe { *((content_addr + 0x0000) as *const usize) };
                esp = unsafe { *((content_addr + 0x000c) as *const usize) };
                let data_size = unsafe { *((content_addr + 0x0010) as *const usize) };
                let content_data_addr = unsafe { *((content_addr + 0x0014) as *const usize) };

                app_mem_addr = memman.alloc_4k(segment_size as u32).unwrap() as usize;
                {
                    let mut task = &mut task_manager.tasks_data[task_index];
                    task.ds_base = app_mem_addr;
                    task.ldt[0] = SegmentDescriptor::new(
                        finfo.size - 1,
                        content_addr as i32,
                        AR_CODE32_ER + 0x60,
                    );
                    task.ldt[1] = SegmentDescriptor::new(
                        segment_size as u32 - 1,
                        app_mem_addr as i32,
                        AR_DATA32_RW + 0x60,
                    );
                }

                for i in 0..data_size {
                    let app_ptr = unsafe { &mut *((app_mem_addr + esp + i) as *mut u8) };
                    *app_ptr = unsafe { *((content_addr + content_data_addr + i) as *const u8) };
                }
            }
        }

        if app_eip > 0 {
            let task = &task_manager.tasks_data[task_index];
            let esp0_addr = unsafe { &(task.tss.esp0) } as *const i32 as usize;
            unsafe {
                _start_app(app_eip, 0 * 8 + 4, esp as i32, 1 * 8 + 4, esp0_addr as i32);
            }
            {
                let sheet_manager = unsafe { &mut *(self.sheet_manager_addr as *mut SheetManager) };
                for i in 0..MAX_SHEETS {
                    let sheet = sheet_manager.sheets_data[i];
                    if sheet.task_index == task_index
                        && sheet.flag != SheetFlag::AVAILABLE
                        && sheet.from_app
                    {
                        sheet_manager.free(i);
                    }
                }
            }
            // クローズしていないファイルをクローズ
            let fhandlers =
                unsafe { &mut *(task.file_handler_addr as *mut [FileHandler; MAX_FILE_HANDLER]) };
            for i in 0..8 {
                let mut fhandler = &mut fhandlers[i];
                if fhandler.buf_addr != 0 {
                    memman
                        .free_4k(fhandler.buf_addr as u32, fhandler.size as u32)
                        .unwrap();
                    fhandler.buf_addr = 0;
                }
            }
            TIMER_MANAGER.lock().cancel_all(task.fifo_addr);
            self.newline();
        } else {
            self.display_error("Bad Format");
        }
        memman.free_4k(content_addr as u32, finfo.size).unwrap();
        if app_mem_addr > 0 {
            memman
                .free_4k(app_mem_addr as u32, segment_size as u32)
                .unwrap();
        }
    }

    pub fn put_string(
        &mut self,
        string_addr: usize,
        string_length: usize,
        initial_x: Option<usize>,
    ) {
        if initial_x.is_some() {
            self.cursor_x = initial_x.unwrap() as isize
        }
        for x in 0..string_length {
            let chr = unsafe { *((string_addr + x as usize) as *const u8) };
            if chr == 0x09 {
                // タブ
                loop {
                    if self.sheet_index != 0 {
                        self.put_char(b' ', true);
                    }
                    if self.cursor_x == MAX_CURSOR_X {
                        self.cursor_x = 8;
                        self.newline();
                    }
                    if (self.cursor_x - 8) & 0x1f == 0 {
                        // 32で割り切れたらbreak
                        break;
                    }
                }
            } else if chr == 0x0a {
                // 改行
                self.cursor_x = 8;
                self.newline();
            } else if chr == 0x0d {
                // 復帰
                // 何もしない
            } else {
                if self.sheet_index != 0 {
                    self.put_char(chr, true);
                }
                if self.cursor_x == MAX_CURSOR_X {
                    self.cursor_x = 8;
                    self.newline();
                }
            }
        }
    }
}

pub extern "C" fn console_task(sheet_index: usize, memtotal: usize) {
    let task_manager = unsafe { &mut *(TASK_MANAGER_ADDR as *mut TaskManager) };
    let task_index = task_manager.now_index();

    let memman = unsafe { &mut *(MEMMAN_ADDR as *mut MemMan) };

    // コマンドを保持するための配列
    let mut cmdline: [u8; MAX_CMD] = [0; MAX_CMD];

    let sheet_manager_addr = unsafe { SHEET_MANAGER_ADDR };
    let sheet_manager = unsafe { &mut *(sheet_manager_addr as *mut SheetManager) };

    let fat_addr = memman.alloc_4k(4 * MAX_FAT as u32).unwrap();
    let fat = unsafe { &mut *(fat_addr as *mut [u32; (MAX_FAT)]) };
    read_fat(fat, unsafe {
        *((ADR_DISKIMG + 0x000200) as *const [u8; (MAX_FAT * 4)])
    });

    let mut console = Console::new(sheet_index, sheet_manager_addr);
    let fifo_addr: usize;
    let fhandlers: [FileHandler; MAX_FILE_HANDLER] = [FileHandler::new(); MAX_FILE_HANDLER];
    {
        let mut task = &mut task_manager.tasks_data[task_index];
        task.console_addr = &console as *const Console as usize;
        fifo_addr = task.fifo_addr;
        task.file_handler_addr = fhandlers.as_ptr() as usize;
        task.fat_addr = fat_addr as usize;
        task.cmdline_addr = cmdline.as_ptr() as usize;
    }
    let fifo = unsafe { &*(fifo_addr as *const Fifo) };

    if sheet_index != 0 {
        console.timer_index = TIMER_MANAGER.lock().alloc().unwrap();
        TIMER_MANAGER
            .lock()
            .init_timer(console.timer_index, fifo_addr, 1);
        TIMER_MANAGER.lock().set_time(console.timer_index, 50);
    }
    let sheet = sheet_manager.sheets_data[sheet_index];

    if sheet_index != 0 {
        console.show_prompt();
    }

    loop {
        cli();
        if fifo.status() == 0 {
            task_manager.sleep(task_index);
            sti();
        } else {
            let i = fifo.get().unwrap();
            sti();
            if i <= 1 && console.sheet_index != 0 {
                if i != 0 {
                    TIMER_MANAGER
                        .lock()
                        .init_timer(console.timer_index, fifo_addr, 0);
                    console.cursor_c = if console.cursor_on {
                        Color::White
                    } else {
                        Color::Black
                    };
                } else {
                    TIMER_MANAGER
                        .lock()
                        .init_timer(console.timer_index, fifo_addr, 1);
                    console.cursor_c = Color::Black;
                }
                TIMER_MANAGER.lock().set_time(console.timer_index, 50);
            } else if KEYBOARD_OFFSET <= i && i <= 511 {
                let key = (i - KEYBOARD_OFFSET) as u8;
                if key != 0 {
                    // バックスペース
                    if key == CONSOLE_BACKSPACE as u8 {
                        if console.cursor_x > MIN_CURSOR_X {
                            console.put_char(b' ', false);
                            cmdline[console.cursor_x as usize / 8 - 2] = b' ';
                            console.cursor_x -= 8;
                        }
                    } else if key == CONSOLE_ENTER as u8 {
                        console.put_char(b' ', false);
                        console.newline();
                        console.run_cmd(cmdline, memtotal, fat);
                        if console.sheet_index == 0 {
                            console.cmd_exit(fat);
                        }
                        cmdline = [b' '; MAX_CMD];
                        // プロンプト表示
                        if sheet_index != 0 {
                            console.show_prompt();
                        }
                        console.cursor_x = 16;
                    } else {
                        if console.cursor_x < MAX_CURSOR_X {
                            cmdline[console.cursor_x as usize / 8 - 2] = key;
                            console.put_char(key, true);
                        }
                    }
                }
            } else if i == CONSOLE_CURSOR_ON {
                console.cursor_c = Color::White;
                console.cursor_on = true;
            } else if i == CONSOLE_CURSOR_OFF {
                if console.sheet_index != 0 {
                    let sheet = sheet_manager.sheets_data[console.sheet_index];
                    boxfill(
                        sheet.buf_addr,
                        sheet.width as isize,
                        Color::Black,
                        console.cursor_x,
                        console.cursor_y,
                        console.cursor_x + 7,
                        console.cursor_y + 15,
                    );
                }
                console.cursor_on = false;
            } else if i == EXIT_CONSOLE {
                console.cmd_exit(fat);
            }
            if console.sheet_index != 0 && console.cursor_on {
                boxfill(
                    sheet.buf_addr,
                    sheet.width as isize,
                    console.cursor_c,
                    console.cursor_x,
                    console.cursor_y,
                    console.cursor_x + 7,
                    console.cursor_y + 15,
                );
                sheet_manager.refresh(
                    console.sheet_index,
                    console.cursor_x as i32,
                    console.cursor_y as i32,
                    console.cursor_x as i32 + 8,
                    console.cursor_y as i32 + 16,
                );
            }
        }
    }
}

fn search_file(filename: &[u8]) -> Option<FileInfo> {
    let mut target_finfo = None;
    // 拡張子の前後でわける
    let mut filename = filename.split(|c| *c == b'.');
    let basename = filename.next();
    let extname = filename.next();
    let mut b = [b' '; 8];
    let mut e = [b' '; 3];
    if let Some(basename) = basename {
        for fi in 0..b.len() {
            if basename.len() <= fi {
                break;
            }
            if b'a' <= basename[fi] && basename[fi] <= b'z' {
                // 小文字は大文字で正規化しておく
                b[fi] = basename[fi] - 0x20;
            } else {
                b[fi] = basename[fi];
            }
        }
    } else {
        return None;
    }
    if let Some(extname) = extname {
        for fi in 0..e.len() {
            if extname.len() <= fi {
                break;
            }
            if b'a' <= extname[fi] && extname[fi] <= b'z' {
                e[fi] = extname[fi] - 0x20;
            } else {
                e[fi] = extname[fi];
            }
        }
    }
    for findex in 0..MAX_FILE_INFO {
        let finfo = unsafe {
            *((ADR_DISKIMG + ADR_FILE_OFFSET + findex * core::mem::size_of::<FileInfo>())
                as *const FileInfo)
        };
        if finfo.name[0] == 0x00 {
            break;
        }
        if finfo.name[0] != 0xe5 {
            if (finfo.ftype & 0x18) == 0 {
                let mut filename_equal = true;
                for y in 0..finfo.name.len() {
                    if finfo.name[y] != b[y] {
                        filename_equal = false;
                        break;
                    }
                }
                for y in 0..finfo.ext.len() {
                    if finfo.ext[y] != e[y] {
                        filename_equal = false;
                        break;
                    }
                }
                if filename_equal {
                    target_finfo = Some(finfo);
                    break;
                }
            }
        }
    }
    target_finfo
}

pub extern "C" fn inthandler0c(esp: *const usize) -> usize {
    exception_handler(b"INT 0C: \n Stack Exception.\n", esp)
}

pub extern "C" fn inthandler0d(esp: *const usize) -> usize {
    exception_handler(b"INT 0D: \n General Protected Exception.\n", esp)
}

pub extern "C" fn exception_handler(message: &[u8], esp: *const usize) -> usize {
    let task_manager = unsafe { &mut *(TASK_MANAGER_ADDR as *mut TaskManager) };
    let task_index = task_manager.now_index();
    let task = &task_manager.tasks_data[task_index];
    let console = unsafe { &mut *(task.console_addr as *mut Console) };
    let sheet_manager_addr = unsafe { SHEET_MANAGER_ADDR };
    let sheet_manager = unsafe { &mut *(sheet_manager_addr as *mut SheetManager) };
    let sheet = sheet_manager.sheets_data[console.sheet_index];
    console.newline();
    console.put_string(message.as_ptr() as usize, message.len(), Some(8));
    write_with_bg!(
        sheet_manager,
        console.sheet_index,
        sheet.width,
        sheet.height,
        8,
        console.cursor_y,
        Color::White,
        Color::Black,
        30,
        "EIP = {:>08X}",
        unsafe { *((esp as usize + 11) as *const usize) }
    );
    console.newline();
    return unsafe { &(task.tss.esp0) } as *const i32 as usize;
}
