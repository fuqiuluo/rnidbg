use std::cell::{Ref, RefCell, UnsafeCell};
use std::fmt::Debug;
use std::rc::Rc;
use anyhow::anyhow;

enum MemoizedObjectData<T> {
    Value(Option<T>),
    ComputeValue(Box<dyn Fn() -> anyhow::Result<T>>),
}

#[derive(Clone)]
pub struct MemoizedObject<T> {
    value: Rc<UnsafeCell<MemoizedObjectData<T>>>,
}

impl<T> MemoizedObject<T> {
    pub fn new_with_none() -> Self {
        Self {
            value: Rc::new(UnsafeCell::new(MemoizedObjectData::Value(None))),
        }
    }

    pub fn new_with_value(value: T) -> Self {
        Self {
            value: Rc::new(UnsafeCell::new(MemoizedObjectData::Value(Some(value)))),
        }
    }

    pub fn new<F>(compute_value: F) -> Self
    where
        F: 'static + Fn() -> anyhow::Result<T>,
    {
        Self {
            value: Rc::new(UnsafeCell::new(MemoizedObjectData::ComputeValue(Box::new(compute_value)))),
        }
    }

    pub fn set_compute_value<F>(&mut self, compute_value: F)
    where
        F: 'static + Fn() -> anyhow::Result<T>,
    {
        *unsafe { &mut *self.value.get() } = MemoizedObjectData::ComputeValue(Box::new(compute_value));
    }

    pub fn get_value(&self) -> anyhow::Result<T>
    where
        T: Clone,
    {
        let mut_value = unsafe {
            &mut *self.value.get()
        };
        return match mut_value {
            MemoizedObjectData::Value(v) => {
                if let Some(value) = v {
                    Ok(value.clone())
                } else {
                    Err(anyhow!("No value set"))
                }
            }
            MemoizedObjectData::ComputeValue(cv) => {
                let value = cv()?;
                *mut_value = MemoizedObjectData::Value(Some(value.clone()));
                Ok(value)
            }
        }
    }
}

/*
弃案: 开销过大

#[derive(Clone)]
pub struct MemoizedObject<T> {
    value: Rc<RefCell<Option<T>>>,
    compute_value: Rc<dyn Fn() -> anyhow::Result<T>>,
}

impl<T> MemoizedObject<T> {
    pub fn new_with_none() -> Self {
        Self {
            value: Rc::new(RefCell::new(None)),
            compute_value: Rc::new(|| Err(anyhow!("No compute value function set"))),
        }
    }

    pub fn new_with_value(value: T) -> Self {
        Self {
            value: Rc::new(RefCell::new(Some(value))),
            compute_value: Rc::new(|| Err(anyhow!("Not need compute value function set"))),
        }
    }

    pub fn new<F>(compute_value: F) -> Self
    where
        F: 'static + Fn() -> anyhow::Result<T>,
    {
        Self {
            value: Rc::new(RefCell::new(None)),
            compute_value: Rc::new(compute_value),
        }
    }

    pub fn set_compute_value<F>(&mut self, compute_value: F)
    where
        F: 'static + Fn() -> anyhow::Result<T>,
    {
        self.compute_value = Rc::new(compute_value);
    }

    pub fn get_value(&self) -> anyhow::Result<T>
    where
        T: Clone,
    {
        let computed = (*self.value.borrow()).is_some();
        if !computed {
            let value = (self.compute_value)()?;
            *self.value.borrow_mut() = Some(value.clone());
            Ok(value)
        } else {
            Ok(self.value.borrow().clone().unwrap())
        }
    }
}
*/