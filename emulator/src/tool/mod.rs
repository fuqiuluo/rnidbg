mod arm;

use bytes::{Buf};

pub use arm::{*};

const ALIGN_SIZE_BASE: usize = 0x10;

pub struct Alignment {
    pub(crate) address: u64,
    pub(crate) size: usize,
    pub(crate) begin: u64,
    pub(crate) data_size: u64,
}

pub fn align_size(size: usize) -> usize {
    ((size - 1) / ALIGN_SIZE_BASE + 1) * ALIGN_SIZE_BASE
}

pub fn align_addr(addr: u64, size: u64, alignment: i64) -> Alignment {
    let mask = -alignment;
    let mut right = addr + size;
    let right = (right as i64 + alignment - 1) & mask;
    let mut addr = addr as i64;
    addr &= mask;
    let size = right - addr;
    let size = (size + alignment - 1) & mask;
    Alignment {
        address: addr as u64,
        size: size as usize,
        begin: 0,
        data_size: 0
    }
}

pub fn get_segment_protection(flags: u32) -> u32 {
    let mut prot = 0;
    if (flags & 4) != 0 {
        prot |= 1;
    }
    if (flags & 2) != 0 {
        prot |= 2;
    }
    if (flags & 1) != 0 {
        prot |= 4;
    }
    return prot;
}