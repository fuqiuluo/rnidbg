use std::ascii::AsciiExt;
use std::cell::OnceCell;
use std::ffi::c_long;
use std::fmt::{format, Write as IGNORE};
use std::io::Write;
use std::mem;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use bitflags::Flags;
use bytes::BytesMut;
use log::{error, info, warn};
use crate::backend::{Backend, Permission};
use crate::backend::RegisterARM64;
use crate::backend::RegisterARM64::{*};
use crate::emulator::{AndroidEmulator, AndroidEmulatorInner, HEAP_BASE};
use crate::emulator::signal::{SignalOps, UnixSigSet};
use crate::emulator::thread::{AbstractTask, MarshmallowThread, RunnableTask, Task, TaskStatus, ThreadDispatcher, Waiter, WaiterTrait};
use crate::linux::errno::Errno;
use crate::linux::file_system::{FileIO, FileIOTrait, SeekResult, StMode};
use crate::linux::fs::ByteArrayFileIO;
use crate::linux::fs::cpuinfo::Cpuinfo;
use crate::linux::fs::direction::Direction;
use crate::linux::fs::linux_file::LinuxFileIO;
use crate::linux::fs::maps::Maps;
use crate::linux::fs::meminfo::Meminfo;
use crate::linux::fs::random_boot_id::RandomBootId;
use crate::linux::fs::urandom::URandom;
use crate::linux::PAGE_ALIGN;
use crate::linux::pipe::Pipe;
use crate::linux::sock::local_socket::LocalSocket;
use crate::linux::structs::{OFlag, prctl, Timespec, Timeval, Timezone, CloneFlag};
use crate::linux::structs::prctl::PrctlOp;
use crate::linux::structs::socket::{Pf, SockType};
use crate::linux::thread::{FutexIndefinitelyWaiter, FutexNanoSleepWaiter};
use crate::pointer::VMPointer;

macro_rules! throw_err {
    ($backend:ident, $emulator:ident, $errno:expr) => {
        let errno = <Errno as Into<i32>>::into($errno);
        $backend.reg_write_i64(RegisterARM64::X0, -(errno as i64)).unwrap();
        $emulator.set_errno(errno).expect("failed to set errno");
        return;
    };
}
macro_rules! ret_u64 {
    ($backend:ident, $X0:expr) => {
        $backend.reg_write(RegisterARM64::X0, $X0).expect("failed to write x0");
    };
}

macro_rules! ret_i32 {
    ($backend:ident, $X0:expr) => {
        $backend.reg_write_i64(RegisterARM64::X0, ($X0 as i64)).expect("failed to write x0");
    };
}

macro_rules! ldr_i32 {
    ($backend:ident, $id:expr) => {
        $backend.reg_read($id).unwrap() as i32
    };
}

macro_rules! ldr_u32 {
    ($backend:ident, $id:expr) => {
        $backend.reg_read_u32($id).unwrap() as u32
    };
}

macro_rules! ldr_u64 {
    ($backend:ident, $id:expr) => {
        $backend.reg_read($id).unwrap()
    };
}

macro_rules! ldr_string {
    ($backend:ident, $id:expr) => {
        $backend.mem_read_c_string(ldr_u64!($backend, $id)).unwrap()
    };
}

pub fn syscall_brk<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    let addr = ldr_u64!(backend, X0);
    if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
        println!("syscall brk({})", addr);
    }

    if addr == 0 {
        emulator.inner_mut().brk = HEAP_BASE;
        ret_u64!(backend, HEAP_BASE);
        return;
    }

    if addr % 8 != 0 {
        throw_err!(backend, emulator, Errno::EINVAL.into());
    }

    let brk = emulator.inner_mut().brk;
    if addr > brk {
        backend.mem_map(brk, (addr - brk) as usize, (Permission::READ | Permission::WRITE).bits()).expect("failed to map memory: brk");
    } else if addr < brk {
        backend.mem_unmap(addr, (brk - addr) as usize).expect("failed to unmap memory: brk");
    }
    emulator.inner_mut().brk = addr;

    ret_u64!(backend, addr);
}

pub fn syscall_prctl<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    let from = emulator.find_caller_name();
    let op = ldr_u32!(backend, X0);
    let arg2 = ldr_u64!(backend, X1);
    let arg3 = ldr_u64!(backend, X2);
    let arg4 = ldr_u64!(backend, X3);

    match PrctlOp::from_bits(op).unwrap_or(PrctlOp::UNKNOWN) {
        PrctlOp::BIONIC_PR_SET_VMA => {
            if from == "libc.so" {
                ret_i32!(backend, 0);
            } else {
                panic!("prctl not supported: {:?} from {}", PrctlOp::BIONIC_PR_SET_VMA, from);
            }
        }
        PrctlOp::UNKNOWN => { panic!("prctl not supported: 0x{:X}", op) }
        _ => { panic!("prctl not supported: {:?}", PrctlOp::from_bits(op)) }
    };
}

pub fn syscall_gettimeofday<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    const TV_SIZE: usize = mem::size_of::<Timeval>();
    const TZ_SIZE: usize = mem::size_of::<Timezone>();

    let tv_pointer = emulator.backend.reg_read(X0).unwrap();
    if tv_pointer != 0 {
        let mut buffer = [0u8; TV_SIZE];
        let tv = unsafe {
            &mut *(buffer.as_mut_ptr() as *mut Timeval)
        };
        let now = chrono::Local::now();
        tv.tv_sec = now.timestamp();
        tv.tv_usec = now.timestamp_subsec_nanos() as i64;
        backend.mem_write(tv_pointer, &buffer).unwrap();
    }

    let tz_pointer = emulator.backend.reg_read(X1).unwrap();
    if tz_pointer != 0 {
        let mut buffer = [0u8; TZ_SIZE];
        let tz = unsafe {
            &mut *(buffer.as_mut_ptr() as *mut Timezone)
        };
        tz.tz_dsttime = 0;
        tz.tz_minuteswest = -480;
        backend.mem_write(tz_pointer, &buffer).unwrap();
    }

    if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
        println!("syscall gettimeofday(tv_pointer=0x{:x}, tz_pointer=0x{:x})", tv_pointer, tz_pointer);
    }

    ret_i32!(backend, 0);
}

