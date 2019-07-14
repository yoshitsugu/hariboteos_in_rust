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

pub fn read_fat(fat: &mut [u32; MAX_FAT], img: [u8; MAX_FAT * 4]) {
    let mut j = 0;
    for i in (0..MAX_FAT).step_by(2) {
        fat[i + 0] = ((img[j + 0] as u32) | (img[j + 1] as u32) << 8) & 0xfff;
        fat[i + 1] = ((img[j + 1] as u32) >> 4 | (img[j + 2] as u32) << 4) & 0xfff;
        j += 3;
    }
}
