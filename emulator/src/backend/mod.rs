pub mod backend_consts;

use std::marker::PhantomData;
use std::sync::atomic::Ordering;
pub use backend_consts::{*};
use anyhow::anyhow;
use log::{info, warn};
#[cfg(feature = "unicorn_backend")]
use unicorn_engine::{Arch, Arm64CpuModel, ContextMode, Mode, Permission as UnicornPermission, TlbType, Unicorn};

#[cfg(feature = "dynarmic_backend")]
use dynarmic::Dynarmic;
use crate::emulator::AndroidEmulator;
use crate::emulator::thread::TaskStatus;

#[derive(Clone)]
pub enum  Backend<'a, T: Clone> {
    #[cfg(feature = "unicorn_backend")]
    Unicorn(Unicorn<'a, T>),
    #[cfg(feature = "dynarmic_backend")]
    Dynarmic(Dynarmic<'a, T>),
}

impl<'a, T: Clone> Backend<'a, T> {
    pub fn new(
        data: T
    ) -> Backend<'static, T> {
        #[cfg(feature = "unicorn_backend")]
        {
            let unicorn = Unicorn::new_with_data(Arch::ARM64, Mode::ARM, data)
                .expect("failed to initialize Unicorn instance"); // createBackend

            unicorn.ctl_set_cpu_model(Arm64CpuModel::UC_CPU_ARM64_A72 as i32).unwrap();
            unicorn.ctl_tlb_type(TlbType::CPU).unwrap();
            unicorn.ctl_exits_disable().unwrap();
            unicorn.ctl_context_mode(ContextMode::CPU).unwrap();

            return Backend::Unicorn(unicorn)
        }

        #[cfg(feature = "dynarmic_backend")]
        if dynarmic::dynarmic_version() == 20240814 {
            let dynarmic = Dynarmic::new();

            return Backend::Dynarmic(dynarmic)
        } else {
            panic!("Dynarmic version mismatch: {}", dynarmic::dynarmic_colorful_egg());
        }

        unreachable!("Not supported backend")
    }

    #[inline]
    pub fn emu_start(
        &self,
        begin: u64,
        until: u64,
        timeout: u64,
        count: usize
    ) -> anyhow::Result<()> {
        if option_env!("EMU_LOG") == Some("1") {
            println!("emu_start: begin: {:#x}, until: {:#x}, timeout: {:#x}, count: {}", begin, until, timeout, count);
        }

        #[cfg(feature = "unicorn_backend")]
        if let Backend::Unicorn(unicorn) = self {
            return unicorn.emu_start(begin, until, timeout, count)
                .map_err(|e| anyhow!("emu_start failed: {:?}", e))
        }

        #[cfg(feature = "dynarmic_backend")]
        if let Backend::Dynarmic(dynarmic) = self {
            return dynarmic.emu_start(begin, until);
        }

        unreachable!()
    }

    #[inline]
    pub fn mem_map(
        &self,
        address: u64,
        size: usize,
        perms: u32,
    ) -> anyhow::Result<()> {
        #[cfg(feature = "unicorn_backend")]
        if let Backend::Unicorn(unicorn) = self {
            unicorn.mem_map(address, size, UnicornPermission::from_bits_truncate(perms))
                .map_err(|e| anyhow!("mem_map failed: {:?}", e))?;
            return Ok(())
        }

        #[cfg(feature = "dynarmic_backend")]
        if let Backend::Dynarmic(dynarmic) = self {
            dynarmic.mem_map(address, size, perms)?;
            return Ok(())
        }

       unreachable!("Not supported backend")
    }

    #[inline]
    pub fn reg_read(
        &self,
        reg_id: RegisterARM64
    ) -> anyhow::Result<u64> {
        #[cfg(feature = "unicorn_backend")]
        if let Backend::Unicorn(unicorn) = self {
            return unicorn.reg_read(reg_id)
                .map_err(|e| anyhow!("reg_read failed: {:?}", e))
        }

        #[cfg(feature = "dynarmic_backend")]
        if let Backend::Dynarmic(dynarmic) = self {
            let value = reg_id.value();
            return match reg_id.value() {
                0 => panic!("Invalid register: {:?}", reg_id),
                1 => dynarmic.reg_read(29),
                2 => dynarmic.reg_read_lr(),
                3 => dynarmic.reg_read_nzcv(),
                4 => dynarmic.reg_read_sp(),
                168 ..= 196 => {
                    let index = reg_id.value() - RegisterARM64::W0.value();
                    dynarmic.reg_read(index as usize)
                }
                199 ..= 227 => {
                    let index = reg_id.value() - RegisterARM64::X0.value();
                    dynarmic.reg_read(index as usize)
                }
                260 => dynarmic.reg_read_pc(),
                262 => dynarmic.reg_read_tpidr_el0(),
                _ => panic!("Invalid register: {:?}", reg_id)
            }
        }

        unreachable!()
    }

    #[inline]
    pub fn reg_read_u32(
        &self,
        reg_id: RegisterARM64
    ) -> anyhow::Result<u32> {
        #[cfg(feature = "unicorn_backend")]
        if let Backend::Unicorn(unicorn) = self {
            return unicorn.reg_read_u32(reg_id)
                .map_err(|e| anyhow!("reg_read_u32 failed: {:?}", e))
        }

        #[cfg(feature = "dynarmic_backend")]
        if let Backend::Dynarmic(dynarmic) = self {
            let value = match reg_id.value() {
                0 => panic!("Invalid register: {:?}", reg_id),
                1 => dynarmic.reg_read(29),
                2 => dynarmic.reg_read_lr(),
                3 => dynarmic.reg_read_nzcv(),
                4 => dynarmic.reg_read_sp(),
                168 ..= 196 => {
                    let index = reg_id.value() - RegisterARM64::W0.value();
                    return dynarmic.reg_read(index as usize).map(|v| v as u32);
                }
                199 ..= 227 => {
                    let index = reg_id.value() - RegisterARM64::X0.value();
                    return dynarmic.reg_read(index as usize).map(|v| v as u32);
                }
                262 => dynarmic.reg_read_tpidr_el0(),
                _ => panic!("Invalid register: {:?}", reg_id)
            };
            return value.map(|v| v as u32);
        }

        unreachable!()
    }

    #[inline]
    pub fn reg_write(
        &self,
        reg_id: RegisterARM64,
        value: u64
    ) -> anyhow::Result<()> {
        #[cfg(feature = "unicorn_backend")]
        if let Backend::Unicorn(unicorn) = self {
            return unicorn.reg_write(reg_id, value)
                .map_err(|e| anyhow!("reg_write failed: {:?}", e))
        }

        #[cfg(feature = "dynarmic_backend")]
        if let Backend::Dynarmic(dynarmic) = self {
            match reg_id.value() {
                0 => panic!("Invalid register: {:?}", reg_id),
                1 => dynarmic.reg_write_raw(29, value)?,
                2 => dynarmic.reg_write_lr(value)?,
                3 => dynarmic.reg_write_nzcv(value)?,
                4 => dynarmic.reg_write_sp(value)?,
                168 ..= 196 => {
                    let index = reg_id.value() - RegisterARM64::W0.value();
                    dynarmic.reg_write_raw(index as usize, value)?;
                }
                199 ..= 227 => {
                    let index = reg_id.value() - RegisterARM64::X0.value();
                    dynarmic.reg_write_raw(index as usize, value)?;
                }
                262 => dynarmic.reg_write_tpidr_el0(value)?,
                263 => dynarmic.reg_write_tpidrr0_el0(value)?,
                _ => panic!("Invalid register: {:?}", reg_id)
            }
            return Ok(())
        }

        unreachable!()
    }

    #[inline]
    pub fn reg_write_i64(
        &self,
        reg_id: RegisterARM64,
        value: i64
    ) -> anyhow::Result<()> {
        match self {
            #[cfg(feature = "unicorn_backend")]
            Backend::Unicorn(unicorn) => {
                unicorn.reg_write_i64(reg_id, value)
                    .map_err(|e| anyhow!("reg_write_i64 failed: {:?}", e))
            }
            #[cfg(feature = "dynarmic_backend")]
            Backend::Dynarmic(dynarmic) => {
                let value = unsafe { std::mem::transmute::<i64, u64>(value) };
                match reg_id.value() {
                    0 => panic!("Invalid register: {:?}", reg_id),
                    1 => dynarmic.reg_write_raw(29, value)?,
                    2 => dynarmic.reg_write_lr(value)?,
                    3 => dynarmic.reg_write_nzcv(value)?,
                    4 => dynarmic.reg_write_sp(value)?,
                    168..=196 => {
                        let index = reg_id.value() - RegisterARM64::W0.value();
                        dynarmic.reg_write_raw(index as usize, value)?;
                    }
                    199..=227 => {
                        let index = reg_id.value() - RegisterARM64::X0.value();
                        dynarmic.reg_write_raw(index as usize, value)?;
                    }
                    262 => dynarmic.reg_write_tpidr_el0(value)?,
                    263 => dynarmic.reg_write_tpidrr0_el0(value)?,
                    _ => panic!("Invalid register: {:?}", reg_id)
                }
                Ok(())
            }
        }
    }

    #[inline]
    pub fn reg_write_i32(
        &self,
        reg_id: RegisterARM64,
        value: i32
    ) -> anyhow::Result<()> {
        #[cfg(feature = "unicorn_backend")]
        if let Backend::Unicorn(unicorn) = self {
            return unicorn.reg_write_i32(reg_id, value)
                .map_err(|e| anyhow!("reg_write_i32 failed: {:?}", e))
        }

        #[cfg(feature = "dynarmic_backend")]
        if let Backend::Dynarmic(dynarmic) = self {
            let value = unsafe { std::mem::transmute::<i32, u32>(value) } as u64;
            match reg_id.value() {
                0 => panic!("Invalid register: {:?}", reg_id),
                1 => dynarmic.reg_write_raw(29, value)?,
                2 => dynarmic.reg_write_lr(value)?,
                3 => dynarmic.reg_write_nzcv(value)?,
                4 => dynarmic.reg_write_sp(value)?,
                168 ..= 196 => {
                    let index = reg_id.value() - RegisterARM64::W0.value();
                    dynarmic.reg_write_raw(index as usize, value)?;
                }
                199 ..= 227 => {
                    let index = reg_id.value() - RegisterARM64::X0.value();
                    dynarmic.reg_write_raw(index as usize, value)?;
                }
                262 => dynarmic.reg_write_tpidr_el0(value)?,
                263 => dynarmic.reg_write_tpidrr0_el0(value)?,
                _ => panic!("Invalid register: {:?}", reg_id)
            }
            return Ok(())
        }

        unreachable!()
    }

    #[inline]
    pub fn reg_write_long(
        &self,
        reg_id: RegisterARM64,
        value: &[u8]
    ) -> anyhow::Result<()> {
        #[cfg(feature = "unicorn_backend")]
        if let Backend::Unicorn(unicorn) = self {
            return unicorn.reg_write_long(reg_id, value)
                .map_err(|e| anyhow!("reg_write_long failed: {:?}", e))
        }

        #[cfg(feature = "dynarmic_backend")]
        {

        }

        unreachable!()
    }

    #[inline]
    pub fn mem_read_c_string(
        &self,
        address: u64
    ) -> anyhow::Result<String> {
        #[cfg(feature = "unicorn_backend")]
        if let Backend::Unicorn(unicorn) = self {
            return unicorn.mem_read_c_string(address)
                .map_err(|e| anyhow!("mem_read_c_string failed: {:?}", e))
        }

        #[cfg(feature = "dynarmic_backend")]
        if let Backend::Dynarmic(dynarmic) = self {
            return dynarmic.mem_read_c_string(address);
        }

        unreachable!()
    }

    #[inline]
    pub fn mem_write(
        &self,
        address: u64,
        bytes: &[u8]
    ) -> anyhow::Result<()> {
        #[cfg(feature = "unicorn_backend")]
        if let Backend::Unicorn(unicorn) = self {
            return unicorn.mem_write(address, bytes)
                .map_err(|e| anyhow!("mem_write failed: {:?}", e))
        }

        #[cfg(feature = "dynarmic_backend")]
        if let Backend::Dynarmic(dynarmic) = self {
            dynarmic.mem_write(address, bytes)?;
            return Ok(())
        }

        unreachable!()
    }

    #[inline]
    pub fn mem_write_i64(
        &self,
        address: u64,
        value: i64
    ) -> anyhow::Result<()> {
        #[cfg(feature = "unicorn_backend")]
        if let Backend::Unicorn(unicorn) = self {
            return unicorn.mem_write_i64(address, value)
                .map_err(|e| anyhow!("mem_write_i64 failed: {:?}", e))
        }

        unreachable!()
    }

    #[inline]
    pub fn mem_read_v2<Data>(
        &self,
        address: u64,
    ) -> anyhow::Result<Data> {
        let size = size_of::<Data>();
        let mut buf = vec![0u8; size];
        self.mem_read(address, &mut buf)?;

        Ok(unsafe { std::ptr::read(buf.as_ptr() as *const Data) })
    }

    #[inline]
    pub fn mem_read(
        &self,
        address: u64,
        buf: &mut [u8]
    ) -> anyhow::Result<()> {
        #[cfg(feature = "unicorn_backend")]
        if let Backend::Unicorn(unicorn) = self {
            return unicorn.mem_read(address, buf)
                .map_err(|e| anyhow!("mem_read failed: {:?}", e))
        }

        #[cfg(feature = "dynarmic_backend")]
        if let Backend::Dynarmic(dynarmic) = self {
            return dynarmic.mem_read(address, buf);
        }

        unreachable!()
    }

    #[inline]
    pub fn mem_read_i64(
        &self,
        address: u64
    ) -> anyhow::Result<i64> {
        let mut buf = [0u8; 8];

        #[cfg(feature = "unicorn_backend")]
        if let Backend::Unicorn(unicorn) = self {
            unicorn.mem_read(address, &mut buf)
                .map_err(|e| anyhow!("mem_read_i64 failed: {:?}", e))?;
            return Ok(i64::from_le_bytes(buf))
        }

        #[cfg(feature = "dynarmic_backend")]
        if let Backend::Dynarmic(dynarmic) = self {
            dynarmic.mem_read(address, &mut buf)?;
            return Ok(i64::from_le_bytes(buf))
        }

        unreachable!()
    }

    #[inline]
    pub fn mem_read_u64(
        &self,
        address: u64
    ) -> anyhow::Result<u64> {
        let mut buf = [0u8; 8];

        #[cfg(feature = "unicorn_backend")]
        if let Backend::Unicorn(unicorn) = self {
            unicorn.mem_read(address, &mut buf)
                .map_err(|e| anyhow!("mem_read_u64 failed: {:?}", e))?;
            return Ok(u64::from_le_bytes(buf))
        }

        #[cfg(feature = "dynarmic_backend")]
        if let Backend::Dynarmic(dynarmic) = self {
            dynarmic.mem_read(address, &mut buf)?;
            return Ok(u64::from_le_bytes(buf))
        }

        unreachable!()
    }

    #[inline]
    pub fn mem_read_i32(
        &self,
        address: u64
    ) -> anyhow::Result<i32> {
        let mut buf = [0u8; 4];

        #[cfg(feature = "unicorn_backend")]
        if let Backend::Unicorn(unicorn) = self {
            unicorn.mem_read(address, &mut buf)
                .map_err(|e| anyhow!("mem_read_i64 failed: {:?}", e))?;
            return Ok(i32::from_le_bytes(buf))
        }

        #[cfg(feature = "dynarmic_backend")]
        if let Backend::Dynarmic(dynarmic) = self {
            dynarmic.mem_read(address, &mut buf)?;
            return Ok(i32::from_le_bytes(buf))
        }

        unreachable!()
    }

    #[inline]
    pub fn mem_read_as_vec(
        &self,
        address: u64,
        size: usize
    ) -> anyhow::Result<Vec<u8>> {
        #[cfg(feature = "unicorn_backend")]
        if let Backend::Unicorn(unicorn) = self {
            return unicorn.mem_read_as_vec(address, size)
                .map_err(|e| anyhow!("mem_read_as_vec failed: {:?}", e))
        }

        #[cfg(feature = "dynarmic_backend")]
        if let Backend::Dynarmic(dynarmic) = self {
            return dynarmic.mem_read_as_vec(address, size);
        }

        unreachable!()
    }

    #[inline]
    pub fn mem_unmap(
        &self,
        address: u64,
        size: usize
    ) -> anyhow::Result<()> {
        #[cfg(feature = "unicorn_backend")]
        if let Backend::Unicorn(unicorn) = self {
            return unicorn.mem_unmap(address, size)
                .map_err(|e| anyhow!("mem_unmap failed: {:?}", e))
        }

        #[cfg(feature = "dynarmic_backend")]
        if let Backend::Dynarmic(dynarmic) = self {
            return dynarmic.mem_unmap(address, size);
        }

        unreachable!()
    }

    #[inline]
    pub fn mem_protect(
        &self,
        address: u64,
        size: usize,
        perms: u32
    ) -> anyhow::Result<()> {
        #[cfg(feature = "unicorn_backend")]
        if let Backend::Unicorn(unicorn) = self {
            return unicorn.mem_protect(address, size, UnicornPermission::from_bits_truncate(perms))
                .map_err(|e| anyhow!("mem_protect failed: {:?}", e))
        }

        #[cfg(feature = "dynarmic_backend")]
        if let Backend::Dynarmic(dynarmic) = self {
            return dynarmic.mem_protect(address, size, perms);
        }

        unreachable!()
    }

    #[inline]
    pub fn context_restore(
        &self,
        context: &Context
    ) -> anyhow::Result<()> {
        #[cfg(feature = "unicorn_backend")]
        if let Backend::Unicorn(unicorn) = self {
            if let Context::Unicorn(context) = context {
                return unicorn.context_restore(context)
                    .map_err(|e| anyhow!("context_restore failed: {:?}", e))
            }
        }

        #[cfg(feature = "dynarmic_backend")]
        if let Backend::Dynarmic(dynarmic) = self {
            if let Context::Dynarmic(context) = context {
                dynarmic.context_restore(context)?;
                return Ok(())
            }
        }

        unreachable!()
    }

    #[inline]
    pub fn context_alloc(
        &self
    ) -> anyhow::Result<Context> {
        #[cfg(feature = "unicorn_backend")]
        if let Backend::Unicorn(unicorn) = self {
            let uc = unicorn.context_alloc().map_err(|e| anyhow!("context_alloc failed: {:?}", e))?;
            return Ok(Context::Unicorn(uc));
        }

        #[cfg(feature = "dynarmic_backend")]
        if let Backend::Dynarmic(dynarmic) = self {
            let context = dynarmic.context_alloc();
            return Ok(Context::Dynarmic(context));
        }

        unreachable!()
    }

    #[inline]
    pub fn context_save(
        &self,
        context: &mut Context
    ) -> anyhow::Result<()> {
        #[cfg(feature = "unicorn_backend")]
        if let Backend::Unicorn(unicorn) = self {
            if let Context::Unicorn(context) = context {
                return unicorn.context_save(context)
                    .map_err(|e| anyhow!("context_save failed: {:?}", e))
            }
        }

        #[cfg(feature = "dynarmic_backend")]
        if let Backend::Dynarmic(dynarmic) = self {
            if let Context::Dynarmic(context) = context {
                return dynarmic.context_save(context);
            }
        }

        unreachable!()
    }

    #[inline]
    pub(crate) fn emu_stop(
        &self,
        status: TaskStatus,
        emulator: &AndroidEmulator<T>
    ) -> anyhow::Result<()> {
        if option_env!("EMU_LOG") == Some("1") {
            warn!("emu_stop: status: {:?}", status);
        }

        emulator.set_task_status(status)?;
        emulator.inner_mut()
            .running.store(false, Ordering::SeqCst);

        #[cfg(feature = "unicorn_backend")]
        if let Backend::Unicorn(unicorn) = self {
            return unicorn.emu_stop()
                .map_err(|e| anyhow!("emu_stop failed: {:?}", e))
        }

        #[cfg(feature = "dynarmic_backend")]
        if let Backend::Dynarmic(dynarmic) = self {
            return dynarmic.emu_stop();
        }

        unreachable!()
    }
}