pub fn syscall_futex<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    let uaddr = ldr_u64!(backend, X0);
    let op = ldr_i32!(backend, X1);
    let val = ldr_u32!(backend, X2);
    let timeout = ldr_u64!(backend, X3);
    let uaddr2 = ldr_u64!(backend, X4);
    let val3 = ldr_i32!(backend, X5);

    if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
        let lr = emulator.get_lr().unwrap();
        let from_module = emulator.find_caller();
        let from_module = if let Some(from_module_cell) = from_module {
            let module = unsafe { &*from_module_cell.get() };
            module.name.clone() + format!("@0x{:X}", lr - module.base).as_str()
        } else {
            format!("@0x{:X}", lr)
        };
        let mut old = [0u8; 4];
        if backend.mem_read(uaddr, &mut old).is_err() {
            info!("syscall futex(uaddr=0x{:x}, op={}, val={}, timeout=0x{:x}, uaddr2=0x{:x}, val3={}) from {}", uaddr, op, val, timeout, uaddr2, val3, from_module);
        } else {
            let old = u32::from_le_bytes(old);
            info!("syscall futex(uaddr=0x{:x}, op={}, val={}, old={}, timeout=0x{:x}, uaddr2=0x{:x}, val3={}) from {}", uaddr, op, val, old, timeout, uaddr2, val3, from_module);
        }
    }

    let is_private = (op & 0x80) != 0;
    let cmd = op & 0x7f;
    if 0 == cmd {
        let mut old = [0u8; 4];
        if backend.mem_read(uaddr, &mut old).is_err() {
            throw_err!(backend, emulator, Errno::EAGAIN);
        }
        let old = u32::from_le_bytes(old);
        if old != val {
            throw_err!(backend, emulator, Errno::EAGAIN);
        }

        let mtype = val & 0xc000;
        let shared = val & 0x2000;

        let time_spec = if timeout <= 0 {
            Timespec::default()
        } else {
            let mut buffer = [0u8; size_of::<Timespec>()];
            backend.mem_read(timeout, &mut buffer)
                .expect("Failed to read from memory: time_spec");
            let time_spec: &Timespec = unsafe {
                &*(buffer.as_ptr() as *const Timespec)
            };
            time_spec.clone()
        };

        let running_task = emulator.inner_mut().thread_dispatcher
            .running_task_mut();
        if let Some(running_task_cell) = running_task {
            let waiter = if timeout == 0 {
                Waiter::FutexIndefinite(FutexIndefinitelyWaiter::new(uaddr, val, &emulator.backend))
            } else {
                Waiter::FutexNanoSleep(FutexNanoSleepWaiter::new(uaddr, val, (time_spec.tv_sec * 1000i64 + time_spec.tv_nsec / 1000000i64) as u64, emulator.backend.clone()))
            };
            if option_env!("EMU_LOG") == Some("1") {
                info!("futex: set waiter: {:?}", match &waiter {
                    Waiter::FutexIndefinite(waiter) => waiter.can_dispatch().to_string() + "/",
                    Waiter::FutexNanoSleep(waiter) => waiter.can_dispatch().to_string() + "|",
                    Waiter::Unknown(_) => unreachable!()
                });
            }
            match unsafe { &mut *running_task_cell.get() } {
                AbstractTask::Function64(task) => {
                    task.set_waiter(emulator, waiter);
                }
                AbstractTask::SignalTask(task) => {
                    task.set_waiter(emulator, waiter);
                }
                AbstractTask::MarshmallowThread(task) => {
                    task.set_waiter(emulator, waiter);
                }
                _ => panic!("futex unexpected task type: running_task"),
            }
            emulator.emu_stop(TaskStatus::S).unwrap();
            return;
        } else {
            unreachable!()
        }

        if emulator.inner_mut().thread_dispatcher.task_counts() > 1 {
            emulator.emu_stop(TaskStatus::X).
                expect("failed to stop emulator");
            ret_i32!(backend, Errno::ETIMEDOUT.as_i32());
            return;
        } else {
            ret_i32!(backend, 0);
            return;
        }
    }
    else if 1 == cmd {
        let task_count = emulator.inner_mut().thread_dispatcher.task_counts();
        if task_count <= 1 {
            ret_i32!(backend, 0);
            return;
        } else {
            //println!("futex: task count = {}", task_count);
        }

        let mut count = 0;
        for task in emulator.inner_mut().thread_dispatcher.task_list_mut() {
            match unsafe { &mut *task.get() } {
                AbstractTask::Function64(task) => {
                    if option_env!("EMU_LOG") == Some("1") {
                        info!("futex unexpected task type: function64");
                    }
                    let waiter = task.get_waiter();
                    if waiter.is_none() { continue }
                    //println!("futex unexpected task type: function64");
                    let waiter = waiter.unwrap();
                    match waiter {
                        Waiter::FutexIndefinite(waiter) => waiter.wake_up(uaddr),
                        Waiter::FutexNanoSleep(waiter) => waiter.wake_up(uaddr),
                        _ => continue
                    };
                    count += 1;
                    if count >= val {
                        break;
                    }
                }
                AbstractTask::SignalTask(task) => {
                    if option_env!("EMU_LOG") == Some("1") {
                        info!("futex unexpected task type: signal_task");
                    }
                    let waiter = task.get_waiter();
                    if waiter.is_none() { continue }
                    let waiter = waiter.unwrap();
                    match waiter {
                        Waiter::FutexIndefinite(waiter) => waiter.wake_up(uaddr),
                        Waiter::FutexNanoSleep(waiter) => waiter.wake_up(uaddr),
                        _ => continue
                    };
                    count += 1;
                    if count >= val {
                        break;
                    }
                }
                AbstractTask::MarshmallowThread(task) => {
                    if option_env!("EMU_LOG") == Some("1") {
                        info!("futex unexpected task type: marshmallow_thread");
                    }
                    //println!("futex unexpected task type: marshmallow_thread");
                    let waiter = task.get_waiter();
                    if waiter.is_none() { continue }
                    let waiter = waiter.unwrap();
                    match waiter {
                        Waiter::FutexIndefinite(waiter) => waiter.wake_up(uaddr),
                        Waiter::FutexNanoSleep(waiter) => waiter.wake_up(uaddr),
                        _ => continue
                    };
                    count += 1;
                    if count >= val {
                        break;
                    }
                }
                AbstractTask::KitKatThread(_) => panic!("futex unexpected task type: kitkat_thread"),
            }
        }
        if count > 0 {
            if option_env!("EMU_LOG") == Some("1") {
                info!("futex: wake up {} tasks", count);
            }
            emulator.emu_stop(TaskStatus::S).unwrap();
            ret_i32!(backend, count as i32);
            return;
        }

        if emulator.inner_mut().context_task.is_some() {
            emulator.emu_stop(TaskStatus::S).unwrap();
            ret_i32!(backend, 1);
            return;
        }

        ret_i32!(backend, 0);
        return;
    }
    else if 4 == cmd {
        ret_i32!(backend, 0);
        return;
    }
}

