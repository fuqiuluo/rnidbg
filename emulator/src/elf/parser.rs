use std::cell::{RefCell, UnsafeCell};
use std::io::Cursor;
use std::rc::Rc;
use anyhow::anyhow;
use bytes::{Buf, BytesMut};
use log::warn;
use crate::elf::memorized_object::MemoizedObject;
use crate::elf::section::{ElfSection, ElfSectionData};
use crate::elf::segment::ElfSegment;
use crate::elf::str_tab::ElfStringTable;

pub const FT_REL: u16 = 1;
pub const FT_EXEC: u16 = 2;
pub const FT_DYN: u16 = 3;
pub const FT_CORE: u16 = 4;

#[derive(Default)]
pub struct ElfFile {
    pub(crate) object_size: u8,
    pub(crate) encoding: u8,
    pub(crate) file_type: i16,
    pub(crate) arch: i16,
    pub(crate) version: i32,
    pub(crate) entry_point: i64,
    pub(crate) ph_offset: usize,
    pub(crate) sh_offset: usize,
    pub(crate) flags: i32,
    pub(crate) eh_size: i16,
    pub(crate) ph_entry_size: i16,
    pub(crate) num_ph: i16,
    pub(crate) sh_entry_size: i16,
    pub(crate) num_sh: i16,
    pub(crate) sh_string_ndx: i32,

    pub section_headers: Vec<ElfSection>,
    pub program_headers: Vec<MemoizedObject<ElfSegment>>,
}

impl ElfFile {
    pub fn from_buffer(buffer: Vec<u8>) -> Rc<UnsafeCell<ElfFile>> {
        let mut parser = ElfParser {
            buffer: Rc::new(UnsafeCell::new(Cursor::new(buffer))),
        };

        let mut elf_file = Self { ..Default::default() };

        let mut ident = [0u8; 16];
        parser.read(&mut ident);
        if ident[0..4] != [0x7F, 0x45, 0x4C, 0x46] {
            panic!("Invalid ELF magic");
        }

        //parser.object_size = ident[4];
        elf_file.object_size = ident[4];
        if elf_file.object_size == 1 {
            panic!("Only support 64bits elf file!")
        }
        elf_file.encoding = ident[5];

        if elf_file.encoding != 1 {
            warn!("Unsupported ELF encoding: {}", elf_file.encoding);
        }
        
        if ident[6] != 1 {
            panic!("Invalid ELF version");
        }
        
        elf_file.file_type = parser.read_short();
        elf_file.arch = parser.read_short();
        if elf_file.arch != 0xB7 {
            eprintln!("Unsupported architecture: {}", elf_file.arch);
        }
        elf_file.version = parser.read_int();
        elf_file.entry_point = parser.read_int_or_long();
        elf_file.ph_offset = parser.read_int_or_long() as usize;
        elf_file.sh_offset = parser.read_int_or_long() as usize;
        elf_file.flags = parser.read_int();
        elf_file.eh_size = parser.read_short();
        elf_file.ph_entry_size = parser.read_short();
        elf_file.num_ph = parser.read_short();
        elf_file.sh_entry_size = parser.read_short();
        elf_file.num_sh = parser.read_short();
        if elf_file.num_sh == 0 {
            panic!("e_shnum is SHN_UNDEF(0), which is not supported yet (the actual number of section header table entries is contained in the sh_size field of the section header at index 0)");
        }
        elf_file.sh_string_ndx = parser.read_short() as i32 & 0xffff;
        if elf_file.sh_string_ndx == 0xffff {
            panic!("e_shstrndx is SHN_XINDEX(0xffff), which is not supported yet (the actual index of the section name string table section is contained in the sh_link field of the section header at index 0)");
        }

        let elf_file = Rc::new(UnsafeCell::new(elf_file));
        let elf_file_mut = unsafe { &mut *elf_file.get() };
        let mut section_headers = Vec::with_capacity(elf_file_mut.num_sh as usize);
        for i in 0..elf_file_mut.num_sh {
            let section_header_offset = elf_file_mut.sh_offset + (i as usize * elf_file_mut.sh_entry_size as usize);
            let section = ElfSection::new(elf_file.clone(), parser.clone(), section_header_offset);
            section_headers.push(section);
        }
        elf_file_mut.section_headers = section_headers;

        let mut program_headers = Vec::with_capacity(elf_file_mut.num_ph as usize);
        for i in 0..elf_file_mut.num_ph {
            let program_header_offset = elf_file_mut.ph_offset + (i as usize * elf_file_mut.ph_entry_size as usize);
            let cloned_parser = parser.clone();
            let cloned_elf = elf_file.clone();
            program_headers.push(MemoizedObject::new(move || {
                Ok(ElfSegment::new(cloned_elf.clone(), cloned_parser.clone(), program_header_offset))
            }));
        }

        elf_file_mut.program_headers = program_headers;

        elf_file
    }

