use std::cell::{RefCell, UnsafeCell};
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32};
use anyhow::anyhow;
use indexmap::IndexMap;
use log::{error, info};
use crate::backend::Backend;
use crate::elf::dynamic_struct::ElfDynamicStructure;
use crate::elf::memorized_object::MemoizedObject;
use crate::elf::parser::ElfFile;
use crate::elf::pt::{ArmExIdx, GnuEhFrameHeader};
use crate::elf::section::ElfSection;
use crate::elf::symbol::ElfSymbol;
use crate::elf::symbol_structure::SymbolLocator;
use crate::emulator::{AndroidEmulator, RcUnsafeCell, VMPointer};
use crate::linux::init_fun::{InitFunction, InitFunctionTrait};
use crate::linux::PAGE_ALIGN;
use crate::linux::symbol::{LinuxSymbol, ModuleSymbol};
use crate::memory::library_file::LibraryFile;
use crate::memory::ModuleMemRegion;
use crate::tool::align_addr;

pub struct LinuxModule<'a, T: Clone> {
    pub name: String,
    pub base: u64,
    pub size: usize,
    pub(crate) needed_libraries: IndexMap<String, RcUnsafeCell<LinuxModule<'a, T>>>,
    pub regions: Vec<ModuleMemRegion>,
    pub ref_cnt: Arc<AtomicU32>,
    /*
     * Elf information
     */
    pub entry_point: u64,

    pub path_pointer: VMPointer<'a, T>,

    /*
    LinuxModule
     */
    pub hook_map: HashMap<String, u64>,
    pub(crate) dynsym: Option<SymbolLocator>,
    pub virtual_base: u64,
    pub(crate) unresolved_symbol: Vec<ModuleSymbol>,
    pub(crate) resolved_symbol: IndexMap<String, ModuleSymbol>,
    pub(crate) init_function_list: VecDeque<InitFunction<'a, T>>,
    pub(crate) arm_ex_idx: Option<MemoizedObject<ArmExIdx>>,
    pub(crate) eh_frame_header: Option<MemoizedObject<GnuEhFrameHeader>>,
    pub(crate) symbol_table_section: Option<ElfSection>,
    pub(crate) dynamic_structure: Option<ElfDynamicStructure>,
    pub(crate) elf_file: Option<Rc<UnsafeCell<ElfFile>>>,
}

impl<'a, T: Clone> LinuxModule<'a, T> {
    pub(crate) fn new(
        virtual_base: u64,
        base: u64,
        size: usize,
        name: String,
        dynsym: Option<SymbolLocator>,
        unresolved_symbol: Vec<ModuleSymbol>,
        init_function_list: VecDeque<InitFunction<'a, T>>,
        needed_libraries: IndexMap<String, RcUnsafeCell<LinuxModule<'a, T>>>,
        regions: Vec<ModuleMemRegion>,
        arm_ex_idx: Option<MemoizedObject<ArmExIdx>>,
        eh_frame_header: Option<MemoizedObject<GnuEhFrameHeader>>,
        symbol_table_section: Option<ElfSection>,
        elf_file: Option<Rc<UnsafeCell<ElfFile>>>,
        dynamic_structure: Option<ElfDynamicStructure>,
    ) -> Self {
        Self {
            virtual_base,
            dynsym,
            unresolved_symbol,
            init_function_list,
            arm_ex_idx,
            symbol_table_section,
            dynamic_structure,
            base,
            size,
            name,
            needed_libraries,
            regions,
            ref_cnt: Arc::new(AtomicU32::new(0)),
            entry_point: 0,
            path_pointer: VMPointer::null(),
            eh_frame_header,
            elf_file,
            hook_map: HashMap::new(),
            resolved_symbol: IndexMap::new(),
        }
    }

