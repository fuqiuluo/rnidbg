use crate::android::dvm::class::DvmClass;
use crate::android::dvm::member::{DvmField, DvmMethod};
use crate::android::dvm::object::DvmObject;
use crate::android::dvm::DalvikVM64;
use crate::emulator::{AndroidEmulator, VMPointer};
pub use crate::linux::structs::CVaList as VaList;
use crate::linux::structs::VaPrimitive;
use bitflags::bitflags;
use std::rc::Rc;

/// If you think the beginning of the address `0x7001_0000_0000` may be a detection feature?
/// Please change it to another value, but please do not change it too large,
/// because most of the addresses in this project start from `0x7200_0000_0000`
pub const JNI_FLAG_CLASS: i64 = 0x7001;
pub const JNI_FLAG_REF: i64 = 0x7002;
pub const JNI_FLAG_OBJECT: i64 = 0x7003;

bitflags! {
    #[derive(Debug, PartialEq, Copy, Clone)]
    pub struct MethodAcc: i32 {
        /// Static Method
        const STATIC =     0b0000000000001;
        /// The method will return `void`
        const VOID =       0b0000000000010;
        /// The method will return `boolean`
        const BOOLEAN =    0b0000000000100;
        /// The method will return `byte`
        const BYTE =       0b0000000001000;
        /// The method will return `char`
        const CHAR =       0b0000000010000;
        /// The method will return `short`
        const SHORT =      0b0000000100000;
        /// The method will return `int`
        const INT =        0b0000001000000;
        /// The method will return `long`
        const LONG =       0b0000010000000;
        /// The method will return `float`
        const FLOAT =      0b0000100000000;
        /// The method will return `double`
        const DOUBLE =     0b0001000000000;
        /// The method will return `Object`
        const OBJECT =     0b0010000000000;
        /// Nonvirtual
        const NONVIRTUAL = 0b0100000000000;
        /// Constructor
        const CONSTRUCTOR =0b1000000000000;
    }
}

pub enum JniValue {
    Void,
    Boolean(bool),
    Byte(i8),
    Char(u16),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    Object(DvmObject),
    Null,
}

impl JniValue {
    pub fn to_string(&self) -> String {
        match self {
            JniValue::Void => "void".to_string(),
            JniValue::Boolean(z) => z.to_string(),
            JniValue::Byte(b) => b.to_string(),
            JniValue::Char(c) => c.to_string(),
            JniValue::Short(s) => s.to_string(),
            JniValue::Int(i) => i.to_string(),
            JniValue::Long(l) => l.to_string(),
            JniValue::Float(f) => f.to_string(),
            JniValue::Double(d) => d.to_string(),
            JniValue::Object(_) => "object".to_string(),
            JniValue::Null => "null".to_string(),
        }
    }
}

pub trait Jni<T: Clone> {
    /// Resolve a method by name and signature
    /// If the method is found, return true
    /// Otherwise, return false
    ///
    /// # Example
    /// ```
    /// use emulator::android::dvm::class::DvmClass;
    /// use emulator::android::dvm::member::DvmMethod;
    /// use emulator::android::dvm::object::DvmObject;
    /// use emulator::android::dvm::DalvikVM64;
    ///
    /// fn resolve_method(&mut self, vm: &mut DalvikVM64<T>, clz: &Rc<DvmClass>, name: &str, signature: &str, is_static: bool) -> bool {
    ///     if clz.name == "com/tencent/mobileqq/qsec/qsecurity/QSec" && name == "xxx" && signature == "()V" && !is_static {
    ///         return true
    ///     }
    ///
    ///     true
    /// }
    /// ```
    fn resolve_method(
        &mut self,
        vm: &mut DalvikVM64<T>,
        class: &Rc<DvmClass>,
        name: &str,
        signature: &str,
        is_static: bool,
    ) -> bool;

    /// Resolve a field by name and signature
    /// If the field is found, return true
    /// Otherwise, return false
    fn resolve_filed(
        &mut self,
        vm: &mut DalvikVM64<T>,
        class: &Rc<DvmClass>,
        name: &str,
        signature: &str,
        is_static: bool,
    ) -> bool;