pub fn syscall_clock_gettime<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    static START: OnceLock<Instant> = OnceLock::new();

    let clk_id = ldr_i32!(backend, X0);
    let tp_pointer = ldr_u64!(backend, X1);

    if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
        println!("syscall clock_gettime(clk_id={}, tp_pointer=0x{:x})", clk_id, tp_pointer);
    }

    match clk_id {
        0 => { // CLOCK_REALTIME
            if let Ok(duration_since_epoch) = SystemTime::now().duration_since(UNIX_EPOCH) {
                let mut buffer = [0u8; size_of::<Timeval>()];
                let tv = unsafe { &mut *(buffer.as_mut_ptr() as *mut Timeval) };
                tv.tv_sec = (duration_since_epoch.as_secs() + 8 * 3600) as i64;
                tv.tv_usec = duration_since_epoch.subsec_micros() as i64;
                backend.mem_write(tp_pointer, &buffer).expect("failed to write timeval");
                ret_i32!(backend, 0);
            } else {
                panic!("SystemTime before UNIX EPOCH!");
            }
        }
        1 => { // CLOCK_MONOTONIC
            let start = START.get_or_init(|| Instant::now()).clone();
            let mut buffer = [0u8; size_of::<Timespec>()];
            let duration = Instant::now().duration_since(start);
            let tv = unsafe { &mut *(buffer.as_mut_ptr() as *mut Timespec) };
            tv.tv_sec = duration.as_secs() as i64;
            tv.tv_nsec = duration.subsec_nanos() as i64;
            backend.mem_write(tp_pointer, &buffer).expect("failed to write timespec");
            ret_i32!(backend, 0);
        }
        3 => { // CLOCK_THREAD_CPUTIME_ID
            let start = START.get_or_init(|| Instant::now()).clone();
            let mut buffer = [0u8; size_of::<Timespec>()];
            let duration = Instant::now().duration_since(start);
            let tv = unsafe { &mut *(buffer.as_mut_ptr() as *mut Timespec) };
            tv.tv_sec = 0;
            tv.tv_nsec = duration.subsec_nanos() as i64;
            backend.mem_write(tp_pointer, &buffer).expect("failed to write timespec");
            ret_i32!(backend, 0);
        }
        _ => panic!("clock_gettime not supported: {}", clk_id)
    }
}

pub fn syscall_openat<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    let dir_fd = ldr_i32!(backend, X0);
    let flags = OFlag::from_bits_truncate(ldr_u32!(backend, X2));
    let mode = ldr_i32!(backend, X3);
    let path_pointer = ldr_u64!(backend, X1);
    let Ok(path) = backend.mem_read_c_string(path_pointer) else {
        if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
            error!("openat: failed to read path");
        }
        panic!("failed to read path");
    };

    if path == "/data/misc/zoneinfo/current/tzdata" {
        throw_err!(backend, emulator, Errno::ENOMEM);
    }

    if path == "/dev/pmsg0" {
        throw_err!(backend, emulator, Errno::EPERM);
    }

    if !path.starts_with("/") {
        if dir_fd != -100 {
            if option_env!("EMU_LOG") == Some("1") {
                error!("openat: dir_fd != AT_FDCWD");
            }

            panic!("dir_fd != AT_FDCWD");
        }
    }

    let from_module = emulator.find_caller_name();
    if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
        info!("syscall try openat(path={}, flags={:?}, mode={}) from {}", path, flags, mode, from_module);
    }
    let (fd, errno) = open(emulator, &path, flags, mode, &from_module);

    if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
        info!("syscall openat(path={}, flags={:?}, mode={}) -> {} from {}", path, flags, mode, fd, from_module);
    }

    ret_i32!(backend, fd);
    emulator.set_errno(errno).expect("failed to set errno");
    return;
}

pub fn syscall_mmap<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    let start = ldr_u64!(backend, X0);
    let length = ldr_i32!(backend, X1) as usize;
    let prot = ldr_i32!(backend, X2);
    let flags = ldr_i32!(backend, X3);
    let fd = ldr_i32!(backend, X4);
    let offset = ldr_i32!(backend, X5) << 12;

    match emulator.mmap2(start, length, prot as u32, flags as u32, fd, offset as i64) {
        Ok((errno, addr)) => {
            if errno != Errno::OK {
                throw_err!(backend, emulator, errno);
            }
            ret_u64!(backend, addr);
            return;
        }
        Err(err) => {
            error!("mmap failed: {:?}", err);
            throw_err!(backend, emulator, Errno::EAGAIN);
        }
    }
}

pub fn syscall_mprotect<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    let addr = ldr_u64!(backend, X0);
    let len = ldr_i32!(backend, X1) as usize;
    let prot = ldr_u32!(backend, X2);

    if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
        println!("syscall mprotect(addr=0x{:x}, len={}, prot={:?})", addr, len, prot);
    }

    let aligned_address = (addr / PAGE_ALIGN as u64) * PAGE_ALIGN as u64;
    let offset = addr - aligned_address;
    let size = len + offset as usize;
    let aligned_length = ((size - 1) / PAGE_ALIGN + 1) * PAGE_ALIGN;

/*    let mut mem_map_item = None;
    for (begin, map) in emulator.inner_mut().memory.memory_map.iter() {
        if *begin <= aligned_address && aligned_address < (map.base + map.size as u64) {
            mem_map_item = Some(map.clone());
            break;
        }
    }

    if let Some(mem_map_item) = mem_map_item {
        if mem_map_item.from_file {
            let block_prot = Permission::from_bits_truncate(mem_map_item.prot);
            if !block_prot.contains(Permission::WRITE) && prot.contains(Permission::WRITE) {
                throw_err!(backend, emulator, Errno::EACCES);
            }
            if !block_prot.contains(Permission::READ) && prot.contains(Permission::READ) {
                throw_err!(backend, emulator, Errno::EACCES);
            }
        }
    }*/

    if aligned_address % PAGE_ALIGN as u64 != 0 {
        throw_err!(backend, emulator, Errno::EINVAL);
    }

    if let Err(e) = backend.mem_protect(aligned_address, aligned_length, prot) {
        warn!("mprotect failed: {:?}", e);
        throw_err!(backend, emulator, Errno::EINVAL);
    }

    ret_i32!(backend, 0);
}

