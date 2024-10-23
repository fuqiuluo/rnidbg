extern crate alloc;

use std::cell::UnsafeCell;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::mem;
use std::process::exit;
use std::ptr::{null_mut};
use std::rc::Rc;
use ansi_term::Color;
use anyhow::anyhow;
use log::{error, warn};
use crate::ffi::{DyHook, SFHook};

mod ffi;

pub type DynarmicContext = Rc<DynarmicContextInner>;

#[derive(Clone)]
pub struct DynarmicContextInner {
    inner_context: *mut c_void,
}

impl DynarmicContextInner {
    pub fn destroy(&self) {
        unsafe {
            ffi::dynarmic_context_free(self.inner_context);
        }
    }
}

impl Drop for DynarmicContextInner {
    fn drop(&mut self) {
        self.destroy();
    }
}

pub fn dynarmic_version() -> u32 {
    unsafe { ffi::dynarmic_version() }
}

pub fn dynarmic_colorful_egg() -> String {
    unsafe {
        let c_str = ffi::dynarmic_colorful_egg();
        let c_str = std::ffi::CStr::from_ptr(c_str);
        c_str.to_string_lossy().into_owned()
    }
}

struct Metadata<'a> {
    svc_callback: Option<Box<dyn SFHook + 'a>>,
    until: u64,
    memory: *mut c_void,
    monitor: *mut c_void,
    page_table: *mut *mut c_void,
    handle: *mut c_void,
}

impl Drop for Metadata<'_> {
    fn drop(&mut self) {
        log::info!("[dynarmic] Dropping Metadata");
        unsafe {
            ffi::dynarmic_destroy(self.handle);
        }
    }
}

#[derive(Clone)]
pub struct Dynarmic<'a, T: Clone> {
    cur_handle: *mut c_void,
    metadata: Rc<UnsafeCell<Metadata<'a>>>,
    pd: PhantomData<&'a T>
}

impl<'a, T: Clone> Dynarmic<'a, T> {
    pub fn new() -> Dynarmic<'static, T> {
        let memory = unsafe { ffi::dynarmic_init_memory() };
        if memory == null_mut() {
            error!("Failed to initialize memory");
            exit(0)
        }

        let mut jit_size = std::env::var("DYNARMIC_JIT_SIZE").unwrap_or("64".to_string())
            .parse::<u64>().unwrap();
        if jit_size < 8 {
            warn!("JIT size is too small, setting to 8");
            jit_size = 8;
        } else if jit_size > 128 {
            warn!("JIT size is too large, setting to 128");
            jit_size = 128;
        }

        let monitor = unsafe { ffi::dynarmic_init_monitor(1) };
        let page_table = unsafe { ffi::dynarmic_init_page_table() };
        let handle = unsafe { ffi::dynarmic_new(0, memory, monitor, page_table, jit_size * 1024 * 1024, false) };

        if option_env!("DYNARMIC_DEBUG") == Some("1") {
            println!("{}[Dynarmic]{} Created new Dynarmic instance: {:X}", Color::Green.paint("[*]"), Color::White.paint(""), handle as usize);
        }

