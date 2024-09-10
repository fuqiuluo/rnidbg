#![allow(non_camel_case_types)]

use std::collections::VecDeque;
use crate::emulator::VMPointer;
use crate::linux::file_system::{FileIOTrait, SeekResult, StMode};
use crate::linux::structs::{Dirent, OFlag};

#[derive(Copy, Clone)]
pub enum DirentType {
    DT_FIFO = 1,
    DT_CHR = 2,
    DT_DIR = 4,
    DT_BLK = 6,
    DT_REG = 8,
    DT_LNK = 10,
    DT_SOCK = 12,
    DT_WHT = 14,
}

#[derive(Clone)]
pub struct DirectionEntry {
    pub direction_type: DirentType,
    pub name: String,
}

impl DirectionEntry {
    pub fn new(is_file: bool, name: &str) -> Self {
        DirectionEntry {
            direction_type: if is_file { DirentType::DT_REG } else { DirentType::DT_DIR },
            name: name.to_string()
        }
    }

    pub fn new_with_type(direction_type: DirentType, name: &str) -> Self {
        DirectionEntry {
            direction_type,
            name: name.to_string()
        }
    }
}

#[derive(Clone)]
pub struct Direction {
    pub files: VecDeque<DirectionEntry>,
    pub path: String,
    pub off: usize
}

impl Direction {
    pub fn new(mut files: VecDeque<DirectionEntry>, path: &str) -> Self {
        files.push_front(DirectionEntry::new(false, ".."));
        files.push_front(DirectionEntry::new(false, "."));
        Direction {
            files,
            path: path.to_string(),
            off: 1
        }
    }
}

impl<T: Clone> FileIOTrait<T> for Direction {
    fn close(&mut self) {}

    fn read(&mut self, buf: VMPointer<T>, count: usize) -> usize {
        panic!("Direction is a directory");
    }

    fn pread(&mut self, buf: VMPointer<T>, count: usize, offset: usize) -> usize {
        panic!("Direction is a directory");
    }

    fn write(&mut self, buf: &[u8]) -> i32 {
        panic!("Direction is a directory");
    }

    fn lseek(&mut self, offset: i64, whence: i32) -> SeekResult {
        panic!("Direction is a directory");
    }

    fn path(&self) -> &str {
        &self.path
    }

    fn getdents64(&mut self, dirp: VMPointer<T>, size: usize) -> i32 {
        let mut offset = 0;
        let random = rand::random::<u64>();

        self.files.retain(|entry| {
            let data = entry.name.as_bytes();
            let d_reclen = ((data.len() + 24 - 1) / 8 + 1) * 8;

            if offset + d_reclen >= size {
                return true;
            }

            let dirent_ptr = dirp.share(offset as i64);
            let mut buf = [0u8; size_of::<Dirent>()];
            let dirent = unsafe {
                &mut *(buf.as_mut_ptr() as *mut Dirent)
            };
            dirent.d_ino = random + offset as u64;
            dirent.d_off = self.off as i64;
            dirent.d_reclen = d_reclen as u16;
            dirent.d_type = entry.direction_type as u8;
            dirent.d_name[..data.len()].copy_from_slice(data);

            dirent_ptr.write_data(&buf).unwrap();

            offset += d_reclen;
            self.off += 1;

            false
        });

        offset as i32
    }

    fn oflags(&self) -> OFlag {
        OFlag::empty()
    }

    fn st_mode(&self) -> StMode {
        StMode::APP_PRIVATE_DIR
    }

    fn uid(&self) -> i32 {
        0
    }

    fn len(&self) -> usize {
        0
    }

    fn to_vec(&mut self) -> Vec<u8> {
        panic!("Direction is a directory");
    }
}