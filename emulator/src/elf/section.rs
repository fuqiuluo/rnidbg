use std::cell::UnsafeCell;
use std::rc::Rc;
use anyhow::anyhow;
use crate::elf::dynamic_struct::ElfDynamicStructure;
use crate::elf::hash_tab::ElfHashTable;
use crate::elf::memorized_object::{MemoizedObject};
use crate::elf::parser::{ElfFile, ElfParser};
use crate::elf::str_tab::ElfStringTable;
use crate::elf::symbol::ElfSymbol;
use crate::elf::abi::{*};
use crate::elf::init_array::ElfInitArray;
use crate::elf::relocation::ElfRelocation;
use crate::elf::symbol_structure::SymbolLocator;

#[derive(Clone)]
pub enum ElfSectionData {
    Symbols(Vec<MemoizedObject<ElfSymbol>>),
    StringTable(MemoizedObject<ElfStringTable>),
    HashTable(MemoizedObject<ElfHashTable>),
    DynamicStructure(MemoizedObject<ElfDynamicStructure>),
    Relocations(Vec<MemoizedObject<ElfRelocation>>),
    InitArray(MemoizedObject<ElfInitArray>),
    PreInitArray(MemoizedObject<ElfInitArray>),
    Unknown()
}

#[derive(Clone)]
pub struct ElfSection {
    pub name_ndx: i32,
    pub typ: u32,
    pub flags: i64,
    pub addr: i64,
    pub section_offset: usize,
    pub size: i64,
    pub link: i32,
    pub info: i32,
    pub addralign: i64,
    pub entsize: i64,
    pub data: ElfSectionData,
}


impl ElfSection {
    pub fn new(elf_file: Rc<UnsafeCell<ElfFile>>, parser: ElfParser, offset: usize) -> Self {
        parser.seek(offset);

        let name_ndx = parser.read_int();
        let typ = parser.read_int() as u32;
        let flags = parser.read_int_or_long();
        let addr = parser.read_int_or_long();
        let section_offset = parser.read_int_or_long();
        let size = parser.read_int_or_long();
        let link = parser.read_int();
        let info = parser.read_int();
        let addralign = parser.read_int_or_long();
        let entsize = parser.read_int_or_long();

        let mut section = ElfSection {
            name_ndx, typ, flags, addr,
            section_offset: section_offset as usize,
            size, link, info, addralign, entsize,
            data: ElfSectionData::Unknown(),
        };

        match typ {
            SHT_SYMTAB | SHT_DYNSYM => {
                let num_entries = size / entsize;
                for i in 0..num_entries {
                    let symbol_offset = section_offset + (i * entsize);
                    let cloned_parser = parser.clone();
                    let mut symbols = vec![];
                    symbols.push(MemoizedObject::new(move || {
                        Ok(ElfSymbol::new(cloned_parser.clone(), symbol_offset as usize, typ as i32))
                    }));
                    section.data = ElfSectionData::Symbols(symbols);
                }
            }
            SHT_STRTAB => {
                let cloned_parser = parser.clone();
                section.data = ElfSectionData::StringTable(MemoizedObject::new(move || {
                    Ok(ElfStringTable::new(cloned_parser.clone(), section_offset as usize, size as usize))
                }));
            }
            SHT_HASH => {
                let cloned_parser = parser.clone();
                section.data = ElfSectionData::HashTable(MemoizedObject::new(move || {
                    Ok(ElfHashTable::new(cloned_parser.clone(), section_offset as usize, size as usize))
                }));
            }
            SHT_DYNAMIC => {
                let cloned_parser = parser.clone();
                let elf_file_c = elf_file.clone();
                section.data = ElfSectionData::DynamicStructure(MemoizedObject::new(move || {
                    Ok(ElfDynamicStructure::new(elf_file_c.clone(), cloned_parser.clone(), section_offset as usize, size as usize))
                }));
            }
            SHT_RELA | SHT_REL => {
                let num_entries = size / entsize;
                for i in 0..num_entries {
                    let relocation_offset = section_offset + (i * entsize);
                    let cloned_parser = parser.clone();
                    let elf_file = elf_file.clone();
                    let mut relocations = vec![];
                    relocations.push(MemoizedObject::new(move || {
                        let symtab = unsafe { &*elf_file.get() }.get_section(link as usize)
                            .expect("Failed to get section from link");

                        Ok(ElfRelocation::new(cloned_parser.clone(), relocation_offset as usize, entsize as u32, SymbolLocator::Section(symtab.clone())))
                    }));
                    section.data = ElfSectionData::Relocations(relocations);
                }
            }
            SHT_INIT_ARRAY => {
                let cloned_parser = parser.clone();
                section.data = ElfSectionData::InitArray(MemoizedObject::new(move || {
                    Ok(ElfInitArray::new(cloned_parser.clone(), section_offset as usize, size as usize))
                }));
            }
            SHT_PREINIT_ARRAY => {
                let cloned_parser = parser.clone();
                section.data = ElfSectionData::PreInitArray(MemoizedObject::new(move || {
                    Ok(ElfInitArray::new(cloned_parser.clone(), section_offset as usize, size as usize))
                }));
            }

            _ => {}
        }

        section
    }

    pub fn get_elf_symbol(&self, index: i32) -> anyhow::Result<ElfSymbol> {
        match self.data {
            ElfSectionData::Symbols(ref symbols) => {
                let symbol = symbols[index as usize].get_value()?;
                Ok(symbol)
            }
            _ => return Err(anyhow!("Section is not a symbol table"))
        }
    }

    pub fn get_elf_symbol_by_name(&self, name: &str, elf_file: &ElfFile) -> anyhow::Result<ElfSymbol> {
        match self.data {
            ElfSectionData::Symbols(ref symbols) => {
                for i in 0..symbols.len() {
                    let symbol = symbols[i].get_value()?;
                    if name == symbol.name(elf_file)? {
                        return Ok(symbol);
                    }
                }
            }
            _ => return Err(anyhow!("Section is not a symbol table"))
        }
        Err(anyhow!("Failed to get symbol by name"))
    }

    pub(crate) fn get_elf_symbol_by_addr(&self, so_addr: u64) -> anyhow::Result<ElfSymbol> {
        match self.data {
            ElfSectionData::Symbols(ref symbols) => {
                for i in 0..symbols.len() {
                    let symbol = symbols[i].get_value()?;
                    if so_addr >= symbol.value as u64 && so_addr < (symbol.value + symbol.size) as u64 {
                        return Ok(symbol);
                    }
                }
            }
            _ => return Err(anyhow!("Section is not a symbol table"))
        }
        Err(anyhow!("Failed to get symbol by name"))
    }

    pub fn name(&self, elf_file: &ElfFile) -> anyhow::Result<String> {
        let string_table = elf_file.get_section_name_string_table()?;
        Ok(string_table.get(self.name_ndx as usize))
    }
}