        Dynarmic {
            cur_handle: handle,
            metadata: Rc::new(UnsafeCell::new(Metadata {
                svc_callback: None,
                until: 0,
                memory,
                monitor,
                page_table,
                handle,
            })),
            pd: PhantomData,
        }
    }

    pub fn emu_start(&self, pc: u64, until: u64) -> anyhow::Result<()> {
        unsafe {
            if option_env!("DYNARMIC_DEBUG") == Some("1") {
                println!("{}[Dynarmic]{} Starting emulator: pc=0x{:x}", Color::Green.paint("[*]"), Color::White.paint(""), pc);
            }

            (*self.metadata.get()).until = until + 4;

            let ret = ffi::dynarmic_emu_start(self.cur_handle, pc);
            if ret != 0 {
                return Err(anyhow!("Failed to start emulator: code={}", ret));
            }
            Ok(())
        }
    }

    pub fn emu_stop(&self) -> anyhow::Result<()> {
        unsafe {
            if option_env!("DYNARMIC_DEBUG") == Some("1") {
                println!("{}[Dynarmic]{} Stopping emulator", Color::Green.paint("[*]"), Color::White.paint(""));
            }
            let ret = ffi::dynarmic_emu_stop(self.cur_handle);
            if ret != 0 {
                return Err(anyhow!("Failed to stop emulator: code={}", ret));
            }
            Ok(())
        }
    }

    pub fn get_cache_size(&self) -> u64 {
        unsafe {
            ffi::dynarmic_get_cache_size(self.cur_handle)
        }
    }

    pub fn context_alloc(&self) -> DynarmicContext {
        unsafe {
            let inner_context = ffi::dynarmic_context_alloc();
            Rc::new(DynarmicContextInner {
                inner_context,
            })
        }
    }

    pub fn context_save(&self, context: &mut DynarmicContext) -> anyhow::Result<()> {
        unsafe {
            let ret = ffi::dynarmic_context_save(self.cur_handle, context.inner_context);
            if ret != 0 {
                return Err(anyhow!("Failed to save context: code={}", ret));
            }
            Ok(())
        }
    }

    pub fn context_restore(&self, context: &DynarmicContext) -> anyhow::Result<()> {
        unsafe {
            let ret = ffi::dynarmic_context_restore(self.cur_handle, context.inner_context);
            if ret != 0 {
                return Err(anyhow!("Failed to restore context: code={}", ret));
            }
            Ok(())
        }
    }

    pub fn context_free(&self, context: DynarmicContext) {
        unsafe {
            ffi::dynarmic_context_free(context.inner_context);
        }
    }

    pub fn mem_map(&self, addr: u64, size: usize, prot: u32) -> anyhow::Result<()> {
        unsafe {
            if option_env!("DYNARMIC_DEBUG") == Some("1") {
                println!("{}[Dynarmic]{} Mapping memory: addr=0x{:x}, size=0x{:x}, prot={}", Color::Green.paint("[*]"), Color::White.paint(""), addr, size, prot);
            }
            let ret = ffi::dynarmic_mmap(self.cur_handle, addr, size as u64, mem::transmute(prot));
            if ret == 4 {
                warn!("Replace mmap?");
            }
            if ret != 0 {
                return Err(anyhow!("Failed to map memory: code={}", ret));
            }
            if option_env!("DYNARMIC_DEBUG_EX") == Some("1") {
                println!("{}[Dynarmic]{} Mapped memory: addr=0x{:x}, size=0x{:x}, prot={}", Color::Green.paint("[*]"), Color::White.paint(""), addr, size, prot);
            }
            Ok(())
        }
    }

    pub fn mem_unmap(&self, addr: u64, size: usize) -> anyhow::Result<()> {
        unsafe {
            if option_env!("DYNARMIC_DEBUG") == Some("1") {
                println!("{}[Dynarmic]{} Unmapping memory: addr=0x{:x}, size=0x{:x}", Color::Green.paint("[*]"), Color::White.paint(""), addr, size);
            }
            let ret = ffi::dynarmic_munmap(self.cur_handle, addr, size as u64);
            if ret != 0 {
                return Err(anyhow!("Failed to unmap memory: code={}", ret));
            }
            Ok(())
        }
    }

    pub fn mem_protect(&self, addr: u64, size: usize, prot: u32) -> anyhow::Result<()> {
        unsafe {
            if option_env!("DYNARMIC_DEBUG") == Some("1") {
                println!("{}[Dynarmic]{} Protecting memory: addr=0x{:x}, size=0x{:x}, prot={}", Color::Green.paint("[*]"), Color::White.paint(""), addr, size, prot);
            }
            let ret = ffi::dynarmic_mem_protect(self.cur_handle, addr, size as u64, mem::transmute(prot));
            if ret != 0 {
                return Err(anyhow!("Failed to protect memory: code={}", ret));
            }
            Ok(())
        }
    }

    pub fn reg_read(&self, index: usize) -> anyhow::Result<u64> {
        unsafe {
            Ok(ffi::reg_read(self.cur_handle, index as u64))
        }
    }

    pub fn reg_read_lr(&self) -> anyhow::Result<u64> {
        unsafe {
            Ok(ffi::reg_read(self.cur_handle, 30))
        }
    }

    pub fn reg_read_nzcv(&self) -> anyhow::Result<u64> {
        unsafe {
            Ok(ffi::reg_read_nzcv(self.cur_handle))
        }
    }

    pub fn reg_read_sp(&self) -> anyhow::Result<u64> {
        unsafe {
            Ok(ffi::reg_read_sp(self.cur_handle))
        }
    }

    pub fn reg_read_tpidr_el0(&self) -> anyhow::Result<u64> {
        unsafe {
            Ok(ffi::reg_read_tpidr_el0(self.cur_handle))
        }
    }

    pub fn reg_read_pc(&self) -> anyhow::Result<u64> {
        unsafe {
            Ok(ffi::reg_read_pc(self.cur_handle))
        }
    }

    pub fn reg_write_pc(&self, value: u64) -> anyhow::Result<()> {
        unsafe {
            if option_env!("DYNARMIC_DEBUG") == Some("1") {
                println!("{}[Dynarmic]{} Writing PC: value=0x{:x}", Color::Green.paint("[*]"), Color::White.paint(""), value);
            }
            let ret = ffi::reg_write_pc(self.cur_handle, value);
            if ret != 0 {
                return Err(anyhow!("Failed to write PC: code={}", ret));
            }
            Ok(())
        }
    }

    pub fn reg_write_sp(&self, value: u64) -> anyhow::Result<()> {
        unsafe {
            if option_env!("DYNARMIC_DEBUG") == Some("1") {
                println!("{}[Dynarmic]{} Writing SP: value=0x{:x}", Color::Green.paint("[*]"), Color::White.paint(""), value);
            }
            let ret = ffi::reg_write_sp(self.cur_handle, value);
            if ret != 0 {
                return Err(anyhow!("Failed to write SP: code={}", ret));
            }
            Ok(())
        }
    }

    pub fn reg_write_lr(&self, value: u64) -> anyhow::Result<()> {
        unsafe {
            if option_env!("DYNARMIC_DEBUG") == Some("1") {
                println!("{}[Dynarmic]{} Writing LR: value=0x{:x}", Color::Green.paint("[*]"), Color::White.paint(""), value);
            }
            let ret = ffi::reg_write(self.cur_handle, 30, value);
            if ret != 0 {
                return Err(anyhow!("Failed to write LR: code={}", ret));
            }
            Ok(())
        }
    }

    pub fn reg_write_tpidr_el0(&self, value: u64) -> anyhow::Result<()> {
        unsafe {
            if option_env!("DYNARMIC_DEBUG") == Some("1") {
                println!("{}[Dynarmic]{} Writing TPIDR_EL0: value=0x{:x}", Color::Green.paint("[*]"), Color::White.paint(""), value);
            }
            let ret = ffi::reg_write_tpidr_el0(self.cur_handle, value);
            if ret != 0 {
                return Err(anyhow!("Failed to write TPIDR_EL0: code={}", ret));
            }
            Ok(())
        }
    }

    pub fn reg_write_tpidrr0_el0(&self, value: u64) -> anyhow::Result<()> {
        unsafe {
            ffi::reg_write_tpidr_el0(self.cur_handle, value);
        }
        Ok(())
    }

    pub fn reg_write_nzcv(&self, value: u64) -> anyhow::Result<()> {
        unsafe {
            if option_env!("DYNARMIC_DEBUG") == Some("1") {
                println!("{}[Dynarmic]{} Writing NZCV: value=0x{:x}", Color::Green.paint("[*]"), Color::White.paint(""), value);
            }
            let ret = ffi::reg_write_nzcv(self.cur_handle, value);
            if ret != 0 {
                return Err(anyhow!("Failed to write NZCV: code={}", ret));
            }
            Ok(())
        }
    }

    pub fn reg_write_raw(&self, index: usize, value: u64) -> anyhow::Result<()> {
        unsafe {
            if option_env!("DYNARMIC_DEBUG") == Some("1") {
                println!("{}[Dynarmic]{} Writing register: index={}, value=0x{:x}", Color::Green.paint("[*]"), Color::White.paint(""), index, value);
            }
            let ret = ffi::reg_write(self.cur_handle, index as u64, value);
            if ret != 0 {
                return Err(anyhow!("Failed to write register: code={}", ret));
            }
            Ok(())
        }
    }

    pub fn mem_read_c_string(&self, mut addr: u64) -> anyhow::Result<String> {
        let mut buf = Vec::new();
        let mut byte = [0u8];
        loop {
            self.mem_read(addr, &mut byte)?;
            if byte[0] == 0 {
                break;
            }
            buf.push(byte[0]);
            addr += 1;
        }
        unsafe { Ok(String::from_utf8_unchecked(buf)) }
    }

    pub fn mem_read(&self, addr: u64, dest: &mut [u8]) -> anyhow::Result<()> {
        unsafe {
            if option_env!("DYNARMIC_DEBUG_EX") == Some("1") {
                println!("{}[Dynarmic]{} Reading memory: addr=0x{:x}, size=0x{:x}", Color::Green.paint("[*]"), Color::White.paint(""), addr, dest.len());
            }
            let ret = ffi::dynarmic_mem_read(self.cur_handle, addr, dest.as_mut_ptr() as *mut _, dest.len());
            if ret != 0 {
                return Err(anyhow!("Failed to read memory: code={}", ret));
            }
            Ok(())
        }
    }

    pub fn mem_read_as_vec(&self, addr: u64, size: usize) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![0; size];
        self.mem_read(addr, &mut buf)?;
        Ok(buf)
    }

    pub fn mem_write(&self, addr: u64, value: &[u8]) -> anyhow::Result<()> {
        unsafe {
            if option_env!("DYNARMIC_DEBUG_EX") == Some("1") {
                println!("{}[Dynarmic]{} Writing memory: addr=0x{:x}, size=0x{:x}", Color::Green.paint("[*]"), Color::White.paint(""), addr, value.len());
            }
            let ret = ffi::dynarmic_mem_write(self.cur_handle, addr, value.as_ptr() as *const _, value.len());
            if ret != 0 {
                return Err(anyhow!("Failed to write memory: code={}", ret));
            }
            Ok(())
        }
    }

    pub fn set_svc_callback<F: 'a>(&self, callback: F)
    where
        F: FnMut(&Dynarmic<T>, u32, u64, u64),
    {
        if option_env!("DYNARMIC_DEBUG") == Some("1") {
            println!("{}[Dynarmic]{} Setting SVC callback", Color::Green.paint("[*]"), Color::White.paint(""));
        }
        unsafe {
            let mut cb = Box::new(DyHook {
                callback,
                dy: self.clone(),
            });
            let user_data = cb.as_mut() as *mut _ as *const c_void;
            ffi::dynarmic_set_svc_callback(self.cur_handle, |swi, user_data| {
                if swi == 114514 {
                    return; // test
                }
                let cb = &mut *(user_data as *mut DyHook<T, F>);
                let dynarmic = &cb.dy;
                let pc = ffi::reg_read_pc(dynarmic.cur_handle);
                let until = (*dynarmic.metadata.get()).until;

                if option_env!("DYNARMIC_DEBUG") == Some("1") {
                    println!("{}[Dynarmic]{} SVC callback: swi={}", Color::Green.paint("[*]"), Color::White.paint(""), swi);
                }
                //panic!("SVC callback is not implemented");
                (cb.callback)(dynarmic, swi, until, pc);
            }, user_data);
            (*self.metadata.get()).svc_callback = Some(cb);
        }
    }

    pub fn destroy_callback(&self) {
        unsafe {
            ffi::dynarmic_set_svc_callback(self.cur_handle, |_, _| {
                unreachable!("SVC callback should not be called after being destroyed");
            }, null_mut());
            let callback = (*self.metadata.get()).svc_callback.take();
            drop(callback);
        }
    }
}
