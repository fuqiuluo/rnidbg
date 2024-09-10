use anyhow::anyhow;
use crate::elf::hash_tab::HashTable;
use crate::elf::memorized_object::{MemoizedObject};
use crate::elf::parser::{ElfFile, ElfParser};
use crate::elf::section::ElfSection;
use crate::elf::str_tab::ElfStringTable;
use crate::elf::symbol::ElfSymbol;

#[derive(Clone)]
pub enum SymbolLocator {
    Section(ElfSection),
    SymbolStructure(ElfSymbolStructure),
}

#[derive(Clone)]
pub struct ElfSymbolStructure {
    parser: ElfParser,
    offset: usize,
    entry_size: u32,
    string_table: MemoizedObject<ElfStringTable>,
    hash_table: MemoizedObject<Option<HashTable>>,
}

impl ElfSymbolStructure {
    pub fn new(parser: ElfParser, offset: usize, entry_size: u32, string_table: MemoizedObject<ElfStringTable>, hash_table: Option<HashTable>) -> Self {
        Self {
            parser,
            offset,
            entry_size,
            string_table,
            hash_table: MemoizedObject::new_with_value(hash_table),
        }
    }

    pub fn get_elf_symbol(&self, index: i32) -> anyhow::Result<ElfSymbol> {
        let mut symbol = ElfSymbol::new(self.parser.clone(), self.offset + index as usize * self.entry_size as usize, -1);
        symbol.set_string_table(self.string_table.get_value()?);
        Ok(symbol)
    }

    pub(crate) fn get_elf_symbol_by_addr(&self, so_addr: u64) -> anyhow::Result<ElfSymbol> {
        let hash_tab = self.hash_table.get_value()?;
        if let Some(hash_tab) = hash_tab {
            return match hash_tab {
                HashTable::Gnu(gnu) => gnu.get_symbol_by_addr(self, so_addr),
                HashTable::SysV(sys) => sys.get_symbol_by_addr(self, so_addr),
            };
        }
        Err(anyhow!("Failed to get symbol by addr: {}, hash_tab is None", so_addr))
    }

    pub fn get_elf_symbol_by_name(&self, name: &str, elf_file: &ElfFile) -> anyhow::Result<ElfSymbol> {
        let hash_tab = self.hash_table.get_value()?;
        if let Some(hash_tab) = hash_tab {
            return match hash_tab {
                HashTable::Gnu(gnu) => gnu.get_symbol(self, name, elf_file),
                HashTable::SysV(sys) => sys.get_symbol(self, name, elf_file),
            };
        }
        Err(anyhow!("Failed to get symbol by name: {}, hash_tab is None", name))
    }
}

