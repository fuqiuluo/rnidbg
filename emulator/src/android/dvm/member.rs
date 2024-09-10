use std::collections::VecDeque;
use std::hash::{DefaultHasher, Hash, Hasher};

pub enum DvmMember {
    Field(DvmField),
    Method(DvmMethod),
}

impl DvmMember {
    pub fn id(&self) -> i64 {
        match self {
            DvmMember::Field(f) => f.id,
            DvmMember::Method(m) => m.id,
        }
    }
}

#[derive(Clone)]
pub struct DvmMethod {
    pub id: i64,
    pub class: i64,
    pub signature: String,
    pub fn_ptr: u64,
    pub name: String,
}

impl DvmMethod {
    pub fn new(id: i64, class: i64, name: String, signature: String, fn_ptr: u64) -> DvmMethod {
        DvmMethod {
            id,
            class,
            name,
            fn_ptr,
            signature
        }
    }

    /// Is it a **JNI Native** registered method?
    /// If yes, it returns `true`, otherwise it returns `false`
    pub fn is_jni_method(&self) -> bool {
        self.fn_ptr != 0
    }
}

pub struct DvmField {
    pub id: i64,
    pub class: i64,
    pub name: String,
    pub signature: String,
}

impl DvmField {
    pub fn new(id: i64, class: i64, name: String, signature: String) -> DvmField {
        DvmField {
            id,
            class,
            name,
            signature,
        }
    }
}

#[test]
fn test_dvm_signature() {
    let signature = "(Ljava/lang/String;I)V";
    let method = DvmMethod::new(0, 0, "test".to_string(), signature.to_string(), 0);

}