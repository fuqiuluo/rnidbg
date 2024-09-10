use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::rc::Rc;
use anyhow::anyhow;
use crate::backend::RegisterARM64;
use crate::emulator::thread::{BaseThreadTask, CoveredTaskSignalOps, DestroyListener, LuoTask, RunnableTask, Task, TaskStatus, Waiter};
use crate::emulator::{AndroidEmulator, VMPointer};
use crate::emulator::func::FunctionCall;
use crate::emulator::signal::{SavableSignalTask, SignalTask};

pub struct MarshmallowThread<'a, T: Clone> {
    base_thread_task: Rc<UnsafeCell<BaseThreadTask<'a, T>>>,
    pub fn_: VMPointer<'a, T>,
    thread: VMPointer<'a, T>,
    tid_ptr: Option<VMPointer<'a, T>>,
    errno: Option<VMPointer<'a, T>>,
}

impl<'a, T: Clone> MarshmallowThread<'a, T> {
    pub fn new(
        emulator: AndroidEmulator<'a, T>,
        tid: u32,
        fn_: VMPointer<'a, T>,
        thread: VMPointer<'a, T>,
        tid_ptr: Option<VMPointer<'a, T>>,
    ) -> Self {
        Self {
            base_thread_task: Rc::new(UnsafeCell::new(BaseThreadTask::new(tid, emulator.get_lr().unwrap()))),
            fn_, thread, tid_ptr,
            errno: None,
        }
    }

    pub fn thread_task_mut(&self) -> &mut BaseThreadTask<'a, T> {
        unsafe { &mut *self.base_thread_task.get() }
    }

    pub fn set_exit_status(&mut self, status: i32) {
        self.thread_task_mut().finished = true;
        self.thread_task_mut().exit_status = status;

        if let Some(tid_ptr) = &self.tid_ptr {
            if tid_ptr.addr != 0 {
                tid_ptr.write_i32_with_offset(0, 0)
                    .expect("failed to write tid");
            }
        }
    }

    pub fn set_tid_ptr(&mut self, tid_ptr: VMPointer<'a, T>) {
        self.tid_ptr = Some(tid_ptr);
    }
}

impl<'a, T: Clone> RunnableTask<'a, T> for MarshmallowThread<'a, T> {
    fn can_dispatch(&self) -> bool {
        self.thread_task_mut().can_dispatch()
    }

    fn save_context(&mut self, emulator: &AndroidEmulator<'a, T>) {
        self.thread_task_mut().save_context(emulator)
    }

    fn is_context_saved(&self) -> bool {
        self.thread_task_mut().is_context_saved()
    }

    fn restore_context(&self, emulator: &AndroidEmulator<'a, T>) {
        self.thread_task_mut().restore_context(emulator)
    }

    fn destroy(&self, emulator: &AndroidEmulator<'a, T>) {
        self.thread_task_mut().destroy(emulator)
    }

    fn set_waiter(&mut self, emulator: &AndroidEmulator<'a, T>, waiter: Waiter<'a, T>) {
        self.thread_task_mut().set_waiter(emulator, waiter)
    }

    fn get_waiter(&mut self) -> Option<&mut Waiter<'a, T>> {
        self.thread_task_mut().get_waiter()
    }

    fn set_result(&self, emulator: &AndroidEmulator<'a, T>, ret: u64) {}

    fn set_destroy_listener(&mut self, listener: Box<dyn DestroyListener<'a, T>>) {
        self.thread_task_mut().set_destroy_listener(listener)
    }

    fn pop_context(&mut self, emulator: &AndroidEmulator<'a, T>) {
        self.thread_task_mut().pop_context(emulator)
    }

    fn push_function(&mut self, emulator: &AndroidEmulator<'a, T>, call: FunctionCall) {
        self.thread_task_mut().push_function(emulator, call)
    }

    fn pop_function(&mut self, emulator: &AndroidEmulator<'a, T>, address: u64) -> Option<FunctionCall> {
        self.thread_task_mut().pop_function(emulator, address)
    }

    fn get_task_status(&self) -> TaskStatus {
        self.thread_task_mut().get_task_status()
    }

    fn set_task_status(&mut self, status: TaskStatus) {
        self.thread_task_mut().set_task_status(status)
    }
}

