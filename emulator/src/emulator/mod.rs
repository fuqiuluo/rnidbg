mod env_fixer;
mod mem_hook;
mod consts;

pub mod syscall_handler;
pub mod memory;
pub mod func;
pub mod trace;
pub(crate) mod thread;
pub(crate) mod signal;

pub use syscall_handler::POST_CALLBACK_SYSCALL_NUMBER;
pub use syscall_handler::PRE_CALLBACK_SYSCALL_NUMBER;
pub use consts::{*};
use crate::emulator::thread::{AbstractTask, Function64, RunnableTask, Task, TaskStatus, ThreadDispatcher, UniThreadDispatcher};
use crate::linux::{LinuxModule, PAGE_ALIGN};
use crate::linux::symbol::ModuleSymbol;
use crate::linux::file_system::AndroidFileSystem;
pub use crate::pointer::VMPointer;
use crate::tool::UnicornArg;
use crate::android::dvm::DalvikVM64;
use crate::android::virtual_library::ld64::ArmLD64;
use crate::android::virtual_library::libc::Libc;
use crate::backend::{Backend, Permission, RegisterARM64};
use crate::emulator::consts::{LR};
use crate::memory::AndroidElfLoader;
use crate::memory::svc_memory::SvcMemory;
use crate::backend::Context;
use std::cell::{RefCell, UnsafeCell};
use std::collections::HashMap;
use std::ffi::c_void;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use anyhow::anyhow;
use bytes::{BufMut, Bytes, BytesMut};
use log::{error, info, warn};
use crate::emulator::signal::ISignalTask;

pub type RcUnsafeCell<T> = Rc<UnsafeCell<T>>;
pub type ArcUnsafeCell<T> = Arc<UnsafeCell<T>>;

#[derive(Clone)]
pub struct AndroidEmulator<'a, T: Clone> {
    inner: Arc<UnsafeCell<AndroidEmulatorInner<'a, T>>>,
    pub backend: Backend<'a, T>,
}

unsafe impl<'a, T: Send + Clone> Send for AndroidEmulator<'a, T> {}
unsafe impl<'a, T: Send + Clone> Sync for AndroidEmulator<'a, T> {}

pub(crate) struct AndroidEmulatorInner<'a, T: Clone> {
    pub pid: u32,
    pub ppid: u32,
    pub proc_name: String,
    pub errno: u64,
    pub brk: u64,

    pub running: AtomicBool,
    pub base_path: String,
    pub file_system: AndroidFileSystem<T>,
    pub memory: AndroidElfLoader<'a, T>,
    pub svc_memory: SvcMemory<'a, T>,
    pub context_stack: Vec<(Context, i32)>,

    // DalvikVM
    pub dalvik: Option<DalvikVM64<'a, T>>,

    // thread dispatcher
    pub thread_dispatcher: UniThreadDispatcher<'a, T>,
    pub task_id_factory: AtomicU32,

    // CONTEXT LABEL
    pub context_task: Option<RcUnsafeCell<AbstractTask<'a, T>>>,
}

#[repr(C)]
struct Arm64CpReg {
    crn: u32,
    crm: u32,
    op0: u32,
    op1: u32,
    op2: u32,
    val: u64,
}

impl <'a, T: Clone> AndroidEmulator<'a, T> {
    pub fn new(pid: u32, ppid: u32, proc_name: String, data: T) -> anyhow::Result<AndroidEmulator<'static, T>> {
        let backend = Backend::new(data);
        let mut svc = SvcMemory::new(&backend)?; // ARMSvcMemory

        mem_hook::register_mem_err_handler(backend.clone()); // add hook
        #[cfg(feature = "unicorn_backend")]
        env_fixer::enable_vfp(&backend)?; // enableVFP | init memory

        let (mut memory, errno) = AndroidElfLoader::new(backend.clone(), pid, proc_name.clone())?;
        memory.add_hook_listeners(Box::new(ArmLD64::new(&mut svc)?));

