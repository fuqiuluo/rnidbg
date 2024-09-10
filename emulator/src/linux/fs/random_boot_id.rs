
use crate::emulator::VMPointer;
use crate::linux::file_system::{FileIOTrait, SeekResult, StMode, SEEK_CUR, SEEK_END, SEEK_SET};
use crate::linux::structs::OFlag;

pub struct RandomBootId {
    pub path: String,
    pub oflags: u32,
    pub data: String,
    pub pos: usize
}

impl RandomBootId {
    pub fn new(path: &str, oflags: u32) -> Self {
        let mut data = if cfg!(target_os = "linux") {
            if let Ok(id) = std::fs::read_to_string("/proc/sys/kernel/random/boot_id") {
                id
            } else {
                generate_random_boot_id()
            }
        } else {
            generate_random_boot_id()
        };
        RandomBootId {
            path: path.to_string(),
            oflags,
            data,
            pos: 0
        }
    }
}

fn generate_random_boot_id() -> String {
    use rand::Rng;

    let mut rng = rand::thread_rng();
    let mut data = String::new();
    for _ in 0..8 {
        data.push_str(&format!("{:x}", rng.gen_range(0..16)));
    }
    data.push_str("-");
    for _ in 0..4 {
        data.push_str(&format!("{:x}", rng.gen_range(0..16)));
    }
    data.push_str("-");
    for _ in 0..4 {
        data.push_str(&format!("{:x}", rng.gen_range(0..16)));
    }
    data.push_str("-");
    for _ in 0..4 {
        data.push_str(&format!("{:x}", rng.gen_range(0..16)));
    }
    data.push_str("-");
    for _ in 0..12 {
        data.push_str(&format!("{:x}", rng.gen_range(0..16)));
    }
    data.push_str("\n");
    data
}

impl<T: Clone> FileIOTrait<T> for RandomBootId {
    fn close(&mut self) {}

    fn read(&mut self, buf: VMPointer<T>, count: usize) -> usize {
        if self.pos >= self.data.len() {
            return 0;
        }
        let read = if count > self.data.len() {
            buf.write_data(self.data.as_bytes()).unwrap();
            self.data.len()
        } else {
            buf.write_data(&self.data.as_bytes()[..count]).unwrap();
            count
        };
        self.pos += read;
        read
    }

    fn pread(&mut self, buf: VMPointer<T>, count: usize, offset: usize) -> usize {
        if offset >= self.data.len() {
            return 0;
        }
        let len = std::cmp::min(count, self.data.len() - offset);
        buf.write_data(&self.data.as_bytes()[offset..offset + len]).unwrap();
        len
    }

    fn write(&mut self, buf: &[u8]) -> i32 {
        panic!("RandomBootId is read-only");
    }

    fn lseek(&mut self, offset: i64, whence: i32) -> SeekResult {
        if offset < 0 || offset as usize > self.data.len() {
            return SeekResult::OffsetError;
        }
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
                let pos = self.data.len();
                self.pos = pos + offset as usize;
                self.pos as i64
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
        panic!("RandomBootId is read-only");
    }
}
