// https://android.googlesource.com/platform/bionic/+/0d787c1fa18c6a1f29ef9840e28a68cf077be1de/libc/bionic/system_properties.c

use std::rc::Rc;
use crate::backend::RegisterARM64;
use crate::emulator::AndroidEmulator;
use crate::linux::structs::PropInfo;
use crate::memory::svc_memory::Arm64Svc;

pub(super) struct SystemPropertyGet(Option<Rc<Box<dyn Fn(&str) -> Option<String>>>>);
pub(super) struct SystemPropertyFind(Option<Rc<Box<dyn Fn(&str) -> Option<String>>>>);
pub(super) struct SystemPropertyRead(Option<Rc<Box<dyn Fn(&str) -> Option<String>>>>);

impl SystemPropertyGet {
    pub fn new(
        service: Option<Rc<Box<dyn Fn(&str) -> Option<String>>>>
    ) -> Self {
        SystemPropertyGet(service)
    }
}

impl SystemPropertyFind {
    pub fn new(
        service: Option<Rc<Box<dyn Fn(&str) -> Option<String>>>>
    ) -> Self {
        SystemPropertyFind(service)
    }
}

impl SystemPropertyRead {
    pub fn new(
        service: Option<Rc<Box<dyn Fn(&str) -> Option<String>>>>
    ) -> Self {
        SystemPropertyRead(service)
    }
}

impl<T: Clone> Arm64Svc<T> for SystemPropertyGet {
    #[cfg(feature = "show_svc_name")]
    fn name(&self) -> &str { "SystemPropertyGet" }

    fn handle(&self, emu: &AndroidEmulator<T>) -> anyhow::Result<Option<i64>> {
        let backend = &emu.backend;
        let name = backend.mem_read_c_string(backend.reg_read(RegisterARM64::X0)?)?;
        let value = backend.reg_read(RegisterARM64::X1)?;

        if option_env!("PRINT_SYSTEM_PROP_LOG") == Some("1") {
            println!("__system_property_get({}, 0x{:X})", name, value);
        }

        if name == "ro.kernel.qemu" || name == "libc.debug.malloc" {
            backend.mem_write(value, b"\0")?;
            return Ok(Some(0))
        }
        if self.0.is_none() {
            panic!("SystemPropertyGet not supported: name={}, msg=system_property_service not set", name);
        }
        match self.0.as_ref().unwrap()(&name) {
            Some(env) => {
                backend.mem_write(value, env.as_bytes())?;
            }
            None => {
                backend.mem_write(value, b"\0")?;
            }
        }
        Ok(Some(0))
    }
}

impl<T: Clone> Arm64Svc<T> for SystemPropertyFind {
    #[cfg(feature = "show_svc_name")]
    fn name(&self) -> &str { "SystemPropertyFind" }

    fn handle(&self, emu: &AndroidEmulator<T>) -> anyhow::Result<Option<i64>> {
        let backend = &emu.backend;
        let name = backend.mem_read_c_string(backend.reg_read(RegisterARM64::X0)?)?;

        if option_env!("PRINT_SYSTEM_PROP_LOG") == Some("1") {
            println!("__system_property_find({})", name);
        }
        
        if name == "debug.atrace.tags.enableflags" {
            return Ok(Some(0))
        }

        if self.0.is_none() {
            panic!("SystemPropertyFind not supported: name={}, msg=system_property_service not set", name);
        }
        match self.0.as_ref().unwrap()(&name) {
            Some(env) => {
                let mut buf = [0u8; size_of::<PropInfo>()];
                let mut prop_info = unsafe { &mut *(buf.as_mut_ptr() as *mut PropInfo) };
                prop_info.name[..name.len()].copy_from_slice(name.as_bytes());
                prop_info.name[name.len()] = 0;
                prop_info.serial = 0;
                prop_info.value[..env.len()].copy_from_slice(env.as_bytes());
                prop_info.value[env.len()] = 0;
                let pointer = emu.falloc(buf.len(), true)?;
                pointer.write_data(&buf)?;
                Ok(Some(pointer.addr as i64))
            }
            None =>  Ok(Some(0))
        }
    }
}

impl<T: Clone> Arm64Svc<T> for SystemPropertyRead {
    #[cfg(feature = "show_svc_name")]
    fn name(&self) -> &str { "SystemPropertyRead" }

    fn handle(&self, emu: &AndroidEmulator<T>) -> anyhow::Result<Option<i64>> {
        panic!("SystemPropertyRead not supported");
    }
}