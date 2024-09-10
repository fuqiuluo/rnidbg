use std::cell::UnsafeCell;
use std::ptr::read;
use std::rc::Rc;
use anyhow::anyhow;
use bytes::{Buf, BytesMut};
use log::error;
use crate::elf::parser::{ElfFile, ElfParser};
use crate::elf::abi::{*};
use crate::elf::hash_tab::{ElfGnuHashTable, ElfHashTable, HashTable};
use crate::elf::init_array::ElfInitArray;
use crate::elf::memorized_object::{MemoizedObject};
use crate::elf::relocation::ElfRelocation;
use crate::elf::str_tab::ElfStringTable;
use crate::elf::symbol_structure::{ElfSymbolStructure, SymbolLocator};

#[derive(Clone)]
pub struct ElfDynamicStructure {
    dt_strtab_offset: i64,
    dt_strtab_size: u32,
    pub init_array_offset: i64,
    pub init_array_size: u32,
    pub preinit_array_offset: i64,
    pub preinit_array_size: u32,
    symbol_entry_size: u32,
    symbol_offset: i64,
    hash_offset: i64,
    gnu_hash_offset: i64,
    rel_offset: i64,
    rel_size: u32,
    rel_entry_size: u32,
    plt_rel_size: u32,
    plt_rel_offset: i64,
    android_rela_size: u32,
    android_rel_size: u32,
    android_rela_offset: i64,
    android_rel_offset: i64,
    pub arm_ex_idx: i64,

    so_name: i32,
    dt_needed_list: Vec<u32>,
    pub init: i32,

    dt_string_table: MemoizedObject<ElfStringTable>,
    pub symbol_structure: MemoizedObject<ElfSymbolStructure>,
    rel: Vec<MemoizedObject<ElfRelocation>>,
    plt_rel: Vec<MemoizedObject<ElfRelocation>>,
    android_rel: Vec<ElfRelocation>,
    init_array: MemoizedObject<ElfInitArray>,
    pre_init_array: MemoizedObject<ElfInitArray>,
}

