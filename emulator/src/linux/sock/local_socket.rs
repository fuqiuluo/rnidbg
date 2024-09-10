use crate::emulator::{AndroidEmulator, VMPointer};
use crate::linux::errno::Errno;
use crate::linux::file_system::{FileIO, FileIOTrait, SeekResult, StMode};
use crate::linux::structs::OFlag;
use crate::linux::structs::socket::Pf;

pub struct LocalSocket {

}

impl LocalSocket {
    pub fn new() -> Self {
        LocalSocket {

        }
    }
}

impl<T: Clone> FileIOTrait<T> for LocalSocket {
    fn connect(&mut self, addr: VMPointer<T>, addr_len: usize, emulator: &AndroidEmulator<T>) -> i32 {
        let sa_family = emulator.backend.mem_read_v2::<u16>(addr.addr).unwrap();
        if sa_family != (Pf::LOCAL as u32) as u16 {
            emulator.set_errno(Errno::EINVAL.as_i32()).unwrap();
            return Errno::EINVAL.as_i32();
        }

        let path = emulator.backend.mem_read_c_string(addr.addr + 2).unwrap();

        panic!("LocalSocket::connect: path={}", path);

        Errno::EACCES.as_i32()
    }

    fn close(&mut self) {}

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