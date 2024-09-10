use log::error;
use syscall_handler::register_syscall_handler;
use crate::emulator::{AndroidEmulator, syscall_handler};

pub mod dvm;
pub mod virtual_library;
pub mod jni;
mod structs;

impl<T: Clone> AndroidEmulator<'_, T> {
    /// Provide [pid], [ppid], [proc_name] to construct an android arm64 emulator.
    /// This process will perform initialization operations and
    /// allocate some memory required by the stack/Svc
    ///
    /// # Example
    /// ```
    /// use core::emulator::AndroidEmulator;
    ///
    /// let emu = AndroidEmulator::create_arm64(32267, 29427, "com.tencent.mobileqq:MSF", ());
    /// ```
    ///
    /// # Arguments
    /// * `pid` - Process ID
    /// * `ppid` - Parent Process ID
    /// * `proc_name` - Process Name
    /// * `data` - Generic data(useless)
    pub fn create_arm64(
        pid: u32,
        ppid: u32,
        proc_name: &str,
        data: T
    ) -> AndroidEmulator<'static, T> {
        let mut context: AndroidEmulator<'static, T> = AndroidEmulator::new(pid, ppid, proc_name.to_string(), data)
            .map_err(|e| error!("failed to init emu: {}", e))
            .unwrap();

        context.set_errno(0)
            .expect("failed to set errno");

        register_syscall_handler(&context);

        context.setup_traps()
            .map_err(|e| error!("failed to setup traps: {}", e))
            .unwrap();

        context
    }
}