        Ok(AndroidEmulator {
            inner: Arc::new(UnsafeCell::new(AndroidEmulatorInner {
                running: AtomicBool::new(false),
                pid,
                ppid,
                proc_name,
                errno,
                memory,
                svc_memory: svc,
                file_system: AndroidFileSystem::new(),
                context_stack: Vec::new(),
                thread_dispatcher: UniThreadDispatcher::new(),
                task_id_factory: AtomicU32::new(pid + 1),
                dalvik: None,
                context_task: None,
                brk: 0,
                base_path: std::env::var("BASE_PATH").unwrap_or("./android/sdk23".to_string()),
            })),
            backend
        })
    }

    pub(crate) fn inner_mut(&self) -> &mut AndroidEmulatorInner<'a, T> {
        unsafe { &mut *self.inner.get() }
    }

    pub fn destroy(&self) {
        #[cfg(feature = "dynarmic_backend")]
        if let Backend::Dynarmic(dynarmic) = &self.backend {
            //info!("destroy dynarmic size: {}", dynarmic.get_cache_size());
            dynarmic.destroy_callback();
        }

        if let Some(vm) = self.inner_mut().dalvik.as_mut() {
            vm.destroy();
        }

        let count = Arc::strong_count(&self.inner);
        if count == 1 {
            info!("destroy emulator");
        } else {
            error!("destroy emulator: strong_count: {}", count);
        }
    }

    pub fn get_base_path(&self) -> &str {
        self.inner_mut().base_path.as_str()
    }

    /// Set errno value in the emulator.
    ///
    /// # Arguments
    /// * `errno` - The errno value to set.
    ///
    /// # Example
    /// ```no_run
    /// use core::emulator::AndroidEmulator;
    /// use emulator::linux::errno::Errno;
    ///
    /// let emu = AndroidEmulator::create_arm64(32267, 29427, "com.tencent.mobileqq:MSF", ());
    /// emu.set_errno(Errno::EACCES.as_i32()).unwrap();
    /// ```
    pub fn set_errno(&self, errno: i32) -> anyhow::Result<()> {
        let errno_ptr = self.inner_mut().errno;
        if errno_ptr != 0 {
            let mut buf = BytesMut::new();
            buf.put_i32_le(errno);
            self.backend.mem_write(errno_ptr, &buf)
                .map_err(|e| anyhow!("failed to set errno: {:?}", e))?;
            return Ok(());
        }
        Err(anyhow!("errno pointer is none!"))
    }

    pub(crate) fn setup_traps(&mut self) -> anyhow::Result<()> {
        self.backend.mem_map(LR, PAGE_ALIGN, (Permission::READ | Permission::EXEC | Permission::WRITE).bits())
            .map_err(|e| anyhow!("failed to setup traps: {:?}", e))?;
        let mut buf = BytesMut::with_capacity(PAGE_ALIGN);
        let code = -738197503i32;

        let mut i = 0;
        loop {
            buf.put_i32_le(code);
            i += 4;
            if i >= PAGE_ALIGN {
                break
            }
        }

        self.backend.mem_write(LR, &buf.freeze())
            .map_err(|e| anyhow!("failed to setup traps: {:?}", e))?;

        Ok(())
    }

    /// Get the file system instance to prepare for redirecting files or outputting file errors.
    ///
    pub fn get_file_system(&self) -> &mut AndroidFileSystem<T> {
        &mut self.inner_mut().file_system
    }

    pub fn memory(&self) -> &mut AndroidElfLoader<'a, T> {
        &mut self.inner_mut().memory
    }

    pub(crate) fn get_lr(&self) -> anyhow::Result<u64> {
        self.backend.reg_read(RegisterARM64::LR)
            .map_err(|e| anyhow!("get lr failed: {:?}", e))
    }

    pub fn get_current_pid(&self) -> u32 {
        match self.inner_mut().thread_dispatcher.running_task() {
            None => self.inner_mut().pid,
            Some(task_cell) => {
                match unsafe { &*task_cell.get() } {
                    AbstractTask::Function64(_) => self.inner_mut().pid,
                    AbstractTask::SignalTask(task) => task.task_id() as u32,
                    AbstractTask::MarshmallowThread(thread) => thread.get_id(),
                    AbstractTask::KitKatThread(_) => unreachable!()
                }
            }
        }
    }

    pub fn find_caller(&self) -> Option<RcUnsafeCell<LinuxModule<'a, T>>> {
        let lr = self.get_lr().unwrap();
        self.inner_mut().memory
            .find_module_by_address(lr)
    }

    pub(crate) fn find_caller_name(&self) -> String {
        let from_module = self.find_caller();
        if let Some(from_module_cell) = from_module {
            let module = unsafe { &*from_module_cell.get() };
            module.name.clone()
        } else {
            "".to_string()
        }
    }

    pub(crate) fn pop_context(&self) -> Option<i32> {
        let mut stack = &mut self.inner_mut().context_stack;
        if let Some((context, off)) = stack.pop() {
            self.backend.context_restore(&context)
                .expect("failed to restore context");
            drop(context);
            return Some(off);
        }
        None
    }

    pub(crate) fn push_context(&self, context: Context, off: i32) {
        let mut stack = &mut self.inner_mut().context_stack;
        let mut context = context;
        self.backend.context_save(&mut context)
            .expect("failed to save context");
        stack.push((context, off));
    }

    pub fn emulate(&self, begin: u64, until: u64) -> Option<u64> {
        if option_env!("EMU_LOG") == Some("1") {
            println!("emulate from 0x{:x} to 0x{:x}", begin, until);
        }

        if self.inner_mut().running.load(Ordering::SeqCst) {
            self.backend.emu_stop(TaskStatus::X, self)
                .expect("failed to stop emulator");
            panic!("emulator is running");
        }

        self.inner_mut().running.store(true, Ordering::SeqCst);
        self.set_task_status(TaskStatus::R)
            .expect("failed to set task status");
        self.backend.emu_start(begin, until, 0, 0)
            .map_err(|e| error!("failed to start emulator: {:?}", e))
            .expect("failed to start emulator");
        self.inner_mut().running.store(false, Ordering::SeqCst);

        if self.get_task_status() != TaskStatus::X {
            return None;
        }

        let x0 = self.backend.reg_read(RegisterARM64::X0).unwrap();

        Some(x0)
    }

    pub fn set_task_status(&self, status: TaskStatus) -> anyhow::Result<()> {
        let task = self.inner_mut().context_task.as_ref()
            .ok_or(anyhow!("task is none"))?;
        let task = unsafe { &mut *task.get() };
        match task {
            AbstractTask::Function64(task) => task.set_task_status(status),
            AbstractTask::SignalTask(task) => task.set_task_status(status),
            AbstractTask::MarshmallowThread(task) => task.set_task_status(status),
            AbstractTask::KitKatThread(task) => task.set_task_status(status),
        };
        Ok(())
    }

    pub fn get_task_status(&self) -> TaskStatus {
        let task = self.inner_mut().context_task.as_ref()
            .expect("task is none");
        let task = unsafe { &mut *task.get() };
        match task {
            AbstractTask::Function64(task) => task.get_task_status(),
            AbstractTask::SignalTask(task) => task.get_task_status(),
            AbstractTask::MarshmallowThread(task) => task.get_task_status(),
            AbstractTask::KitKatThread(task) => task.get_task_status(),
        }
    }
}

