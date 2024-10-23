// https://android.googlesource.com/platform/bionic/+/0d787c1fa18c6a1f29ef9840e28a68cf077be1de/libc/bionic/system_properties.c

use std::rc::Rc;
use anyhow::anyhow;
use log::{debug, error};
use crate::backend::RegisterARM64;
use crate::emulator::AndroidEmulator;
use crate::linux::structs::PropInfo;
use crate::memory::svc_memory::{Arm64Svc, SvcCallResult};
use crate::memory::svc_memory::SvcCallResult::{FUCK, RET};

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
    fn name(&self) -> &str { "SystemPropertyGet" }

    fn handle(&self, emu: &AndroidEmulator<T>) -> SvcCallResult {
        let backend = &emu.backend;
        let Ok(name_pointer) = backend.reg_read(RegisterARM64::X0) else {
            return FUCK(anyhow!("unable to get name_pointer"))
        };
        let Ok(name) = backend.mem_read_c_string(name_pointer) else {
            return FUCK(anyhow!("unable to read name from name pointer: 0x{:X}", name_pointer))
        };
        let Ok(value) = backend.reg_read(RegisterARM64::X1) else {
            return FUCK(anyhow!("unable to get value when handle SystemPropGet"))
        };

        if option_env!("PRINT_SYSTEM_PROP_LOG") == Some("1") {
            debug!("__system_property_get({}, 0x{:X})", name, value);
        }

        if name == "ro.kernel.qemu" || name == "libc.debug.malloc" {
            if let Err(e) = backend.mem_write(value, b"\0") {
                return FUCK(anyhow!("unable to write_mem(x1): {}", e))
            }
            return RET(0)
        }
        if self.0.is_none() {
            panic!("SystemPropertyGet not supported: name={}, msg=system_property_service not set", name);
        }
        match self.0.as_ref().unwrap()(&name) {
            Some(env) => {
                if let Err(e) = backend.mem_write(value, env.as_bytes()) {
                    return FUCK(anyhow!("unable to write_mem when handle SystemPropGet: {}", e))
                }
            }
            None => {
                if let Err(e) = backend.mem_write(value, b"\0") {
                    return FUCK(anyhow!("unable to write_mem when handle SystemPropGet: {}", e))
                }
            }
        }
        RET(0)
    }
}

impl<T: Clone> Arm64Svc<T> for SystemPropertyFind {
    fn name(&self) -> &str { "SystemPropertyFind" }

    fn handle(&self, emu: &AndroidEmulator<T>) -> SvcCallResult {
        let backend = &emu.backend;
        let Ok(name_pointer) = backend.reg_read(RegisterARM64::X0) else {
            return FUCK(anyhow!("unable to get name_pointer"))
        };
        let Ok(name) = backend.mem_read_c_string(name_pointer) else {
            return FUCK(anyhow!("unable to read name from name pointer: 0x{:X}", name_pointer))
        };

        if option_env!("PRINT_SYSTEM_PROP_LOG") == Some("1") {
            debug!("__system_property_find({})", name);
        }
        
        if name == "debug.atrace.tags.enableflags" {
            return RET(0)
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
                let Ok(pointer) = emu.falloc(buf.len(), true) else {
                  return FUCK(anyhow!("unable to alloc memory for prop_info"))
                };
                if let Err(e) = pointer.write_data(&buf) {
                    return FUCK(anyhow!("unable to write prop_info: {}", e))
                }
                RET(pointer.addr as i64)
            }
            None =>  RET(0)
        }
    }
}

impl<T: Clone> Arm64Svc<T> for SystemPropertyRead {
    fn name(&self) -> &str { "SystemPropertyRead" }

    fn handle(&self, emu: &AndroidEmulator<T>) -> SvcCallResult {
        panic!("SystemPropertyRead not supported");
    }
}