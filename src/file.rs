pub const ADR_DISKIMG: usize = 0x00100000;
pub const ADR_FILE_OFFSET: usize = 0x002600;
pub const MAX_FILE_INFO: usize = 224;

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
    pub fn content_addr(&self) -> usize {
        self.clustno as usize * 512 + 0x003e00 + ADR_DISKIMG
    }
}
