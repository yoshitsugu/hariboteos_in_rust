use core::cmp::{min, max};

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

const MAX_SHEETS: usize = 256;

pub struct SheetManager {
    pub z_max: Option<usize>,             // 一番上のSheetのz
    pub sheets: [usize; MAX_SHEETS],      // sheets_data上のindexを保持する
    pub sheets_data: [Sheet; MAX_SHEETS], // sheetデータの実体
}

impl SheetManager {
    pub fn new() -> SheetManager {
        SheetManager {
            z_max: None,
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

    pub fn refresh_part(&self, x0: i32, y0: i32, x1: i32, y1: i32) {
        if self.z_max.is_none() {
            return;
        }
        let x0 = max(0, x0);
        let y0 = max(0, y0);
        let x1 = min(x1, *SCREEN_WIDTH as i32);
        let y1 = min(y1, *SCREEN_HEIGHT as i32);

        for h in 0..=self.z_max.unwrap() {
            let sheet = &self.sheets_data[self.sheets[h as usize]];
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
                let vy = sheet.y as usize + by;
                for bx in bx0..bx1 {
                    let vx = sheet.x as usize + bx;
                    let width = sheet.width as usize;
                    let c = unsafe { *((sheet.buf_addr + by * width + bx) as *const Color) };
                    if Some(c) != sheet.transparent {
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
            if let Some(o) = old {
                if let Some(z) = oz {
                    if o > z {
                        let mut h = o;
                        while h > z {
                            self.sheets[h] = self.sheets[h - 1];
                            let mut sh = &mut self.sheets_data[self.sheets[h]];
                            sh.z = Some(h);
                            h -= 1;
                        }
                        self.sheets[z] = sheet_index;
                    } else if o < z {
                        for h in o..z {
                            self.sheets[h] = self.sheets[h + 1];
                            let mut sh = &mut self.sheets_data[self.sheets[h]];
                            sh.z = Some(h);
                        }
                        self.sheets[z] = sheet_index;
                    }
                } else {
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
                }
            } else {
                if let Some(z) = oz {
                    let zmax = if let Some(zmax) = self.z_max { zmax } else { 0 };
                    for h in z..zmax {
                        self.sheets[h + 1] = self.sheets[h];
                        let mut sh = &mut self.sheets_data[self.sheets[h + 1]];
                        sh.z = Some(h + 1);
                    }
                    self.sheets[z] = sheet_index;
                    if let Some(zmax) = self.z_max {
                        self.z_max = Some(zmax + 1);
                    } else {
                        self.z_max = Some(0)
                    }
                }
            }
            self.refresh_part(
                sheet.x,
                sheet.y,
                sheet.x + sheet.width,
                sheet.y + sheet.height,
            );
        }
    }

    pub fn refresh(&self, sheet_index: usize, x0: i32, y0: i32, x1: i32, y1: i32) {
        let sheet = self.sheets_data[sheet_index];
        if sheet.z.is_some() {
            self.refresh_part(sheet.x + x0, sheet.y + y0, sheet.x + x1, sheet.y + y1);
        }
    }

    pub fn slide_by_diff(&mut self, sheet_index: usize, dx: i32, dy: i32) {
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
        if sheet.z.is_some() {
            self.refresh_part(
                old_x,
                old_y,
                old_x + sheet.width as i32,
                old_y + sheet.height as i32,
            );
            self.refresh_part(x, y, x + sheet.width, y + sheet.height);
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
