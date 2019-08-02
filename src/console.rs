use core::str::from_utf8;

use crate::asm::{cli, sti};
use crate::descriptor_table::{SegmentDescriptor, ADR_GDT, AR_CODE32_ER, AR_DATA32_RW};
use crate::fifo::Fifo;
use crate::file::*;
use crate::keyboard::KEYBOARD_OFFSET;
use crate::memory::{MemMan, MEMMAN_ADDR};
use crate::mt::{TaskManager, TASK_MANAGER_ADDR};
use crate::sheet::{SheetManager, MAX_SHEETS};
use crate::timer::TIMER_MANAGER;
use crate::vga::{boxfill, draw_line, make_window, to_color, Color};
use crate::{write_with_bg, SHEET_MANAGER_ADDR};

pub const CONSOLE_CURSOR_ON: u32 = 2;
pub const CONSOLE_CURSOR_OFF: u32 = 3;
pub const CONSOLE_BACKSPACE: u32 = 8;
pub const CONSOLE_ENTER: u32 = 10;
const MIN_CURSOR_X: isize = 16;
const MIN_CURSOR_Y: isize = 28;
const MAX_CURSOR_X: isize = 8 + 240;
const MAX_CURSOR_Y: isize = 28 + 112;
pub const CONSOLE_ADDR: usize = 0x0fec;
pub const DS_BASE_ADDR: usize = 0xfe8;
const MAX_CMD: usize = 30;

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
    let ds_base = unsafe { *(DS_BASE_ADDR as *const usize) };
    let console_addr = unsafe { *(CONSOLE_ADDR as *const usize) };
    let console = unsafe { &mut *(console_addr as *mut Console) };
    let sheet_manager = unsafe { &mut *(console.sheet_manager_addr as *mut SheetManager) };
    let reg = &eax as *const i32 as usize + 4;
    if edx == 1 {
        // 1文字出力
        console.put_char(eax as u8, true);
    } else if edx == 2 {
        // 0がくるまで1文字ずつ出力
        let mut i = 0;
        loop {
            let chr = unsafe { *((ebx as usize + i as usize + ds_base) as *const u8) };
            if chr == 0 {
                break;
            }
            console.put_char(chr, true);
            i += 1;
        }
    } else if edx == 3 {
        // 指定した文字数出力
        console.put_string(ebx as usize, ecx as usize, 8);
    } else if edx == 4 {
        let task_manager = unsafe { &mut *(TASK_MANAGER_ADDR as *mut TaskManager) };
        let task_index = task_manager.now_index();
        let task = &task_manager.tasks_data[task_index];
        return unsafe { &(task.tss.esp0) } as *const i32 as usize;
    } else if edx == 5 {
        let sheet_index = sheet_manager.alloc().unwrap();
        {
            let new_sheet = &mut sheet_manager.sheets_data[sheet_index];
            new_sheet.set(ebx as usize + ds_base, esi, edi, to_color(eax as i8));
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
        sheet_manager.slide(sheet_index, 100, 50);
        sheet_manager.updown(sheet_index, Some(3));
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
        }
    }

    pub fn show_prompt(&mut self) {
        let cx = self.cursor_x;
        self.cursor_x = 8;
        self.put_char(b'>', false);
        self.cursor_x = cx;
    }

    pub fn put_char(&mut self, char_num: u8, move_cursor: bool) {
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
        if cmd_str == "mem" {
            self.cmd_mem(memtotal);
        } else if cmd_str == "clear" {
            self.cmd_clear();
        } else if cmd_str == "ls" {
            self.cmd_ls();
        } else if cmd_str == "cat" {
            self.cmd_cat(cmdline_strs, fat);
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

    pub fn cmd_cat<'a>(
        &mut self,
        cmdline_strs: impl Iterator<Item = &'a [u8]>,
        fat: &[u32; MAX_FAT],
    ) {
        let memman = unsafe { &mut *(MEMMAN_ADDR as *mut MemMan) };
        // ファイル名となるところを抽出
        let mut filename = cmdline_strs.skip_while(|strs| strs.len() == 0);
        let filename = filename.next();
        if filename.is_none() {
            self.display_error("File Not Found");
            return;
        }
        let filename = filename.unwrap();
        let target_finfo = search_file(filename);
        if let Some(finfo) = target_finfo {
            let content_addr = memman.alloc_4k(finfo.size).unwrap() as usize;
            finfo.load_file(content_addr, fat, ADR_DISKIMG + 0x003e00);
            self.put_string(content_addr, finfo.size as usize, 8);

            self.newline();
            memman.free_4k(content_addr as u32, finfo.size).unwrap();
        } else {
            self.display_error("File Not Found");
            return;
        }
    }

    fn display_error(&mut self, error_message: &'static str) {
        self.put_string(
            error_message.as_bytes().as_ptr() as usize,
            error_message.len(),
            8,
        );
        self.newline();
        self.newline();
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

        // kernel.ldを使ってリンクされたファイルのみ実行可能
        let mut app_eip = 0;
        let content_gdt = 1003;
        let app_gdt = 1004;
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
                let ptr = unsafe { &mut *(DS_BASE_ADDR as *mut usize) };
                *ptr = app_mem_addr;

                let gdt = unsafe { &mut *((ADR_GDT + content_gdt * 8) as *mut SegmentDescriptor) };
                *gdt = SegmentDescriptor::new(
                    finfo.size - 1,
                    content_addr as i32,
                    AR_CODE32_ER + 0x60,
                );
                let gdt = unsafe { &mut *((ADR_GDT + app_gdt * 8) as *mut SegmentDescriptor) };
                *gdt = SegmentDescriptor::new(
                    segment_size as u32 - 1,
                    app_mem_addr as i32,
                    AR_DATA32_RW + 0x60,
                );

                for i in 0..data_size {
                    let app_ptr = unsafe { &mut *((app_mem_addr + esp + i) as *mut u8) };
                    *app_ptr = unsafe { *((content_addr + content_data_addr + i) as *const u8) };
                }
            }
        }

        if app_eip > 0 {
            let task_manager = unsafe { &mut *(TASK_MANAGER_ADDR as *mut TaskManager) };
            let task_index = task_manager.now_index();

            let task = &task_manager.tasks_data[task_index];
            let esp0_addr = unsafe { &(task.tss.esp0) } as *const i32 as usize;
            unsafe {
                _start_app(
                    app_eip,
                    content_gdt * 8,
                    esp as i32,
                    app_gdt * 8,
                    esp0_addr as i32,
                );
            }
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

    pub fn put_string(&mut self, string_addr: usize, string_length: usize, initial_x: usize) {
        self.cursor_x = initial_x as isize;
        for x in 0..string_length {
            let chr = unsafe { *((string_addr + x as usize) as *const u8) };
            if chr == 0x09 {
                // タブ
                loop {
                    self.put_char(b' ', true);
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
                self.put_char(chr, true);
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

    let fifo = Fifo::new(128, Some(task_index));
    let fifo_addr = &fifo as *const Fifo as usize;
    {
        let mut task = &mut task_manager.tasks_data[task_index];
        task.fifo_addr = fifo_addr;
    }

    // コマンドを保持するための配列
    let mut cmdline: [u8; MAX_CMD] = [0; MAX_CMD];

    let sheet_manager_addr = unsafe { SHEET_MANAGER_ADDR };
    let sheet_manager = unsafe { &mut *(sheet_manager_addr as *mut SheetManager) };

    let mut console = Console::new(sheet_index, sheet_manager_addr);
    {
        let ptr = unsafe { &mut *(CONSOLE_ADDR as *mut usize) };
        *ptr = &console as *const Console as usize;
    }

    let timer_index = TIMER_MANAGER.lock().alloc().unwrap();
    TIMER_MANAGER.lock().init_timer(timer_index, fifo_addr, 1);
    TIMER_MANAGER.lock().set_time(timer_index, 50);
    let sheet = sheet_manager.sheets_data[sheet_index];

    let memman = unsafe { &mut *(MEMMAN_ADDR as *mut MemMan) };

    let fat_addr = memman.alloc_4k(4 * MAX_FAT as u32).unwrap();
    let fat = unsafe { &mut *(fat_addr as *mut [u32; (MAX_FAT)]) };
    read_fat(fat, unsafe {
        *((ADR_DISKIMG + 0x000200) as *const [u8; (MAX_FAT * 4)])
    });

    console.show_prompt();

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
                    console.cursor_c = if console.cursor_on {
                        Color::White
                    } else {
                        Color::Black
                    };
                } else {
                    TIMER_MANAGER.lock().init_timer(timer_index, fifo_addr, 1);
                    console.cursor_c = Color::Black;
                }
                TIMER_MANAGER.lock().set_time(timer_index, 50);
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
                        cmdline = [b' '; MAX_CMD];
                        // プロンプト表示
                        console.show_prompt();
                        console.cursor_x = 16;
                    } else {
                        if console.cursor_x < MAX_CURSOR_X {
                            cmdline[console.cursor_x as usize / 8 - 2] = key;
                            console.put_char(key, true);
                        }
                    }
                }
            } else if i == CONSOLE_CURSOR_ON {
                console.cursor_on = true;
            } else if i == CONSOLE_CURSOR_OFF {
                console.cursor_on = false;
            }
            if console.cursor_on {
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
    let console_addr = unsafe { *(CONSOLE_ADDR as *const usize) };
    let console = unsafe { &mut *(console_addr as *mut Console) };
    let sheet_manager_addr = unsafe { SHEET_MANAGER_ADDR };
    let sheet_manager = unsafe { &mut *(sheet_manager_addr as *mut SheetManager) };
    let sheet = sheet_manager.sheets_data[console.sheet_index];
    console.newline();
    console.put_string(message.as_ptr() as usize, message.len(), 8);
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
    let task_manager = unsafe { &mut *(TASK_MANAGER_ADDR as *mut TaskManager) };
    let task_index = task_manager.now_index();
    let task = &task_manager.tasks_data[task_index];
    return unsafe { &(task.tss.esp0) } as *const i32 as usize;
}
