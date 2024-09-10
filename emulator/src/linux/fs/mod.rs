pub mod linux_file;
pub(crate) mod urandom;
pub(crate) mod meminfo;
pub(crate) mod cpuinfo;
pub(crate) mod random_boot_id;
pub mod direction;
pub(crate) mod maps;

use std::io::{Cursor, Read};
use std::marker::PhantomData;
use crate::emulator::VMPointer;
use crate::linux::file_system::{*};
use crate::linux::structs::OFlag;

pub struct ByteArrayFileIO<T: Clone> {
    pub data: Cursor<Vec<u8>>,
    pub path: String,
    pub uid: i32,
    pub oflags: u32,
    pub st_mode: StMode,
    pub pd: PhantomData<T>
}

impl<T: Clone> ByteArrayFileIO<T> {
    pub fn new(data: Vec<u8>, path: String, uid: i32, oflags: u32, st_mode: StMode) -> Self {
        ByteArrayFileIO {
            data: Cursor::new(data),
            path,
            uid,
            st_mode,
            oflags,
            pd: PhantomData
        }
    }
}

impl<T: Clone> FileIOTrait<T> for ByteArrayFileIO<T> {
    fn close(&mut self) {
       self.data.set_position(0);
    }

    fn read(&mut self, buf: VMPointer<T>, mut count: usize) -> usize {
        if self.data.position() >= self.len() as u64 {
            return 0;
        }

        let remaining = self.len() - self.data.position() as usize;
        if count > remaining {
            count = remaining;
        }

        let mut buffer = vec![0; count];
        self.data.read_exact(&mut buffer).unwrap();
        buf.write_buf(buffer).expect("failed to write buffer");

        count
    }

    fn pread(&mut self, buf: VMPointer<T>, count: usize, offset: usize) -> usize {
        self.data.set_position(offset as u64);
        self.read(buf, count)
    }

    fn write(&mut self, buf: &[u8]) -> i32 { panic!("write file: {} is not supported: BytesFileIO", self.path); }

    fn lseek(&mut self, offset: i64, whence: i32) -> SeekResult {
        SeekResult::Ok(match whence {
            SEEK_SET => {
                self.data.set_position(offset as u64);
                offset
            }
            SEEK_CUR => {
                let pos = self.data.position() as i64;
                self.data.set_position((pos + offset) as u64);
                pos + offset
            }
            SEEK_END => {
                let pos = self.data.get_ref().len() as i64;
                self.data.set_position((pos + offset) as u64);
                pos + offset
            }
            _ => return SeekResult::WhenceError
        })
    }

    fn path(&self) -> &str {
        &self.path
    }

    fn oflags(&self) -> OFlag {
        OFlag::from_bits_truncate(self.oflags)
    }

    fn st_mode(&self) -> StMode {
        self.st_mode.clone()
    }

    fn uid(&self) -> i32 {
        self.uid
    }

    fn len(&self) -> usize {
        self.data.get_ref().len()
    }

    fn to_vec(&mut self) -> Vec<u8> {
        self.data.get_ref().clone()
    }
}

