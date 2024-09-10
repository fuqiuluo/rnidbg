mod tests;
mod allocator;

use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::process::id;
use anyhow::anyhow;
use bitflags::Flags;
use log::{info, warn};
use crate::backend::Permission;
use crate::emulator::{AndroidEmulator, syscall_handler};
use crate::emulator::consts::MMAP_BASE;
use crate::linux::errno::Errno;
use crate::linux::file_system::{FileIO, FileIOTrait, StMode};
use crate::linux::PAGE_ALIGN;
use crate::linux::structs::OFlag;
use crate::linux::symbol::{LinuxSymbol, ModuleSymbol};
use crate::memory::AndroidElfLoader;
use crate::pointer::VMPointer;
use crate::tool::{UnicornArg};

const MAP_FAILED: i64 = -1;
const MAP_FIXED: i32 = 0x10;
const MAP_ANONYMOUS: i32 = 0x20;

#[derive(Debug, Clone)]
pub struct MemoryMap {
    pub base: u64,
    pub size: usize,
    pub prot: u32,
    pub name: Option<String>,
    pub from_file: bool
}

impl MemoryMap {
    pub fn new(base: u64, size: usize, prot: u32) -> Self {
        Self {
            base,
            size,
            prot,
            name: None,
            from_file: false
        }
    }

    pub fn new_with_name(base: u64, size: usize, prot: u32, name: String) -> Self {
        Self {
            base,
            size,
            prot,
            name: Some(name),
            from_file: false
        }
    }

    pub fn new_from_file(base: u64, size: usize, prot: u32) -> Self {
        Self {
            base,
            size,
            prot,
            name: None,
            from_file: true
        }
    }
}

pub trait MemoryBlockTrait<'a, T: Clone> {
    fn pointer(&self) -> VMPointer<'a, T>;

    fn free(&self, emu: Option<AndroidEmulator<'a, T>>);
}

pub struct MemoryBlock<'a, T: Clone> {
    pub pointer: VMPointer<'a, T>,
    pub libc: bool,
    pub free: Option<LinuxSymbol>
}

impl<'a, T: Clone> MemoryBlockTrait<'a, T> for MemoryBlock<'a, T> {
    fn pointer(&self) -> VMPointer<'a, T> {
        self.pointer.clone()
    }

    fn free(&self, emu: Option<AndroidEmulator<'a, T>>) {
        if self.libc {
            let emu = emu.unwrap();
            if let Some(free) = &self.free {
                free.call(&emu, vec![UnicornArg::Ptr(self.pointer.addr)]);
            } else {
                warn!("free symbol not found")
            }
        } else {
            if let Some(emu) = emu {
                emu.munmap(self.pointer.addr, self.pointer.size as u64).unwrap();
            } else {
                warn!("free memory block failed: AndroidEmulator not found")
            }
        }
    }
}

