use crate::emulator::AndroidEmulator;
use crate::pointer::VMPointer;
use anyhow::anyhow;
use log::error;
use std::marker::PhantomData;

#[derive(Clone)]
pub(crate) enum InitFunction<'a, T: Clone> {
    ABSOLUTE(AbsoluteInitFunction<'a, T>),
    LINUX(LinuxInitFunction<'a, T>),
}

pub(crate) trait InitFunctionTrait<'a, T: Clone> {
    fn addr(&self) -> u64;

    fn call(&self, emu: AndroidEmulator<'a, T>) -> anyhow::Result<u64>;
}

#[derive(Clone)]
pub(crate) struct AbsoluteInitFunction<'a, T: Clone> {
    pub load_base: u64,
    pub lib_name: String,
    pub addr: u64,
    pub ptr: VMPointer<'a, T>,
}

impl<'a, T: Clone> AbsoluteInitFunction<'a, T> {
    pub fn new(
        load_base: u64,
        lib_name: String,
        ptr: VMPointer<'a, T>,
    ) -> anyhow::Result<AbsoluteInitFunction<'a, T>> {
        let addr = ptr.read_u64_with_offset(0)?;
        Ok(Self {
            load_base,
            lib_name,
            addr,
            ptr,
        })
    }
}

impl<'a, T: Clone> InitFunctionTrait<'a, T> for AbsoluteInitFunction<'a, T> {
    fn addr(&self) -> u64 {
        self.addr
    }

    fn call(&self, emu: AndroidEmulator<'a, T>) -> anyhow::Result<u64> {
        let mut address = self.ptr.read_u64_with_offset(0)?;
        if address == 0 {
            address = self.addr;
        }

        if address == 0 {
            error!(
                "[{}] CallInitFunction: address=0x{:X}, ptr={:X}, func={:X}",
                self.lib_name,
                address,
                self.ptr.addr,
                self.ptr.read_i64_with_offset(0)?
            );
            return Ok(address);
        }

        //let pointer = VMPointer::new(address, 0, emu.backend.clone());
        //info!("[{}] CallInitFunction: addr=0x{:X}, offset=0x{:X}", self.lib_name, pointer.addr, address - self.load_base);

        emu.e_func(address, vec![])
            .ok_or(anyhow!("failed to call init function"))

        //info!("CallInitFunction: addr=0x{:X}", address);
    }
}

#[derive(Clone)]
pub(crate) struct LinuxInitFunction<'a, T: Clone> {
    pub load_base: u64,
    #[allow(unused)]
    pub lib_name: String,
    pub addr: u64,
    pd: PhantomData<&'a T>,
}

impl<'a, T: Clone> LinuxInitFunction<'a, T> {
    pub fn new(load_base: u64, lib_name: String, addr: u64) -> LinuxInitFunction<'a, T> {
        Self {
            load_base,
            lib_name,
            addr,
            pd: PhantomData,
        }
    }
}

impl<'a, T: Clone> InitFunctionTrait<'a, T> for LinuxInitFunction<'a, T> {
    fn addr(&self) -> u64 {
        self.addr + self.load_base
    }

    fn call(&self, emu: AndroidEmulator<'a, T>) -> anyhow::Result<u64> {
        if self.addr == 0 {
            //debug!("[{}] CallInitFunction: address=0x{:X}", self.lib_name, self.addr);
            return Ok(self.addr);
        }

        //debug!("[{}] CallInitFunction: addr=0x{:X}", self.lib_name, self.addr);

        emu.e_func(self.addr(), vec![]);

        Ok(self.addr)
    }
}
