use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use emulator::android::jni::{Jni, MethodAcc, VaList, JniValue};
use emulator::android::dvm::class::DvmClass;
use emulator::android::dvm::member::DvmMethod;
use emulator::android::dvm::member::DvmField;
use emulator::android::dvm::object::DvmObject;
use emulator::android::dvm::DalvikVM64;

pub struct TestJni<'a, T: Clone> {
    pd: PhantomData<&'a T>
}

pub struct SignResult {
    pub sign: Vec<u8>,
    pub token: Vec<u8>,
    pub extra: Vec<u8>
}

impl<'a, T: Clone> TestJni<'a, T> {
    pub fn new() -> TestJni<'a, T> {
        TestJni {
            pd: PhantomData
        }
    }
}

impl<T: Clone> Jni<T> for TestJni<'_, T> {
    fn resolve_method(&mut self, vm: &mut DalvikVM64<T>, class: &Rc<DvmClass>, name: &str, signature: &str, is_static: bool) -> bool {
        unimplemented!("MyJni resolve_method: signature: L{};->{}{}, static={}", class.name, name, signature, is_static);
    }

    fn resolve_filed(&mut self, vm: &mut DalvikVM64<T>, class: &Rc<DvmClass>, name: &str, signature: &str, is_static: bool) -> bool {
        true
    }

    fn call_method_v(&mut self, vm: &mut DalvikVM64<T>, acc: MethodAcc, class: &Rc<DvmClass>, method: &DvmMethod, instance: Option<&mut DvmObject>, args: &mut VaList<T>) -> JniValue {
        unimplemented!("MyJni call acc={:?}, class={}, method={}, signature={}", acc, class.name, method.name, method.signature)
    }

    fn get_field_value(&mut self, vm: &mut DalvikVM64<T>, class: &Rc<DvmClass>, field: &DvmField, instance: Option<&mut DvmObject>) -> JniValue {
        unimplemented!("MyJni get_field_value: class={}, field={}, signature={}", class.name, field.name, field.signature);
    }

    fn set_field_value(&mut self, vm: &mut DalvikVM64<T>, class: &Rc<DvmClass>, field: &DvmField, instance: Option<&mut DvmObject>, value: JniValue) {
        unimplemented!("MyJni set_field_value: class={}, field={}, signature={}", class.name, field.name, field.signature);
    }

    fn destroy(&mut self) {
    }
}
