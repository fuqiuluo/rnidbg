//__system_property_get
//__system_property_find
//__system_property_read

use std::marker::PhantomData;
use std::rc::Rc;
use log::info;
use crate::emulator::{AndroidEmulator, VMPointer};
use crate::memory::svc_memory::{HookListener, SvcMemory};

pub(super) mod system_properties;
mod memory;
mod string;

pub struct Libc<'a, T> {
    system_property_service: Option<Rc<Box<dyn Fn(&str) -> Option<String>>>>,



    pd: PhantomData<&'a T>,
}

impl<T: Clone> Libc<'_, T> {
    pub fn new<'a>() -> Libc<'a, T> {
        Libc {
            system_property_service: None,
            pd: PhantomData
        }
    }

    pub fn set_system_property_service(&mut self, service: Rc<Box<dyn Fn(&str) -> Option<String>>>) {
        self.system_property_service = Some(service);
    }
}

impl<'a, T: Clone> HookListener<'a, T> for Libc<'a, T> {
    fn hook(&self, emu: &AndroidEmulator<'a, T>, lib_name: String, symbol_name: String, old: u64) -> u64 {
        if lib_name != "libc.so" {
            return 0
        }
        if option_env!("SHOW_LIBC_TRY_LINK") == Some("1") {
            info!("[libc.so] link {}, old=0x{:X}", symbol_name, old)
        }
        let svc = &mut emu.inner_mut().svc_memory;
        let entry = match symbol_name.as_str() {
            "__system_property_get" => svc.register_svc(Box::new(system_properties::SystemPropertyGet::new(self.system_property_service.clone()))),
            "__system_property_find" => svc.register_svc(Box::new(system_properties::SystemPropertyFind::new(self.system_property_service.clone()))),
            "__system_property_read" => svc.register_svc(Box::new(system_properties::SystemPropertyRead::new(self.system_property_service.clone()))),
            "strcmp" => svc.register_svc(Box::new(string::StrCmp)),
            "strncmp" => svc.register_svc(Box::new(string::StrNCmp)),
            "strcasecmp" => svc.register_svc(Box::new(string::StrCaseCmp)),
            "strncasecmp" => svc.register_svc(Box::new(string::StrNCasCmp)),
            _ => 0
        };


        entry
    }
}