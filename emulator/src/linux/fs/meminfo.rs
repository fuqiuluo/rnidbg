use crate::emulator::VMPointer;
use crate::linux::file_system::{FileIOTrait, SeekResult, StMode, SEEK_CUR, SEEK_END, SEEK_SET};
use crate::linux::structs::OFlag;

pub struct Meminfo {
    pub path: String,
    pub oflags: u32,
    pub data: [u8; 4096],
    pub pos: usize
}

impl Meminfo {
    pub fn new(path: &str, oflags: u32) -> Self {
        let mut data = [0; 4096];
        if cfg!(target_os = "linux") {
            if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
                let meminfo = meminfo.as_bytes();
                let len = std::cmp::min(meminfo.len(), data.len());
                data[..len].copy_from_slice(&meminfo[..len]);
            } else {
                generate_meminfo(&mut data)
            }
        } else {
            generate_meminfo(&mut data)
        }
        Meminfo {
            path: path.to_string(),
            oflags,
            data,
            pos: 0
        }
    }
}

fn generate_meminfo(data: &mut [u8]) {
    use rand::Rng;
    use std::fmt::Write;

    let mut buf = String::new();
    let mut rng = rand::thread_rng();

    let mem_total = 11470020;
    let mem_free = rng.gen_range(200000..500000);
    let mem_available = rng.gen_range(4000000..5000000);
    let buffers = rng.gen_range(1000..5000);
    let cached = rng.gen_range(3000000..4000000);
    let swap_cached = rng.gen_range(100000..200000);
    let active = rng.gen_range(3000000..4000000);
    let inactive = rng.gen_range(2000000..3000000);
    let active_anon = rng.gen_range(1000000..2000000);
    let inactive_anon = rng.gen_range(1000000..2000000);
    let active_file = rng.gen_range(1000000..2000000);
    let inactive_file = rng.gen_range(1000000..2000000);
    let unevictable = rng.gen_range(200000..300000);
    let mlocked = rng.gen_range(200000..300000);
    let swap_total = 8388604;
    let swap_free = rng.gen_range(4000000..5000000);
    let dirty = rng.gen_range(1000..5000);
    let writeback = rng.gen_range(0..1000);
    let anon_pages = rng.gen_range(2000000..3000000);
    let mapped = rng.gen_range(1000000..2000000);
    let shmem = rng.gen_range(50000..100000);
    let k_reclaimable = rng.gen_range(800000..900000);
    let slab = rng.gen_range(900000..1000000);
    let s_reclaimable = rng.gen_range(200000..300000);
    let s_unreclaim = rng.gen_range(600000..700000);
    let kernel_stack = rng.gen_range(100000..200000);
    let page_tables = rng.gen_range(200000..300000);
    let nfs_unstable = 0;
    let bounce = 0;
    let writeback_tmp = 0;
    let commit_limit = 14123612;
    let committed_as = rng.gen_range(190000000..200000000);
    let vmalloc_total = 263061440;
    let vmalloc_used = rng.gen_range(300000..400000);
    let vmalloc_chunk = 0;
    let percpu = rng.gen_range(20000..30000);
    let anon_huge_pages = 0;
    let shmem_huge_pages = 0;
    let shmem_pmd_mapped = 0;
    let file_huge_pages = rng.gen_range(5000..10000);
    let file_pmd_mapped = rng.gen_range(5000..10000);
    let cma_total = 495616;
    let cma_free = rng.gen_range(0..500000);

    write!(buf, "MemTotal:       {} kB\n", mem_total).unwrap();
    write!(buf, "MemFree:        {} kB\n", mem_free).unwrap();
    write!(buf, "MemAvailable:   {} kB\n", mem_available).unwrap();
    write!(buf, "Buffers:        {} kB\n", buffers).unwrap();
    write!(buf, "Cached:         {} kB\n", cached).unwrap();
    write!(buf, "SwapCached:     {} kB\n", swap_cached).unwrap();
    write!(buf, "Active:         {} kB\n", active).unwrap();
    write!(buf, "Inactive:       {} kB\n", inactive).unwrap();
    write!(buf, "Active(anon):   {} kB\n", active_anon).unwrap();
    write!(buf, "Inactive(anon): {} kB\n", inactive_anon).unwrap();
    write!(buf, "Active(file):   {} kB\n", active_file).unwrap();
    write!(buf, "Inactive(file): {} kB\n", inactive_file).unwrap();
    write!(buf, "Unevictable:    {} kB\n", unevictable).unwrap();
    write!(buf, "Mlocked:        {} kB\n", mlocked).unwrap();
    write!(buf, "SwapTotal:      {} kB\n", swap_total).unwrap();
    write!(buf, "SwapFree:       {} kB\n", swap_free).unwrap();
    write!(buf, "Dirty:          {} kB\n", dirty).unwrap();
    write!(buf, "Writeback:      {} kB\n", writeback).unwrap();
    write!(buf, "AnonPages:      {} kB\n", anon_pages).unwrap();
    write!(buf, "Mapped:         {} kB\n", mapped).unwrap();
    write!(buf, "Shmem:          {} kB\n", shmem).unwrap();
    write!(buf, "KReclaimable:   {} kB\n", k_reclaimable).unwrap();
    write!(buf, "Slab:           {} kB\n", slab).unwrap();
    write!(buf, "SReclaimable:   {} kB\n", s_reclaimable).unwrap();
    write!(buf, "SUnreclaim:     {} kB\n", s_unreclaim).unwrap();
    write!(buf, "KernelStack:    {} kB\n", kernel_stack).unwrap();
    write!(buf, "PageTables:     {} kB\n", page_tables).unwrap();
    write!(buf, "NFS_Unstable:   {} kB\n", nfs_unstable).unwrap();
    write!(buf, "Bounce:         {} kB\n", bounce).unwrap();
    write!(buf, "WritebackTmp:   {} kB\n", writeback_tmp).unwrap();
    write!(buf, "CommitLimit:    {} kB\n", commit_limit).unwrap();
    write!(buf, "Committed_AS:   {} kB\n", committed_as).unwrap();
    write!(buf, "VmallocTotal:   {} kB\n", vmalloc_total).unwrap();
    write!(buf, "VmallocUsed:    {} kB\n", vmalloc_used).unwrap();
    write!(buf, "VmallocChunk:   {} kB\n", vmalloc_chunk).unwrap();
    write!(buf, "Percpu:         {} kB\n", percpu).unwrap();
    write!(buf, "AnonHugePages:  {} kB\n", anon_huge_pages).unwrap();
    write!(buf, "ShmemHugePages: {} kB\n", shmem_huge_pages).unwrap();
    write!(buf, "ShmemPmdMapped: {} kB\n", shmem_pmd_mapped).unwrap();
    write!(buf, "FileHugePages:  {} kB\n", file_huge_pages).unwrap();
    write!(buf, "FilePmdMapped:  {} kB\n", file_pmd_mapped).unwrap();
    write!(buf, "CmaTotal:       {} kB\n", cma_total).unwrap();
    write!(buf, "CmaFree:        {} kB\n", cma_free).unwrap();
    let len = std::cmp::min(buf.len(), data.len());
    data[..len].copy_from_slice(&buf.as_bytes()[..len]);
}

impl<T: Clone> FileIOTrait<T> for Meminfo {
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
        panic!("Meminfo is read-only");
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
        panic!("Meminfo is read-only");
    }
}


