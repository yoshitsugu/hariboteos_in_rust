use core::cell::{Cell, RefCell};
use lazy_static::lazy_static;
use spin::Mutex;

pub struct Fifo {
    pub buf: RefCell<[u32; 128]>,
    pub p: Cell<u32>,
    pub q: Cell<u32>,
    pub free: Cell<u32>,
    pub flags: Cell<u32>,
    pub size: u32,
}

const FLAGS_OVERRUN: u32 = 0x0001;

impl Fifo {
    pub fn new(size: u32) -> Fifo {
        Fifo {
            p: Cell::new(0),
            q: Cell::new(0),
            free: Cell::new(size),
            flags: Cell::new(0),
            size: size,
            buf: RefCell::new([0; 128]),
        }
    }

    pub fn put(&self, data: u32) -> Result<(), &'static str> {
        if self.free.get() == 0 {
            self.flags.set(self.flags.get() | FLAGS_OVERRUN);
            return Err("FLAGS_OVERRUN ERROR");
        }
        {
            let mut buf = self.buf.borrow_mut();
            buf[self.p.get() as usize] = data;
        }
        self.p.set(self.p.get() + 1);
        if self.p.get() == self.size {
            self.p.set(0);
        }
        self.free.set(self.free.get() - 1);
        return Ok(());
    }

    pub fn get(&self) -> Result<u32, &'static str> {
        if self.free.get() == self.size {
            return Err("NO DATA");
        }
        let data = self.buf.borrow()[self.q.get() as usize];
        self.q.set(self.q.get() + 1);
        if self.q.get() == self.size {
            self.q.set(0);
        }
        self.free.set(self.free.get() + 1);
        Ok(data)
    }

    pub fn status(&self) -> u32 {
        self.size - self.free.get()
    }
}

lazy_static! {
    pub static ref FIFO_BUF: Mutex<Fifo> = Mutex::new(Fifo::new(128));
}
