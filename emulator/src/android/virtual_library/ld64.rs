use std::collections::HashMap;
use std::mem;
use anyhow::anyhow;
use bytes::{Buf, BufMut, BytesMut};
use log::{error, info};
use crate::backend::RegisterARM64;
use crate::emulator::{AndroidEmulator, POST_CALLBACK_SYSCALL_NUMBER, VMPointer};
use crate::keystone;
use crate::emulator::memory::MemoryBlockTrait;
use crate::linux::errno::Errno;
use crate::linux::PAGE_ALIGN;
use crate::linux::structs::DlInfo;
use crate::memory::svc_memory::{Arm64Svc, assemble_svc, HookListener, SvcMemory};

struct DlIteratePhdr;
struct DlClose<'a, T: Clone>(pub VMPointer<'a, T>);
struct DlError<'a, T: Clone>(pub VMPointer<'a, T>);
struct DlOpen<'a, T: Clone>(pub VMPointer<'a, T>);
struct DlAddr;
struct DlSym;
struct DlUnwindFindExidx;

pub struct ArmLD64<'a, T: Clone> {
    error: VMPointer<'a, T>,
}

impl<T: Clone> ArmLD64<'_, T> {
    pub fn new<'a>(svc_memory: &mut SvcMemory<'a, T>) -> anyhow::Result<ArmLD64<'a, T>> {
        let pointer = svc_memory.allocate(0x80, "Dlfcn.error");
        Ok(ArmLD64 {
            error: pointer,
        })
    }
}

impl<'a, T: Clone> HookListener<'a, T> for ArmLD64<'a, T> {
    fn hook(&self, emu: &AndroidEmulator<'a, T>, lib_name: String, symbol_name: String, old: u64) -> u64 {
        if lib_name != "libdl.so" {
            return 0;
        }
        info!("[libdl.so] link {}, old=0x{:X}", symbol_name, old);
        let svc = &mut emu.inner_mut().svc_memory;
        match symbol_name.as_str() {
            "dl_iterate_phdr" => svc.register_svc(Box::new(DlIteratePhdr)) ,
            "dlerror" => svc.register_svc(Box::new(DlError(self.error.clone()))),
            "dlclose" => svc.register_svc(Box::new(DlClose(self.error.clone()))),
            "dlopen" => svc.register_svc(Box::new(DlOpen(self.error.clone()))),
            "dladdr" => svc.register_svc(Box::new(DlAddr)),
            "dlsym" => svc.register_svc(Box::new(DlSym)),
            "dl_unwind_find_exidx" => svc.register_svc(Box::new(DlUnwindFindExidx)),
            _ => panic!("[libdl] symbol not found: {}", symbol_name)
        }
    }
}

impl<T: Clone> Arm64Svc<T> for DlIteratePhdr {
    #[cfg(feature = "show_svc_name")]
    fn name(&self) -> &str {
        "DlIteratePhdr"
    }

    fn on_register(&self, svc: &mut SvcMemory<T>, number: u32) -> u64 {
        let code = [
            "sub sp, sp, #0x10",
            "stp x29, x30, [sp]",
            &format!("svc #0x{:x}", number),
            "ldr x13, [sp]", // x13 == callback 0xc
            "add sp, sp, #0x8", // pop callback
            "cmp x13, #0", // if callback == 0
            "b.eq #0x58", // 0x58
            "ldr x0, [sp]", // x0 == ptr
            "add sp, sp, #0x8", // pop ptr
            "ldr x1, [sp]", // x1 == size
            "add sp, sp, #0x8", // pop size
            "ldr x2, [sp]", // x2 == data
            "add sp, sp, #0x8", // pop data
            "blr x13", // callback(ptr, size, data)
                        // int (*callback)(struct dl_phdr_info *info,
                        //        size_t size, void *data)
            "cmp x0, #0", // if callback return 0
            "b.eq #0xc", // loop
            "ldr x13, [sp]", // 0x40
            "add sp, sp, #0x8",
            "cmp x13, #0",
            "b.eq #0x58", // 0x58
            "add sp, sp, #0x18",
            "b 0x40",
            "mov x8, #0", // 0x58
            &format!("mov x12, #0x{:x}", number),
            &format!("mov x16, #0x{:x}", POST_CALLBACK_SYSCALL_NUMBER),
            "svc #0",
            "ldp x29, x30, [sp]",
            "add sp, sp, #0x10",
            "ret"
        ];
        let code = code.join("\n");
        let code = keystone::assemble_no_check(&code);
        let pointer = svc.allocate(code.len(), "DlIteratePhdr");
        pointer.write_buf(code)
            .expect("try register svc");
        info!("DlIteratePhdr: pointer={:X}", pointer.addr);
        pointer.addr
    }

