use std::sync::atomic::AtomicI32;
use anyhow::anyhow;
use bytes::{Buf, BufMut, BytesMut};
use crate::backend::RegisterARM64;
use crate::emulator::memory::{MemoryBlockTrait, MemoryBlock};
use crate::emulator::signal::{ISignalTask, SignalOps, UnixSigSet};
use crate::emulator::{AndroidEmulator, VMPointer};
use crate::emulator::func::FunctionCall;
use crate::emulator::thread::{BaseTask, CoveredTaskSignalOps, DestroyListener, RunnableTask, TaskStatus, Waiter};

pub const SA_SIGINFO: i32 = 0x00000004;

pub struct SignalTask<'a, T: Clone> {
    sig_action: SigAction64<'a, T>,
    sig_num: i32,
    sig_seq: i32,
    sig_info: Option<VMPointer<'a, T>>,
    stack: Option<VMPointer<'a, T>>,
    ucontext: Option<MemoryBlock<'a, T>>,
    info_block: Option<MemoryBlock<'a, T>>,
    base_task: BaseTask<'a, T>
}

impl<'a, T: Clone> SignalTask<'a, T> {
    pub fn new(sig_num: i32, sig_action: SigAction64<'a, T>, sig_info: Option<VMPointer<'a, T>>) -> Self {
        Self {
            sig_action,
            sig_num,
            sig_seq: next_sig_seq(),
            sig_info,
            stack: None,
            ucontext: None,
            info_block: None,
            base_task: BaseTask::new()
        }
    }

    pub fn run(&mut self, emulator: &AndroidEmulator<'a, T>) -> anyhow::Result<Option<u64>> {
        let backend = emulator.backend.clone();
        if self.stack.is_none() {
            self.stack = Some(self.base_task.allocate_stack(&emulator));
        }

        if (self.sig_action.sa_flags & SA_SIGINFO) != 0 && self.info_block.is_none() && self.sig_info.is_none() {
            self.info_block = Some(emulator.malloc(128, false)?);
            if let Some(info_block) = &self.info_block {
                info_block.pointer.write_i32_with_offset(0, self.sig_num)?;
                self.sig_info = Some(info_block.pointer.clone());
            }
        }

        if self.ucontext.is_none() {
            self.ucontext = Some(emulator.malloc(0x1000, false)?);
        }

        backend.reg_write(RegisterARM64::SP, self.stack.as_ref().unwrap().addr).map_err(|e| anyhow!("failed to write SP: {:?}", e))?;
        backend.reg_write(RegisterARM64::X0, self.sig_num as u64).map_err(|e| anyhow!("failed to write X0: {:?}", e))?;
        backend.reg_write(RegisterARM64::X1, self.sig_info.as_ref().unwrap().addr).map_err(|e| anyhow!("failed to write X1: {:?}", e))?;
        backend.reg_write(RegisterARM64::X2, self.ucontext.as_ref().unwrap().pointer.addr).map_err(|e| anyhow!("failed to write X2: {:?}", e))?;
        backend.reg_write(RegisterARM64::LR, emulator.get_lr()?).map_err(|e| anyhow!("failed to write LR: {:?}", e))?;

        Ok(emulator.emulate(self.sig_action.sa_handler, emulator.get_lr().unwrap()))
    }
}

impl<'a, T: Clone> RunnableTask<'a, T> for SignalTask<'a, T> {
    fn can_dispatch(&self) -> bool {
        self.base_task.can_dispatch()
    }

    fn save_context(&mut self, emulator: &AndroidEmulator<'a, T>) {
        self.base_task.save_context(emulator)
    }

    fn is_context_saved(&self) -> bool {
        self.base_task.is_context_saved()
    }

    fn restore_context(&self, emulator: &AndroidEmulator<'a, T>) {
        self.base_task.restore_context(emulator)
    }

    fn destroy(&self, emulator: &AndroidEmulator<'a, T>) {
        self.base_task.destroy(emulator)
    }

    fn set_waiter(&mut self, emulator: &AndroidEmulator<'a, T>, waiter: Waiter<'a, T>) {
        self.base_task.set_waiter(waiter)
    }

    fn get_waiter(&mut self) -> Option<&mut Waiter<'a, T>> {
        self.base_task.get_waiter()
    }

    fn set_result(&self, emulator: &AndroidEmulator<'a, T>, ret: u64) {
        self.base_task.set_result(emulator, ret)
    }

    fn set_destroy_listener(&mut self, listener: Box<dyn DestroyListener<'a, T>>) {
        self.base_task.set_destroy_listener(listener)
    }

    fn pop_context(&mut self, emulator: &AndroidEmulator<'a, T>) {
        self.base_task.pop_context(emulator)
    }

    fn push_function(&mut self, emulator: &AndroidEmulator<'a, T>, call: FunctionCall) {
        self.base_task.push_function(emulator, call)
    }

    fn pop_function(&mut self, emulator: &AndroidEmulator<'a, T>, address: u64) -> Option<FunctionCall> {
        self.base_task.pop_function(emulator, address)
    }

    fn get_task_status(&self) -> TaskStatus {
        self.base_task.get_task_status()
    }

    fn set_task_status(&mut self, status: TaskStatus) {
        self.base_task.set_task_status(status);
    }
}

