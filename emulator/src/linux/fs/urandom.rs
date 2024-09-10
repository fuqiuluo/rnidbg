use std::marker::PhantomData;
use rand::Rng;
use crate::emulator::VMPointer;
use crate::linux::file_system::{FileIOTrait, SeekResult, StMode};
use crate::linux::structs::OFlag;

pub struct URandom {
    pub path: String,
    pub oflags: u32,
    pub uid: i32,
    pub st_mode: StMode,
}

impl URandom {
    pub fn new(path: &str, oflags: u32, uid: i32, st_mode: StMode) -> Self {
        URandom {
            path: path.to_string(),
            oflags,
            uid,
            st_mode,
        }
    }
}

impl<T: Clone> FileIOTrait<T> for URandom {
    fn close(&mut self) {

    }

    fn read(&mut self, buf: VMPointer<T>, count: usize) -> usize {
        let mut rng = rand::thread_rng();
        let random_bytes: Vec<u8> = (0..count).map(|_| rng.random::<u8>()).collect();
        buf.write_buf(random_bytes).expect("failed to write buffer");
        count
    }

    fn pread(&mut self, buf: VMPointer<T>, count: usize, offset: usize) -> usize {
        todo!()
    }

    fn write(&mut self, buf: &[u8]) -> i32 {
        -1
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
        self.st_mode.clone()
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