    fn handle(&self, emu: &AndroidEmulator<T>) -> anyhow::Result<Option<i64>> {
        let cb = emu.backend.reg_read(RegisterARM64::X0)?;
        let data = emu.backend.reg_read(RegisterARM64::X1)?;

        let mut modules = emu.inner_mut()
            .memory
            .modules
            .iter()
            .filter(|(name, module)| {
                unsafe { (&*module.get()).elf_file.is_some() }
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|(a, b)| b.clone())
            .rev()
            .collect::<Vec<_>>();

        let mut modules = modules.iter().map(|module_cell| {
            let module = unsafe { &mut *module_cell.get() };
            let elf_file = unsafe { &*module.elf_file.as_ref().unwrap().get() };
            (module.path(emu), module.virtual_base, elf_file.ph_offset, elf_file.num_ph)
        }).collect::<Vec<_>>();

        modules.push(("/apex/com.android.art/lib64/libart.so".to_string(), 0, 0, 0));

        let size = 64;
        let modules_len = modules.len();

        let mut ptr = emu.falloc(size * modules_len, true)?;
        let sp = emu.backend.reg_read(RegisterARM64::SP)
            .map_err(|e| anyhow!("failed to read SP: {:?}", e))?;

        if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
            println!("DlIteratePhdr cb={:X}, data={:X}, size={}, sp={:X}", cb, data, modules_len, sp);
        }

        let sp = VMPointer::new(sp, 0, emu.backend.clone());
        let mut sp = sp.share(-8);
        sp.write_u64(0)?; // NULL-terminated

        for (name, vaddr, phdr, phnum) in modules {
            info!("DlIteratePhdr: name={}, vaddr={:X}, phdr={:X}, phnum={}", name, vaddr, phdr, phnum);

            let dlpi_name = emu.falloc(name.len() + 1, true)?;
            dlpi_name.write_c_string(name.as_str())?;
            ptr.write_u64_with_offset(0, vaddr)?;
            ptr.write_u64_with_offset(8, dlpi_name.addr)?;
            ptr.write_u64_with_offset(16, phdr as u64)?;
            ptr.write_u16_with_offset(24, phnum as u16)?;

            sp = sp.share(-8);
            sp.write_u64(data)?;

            sp = sp.share(-8);
            sp.write_u64(size as u64)?;

            sp = sp.share(-8);
            sp.write_u64(ptr.addr)?;

            sp = sp.share(-8);
            sp.write_u64(cb)?;

            ptr = ptr.share(size as i64);
        }

        emu.backend.reg_write(RegisterARM64::SP, sp.addr)
            .map_err(|e| anyhow!("failed to write SP: {:?}", e))?;

       Ok(None)
    }

    fn on_post_callback(&self, emu: &AndroidEmulator<T>) -> u64 {
        0
    }
}

impl<T: Clone> Arm64Svc<T> for DlError<'_, T> {
    #[cfg(feature = "show_svc_name")]
    fn name(&self) -> &str {
        "DlError"
    }

    fn handle(&self, emu: &AndroidEmulator<T>) -> anyhow::Result<Option<i64>> {
        panic!("dlerror not supported");
    }
}