    pub fn find_symbol_by_closest_addr(&self, addr: u64) -> anyhow::Result<LinuxSymbol> {
        if let Some(dynsym) = self.dynsym.as_ref() {
            let so_addr = addr.overflowing_sub(self.base);
            if so_addr.1 {
                return Err(anyhow!("Failed to find symbol by closest addr: 0x{:X}", addr))
            }
            let elf_symbol = match dynsym {
                SymbolLocator::Section(sc) => sc.get_elf_symbol_by_addr(so_addr.0),
                SymbolLocator::SymbolStructure(ss) => ss.get_elf_symbol_by_addr(so_addr.0)
            };
            let elf_file = unsafe { &*self.elf_file.as_ref().unwrap().get() };
            let mut symbol = if let Ok(elf_symbol) = elf_symbol {
                LinuxSymbol::new(elf_symbol.name(elf_file)?, elf_symbol, self.base, self.name.clone()).into()
            } else { None };
            let entry = self.base + self.entry_point;
            if addr >= entry && symbol.is_some() && entry > symbol.as_ref().unwrap().address() {
                symbol = Some(LinuxSymbol::new("start".to_string(), symbol.unwrap().symbol, self.base, self.name.clone()));
            }

            if let Some(symbol) = symbol {
                return Ok(symbol);
            }
        }
        Err(anyhow!("Failed to find symbol by closest addr: 0x{:X}", addr))
    }

    pub fn find_symbol_by_name(&self, name: &str, with_dep: bool) -> anyhow::Result<LinuxSymbol> {
        if let Some(dynsym) = self.dynsym.as_ref() {
            let elf_file = unsafe { &*self.elf_file.as_ref().unwrap().get() };
            let elf_symbol = match dynsym {
                SymbolLocator::Section(sec) => sec.get_elf_symbol_by_name(name, elf_file),
                SymbolLocator::SymbolStructure(ss) => ss.get_elf_symbol_by_name(name, elf_file)
            };
            if let Ok(elf_symbol) = elf_symbol {
                return Ok(LinuxSymbol::new(
                    name.to_string(),
                    elf_symbol,
                    self.base,
                    self.name.clone()
                ));
            }

            if with_dep {
                for (_, lib) in &self.needed_libraries {
                    let module = unsafe { &*lib.get() };
                    let symbol = module.find_symbol_by_name(name, with_dep);
                    if let Ok(symbol) = symbol {
                        return Ok(symbol);
                    }
                }
            }
        }
        Err(anyhow!("Failed to find symbol by name: {}", name))
    }

    pub fn get_elf_symbol_by_name(&self, name: &str, elf_file: &ElfFile) -> anyhow::Result<ElfSymbol> {
        if let Some(dynsym) = &self.dynsym {
            let symbol = match dynsym {
                SymbolLocator::Section(sec) => sec.get_elf_symbol_by_name(name, elf_file),
                SymbolLocator::SymbolStructure(ss) => ss.get_elf_symbol_by_name(name, elf_file)
            };
            return symbol;
        }

        Err(anyhow!("dynsym is none"))
    }

    pub fn create_virtual_module(
        name: String,
        symbol: HashMap<String, u64>
    ) -> Self {
        if symbol.is_empty() {
            panic!("symbol is empty!")
        }
        let mut list = symbol.iter()
            .map(|(k, v)| v.clone())
            .collect::<Vec<_>>();
        list.sort();

        let first = list.first().unwrap().clone();
        let last = list.last().unwrap().clone();

        let alignment = align_addr(first, last - first, PAGE_ALIGN as i64);
        let base = alignment.address;
        let size = alignment.size;

        info!("createVirtualModule first=0x{:X} , last=0x{:X}, base=0x{:X}, size=0x{:X}", first, last, base, size);

        let module = LinuxModule::new(
            base,
            base,
            size,
            name,
            None,
            vec![],
            VecDeque::new(),
            IndexMap::new(),
            vec![],
            None,
            None,
            None,
            None,
            None,
        );

        module
    }

