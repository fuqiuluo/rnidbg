use std::cell::UnsafeCell;
use std::rc::Rc;
use log::info;
use crate::emulator::AndroidEmulator;
use crate::emulator::func::FunctionCall;
use crate::emulator::signal::{SavableSignalTask, SignalOps, ISignalTask, SigSet, SignalTask};
use crate::emulator::thread::{AbstractTask, BaseTask, DestroyListener, RunnableTask, Task, TaskStatus, Waiter, WaiterTrait};
use crate::emulator::thread::main_task::LuoTask;

pub struct CoveredTask<'a, T: Clone> {
    id: u32,
    pub(crate) base_task: BaseTask<'a, T>,
    signal_task_list: Vec<SavableSignalTask<'a, T>>,
    signal_ops: Rc<UnsafeCell<CoveredTaskSignalOps>>
}

pub struct CoveredTaskSignalOps {
    pub(crate) sig_mask_set: Option<Box<dyn SigSet>>,
    pub(crate) sig_pending_set: Option<Box<dyn SigSet>>,
}

impl<'a, T: Clone> CoveredTask<'a, T> {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            signal_ops: Rc::new(UnsafeCell::new(CoveredTaskSignalOps {
                sig_mask_set: None,
                sig_pending_set: None,
            })),
            signal_task_list: Vec::new(),
            base_task: BaseTask::new(),
        }
    }
}

impl SignalOps for CoveredTaskSignalOps {
    fn get_sig_mask_set(&mut self) -> Option<&mut Box<dyn SigSet>> {
        self.sig_mask_set.as_mut()
    }

    fn set_sig_mask_set(&mut self, sig_mask_set: Box<dyn SigSet>) {
        self.sig_mask_set = Some(sig_mask_set);
    }

    fn get_sig_pending_set(&mut self) -> Option<&mut Box<dyn SigSet>> {
        self.sig_pending_set.as_mut()
    }

    fn set_sig_pending_set(&mut self, sig_pending_set: Box<dyn SigSet>) {
        self.sig_pending_set = Some(sig_pending_set);
    }
}

impl<'a, T: Clone> RunnableTask<'a, T> for CoveredTask<'a, T> {
    fn can_dispatch(&self) -> bool {
        self.base_task.can_dispatch()
    }

    fn save_context(&mut self, emulator: &AndroidEmulator<'a, T>) {
        self.base_task.save_context(emulator);
    }

    fn is_context_saved(&self) -> bool {
        self.base_task.is_context_saved()
    }

    fn restore_context(&self, emulator: &AndroidEmulator<'a, T>) {
        self.base_task.restore_context(emulator);
    }

    fn destroy(&self, emulator: &AndroidEmulator<'a, T>) {
        self.base_task.destroy(emulator);
    }

    fn set_waiter(&mut self, emulator: &AndroidEmulator<'a, T>, waiter: Waiter<'a, T>) {
        self.base_task.set_waiter(waiter);
    }

    fn get_waiter(&mut self) -> Option<&mut Waiter<'a, T>> {
        self.base_task.get_waiter()
    }

    fn set_result(&self, emulator: &AndroidEmulator<'a, T>, ret: u64) {
        self.base_task.set_result(emulator, ret);
    }

    fn set_destroy_listener(&mut self, listener: Box<dyn DestroyListener<'a, T>>) {
        self.base_task.set_destroy_listener(listener);
    }

    fn pop_context(&mut self, emulator: &AndroidEmulator<'a, T>) {
        self.base_task.pop_context(emulator);
    }

    fn push_function(&mut self, emulator: &AndroidEmulator<'a, T>, call: FunctionCall) {
        self.base_task.push_function(emulator, call);
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

impl<'a, T: Clone> Task<'a, T> for CoveredTask<'a, T> {
    fn get_id(&self) -> u32 {
        self.id
    }

    fn dispatch_inner(&mut self, emulator: &AndroidEmulator<'a, T>, luo_task: &dyn LuoTask<'a, T>) -> anyhow::Result<Option<u64>> {
        panic!("这不是梦开始的地方！")
    }

    fn dispatch(&mut self, emulator: &AndroidEmulator<'a, T>) -> anyhow::Result<Option<u64>> {
        panic!("要在这个纷扰的世界里快乐的活着")
    }

    fn is_main_thread(&self) -> bool {
        panic!("为什么要来到这里！")
    }

    fn is_finish(&self) -> bool {
        panic!("我恨你！");
    }

    fn add_signal_task(&mut self, task: SignalTask<'a, T>) {
        if let Some(waiter) = self.base_task.waiter.as_ref() {
            match waiter {
                Waiter::FutexIndefinite(futex_task) => {
                    futex_task.on_signal(&task);
                }
                Waiter::FutexNanoSleep(futex_task) => {
                    futex_task.on_signal(&task);
                }
                Waiter::Unknown(_) => {
                    panic!("unknown waiter: add_signal_task");
                }
            }
        }
        self.signal_task_list.push(Rc::new(UnsafeCell::new(AbstractTask::SignalTask(task))));
    }

    fn get_signal_task_list(&self) -> &Vec<SavableSignalTask<'a, T>> {
        &self.signal_task_list
    }

    fn remove_signal_task(&mut self, task_cell: SavableSignalTask<'a, T>) {
        let task_id = match unsafe { &*task_cell.get() } {
            AbstractTask::SignalTask(task) => task.task_id(),
            _ => panic!("奔跑在人群里面")
        };
        self.signal_task_list.retain(|t| {
            match unsafe { &*t.get() } {
                AbstractTask::SignalTask(task) => task.task_id() != task_id,
                _ => panic!("偶尔和孤单遇见")
            }
        });
    }

    fn set_errno(&self, emulator: &AndroidEmulator<'a, T>, errno: i32) -> bool {
        false
    }

    fn signal_ops_mut(&self) -> &mut CoveredTaskSignalOps {
        unsafe { &mut *self.signal_ops.get() }
    }
}