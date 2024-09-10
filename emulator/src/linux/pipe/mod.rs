use std::cell::UnsafeCell;
use std::rc::Rc;
use std::sync::atomic::AtomicU8;
use crate::emulator::VMPointer;
use crate::linux::file_system::{FileIOTrait, SeekResult, StMode};
use crate::linux::structs::OFlag;

#[derive(Clone)]
pub struct Pipe {
    pub buf: Rc<UnsafeCell<Vec<u8>>>,
    pub count: Rc<AtomicU8>
}

impl Pipe {
    pub fn new() -> Self {
        Self {
            buf: Rc::new(UnsafeCell::new(Vec::new())),
            count: Rc::new(AtomicU8::new(0))
        }
    }
}

impl<T: Clone> FileIOTrait<T> for Pipe {
    fn close(&mut self) {
        todo!()
    }

    fn read(&mut self, buf: VMPointer<T>, count: usize) -> usize {
        todo!()
    }

    fn pread(&mut self, buf: VMPointer<T>, count: usize, offset: usize) -> usize {
        todo!()
    }

    fn write(&mut self, buf: &[u8]) -> i32 {
        todo!()
    }

    fn lseek(&mut self, offset: i64, whence: i32) -> SeekResult {
        todo!()
    }

    fn path(&self) -> &str {
        todo!()
    }

    fn oflags(&self) -> OFlag {
        todo!()
    }

    fn st_mode(&self) -> StMode {
        todo!()
    }

    fn uid(&self) -> i32 {
        todo!()
    }

    fn len(&self) -> usize {
        todo!()
    }

    fn to_vec(&mut self) -> Vec<u8> {
        todo!()
    }
}