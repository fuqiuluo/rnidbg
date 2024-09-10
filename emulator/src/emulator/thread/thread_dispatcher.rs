use std::cell::UnsafeCell;
use std::collections::VecDeque;
use std::ptr::read;
use std::rc::Rc;
use log::{error, warn};
use crate::emulator::AndroidEmulator;
use crate::emulator::signal::{SignalOps, ISignalTask, SigSet, UnixSigSet, SavableSignalTask, SignalTask};
use crate::emulator::thread::{AbstractTask, BaseThreadTask, CoveredTaskSignalOps, LuoTask, RunnableTask, Task, TaskStatus};

pub type SavableTask<'a, T> = Rc<UnsafeCell<Box<dyn LuoTask<'a, T>>>>;

pub trait ThreadDispatcher<'a, T: Clone>  {
    fn add_thread(&self, thread: AbstractTask<'a, T>);

    fn task_list(&self) -> &VecDeque<Rc<UnsafeCell<AbstractTask<'a, T>>>>;

    fn run_main_for_result(&self, main_task: AbstractTask<'a, T>, emulator: AndroidEmulator<'a, T>) -> Option<u64>;

    fn task_counts(&self) -> usize;

    fn alive_task_counts(&self) -> usize;

    fn send_signal(&self, tid: u32, sig: i32, signal_task: Option<SignalTask<'a, T>>) -> bool;

    fn running_task(&self) -> Option<Rc<UnsafeCell<AbstractTask<'a, T>>>>;
}

#[derive(Clone)]
pub struct UniThreadDispatcher<'a, T: Clone> {
    task_list: Rc<UnsafeCell<VecDeque<Rc<UnsafeCell<AbstractTask<'a, T>>>>>>,
    thread_task: Rc<UnsafeCell<VecDeque<Rc<UnsafeCell<AbstractTask<'a, T>>>>>>,
    running_task: Rc<UnsafeCell<Option<Rc<UnsafeCell<AbstractTask<'a, T>>>>>>,
    signal_ops: Rc<UnsafeCell<CoveredTaskSignalOps>>,
}

impl<'a, T: Clone> UniThreadDispatcher<'a, T> {
    pub fn new() -> Self {
        Self {
            task_list: Rc::new(UnsafeCell::new(VecDeque::new())),
            thread_task: Rc::new(UnsafeCell::new(VecDeque::new())),
            signal_ops: Rc::new(UnsafeCell::new(CoveredTaskSignalOps {
                sig_mask_set: None,
                sig_pending_set: None
            })),
            running_task: Rc::new(UnsafeCell::new(None)),
        }
    }

