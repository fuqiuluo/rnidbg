
use std::collections::HashMap;
use std::convert::Infallible;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use anyhow::anyhow;
use bytes::{BufMut, BytesMut};
use log::{debug, info};
use crate::backend::{Backend, Permission};
use crate::emulator::{AndroidEmulator, VMPointer, SVC_BASE, SVC_MAX, SVC_SIZE};
use crate::tool::align_size;

#[repr(C)]
#[derive(Debug, Clone)]
pub struct SvcMemRegion {
    pub virtual_address: u64,
    pub begin: u64,
    pub end: u64,
    pub perms: Permission,
    pub label: String,
    pub library_file_path: Option<String>,
    pub offset: u64
}

pub struct SvcMemory<'a, T: Clone> {
    base: VMPointer<'a, T>,
    mem_region: Vec<SvcMemRegion>,
    arm_svc_number: u32,
    svc_map: HashMap<u32, Box<dyn Arm64Svc<T> + 'a>>
}

impl<'a, T: Clone> SvcMemory<'a, T> {
    pub(crate) fn get_svc(&self, number: u32) -> Option<&Box<dyn Arm64Svc<T> + 'a>> {
        self.svc_map.get(&number)
    }
}

impl<'a, T: Clone> SvcMemory<'a, T> {
    pub fn new(backend: &Backend<'a, T>) -> anyhow::Result<SvcMemory<'a, T>> {
        backend.mem_map(SVC_BASE, SVC_SIZE, (Permission::READ | Permission::EXEC).bits())
            .map_err(|e| anyhow!("init svc failed: {:?}", e))?;
        Ok(SvcMemory {
            base: VMPointer::new(SVC_BASE, SVC_SIZE, backend.clone()),
            mem_region: Vec::new(),
            arm_svc_number: 0x200, // 避免占用系统调用
            svc_map: HashMap::new()
        })
    }

    pub fn register_svc(&mut self, svc_box: Box<dyn Arm64Svc<T> + 'a>) -> u64 {
        if option_env!("PRINT_SVC_REGISTER").unwrap_or("") == "1" {
            debug!("register_svc: name={}, svc_number=0x{:x}", &svc_box.name(), self.arm_svc_number + 1);
        }
        let pointer = unsafe {
            let number = {
                self.arm_svc_number += 1;
                self.arm_svc_number
            };
            let pointer = svc_box.on_register(self, number);

            self.svc_map.insert(number, svc_box);
            pointer
        };
        pointer
    }

    pub fn allocate(&mut self, size: usize, label: &str) -> VMPointer<'a, T> {
        let size = align_size(size);
        let mut pointer = self.base.share(0);

        if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
            debug!("svc allocate: base=0x{:X}, size={}, label={}", pointer.addr, size, label);
        }

        self.base = pointer.share(size as i64);
        pointer.size = size;

        self.mem_region.push(SvcMemRegion {
            virtual_address: pointer.addr,
            begin: pointer.addr,
            end: pointer.addr + size as u64,
            perms: Permission::READ | Permission::EXEC,
            label: label.to_string(),
            offset: 0,
            library_file_path: None
        });

        pointer
    }
}

pub fn assemble_svc(number: u32) -> u32 {
    if number >= 0 && number < SVC_MAX - 1 {
        0xd4000001 | (number << 5)
    } else {
        panic!("svc_number out of range: {}", number)
    }
}

pub trait HookListener<'a, T: Clone> {
    // long hook(SvcMemory svcMemory, String libraryName, String symbolName, long old);
    // HookListener 返回0表示没有hook，否则返回hook以后的调用地址
    fn hook(&self, emu: &AndroidEmulator<'a, T>, lib_name: String, symbol_name: String, old: u64) -> u64;
}

pub trait Arm64Svc<T: Clone> {
    fn name(&self) -> &str;

    /// 通过SVC实现JNI函数的调用
    fn on_register(&self, svc: &mut SvcMemory<T>, number: u32) -> u64 {
        let mut buf = BytesMut::new();
        buf.put_u32_le(assemble_svc(number)); // "svc #0x" + svcNumber
        buf.put_u32_le(0xd65f03c0); // ret

        {
            let ptr = svc.allocate(buf.len(), format!("Arm64Svc.{}", self.name()).as_str());
            ptr.write_bytes(buf.freeze()).unwrap();
            return ptr.addr;
        }
        let ptr = svc.allocate(buf.len(), "Arm64Svc");
        ptr.write_bytes(buf.freeze()).unwrap();
        ptr.addr
    }

    fn handle(&self, emu: &AndroidEmulator<T>) -> SvcCallResult;

    fn on_post_callback(&self, emu: &AndroidEmulator<T>) -> u64 {
        0
    }

    fn on_pre_callback(&self, emu: &AndroidEmulator<T>) -> u64 {
        0
    }
}

pub struct SimpleArm64Svc<T: Clone> {
    pub name: String,
    pub handle: fn(name: &str, &AndroidEmulator<T>) -> SvcCallResult
}

impl<T: Clone> Arm64Svc<T> for SimpleArm64Svc<T> {
    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn handle(&self, emu: &AndroidEmulator<T>) -> SvcCallResult {
        (self.handle)(self.name.as_str(), emu)
    }
}

#[derive(Debug)]
pub enum SvcCallResult {
    VOID,
    FUCK(anyhow::Error),
    RET(i64)
}

impl SvcCallResult {
    fn is_ok(&self) -> bool {
        !matches!(self, SvcCallResult::FUCK(_))
    }

    fn is_err(&self) -> bool {
        matches!(self, SvcCallResult::FUCK(_))
    }
}
//
// impl Try for SvcCallResult {
//     type Output = i64;
//     type Residual = SvcCallResult;
//
//     fn from_output(output: Self::Output) -> Self {
//         SvcCallResult::RET(output)
//     }
//
//     fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
//         match self {
//             SvcCallResult::RET(val) => ControlFlow::Continue(val),
//             SvcCallResult::FUCK(err) => ControlFlow::Break(SvcCallResult::FUCK(err)),
//             SvcCallResult::VOID => ControlFlow::Break(SvcCallResult::VOID),
//         }
//     }
// }
//
// impl FromResidual for SvcCallResult {
//     fn from_residual(residual: Self::Residual) -> Self {
//         residual
//     }
// }
//
// impl FromResidual<Result<Infallible, anyhow::Error>> for SvcCallResult {
//     fn from_residual(residual: Result<Infallible, anyhow::Error>) -> Self {
//         match residual {
//             Err(err) => SvcCallResult::FUCK(err),
//             _ => unreachable!(),
//         }
//     }
// }

impl<T: Clone> SimpleArm64Svc<T> {
    pub fn new(name: &str, handle: fn(&str, &AndroidEmulator<T>) -> SvcCallResult) -> Box<SimpleArm64Svc<T>> {
        Box::new(SimpleArm64Svc {
            name: name.to_string(),
            handle
        })
    }
}