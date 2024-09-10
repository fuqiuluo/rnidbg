mod unix_sig_set;
mod signal_task;

use crate::emulator::{AndroidEmulator, RcUnsafeCell};

pub use unix_sig_set::{*};
use crate::emulator::thread::{AbstractTask, CoveredTaskSignalOps, RunnableTask};
pub use signal_task::{*};

pub type SavableSignalTask<'a, T> = RcUnsafeCell<AbstractTask<'a, T>>;

pub trait ISignalTask<'a, T: Clone>: RunnableTask<'a, T> {
    fn call_handler(&mut self, signal_ops: &mut CoveredTaskSignalOps, emulator: &AndroidEmulator<'a, T>) -> anyhow::Result<Option<u64>>;

    fn task_id(&self) -> i32;
}

pub trait SignalOps {
    fn get_sig_mask_set(&mut self) -> Option<&mut Box<dyn SigSet>>;

    fn set_sig_mask_set(&mut self, sig_mask_set: Box<dyn SigSet>);

    fn get_sig_pending_set(&mut self) -> Option<&mut Box<dyn SigSet>>;

    fn set_sig_pending_set(&mut self, sig_pending_set: Box<dyn SigSet>);
}

pub trait SigSet {
    fn get_mask(&self) -> u64;

    fn set_mask(&mut self, mask: u64);

    fn block_sig_set(&mut self, mask: u64);

    fn unblock_sig_set(&mut self, mask: u64);

    fn contains_sig_number(&self, signum: i32) -> bool;

    fn remove_sig_number(&mut self, signum: i32);

    fn add_sig_number(&mut self, signum: i32);
}