    pub fn task_list(&self) -> &VecDeque<Rc<UnsafeCell<AbstractTask<'a, T>>>> {
        unsafe { &*self.task_list.get() }
    }

    pub fn thread_task(&self) -> &VecDeque<Rc<UnsafeCell<AbstractTask<'a, T>>>> {
        unsafe { &*self.thread_task.get() }
    }

    pub fn task_list_mut(&self) -> &mut VecDeque<Rc<UnsafeCell<AbstractTask<'a, T>>>> {
        unsafe { &mut *self.task_list.get() }
    }

    pub fn thread_task_mut(&self) -> &mut VecDeque<Rc<UnsafeCell<AbstractTask<'a, T>>>> {
        unsafe { &mut *self.thread_task.get() }
    }

    pub fn signal_ops_mut(&self) -> &mut CoveredTaskSignalOps {
        unsafe { &mut *self.signal_ops.get() }
    }

    pub fn running_task_mut(&self) -> &mut Option<Rc<UnsafeCell<AbstractTask<'a, T>>>> {
        unsafe { &mut *self.running_task.get() }
    }

    pub fn run_thread(&self, emulator: AndroidEmulator<'a, T>) -> anyhow::Result<()> {
        let mut context = emulator.backend.context_alloc()?;
        emulator.backend.context_save(&mut context)?;
        self.run(emulator.clone(), 0)?;
        emulator.backend.context_restore(&context)?;
        Ok(())
    }

    fn run(&self, emulator: AndroidEmulator<'a, T>, timeout_sec: u64) -> anyhow::Result<Option<u64>> {
        let start = std::time::Instant::now();
        loop {
            if self.task_list_mut().is_empty() {
                break Ok(None)
            }

            let mut main_ret = None;
            let mut is_main_finish = false;
            let mut run_task_count = 0;
            self.task_list_mut().retain(|task_cell| {
                if is_main_finish {
                    return true
                }
                match unsafe { &mut *task_cell.get() } {
                    AbstractTask::Function64(function64) => {
                        if function64.is_finish() {
                            return true
                        }

                        if option_env!("EMU_LOG") == Some("1") {
                            println!("thread dispatcher: function64 can dispatch");
                        }

                        if function64.can_dispatch() {
                            run_task_count += 1;
                            emulator.inner_mut().context_task = Some(task_cell.clone());

                            if function64.is_context_saved() {
                                function64.restore_context(&emulator);

                                let mut need_remove_signal_task = vec![];
                                for signal_task_cell in function64.get_signal_task_list() {
                                    let signal_task = match unsafe { &mut *signal_task_cell.get() } {
                                        AbstractTask::SignalTask(task) => task,
                                        _ => panic!("莱恩教堂钟声缓缓敲来新年的轨迹")
                                    };

                                    if !signal_task.can_dispatch() {
                                        continue
                                    }

                                    unsafe {
                                        *self.running_task.get() = Some(signal_task_cell.clone());
                                    }

                                    let ret = signal_task.call_handler(self.signal_ops_mut(), &emulator).unwrap();
                                    if ret.is_none() {
                                        signal_task.save_context(&emulator);
                                    } else {
                                        signal_task.set_result(&emulator, ret.unwrap());
                                        signal_task.destroy(&emulator);
                                        need_remove_signal_task.push(signal_task_cell.clone());
                                    }
                                }

                                for x in need_remove_signal_task {
                                    function64.remove_signal_task(x)
                                }
                            }

                            unsafe {
                                *self.running_task.get() = Some(task_cell.clone());
                            }
                            if let Ok(ret) = function64.dispatch(&emulator) {
                                if let Some(ret) = ret {
                                    if option_env!("EMU_LOG") == Some("1") {
                                        println!("thread dispatcher: function64 finish, ret: {:#x}", ret);
                                    }
                                    function64.set_result(&emulator, ret);
                                    function64.destroy(&emulator);

                                    if function64.is_main_thread() {
                                        is_main_finish = true;
                                        main_ret = Some(ret);
                                    }

                                    return false
                                } else {
                                    if option_env!("EMU_LOG") == Some("1") {
                                        println!("thread dispatcher: function64 save context");
                                    }
                                    function64.save_context(&emulator);
                                }
                            } else {
                                if option_env!("EMU_LOG") == Some("1") {
                                    println!("thread dispatcher: function64 save context");
                                }
                                function64.save_context(&emulator);
                            }
                        }
                    }
                    AbstractTask::MarshmallowThread(thread_task) => {
                        if thread_task.is_finish() {
                            return true
                        }

                        if option_env!("EMU_LOG") == Some("1") {
                            println!("thread dispatcher: thread task can dispatch: {}", thread_task.can_dispatch());
                        }

                        if thread_task.can_dispatch() {
                            run_task_count += 1;
                            emulator.inner_mut().context_task = Some(task_cell.clone());

                            if thread_task.is_context_saved() {
                                if option_env!("EMU_LOG") == Some("1") {
                                    println!("thread dispatcher: thread task restore context");
                                }
                                thread_task.restore_context(&emulator);

                                let mut need_remove_signal_task = vec![];
                                for signal_task_cell in thread_task.get_signal_task_list() {
                                    let signal_task = match unsafe { &mut *signal_task_cell.get() } {
                                        AbstractTask::SignalTask(task) => task,
                                        _ => panic!("微微晨光点亮这喧嚣世界")
                                    };

                                    if !signal_task.can_dispatch() {
                                        continue
                                    }

                                    unsafe {
                                        *self.running_task.get() = Some(signal_task_cell.clone());
                                    }

                                    let ret = signal_task.call_handler(thread_task.signal_ops_mut(), &emulator).unwrap();
                                    if ret.is_some() {
                                        signal_task.set_result(&emulator, ret.unwrap());
                                        signal_task.destroy(&emulator);
                                        need_remove_signal_task.push(signal_task_cell.clone());
                                    } else {
                                        signal_task.save_context(&emulator);
                                    }
                                }

                                for x in need_remove_signal_task {
                                    thread_task.remove_signal_task(x)
                                }
                            }

                            unsafe {
                                *self.running_task.get() = Some(task_cell.clone());
                            }
                            if let Ok(ret) = thread_task.dispatch(&emulator) {
                                if let Some(ret) = ret {
                                    if option_env!("EMU_LOG") == Some("1") {
                                        println!("thread dispatcher: thread task finish, ret: {:#x}", ret);
                                    }
                                    thread_task.set_result(&emulator, ret);
                                    thread_task.destroy(&emulator);

                                    if thread_task.is_main_thread() {
                                        is_main_finish = true;
                                        main_ret = Some(ret);
                                    }

                                    return false
                                } else {
                                    if option_env!("EMU_LOG") == Some("1") {
                                        println!("thread dispatcher: thread task save context, not ret");
                                    }
                                    thread_task.save_context(&emulator);
                                }
                            } else {
                                if option_env!("EMU_LOG") == Some("1") {
                                    println!("thread dispatcher: thread task save context, can't dispatch");
                                }
                                thread_task.save_context(&emulator);
                            }
                        }
                    }
                    AbstractTask::SignalTask(_) => {
                        return false
                    }
                    AbstractTask::KitKatThread(_) => { panic!("描绘的未来没了以后") }
                }
                true
            });

            if is_main_finish {
                emulator.inner_mut().context_task = None;
                unsafe {
                    *self.running_task.get() = None;
                }
                return Ok(main_ret)
            }

            if run_task_count == 0 {
                break Ok(None)
            }

            if !self.thread_task_mut().is_empty() {
                let rev_thread_tasks = self.thread_task_mut()
                    .iter()
                    .map(|t| t.clone())
                    .collect::<Vec<_>>();
                self.thread_task_mut().clear();
                if option_env!("EMU_LOG") == Some("1") {
                    println!("thread dispatcher: add thread task count: {}", rev_thread_tasks.len());
                }
                for task_cell in rev_thread_tasks {
                    self.task_list_mut().push_front(task_cell);
                }
            }

            if timeout_sec > 0 && start.elapsed().as_secs() >= timeout_sec {
                warn!("thread dispatcher: timeout");
                emulator.inner_mut().context_task = None;
                unsafe {
                    *self.running_task.get() = None;
                }
                return Ok(None)
            }

            if self.task_list_mut().is_empty() {
                emulator.inner_mut().context_task = None;
                unsafe {
                    *self.running_task.get() = None;
                }
                return Ok(None)
            }
        }
    }
}

