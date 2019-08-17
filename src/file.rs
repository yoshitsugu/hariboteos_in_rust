pub const ADR_DISKIMG: usize = 0x00100000;
pub const ADR_FILE_OFFSET: usize = 0x002600;
pub const MAX_FILE_INFO: usize = 224;
pub const MAX_FAT: usize = 2880;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed)]
pub struct FileInfo {
    pub name: [u8; 8],
    pub ext: [u8; 3],
    pub ftype: u8,
    pub reserve: [i8; 10],
    pub time: u16,
    pub date: u16,
    pub clustno: u16,
    pub size: u32,
}

impl FileInfo {
    pub fn load_file(&self, buf_addr: usize, fat: &[u32; MAX_FAT], img_addr: usize) {
        let mut size = self.size as usize;
        let mut buf_addr = buf_addr as usize;
        let mut clustno = self.clustno as usize;
        loop {
            if size <= 512 {
                for i in 0..size {
                    let buf = unsafe { &mut *((buf_addr + i) as *mut u8) };
                    *buf = unsafe { *((img_addr + clustno * 512 + i) as *const u8) };
                }
                break;
            }
            for i in 0..512 {
                let buf = unsafe { &mut *((buf_addr + i) as *mut u8) };
                *buf = unsafe { *((img_addr + clustno * 512 + i) as *const u8) };
            }
            size -= 512;
            buf_addr += 512;
            clustno = fat[clustno] as usize;
        }
    }
}

pub fn search_file(filename: &[u8]) -> Option<FileInfo> {
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

pub fn read_fat(fat: &mut [u32; MAX_FAT], img: [u8; MAX_FAT * 4]) {
    let mut j = 0;
    for i in (0..MAX_FAT).step_by(2) {
        fat[i + 0] = ((img[j + 0] as u32) | (img[j + 1] as u32) << 8) & 0xfff;
        fat[i + 1] = ((img[j + 1] as u32) >> 4 | (img[j + 2] as u32) << 4) & 0xfff;
        j += 3;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct FileHandler {
    pub buf_addr: usize,
    pub size: i32,
    pub pos: i32,
}

impl FileHandler {
    pub fn new() -> FileHandler {
        FileHandler {
            buf_addr: 0,
            size: 0,
            pos: 0,
        }
    }
}