impl ElfDynamicStructure {
    pub fn new(elf_file: Rc<UnsafeCell<ElfFile>>, parser: ElfParser, offset: usize, size: usize) -> Self {
        parser.seek(offset);
        let mut dynamic_structure = ElfDynamicStructure {
            dt_strtab_offset: 0,
            dt_strtab_size: 0,
            init_array_offset: 0,
            init_array_size: 0,
            preinit_array_offset: 0,
            preinit_array_size: 0,
            symbol_entry_size: 0,
            symbol_offset: 0,
            hash_offset: 0,
            gnu_hash_offset: 0,
            rel_offset: 0,
            rel_size: 0,
            rel_entry_size: 0,
            plt_rel_size: 0,
            plt_rel_offset: 0,
            android_rela_size: 0,
            android_rel_size: 0,
            android_rela_offset: 0,
            android_rel_offset: 0,
            arm_ex_idx: 0,

            so_name: 0,
            dt_needed_list: vec![],
            init: 0,

            dt_string_table: MemoizedObject::new_with_none(),
            symbol_structure: MemoizedObject::new_with_none(),
            rel: vec![],
            plt_rel: vec![],
            android_rel: vec![],
            init_array: MemoizedObject::new_with_none(),
            pre_init_array: MemoizedObject::new_with_none(),
        };
        //let num_entries = size / if parser.object_size == 1 { 8 } else { 16 };
        let num_entries = size / 16;

        let (dt_needed_list, so_name, init) = parse_dynamic_basic_info(&mut dynamic_structure, &parser, num_entries);

        let elf_file = unsafe { &*elf_file.get() };
        if dynamic_structure.dt_strtab_offset > 0 {
            let cloned_parser = parser.clone();
            let offset = elf_file.virtual_memory_addr_to_file_offset(dynamic_structure.dt_strtab_offset as u64);
            dynamic_structure.dt_string_table.set_compute_value(move || {
                Ok(ElfStringTable::new(cloned_parser.clone(), offset as usize, dynamic_structure.dt_strtab_size as usize))
            });
        }

        let hash_tab = if dynamic_structure.hash_offset > 0 {
            let offset = elf_file.virtual_memory_addr_to_file_offset(dynamic_structure.hash_offset as u64);
            let cloned_parser = parser.clone();
            Some(HashTable::SysV(ElfHashTable::new(cloned_parser.clone(), offset as usize, 0)))
        } else if dynamic_structure.gnu_hash_offset > 0 {
            let offset = elf_file.virtual_memory_addr_to_file_offset(dynamic_structure.gnu_hash_offset as u64);
            let cloned_parser = parser.clone();
            Some(HashTable::Gnu(ElfGnuHashTable::new(cloned_parser.clone(), offset as usize)))
        } else {
            None
        };

        if dynamic_structure.symbol_offset > 0 {
            let offset = elf_file.virtual_memory_addr_to_file_offset(dynamic_structure.symbol_offset as u64);
            let cloned_parser = parser.clone();
            let entry_size = dynamic_structure.symbol_entry_size;
            let dt_string_table = dynamic_structure.dt_string_table.clone();
            dynamic_structure.symbol_structure.set_compute_value(move || {
                Ok(ElfSymbolStructure::new(cloned_parser.clone(), offset as usize, entry_size, dt_string_table.clone(), hash_tab.clone()))
            });
        }

        if dynamic_structure.rel_offset > 0 {
            let num_entries = dynamic_structure.rel_size / dynamic_structure.rel_entry_size;
            let offset = elf_file.virtual_memory_addr_to_file_offset(dynamic_structure.rel_offset as u64) as usize;
            for i in 0..num_entries {
                let relocation_offset = offset + (i as usize * dynamic_structure.rel_entry_size as usize);
                let cloned_parser = parser.clone();
                let entry_size = dynamic_structure.rel_entry_size;
                let symbol_structure = dynamic_structure.symbol_structure.clone();
                dynamic_structure.rel.push(MemoizedObject::new(move || {
                    let symtab = SymbolLocator::SymbolStructure(symbol_structure.get_value()
                        .expect("Failed to get symbol structure: rel_offset"));

                    Ok(ElfRelocation::new(cloned_parser.clone(), relocation_offset, entry_size, symtab))
                }));
            }
        }

        if dynamic_structure.plt_rel_offset > 0 {
            if dynamic_structure.rel_entry_size == 0 {
                dynamic_structure.rel_entry_size = 24;
            }
            let num_entries = dynamic_structure.plt_rel_size / dynamic_structure.rel_entry_size;
            let offset = elf_file.virtual_memory_addr_to_file_offset(dynamic_structure.plt_rel_offset as u64) as usize;
            for i in 0..num_entries {
                let relocation_offset = offset + (i as usize * dynamic_structure.rel_entry_size as usize);
                let cloned_parser = parser.clone();
                let entry_size = dynamic_structure.rel_entry_size;
                let symbol_structure = dynamic_structure.symbol_structure.clone();
                dynamic_structure.plt_rel.push(MemoizedObject::new(move || {
                    let symtab = SymbolLocator::SymbolStructure(symbol_structure.get_value()
                        .expect("Failed to get symbol structure: plt_rel_offset"));
                    Ok(ElfRelocation::new(cloned_parser.clone(), relocation_offset, entry_size, symtab))
                }));
            }
        }

        if dynamic_structure.android_rel_offset > 0 {
            parse_android_rel(elf_file, &mut dynamic_structure, &parser, false);
        } else if dynamic_structure.android_rela_offset > 0 {
            parse_android_rel(elf_file, &mut dynamic_structure, &parser, true);
        }

        if dynamic_structure.init_array_offset > 0 {
            let offset = elf_file.virtual_memory_addr_to_file_offset(dynamic_structure.init_array_offset as u64);
            let cloned_parser = parser.clone();
            dynamic_structure.init_array.set_compute_value(move || {
                Ok(ElfInitArray::new(cloned_parser.clone(), offset as usize, dynamic_structure.init_array_size as usize))
            });
        }

        if dynamic_structure.preinit_array_offset > 0 {
            let offset = elf_file.virtual_memory_addr_to_file_offset(dynamic_structure.preinit_array_offset as u64);
            let cloned_parser = parser.clone();
            dynamic_structure.pre_init_array.set_compute_value(move || {
                Ok(ElfInitArray::new(cloned_parser.clone(), offset as usize, dynamic_structure.preinit_array_size as usize))
            });
        }

        dynamic_structure.dt_needed_list = dt_needed_list;
        dynamic_structure.so_name = so_name;
        dynamic_structure.init = init;

        dynamic_structure
    }