pub fn syscall_madvise<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    let addr = ldr_u64!(backend, X0);
    let len = ldr_i32!(backend, X1) as usize;
    let advice = ldr_i32!(backend, X2);
    if advice == 4 {
        if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
            println!("syscall madvice(addr=0x{:x}, len={}, advice={}) => success", addr, len, advice);
        }

        ret_i32!(backend, 0);
    }

    if addr <= 0 {
        if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
            println!("syscall madvice(addr=0x{:x}, len={}, advice={}) => addr is nullptr", addr, len, advice);
        }
        throw_err!(backend, emulator, Errno::EINVAL);
    }

    if addr % PAGE_ALIGN as u64 != 0 {
        if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
            println!("syscall madvice(addr=0x{:x}, len={}, advice={}) => addr not aligned", addr, len, advice);
        }
        throw_err!(backend, emulator, Errno::EINVAL);
    }
    if len % PAGE_ALIGN != 0 {
        if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
            println!("syscall madvice(addr=0x{:x}, len={}, advice={}) => len is not aligned", addr, len, advice);
        }
        throw_err!(backend, emulator, Errno::EINVAL);
    }

    if let Some(_) = emulator.inner_mut().memory.memory_map.get(&addr) {
        if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
            println!("syscall madvice(addr=0x{:x}, len={}, advice={}) => success", addr, len, advice);
        }
        ret_i32!(backend, 0);
    } else {
        if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
            println!("syscall madvice(addr=0x{:x}, len={}, advice={}) => locked memory", addr, len, advice);
        }
        throw_err!(backend, emulator, Errno::EINVAL);
    }
}

pub fn syscall_fstat<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    let fd = ldr_i32!(backend, X0);
    let stat_pointer = ldr_u64!(backend, X1);
    let file_system = &mut emulator.inner_mut().file_system;

    if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
        println!("syscall fstat(fd={}, stat_pointer=0x{:x})", fd, stat_pointer);
    }

    if let Some(file) = file_system.get_file_mut(fd) {
        match file {
            FileIO::Bytes(file) => {
                file.fstat(VMPointer::new(stat_pointer, 0, backend.clone()));
            }
            FileIO::File(file) => {
                file.fstat(VMPointer::new(stat_pointer, 0, backend.clone()));
            }
            FileIO::Dynamic(file) => {
                file.fstat(VMPointer::new(stat_pointer, 0, backend.clone()));
            }
            FileIO::Error(_) => panic!("fstat error, fd: {}, reason: file not found", fd),
            FileIO::Direction(dir) => {
                dir.fstat(VMPointer::new(stat_pointer, 0, backend.clone()));
            },
            FileIO::LocalSocket(_) => unreachable!()
        }
    } else {
        throw_err!(backend, emulator, Errno::EBADF);
    }
}

pub fn syscall_munmap<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    let start = ldr_u64!(backend, X0);
    let length = ldr_i32!(backend, X1) as usize;

    if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
        println!("syscall munmap(start=0x{:x}, length={})", start, length);
    }

    if let Err(e) = emulator.munmap(start, length as u64) {
        warn!("munmap failed: {:?}", e);
        throw_err!(backend, emulator, Errno::EINVAL);
    }

    ret_u64!(backend, 0);
}

pub fn syscall_close<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    let fd = ldr_i32!(backend, X0);

    if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
        println!("syscall close(fd={})", fd);
    }

    let file_system = &mut emulator.inner_mut().file_system;
    if let Some(file) = file_system.remove_file(fd) {
/*        let running_task = emulator.inner_mut().thread_dispatcher
            .running_task_mut();
        let is_main_task = if let Some(running_task_cell) = running_task {
            match unsafe { &mut *running_task_cell.get() } {
                AbstractTask::Function64(_) => true,
                AbstractTask::SignalTask(_) => false,
                AbstractTask::MarshmallowThread(_) => false,
                _ => panic!("close unexpected task type: running_task"),
            }
        } else {
            false
        };*/

        match file {
            FileIO::Bytes(_) => {}
            FileIO::File(mut file) => {
                file.close();
            }
            FileIO::Error(_) => panic!("close error, fd: {}, reason: file not found", fd),
            FileIO::Dynamic(mut file) => {
                file.close();
            }
            FileIO::Direction(_) => {}
            FileIO::LocalSocket(mut socket) => {
                <LocalSocket as FileIOTrait<T>>::close(&mut socket);
            }
        }
    } else {
        throw_err!(backend, emulator, Errno::EBADF);
    }
    ret_i32!(backend, 0);
}

pub fn syscall_read<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    let fd = ldr_i32!(backend, X0);
    let buf = ldr_u64!(backend, X1);
    let count = ldr_i32!(backend, X2) as usize;
    let from_module = emulator.find_caller_name();

    let file_system = &mut emulator.inner_mut().file_system;
    if let Some(file) = file_system.get_file_mut(fd) {
        let mode = match file {
            FileIO::Bytes(bytes) => bytes.st_mode(),
            FileIO::File(file) => file.st_mode(),
            FileIO::Error(_) => {
                throw_err!(backend, emulator, Errno::EBADF);
            }
            FileIO::Dynamic(file) => {
                file.st_mode()
            }
            FileIO::Direction(dir) => {
                <Direction as FileIOTrait<T>>::st_mode(dir)
            }
            FileIO::LocalSocket(_) => {
                StMode::S_IRUSR | StMode::S_IWUSR
            }
        };

        if !(mode.contains(StMode::S_IRUSR) || mode.contains(StMode::S_IROTH) || mode.contains(StMode::S_IRGRP)) && from_module != "libc.so" {
            throw_err!(backend, emulator, Errno::EACCES);
        }

        let read = match file {
            FileIO::Bytes(file) => file.read(VMPointer::new(buf, 0, backend.clone()), count),
            FileIO::File(file) => file.read(VMPointer::new(buf, 0, backend.clone()), count),
            FileIO::Error(_) => unreachable!(),
            FileIO::Dynamic(file) => file.read(VMPointer::new(buf, 0, backend.clone()), count),
            FileIO::Direction(_) => unreachable!(),
            FileIO::LocalSocket(socket) => socket.read(VMPointer::new(buf, 0, backend.clone()), count),
        };

        if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
            println!("syscall read(fd={}, buf=0x{:x}, count={}) => {} from {}", fd, buf, count, read, from_module);
        }

        ret_u64!(backend, read as u64);
    } else {
        if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
            println!("syscall read(fd={}, buf=0x{:x}, count={}) => EBADF from {}", fd, buf, count, from_module);
        }

        throw_err!(backend, emulator, Errno::EBADF);
    }
}