impl<'a, T: Clone> Task<'a, T> for MarshmallowThread<'a, T> {
    fn get_id(&self) -> u32 {
        self.thread_task_mut().get_id()
    }

    fn dispatch_inner(&mut self, emulator: &AndroidEmulator<'a, T>, luo_task: &dyn LuoTask<'a, T>) -> anyhow::Result<Option<u64>> {
        self.thread_task_mut().dispatch_inner(emulator, luo_task)
    }

    fn dispatch(&mut self, emulator: &AndroidEmulator<'a, T>) -> anyhow::Result<Option<u64>> {
        let backend = emulator.backend.clone();
        let main_task = self.thread_task_mut();
        let stack = main_task.allocate_stack(emulator);

        let tls = self.thread.share(0xb0);
        self.errno = Some(tls.share(16));
        backend.reg_write(RegisterARM64::X0, self.thread.addr)
            .map_err(|e| anyhow!("[thread_addr] failed to write X0: {:?}", e))?;
        backend.reg_write(RegisterARM64::SP, stack.addr)
            .map_err(|e| anyhow!("[stack_addr] failed to write SP: {:?}", e))?;
        backend.reg_write(RegisterARM64::TPIDR_EL0, tls.addr)
            .map_err(|e| anyhow!("[tls_addr] failed to write TPIDR_EL0: {:?}", e))?;
        backend.reg_write(RegisterARM64::LR, self.thread_task_mut().until)
            .map_err(|e| anyhow!("[base_thread_until] failed to write LR: {:?}", e))?;

        self.thread_task_mut().dispatch_inner(emulator, self)
    }

    fn is_main_thread(&self) -> bool {
        self.thread_task_mut().is_main_thread()
    }

    fn is_finish(&self) -> bool {
        self.thread_task_mut().is_finish()
    }

    fn add_signal_task(&mut self, task: SignalTask<'a, T>) {
        self.thread_task_mut().add_signal_task(task)
    }

    fn get_signal_task_list(&self) -> &Vec<SavableSignalTask<'a, T>> {
        self.thread_task_mut().get_signal_task_list()
    }

    fn remove_signal_task(&mut self, task_cell: SavableSignalTask<'a, T>) {
        self.thread_task_mut().remove_signal_task(task_cell)
    }

    fn set_errno(&self, emulator: &AndroidEmulator<'a, T>, errno_value: i32) -> bool {
        if let Some(errno) = &self.errno {
            errno.write_i32_with_offset(0, errno_value)
                .expect("failed to write errno");
            return true;
        }
        self.thread_task_mut().set_errno(emulator, errno_value)
    }

    fn signal_ops_mut(&self) -> &mut CoveredTaskSignalOps {
        self.thread_task_mut().signal_ops_mut()
    }
}

impl<'a, T: Clone> LuoTask<'a, T> for MarshmallowThread<'a, T> {
    fn run(&self, emulator: &AndroidEmulator<'a, T>) -> anyhow::Result<Option<u64>> {
        Ok(emulator.emulate(self.fn_.addr, self.thread_task_mut().until))
    }
}

pub struct KitKatThread<'a, T: Clone> {
    base_thread_task: BaseThreadTask<'a, T>,
    fn_: VMPointer<'a, T>,
    child_stack: VMPointer<'a, T>,
    arg: Option<VMPointer<'a, T>>,
    errno: Option<VMPointer<'a, T>>,
    pd: PhantomData<&'a T>,
}

impl<'a, T: Clone> KitKatThread<'a, T> {
    pub fn new(
        emulator: AndroidEmulator<'a, T>,
        tid: u32,
        fn_: VMPointer<'a, T>,
        child_stack: VMPointer<'a, T>,
        arg: Option<VMPointer<'a, T>>,
    ) -> Self {
        Self {
            base_thread_task: BaseThreadTask::new(tid, emulator.get_lr().unwrap()),
            fn_, child_stack, arg,
            errno: None,
            pd: PhantomData,
        }
    }