    pub fn so_name(&self, file_name: String) -> anyhow::Result<String> {
        let string_table = self.dt_string_table.get_value()?;
        if self.so_name == -1 {
            return Ok(file_name);
        }
        Ok(string_table.get(self.so_name as usize))
    }

    pub fn needed_libraries(&self) -> anyhow::Result<Vec<String>> {
        let string_table = self.dt_string_table.get_value()?;
        let mut needed = Vec::with_capacity(self.dt_needed_list.len());
        for &index in self.dt_needed_list.iter() {
            needed.push(string_table.get(index as usize));
        }
        Ok(needed)
    }

    pub fn relocations(&self) -> Vec<ElfRelocation> {
        let mut result = vec![];
        if !self.android_rel.is_empty() {
            result.extend(self.android_rel.iter().cloned());
        }

        for rel in self.rel.iter() {
            result.push(rel.get_value()
                .map_err(|e| error!("Failed to get relocation: {:?}", e))
                .expect("Failed to get relocation: rel"));
        }

        for rel in self.plt_rel.iter() {
            result.push(rel.get_value()
                .map_err(|e| error!("Failed to get relocation: {:?}", e))
                .expect("Failed to get relocation: plt_rel"));
        }

        result
    }
}

fn parse_android_rel(
    elf_file: &ElfFile,
    dynamic_structure: &mut ElfDynamicStructure,
    parser: &ElfParser,
    rela: bool
)
{
    let rel_size = if rela {
        dynamic_structure.android_rela_size
    } else {
        dynamic_structure.android_rel_size
    };
    let rel_offset = if rela {
        dynamic_structure.android_rela_offset
    } else {
        dynamic_structure.android_rel_offset
    } as u64;
    let offset = elf_file.virtual_memory_addr_to_file_offset(rel_offset);
    parser.seek(offset as usize);
    let mut magic = [0u8; 4];
    parser.read(&mut magic);
    if rel_size > 4 && String::from_utf8_lossy(&magic) == "APS2" {
        let mut buffer = vec![0u8; rel_size as usize - 4];
        parser.read(&mut buffer);
        let mut buffer = BytesMut::from(buffer.as_slice());
        let mut relocation_count_ = read_sleb128(&mut buffer).unwrap();

        let mut reloc_offset = read_sleb128(&mut buffer).unwrap();
        let mut reloc_info = 0;
        let mut reloc_addend = 0;

        let mut relocation_index_ = 0;
        let mut relocation_group_index_ = 0;
        let mut group_size_ = 0;
        let mut group_flags_ = 0;
        let mut group_r_offset_delta_ = 0;

        //let is_relocation_grouped_by_info = || { (group_flags_ & 1) != 0 };
        //let is_relocation_grouped_by_offset_delta = || { (group_flags_ & 2) != 0 };
        //let is_relocation_grouped_by_addend = || { (group_flags_ & 4) != 0 };
        //let is_relocation_group_has_addend = || { (group_flags_ & 8) != 0 };

        while relocation_index_ < relocation_count_ {
            if relocation_group_index_ == group_size_ {
                group_size_ = read_sleb128(&mut buffer).unwrap();
                group_flags_ = read_sleb128(&mut buffer).unwrap();

                if (group_flags_ & 2) != 0 {
                    group_r_offset_delta_ = read_sleb128(&mut buffer).unwrap();
                }

                if (group_flags_ & 1) != 0 {
                    reloc_info = read_sleb128(&mut buffer).unwrap();
                }

                if (group_flags_ & 8) != 0 && (group_flags_ & 4) != 0 {
                    reloc_addend += read_sleb128(&mut buffer).unwrap();
                } else if (group_flags_ & 8) == 0 {
                    if rela {
                        reloc_addend = 0;
                    }
                }

                relocation_group_index_ = 0;
            }

            if (group_flags_ & 2) != 0 {
                reloc_offset += group_r_offset_delta_;
            } else {
                reloc_offset += read_sleb128(&mut buffer).unwrap();
            }

            if (group_flags_ & 1) == 0 {
                reloc_info = read_sleb128(&mut buffer).unwrap();
            }

            if (group_flags_ & 8) != 0 && (group_flags_ & 4) == 0 {
                if !rela {
                    panic!("Unexpected r_addend in android.rel section")
                }
                reloc_addend += read_sleb128(&mut buffer).unwrap();
            }

            relocation_group_index_ += 1;
            relocation_index_ += 1;

            let symtab = SymbolLocator::SymbolStructure(dynamic_structure.symbol_structure.get_value()
                .expect("Failed to get symbol structure: android_rel_offset"));
            dynamic_structure.android_rel.push(ElfRelocation {
                //object_size: parser.object_size,
                symtab,
                android: true,
                offset: reloc_offset as u64,
                info: reloc_info,
                addend: reloc_addend,
            });
        }
    }
}

