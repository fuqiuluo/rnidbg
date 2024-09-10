use crate::elf::parser::ElfParser;

#[derive(Clone)]
pub struct ElfInitArray {
    pub(crate) array: Vec<i64>
}

impl ElfInitArray {
    pub fn new(parser: ElfParser, offset: usize, size: usize) -> Self {
        parser.seek(offset);

        let mut array = Vec::new();
/*        let size = if parser.object_size == 1 {
            size / 4
        } else {
            size / 8
        };*/
        let size = size / 8;
        for _ in 0..size {
            array.push(parser.read_int_or_long());
        }

        Self {
            array
        }
    }
}