    pub fn set_exit_status(&mut self, status: i32) {
        self.base_thread_task.finished = true;
        self.base_thread_task.exit_status = status;
    }
}

impl<'a, T: Clone> Task<'a, T> for KitKatThread<'a, T> {
    fn get_id(&self) -> u32 {
        panic!("持有一半的梦尚未回环")
    }

    fn dispatch_inner(&mut self, emulator: &AndroidEmulator<'a, T>, luo_task: &dyn LuoTask<'a, T>) -> anyhow::Result<Option<u64>> {
        panic!("许三生约定的千万羁绊")
    }

    fn dispatch(&mut self, emulator: &AndroidEmulator<'a, T>) -> anyhow::Result<Option<u64>> {
        panic!("一条殊途")
    }

    fn is_main_thread(&self) -> bool {
        panic!("绝不回转")
    }

    fn is_finish(&self) -> bool {
        panic!("你眼中倒映的星河浪漫")
    }

    fn add_signal_task(&mut self, task: SignalTask<'a, T>) {
        panic!("是不曾见过的世外梦幻")
    }

    fn get_signal_task_list(&self) -> &Vec<SavableSignalTask<'a, T>> {
        panic!("万水千山，你陪我看")
    }

    fn remove_signal_task(&mut self, task_cell: SavableSignalTask<'a, T>) {
        panic!("为你闯出的世界，贯穿世界的消亡")
    }

    fn set_errno(&self, emulator: &AndroidEmulator<'a, T>, errno: i32) -> bool {
        panic!("将弱小的自己藏匿抹杀")
    }

    fn signal_ops_mut(&self) -> &mut CoveredTaskSignalOps {
        panic!("可你说过的话，全部都被遗忘")
    }
}

impl<'a, T: Clone> RunnableTask<'a, T> for KitKatThread<'a, T> {
    fn can_dispatch(&self) -> bool {
        todo!()
    }

    fn save_context(&mut self, emulator: &AndroidEmulator<'a, T>) {
        todo!()
    }

    fn is_context_saved(&self) -> bool {
        todo!()
    }

    fn restore_context(&self, emulator: &AndroidEmulator<'a, T>) {
        todo!()
    }

    fn destroy(&self, emulator: &AndroidEmulator<'a, T>) {
        todo!()
    }

    fn set_waiter(&mut self, emulator: &AndroidEmulator<'a, T>, waiter: Waiter<'a, T>) {
        todo!()
    }

    fn get_waiter(&mut self) -> Option<&mut Waiter<'a, T>> {
        todo!()
    }

    fn set_result(&self, emulator: &AndroidEmulator<'a, T>, ret: u64) {
        todo!()
    }

    fn set_destroy_listener(&mut self, listener: Box<dyn DestroyListener<'a, T>>) {
        todo!()
    }

    fn pop_context(&mut self, emulator: &AndroidEmulator<'a, T>) {
        todo!()
    }

    fn push_function(&mut self, emulator: &AndroidEmulator<'a, T>, call: FunctionCall) {
        todo!()
    }

    fn pop_function(&mut self, emulator: &AndroidEmulator<'a, T>, address: u64) -> Option<FunctionCall> {
        todo!()
    }

    fn get_task_status(&self) -> TaskStatus {
        todo!()
    }

    fn set_task_status(&mut self, status: TaskStatus) {
        todo!()
    }
}

impl<'a, T: Clone> LuoTask<'a, T> for KitKatThread<'a, T> {
    fn run(&self, emulator: &AndroidEmulator<'a, T>) -> anyhow::Result<Option<u64>> {
        panic!("I'm sorry, Dave. I'm afraid I can't do that.")

        //let backend = emulator.backend.clone();
        //let stack = self.base_thread_task.allocate_stack(emulator);
        //self.errno = Some(self.child_stack.share(8));

        //backend.reg_write(RegisterARM64::SP, stack.addr)
        //    .map_err(|e| anyhow!("[stack_addr] failed to write SP: {:?}", e))?;
    }
}