fn parse_dynamic_basic_info(dynamic_structure: &mut ElfDynamicStructure, parser: &ElfParser, num_entries: usize) -> (Vec<u32>, i32, i32) {
    let mut dt_needed_list = vec![0u32; 0];

    let mut so_name = 0i32;
    let mut init = 0i32;

    for _ in 0..num_entries {
        let tag = parser.read_int_or_long();
        let val = parser.read_int_or_long();
        match tag {
            DT_NULL => {
                break
            }
            DT_NEEDED => {
                dt_needed_list.push(val as u32);
            }
            DT_STRTAB => {
                dynamic_structure.dt_strtab_offset = val;
            }
            DT_STRSZ => {
                dynamic_structure.dt_strtab_size = val as u32;
            }
            DT_SONAME => {
                so_name = val as i32;
            }
            DT_INIT => {
                init = val as i32;
            }
            DT_INIT_ARRAY => {
                dynamic_structure.init_array_offset = val;
            }
            DT_INIT_ARRAYSZ => {
                dynamic_structure.init_array_size = val as u32;
            }
            DT_PREINIT_ARRAY => {
                dynamic_structure.preinit_array_offset = val;
            }
            DT_PREINIT_ARRAYSZ => {
                dynamic_structure.preinit_array_size = val as u32;
            }
            DT_SYMENT => {
                dynamic_structure.symbol_entry_size = val as u32;
            }
            DT_SYMTAB => {
                dynamic_structure.symbol_offset = val;
            }
            DT_HASH => {
                dynamic_structure.hash_offset = val;
            }
            DT_GNU_HASH => {
                dynamic_structure.gnu_hash_offset = val;
            }
            DT_RELA | DT_REL => {
                dynamic_structure.rel_offset = val;
            }
            DT_RELASZ | DT_RELSZ => {
                dynamic_structure.rel_size = val as u32;
            }
            DT_RELAENT | DT_RELENT => {
                dynamic_structure.rel_entry_size = val as u32;
            }
            DT_PLTRELSZ => {
                dynamic_structure.plt_rel_size = val as u32;
            }
            DT_JMPREL => {
                dynamic_structure.plt_rel_offset = val;
            }
            0x60000012 => {
                dynamic_structure.android_rela_size = val as u32;
            }
            0x60000010 => {
                dynamic_structure.android_rel_size = val as u32;
            }
            0x60000011 => {
                dynamic_structure.android_rela_offset = val;
            }
            0x6000000f => {
                dynamic_structure.android_rel_offset = val;
            }
            0x70000001 => {
                dynamic_structure.arm_ex_idx = val;
            }
            DT_VERSYM | DT_VERNEED | DT_VERNEEDNUM | DT_VERDEF | DT_VERDEFNUM => {}
            DT_RELACOUNT | DT_RELCOUNT | DT_FLAGS_1 | DT_AUXILIARY => {}
            _ => {
                let android_tag = tag & 0x60000000;
                if android_tag != 0 {
                    eprintln!("Unsupported android tag: 0x{:x}", tag);
                }
            }
        }
    }

    (dt_needed_list, so_name, init)
}

fn read_sleb128(buf: &mut BytesMut) -> anyhow::Result<i64> {
    let mut result = 0;
    let mut shift = 0;

    loop {
        let byte = buf.get_u8();
        if shift == 63 && byte != 0x00 && byte != 0x7f {
            return Err(anyhow!("SLEB128 value too large"));
        }
        result |= i64::from(byte & 0x7f) << shift;
        shift += 7;

        if byte & 0x80 == 0 {
            if shift < 64 && (byte & 0x40) != 0 {
                result |= !0 << shift;
            }
            return Ok(result);
        }
    }
}