pub fn syscall_geteuid<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
        println!("syscall geteuid()");
    }

    ret_i32!(backend, 10261)
}

pub fn syscall_renameat<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    let old_dir_fd = ldr_i32!(backend, X0);
    let old_path_ptr = ldr_u64!(backend, X1);
    let new_dir_fd = ldr_i32!(backend, X2);
    let new_path_ptr = ldr_u64!(backend, X3);

    let old_path = backend.mem_read_c_string(old_path_ptr).unwrap();
    let new_path = backend.mem_read_c_string(new_path_ptr).unwrap();

    if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
        if old_path.is_ascii() && new_path.is_ascii() {
            println!("syscall renameat(old_dir_fd={}, old_path={}, new_dir_fd={}, new_path={})", old_dir_fd, old_path, new_dir_fd, new_path);
        } else {
            println!("syscall renameat(old_dir_fd={}, old_path=hex::decode({}), new_dir={}, new_path=hex::deecode({}))", old_dir_fd, hex::encode(old_path.as_bytes()), new_dir_fd, hex::encode(new_path.as_bytes()));
        }
    }

    if !new_path.is_empty() && new_path.as_bytes()[0] != b'/' {
        throw_err!(backend, emulator, Errno::EROFS);
    }

    unreachable!("renameat not supported");
}

pub fn syscall_fstatat<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    let fd = ldr_i32!(backend, X0);
    let path = ldr_string!(backend, X1);
    let stat_pointer = ldr_u64!(backend, X2);
    let flag = ldr_u32!(backend, X3);

    if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
        if path.is_ascii() {
            println!("syscall fstatat(fd={}, path={}, stat_pointer=0x{:x}, flag={})", fd, path, stat_pointer, flag);
        } else {
            println!("syscall fstatat(fd={}, path=hex::decode({}), stat_pointer=0x{:x}, flag={})", fd, hex::encode(path.as_bytes()), stat_pointer, flag);
        }
    }

    if path.is_empty() || path.as_bytes()[0] != b'/' {
        throw_err!(backend, emulator, Errno::ENOENT);
    }

    if fd != -100 {
        throw_err!(backend, emulator, Errno::EBADF);
    }

    let file_system = &mut emulator.inner_mut().file_system;
    if let Some(ref resolver) = file_system.file_resolver {
        if let Some(file) = resolver(file_system, path.as_str(), OFlag::from_bits_truncate(flag), 0) {
            match file {
                FileIO::Bytes(file) => file.fstat(VMPointer::new(stat_pointer, 0, backend.clone())),
                FileIO::Error(errno) => {
                    ret_i32!(backend, fd);
                    emulator.set_errno(errno).expect("failed to set errno");
                    return;
                },
                FileIO::File(file) => file.fstat(VMPointer::new(stat_pointer, 0, backend.clone())),
                FileIO::Dynamic(file) => file.fstat(VMPointer::new(stat_pointer, 0, backend.clone())),
                FileIO::Direction(dir) => {
                    dir.fstat(VMPointer::new(stat_pointer, 0, backend.clone()));
                }
                FileIO::LocalSocket(_) => unreachable!()
            }
        } else {
            throw_err!(backend, emulator, Errno::ENOENT);
        }
    } else {
        panic!("file_resolver not found");
    }
}

pub fn syscall_getppid<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
        println!("syscall getppid()");
    }

    ret_i32!(backend, emulator.inner_mut().ppid as i32);
}

pub fn syscall_getpid<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
        println!("syscall getpid()");
    }

    ret_i32!(backend, emulator.inner_mut().pid as i32);
}

pub fn syscall_getuid<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
        println!("syscall getuid()");
    }
    ret_i32!(backend, 10261);
}

pub fn syscall_clone<'a, T: Clone>(backend: &Backend<'a, T>, emulator: &AndroidEmulator<'a, T>) {
    // // pid_t __bionic_clone(int flags, void* child_stack, pid_t* parent_tid, void* tls, pid_t* child_tid, int (*fn)(void*), void* arg);
    let child_stack = ldr_u64!(backend, X1);
    let parent_tid = ldr_u32!(backend, X2);

    if child_stack == 0 && parent_tid == 0 {
        if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
            println!("syscall clone(child_stack=0, parent_tid=0)");
        }
        syscall_fork(backend, emulator);
        return;
    }

    let fnc = emulator.backend.reg_read(X5).unwrap() as i64;
    let arg = emulator.backend.reg_read(X6).unwrap() as i64;
    if child_stack != 0 && backend.mem_read_i64(child_stack).unwrap() == fnc && backend.mem_read_i64(child_stack + 8).unwrap() == arg {
        if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
            println!("syscall clone(child_stack=0x{:x}, parent_tid=0x{:x}, fn=0x{:x}, arg=0x{:x}) => bionic_clone", child_stack, parent_tid, fnc, arg);
        }
        syscall_bionic_clone(backend, emulator);
        return;
    } else {
        panic!("pthread_clone")
    }
}

pub fn syscall_sigaltstack<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    let ss = ldr_u64!(backend, X0);
    let old_ss = ldr_u64!(backend, X1);

    if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
        println!("syscall sigaltstack(ss=0x{:x}, old_ss=0x{:x})", ss, old_ss);
    }

    if old_ss != 0 {
        panic!("sigaltstack not supported: old_ss");
    }

    ret_i32!(backend, 0);
}

