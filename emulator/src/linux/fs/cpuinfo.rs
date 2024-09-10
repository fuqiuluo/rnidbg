use crate::emulator::VMPointer;
use crate::linux::structs::OFlag;
use crate::linux::file_system::{*};

pub struct Cpuinfo {
    pub path: String,
    pub oflags: u32,
    pub data: [u8; 4096],
    pub pos: usize
}

impl Cpuinfo {
    pub fn new(path: &str, oflags: u32) -> Self {
        let mut data = [0; 4096];
        if cfg!(target_os = "linux") {
            if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
                let cpuinfo = cpuinfo.as_bytes();
                let len = std::cmp::min(cpuinfo.len(), data.len());
                data[..len].copy_from_slice(&cpuinfo[..len]);
            } else {
                generate_cpuinfo(&mut data)
            }
        } else {
            generate_cpuinfo(&mut data)
        }
        Cpuinfo {
            path: path.to_string(),
            oflags,
            data,
            pos: 0
        }
    }
}

fn generate_cpuinfo(data: &mut [u8]) {
    use std::fmt::Write;

    const CPU_FLAGS: &str = "fp asimd evtstrm aes pmull sha1 sha2 crc32 atomics fphp asimdhp cpuid asimdrdm jscvt fcma lrcpc dcpop sha3 sm3 sm4 asimddp sha512 asimdfhm dit uscat ilrcpc flagm ssbs sb paca pacg dcpodp flagm2 frint i8mm bti";
    let mut buf = String::new();

    for processor in 0..8 {
        write!(buf, "processor\t: {}\n", processor).unwrap();
        write!(buf, "BogoMIPS\t: 38.40\n").unwrap();
        write!(buf, "Features\t: {}\n", CPU_FLAGS).unwrap();
        write!(buf, "CPU implementer\t: 0x41\n").unwrap();
        write!(buf, "CPU architecture: 8\n").unwrap();
        write!(buf, "CPU variant\t: 0x0\n").unwrap();
        write!(buf, "CPU part\t: 0xd46\n").unwrap();
        write!(buf, "CPU revision\t: 3\n\n").unwrap();
    }

    let cpuinfo = buf.as_bytes();
    let len = std::cmp::min(cpuinfo.len(), data.len());
    data[..len].copy_from_slice(&cpuinfo[..len]);
}

impl<T: Clone> FileIOTrait<T> for Cpuinfo {
    fn close(&mut self) {}

    fn read(&mut self, buf: VMPointer<T>, count: usize) -> usize {
        if self.pos >= 4096 {
            return 0;
        }
        let read = if count > 4096 {
            buf.write_data(&self.data).unwrap();
            4096
        } else {
            buf.write_data(&self.data[..count]).unwrap();
            count
        };
        self.pos += read;
        read
    }

    fn pread(&mut self, buf: VMPointer<T>, count: usize, offset: usize) -> usize {
        if offset >= 4096 {
            return 0;
        }
        let len = std::cmp::min(count, 4096 - offset);
        buf.write_data(&self.data[offset..offset + len]).unwrap();
        len
    }

    fn write(&mut self, buf: &[u8]) -> i32 {
        panic!("Cpuinfo is read-only");
    }

    fn lseek(&mut self, offset: i64, whence: i32) -> SeekResult {
        SeekResult::Ok(match whence {
            SEEK_SET => {
                self.pos = offset as usize;
                offset
            }
            SEEK_CUR => {
                let pos = self.pos as i64;
                self.pos = (pos + offset) as usize;
                pos + offset
            }
            SEEK_END => {
                let pos = 4096i64;
                self.pos = (pos + offset) as usize;
                pos + offset
            }
            _ => return SeekResult::WhenceError
        })
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
        panic!("Cpuinfo is read-only");
    }
}
