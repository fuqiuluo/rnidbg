use crate::emulator::AndroidEmulator;
use crate::emulator::signal::{SavableSignalTask, SignalTask};
use crate::emulator::thread::{CoveredTaskSignalOps, RunnableTask};
use crate::emulator::thread::main_task::LuoTask;

pub trait Task<'a, T: Clone>: RunnableTask<'a, T> {
    fn get_id(&self) -> u32;

    fn dispatch_inner(&mut self, emulator: &AndroidEmulator<'a, T>, luo_task: &dyn LuoTask<'a, T>) -> anyhow::Result<Option<u64>>;

    fn dispatch(&mut self, emulator: &AndroidEmulator<'a, T>) -> anyhow::Result<Option<u64>>;

    fn is_main_thread(&self) -> bool;

    fn is_finish(&self) -> bool;

    fn add_signal_task(&mut self, task: SignalTask<'a, T>);

    fn get_signal_task_list(&self) -> &Vec<SavableSignalTask<'a, T>>;

    fn remove_signal_task(&mut self, task_cell: SavableSignalTask<'a, T>);

    fn set_errno(&self, emulator: &AndroidEmulator<'a, T>, errno: i32) -> bool;

    fn signal_ops_mut(&self) -> &mut CoveredTaskSignalOps;
}

