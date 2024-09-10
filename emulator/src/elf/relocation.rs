use crate::elf::memorized_object::MemoizedObject;
use crate::elf::parser::ElfParser;
use crate::elf::symbol::ElfSymbol;
use crate::elf::symbol_structure::{ElfSymbolStructure, SymbolLocator};

#[derive(Clone)]
pub struct ElfRelocation {
    //pub(crate) object_size: u8,
    pub(crate) symtab: SymbolLocator,
    pub(crate) android: bool,

    pub(crate) offset: u64,
    pub(crate) info: i64,
    pub(crate) addend: i64,
}

impl ElfRelocation {
    pub fn new(parser: ElfParser, offset: usize, entry_size: u32, symtab: SymbolLocator) -> Self {
        parser.seek(offset);

        //let object_size = parser.object_size;

        let offset = parser.read_int_or_long() as u64;
        let info = parser.read_int_or_long();
/*        let addend = if object_size == 1 {
            if entry_size >= 12 {
                parser.read_int() as i64
            } else {
                0
            }
        } else {
            if entry_size >= 24 {
                parser.read_long()
            } else {
                0
            }
        };*/

        let addend = if entry_size >= 24 {
            parser.read_long()
        } else {
            0
        };

        Self {
            //object_size,
            symtab,
            android: false,
            offset,
            info,
            addend,
        }
    }

    pub fn typ(&self) -> i32 {
        //let mask = if self.object_size == 1 { 0xff } else { 0xffffffff };
        let mask = 0xffffffff;
        (self.info & mask) as i32
    }

    pub fn sym(&self) -> i32 {
        //let mask = if self.object_size == 1 { 8 } else { 32 };
        let mask = 32;
        (self.info >> mask) as i32
    }

    pub fn symbol(&self) -> anyhow::Result<ElfSymbol> {
        let sym = self.sym();
        match &self.symtab {
            SymbolLocator::Section(sec) => sec.get_elf_symbol(sym),
            SymbolLocator::SymbolStructure(ss) => ss.get_elf_symbol(sym),
        }
    }

    pub(crate) fn offset(&self) -> u64 {
        self.offset
    }
}