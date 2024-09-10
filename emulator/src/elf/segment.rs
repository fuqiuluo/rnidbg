use std::cell::UnsafeCell;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use crate::elf::parser::{ElfFile, ElfParser};
use crate::elf::abi::{*};
use crate::elf::dynamic_struct::ElfDynamicStructure;
use crate::elf::memorized_object::{MemoizedObject};
use crate::elf::pt::{ArmExIdx, GnuEhFrameHeader, PtLoadData};

#[derive(Clone)]
pub enum ElfSegmentData {
    PtLoad(MemoizedObject<PtLoadData>),
    DynamicStructure(MemoizedObject<ElfDynamicStructure>),
    GnuEhFrameHeader(MemoizedObject<GnuEhFrameHeader>),
    ArmExIdx(MemoizedObject<ArmExIdx>),
    Unknown(),
}

#[derive(Clone)]
pub struct ElfSegment {
    pub typ: u32,
    pub flags: i32,
    pub offset: i64,
    pub virtual_address: u64,
    pub physical_address: u64,
    pub file_size: i64,
    pub mem_size: i64,
    pub align: u64,
    pub data: ElfSegmentData
}

impl Debug for ElfSegment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ElfSegment")
            .field("typ", &self.typ)
            .field("flags", &self.flags)
            .field("offset", &self.offset)
            .finish()
    }
}

impl ElfSegment {
    pub fn new(elf_file: Rc<UnsafeCell<ElfFile>>, parser: ElfParser, offset: usize) -> Self {
        parser.seek(offset);

        let typ = parser.read_int() as u32;
        let flags = parser.read_int();
        let segment_offset = parser.read_int_or_long();
        let virtual_address = parser.read_int_or_long() as u64;
        let physical_address = parser.read_int_or_long() as u64;
        let file_size = parser.read_int_or_long();
        let mem_size = parser.read_int_or_long();
        let align = parser.read_int_or_long() as u64;

        let mut segment = ElfSegment {
            typ, flags,
            offset: segment_offset, virtual_address, physical_address, file_size, mem_size, align,
            data: ElfSegmentData::Unknown(),
        };

        match typ {
            PT_INTERP => {
            }
            PT_LOAD => {
                let clone_parser = parser.clone();
                segment.data = ElfSegmentData::PtLoad(MemoizedObject::new(move || {
                    clone_parser.seek(segment_offset as usize);
                    let mut data = vec![0; file_size as usize];
                    clone_parser.read(&mut data);
                    Ok(PtLoadData {
                        buffer: data,
                    })
                }));
            }
            PT_DYNAMIC => {
                let clone_parser = parser.clone();
                let elf_file_c = elf_file.clone();
                let offset = unsafe { &*elf_file.get() }.virtual_memory_addr_to_file_offset(virtual_address) as usize;
/*                segment.dynamic_structure.set_compute_value(move || {
                    Ok(ElfDynamicStructure::new(elf_file_c.clone(), clone_parser.clone(), offset, mem_size as usize))
                });*/
                segment.data = ElfSegmentData::DynamicStructure(MemoizedObject::new(move || {
                    Ok(ElfDynamicStructure::new(elf_file_c.clone(), clone_parser.clone(), offset, mem_size as usize))
                }));
            }
            PT_GNU_EH_FRAME => {
                let clone_parser = parser.clone();
                let offset = unsafe { &*elf_file.get() }.virtual_memory_addr_to_file_offset(virtual_address) as usize;
/*                segment.eh_frame_header.set_compute_value(move || {
                    Ok(GnuEhFrameHeader::new(clone_parser.clone(), offset, mem_size as usize, virtual_address))
                });*/
                segment.data = ElfSegmentData::GnuEhFrameHeader(MemoizedObject::new(move || {
                    Ok(GnuEhFrameHeader::new(clone_parser.clone(), offset, mem_size as usize, virtual_address))
                }));
            }
            PT_ARM_EXIDX => {
                let clone_parser = parser.clone();
                let offset = segment_offset;
/*                segment.arm_exidx.set_compute_value(move || {
                    clone_parser.seek(offset as usize);
                    let mut buffer = vec![0u8; file_size as usize];
                    clone_parser.read(&mut buffer);
                    Ok(ArmExIdx::new(virtual_address, buffer))
                });*/
                segment.data = ElfSegmentData::ArmExIdx(MemoizedObject::new(move || {
                    clone_parser.seek(offset as usize);
                    let mut buffer = vec![0u8; file_size as usize];
                    clone_parser.read(&mut buffer);
                    Ok(ArmExIdx::new(virtual_address, buffer))
                }));
            }
            _ => {}
        }

        segment
    }
}
