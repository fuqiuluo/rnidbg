use std::cell::UnsafeCell;
use std::fmt::{Debug, Formatter};
use std::io::Cursor;
use std::rc::Rc;
use bytes::Buf;
use crate::elf::parser::ElfParser;

#[derive(Clone)]
pub struct ElfStringTable {
    buffer: Rc<UnsafeCell<Cursor<Vec<u8>>>>
}

impl Debug for ElfStringTable {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ElfStringTable")
            .finish()
    }
}

impl ElfStringTable {
    pub fn new(parser: ElfParser, offset: usize, length: usize) -> Self {
        parser.seek(offset);

        let mut buffer = vec![0u8; length];
        parser.read(&mut buffer);

        Self {
            buffer: Rc::new(UnsafeCell::new(Cursor::new(buffer)))
        }
    }

    fn buffer_mut(&self) -> &mut Cursor<Vec<u8>> {
        unsafe { &mut *self.buffer.get() }
    }

    pub fn get(&self, index: usize) -> String {
        self.buffer_mut().set_position(index as u64);
        let mut bytes = vec![];
        loop {
            let byte = self.buffer_mut().get_u8();
            if byte == 0 {
                break;
            }
            bytes.push(byte);
        }
        String::from_utf8(bytes).unwrap()
    }
}