pub fn syscall_lseek<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    let fd = ldr_i32!(backend, X0);
    let offset = ldr_u64!(backend, X1) as i64;
    let whence = ldr_i32!(backend, X2);


    let file_system = &mut emulator.inner_mut().file_system;
    if let Some(file) = file_system.get_file_mut(fd) {
        let result = match file {
            FileIO::Bytes(file) => file.lseek(offset, whence),
            FileIO::File(file) => file.lseek(offset, whence),
            FileIO::Error(_) => unreachable!(),
            FileIO::Dynamic(file) => file.lseek(offset, whence),
            FileIO::Direction(_) => unreachable!(),
            FileIO::LocalSocket(_) => panic!("lseek not supported: local socket"),
        };
        if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
            println!("syscall lseek(fd={}, offset={}, whence={}) => {:?}", fd, offset, whence, result);
        }
        match result {
            SeekResult::Ok(offset) => {
                ret_u64!(backend, offset as u64);
            }
            SeekResult::WhenceError => {
                throw_err!(backend, emulator, Errno::EINVAL);
            }
            SeekResult::OffsetError => {
                throw_err!(backend, emulator, Errno::ENXIO);
            }
            SeekResult::UnknownError => {
                throw_err!(backend, emulator, Errno::ESPIPE);
            }
        }
    } else {
        if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
            println!("syscall lseek(fd={}, offset={}, whence={}) => EBADF", fd, offset, whence);
        }
        throw_err!(backend, emulator, Errno::EBADF);
    }
}

pub fn syscall_mkdirat<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    let dir_fd = ldr_i32!(backend, X0);
    let path = ldr_string!(backend, X1);
    let mode = ldr_u32!(backend, X2);

    if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
        println!("syscall mkdirat(dir_fd={}, path={}, mode={:X})", dir_fd, path, mode);
    }

    if path.is_empty() || path.as_bytes()[0] != b'/' {
        throw_err!(backend, emulator, Errno::ENOENT);
    }

    if dir_fd != -100 {
        throw_err!(backend, emulator, Errno::EBADF);
    }

    if path == "/sdcard/Android/" || path == "/sdcard/Android" {
        throw_err!(backend, emulator, Errno::EEXIST);
    }

    unreachable!()
}

pub fn syscall_set_tid_address<'a, T: Clone>(backend: &Backend<'a, T>, emulator: &AndroidEmulator<'a, T>) {
    let tidptr = ldr_u64!(backend, X0);

    if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
        let from = emulator.find_caller_name();
        println!("syscall set_tid_address(tidptr=0x{:x}) from {}", tidptr, from);
    }

    let task = emulator.inner_mut().context_task.as_ref().unwrap();
    match unsafe { &mut *task.get() } {
        AbstractTask::MarshmallowThread(task) => {
            task.set_tid_ptr(VMPointer::new(tidptr, 0, backend.clone()));
        }
        _ => panic!("set_tid_address not supported: task type")
    }

    ret_i32!(backend, 0);
}

pub fn syscall_rt_sigprocmask<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    let how = ldr_i32!(backend, X0);
    let set = ldr_u64!(backend, X1);
    let oldset = ldr_u64!(backend, X2);

    if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
        let from = emulator.find_caller_name();
        println!("syscall rt_sigprocmask(how={}, set=0x{:x}, oldset=0x{:x}) from {}", how, set, oldset, from);
    }

    let task = emulator.inner_mut().context_task.as_ref().unwrap();
    match unsafe { &mut *task.get() } {
        AbstractTask::MarshmallowThread(task) => {
            let ops = task.signal_ops_mut();
            let old = ops.get_sig_mask_set();
            if oldset != 0 {
                if let Some(ref old) = old {
                    let mask = old.get_mask();
                    backend.mem_write(oldset, &mask.to_le_bytes()).unwrap();
                }
            }

            if set == 0 {
                ret_i32!(backend, 0);
                return;
            }

            let mask = backend.mem_read_u64(set).unwrap();
            match how {
                0 => {
                    if old.is_none() {
                        let set = UnixSigSet::new(mask);
                        let pending_set = UnixSigSet::new(0);
                        ops.set_sig_mask_set(Box::new(set));
                        ops.set_sig_pending_set(Box::new(pending_set));
                    } else {
                        old.unwrap().block_sig_set(mask);
                    }
                    ret_i32!(backend, 0);
                    return;
                }
                1 => {
                    if old.is_some() {
                        old.unwrap().unblock_sig_set(mask);
                    }
                    ret_i32!(backend, 0);
                    return;
                }
                2 => {
                    let set = UnixSigSet::new(mask);
                    let pending_set = UnixSigSet::new(0);
                    ops.set_sig_mask_set(Box::new(set));
                    ops.set_sig_pending_set(Box::new(pending_set));
                    ret_i32!(backend, 0);
                    return;
                }
                _ => panic!("rt_sigprocmask not supported: {}", how)
            }
        }
        _ => panic!("set_tid_address not supported: task type")
    }

    throw_err!(backend, emulator, Errno::EINVAL);
}

pub fn syscall_exit<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    let status = ldr_i32!(backend, X0);

    let task = emulator.inner_mut().context_task.as_ref().unwrap();
    match unsafe { &mut *task.get() } {
        AbstractTask::MarshmallowThread(task) => {
            if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
                println!("syscall exit(status={}) by ThreadTask", status);
            }
            task.set_exit_status(status)
        }
        _ => panic!("set_tid_address not supported: task type")
    }

    emulator.emu_stop(TaskStatus::X).unwrap();
}

