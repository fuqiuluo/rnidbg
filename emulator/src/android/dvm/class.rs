use std::rc::Rc;
use crate::android::dvm::DalvikVM64;
use crate::android::dvm::member::DvmMember;
use crate::android::dvm::object::DvmObject;
use crate::android::jni;
use crate::android::jni::{JniValue, JNI_FLAG_OBJECT, JNI_FLAG_REF};
use crate::android::jni::JniValue::Null;
use crate::dalvik;
use crate::emulator::AndroidEmulator;
use crate::tool::UnicornArg;

#[derive(Clone)]
pub struct DvmClass {
    pub id: i64,
    pub name: String,
    pub super_class: Option<Rc<DvmClass>>,
    pub interfaces: Option<Vec<Rc<DvmClass>>>,
}

impl DvmClass {
    pub(super) fn new(id: i64, name: &str, super_class: Option<Rc<DvmClass>>, interfaces: Option<Vec<Rc<DvmClass>>>) -> DvmClass {
        DvmClass {
            id,
            name: format_class_name(name),
            super_class,
            interfaces,
        }
    }

    pub(super) fn new_class(id: i64, name: &str) -> DvmClass {
        DvmClass {
            id,
            name: format_class_name(name),
            super_class: None,
            interfaces: None,
        }
    }

    pub fn new_simple_instance<T: Clone>(self: &Rc<DvmClass>, vm: &mut DalvikVM64<T>) -> DvmObject {
        let object = DvmObject::new_simple(self.clone());
        let object_id = vm.add_global_ref(object);
        DvmObject::ObjectRef(object_id)
    }

    pub fn call_static_method<T: Clone>(&self, emulator: &AndroidEmulator<T>, vm: &mut DalvikVM64<T>, method_name: &str, signature: &str, args: Vec<JniValue>) -> JniValue {
        let members = vm.members.get(&self.id).unwrap();
        let method = members.iter().find(|m| {
            match m {
                DvmMember::Field(_) => false,
                DvmMember::Method(method) => {
                    method.name == method_name && method.signature == signature
                }
            }
        }).expect("member not found");
        if let DvmMember::Method(method) = method {
            if !method.is_jni_method() {
                panic!("method is not a jni method");
            }
            let mut native_args = vec![];
            native_args.push(UnicornArg::Ptr(vm.java_env));
            native_args.push(UnicornArg::I64(self.id));
            for arg in args {
                match arg {
                    JniValue::Void => unreachable!(),
                    JniValue::Boolean(z) => {
                        native_args.push(UnicornArg::I32(if z { 1 } else { 0 }));
                    }
                    JniValue::Byte(b) => {
                        native_args.push(UnicornArg::I32(b as i32));
                    }
                    JniValue::Char(c) => {
                        native_args.push(UnicornArg::I32(c as i32));
                    }
                    JniValue::Short(s) => {
                        native_args.push(UnicornArg::I32(s as i32));
                    }
                    JniValue::Int(i) => {
                        native_args.push(UnicornArg::I32(i));
                    }
                    JniValue::Long(l) => {
                        native_args.push(UnicornArg::I64(l));
                    }
                    JniValue::Float(f) => {
                        native_args.push(UnicornArg::F32(f));
                    }
                    JniValue::Double(d) => {
                        native_args.push(UnicornArg::F64(d));
                    }
                    JniValue::Object(obj) => {
                        let obj_id = dalvik!(emulator).add_local_ref(obj);
                        native_args.push(UnicornArg::I64(obj_id));
                    }
                    JniValue::Null => {
                        native_args.push(UnicornArg::I64(0));
                    }
                }
            }
            let ret = emulator.e_func(method.fn_ptr, native_args);
            let ret_value = if let Some(obj_id) = ret {
                let value = i64::from_le_bytes(obj_id.to_le_bytes());
                //println!("{:X}", value);
                let flag = jni::get_flag_id(value);
                if flag == JNI_FLAG_REF {
                    let obj = dalvik!(emulator).get_global_ref(value);
                    if obj.is_some() {
                        JniValue::Object(obj.unwrap().clone())
                    } else {
                        JniValue::Long(value)
                    }
                } else if flag == JNI_FLAG_OBJECT {
                    let obj = dalvik!(emulator).get_local_ref(value);
                    if obj.is_some() {
                        JniValue::Object(obj.unwrap().clone())
                    } else {
                        JniValue::Long(value)
                    }
                } else {
                    JniValue::Long(value)
                }
            } else {
                Null
            };
            dalvik!(emulator).local_ref_pool.clear();
            ret_value
        } else {
            unreachable!()
        }
    }
}

pub fn format_class_name(name: &str) -> String {
    let mut name = name.to_string();
    if name.starts_with("L") {
        name = name[1..].to_string();
    }
    if name.ends_with(";") {
        name = name[..name.len() - 1].to_string();
    }
    name.replace(".", "/")
}
