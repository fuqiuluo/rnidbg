use anyhow::anyhow;
use crate::emulator::{AndroidEmulator, VMPointer};
use crate::emulator::func::FunctionCall;
use crate::emulator::signal::{SavableSignalTask, SignalOps, ISignalTask, SigSet, SignalTask};
use crate::emulator::thread::{CoveredTask, CoveredTaskSignalOps, DestroyListener, LuoTask, RunnableTask, Task, TaskStatus, Waiter};

pub struct BaseThreadTask<'a, T: Clone> {
    pub until: u64,
    pub exit_status: i32,
    pub finished: bool,
    covered_task: CoveredTask<'a, T>
}

impl<'a, T: Clone> BaseThreadTask<'a, T> {
    pub fn allocate_stack(&mut self, p0: &AndroidEmulator<'a, T>) -> VMPointer<'a, T> {
        self.covered_task.base_task.allocate_stack(p0)
    }
}

impl<T: Clone> BaseThreadTask<'_, T> {
    pub fn new(tid: u32, until: u64) -> Self<> {
        Self {
            until, exit_status: 0, finished: false,
            covered_task: CoveredTask::new(tid)
        }
    }
}

impl<'a, T: Clone> RunnableTask<'a, T> for BaseThreadTask<'a, T> {
    fn can_dispatch(&self) -> bool {
        self.covered_task.can_dispatch()
    }

    fn save_context(&mut self, emulator: &AndroidEmulator<'a, T>) {
        self.covered_task.save_context(emulator)
    }

    fn is_context_saved(&self) -> bool {
        self.covered_task.is_context_saved()
    }

    fn restore_context(&self, emulator: &AndroidEmulator<'a, T>) {
        self.covered_task.restore_context(emulator)
    }

    fn destroy(&self, emulator: &AndroidEmulator<'a, T>) {
        self.covered_task.destroy(emulator)
    }

    fn set_waiter(&mut self, emulator: &AndroidEmulator<'a, T>, waiter: Waiter<'a, T>) {
        self.covered_task.set_waiter(emulator, waiter)
    }

    fn get_waiter(&mut self) -> Option<&mut Waiter<'a, T>> {
        self.covered_task.get_waiter()
    }

    fn set_result(&self, emulator: &AndroidEmulator<'a, T>, ret: u64) {
        self.covered_task.set_result(emulator, ret)
    }

    fn set_destroy_listener(&mut self, listener: Box<dyn DestroyListener<'a, T>>) {
        self.covered_task.set_destroy_listener(listener)
    }

    fn pop_context(&mut self, emulator: &AndroidEmulator<'a, T>) {
        self.covered_task.pop_context(emulator)
    }

    fn push_function(&mut self, emulator: &AndroidEmulator<'a, T>, call: FunctionCall) {
        self.covered_task.push_function(emulator, call)
    }

    fn pop_function(&mut self, emulator: &AndroidEmulator<'a, T>, address: u64) -> Option<FunctionCall> {
        self.covered_task.pop_function(emulator, address)
    }

    fn get_task_status(&self) -> TaskStatus {
        self.covered_task.get_task_status()
    }

    fn set_task_status(&mut self, status: TaskStatus) {
        self.covered_task.set_task_status(status)
    }
}

impl<'a, T: Clone> Task<'a, T> for BaseThreadTask<'a, T>
{
    fn get_id(&self) -> u32 {
        self.covered_task.get_id()
    }

    fn dispatch_inner(&mut self, emulator: &AndroidEmulator<'a, T>, luo_task: &dyn LuoTask<'a, T>) -> anyhow::Result<Option<u64>> {
        if self.is_context_saved() {
            return Ok(self.covered_task.base_task.continue_run(emulator, self.until))
        }
        luo_task.run(emulator)
    }

    fn dispatch(&mut self, emulator: &AndroidEmulator<'a, T>) -> anyhow::Result<Option<u64>> {
        panic!("不善于交际 更没兴趣认识你")
    }

    fn is_main_thread(&self) -> bool {
        false
    }

    fn is_finish(&self) -> bool {
        self.finished
    }

    fn add_signal_task(&mut self, task: SignalTask<'a, T>) {
        self.covered_task.add_signal_task(task)
    }

    fn get_signal_task_list(&self) -> &Vec<SavableSignalTask<'a, T>> {
        self.covered_task.get_signal_task_list()
    }

    fn remove_signal_task(&mut self, task_cell: SavableSignalTask<'a, T>) {
        self.covered_task.remove_signal_task(task_cell)
    }

    fn set_errno(&self, emulator: &AndroidEmulator<'a, T>, errno: i32) -> bool {
        self.covered_task.set_errno(emulator, errno)
    }

    fn signal_ops_mut(&self) -> &mut CoveredTaskSignalOps {
        self.covered_task.signal_ops_mut()
    }
}