impl<'a, T: Clone> AndroidEmulator<'a, T> {
    pub fn malloc(&self, size: usize, libc: bool) -> anyhow::Result<MemoryBlock<'a, T>> {
        if !libc {
            let ptr = self.falloc(size, false)?;
            Ok(MemoryBlock {
                pointer: ptr,
                libc: false,
                free: None
            })
        } else {
            let memory = &mut self.inner_mut().memory;
            let malloc = memory.malloc.as_ref().unwrap();
            let free = memory.free.as_ref().unwrap();
            let ptr = malloc.call(self, vec![UnicornArg::U64(size as u64)])
                .ok_or(anyhow!("malloc failed"))?;
            let ptr: VMPointer<'a, T> = VMPointer::new(ptr, size, self.backend.clone());
            Ok(MemoryBlock {
                pointer: ptr,
                libc: true,
                free: Some(free.clone())
            })
        }
    }

    pub fn falloc(&self, size: usize, temp_mem: bool) -> anyhow::Result<VMPointer<'a, T>> {
        let (pointer, size) = self.mmap(size, (Permission::READ | Permission::WRITE).bits())?;

        if temp_mem {
            self.inner_mut().memory.temp_memory.insert(pointer.addr, size);
        }

        Ok(pointer)
    }

    pub fn ffree(&self, addr: u64, size: usize) -> anyhow::Result<()> {
        let aligned = ((size - 1) / PAGE_ALIGN + 1) * PAGE_ALIGN;
        self.munmap(addr, aligned as u64)?;
        Ok(())
    }

    fn allocate_map_address(&self, mask: u64, length: usize) -> u64 {
        let mut last_entry = None;
        for (start, value) in self.inner_mut().memory.memory_map.iter() {
            if last_entry.is_none() {
                last_entry = Option::from(value.clone());
            }

            let end = value.base + value.size as u64;
            if (end + length as u64) < *start && (end & mask == 0) {
                return end;
            } else if let Some(map) = &last_entry {
                if map.base < value.base {
                    last_entry = Option::from(value.clone());
                } else if map.base == value.base && value.size > map.size {
                    last_entry = Option::from(value.clone());
                }
            }
        }

        if let Some(last) = last_entry {
            let mmap_address = last.base + last.size as u64;
            if mmap_address < self.inner_mut().memory.mmap_base {
                self.inner_mut().memory.set_mmap_base(mmap_address)
            }
        }

        let mut addr = self.inner_mut().memory.mmap_base;
        while (addr & mask) != 0 {
            addr += PAGE_ALIGN as u64;
        }

        self.inner_mut().memory
            .set_mmap_base(addr + length as u64);

        addr
    }

    pub fn mmap(&self, length: usize, prot: u32) -> anyhow::Result<(VMPointer<'a, T>, usize)> {
        if length == 0 {
            return Err(anyhow!("mmap failed, length is zero"));
        }

        let aligned = ((length - 1) / PAGE_ALIGN + 1) * PAGE_ALIGN;
        let (errno, addr) = self.mmap2(0, aligned, prot, 0, -1, 0)?;
        if errno != Errno::OK {
            return Err(anyhow!("mmap failed: errno={:?}", errno));
        }
        let base = self.inner_mut().memory.mmap_base;
        if option_env!("PRINT_MMAP_LOG") == Some("1") {
            println!("mmap addr=0x{:X}, mmap_base_address=0x{:X}, length={}, prot={}", addr, base, length, prot);
        }

        let mut pointer = VMPointer::new(addr, 0, self.backend.clone());
        pointer.size = aligned;

        Ok((pointer, aligned))
    }

    pub fn mmap2(&self, start: u64, length: usize, prot: u32, flags: u32, fd: i32, offset: i64) -> anyhow::Result<(Errno, u64)> {
        let aligned = ((length - 1) / PAGE_ALIGN + 1) * PAGE_ALIGN;
        let is_anonymous = (flags as i32 & MAP_ANONYMOUS) != 0 || (start == 0 && fd <= 0 && offset == 0);

        if (flags as i32 & MAP_FIXED) != 0 && is_anonymous {
            if option_env!("PRINT_MMAP_LOG") == Some("1") {
                println!("mmap2 MAP_FIXED start=0x{:X}, length={}, prot={:?}", start, length, prot);
            }

            self.munmap(start, length as u64)?;
            self.backend.mem_map(start, aligned, prot).map_err(|e| anyhow!("mmap2 failed: {:?}", e))?;

            let memory = &mut self.inner_mut().memory;
            if memory.memory_map.insert(start, MemoryMap::new(start, aligned, prot)).is_some() {
                warn!("mmap2 replace exists memory map: start=0x{:X}", start);
            }

            return Ok((Errno::OK, start));
        }

        if is_anonymous {
            let base = self.inner_mut().memory.mmap_base;
            let addr = self.allocate_map_address(0, aligned);
            if option_env!("PRINT_MMAP_LOG") == Some("1") {
                println!("mmap2 addr=0x{:X}, mmap_base_address=0x{:X}, start=0x{:X}, fd={}, offset={}, aligned={}, LR=0x{:X}", addr, base, start, fd, offset, aligned, self.get_lr()?);
            }

            self.backend.mem_map(addr, aligned, prot)
                .map_err(|e| anyhow!("mmap2 failed: {:?}", e))?;

            let memory = &mut self.inner_mut().memory;
            if memory.memory_map.insert(addr, MemoryMap::new(addr, aligned, prot)).is_some() {
                warn!("mmap2 replace exists memory map: addr=0x{:X}", addr);
            }

            return Ok((Errno::OK, addr));
        }

        if fd > 0 {
            let syscall_handler = &mut self.inner_mut().file_system;
            return if let Some(file) = syscall_handler.get_file_mut(fd) {
                let oflags = match file {
                    FileIO::Bytes(file) => file.oflags(),
                    FileIO::File(file) => file.oflags(),
                    FileIO::Error(_) => return Ok((Errno::ENODEV, 0)),
                    FileIO::Dynamic(file) => file.oflags(),
                    FileIO::Direction(_) => unreachable!(),
                    FileIO::LocalSocket(_) => unreachable!()
                };
                if prot & (Permission::WRITE.bits()) != 0 {
                    if !oflags.contains(OFlag::O_RDWR) && !oflags.contains(OFlag::O_WRONLY) {
                        return Ok((Errno::EACCES, 0));
                    }
                }
                if prot & (Permission::READ.bits()) != 0 {
                    if !oflags.contains(OFlag::O_RDWR) && !oflags.contains(OFlag::O_RDONLY) {
                        return Ok((Errno::EACCES, 0));
                    }
                }
                //let writable = prot & (Permission::WRITE) != Permission::empty();

                if start == 0 {
                    let addr = self.allocate_map_address(0, aligned);
                    let base = self.inner_mut().memory.mmap_base;
                    if option_env!("PRINT_MMAP_LOG") == Some("1") {
                        println!("mmap2 addr=0x{:X}, mmap_base_address=0x{:X}, start=0x{:X}, fd={}, offset={}, aligned={}, LR={:X}",addr, base, start, fd, offset, aligned, self.get_lr()?);
                    }
                    let ret = match file {
                        FileIO::Bytes(file) => file.mmap(self, addr, aligned as i32, prot, offset as u32, length as u64)?,
                        FileIO::Error(_) => panic!("mmap2 failed, file not found"),
                        FileIO::File(file) => file.mmap(self, addr, aligned as i32, prot, offset as u32, length as u64)?,
                        FileIO::Dynamic(file) => file.mmap(self, addr, aligned as i32, prot, offset as u32, length as u64)?,
                        FileIO::Direction(_) => unreachable!(),
                        FileIO::LocalSocket(_) => unreachable!()
                    };

                    let memory = &mut self.inner_mut().memory;
                    if memory.memory_map.insert(addr, MemoryMap::new_from_file(addr, aligned, prot)).is_some() {
                        warn!("mmap2 replace exists memory map: addr=0x{:X}", addr);
                    }

                    return Ok((Errno::OK, ret));
                }

                if (start as usize & (PAGE_ALIGN - 1)) != 0 {
                    return Ok((Errno::EINVAL, 0));
                }
                let end = start + length as u64;
                let memory = &mut self.inner_mut().memory;
                for (k, map) in memory.memory_map.iter() {
                    if std::cmp::max(start, *k) <= std::cmp::min(map.base + map.size as u64, end) {
                        return Ok((Errno::EEXIST, 0));
                    }
                }

                //let ret = file.mmap(self, start, aligned as i32, prot, offset as u32, length as u64)?;
                let ret = match file {
                    FileIO::Bytes(file) => file.mmap(self, start, aligned as i32, prot, offset as u32, length as u64)?,
                    FileIO::Error(_) => panic!("mmap2 failed, file not found"),
                    FileIO::File(file) => file.mmap(self, start, aligned as i32, prot, offset as u32, length as u64)?,
                    FileIO::Dynamic(file) => file.mmap(self, start, aligned as i32, prot, offset as u32, length as u64)?,
                    FileIO::Direction(_) => unreachable!(),
                    FileIO::LocalSocket(_) => unreachable!()
                };

                let memory = &mut self.inner_mut().memory;
                if memory.memory_map.insert(start, MemoryMap::new_from_file(start, aligned, prot)).is_some() {
                    warn!("mmap2 replace exists memory map: start=0x{:X}", start);
                }

                Ok((Errno::OK, ret))
            } else {
                Ok((Errno::EBADF, 0))
            }
        }

        warn!("mmap2 failed, start=0x{:X}, length={}, prot={:?}, flags=0x{:X}, fd={}, offset={}", start, length, prot, flags, fd, offset);

        Ok((Errno::ENODEV, 0))
    }

    pub fn munmap(&self, start: u64, length: u64) -> anyhow::Result<()> {
        #[inline]
        fn find_segment(mem_map: &HashMap<u64, MemoryMap>, start: u64) -> Option<(u64, MemoryMap)> {
            let mut segment = None;
            for (key, value) in mem_map.iter() {
                if start >*key && start < value.base + value.size as u64 {
                    segment = Some((*key, value.clone()));
                    break
                }
            }
            segment
        }

        let aligned = ((length as usize - 1) / PAGE_ALIGN + 1) * PAGE_ALIGN;
        self.backend.mem_unmap(start, aligned)
            .map_err(|e| anyhow!("munmap failed: {:?}", e))?;

        let memory = &mut self.inner_mut().memory;
        let mem_map = &mut memory.memory_map;
        let removed = mem_map.remove(&start);
        if removed.is_none() {
            let sg = find_segment(mem_map, start);
            if sg.is_none() {
                return Err(anyhow!("munmap failed, start=0x{:X}", start));
            }
            let sg = sg.unwrap().1;
            if sg.size < aligned {
                return Err(anyhow!("munmap failed, start=0x{:X}, size={}, aligned={}", start, sg.size, aligned));
            }

            if (start + aligned as u64) < (sg.base + sg.size as u64) {
                let new_size = sg.base + sg.size as u64 - start - aligned as u64;
                if option_env!("PRINT_MMAP_LOG") == Some("1") {
                    println!("munmap aligned=0x{:X}, start=0x{:X}, base=0x{:X}, size={}", aligned, start, start + aligned as u64, new_size);
                }

                if mem_map.insert(start + aligned as u64, MemoryMap::new(start + aligned as u64, new_size as usize, sg.prot)).is_some() {
                    if option_env!("PRINT_MMAP_LOG") == Some("1") {
                        eprintln!("munmap replace exists memory map addr=0x{:X}", start + aligned as u64);
                    }
                }
            }

            if mem_map.insert(sg.base, MemoryMap::new(sg.base, (start -  sg.base) as usize, sg.prot)).is_none() {
                if option_env!("PRINT_MMAP_LOG") == Some("1") {
                    eprintln!("munmap replace failed warning: addr=0x{:X}", sg.base);
                }
            }

            return Ok(())
        }
        let removed = removed.unwrap();
        if removed.size != aligned {
            if aligned >= removed.size {
                if option_env!("PRINT_MMAP_LOG") == Some("1") {
                    println!("munmap removed=0x{:X}, aligned=0x{:X}, start=0x{:X}", removed.size, aligned, start);
                }

                let mut address = start + removed.size as u64;
                let mut size = aligned - removed.size;
                while size != 0 {
                    let remove = memory.memory_map.remove(&address).unwrap();
                    if removed.prot != remove.prot {
                        return Err(anyhow!("munmap failed, prot not equal"));
                    }
                    address += remove.size as u64;
                    size -= remove.size;
                }

                return Ok(());
            }

            if memory.memory_map.insert(start + aligned as u64, MemoryMap::new(start + aligned as u64, removed.size - aligned, removed.prot)).is_none() {
                if option_env!("PRINT_MMAP_LOG") == Some("1") {
                    eprintln!("munmap replace failed warning: addr=0x{:X}", start);
                }
            }

            if option_env!("PRINT_MMAP_LOG") == Some("1") {
                println!("munmap removed=0x{:X}, aligned=0x{:X}, base=0x{:X}, size={}", removed.size, aligned, start + aligned as u64, removed.size - aligned);
            }

            return Ok(());
        }
        if option_env!("PRINT_MMAP_LOG") == Some("1") {
            println!("munmap aligned=0x{:X}, start=0x{:X}, base=0x{:X}, size={}", aligned, start, removed.base, removed.size);
        }
        if memory.memory_map.is_empty() {
            memory.set_mmap_base(MMAP_BASE);
        }

        Ok(())
    }
}