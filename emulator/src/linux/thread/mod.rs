use std::time::{SystemTime, UNIX_EPOCH};
use crate::backend::{Backend, RegisterARM64};
use crate::emulator::AndroidEmulator;
use crate::emulator::signal::SignalTask;
use crate::emulator::thread::{WaiterTrait};
use crate::linux::errno::Errno;

pub struct FutexIndefinitelyWaiter<'a, T: Clone> {
    uaddr: u64,
    val: u32,
    woken_up: bool,
    backend: Backend<'a, T>
}

impl<'a, T: Clone> FutexIndefinitelyWaiter<'a, T> {
    pub fn new(uaddr: u64, val: u32, backend: &Backend<'a, T>) -> Self {
        Self {
            uaddr,
            val,
            woken_up: false,
            backend: backend.clone()
        }
    }

    pub fn wake_up(&mut self, addr: u64) -> bool {
        if addr == self.uaddr {
            self.woken_up = true;
            return true;
        }
        false
    }
}

impl<'a, T: Clone> WaiterTrait<'a, T> for FutexIndefinitelyWaiter<'a, T> {
    fn can_dispatch(&self) -> bool {
        if self.woken_up {
            return true
        }
        let mut old = [0u8; 4];
        self.backend.mem_read(self.uaddr, &mut old)
            .expect("failed to read uaddr");
        let val = u32::from_le_bytes(old);
        val != self.val
    }

    fn on_continue_run(&self, emulator: &AndroidEmulator<'a, T>) {
        if self.woken_up {
            self.backend.reg_write(RegisterARM64::X0, 0).expect("failed to write X0");
        } else {
            let errno: i32 = Errno::EAGAIN.into();
            self.backend.reg_write_i32(RegisterARM64::X0, -errno).expect("failed to write X0: EAGAIN");
        }
    }

    fn on_signal(&self, task: &SignalTask<'a, T>) {
    }
}

pub struct FutexNanoSleepWaiter<'a, T: Clone> {
    uaddr: u64,
    val: u32,
    woken_up: bool,
    wait_millis: u64,
    start_time: u64,
    backend: Backend<'a, T>
}

impl<'a, T: Clone> FutexNanoSleepWaiter<'a, T> {
    pub fn new(uaddr: u64, val: u32, wait_millis: u64, backend: Backend<'a, T>) -> Self {
        let start_time = if let Ok(duration_since_epoch) = SystemTime::now().duration_since(UNIX_EPOCH) {
            duration_since_epoch.as_secs()
        } else {
            panic!("SystemTime before UNIX EPOCH!");
        };
        Self {
            uaddr,
            val,
            wait_millis,
            start_time,
            backend,
            woken_up: false
        }
    }

    pub fn wake_up(&mut self, addr: u64) -> bool {
        if addr == self.uaddr {
            self.woken_up = true;
            return true;
        }
        false
    }
}

impl<'a, T: Clone> WaiterTrait<'a, T> for FutexNanoSleepWaiter<'a, T> {
    fn can_dispatch(&self) -> bool {
        if self.woken_up {
            return true
        }
        let mut old = [0u8; 4];
        self.backend.mem_read(self.uaddr, &mut old)
            .expect("failed to read uaddr");
        let val = u32::from_le_bytes(old);
        if val != self.val {
            return true;
        }

        let time_millis_now = if let Ok(duration_since_epoch) = SystemTime::now().duration_since(UNIX_EPOCH) {
            duration_since_epoch.as_secs()
        } else {
            panic!("SystemTime before UNIX EPOCH!");
        };
        if time_millis_now - self.start_time >= self.wait_millis {
            return true
        }

        false
    }

    fn on_continue_run(&self, emulator: &AndroidEmulator<'a, T>) {
        if self.woken_up {
            self.backend.reg_write(RegisterARM64::X0, 0).expect("failed to write X0");
        } else {
            let errno: i32 = Errno::ETIMEDOUT.into();
            self.backend.reg_write_i32(RegisterARM64::X0, -errno).expect("failed to write X0: ETIMEDOUT");
        }
    }

    fn on_signal(&self, task: &SignalTask<'a, T>) {
    }
}