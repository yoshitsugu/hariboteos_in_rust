pub fn hlt() {
    unsafe {
        asm!("hlt");
    }
}

pub fn load_eflags() -> i32 {
    let result: i32;
    unsafe {
        asm!("PUSHFD" : : : : "intel");
        asm!("POP EAX" : "={EAX}"(result) : : : "intel");
    }
    result
}

pub fn store_eflags(flags: i32) {
    unsafe {
        asm!("PUSH EAX" : : "EAX"(flags) : : "intel");
        asm!("POPFD");
    }
}

pub fn load_cr0() -> u32 {
    let result: u32;
    unsafe {
        asm!("MOV EAX,CR0" : "={EAX}"(result) : : : "intel");
    }
    result
}

pub fn store_cr0(cr0: u32) {
    unsafe {
        asm!("MOV CR0,EAX" : : "{EAX}"(cr0) : : "intel");
    }
}

pub fn cli() {
    unsafe {
        asm!("CLI" : : : : "intel");
    }
}

pub fn sti() {
    unsafe {
        asm!("STI" : : : : "intel");
    }
}

pub fn stihlt() {
    unsafe {
        asm!("STI
              HLT" : : : : "intel");
    }
}

pub fn out8(port: u32, data: u8) {
    unsafe {
        asm!("OUT DX,AL" : : "{EDX}"(port), "{AL}"(data) : : "intel");
    }
}

pub fn in8(port: u32) -> u8 {
    let mut r: u8;
    unsafe {
        asm!("MOV EDX,$0" : : "i"(port) : : "intel");
        asm!("MOV EAX,0" : : : : "intel");
        asm!("IN AL,DX" : "={AL}"(r) : : : "intel");
    }
    r
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct Dtr {
    limit: i16,
    base: i32,
}

pub fn load_gdtr(limit: i32, adr: i32) {
    unsafe {
        asm!("LGDT ($0)" :: "r"(&Dtr { limit: limit as i16, base: adr } ) : "memory");
    }
}

pub fn load_idtr(limit: i32, adr: i32) {
    unsafe {
        asm!("LIDT ($0)" :: "r"(&Dtr { limit: limit as i16, base: adr }) : "memory");
    }
}

pub fn load_tr(adr: i32) {
    unsafe {
        asm!("LTR [$0]" :: "r"(&adr) : "memory" : "intel");
    }
}

#[naked]
pub fn farjmp(eip: i32, cs: i32) {
    unsafe {
        asm!("LJMP $0,$1" :: "i"(cs), "i"(eip));
    }
}

#[macro_export]
macro_rules! handler {
    ($name: ident) => {{
        #[naked]
        pub extern "C" fn wrapper() {
            unsafe {
                asm!("PUSH ES
                      PUSH DS
                      PUSHAD
                      MOV EAX,ESP
                      PUSH EAX
                      MOV AX,SS
                      MOV DS,AX
                      MOV ES,AX" : : : : "intel", "volatile");
                asm!("CALL $0" : : "r"($name as extern "C" fn()) : : "intel");
                asm!("POP EAX
                    POPAD
                    POP DS
                    POP ES
                    IRETD" : : : : "intel", "volatile");
            }
        }
        wrapper
    }}
}