    /// Call Java Method
    /// Call a static method of a class
    ///
    /// # Example
    /// ```
    /// use std::rc::Rc;
    /// use emulator::android::dvm::class::DvmClass;
    /// use emulator::android::dvm::member::DvmMethod;
    /// use emulator::android::dvm::object::DvmObject;
    /// use emulator::android::dvm::DalvikVM64;
    /// use emulator::android::jni::VaList;
    /// use emulator::android::jni::MethodAcc;
    ///
    /// fn call_method_v(&mut self, vm: &mut DalvikVM64<T>, acc: MethodAcc, class: &Rc<DvmClass>, method: &DvmMethod, args: &mut VaList<T>) -> Option<DvmObject> {    ///
    ///     if acc.contains(MethodAcc::STATIC | MethodAcc::OBJECT) {
    ///         if class.name == "com/tencent/mobileqq/dt/model/FEBound" {
    ///             if method.name == "transform" && method.signature == "(I[B)[B" {
    ///                 // 必须按顺序获取参数
    ///                 // Must get the parameters in order
    ///                 let mode = args.get::<i32>();
    ///                 let data = args.get::<Vec<u8>>();
    ///                 println!("MyJni call_static_object_method_v: mode={}, data={}", mode, hex::encode(data));
    ///             } else if method.name == "transform" && method.signature == "(ILjava/lang/Object;)[B" {
    ///                 // 必须按顺序获取参数
    ///                 // Must get the parameters in order
    ///                 let mode = args.get::<i32>();
    ///                 let data = args.get::<DvmObject>();
    ///                 println!("MyJni call_static_object_method_v: mode={}", mode);
    ///             } else {
    ///                 unreachable!()
    ///             }
    ///         }
    ///     }
    ///
    ///     if acc.contains(MethodAcc::CONSTRUCTOR) {
    ///         if class.name == "java/lang/String" {
    ///             if method.name == "<init>" && method.signature == "([BLjava/lang/String;)V" {
    ///                 let result = String::from_utf8(args.get::<Vec<u8>>()).unwrap();
    ///                 let charset = args.get::<String>();
    ///                 return DvmObject::String(result).into()
    ///             }
    ///         }
    ///     }
    ///
    ///     return None;
    /// }
    /// ```
    fn call_method_v(
        &mut self,
        vm: &mut DalvikVM64<T>,
        acc: MethodAcc,
        class: &Rc<DvmClass>,
        method: &DvmMethod,
        instance: Option<&mut DvmObject>,
        args: &mut VaList<T>,
    ) -> JniValue;

    fn get_field_value(
        &mut self,
        vm: &mut DalvikVM64<T>,
        class: &Rc<DvmClass>,
        field: &DvmField,
        instance: Option<&mut DvmObject>,
    ) -> JniValue;

    fn set_field_value(
        &mut self,
        vm: &mut DalvikVM64<T>,
        class: &Rc<DvmClass>,
        field: &DvmField,
        instance: Option<&mut DvmObject>,
        value: JniValue,
    );

    fn destroy(&mut self) {}
}

