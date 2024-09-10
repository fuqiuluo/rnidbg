use std::process::exit;
use log::error;
#[cfg(feature = "unicorn_backend")]
use unicorn_engine::{RegisterARM64, Unicorn};
#[cfg(feature = "unicorn_backend")]
use unicorn_engine::RegisterARM64::{*};
#[cfg(feature = "unicorn_backend")]
use unicorn_engine::unicorn_const::{HookType, MemType};
use crate::backend::Backend;
use crate::emulator::AndroidEmulator;
use crate::pointer::VMPointer;

#[cfg(feature = "unicorn_backend")]
fn mem_hook_unmapped_unicorn<T: Clone>(hook_type: HookType, backend: &mut Unicorn<T>, mem_type: MemType, addr: u64, size: usize, value: i64) -> bool {
    error!("{:?}::{:?}  memory failed: address=0x{:X}, size={}, value=0x{:X}, LR=0x{:X}", hook_type, mem_type, addr, size, value, backend.reg_read(RegisterARM64::LR).unwrap());
    backend.dump_context(addr, size);
    false
}

//noinspection DuplicatedCode
pub fn register_mem_err_handler<T: Clone>(backend: Backend<T>) {
    #[cfg(feature = "unicorn_backend")]
    if let Backend::Unicorn(unicorn) = backend {
        unicorn.add_mem_hook(HookType::MEM_READ_UNMAPPED, 1, 0, |backend: &mut Unicorn<'_, T>, mem_type: MemType, addr: u64, size: usize, value: i64| {
            mem_hook_unmapped_unicorn(HookType::MEM_READ_UNMAPPED, backend, mem_type, addr, size, value)
        }).expect("failed to add MEM_READ_UNMAPPED hook");
        unicorn.add_mem_hook(HookType::MEM_WRITE_UNMAPPED, 1, 0, |backend: &mut Unicorn<'_, T>, mem_type: MemType, addr: u64, size: usize, value: i64| {
            mem_hook_unmapped_unicorn(HookType::MEM_WRITE_UNMAPPED, backend, mem_type, addr, size, value)
        }).expect("failed to add MEM_WRITE_UNMAPPED hook");
        unicorn.add_mem_hook(HookType::MEM_FETCH_UNMAPPED, 1, 0, |backend: &mut Unicorn<'_, T>, mem_type: MemType, addr: u64, size: usize, value: i64| {
            mem_hook_unmapped_unicorn(HookType::MEM_FETCH_UNMAPPED, backend, mem_type, addr, size, value)
        }).expect("failed to add MEM_FETCH_UNMAPPED hook");
        unicorn.add_mem_hook(HookType::MEM_INVALID, 1, 0, |backend: &mut Unicorn<'_, T>, mem_type: MemType, addr: u64, size: usize, value: i64| {
            error!("MEM_INVALID::{:?}  memory failed: address=0x{:X}, size={}, value=0x{:X}, LR=0x{:X}", mem_type, addr, size, value, backend.reg_read(RegisterARM64::LR).unwrap());
            backend.emu_stop().unwrap();
            return false;
        }).expect("failed to add MEM_INVALID hook");
        return;
    }

    #[cfg(feature = "dynarmic_backend")]
    if let Backend::Dynarmic(dynarmic) = backend {
        return;
    }

    unreachable!("Not supported backend: register_mem_err_handler")
}
