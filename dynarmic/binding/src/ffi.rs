use std::ffi::{c_char, c_void};
use crate::Dynarmic;

pub trait SFHook {}

pub struct DyHook<'a, T: Clone, F> {
    pub callback: F,
    pub dy: Dynarmic<'a, T>,
}

impl<'a, T: Clone, F> SFHook for DyHook<'a, T, F> {}

extern "C" {
    pub fn dynarmic_version() -> u32;

    pub fn dynarmic_colorful_egg() -> *const c_char;

    pub fn dynarmic_init_memory() -> *mut c_void;

    pub fn dynarmic_init_monitor(processor_count: u32) -> *mut c_void;

    pub fn dynarmic_init_page_table() -> *mut *mut c_void;

    pub fn dynarmic_new(process_id: u32, memory: *mut c_void, monitor: *mut c_void, page_table: *mut *mut c_void, jit_size: u64, unsafe_optimizations: bool) -> *mut c_void;

    pub fn dynarmic_get_cache_size(dynarmic: *mut c_void) -> u64;

    pub fn dynarmic_destroy(dynarmic: *mut c_void);

    pub fn dynarmic_set_svc_callback(dynarmic: *mut c_void, cb: fn(swi: u32, user_data: *const c_void), user_data: *const c_void);

    pub fn dynarmic_munmap(dynarmic: *mut c_void, address: u64, size: u64) -> i32;

    pub fn dynarmic_mmap(dynarmic: *mut c_void, address: u64, size: u64, perms: i32) -> i32;

    pub fn dynarmic_mem_protect(dynarmic: *mut c_void, address: u64, size: u64, perms: i32) -> i32;

    pub fn dynarmic_mem_write(dynarmic: *mut c_void, address: u64, data: *const c_char, size: usize) -> i32;

    pub fn dynarmic_mem_read(dynarmic: *mut c_void, address: u64, bytes: *mut c_char, size: usize) -> i32;

    pub fn reg_read_pc(dynarmic: *mut c_void) -> u64;

    pub fn reg_write_pc(dynarmic: *mut c_void, value: u64) -> i32;

    pub fn reg_write_sp(dynarmic: *mut c_void, value: u64) -> i32;

    pub fn reg_read_sp(dynarmic: *mut c_void) -> u64;

    pub fn reg_read_nzcv(dynarmic: *mut c_void) -> u64;

    pub fn reg_write_nzcv(dynarmic: *mut c_void, value: u64) -> i32;

    pub fn reg_write_tpidr_el0(dynarmic: *mut c_void, value: u64) -> i32;

    pub fn reg_read_tpidr_el0(dynarmic: *mut c_void) -> u64;

    pub fn reg_write_vector(dynarmic: *mut c_void, index: u64, array: *mut u64) -> i32;

    pub fn reg_read_vector(dynarmic: *mut c_void, index: u64, array: *mut u64) -> i32;

    pub fn reg_write(dynarmic: *mut c_void, index: u64, value: u64) -> i32;

    pub fn reg_read(dynarmic: *mut c_void, index: u64) -> u64;

    pub fn dynarmic_emu_start(dynarmic: *mut c_void, pc: u64) -> i32;

    pub fn dynarmic_emu_stop(dynarmic: *mut c_void) -> i32;

    pub fn dynarmic_context_alloc() -> *mut c_void;

    pub fn dynarmic_context_free(context: *mut c_void);

    pub fn dynarmic_context_restore(dynarmic: *mut c_void, context: *mut c_void) -> i32;

    pub fn dynarmic_context_save(dynarmic: *mut c_void, context: *mut c_void) -> i32;
}