pub fn syscall_bionic_clone<'a, T: Clone>(backend: &Backend<'a, T>, emulator: &AndroidEmulator<'a, T>) {
    let flag = ldr_u32!(backend, X0);
    let child_stack = ldr_u64!(backend, X1);
    let parent_tid = ldr_u64!(backend, X2);
    let tls = ldr_u64!(backend, X3);
    let child_tid = ldr_u64!(backend, X4);
    let fn_ptr = ldr_u64!(backend, X5);
    let arg = ldr_u64!(backend, X6);

    let flag = CloneFlag::from_bits(flag).unwrap();

    if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
        println!("syscall bionic_clone(flag={:?}, child_stack=0x{:x}, parent_tid=0x{:x}, tls=0x{:x}, child_tid=0x{:x}, fn=0x{:x}, arg=0x{:x})", flag, child_stack, parent_tid, tls, child_tid, fn_ptr, arg);
    }

    if !flag.contains(CloneFlag::CLONE_VM) {
        // 不支持使用clone创建新进程
        panic!("bionic_clone not supported: CLONE_VM, unable to copy address space");
    }

    if !flag.contains(CloneFlag::CLONE_FS) {
        panic!("bionic_clone not supported: CLONE_FS, unable to share file system");
    }

    if !flag.contains(CloneFlag::CLONE_FILES) {
        panic!("bionic_clone not supported: CLONE_FILES, unable to share file descriptors");
    }

    if !flag.contains(CloneFlag::CLONE_SIGHAND) {
        panic!("bionic_clone not supported: CLONE_SIGHAND, unable to share signal handlers");
    } else {
        if !flag.contains(CloneFlag::CLONE_VM) {
            throw_err!(backend, emulator, Errno::EINVAL);
        }
    }

    if !flag.contains(CloneFlag::CLONE_THREAD) {
        panic!("bionic_clone not supported: CLONE_THREAD, unable to share thread group");
    } else {
        if !flag.contains(CloneFlag::CLONE_SIGHAND) {
            throw_err!(backend, emulator, Errno::EINVAL);
        }
    }

    let thread_id = emulator.inner_mut().task_id_factory
        .fetch_add(1, Ordering::SeqCst);

    if flag.contains(CloneFlag::CLONE_PARENT_SETTID) {
        if parent_tid == 0 {
            throw_err!(backend, emulator, Errno::EINVAL);
        }
        backend.mem_write(parent_tid, &thread_id.to_le_bytes()).unwrap();
    }

    //println!("bbbbbbbbbbbb");
    let thread = AbstractTask::MarshmallowThread(MarshmallowThread::new(
        emulator.clone(),
        thread_id,
        VMPointer::new(fn_ptr, 0, backend.clone()),
        VMPointer::new(arg, 0, backend.clone()),
        Some(VMPointer::new(child_tid, 0, backend.clone())),
    ));
    //println!("ddddddddddd");
    emulator.inner_mut().thread_dispatcher.add_thread(thread);
    //println!("cccccc");

    if child_tid != 0 {
        backend.mem_write(child_tid, &thread_id.to_le_bytes()).unwrap();
    }
    ret_i32!(backend, thread_id as i32);
}

#[inline]
pub fn syscall_fork<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    panic!()
}

pub fn syscall_faccessat<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    let dir_fd = ldr_i32!(backend, X0);
    let path = ldr_string!(backend, X1);
    let mode = ldr_i32!(backend, X2);
    let flag = ldr_i32!(backend, X3);

    if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
        println!("syscall faccessat(dir_fd={}, path={}, mode={}, flag={})", dir_fd, path, mode, flag);
    }

    if !path.is_ascii() {
        panic!("faccessat not supported: path is not ascii");
    }

    if dir_fd == -100 {
        if path == "/proc/rk_dmabuf" {
            throw_err!(backend, emulator, Errno::EINVAL);
        }
        if path == "/proc/device-tree/rockchip-suspend" {
            throw_err!(backend, emulator, Errno::EINVAL);
        }
        if path == "/proc/device-tree/rockchip-system-monitor" {
            throw_err!(backend, emulator, Errno::EINVAL);
        }
        if path == "/proc/mpp_service/rkvenc-core0" {
            throw_err!(backend, emulator, Errno::EINVAL);
        }
        if path.starts_with("/vendor") {
            throw_err!(backend, emulator, Errno::EINVAL);
        }
        if path == "/sys/bus/platform/drivers/hisi-lpc" {
            throw_err!(backend, emulator, Errno::EINVAL);
        }
        if path == "/hmdocker" {
            throw_err!(backend, emulator, Errno::EINVAL);
        }

        if path == "/data/local/su"
            || path == "/data/local/bin/su"
            || path == "/data/local/xbin/su"
            || path == "/sbin/su"
            || path == "/su/bin/su"
            || path == "/system/bin/su"
            || path == "/system/bin/.ext/su"
            || path == "/system/bin/failsafe/su"
            || path == "/system/sd/xbin/su"
            || path == "/system/usr/we-need-root/su"
            || path == "/system/xbin/su"
            || path == "/cache/su"
            || path == "/data/su"
            || path == "/dev/su" {
            throw_err!(backend, emulator, Errno::EINVAL);
        }

        if path == "/data/data/com.tencent.mobileqq" {
            ret_i32!(backend, 0);
            return;
        }
    }

    panic!()
}

pub fn syscall_getdents64<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    let fd = ldr_i32!(backend, X0);
    let dirp = ldr_u64!(backend, X1);
    let size = ldr_i32!(backend, X2) as usize;

    if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
        println!("syscall getdents64(fd={}, dirp=0x{:x}, size={})", fd, dirp, size);
    }

    let file_system = &mut emulator.inner_mut().file_system;
    if let Some(file) = file_system.get_file_mut(fd) {
        let ret = match file {
            FileIO::Bytes(_) => unreachable!(),
            FileIO::File(_) => unreachable!(),
            FileIO::Error(_) => unreachable!(),
            FileIO::Dynamic(_) => unreachable!(),
            FileIO::Direction(dir) => dir.getdents64(VMPointer::new(dirp, 0, backend.clone()), size),
            FileIO::LocalSocket(_) => unreachable!()
        };

        ret_u64!(backend, ret as u64);
    } else {
        throw_err!(backend, emulator, Errno::EBADF);
    }
}

pub fn syscall_write<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    let fd = ldr_i32!(backend, X0);
    let buf = ldr_u64!(backend, X1);
    let count = ldr_i32!(backend, X2) as usize;

    let from_module = emulator.find_caller_name();
    let file_system = &mut emulator.inner_mut().file_system;
    if let Some(file) = file_system.get_file_mut(fd) {
        let mode = match file {
            FileIO::Bytes(bytes) => bytes.st_mode(),
            FileIO::File(file) => file.st_mode(),
            FileIO::Error(_) => {
                throw_err!(backend, emulator, Errno::EBADF);
            }
            FileIO::Dynamic(file) => file.st_mode(),
            FileIO::Direction(_) => unreachable!(),
            FileIO::LocalSocket(_) => {
                StMode::S_IRUSR | StMode::S_IWUSR
            }
        };

        if !(mode.contains(StMode::S_IWUSR) || mode.contains(StMode::S_IWOTH) || mode.contains(StMode::S_IWGRP)) && from_module != "libc.so" {
            if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
                println!("syscall write(fd={}, buf=0x{:x}, count={}) => EACCES from {}", fd, buf, count, from_module);
            }
            throw_err!(backend, emulator, Errno::EACCES);
        }

        let data = backend.mem_read_as_vec(buf, count).unwrap();

        let written = match file {
            FileIO::Bytes(file) => file.write(data.as_slice()),
            FileIO::File(file) => file.write(data.as_slice()),
            FileIO::Error(_) => unreachable!(),
            FileIO::Dynamic(file) => file.write(data.as_slice()),
            FileIO::Direction(_) => unreachable!(),
            FileIO::LocalSocket(socket) => {
                <LocalSocket as FileIOTrait<T>>::write(socket, data.as_slice())
            },
        };

        if written == -1 {
            throw_err!(backend, emulator, Errno::EACCES);
        }

        if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
            println!("syscall write(fd={}, buf=0x{:x}, count={}) => {} from {}", fd, buf, count, written, from_module);
        }

        ret_u64!(backend, written as u64);
    } else {
        if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
            println!("syscall write(fd={}, buf=0x{:x}, count={}) => EBADF from {}", fd, buf, count, from_module);
        }

        throw_err!(backend, emulator, Errno::EBADF);
    }
}

