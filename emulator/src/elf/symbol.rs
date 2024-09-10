use anyhow::anyhow;
use crate::elf::abi::{*};
use crate::elf::parser::{ElfFile, ElfParser};
use crate::elf::str_tab::ElfStringTable;

const SHN_UNDEF: i16 = 0;

#[derive(Clone, Debug)]
pub struct ElfSymbol {
    pub name_ndx: i32,
    pub value: i64,
    pub size: i64,
    pub info: i8,
    pub other: i8,
    pub section_header_ndx: i16,
    pub section_type: i32,
    pub offset: usize,
    string_table: Option<ElfStringTable>,
}

impl ElfSymbol {
    pub fn new(parser: ElfParser, offset: usize, typ: i32) -> Self {
        parser.seek(offset);

        let name_ndx;
        let value;
        let size;
        let info;
        let other;
        let section_header_ndx;
/*        if parser.object_size == 1 { // CLASS_32
            name_ndx = parser.read_int();
            value = parser.read_int_or_long();
            size = parser.read_int_or_long();
            info = parser.read_byte();
            other = parser.read_byte();
            section_header_ndx = parser.read_short();
        } else {
            name_ndx = parser.read_int();
            info = parser.read_byte();
            other = parser.read_byte();
            section_header_ndx = parser.read_short();
            value = parser.read_long();
            size = parser.read_long();
        }*/
        name_ndx = parser.read_int();
        info = parser.read_byte();
        other = parser.read_byte();
        section_header_ndx = parser.read_short();
        value = parser.read_long();
        size = parser.read_long();

        Self {
            name_ndx,
            value,
            size,
            info,
            other,
            section_header_ndx,
            section_type: typ,
            offset,
            string_table: None
        }
    }

    pub fn binding(&self) -> i32 {
        (self.info >> 4) as i32
    }

    pub fn matches(&self, soaddr: u64) -> bool {
        let value = self.value & !1i64;
        return self.section_header_ndx != SHN_UNDEF &&
            soaddr >= value as u64 &&
            soaddr < value as u64 + self.size as u64;
    }

    pub fn set_string_table(&mut self, string_table: ElfStringTable) {
        self.string_table = Some(string_table);
    }

    pub fn is_undefined(&self) -> bool {
        self.section_header_ndx == SHN_UNDEF
    }

    pub fn name(&self, elf_file: &ElfFile) -> anyhow::Result<String> {
        if self.name_ndx == 0 {
            return Err(anyhow!("Symbol name index is 0"));
        }

        if let Some(strtab) = &self.string_table {
            return Ok(strtab.get(self.name_ndx as usize));
        }
        
        if self.section_type as u32 == SHT_SYMTAB {
            let str_tab = elf_file.get_string_table()?;
            return Ok(str_tab.get(self.name_ndx as usize));
        }

        if self.section_type as u32 == SHT_DYNSYM {
            let dyn_str_tab = elf_file.get_dynamic_string_table()?;
            return Ok(dyn_str_tab.get(self.name_ndx as usize));
        }
        
        Err(anyhow!("Failed to get symbol name"))
    }
}