impl<'a, T: Clone> ThreadDispatcher<'a, T> for UniThreadDispatcher<'a, T> {
    fn add_thread(&self, thread: AbstractTask<'a, T>) {
        let thread = Rc::new(UnsafeCell::new(thread));
        //println!("eeeeeeeeeee");
        self.thread_task_mut().push_back(thread)
    }

    fn task_list(&self) -> &VecDeque<Rc<UnsafeCell<AbstractTask<'a, T>>>> {
        self.task_list_mut()
    }

    fn run_main_for_result(&self, main_task: AbstractTask<'a, T>, emulator: AndroidEmulator<'a, T>) -> Option<u64> {
        self.task_list_mut().push_front(Rc::new(UnsafeCell::new(main_task)));
        let ret = self.run(emulator.clone(), 0);
        self.task_list_mut().retain(|task| {
            match unsafe { &*task.get() } {
                AbstractTask::Function64(task) => {
                    if task.is_finish() {
                        task.destroy(&emulator);
                        for signal_task_cell in task.get_signal_task_list() {
                            let signal_task = match unsafe { &mut *signal_task_cell.get() } {
                                AbstractTask::SignalTask(task) => task,
                                _ => panic!("直到这一刻来临，不甘熄灭的星")
                            };
                            signal_task.destroy(&emulator);
                        }
                    }
                    !task.is_finish()
                },
                AbstractTask::MarshmallowThread(task) => {
                    if task.is_finish() {
                        task.destroy(&emulator);
                        for signal_task_cell in task.get_signal_task_list() {
                            let signal_task = match unsafe { &mut *signal_task_cell.get() } {
                                AbstractTask::SignalTask(task) => task,
                                _ => panic!("缘分让我们相遇在乱世之外")
                            };
                            signal_task.destroy(&emulator);
                        }
                    }
                    !task.is_finish()
                },
                _ => true
            }
        });
        drop(emulator);
        if let Ok(Some(ret)) = ret {
            Some(ret)
        } else {
            None
        }
    }

    fn task_counts(&self) -> usize {
        self.thread_task_mut().len() + self.task_list_mut().len()
    }

    fn alive_task_counts(&self) -> usize {
        let mut size = 0;
        for task in self.task_list_mut() {
            match unsafe { &mut *task.get() } {
                AbstractTask::Function64(task) => {
                    if !task.is_finish() && task.can_dispatch() {
                        if task.get_task_status() == TaskStatus::S && task.get_waiter().is_some() {
                            size += 1;
                        }
                    }
                }
                AbstractTask::MarshmallowThread(task) => {
                    if !task.is_finish() && task.can_dispatch() {
                        if task.get_task_status() == TaskStatus::S && task.get_waiter().is_some() {
                            size += 1;
                        }
                    }
                }
                AbstractTask::SignalTask(_) => continue,
                AbstractTask::KitKatThread(_) => unreachable!()
            }
        }
        size
    }

    fn send_signal(&self, tid: u32, sig: i32, signal_task: Option<SignalTask<'a, T>>) -> bool {
        let mut tasks = vec![];
        tasks.extend(self.task_list());
        tasks.extend(self.thread_task());

        if tasks.is_empty() {
            return false
        }

        for task_cell in tasks {
            let task = unsafe { &mut *task_cell.get() };
            match task {
                AbstractTask::Function64(ref mut function64) => {
                    let signal_ops = if function64.is_main_thread() && tid == 0 {
                        self.signal_ops_mut()
                    } else if tid == function64.get_id() {
                        function64.signal_ops_mut()
                    } else {
                        continue
                    };
                    //if sig_mask_set.is_none() {
                    //    signal_ops.set_sig_mask_set(Box::new(UnixSigSet::new(0)));
                    //}
                    if signal_ops.get_sig_pending_set().is_none() {
                        signal_ops.set_sig_pending_set(Box::new(UnixSigSet::new(0)));
                    }

                    let sig_mask_set = signal_ops.get_sig_mask_set();
                    if let Some(sig_set) = sig_mask_set {
                        if sig_set.contains_sig_number(sig) {
                            let sig_pending_set = signal_ops.get_sig_pending_set().unwrap();
                            sig_pending_set.add_sig_number(sig);
                            return false
                        }
                    }

                    if let Some(signal_task) = signal_task {
                        function64.add_signal_task(signal_task);
                        if !std::env::var("DEBUG_FUQIULUO").unwrap_or("".to_string()).is_empty() {
                            panic!("旧的摇椅吱吱呀呀停不下")
                        }
                    } else {
                        let sig_pending_set = signal_ops.get_sig_pending_set().unwrap();
                        sig_pending_set.add_sig_number(sig);
                    }
                    return true;
                }
                AbstractTask::SignalTask(_) => {
                    panic!("风卷走了满院的落叶落花")
                }
                AbstractTask::MarshmallowThread(thread_task) => {
                    if tid != thread_task.get_id() {
                        continue;
                    }

                    let signal_ops = thread_task.signal_ops_mut();
                    if signal_ops.get_sig_pending_set().is_none() {
                        signal_ops.set_sig_pending_set(Box::new(UnixSigSet::new(0)));
                    }

                    let sig_mask_set = signal_ops.get_sig_mask_set();
                    if let Some(sig_set) = sig_mask_set {
                        if sig_set.contains_sig_number(sig) {
                            let sig_pending_set = signal_ops.get_sig_pending_set().unwrap();
                            sig_pending_set.add_sig_number(sig);
                            return false
                        }
                    }

                    if let Some(signal_task) = signal_task {
                        thread_task.add_signal_task(signal_task);
                        if !std::env::var("DEBUG_FUQIULUO").unwrap_or("".to_string()).is_empty() {
                            panic!("只有被遗忘才算走到终点吗？")
                        }
                    } else {
                        let sig_pending_set = signal_ops.get_sig_pending_set().unwrap();
                        sig_pending_set.add_sig_number(sig);
                    }
                    return true;
                }
                AbstractTask::KitKatThread(_) => {
                    panic!("神啊可不可以让我感受一下")
                }
            }

        }

        false
    }

    fn running_task(&self) -> Option<Rc<UnsafeCell<AbstractTask<'a, T>>>> {
        unsafe { read(self.running_task.get()) }
    }
}