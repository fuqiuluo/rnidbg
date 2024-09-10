#![allow(clippy::wrong_self_convention)]

use std::hash::{DefaultHasher, Hash, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::anyhow;
use bitflags::bitflags;
use bytes::Bytes;
use sparse_list::SparseList;
use crate::emulator::AndroidEmulator;
use crate::linux::fs::ByteArrayFileIO;
use crate::linux::fs::direction::Direction;
use crate::linux::fs::linux_file::LinuxFileIO;
use crate::linux::sock::local_socket::LocalSocket;
use crate::linux::structs::{OFlag, Stat64, Timespec};
use crate::pointer::VMPointer;

pub const SEEK_SET: i32 = 0;
pub const SEEK_CUR: i32 = 1;
pub const SEEK_END: i32 = 2;

#[derive(Debug)]
pub enum SeekResult {
    Ok(i64),
    WhenceError,
    OffsetError,
    UnknownError
}

pub const ST_DEV: i64 = 2054;

bitflags! {
    // StMode(S_IFREG | S_IRUSR | S_IWUSR | S_IRGRP | S_IROTH)
    #[derive(Debug, PartialEq, Clone)]
    pub struct StMode: u32 {
        /// regular file
        const S_IFREG = 1 << 15;
        /// directory
        const S_IFDIR = 1 << 14;
        /// character device
        const S_IFCHR = 1 << 13;
        /// 是否设置 uid/gid/sticky
        //const S_ISUID = 1 << 14;
        //const S_ISGID = 1 << 13;
        //const S_ISVTX = 1 << 12;
        /// user-read permission
        const S_IRUSR = 1 << 8;
        /// user-write permission
        const S_IWUSR = 1 << 7;
        /// user-execute permission
        const S_IXUSR = 1 << 6;
        /// group-read permission
        const S_IRGRP = 1 << 5;
        /// group-write permission
        const S_IWGRP = 1 << 4;
        /// group-execute permission
        const S_IXGRP = 1 << 3;
        /// other-read permission
        const S_IROTH = 1 << 2;
        /// other-write permission
        const S_IWOTH = 1 << 1;
        /// other-execute permission
        const S_IXOTH = 1 << 0;
        /// exited-user-process status
        const WIMTRACED = 1 << 1;
        /// continued-process status
        const WCONTINUED = 1 << 3;

        /// 系统普通文件（配置文件/安装包/动态库）
        const SYSTEM_FILE = 33188;
        /// 系统可执行文件
        const SYSTEM_EXECUTABLE = 33261;
        /// 系统私有文件
        const SYSTEM_PRIVATE_FILE = 33152;
        /// 系统文件夹
        const SYSTEM_DIR = 16889;
        /// 应用程序文件
        const APP_FILE = 33152;
        /// 应用程序私有文件夹
        const APP_PRIVATE_DIR = 16832;
    }
}

pub enum FileIO<T: Clone> {
    Bytes(ByteArrayFileIO<T>),
    File(LinuxFileIO<T>),
    Direction(Direction),
    Dynamic(Box<dyn FileIOTrait<T>>),
    LocalSocket(LocalSocket),
    Error(i32)
}

pub struct FileIOEntry<T: Clone> {
    pub file: FileIO<T>,
    pub fd: i32,
    pub owner_pid: u32,
    pub access_pid: Vec<u32>
}

pub struct AndroidFileSystem<T: Clone> {
    fd_map: SparseList<FileIO<T>>,
    pub file_resolver: Option<Box<dyn Fn(&AndroidFileSystem<T>, &str, OFlag, i32) -> Option<FileIO<T>>>>
}

impl<T: Clone> AndroidFileSystem<T> {
    pub(crate) fn new() -> Self {
        AndroidFileSystem {
            fd_map: SparseList::new(),
            file_resolver: None
        }
    }

    pub(crate) fn insert_file(&mut self, file: FileIO<T>) -> i32 {
        self.fd_map.insert(file) as i32
    }

    pub(crate) fn get_file_mut(&mut self, fd: i32) -> Option<&mut FileIO<T>> {
        self.fd_map.get_mut(fd as usize)
    }

    pub(crate) fn remove_file(&mut self, fd: i32) -> Option<FileIO<T>> {
        let ret = self.fd_map.remove(fd as usize);
        ret
    }

    ///Set up a file handler to handle file redirection
    ///
    /// # Example
    /// ```no_ru
    /// use core::emulator::AndroidEmulator;
    /// use emulator::linux::file_system::{FileIO, FileIOTrait, StMode};
    /// use emulator::linux::fs::linux_file::LinuxFileIO;
    /// use emulator::linux::errno::Errno;
    ///
    /// let emulator = AndroidEmulator::create_arm64(32267, 29427, "com.tencent.mobileqq:MSF", ());
    /// let file_system = emulator.get_file_system();
    /// file_system.set_file_resolver(Box::new(|file_system, path, flags, mode| {
    ///     if path == "/data/local/tmp/fuqiuluo-114514" {
    ///         return Some(FileIO::File(LinuxFileIO::new(&path, flags.bits(), 0, StMode::SYSTEM_FILE)));
    ///     } else if path == "/not/exists/file" {
    ///         return None;
    ///     } else if path == "/unreachable/file" {
    ///         return Some(FileIO::Error(Errno::ENOENT.as_i32()));
    ///     }
    ///
    ///     panic!("unresolved file_resolver: path={}, flags={:?}, mode={}", path, flags, mode);
    /// }));
    /// ```
    ///
    /// # Arguments
    /// * `resolver` - A function that takes a file system, path, flags, and mode as arguments and returns an option of FileIO
    pub fn set_file_resolver(&mut self, resolver: Box<dyn Fn(&AndroidFileSystem<T>, &str, OFlag, i32) -> Option<FileIO<T>>>) {
        self.file_resolver = Some(resolver);
    }
}

pub trait FileIOTrait<T: Clone> {
    fn close(&mut self);

    fn read(&mut self, buf: VMPointer<T>, count: usize) -> usize;

    fn pread(&mut self, buf: VMPointer<T>, count: usize, offset: usize) -> usize;

    fn write(&mut self, buf: &[u8]) -> i32;

    fn lseek(&mut self, offset: i64, whence: i32) -> SeekResult;

    fn path(&self) -> &str;

    fn connect(&mut self, _addr: VMPointer<T>, _addr_len: usize, _emulator: &AndroidEmulator<T>) -> i32 {
        panic!("connect not implemented");
    }

    fn getdents64(&mut self, _dirp: VMPointer<T>, _size: usize) -> i32 {
        panic!("getdents64 not implemented");
    }

    fn fstat(&self, stat_pointer: VMPointer<T>) {
        const STAT_SIZE: usize = size_of::<Stat64>();
        let mut buf = stat_pointer.read_bytes_with_len(STAT_SIZE).unwrap();

        let stat: &mut Stat64 = unsafe {
            &mut *(buf.as_mut_ptr() as *mut Stat64)
        };

        let (timestamp, timestamp_subsec_nanos) = if let Ok(duration_since_epoch) = SystemTime::now().duration_since(UNIX_EPOCH) {
            (duration_since_epoch.as_secs(), duration_since_epoch.subsec_nanos())
        } else {
            panic!("SystemTime before UNIX EPOCH!");
        };
        let now_time_spec = Timespec {
            tv_sec: timestamp as i64,
            tv_nsec: timestamp_subsec_nanos as i64
        };

        stat.st_dev = ST_DEV;
        stat.st_ino = self.hash() as i64;
        stat.st_mode = self.st_mode().bits();
        stat.st_nlink = 1;
        stat.st_uid = self.uid();
        stat.st_gid = self.uid();
        stat.st_rdev = 0;
        stat.st_size = self.len();
        stat.st_blksize = 4096;
        stat.st_blocks = (self.len() as f64 / 512.0).ceil() as i64;
        stat.st_atime = now_time_spec.clone();
        stat.st_mtime = now_time_spec.clone();
        stat.st_ctime = now_time_spec;

        stat_pointer.write_buf(buf).unwrap();
    }

    fn fstat2(&self) -> Stat64 {
        let mut stat = Stat64::default();

        let (timestamp, timestamp_subsec_nanos) = if let Ok(duration_since_epoch) = SystemTime::now().duration_since(UNIX_EPOCH) {
            (duration_since_epoch.as_secs(), duration_since_epoch.subsec_nanos())
        } else {
            panic!("SystemTime before UNIX EPOCH!");
        };
        let now_time_spec = Timespec {
            tv_sec: timestamp as i64,
            tv_nsec: timestamp_subsec_nanos as i64
        };

        stat.st_dev = ST_DEV;
        stat.st_ino = self.hash() as i64;
        stat.st_mode = self.st_mode().bits();
        stat.st_nlink = 1;
        stat.st_uid = self.uid();
        stat.st_gid = self.uid();
        stat.st_rdev = 0;
        stat.st_size = self.len();
        stat.st_blksize = 4096;
        stat.st_blocks = (self.len() as f64 / 512.0).ceil() as i64;
        stat.st_atime = now_time_spec.clone();
        stat.st_mtime = now_time_spec.clone();
        stat.st_ctime = now_time_spec;

        stat
    }

    fn oflags(&self) -> OFlag;

    fn mmap<'a>(&mut self, emu: &AndroidEmulator<'a, T>, addr: u64, aligned: i32, prot: u32, _offset: u32, _length: u64) -> anyhow::Result<u64> {
        let data = self.to_vec();
        emu.backend.mem_map(addr, aligned as usize, prot)
            .map_err(|e| anyhow!("FileIO failed to mmap: {:?}", e))?;
        let pointer = VMPointer::new(addr, 0, emu.backend.clone());
        pointer.write_bytes(Bytes::from(data))?;
        Ok(pointer.addr)
    }

    fn hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.path().hash(&mut hasher);
        hasher.finish()
    }

    fn st_mode(&self) -> StMode;

    fn uid(&self) -> i32;

    fn len(&self) -> usize;

    fn to_vec(&mut self) -> Vec<u8>;
}