pub fn syscall_socket<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    let domain = Pf::from_u32(ldr_u32!(backend, X0));
    let typ = SockType::from_bits_truncate(ldr_u32!(backend, X1));
    let protocol = ldr_i32!(backend, X2);

    if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
        println!("syscall socket(domain={:?}, type={:?}, protocol={})", domain, typ, protocol);
    }

    let file_system = &mut emulator.inner_mut().file_system;
    if domain == Pf::LOCAL {
        let fd = file_system.insert_file(FileIO::LocalSocket(LocalSocket::new()));
        ret_i32!(backend, fd);
        return;
    }

    panic!()
}

pub fn syscall_connect<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    let sock_fd = ldr_i32!(backend, X0);
    let addr = ldr_u64!(backend, X1);
    let addr_len = ldr_i32!(backend, X2) as usize;
    let from = emulator.find_caller_name();

    if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
        println!("syscall connect(sock_fd={}, addr=0x{:x}, addr_len={}) from {}", sock_fd, addr, addr_len, from);
    }

    if from == "libc.so" {
        throw_err!(backend, emulator, Errno::EACCES);
    }

    let file_system = &mut emulator.inner_mut().file_system;
    if let Some(file) = file_system.get_file_mut(sock_fd) {
        match file {
            FileIO::LocalSocket(socket) => {
                let ret = socket.connect(VMPointer::new(addr, 0, backend.clone()), addr_len, emulator);
                ret_i32!(backend, ret);
                return;
            }
            _ => {
                throw_err!(backend, emulator, Errno::ENOTSOCK);
            }
        }
    } else {
        throw_err!(backend, emulator, Errno::EBADF);
    }

    unreachable!()
}

pub fn syscall_pipe2<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    let pipefd = ldr_u64!(backend, X0);
    let flags = ldr_i32!(backend, X1);

    if option_env!("PRINT_SYSCALL_LOG") == Some("1") {
        println!("syscall pipe2(pipefd=0x{:x}, flags={})", pipefd, flags);
    }

    throw_err!(backend, emulator, Errno::ENFILE);
    /*
    let file_system = &mut emulator.inner_mut().file_system;
    let read_pipe = Pipe::new();
    let write_pipe = read_pipe.clone();*/
}

pub fn syscall_nr3264_fcntl<T: Clone>(backend: &Backend<T>, emulator: &AndroidEmulator<T>) {
    panic!();
}

#[inline]
fn open<T: Clone>(emulator: &AndroidEmulator<T>, path: &str, flags: OFlag, mode: i32, from_module: &str) -> (i32, i32) {
    if path == "/dev/__properties__" {
        let errno: i32 = Errno::EPERM.into();
        return (-errno, errno);
    }

    let file_system = &mut emulator.inner_mut().file_system;
    if path == "/dev/urandom" {
        let fd = file_system.insert_file(FileIO::Dynamic(Box::new(
            URandom::new(path, flags.bits(), 0, StMode::SYSTEM_FILE)
        )));
        return (fd, 0)
    } else if path == "/proc/meminfo" {
        let fd = file_system.insert_file(FileIO::Dynamic(Box::new(
            Meminfo::new(path, flags.bits())
        )));
        return (fd, 0)
    } else if path == "/proc/cpuinfo" {
        let fd = file_system.insert_file(FileIO::Dynamic(Box::new(
            Cpuinfo::new(path, flags.bits())
        )));
        return (fd, 0)
    } else if path == "/proc/sys/kernel/random/boot_id" {
        let fd = file_system.insert_file(FileIO::Dynamic(Box::new(
            RandomBootId::new(path, flags.bits())
        )));
        return (fd, 0)
    }

    if path == "/proc/stat" {
        return if from_module == "libc.so" {
            // 8个cpu?
            let mut buf = BytesMut::new();
            buf.write_str("cpu 9160 11352 15848 9160 11352 1584 80 0 0 0").unwrap();
            for i in 0..8 {
                buf.write_str(format!("cpu{} 1145 1419 1981 1145 1419 198 10 0 0 0", i).as_str()).unwrap();
            }
            let bytes_file = ByteArrayFileIO::new(buf.freeze().to_vec(), path.to_string(), 0, flags.bits(), StMode::SYSTEM_FILE);
            let fd = file_system.insert_file(FileIO::Bytes(bytes_file));
            (fd, 0)
        } else {
            let errno: i32 = Errno::EPERM.into();
            (-errno, errno)
        }
    }

    if let Some(ref resolver) = file_system.file_resolver {
        if let Some(file) = resolver(file_system, path, flags, mode) {
            return match file {
                FileIO::Bytes(file) => {
                    let fd = file_system.insert_file(FileIO::Bytes(file));
                    (fd, 0)
                }
                FileIO::Error(errno) => (-errno, errno),
                FileIO::File(file) => {
                    let fd = file_system.insert_file(FileIO::File(file));
                    (fd, 0)
                }
                FileIO::Dynamic(file) => {
                    let fd = file_system.insert_file(FileIO::Dynamic(file));
                    (fd, 0)
                }
                FileIO::Direction(dir) => {
                    let fd = file_system.insert_file(FileIO::Direction(dir));
                    (fd, 0)
                }
                FileIO::LocalSocket(_) => unreachable!()
            }
        }
    }

    if path == "/proc/self/maps" {
        let fd = file_system.insert_file(FileIO::Dynamic(Box::new(
            Maps::new(path, flags.bits())
        )));
        return (fd, 0)
    }

    let errno: i32 = Errno::ENOENT.into();
    (-errno, errno)
}