    pub fn get_program_header(&self, index: usize) -> Option<ElfSegment> {
        let header = self.program_headers.get(index)?;
        header.get_value()
            .map_err(|e| eprintln!("Error getting program header: {:?}", e))
            .ok()
    }

    pub fn get_section(&self, index: usize) -> Option<&ElfSection> {
        self.section_headers.get(index)
    }

    pub fn get_section_name_string_table(&self) -> anyhow::Result<ElfStringTable> {
        let section = self.get_section(self.sh_string_ndx as usize)
            .ok_or(anyhow!("Failed to get section: get_section_name_string_table"))?;
        match section.data {
            ElfSectionData::StringTable(ref string_table) => {
                string_table.get_value()
            }
            _ => Err(anyhow!("Section {} is not a string table", self.sh_string_ndx))
        }
    }

    pub fn virtual_memory_addr_to_file_offset(&self, address: u64) -> u64 {
        for i in 0..self.num_ph {
            let ph = self.get_program_header(i as usize)
                .expect("Failed to get program header");
            if address >= ph.virtual_address && address < (ph.virtual_address + ph.mem_size as u64) {
                let relative_offset = address - ph.virtual_address;
                if relative_offset >= ph.file_size as u64 {
                    panic!("Can not convert virtual memory address {} to file offset - found segment {:?} but address maps to memory outside file range", address, ph);
                }
                return ph.offset as u64 + relative_offset;
            }
        }
        panic!("Cannot find segment for address 0x{:x}", address);
    }

    pub fn get_string_table(&self) -> anyhow::Result<ElfStringTable> {
        self.find_str_tab_with_name(".strtab")
    }

    pub fn get_dynamic_string_table(&self) -> anyhow::Result<ElfStringTable> {
        self.find_str_tab_with_name(".dynstr")
    }

    pub fn find_str_tab_with_name(&self, name: &str) -> anyhow::Result<ElfStringTable> {
        for i in 0..self.num_sh {
            let sh = self.get_section(i as usize)
                .expect("Failed to get section");
            let sh_name= sh.name(self)?;
            if sh_name == name {
                if let ElfSectionData::StringTable(ref string_table) = sh.data {
                    return string_table.get_value()
                }
            }
        }
        Err(anyhow!("Failed to find string table with name: {}", name))
    }

    pub fn find_section_by_type(&self, typ: u32) -> anyhow::Result<ElfSection> {
        for i in 0..self.num_sh {
            let sh = self.get_section(i as usize)
                .expect("Failed to get section");
            if sh.typ == typ {
                return Ok(sh.clone());
            }
        }
        Err(anyhow!("Failed to find section by type: {}", typ))
    }
}

#[derive(Default, Clone)]
pub struct ElfParser {
    /// 频繁发生Clone，避免直接clone到数据体
    buffer: Rc<UnsafeCell<Cursor<Vec<u8>>>>,
}

impl ElfParser {
    fn buffer_mut(&self) -> &mut Cursor<Vec<u8>> {
        unsafe { &mut *self.buffer.get() }
    }

    pub fn seek(&self, offset: usize) {
        self.buffer_mut().set_position(offset as u64);
    }

    pub fn read(&self, dest: &mut [u8]) {
        self.buffer_mut().copy_to_slice(dest);
    }

    pub fn read_byte(&self) -> i8 {
        self.buffer_mut().get_i8()
    }

    pub fn read_ubyte(&self) -> u8 {
        self.buffer_mut().get_u8()
    }

    pub fn read_short(&self) -> i16 {
        self.buffer_mut().get_i16_le()
    }

    pub fn read_int(&self) -> i32 {
        self.buffer_mut().get_i32_le()
    }

    pub fn read_long(&self) -> i64 {
        self.buffer_mut().get_i64_le()
    }

    pub fn read_int_or_long(&self) -> i64 {
/*        if self.object_size == 1 {
            self.read_int() as i64
        } else {
            self.read_long()
        }*/
        self.read_long()
    }
}