impl<T: Clone> Arm64Svc<T> for DlClose<'_, T> {
    #[cfg(feature = "show_svc_name")]
    fn name(&self) -> &str {
        "DlClose"
    }

    fn handle(&self, emu: &AndroidEmulator<T>) -> anyhow::Result<Option<i64>> {
        panic!("dlclose not supported")
    }
}

impl<T: Clone> Arm64Svc<T> for DlOpen<'_, T> {
    #[cfg(feature = "show_svc_name")]
    fn name(&self) -> &str {
        "DlOpen"
    }

    fn on_register(&self, svc: &mut SvcMemory<T>, number: u32) -> u64 {
        let mut buf = BytesMut::new();
        buf.put_u32_le(0xd10043ff);// "sub sp, sp, #0x10"
        buf.put_u32_le(0xa9007bfd);// "stp x29, x30, [sp]"
        buf.put_u32_le(assemble_svc(number));// "svc #0x" + Integer.toHexString(svcNumber)
        buf.put_u32_le(0xf94003ed);// "ldr x13, [sp]"
        buf.put_u32_le(0x910023ff);// "add sp, sp, #0x8", manipulated stack in dlopen
        buf.put_u32_le(0xf10001bf);// "cmp x13, #0"
        buf.put_u32_le(0x54000060);// "b.eq #0x24"
        buf.put_u32_le(0x10ffff9e);// "adr lr, #-0xf", jump to ldr x13, [sp]
        buf.put_u32_le(0xd61f01a0);// "br x13", call init array // "b.eq #0x24" to here
        buf.put_u32_le(0xf94003e0);// "ldr x0, [sp]", with return address
        buf.put_u32_le(0x910023ff);// "add sp, sp, #0x8"
        buf.put_u32_le(0xa9407bfd);// "ldp x29, x30, [sp]"
        buf.put_u32_le(0x910043ff);// "add sp, sp, #0x10"
        buf.put_u32_le(0xd65f03c0);// "ret"
        let pointer = svc.allocate(buf.len(), "dlopen");
        pointer.write_bytes(buf.freeze())
            .expect("try register svc failed");
        pointer.addr
    }

    fn handle(&self, emu: &AndroidEmulator<T>) -> anyhow::Result<Option<i64>> {
        let file_name_ptr = VMPointer::new(emu.backend.reg_read(RegisterARM64::X0)?, 0, emu.backend.clone());

        let flags = emu.backend.reg_read(RegisterARM64::X1)?;
        let file_name = file_name_ptr.read_string()?;

        let pointer = VMPointer::new(emu.backend.reg_read(RegisterARM64::SP)?, 0, emu.backend.clone());
        let pointer = pointer.share_with_size(-8, 0); // ret

        if !file_name.is_ascii() {
            if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
                println!("syscall dlopen(file_name=hex::decode({}), flags=0x{:X}) => 0", hex::encode(file_name.as_bytes()), flags);
            }

            pointer.write_u64(0)?; // dlopen函数调用返回值
            let pointer = pointer.share_with_size(-8, 0);
            pointer.write_u64(0)?;
            emu.set_errno(Errno::ENOENT.as_i32())?;
            emu.backend.reg_write(RegisterARM64::SP, pointer.addr)?;

            return Ok(Some(0));
        } else {
            if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
                println!("syscall dlopen(file_name={}, flags=0x{:X})", file_name, flags);
            }
        }

        if file_name == "libnetd_client.so" {
            pointer.write_u64(0)?; // dlopen函数调用返回值
            let pointer = pointer.share_with_size(-8, 0);
            pointer.write_u64(0)?;
            if pointer.addr <= 0 {
                panic!("dlopen failed");
            }
            emu.backend.reg_write(RegisterARM64::SP, pointer.addr)?;
            return Ok(Some(0));
        } else {
            panic!("dlopen not supported");
        }

        Err(anyhow!("dlopen not supported"))
    }
}

