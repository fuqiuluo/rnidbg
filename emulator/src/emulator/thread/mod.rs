mod waiter;
mod task;
mod base_task;
mod covered_task;
mod main_task;
mod function64;
mod thread_dispatcher;
mod thread_task;
mod linux_thread;

use crate::emulator::AndroidEmulator;
use crate::emulator::func::FunctionCall;
pub use waiter::{*};
pub use task::Task;
pub use base_task::BaseTask;
pub use covered_task::CoveredTask;
pub use main_task::{*};
pub use function64::Function64;
pub use thread_task::{*};
pub use thread_dispatcher::{*};
pub use covered_task::{*};
pub use linux_thread::{*};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialOrd, PartialEq)]
pub enum TaskStatus {
    R,
    S,
    T,
    Z,
    X,
    I,
    K,
    W
}

pub trait RunnableTask<'a, T: Clone> {
    fn can_dispatch(&self) -> bool;

    fn save_context(&mut self, emulator: &AndroidEmulator<'a, T>);

    fn is_context_saved(&self) -> bool;

    fn restore_context(&self, emulator: &AndroidEmulator<'a, T>);

    fn destroy(&self, emulator: &AndroidEmulator<'a, T>);

    fn set_waiter(&mut self, emulator: &AndroidEmulator<'a, T>, waiter: Waiter<'a, T>);

    fn get_waiter(&mut self) -> Option<&mut Waiter<'a, T>>;

    fn set_result(&self, emulator: &AndroidEmulator<'a, T>, ret: u64);

    fn set_destroy_listener(&mut self, listener: Box<dyn DestroyListener<'a, T>>);

    fn pop_context(&mut self, emulator: &AndroidEmulator<'a, T>);

    fn push_function(&mut self, emulator: &AndroidEmulator<'a, T>, call: FunctionCall);

    fn pop_function(&mut self, emulator: &AndroidEmulator<'a, T>, address: u64) -> Option<FunctionCall>;

    fn get_task_status(&self) -> TaskStatus;

    fn set_task_status(&mut self, status: TaskStatus);
}

pub trait DestroyListener<'a, T: Clone> {
    fn on_destroy(&self, emulator: &AndroidEmulator<'a, T>);
}