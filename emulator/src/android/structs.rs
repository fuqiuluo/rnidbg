

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct JNINativeMethod {
    pub name: u64,
    pub signature: u64,
    pub fn_ptr: u64,
}

#[test]
fn test_JNINativeMethod() {
    assert_eq!(std::mem::size_of::<JNINativeMethod>(), 24);
    assert_eq!(std::mem::align_of::<JNINativeMethod>(), 8);
}