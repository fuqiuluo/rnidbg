//const MEMORY_HEAP_BASE: u64 = 0x7201_0000_0000;
//const MEMORY_MMAP_BASE: u64 = 0x7203_0000_0000;

use crate::linux::PAGE_ALIGN;

pub const STACK_BASE: u64 = {
    #[cfg(not(feature = "big_address"))]
    {
        0xc0000000
    }

    #[cfg(feature = "big_address")]
    {
        (0x7200_0000_0000 + (STACK_SIZE_OF_PAGE * PAGE_ALIGN)) as u64
    }
};

pub const LR: u64 =        0x7ffff0000;
//pub const MMAP_BASE: u64 = 0x7203_0000_0000;
//pub const MMAP_BASE: u64 = 0x40000000;
pub const MMAP_BASE: u64 = {
    #[cfg(not(feature = "big_address"))]
    {
        0x40000000
    }

    #[cfg(feature = "big_address")]
    {
        0x7203_0000_0000
    }
};

pub const STACK_SIZE_OF_PAGE: usize = 0x4000;

pub const HEAP_BASE: u64 = {
    #[cfg(not(feature = "big_address"))]
    {
        0x8048000
    }

    #[cfg(feature = "big_address")]
    {
        0x7201_0000_0000
    }
};

pub const SVC_BASE: u64 = 0xfffe0000;
pub const SVC_SIZE: usize = 0x4000;
pub const SVC_MAX: u32 = 0xffff;