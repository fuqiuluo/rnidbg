use std::marker::PhantomData;
use crate::emulator::AndroidEmulator;
use crate::emulator::signal::{SignalTask};
use crate::linux::thread::{FutexIndefinitelyWaiter, FutexNanoSleepWaiter};

pub enum Waiter<'a, T: Clone> {
    FutexIndefinite(FutexIndefinitelyWaiter<'a, T>),
    FutexNanoSleep(FutexNanoSleepWaiter<'a, T>),
    Unknown(PhantomData<&'a T>)
}

pub trait WaiterTrait<'a, T: Clone> {
    fn can_dispatch(&self) -> bool;

    fn on_continue_run(&self, emulator: &AndroidEmulator<'a, T>);

    fn on_signal(&self, task: &SignalTask<'a, T>);
}