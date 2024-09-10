use std::cell::UnsafeCell;
use std::rc::Rc;
use anyhow::anyhow;
use log::info;
use crate::backend::RegisterARM64;
use crate::emulator::AndroidEmulator;
use crate::emulator::func::FunctionCall;
use crate::emulator::signal::{SavableSignalTask, SignalOps, ISignalTask, SigSet, SignalTask};
use crate::emulator::thread::main_task::LuoTask;
use crate::emulator::thread::{DestroyListener, BaseMainTask, RunnableTask, Task, WaiterTrait, BaseThreadTask, CoveredTaskSignalOps, Waiter, TaskStatus};
use crate::tool::{init_args, UnicornArg};

pub struct Function64<'a, T: Clone> {
    pub(crate) main_task: Rc<UnsafeCell<BaseMainTask<'a, T>>>,
    pub padding_argument: bool,
    pub address: u64,
    pub args: Vec<UnicornArg>,
}

impl<'a, T: Clone> Function64<'a, T> {
    pub fn new(pid: u32, address: u64, until: u64, args: Vec<UnicornArg>, padding_argument: bool) -> Self {
        Self {
            main_task: Rc::new(UnsafeCell::new(BaseMainTask::new(pid, until))),
            padding_argument,
            address,
            args,
        }
    }

    pub fn main_task_mut(&self) -> &mut BaseMainTask<'a, T> {
        unsafe { &mut *self.main_task.get() }
    }

    pub fn signal_ops_mut(&self) -> &mut CoveredTaskSignalOps {
        self.main_task_mut().signal_ops_mut()
    }
}

impl<'a, T: Clone> LuoTask<'a, T> for Function64<'a, T> {
    fn run(&self, emulator: &AndroidEmulator<'a, T>) -> anyhow::Result<Option<u64>> {
        let backend = emulator.backend.clone();
        let sp = emulator.inner_mut().memory.sp;
        if sp % 16 != 0 {
            info!("SP NOT 16 bytes aligned");
        }
        init_args(emulator, self.padding_argument, self.args.clone());

        let sp = emulator.inner_mut().memory.sp;
        if sp % 16 != 0 {
            info!("SP NOT 16 bytes aligned");
        }

        let main_task = self.main_task_mut();
        backend.reg_write(RegisterARM64::LR, main_task.until)
            .expect("failed to write LR");

        Ok(emulator.emulate(self.address, main_task.until))
    }
}

impl<'a, T: Clone> RunnableTask<'a, T> for Function64<'a, T>  {
    fn can_dispatch(&self) -> bool {
        let main_task = self.main_task_mut();
        main_task.can_dispatch()
    }

    fn save_context(&mut self, emulator: &AndroidEmulator<'a, T>) {
        let main_task = self.main_task_mut();
        main_task.save_context(emulator);
    }

    fn is_context_saved(&self) -> bool {
        let main_task = self.main_task_mut();
        main_task.is_context_saved()
    }

    fn restore_context(&self, emulator: &AndroidEmulator<'a, T>) {
        let main_task = self.main_task_mut();
        main_task.restore_context(emulator);
    }

    fn destroy(&self, emulator: &AndroidEmulator<'a, T>) {
        let main_task = self.main_task_mut();
        main_task.destroy(emulator);
    }

    fn set_waiter(&mut self, emulator: &AndroidEmulator<'a, T>, waiter: Waiter<'a, T>) {
        let main_task = self.main_task_mut();
        main_task.set_waiter(emulator, waiter);
    }

    fn get_waiter(&mut self) -> Option<&mut Waiter<'a, T>> {
        let main_task = self.main_task_mut();
        main_task.get_waiter()
    }

    fn set_result(&self, emulator: &AndroidEmulator<'a, T>, ret: u64) {
        let main_task = self.main_task_mut();
        main_task.set_result(emulator, ret);
    }

    fn set_destroy_listener(&mut self, listener: Box<dyn DestroyListener<'a, T>>) {
        let main_task = self.main_task_mut();
        main_task.set_destroy_listener(listener);
    }

    fn pop_context(&mut self, emulator: &AndroidEmulator<'a, T>) {
        let main_task = self.main_task_mut();
        main_task.pop_context(emulator);
    }

    fn push_function(&mut self, emulator: &AndroidEmulator<'a, T>, call: FunctionCall) {
        let main_task = self.main_task_mut();
        main_task.push_function(emulator, call);
    }

    fn pop_function(&mut self, emulator: &AndroidEmulator<'a, T>, address: u64) -> Option<FunctionCall> {
        let main_task = self.main_task_mut();
        main_task.pop_function(emulator, address)
    }

    fn get_task_status(&self) -> TaskStatus {
        let main_task = self.main_task_mut();
        main_task.get_task_status()
    }

    fn set_task_status(&mut self, status: TaskStatus) {
        let main_task = self.main_task_mut();
        main_task.set_task_status(status);
    }
}

impl<'a, T: Clone> Task<'a, T> for Function64<'a, T> {
    fn get_id(&self) -> u32 {
        let main_task = self.main_task_mut();
        main_task.get_id()
    }

    fn dispatch_inner(&mut self, emulator: &AndroidEmulator<'a, T>, luo_task: &dyn LuoTask<'a, T>) -> anyhow::Result<Option<u64>> {
        panic!("人各有志")
    }

    fn dispatch(&mut self, emulator: &AndroidEmulator<'a, T>) -> anyhow::Result<Option<u64>> {
        let main_task = self.main_task_mut();
        main_task.dispatch_inner(emulator, self)
    }

    fn is_main_thread(&self) -> bool {
        let main_task = self.main_task_mut();
        main_task.is_main_thread()
    }

    fn is_finish(&self) -> bool {
        let main_task = self.main_task_mut();
        main_task.is_finish()
    }

    fn add_signal_task(&mut self, task: SignalTask<'a, T>) {
        let main_task = self.main_task_mut();
        main_task.add_signal_task(task);
    }

    fn get_signal_task_list(&self) -> &Vec<SavableSignalTask<'a, T>> {
        let main_task = self.main_task_mut();
        main_task.get_signal_task_list()
    }

    fn remove_signal_task(&mut self, task_cell: SavableSignalTask<'a, T>) {
        let main_task = self.main_task_mut();
        main_task.remove_signal_task(task_cell);
    }

    fn set_errno(&self, emulator: &AndroidEmulator<'a, T>, errno: i32) -> bool {
        let main_task = self.main_task_mut();
        main_task.set_errno(emulator, errno)
    }

    fn signal_ops_mut(&self) -> &mut CoveredTaskSignalOps {
        self.signal_ops_mut()
    }
}