    pub fn path(&self, emulator: &AndroidEmulator<T>) -> String {
        if self.name == "liblog.so" {
            return "/system/lib64/liblog.so".to_string()
        } else if self.name == "libc++.so" {
            return "/system/lib64/libc++.so".to_string()
        } else if self.name == "libc.so" {
            return "/system/lib64/libc.so".to_string()
        } else if self.name == "libm.so" {
            return "/system/lib64/libm.so".to_string()
        } else if self.name == "libdl.so" {
            return "/system/lib64/libdl.so".to_string()
        } else if self.name == "libstdc++.so" {
            return "/system/lib64/libstdc++.so".to_string()
        } else if self.name == "libz.so" {
            return "/system/lib64/libz.so".to_string()
        } else if self.name == "libandroid.so" {
            return "/system/lib64/libandroid.so".to_string()
        }
        let package_name = emulator.inner_mut().proc_name
            .split(":")
            .next()
            .unwrap_or("");
        format!("/data/app/~~YuanShenZhenChaoHaoWan/{}-0/lib/arm64/{}", package_name, self.name)
    }

    pub fn unload(&self, unicorn: &Backend<T>) -> anyhow::Result<()> {
        for region in &self.regions {
            unicorn.mem_unmap(region.begin, (region.end - region.begin) as usize)
                .map_err(|e| anyhow!("LinuxModule unload, but failed to mem_unmap: {:?}", e))?;
        }
        Ok(())
    }
}

impl<'a, T: Clone> LinuxModule<'a, T> {
    pub(crate) fn call_init_functions(&mut self, must_call_init: bool, emulator: &AndroidEmulator<'a, T>) -> anyhow::Result<()> {
        if !must_call_init && !self.unresolved_symbol.is_empty() {
            return Ok(())
        }

        //let mut called_functions = vec![];

        loop {
            let init_function = self.init_function_list.pop_front();

            if let Some(init_function) = init_function {
                match init_function {
                    InitFunction::ABSOLUTE(absolute) => {
                        if option_env!("SHOW_INIT_FUNC_CALL").unwrap_or("") == "1" {
                            let start_time = std::time::Instant::now();
                            let mut address = absolute.ptr.read_u64_with_offset(0)?;
                            if address == 0 {
                                address = absolute.addr;
                            }

                            /*if called_functions.contains(&address) {
                                error!("[{}] Already CallInitFunction: address=0x{:X}, base=0x{:X}, offset=0x{:X}, start={:?}", self.name, address, absolute.load_base, address - absolute.load_base, start_time);
                                panic!()
                            } else {
                                called_functions.push(address);
                            }*/

                            println!("[{}] CallInitFunctionStart: address=0x{:X}, base=0x{:X}, offset=0x{:X}, start={:?}", self.name, address, absolute.load_base, address - absolute.load_base, start_time);

                            let offset = address - absolute.load_base;
                            //if offset == 0x1aa74 && self.name == "libc.so"  {
                            //    continue
                            //}

                            let ret = absolute.call(emulator.clone())?;
                            let cost = start_time.elapsed().as_millis();
                            println!("[{}] CallInitFunctionEnd: address=0x{:X}, base=0x{:X}, offset=0x{:X}, ret={:X}, cost={}ms", self.name, address, absolute.load_base, offset, ret, cost);
                        } else {
                            let ret = absolute.call(emulator.clone())?;
                        }
                    }
                    InitFunction::LINUX(linux) => {
                        if option_env!("SHOW_INIT_FUNC_CALL").unwrap_or("") == "1" {
                            let start_time = std::time::Instant::now();
                            let address = linux.addr;

                            println!("[{}] CallInitFunctionStart base=0x{:X}, offset=0x{:X}, start={:?}", self.name, linux.load_base, address - linux.load_base, start_time);

                            /*if called_functions.contains(&address) {
                                error!("[{}] ALREADY CallInitFunction: base=0x{:X}, offset=0x{:X}, start={:?}", self.name, linux.load_base, address - linux.load_base, start_time);
                                panic!()
                            } else {
                                called_functions.push(address);
                            }*/

                            let ret = linux.call(emulator.clone())?;
                            let cost = start_time.elapsed().as_millis();
                            println!("[{}] CallInitFunctionEnd: base=0x{:X}, offset=0x{:X}, ret={:X}, cost={}ms", self.name, linux.load_base, address - linux.load_base, ret, cost);
                        } else {
                            let ret = linux.call(emulator.clone())?;
                        }
                    }
                }
            } else {
                break
            }
        }

        Ok(())
    }
}