impl<'a, T: Clone> AndroidEmulator<'a, T> {
    pub fn run_thread(&self) -> anyhow::Result<()> {
        let mut dispatcher = self.inner_mut().thread_dispatcher.clone();
        let ret = dispatcher.run_thread(self.clone());
        ret
    }

    pub fn emu_stop(&self, status: TaskStatus) -> anyhow::Result<()> {
        self.backend.emu_stop(status, self)
            .map_err(|e| anyhow!("failed to stop emulator: {:?}", e))
    }

    pub fn e_func(&self, begin: u64, args: Vec<UnicornArg>) -> Option<u64> {
        let f64 = Function64::<T>::new(self.inner_mut().pid, begin, LR, args, false);
        while self.inner_mut().running.load(Ordering::SeqCst) {}
        self.run_main_for_result(AbstractTask::Function64(f64))
    }

    pub(crate) fn run_main_for_result<'b>(&self, luo_task: AbstractTask<'a, T>) -> Option<u64> {
        let sp_back = self.inner_mut().memory.sp;
        let thread_dispatcher = &self.inner_mut().thread_dispatcher;
        let ret = thread_dispatcher.run_main_for_result(luo_task, self.clone());
        self.inner_mut().memory.set_stack_point(sp_back);

        loop {
            let task_count = thread_dispatcher.alive_task_counts();
            if option_env!("EMU_LOG") == Some("1") {
                println!("task count: {}", task_count);
            }
            if task_count != 0 {
                self.run_thread().unwrap();
                //panic!("task count: {}", task_count);
            } else {
                //self.memory().release_tmp_memory();
                break
            }
        }

        ret
    }
}

