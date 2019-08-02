use core::cmp::{max, min};

use crate::vga::{Color, SCREEN_HEIGHT, SCREEN_WIDTH, VRAM_ADDR};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetFlag {
    AVAILABLE,
    USED,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sheet {
    pub buf_addr: usize,
    pub width: i32,
    pub height: i32,
    pub x: i32,
    pub y: i32,
    pub transparent: Option<Color>,
    pub z: Option<usize>, // 重ねあわせたときの高さ
    pub flag: SheetFlag,
}

impl Sheet {
    pub fn new() -> Sheet {
        Sheet {
            buf_addr: 0,
            width: 0,
            height: 0,
            x: 0,
            y: 0,
            transparent: None,
            z: None,
            flag: SheetFlag::AVAILABLE,
        }
    }

    pub fn set(&mut self, buf_addr: usize, width: i32, height: i32, transparent: Option<Color>) {
        self.buf_addr = buf_addr;
        self.width = width;
        self.height = height;
        self.transparent = transparent;
    }
}

pub const MAX_SHEETS: usize = 256;

pub struct SheetManager {
    pub z_max: Option<usize>,             // 一番上のSheetのz
    pub map_addr: i32,                    // 重ね合わせ計算用のマップをもつ
    pub sheets: [usize; MAX_SHEETS],      // sheets_data上のindexを保持する
    pub sheets_data: [Sheet; MAX_SHEETS], // sheetデータの実体
}

impl SheetManager {
    pub fn new(map_addr: i32) -> SheetManager {
        SheetManager {
            z_max: None,
            map_addr,
            sheets: [0; MAX_SHEETS],
            sheets_data: [Sheet::new(); MAX_SHEETS],
        }
    }

    pub fn set_buf(
        &mut self,
        sheet_index: usize,
        buf_addr: usize,
        width: i32,
        height: i32,
        transparent: Option<Color>,
    ) {
        let sheet = &mut self.sheets_data[sheet_index];
        sheet.set(buf_addr, width, height, transparent);
    }

    pub fn get_buf_addr(&self, sheet_index: usize) -> usize {
        let sheet = &self.sheets_data[sheet_index];
        sheet.buf_addr
    }

    pub fn alloc(&mut self) -> Option<usize> {
        for i in 0..MAX_SHEETS {
            if self.sheets_data[i].flag == SheetFlag::AVAILABLE {
                let mut sheet = &mut self.sheets_data[i];
                sheet.flag = SheetFlag::USED;
                sheet.z = None;
                return Some(i);
            }
        }
        None
    }

    pub fn refresh_map(&self, x0: i32, y0: i32, x1: i32, y1: i32, z0: i32) {
        if self.z_max.is_none() {
            return;
        }
        let x0 = max(0, x0);
        let y0 = max(0, y0);
        let x1 = min(x1, *SCREEN_WIDTH as i32);
        let y1 = min(y1, *SCREEN_HEIGHT as i32);
        for h in (z0 as usize)..=self.z_max.unwrap() {
            let si = self.sheets[h as usize];
            let sheet = &self.sheets_data[si];
            let bx0 = if x0 > sheet.x { x0 - sheet.x } else { 0 } as usize;
            let by0 = if y0 > sheet.y { y0 - sheet.y } else { 0 } as usize;
            let bx1 = if x1 > sheet.x {
                min(x1 - sheet.x, sheet.width)
            } else {
                0
            } as usize;
            let by1 = if y1 > sheet.y {
                min(y1 - sheet.y, sheet.height)
            } else {
                0
            } as usize;
            for by in by0..by1 {
                let vy = (sheet.y + by as i32) as usize;
                for bx in bx0..bx1 {
                    let vx = (sheet.x + bx as i32) as usize;
                    let width = sheet.width as usize;
                    let c = unsafe { *((sheet.buf_addr + by * width + bx) as *const Color) };
                    if Some(c) != sheet.transparent {
                        let ptr = unsafe {
                            &mut *((self.map_addr as *mut u8)
                                .offset(vy as isize * *SCREEN_WIDTH as isize + vx as isize))
                        };
                        *ptr = si as u8;
                    }
                }
            }
        }
    }

    pub fn refresh_part(&self, x0: i32, y0: i32, x1: i32, y1: i32, z0: i32, z1: i32) {
        if self.z_max.is_none() {
            return;
        }
        let x0 = max(0, x0);
        let y0 = max(0, y0);
        let x1 = min(x1, *SCREEN_WIDTH as i32);
        let y1 = min(y1, *SCREEN_HEIGHT as i32);

        for h in (z0 as usize)..=(z1 as usize) {
            let si = self.sheets[h as usize];
            let sheet = &self.sheets_data[si];
            let bx0 = if x0 > sheet.x { x0 - sheet.x } else { 0 } as usize;
            let by0 = if y0 > sheet.y { y0 - sheet.y } else { 0 } as usize;
            let bx1 = if x1 > sheet.x {
                min(x1 - sheet.x, sheet.width)
            } else {
                0
            } as usize;
            let by1 = if y1 > sheet.y {
                min(y1 - sheet.y, sheet.height)
            } else {
                0
            } as usize;
            for by in by0..by1 {
                let vy = (sheet.y + by as i32) as usize;
                for bx in bx0..bx1 {
                    let vx = (sheet.x + bx as i32) as usize;
                    let width = sheet.width as usize;
                    let map_si = unsafe {
                        *((self.map_addr as isize
                            + vy as isize * *SCREEN_WIDTH as isize
                            + vx as isize) as *const u8)
                    };
                    if si as u8 == map_si {
                        let c = unsafe { *((sheet.buf_addr + by * width + bx) as *const Color) };
                        let ptr = unsafe {
                            &mut *((*VRAM_ADDR as *mut u8)
                                .offset(vy as isize * *SCREEN_WIDTH as isize + vx as isize))
                        };
                        *ptr = c as u8;
                    }
                }
            }
        }
    }

    pub fn updown(&mut self, sheet_index: usize, oz: Option<usize>) {
        let sheet = self.sheets_data[sheet_index];
        let old = sheet.z;
        let oz = if let Some(z) = oz {
            Some(min(
                if let Some(zmax) = self.z_max {
                    zmax as usize + 1
                } else {
                    0
                },
                z,
            ))
        } else {
            None
        };
        {
            let mut sh = &mut self.sheets_data[sheet_index];
            sh.z = oz;
        }
        if old != oz {
            let z0: i32;
            let z1: i32;
            if let Some(o) = old {
                if let Some(z) = oz {
                    // 下げる
                    if o > z {
                        let mut h = o;
                        while h > z {
                            self.sheets[h] = self.sheets[h - 1];
                            let mut sh = &mut self.sheets_data[self.sheets[h]];
                            sh.z = Some(h);
                            h -= 1;
                        }
                        self.sheets[z] = sheet_index;
                        z0 = z as i32;
                        z1 = o as i32;
                    // 上げる
                    } else if o < z {
                        for h in o..z {
                            self.sheets[h] = self.sheets[h + 1];
                            let mut sh = &mut self.sheets_data[self.sheets[h]];
                            sh.z = Some(h);
                        }
                        self.sheets[z] = sheet_index;
                        z0 = z as i32;
                        z1 = z as i32;
                    } else {
                        return;
                    }
                } else {
                    // 表示 -> 非表示
                    if let Some(zmax) = self.z_max {
                        if zmax > o {
                            for h in o..zmax as usize {
                                self.sheets[h] = self.sheets[h + 1];
                                let mut sh = &mut self.sheets_data[self.sheets[h]];
                                sh.z = Some(h);
                            }
                        }
                        self.sheets[zmax + 1] = sheet_index;
                        self.z_max = if zmax > 0 { Some(zmax - 1) } else { None }
                    }
                    z0 = 0;
                    z1 = o as i32 - 1;
                }
            } else {
                // 非表示 -> 表示
                if let Some(z) = oz {
                    let zmax = if let Some(zmax) = self.z_max { zmax } else { 0 };
                    let mut h = zmax;
                    while h >= z {
                        self.sheets[h + 1] = self.sheets[h];
                        let mut sh = &mut self.sheets_data[self.sheets[h + 1]];
                        sh.z = Some(h + 1);
                        if h == 0 {
                            break;
                        }
                        h -= 1;
                    }
                    self.sheets[z] = sheet_index;
                    if let Some(zmax) = self.z_max {
                        self.z_max = Some(zmax + 1);
                    } else {
                        self.z_max = Some(0)
                    }
                    z0 = z as i32;
                    z1 = z as i32;
                } else {
                    return;
                }
            }
            self.refresh_map(
                sheet.x,
                sheet.y,
                sheet.x + sheet.width,
                sheet.y + sheet.height,
                z0,
            );
            self.refresh_part(
                sheet.x,
                sheet.y,
                sheet.x + sheet.width,
                sheet.y + sheet.height,
                z0,
                z1,
            );
        }
    }

    pub fn refresh(&self, sheet_index: usize, x0: i32, y0: i32, x1: i32, y1: i32) {
        let sheet = self.sheets_data[sheet_index];
        if let Some(z) = sheet.z {
            self.refresh_part(
                sheet.x + x0,
                sheet.y + y0,
                sheet.x + x1,
                sheet.y + y1,
                z as i32,
                z as i32,
            );
        }
    }

    pub fn get_new_point(&self, sheet_index: usize, dx: i32, dy: i32) -> (i32, i32) {
        let scrnx = *SCREEN_WIDTH as i32;
        let scrny = *SCREEN_HEIGHT as i32;
        let sheet = self.sheets_data[sheet_index];
        let mut new_x = sheet.x + dx;
        let mut new_y = sheet.y + dy;
        let xmax = scrnx - 1;
        let ymax = scrny - 1;
        if new_x < 0 {
            new_x = 0;
        } else if new_x > xmax {
            new_x = xmax;
        }
        if new_y < 0 {
            new_y = 0;
        } else if new_y > ymax {
            new_y = ymax;
        }
        return (new_x, new_y);
    }

    pub fn slide_by_diff(&mut self, sheet_index: usize, dx: i32, dy: i32) {
        let (new_x, new_y) = self.get_new_point(sheet_index, dx, dy);
        self.slide(sheet_index, new_x, new_y);
    }

    pub fn slide(&mut self, sheet_index: usize, x: i32, y: i32) {
        let sheet = self.sheets_data[sheet_index];
        let old_x = sheet.x;
        let old_y = sheet.y;
        {
            let sh = &mut self.sheets_data[sheet_index];
            sh.x = x;
            sh.y = y;
        }
        if let Some(z) = sheet.z {
            self.refresh_map(old_x, old_y, old_x + sheet.width, old_y + sheet.height, 0);
            self.refresh_map(x, y, x + sheet.width, y + sheet.height, z as i32);
            self.refresh_part(
                old_x,
                old_y,
                old_x + sheet.width,
                old_y + sheet.height,
                0,
                z as i32 - 1,
            );
            self.refresh_part(x, y, x + sheet.width, y + sheet.height, z as i32, z as i32);
        }
    }

    pub fn free(&mut self, sheet_index: usize) {
        let sheet = self.sheets_data[sheet_index];
        if sheet.z.is_some() {
            self.updown(sheet_index, None);
        }
        let mut sheet = &mut self.sheets_data[sheet_index];
        sheet.flag = SheetFlag::AVAILABLE;
    }
}
