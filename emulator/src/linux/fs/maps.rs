use crate::emulator::VMPointer;
use crate::linux::file_system::{FileIOTrait, SeekResult, StMode, SEEK_CUR, SEEK_END, SEEK_SET};
use crate::linux::structs::OFlag;
use std::fmt::Write;

pub struct Maps {
    pub path: String,
    pub oflags: u32,
    pub data: Vec<u8>,
    pub pos: usize,
}

impl Maps {
    pub fn new(path: &str, oflags: u32) -> Self {
        let data = generate_maps();
        Maps {
            path: path.to_string(),
            oflags,
            data: data.as_bytes().to_vec(),
            pos: 0,
        }
    }
}

fn generate_maps() -> String {
    let mut buf = String::new();

    write!(buf, "12c00000-52c00000 rw-p 00000000 00:00 0                                  [anon:dalvik-main space (region space)]\n").unwrap();
    write!(buf, "6fefc000-7102d000 rw-p 00000000 00:00 0                                  [anon:dalvik-/data/misc/apexdata/com.android.art/dalvik-cache/boot.art]\n").unwrap();
    write!(buf, "7102d000-712e4000 r--p 00000000 103:15 337                               /data/misc/apexdata/com.android.art/dalvik-cache/arm64/boot.oat\n").unwrap();
    write!(buf, "7cb3e00000-7cb3f69000 r--p 00000000 fe:25 85                             /apex/com.android.art/lib64/libart.so\n").unwrap();

    buf
}

impl<T: Clone> FileIOTrait<T> for Maps {
    fn close(&mut self) {}

    fn read(&mut self, buf: VMPointer<T>, count: usize) -> usize {
        let count = count.min(self.data.len() - self.pos);
        let data = &self.data[self.pos..self.pos + count];
        self.pos += count;
        buf.write_data(data).unwrap();
        count
    }

    fn pread(&mut self, buf: VMPointer<T>, count: usize, offset: usize) -> usize {
        panic!()
    }

    fn write(&mut self, buf: &[u8]) -> i32 {
        panic!()
    }

    fn lseek(&mut self, offset: i64, whence: i32) -> SeekResult {
        panic!()
    }

    fn path(&self) -> &str {
        self.path.as_str()
    }

    fn oflags(&self) -> OFlag {
        OFlag::from_bits_truncate(self.oflags)
    }

    fn st_mode(&self) -> StMode {
        StMode::from_bits_truncate(33060)
    }

    fn uid(&self) -> i32 {
        0
    }

    fn len(&self) -> usize {
        0
    }

    fn to_vec(&mut self) -> Vec<u8> {
        panic!()
    }
}


