use crate::emulator::AndroidEmulator;
use crate::emulator::func::FunctionCall;
use crate::emulator::signal::{SavableSignalTask, SignalTask};
use crate::emulator::thread::{*};

pub enum AbstractTask<'a, T: Clone> {
    Function64(Function64<'a, T>),
    SignalTask(SignalTask<'a, T>),
    MarshmallowThread(MarshmallowThread<'a, T>),
    KitKatThread(KitKatThread<'a, T>), // not implement KitKatThread
}

pub trait LuoTask<'a, T: Clone>: Task<'a, T> + RunnableTask<'a, T> {
    fn run(&self, emulator: &AndroidEmulator<'a, T>) -> anyhow::Result<Option<u64>>;
}

pub struct BaseMainTask<'a, T: Clone> {
    pub until: u64,
    covered_task: CoveredTask<'a, T>,
}

impl<'a, T: Clone> BaseMainTask<'a, T> {
    pub fn new(id: u32, until: u64) -> Self {
        Self {
            until,
            covered_task: CoveredTask::new(id),
        }
    }

    pub fn signal_ops_mut(&self) -> &mut CoveredTaskSignalOps {
        self.covered_task.signal_ops_mut()
    }
}

impl<'a, T: Clone> RunnableTask<'a, T> for BaseMainTask<'a, T> {
    fn can_dispatch(&self) -> bool {
        self.covered_task.can_dispatch()
    }

    fn save_context(&mut self, emulator: &AndroidEmulator<'a, T>) {
        self.covered_task.save_context(emulator);
    }

    fn is_context_saved(&self) -> bool {
        self.covered_task.is_context_saved()
    }

    fn restore_context(&self, emulator: &AndroidEmulator<'a, T>) {
        self.covered_task.restore_context(emulator);
    }

    fn destroy(&self, emulator: &AndroidEmulator<'a, T>) {
        self.covered_task.destroy(emulator);
    }

    fn set_waiter(&mut self, emulator: &AndroidEmulator<'a, T>, waiter: Waiter<'a, T>) {
        self.covered_task.set_waiter(emulator, waiter);
    }

    fn get_waiter(&mut self) -> Option<&mut Waiter<'a, T>> {
        self.covered_task.get_waiter()
    }

    fn set_result(&self, emulator: &AndroidEmulator<'a, T>, ret: u64) {
        self.covered_task.set_result(emulator, ret);
    }

    fn set_destroy_listener(&mut self, listener: Box<dyn DestroyListener<'a, T>>) {
        self.covered_task.set_destroy_listener(listener);
    }

    fn pop_context(&mut self, emulator: &AndroidEmulator<'a, T>) {
        self.covered_task.pop_context(emulator);
    }

    fn push_function(&mut self, emulator: &AndroidEmulator<'a, T>, call: FunctionCall) {
        self.covered_task.push_function(emulator, call);
    }

    fn pop_function(&mut self, emulator: &AndroidEmulator<'a, T>, address: u64) -> Option<FunctionCall> {
        self.covered_task.pop_function(emulator, address)
    }

    fn get_task_status(&self) -> TaskStatus {
        self.covered_task.get_task_status()
    }

    fn set_task_status(&mut self, status: TaskStatus) {
        self.covered_task.set_task_status(status);
    }
}

impl<'a, T: Clone> Task<'a, T> for BaseMainTask<'a, T>  {
    fn get_id(&self) -> u32 {
        self.covered_task.get_id()
    }

    fn dispatch_inner(&mut self, emulator: &AndroidEmulator<'a, T>, luo_task: &dyn LuoTask<'a, T>) -> anyhow::Result<Option<u64>> {
        if self.is_context_saved() {
            let ret = self.covered_task.base_task
                .continue_run(emulator, self.until);
            return Ok(ret);
        }

        luo_task.run(emulator)
    }

    fn dispatch(&mut self, emulator: &AndroidEmulator<'a, T>) -> anyhow::Result<Option<u64>> {
        panic!("天下乌鸦一般黑！")
    }

    fn is_main_thread(&self) -> bool {
        true
    }

    fn is_finish(&self) -> bool {
        false
    }

    fn add_signal_task(&mut self, task: SignalTask<'a, T>) {
        self.covered_task.add_signal_task(task);
    }

    fn get_signal_task_list(&self) -> &Vec<SavableSignalTask<'a, T>> {
        self.covered_task.get_signal_task_list()
    }

    fn remove_signal_task(&mut self, task_cell: SavableSignalTask<'a, T>) {
        self.covered_task.remove_signal_task(task_cell);
    }

    fn set_errno(&self, emulator: &AndroidEmulator<'a, T>, errno: i32) -> bool {
        self.covered_task.set_errno(emulator, errno)
    }

    fn signal_ops_mut(&self) -> &mut CoveredTaskSignalOps {
        self.covered_task.signal_ops_mut()
    }
}