impl<'a, T: Clone> ISignalTask<'a, T> for SignalTask<'a, T> {
    fn call_handler(&mut self, signal_ops: &mut CoveredTaskSignalOps, emulator: &AndroidEmulator<'a, T>) -> anyhow::Result<Option<u64>> {
        let mut sig_set = signal_ops.get_sig_mask_set();
        if sig_set.is_none() {
            let new_sig_set = UnixSigSet::new(self.sig_action.sa_mask);
            signal_ops.set_sig_mask_set(Box::new(new_sig_set));
        } else {
            sig_set.as_mut().unwrap().block_sig_set(self.sig_action.sa_mask);
        }

        let ret = if self.is_context_saved() {
            self.base_task.continue_run(&emulator, emulator.get_lr()?)
        } else {
            self.run(&emulator)?
        };

        //signal_ops.set_sig_mask_set(sig_set.unwrap());
        Ok(ret)
    }

    fn task_id(&self) -> i32 {
        self.sig_num + self.sig_seq
    }
}

impl<'a, T: Clone> Drop for SignalTask<'a, T> {
    fn drop(&mut self) {
        if let Some(uccontext) = &self.ucontext {
            uccontext.free(None);
        }
        if let Some(info_block) = &self.info_block {
            info_block.free(None);
        }
    }
}


// "sa_handler", "sa_flags", "sa_mask", "sa_restorer"
pub struct SigAction64<'a, T: Clone> {
    pub sa_handler: u64,
    pub sa_flags: i32,
    pub sa_restorer: u64,
    pub sa_mask: u64,
    pub pointer: VMPointer<'a, T>
}

impl<'a, T: Clone> SigAction64<'a, T> {
    pub fn new(pointer: VMPointer<'a, T>, sa_handler: u64, sa_flags: i32, sa_restorer: u64, sa_mask: u64) -> Self {
        Self {
            sa_handler,
            sa_flags,
            sa_restorer,
            sa_mask,
            pointer
        }
    }

    pub fn from(pointer: VMPointer<'a, T>) -> Self {
        let data = pointer.read_bytes_with_len(32)
            .expect("Try read sigaction64 failed");
        let mut buf = BytesMut::from(data.as_slice());
        let sa_handler = buf.get_u64_le();
        let sa_flags = buf.get_i32_le();
        buf.advance(4);
        let sa_mask = buf.get_u64_le();
        let sa_restorer = buf.get_u64_le();
        Self {
            sa_handler,
            sa_flags,
            sa_restorer,
            sa_mask,
            pointer
        }
    }

    pub fn pack(&self) {
        let mut buf = BytesMut::with_capacity(32);
        buf.put_u64_le(self.sa_handler);
        buf.put_i32_le(self.sa_flags);
        buf.put_u32_le(0);
        buf.put_u64_le(self.sa_mask);
        buf.put_u64_le(self.sa_restorer);
        self.pointer.write_bytes(buf.freeze()).expect("Try pack sigaction64 failed");
    }
}

fn next_sig_seq() -> i32 {
    static SIG_SEQ: AtomicI32 = AtomicI32::new(0);
    SIG_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
}
/*
struct sigaction64 {
  union {
    sighandler_t sa_handler;
    void (*sa_sigaction)(int, struct siginfo*, void*);
  };
  int sa_flags;
  void (*sa_restorer)(void);
  sigset64_t sa_mask;
};
 */