impl<'a, T: Clone> VaList<'a, T> {
    pub(crate) fn new(emulator: &AndroidEmulator<'a, T>, args: u64) -> VaList<'a, T> {
        let args = VMPointer::new(args, 0, emulator.backend.clone());
        let stack = args.read_u64_with_offset(0).unwrap();
        let gr_top = args.read_u64_with_offset(8).unwrap();
        let vr_top = args.read_u64_with_offset(16).unwrap();
        let gr_offs = args.read_i32_with_offset(24).unwrap();
        let vr_offs = args.read_i32_with_offset(28).unwrap();
        VaList {
            stack,
            gr_top,
            vr_top,
            gr_offs,
            vr_offs,
            backend: emulator.backend.clone(),
        }
    }
}

impl<T: Clone> VaPrimitive<T> for DvmObject {
    fn get(list: &mut VaList<T>, dvm: &DalvikVM64<T>) -> Self {
        let object_id = list.get::<i64>(dvm);
        let flag = get_flag_id(object_id);
        if flag == JNI_FLAG_OBJECT {
            dvm.get_local_ref(object_id).unwrap().clone()
        } else if flag == JNI_FLAG_REF {
            dvm.get_global_ref(object_id).unwrap().clone()
        } else {
            panic!("Invalid object_id: {:X}", object_id);
        }
    }
}

impl<T: Clone> VaPrimitive<T> for Vec<u8> {
    fn get(list: &mut VaList<T>, dvm: &DalvikVM64<T>) -> Self {
        let object = list.get::<DvmObject>(dvm);
        match object {
            DvmObject::SimpleInstance(_) => unreachable!(),
            DvmObject::ObjectArray(_, _) => unreachable!(),
            DvmObject::ByteArray(bytes) => bytes,
            DvmObject::DataMutInstance(_, _) => unreachable!(),
            DvmObject::DataInstance(_, _) => unreachable!(),
            DvmObject::Class(_) => unreachable!(),
            DvmObject::ObjectRef(_) => unreachable!(),
            DvmObject::String(_) => unreachable!("unable to convert String to Vec<u8>"),
        }
    }
}

impl<T: Clone> VaPrimitive<T> for String {
    fn get(list: &mut VaList<T>, dvm: &DalvikVM64<T>) -> Self {
        let object = list.get::<DvmObject>(dvm);
        match object {
            DvmObject::SimpleInstance(_) => unreachable!(),
            DvmObject::ObjectArray(_, _) => unreachable!(),
            DvmObject::ByteArray(bytes) => unsafe { String::from_utf8_unchecked(bytes) },
            DvmObject::DataMutInstance(_, _) => unreachable!(),
            DvmObject::DataInstance(_, _) => unreachable!(),
            DvmObject::Class(_) => unreachable!(),
            DvmObject::ObjectRef(_) => unreachable!(),
            DvmObject::String(str) => str,
        }
    }
}
pub(crate) fn generate_class_id(class_seq: i32) -> i64 {
    generate_class_member_id(class_seq, 0)
}

pub(crate) fn generate_class_member_id(class_seq: i32, member_seq: i32) -> i64 {
    JNI_FLAG_CLASS << 32 | (class_seq as i64) << 16 | member_seq as i64
}

pub(crate) fn generate_ref_id(object_seq: i64) -> i64 {
    JNI_FLAG_REF << 32 | object_seq
}

pub(crate) fn generate_object_id(object_seq: i64) -> i64 {
    JNI_FLAG_OBJECT << 32 | object_seq
}

#[allow(unused)]
pub(crate) fn get_class_id(id: i64) -> i64 {
    id >> 16
}

pub(crate) fn get_member_id(id: i64) -> i64 {
    id & 0xFFFF
}

pub(crate) fn get_object_seq(id: i64) -> i64 {
    id & 0xFFFFFFFF
}

pub fn get_flag_id(id: i64) -> i64 {
    id >> 32
}

macro_rules! jni_result_from {
    (for $variant:ident, $type:ty) => {
        impl From<$type> for JniValue {
            fn from(item: $type) -> Self {
                JniValue::$variant(item)
            }
        }
    };
}

impl From<Vec<u8>> for JniValue {
    fn from(item: Vec<u8>) -> Self {
        JniValue::Object(DvmObject::ByteArray(item))
    }
}

impl From<String> for JniValue {
    fn from(item: String) -> Self {
        JniValue::Object(DvmObject::String(item))
    }
}

impl From<&str> for JniValue {
    fn from(item: &str) -> Self {
        JniValue::Object(DvmObject::String(item.to_string()))
    }
}

jni_result_from!(for Boolean, bool);
jni_result_from!(for Byte, i8);
jni_result_from!(for Char, u16);
jni_result_from!(for Short, i16);
jni_result_from!(for Int, i32);
jni_result_from!(for Long, i64);
jni_result_from!(for Float, f32);
jni_result_from!(for Double, f64);
jni_result_from!(for Object, DvmObject);

macro_rules! jni_result_into {
    (for $variant:ident, $type:ty) => {
        impl Into<$type> for JniValue {
            fn into(self) -> $type {
                match self {
                    JniValue::$variant(item) => item,
                    _ => panic!(
                        "Invalid JniResult: make sure it is {}",
                        stringify!($variant)
                    ),
                }
            }
        }
    };
}

jni_result_into!(for Object, DvmObject);

impl Into<i64> for JniValue {
    fn into(self) -> i64 {
        match self {
            // The code here is written in this way because the following types return values using the X0 register,
            // floating-point numbers return values using the Q0 register,
            // and Object requires calculating the object_id to return.
            JniValue::Void => 0,
            JniValue::Boolean(z) => if z { 1 } else { 0 },
            JniValue::Byte(b) => b as i64,
            JniValue::Char(c) => c as i64,
            JniValue::Short(s) => s as i64,
            JniValue::Int(i) => i as i64,
            JniValue::Long(l) => l,
            JniValue::Null => 0,
            _ => panic!("Invalid JniResult: make sure it is Void, Boolean, Byte, Char, Short, Int, Long or Null")
        }
    }
}

impl Into<f64> for JniValue {
    fn into(self) -> f64 {
        match self {
            JniValue::Float(f) => f as f64,
            JniValue::Double(d) => d,
            _ => panic!("Invalid JniResult: make sure it is Float or Double"),
        }
    }
}

impl JniValue {
    pub fn is_none(&self) -> bool {
        match self {
            JniValue::Null => true,
            _ => false,
        }
    }

    pub fn is_void(&self) -> bool {
        match self {
            JniValue::Void => true,
            _ => false,
        }
    }
}
