use std::fs::File;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::marker::PhantomData;
use log::warn;
use crate::emulator::VMPointer;
use crate::linux::file_system::{FileIOTrait, ST_DEV, StMode, SeekResult};
use crate::linux::structs::{OFlag, Stat64, Timespec};

pub struct LinuxFileIO<T: Clone> {
    pub fake_path: String,
    pub data: File,
    pub oflags: u32,
    pub uid: i32,
    pub st_mode: StMode,
    pub pd: PhantomData<T>,
}

impl<T: Clone> LinuxFileIO<T> {
    pub fn new(path: &str, fake_path: &str, oflags: u32, uid: i32, st_mode: StMode) -> Self {
        let Ok(file) = File::open(path) else {
            if option_env!("EMU_LOG") == Some("1") {
                warn!("failed to open file: {}", path);
            }
            panic!("failed to open file: {}", path);
        };
        if file.metadata().unwrap().is_dir() {
            if option_env!("EMU_LOG") == Some("1") {
                warn!("{} is a directory", path);
            }
            panic!("{} is a directory", path);
        }
        LinuxFileIO {
            fake_path: fake_path.to_string(),
            data: file,
            oflags,
            uid,
            st_mode,
            pd: PhantomData
        }
    }

    pub fn new_with_file(file: File, fake_path: &str, oflags: u32, uid: i32, st_mode: StMode) -> Self {
        LinuxFileIO {
            fake_path: fake_path.to_string(),
            data: file,
            oflags,
            uid,
            st_mode,
            pd: PhantomData
        }
    }
}

impl<T: Clone> FileIOTrait<T> for LinuxFileIO<T> {
    fn close(&mut self) {
        self.data.seek(SeekFrom::Start(0)).unwrap();
    }

    fn read(&mut self, buf: VMPointer<T>, count: usize) -> usize {
        let mut buffer = vec![0; count];
        let read = self.data.read(&mut buffer).unwrap();
        buf.write_data(&buffer.as_slice()[..read]).expect("failed to write buffer");
        read
    }

    fn pread(&mut self, buf: VMPointer<T>, count: usize, offset: usize) -> usize {
        self.data.seek(SeekFrom::Start(offset as u64)).unwrap();
        self.read(buf, count)
    }

    fn write(&mut self, buf: &[u8]) -> i32 {
        let ret = if self.oflags & 0x400 != 0 {
            // append
            self.data.seek(SeekFrom::End(0)).unwrap();
            self.data.write(buf).unwrap() as i32
        } else {
            self.data.write(buf).unwrap() as i32
        };
        ret
    }

    fn lseek(&mut self, offset: i64, whence: i32) -> SeekResult {
        let seek_from = match whence {
            0 => SeekFrom::Start(offset as u64),
            1 => SeekFrom::Current(offset),
            2 => SeekFrom::End(offset),
            _ => return SeekResult::WhenceError
        };
        if let Ok(off) = self.data.seek(seek_from) {
            return SeekResult::Ok(off as i64)
        }
        SeekResult::OffsetError
    }

    fn path(&self) -> &str {
        &self.fake_path
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
        self.data.metadata().unwrap().len() as usize
    }

    fn to_vec(&mut self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.data.seek(SeekFrom::Start(0)).unwrap();
        std::io::copy(&mut self.data, &mut buf).unwrap();
        buf
    }
}

#[test]
fn stat64_test() {
    //         return Arrays.asList("st_dev", "st_ino", "st_mode", "st_nlink", "st_uid", "st_gid", "st_rdev", "__pad1", "st_size", "st_blksize",
    //                 "__pad2", "st_blocks", "st_atim", "st_mtim", "st_ctim", "__unused4", "__unused5");
    //   dev_t st_dev; \
    //   ino_t st_ino; \
    //   mode_t st_mode; \
    //   nlink_t st_nlink; \
    //   uid_t st_uid; \
    //   gid_t st_gid; \

    //   dev_t st_rdev; \
    //   unsigned long __pad1; \


    //   off_t st_size; \
    //   int st_blksize; \
    //   int __pad2; \
    //   long st_blocks; \


    //   struct timespec st_atim; \
    //   struct timespec st_mtim; \
    //   struct timespec st_ctim; \
    //   unsigned int __unused4; \
    //   unsigned int __unused5; \
    #[repr(C)]
    pub struct MyStat64 {
        pub st_dev: i64,
        pub st_ino: i64,
        pub st_mode: i32,
        pub st_nlink: i32,
        pub st_uid: i32,
        pub st_gid: i32,
        pub st_rdev: i64,
        pub __pad1: u64,
        pub st_size: i64,
        pub st_blksize: i32,
        pub __pad2: i32,
        pub st_blocks: i64,

        pub st_atime: Timespec,
        pub st_mtime: Timespec,
        pub st_ctime: Timespec,

        pub __unused4: u32,
        pub __unused5: u32
    }

    const STAT_SIZE: usize = std::mem::size_of::<MyStat64>();

    assert_eq!(std::mem::size_of::<Timespec>(), 16);
    assert_eq!(STAT_SIZE, 128);
}