impl<T: Clone> Arm64Svc<T> for DlAddr {
    #[cfg(feature = "show_svc_name")]
    fn name(&self) -> &str {
        "DlAddr"
    }

    fn handle(&self, emu: &AndroidEmulator<T>) -> anyhow::Result<Option<i64>> {
        let addr = emu.backend.reg_read(RegisterARM64::X0)?;
        let info_ptr = emu.backend.reg_read(RegisterARM64::X1)?;

        let module = emu.inner_mut().memory.find_module_by_address(addr);
        if let Some(module) = module {
            let module = unsafe { &*module.get() };

            const INFO_SIZE: usize = size_of::<DlInfo>();
            let symbol = module.find_symbol_by_closest_addr(addr);
            return if let Ok(symbol) = symbol {
                let path = &module.path(emu);
                //let path = path.split('/').last().unwrap();
                let path_ptr = emu.falloc(path.len() + 1 + symbol.name.len() + 1, true)?;
                path_ptr.write_c_string(path)?;
                let sname_ptr = path_ptr.share((path.len() + 1) as i64);
                sname_ptr.write_c_string(symbol.name.as_str())?;

                let mut buffer = [0u8; INFO_SIZE];
                let info = unsafe { &mut *(buffer.as_mut_ptr() as *mut DlInfo) };
                info.dli_fname = path_ptr.addr;
                info.dli_fbase = module.virtual_base;
                info.dli_sname = sname_ptr.addr;
                info.dli_saddr = symbol.address();

                emu.backend.mem_write(info_ptr, &buffer)?;

                if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
                    println!("syscall dladdr(addr=0x{:X}, info_ptr=0x{:X}) => Module(name={}, function)", addr, info_ptr, module.name);
                }

                Ok(Some(1))
            } else {
                let entry_point = module.entry_point;
                let path = module.path(emu);
                let path_ptr = emu.malloc(path.len() + 1 + 6, false)?.pointer;
                path_ptr.write_c_string(path.as_str())?;
                let sname_ptr = path_ptr.share((path.len() + 1) as i64);
                sname_ptr.write_c_string("start")?;

                let mut buffer = [0u8; INFO_SIZE];
                let info = unsafe { &mut *(buffer.as_mut_ptr() as *mut DlInfo) };
                info.dli_fname = path_ptr.addr;
                info.dli_fbase = module.virtual_base;
                info.dli_sname = sname_ptr.addr;
                info.dli_saddr = entry_point;
                emu.backend.mem_write(info_ptr, buffer.as_slice())?;

                if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
                    println!("syscall dladdr(addr=0x{:X}, info_ptr=0x{:X}, path={}) => Module(name={}, unk)", addr, info_ptr, path, module.name);
                }

                Ok(Some(1))
            }
        } else {
            if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
                println!("syscall dladdr(addr=0x{:X}, info_ptr=0x{:X}) => NotFound", addr, info_ptr);
            }
        }
        Ok(Some(0))
    }
}

impl<T: Clone> Arm64Svc<T> for DlSym {
    #[cfg(feature = "show_svc_name")]
    fn name(&self) -> &str {
        "DlSym"
    }

    fn handle(&self, emu: &AndroidEmulator<T>) -> anyhow::Result<Option<i64>> {
        panic!("dlsym not supported")
    }
}

impl<T: Clone> Arm64Svc<T> for DlUnwindFindExidx {
    #[cfg(feature = "show_svc_name")]
    fn name(&self) -> &str {
        "DlUnwindFindExidx"
    }

    fn handle(&self, emu: &AndroidEmulator<T>) -> anyhow::Result<Option<i64>> {
        panic!("DlUnwindFindExidx not supported")
    }
}
