use volatile::Volatile;

use crate::asm;

const EFLAGS_AC_BIT: u32 = 0x00040000;
const CR0_CACHE_DISABLE: u32 = 0x60000000;

pub fn memtest(start: u32, end: u32) -> u32 {
    let mut flg486 = false;
    asm::store_eflags((asm::load_eflags() as u32 | EFLAGS_AC_BIT) as i32);
    let mut eflags = asm::load_eflags() as u32;
    // 386ではAC=1にしても自動で0に戻ってしまう
    if eflags & EFLAGS_AC_BIT != 0 {
        flg486 = true;
    }
    eflags &= !EFLAGS_AC_BIT;
    asm::store_eflags(eflags as i32);

    if flg486 {
        // キャッシュ禁止
        let cr0 = asm::load_cr0() | CR0_CACHE_DISABLE;
        asm::store_cr0(cr0);
    }

    let memory = memtest_main(start, end);

    if flg486 {
        // キャッシュ許可
        let mut cr0 = asm::load_cr0();
        cr0 &= !CR0_CACHE_DISABLE;
        asm::store_cr0(cr0);
    }

    memory
}

fn memtest_main(start: u32, end: u32) -> u32 {
    let pat0: u32 = 0xaa55aa55;
    let pat1: u32 = 0x55aa55aa;
    let mut r = start;
    for i in (start..end).step_by(0x1000) {
        r = i;
        let mp = (i + 0xffc) as *mut u32;
        let p = unsafe { &mut *(mp as *mut Volatile<u32>) };
        let old = p.read();
        p.write(pat0);
        p.write(!p.read());
        if p.read() != pat1 {
            p.write(old);
            break;
        }
        p.write(!p.read());
        if p.read() != pat0 {
            p.write(old);
            break;
        }
        p.write(old);
    }
    r
}

const MEMMAN_FREES: u32 = 4090; // 約32KB
pub const MEMMAN_ADDR: u32 = 0x003c0000;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C, packed)]
struct FreeInfo {
    addr: u32,
    size: u32,
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct MemMan {
    frees: u32,
    maxfrees: u32,
    lostsize: u32,
    losts: u32,
    free: [FreeInfo; MEMMAN_FREES as usize],
}

impl MemMan {
    pub fn new() -> MemMan {
        MemMan {
            frees: 0,
            maxfrees: 0,
            lostsize: 0,
            losts: 0,
            free: [FreeInfo { addr: 0, size: 0 }; MEMMAN_FREES as usize],
        }
    }

    pub fn total(&self) -> u32 {
        let mut t = 0;
        for i in 0..self.frees {
            t += self.free[i as usize].size;
        }
        t
    }

    pub fn alloc(&mut self, size: u32) -> Result<u32, &'static str> {
        for i in 0..self.frees {
            let i = i as usize;
            if self.free[i].size >= size {
                let a = self.free[i].addr;
                self.free[i].addr += size;
                self.free[i].size -= size;
                if self.free[i].size == 0 {
                    self.frees -= 1;
                    self.free[i] = self.free[i + 1]
                }
                return Ok(a);
            }
        }
        Err("CANNOT ALLOCATE MEMORY")
    }

    pub fn free(&mut self, addr: u32, size: u32) -> Result<(), &'static str> {
        let mut idx: usize = 0;
        // addrの順に並ぶように、insertすべきindexを決める
        for i in 0..self.frees {
            let i = i as usize;
            if self.free[i].addr > addr {
                idx = i;
                break;
            }
        }
        if idx > 0 {
            if self.free[idx - 1].addr + self.free[idx - 1].size == addr {
                self.free[idx - 1].size += size;
                if idx < self.frees as usize {
                    if addr + size == self.free[idx].addr {
                        self.free[idx - 1].size += self.free[idx].size;
                    }
                    self.frees -= 1;
                    for i in idx..(self.frees as usize) {
                        self.free[i] = self.free[i + 1];
                    }
                }
                return Ok(());
            }
        }
        if idx < self.frees as usize {
            if addr + size == self.free[idx].addr {
                self.free[idx].addr = addr;
                self.free[idx].size += size;
                return Ok(());
            }
        }
        if self.frees < MEMMAN_FREES {
            let mut j = self.frees as usize;
            while j > idx {
                self.free[j] = self.free[j - 1];
                j -= 1;
            }
            self.frees += 1;
            if self.maxfrees < self.frees {
                self.maxfrees = self.frees;
            }
            self.free[idx].addr = addr;
            self.free[idx].size = size;
            return Ok(());
        }
        self.losts += 1;
        self.lostsize += size;
        Err("CANNOT FREE MEMORY")
    }

    pub fn alloc_4k(&mut self, size: u32) -> Result<u32, &'static str> {
        let size = (size + 0xfff) & 0xfffff000;
        self.alloc(size)
    }

    pub fn free_4k(&mut self, addr: u32, size: u32) -> Result<(), &'static str> {
        let size = (size + 0xfff) & 0xfffff000;
        self.free(addr, size)
    }
}
