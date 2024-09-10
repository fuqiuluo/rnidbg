use bytes::BytesMut;
use crate::backend::Backend;
use crate::elf::parser::ElfParser;

#[derive(Clone)]
pub struct PtLoadData {
    pub(crate) buffer: Vec<u8>,
}

impl PtLoadData {
    pub fn write_to<'a, T: Clone>(&self, address: u64, backend: &Backend<T>) {
        backend.mem_write(address, self.buffer.as_slice())
            .expect("Failed to write PT_LOAD data");
    }
}

#[derive(Clone)]
pub struct GnuEhFrameHeader {
    parser: ElfParser,
    entries: Vec<TableEntry>,
}

impl GnuEhFrameHeader {
    pub fn new(parser: ElfParser, offset: usize, mem_size: usize, virtual_address: u64) -> Self {
        parser.seek(offset);

        let mut off = Off{ pos: offset, init: offset };
        let version = parser.read_byte(); off.pos += 1;
        if version != 1 {
            panic!("Invalid version in .eh_frame_hdr section: {}", version);
        }

        let delta = virtual_address - offset as u64;
        let eh_frame_ptr_enc = parser.read_ubyte(); off.pos += 1;
        let fde_count_enc = parser.read_ubyte(); off.pos += 1;
        let table_enc = parser.read_ubyte(); off.pos += 1;
        let eh_frame_ptr = read_encoded_pointer(&parser, eh_frame_ptr_enc, &mut off, true);
        let fde_count = read_encoded_pointer(&parser, fde_count_enc, &mut off, true);
        let mut entries = vec![];
        for _ in 0..fde_count {
            let location = read_encoded_pointer(&parser, table_enc, &mut off, true) + delta;
            let address = read_encoded_pointer(&parser, table_enc, &mut off, true);
            entries.push(TableEntry{ location, address });
        }

        if off.pos - off.init != mem_size {
            panic!("size={} pos={}", mem_size, off.pos);
        }

        Self {
            parser,
            entries,
        }
    }
}

struct Off {
    pub(super) pos: usize,
    pub(super) init: usize,
}

#[derive(Clone)]
pub struct TableEntry {
    pub location: u64,
    pub address: u64,
}

fn read_encoded_pointer(parser: &ElfParser, encoding: u8, off: &mut Off, check_indirect: bool) -> u64 {
    if encoding == 0xff {
        return 0;
    }
    let last_pos = off.pos;
    let mut result = 0;
    match encoding & 0xf {
        0x01 => {
            result = read_uleb128(parser, off);
        }
        0x0D => {
            result = parser.read_byte() as u64;
            off.pos += 1;
        }
        0x02 => {
            result = (parser.read_short() as i64 & 0xffffi64) as u64;
            off.pos += 2;
        }
        0x03 => {
            result = (parser.read_int() as i64 & 0xffffffff) as u64;
            off.pos += 4;
        }
        0x0A => {
            result = parser.read_short() as u64;
            off.pos += 2;
        }
        0x0B => {
            result = parser.read_int() as u64;
            off.pos += 4;
        }
        0x04 | 0x0C => {
            result = parser.read_long() as u64;
            off.pos += 8;
        }
        0x09 | _ => panic!("Invalid encoding: 0x{:X}", encoding),
    }

    match encoding & 0x70 {
        0x00 => {}
        0x10 => {
            result += last_pos as u64;
        }
        0x30 => {
            result += off.init as u64;
        }
        _ => {
            panic!("Invalid encoding: 0x{:X}", encoding);
        }
    }

    if encoding & 0x80 != 0 {
        if check_indirect {
            panic!("DW_EH_PE_indirect: Indirect encoding not supported: 0x{:X}", encoding);
        }
    }

    result
}

fn read_uleb128(parser: &ElfParser, off: &mut Off) -> u64 {
    let mut result = 0;
    let mut shift = 0;
    loop {
        let b = (parser.read_byte() & 0x7f) as u64;
        off.pos += 1;
        result |= b << shift;
        shift += 7;
        if (b & 0x80) == 0 {
            break result;
        }
    }
}

#[derive(Clone)]
pub struct ArmExIdx {
    buffer: BytesMut,
    virtual_address: u64
}

impl ArmExIdx {
    pub fn new(virtual_address: u64, buffer: Vec<u8>) -> Self {
        Self {
            buffer: BytesMut::from(buffer.as_slice()),
            virtual_address
        }
    }
}