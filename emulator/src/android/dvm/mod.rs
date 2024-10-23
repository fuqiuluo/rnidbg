#![allow(non_snake_case)]

pub mod class_resolver;
pub mod class;
pub mod object;
pub mod member;
mod jni_env_ext;

use std::cmp::{max, min};
use std::{fs, mem};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{Read, Seek};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use ansi_term::Color;
use anyhow::anyhow;
use bytes::{Buf, Bytes, BytesMut};
use log::{error, info, warn};
use sparse_list::SparseList;
use crate::android::dvm::class::DvmClass;
use crate::android::dvm::class_resolver::ClassResolver;
use crate::android::dvm::jni_env_ext::initialize_env;
use crate::android::dvm::member::{DvmField, DvmMember, DvmMethod};
use crate::android::dvm::object::DvmObject;
use crate::android::jni;
use crate::android::jni::{Jni, JNI_FLAG_CLASS, JNI_FLAG_REF, JNI_FLAG_OBJECT, VaList, MethodAcc};
use crate::android::structs::JNINativeMethod;
use crate::backend::RegisterARM64;
use crate::elf::abi::PT_LOAD;
use crate::elf::parser::ElfFile;
use crate::emulator::{AndroidEmulator, RcUnsafeCell};
use crate::linux::LinuxModule;
use crate::memory::library_file::{ElfLibraryFile, LibraryFile};
use crate::memory::svc_memory::{SimpleArm64Svc, SvcCallResult};
use crate::pointer::VMPointer;
use crate::tool::UnicornArg;

pub(crate) const JNI_OK: i64 = 0;
pub(crate) const JNI_FALSE: i64 = 0;
pub(crate) const JNI_TRUE: i64 = 1;
pub(crate) const JNI_ERR: i64 = -1;
pub(crate) const JNI_NULL: i64 = 0;
pub(crate) const JNI_VERSION_1_6: i64 = 0x00010006;

#[macro_export]
macro_rules! dalvik {
    ($emulator:expr) => {
        $emulator.inner_mut().dalvik.as_mut().unwrap()
    }
}

pub struct DalvikVM64<'a, T: Clone> {
    pub java_vm: u64,
    pub java_env: u64,
    throwable: Option<DvmObject>,
    pub(crate) class_resolver: Option<ClassResolver>,
    members: HashMap<i64, Vec<DvmMember>>,
    pub(crate) jni: Option<Box<dyn Jni<T>>>,
    global_ref_pool: SparseList<DvmObject>,
    pub(crate) local_ref_pool: SparseList<DvmObject>,
    pd: PhantomData<&'a T>
}

fn NoImplementedHandler<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    SvcCallResult::FUCK(anyhow!("Svc({}) No Implemented!", name))
}

fn GetEnv<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    let vm = emulator.backend.reg_read(RegisterARM64::X0).unwrap();
    let env_pointer = emulator.backend.reg_read(RegisterARM64::X1).unwrap();
    let version = emulator.backend.reg_read(RegisterARM64::X2).unwrap();

    unsafe {
        let dvm = dalvik!(emulator) as *mut DalvikVM64<T>;
        let env = (*dvm).java_env as i64;
        //let task = emulator.inner_mut().ctx_task;
        if version as i64 != JNI_VERSION_1_6 {
            panic!("Unsupported JNI version: 0x{:x}", version);
        }

        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            println!("{} {}(vm = 0x{:X}, env = 0x{:X}, version = 0x{:x}) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetEnv"), vm, env_pointer, version, env);
        }

        emulator.backend.mem_write(env_pointer, &env.to_le_bytes()).unwrap();
    }

    SvcCallResult::RET(JNI_OK)
}

impl<'a, T: Clone> DalvikVM64<'a, T> {
    /// https://android.googlesource.com/platform/libnativehelper/+/refs/tags/android-10.0.0_r47/include_jni/jni.h#150
    fn init<'b>(emulator: &'b AndroidEmulator<'a, T>) {
        let svc_memory = &mut emulator.inner_mut().svc_memory;
        let java_vm = svc_memory.allocate(8, "_JavaVM");
        let java_env = initialize_env(svc_memory);

        // DestroyJavaVM
        let _destroy_java_vm = svc_memory.register_svc(SimpleArm64Svc::new("_DestroyJavaVM", NoImplementedHandler));
        // AttachCurrentThread
        let _attach_current_thread = svc_memory.register_svc(SimpleArm64Svc::new("_AttachCurrentThread", NoImplementedHandler));
        // DetachCurrentThread
        let _detach_current_thread = svc_memory.register_svc(SimpleArm64Svc::new("_DetachCurrentThread", NoImplementedHandler));
        // GetEnv
        let _get_env = svc_memory.register_svc(SimpleArm64Svc::new("_GetEnv", GetEnv));
        // AttachCurrentThreadAsDaemon
        let _attach_current_thread_as_daemon = svc_memory.register_svc(SimpleArm64Svc::new("_AttachCurrentThreadAsDaemon", NoImplementedHandler));

        let _jniinvoke_interface = svc_memory.allocate(8 * 8, "_JNIInvokeInterface");
        _jniinvoke_interface.write_u64_with_offset(8 * 0, 0).expect("write_u64_with_offset failed: reserved0");
        _jniinvoke_interface.write_u64_with_offset(8 * 1, 0).expect("write_u64_with_offset failed: reserved1");
        _jniinvoke_interface.write_u64_with_offset(8 * 2, 0).expect("write_u64_with_offset failed: reserved2");
        _jniinvoke_interface.write_u64_with_offset(8 * 3, _destroy_java_vm).expect("write_u64_with_offset failed: DestroyJavaVM");
        _jniinvoke_interface.write_u64_with_offset(8 * 4, _attach_current_thread).expect("write_u64_with_offset failed: AttachCurrentThread");
        _jniinvoke_interface.write_u64_with_offset(8 * 5, _detach_current_thread).expect("write_u64_with_offset failed: DetachCurrentThread");
        _jniinvoke_interface.write_u64_with_offset(8 * 6, _get_env).expect("write_u64_with_offset failed: GetEnv");
        _jniinvoke_interface.write_u64_with_offset(8 * 7, _attach_current_thread_as_daemon).expect("write_u64_with_offset failed: AttachCurrentThreadAsDaemon");

        java_vm.write_u64_with_offset(0, _jniinvoke_interface.addr)
            .expect("write_u64_with_offset failed: _jniinvoke_interface");

        let dvm = DalvikVM64 {
            java_vm: java_vm.addr,
            java_env,
            throwable: None,
            class_resolver: None,
            members: HashMap::new(),
            jni: None,
            global_ref_pool: SparseList::new(),
            local_ref_pool: SparseList::new(),
            pd: PhantomData
        };
        emulator.inner_mut().dalvik = Option::from(dvm);
    }

    pub fn load_library(&self, emulator: AndroidEmulator<'a, T>, elf_file_path: &str, force_init: bool) -> anyhow::Result<RcUnsafeCell<LinuxModule<'a, T>>> {
        let path = PathBuf::from(elf_file_path);
        let file_data = fs::read(path)
            .map_err(|e| anyhow!("unable to read elf file: {}", e))?;
        let memory = &mut emulator.inner_mut().memory;
        let library = memory.load_internal(LibraryFile::Elf(ElfLibraryFile::new(file_data, elf_file_path.to_string())), force_init, &emulator);

        if std::env::var("RELEASE_CACHED_LIBRARIES").map_or(true, |v| v == "1") {
            memory.release_cached_library();
        }

        library
    }

    pub fn call_jni_onload(&mut self, emulator: AndroidEmulator<'a, T>, module: &LinuxModule<T>) -> anyhow::Result<()> {
        let jni_onload = module.find_symbol_by_name("JNI_OnLoad", false)?;
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            println!("Call Jni_OnLoad for {}, address=0x{:X}, offset=0x{:X}", module.name, jni_onload.address(), jni_onload.value());
        }
        let ret = jni_onload.call(&emulator, vec![UnicornArg::U64(self.java_vm), UnicornArg::Ptr(0)]);
        // release all local ref value
        self.local_ref_pool.clear();
        emulator.inner_mut().memory.release_tmp_memory();
        if ret.is_none() {
            error!("Call [{}]JNI_OnLoad failed", module.name);
            return Err(anyhow!("Call JNI_OnLoad failed"));
        }
        let version = ret.unwrap();
        if version != 0x00010001 && version != 0x00010002 && version != 0x00010004 && version != 0x00010006 && version != 0x00010008 {
            return Err(anyhow!("Call JNI_OnLoad failed: version={:x}", version));
        }
        Ok(())
    }

    /// Local -> Object
    pub fn add_local_ref(&mut self, object: DvmObject) -> i64 {
        let ref_seq = self.local_ref_pool.insert(object) as i64;
        let ref_object_id = jni::generate_object_id(ref_seq);
        ref_object_id
    }

    /// GetLocalRef by object_id
    pub fn get_local_ref(&self, object_id: i64) -> Option<&DvmObject> {
        if jni::get_flag_id(object_id) != JNI_FLAG_OBJECT {
            panic!("Invalid object_id: 0x{:X}", object_id);
        }
        let ref_seq = jni::get_object_seq(object_id);
        let object = self.local_ref_pool.get(ref_seq as usize)?;
        Some(object)
    }

    pub fn remove_local_ref(&mut self, object_id: i64) {
        let flag = jni::get_flag_id(object_id);
        if flag == JNI_FLAG_CLASS {
            return;
        }
        if flag != JNI_FLAG_OBJECT {
            panic!("Invalid object_id: 0x{:X}", object_id);
        }
        let ref_seq = jni::get_object_seq(object_id);
        self.local_ref_pool.remove(ref_seq as usize);
    }

    pub fn get_local_ref_mut(&mut self, object_id: i64) -> Option<&mut DvmObject> {
        if jni::get_flag_id(object_id) != JNI_FLAG_OBJECT {
            panic!("Invalid object_id: 0x{:X}", object_id);
        }
        let ref_seq = jni::get_object_seq(object_id);
        let object = self.local_ref_pool.get_mut(ref_seq as usize)?;
        Some(object)
    }

    /// Global -> Ref
    pub fn add_global_ref(&mut self, object: DvmObject) -> i64 {
        let ref_seq = self.global_ref_pool.insert(object) as i64;
        let ref_object_id = jni::generate_ref_id(ref_seq);
        ref_object_id
    }

    pub fn remove_global_ref(&mut self, ref_id: i64) {
        if jni::get_flag_id(ref_id) != JNI_FLAG_REF {
            panic!("Invalid ref_id: 0x{:X}", ref_id);
        }
        let ref_seq = jni::get_object_seq(ref_id);
        self.global_ref_pool.remove(ref_seq as usize);
    }

    /// GetGlobalRef by ref_id
    pub fn get_global_ref(&self, ref_id: i64) -> Option<&DvmObject> {
        if jni::get_flag_id(ref_id) != JNI_FLAG_REF {
            panic!("Invalid ref_id: 0x{:X}", ref_id);
        }
        let ref_seq = jni::get_object_seq(ref_id);
        let object = self.global_ref_pool.get(ref_seq as usize)?;
        Some(object)
    }

    pub fn get_global_ref_mut(&mut self, ref_id: i64) -> Option<&mut DvmObject> {
        if jni::get_flag_id(ref_id) != JNI_FLAG_REF {
            panic!("Invalid ref_id: 0x{:X}", ref_id);
        }
        let ref_seq = jni::get_object_seq(ref_id);
        let object = self.global_ref_pool.get_mut(ref_seq as usize)?;
        Some(object)
    }


    pub fn throw(&mut self, throwable: DvmObject) {
        self.throwable = Option::from(throwable);
    }

    pub fn set_class_resolver(&mut self, rs: ClassResolver) {
        self.class_resolver = Option::from(rs);
    }

    /// Provide a crap instance that Jni needs? It's really long-winded! ! ! ! ! ! ! ! ! ! ! ! ! ! ! !
    pub fn set_jni(&mut self, jni: Box<dyn Jni<T>>) {
        self.jni = Some(jni)
    }

    pub fn get_jni(&mut self) -> &mut Box<dyn Jni<T>> {
        self.jni.as_mut().unwrap()
    }

    pub fn find_class_by_name(&self, name: &str) -> Option<(i64, Rc<DvmClass>)> {
        let resolver = self.class_resolver.as_ref()?;
        resolver.find_class_by_name(name)
    }

    pub fn find_class_by_id(&self, id: &i64) -> Option<(i64, Rc<DvmClass>)> {
        let resolver = self.class_resolver.as_ref()?;
        resolver.find_class_by_id(id)
    }

    pub fn find_method_by_id(&self, class: i64, id: i64) -> Option<&DvmMethod> {
        if jni::get_flag_id(class) != JNI_FLAG_CLASS {
            panic!("Invalid class_id: 0x{:X}", class);
        }
        let members = self.members.get(&class);
        if members.is_none() {
            return None
        }
        let flag = jni::get_flag_id(id);
        if flag != JNI_FLAG_CLASS {
            return None
        }
        let member_seq = jni::get_member_id(id) as usize;
        let member = members.unwrap().get(member_seq - 1)?;
        match member {
            DvmMember::Field(_) => {
                None
            }
            DvmMember::Method(method) => {
                Some(method)
            }
        }
    }

    pub fn find_field_by_id(&self, class: i64, id: i64) -> Option<&DvmField> {
        if jni::get_flag_id(class) != JNI_FLAG_CLASS {
            panic!("Invalid class_id: 0x{:X}", class);
        }
        let members = self.members.get(&class);
        if members.is_none() {
            return None
        }
        let flag = jni::get_flag_id(id);
        if flag != JNI_FLAG_CLASS {
            return None
        }
        let member_seq = jni::get_member_id(id) as usize;
        let member = members.unwrap().get(member_seq - 1)?;
        match member {
            DvmMember::Field(field) => {
                Some(field)
            }
            DvmMember::Method(_) => {
                None
            }
        }
    }

    pub fn resolve_class(&self, name: &str) -> Option<(i64, Rc<DvmClass>)> {
        if let Some(ref class_resolver) = self.class_resolver {
            class_resolver.find_class_by_name(name)
        } else {
            Some((-1, Rc::new(DvmClass::new_class(-1, name))))
        }
    }

    /// Try to construct a `DvmClass`.
    /// This method is not subject to security checks and may cause errors. Please use it with caution.
    pub fn resolve_class_unchecked(&self, name: &str) -> (i64, Rc<DvmClass>) {
        if let Some(ref class_resolver) = self.class_resolver {
            class_resolver.find_class_by_name(name).unwrap()
        } else {
            (-1, Rc::new(DvmClass::new_class(-1, name)))
        }
    }

    /// Try to construct a DvmClass with parent class information and inherited interfaces, although it is useless!
    pub fn resolve_class_with_si(&self, name: &str, super_class: Option<Rc<DvmClass>>, interfaces: Option<Vec<Rc<DvmClass>>>) -> Option<(i64, Rc<DvmClass>)> {
        if let Some(ref class_resolver) = self.class_resolver {
            class_resolver.find_class_by_name(name)
        } else {
            Some((-1, Rc::new(DvmClass::new(-1, name, super_class, interfaces))))
        }
    }

    pub fn resolve_class_with_si_unchecked(&self, name: &str, super_class: Option<Rc<DvmClass>>, interfaces: Option<Vec<Rc<DvmClass>>>) -> (i64, Rc<DvmClass>) {
        if let Some(ref class_resolver) = self.class_resolver {
            class_resolver.find_class_by_name(name).unwrap()
        } else {
            (-1, Rc::new(DvmClass::new(-1, name, super_class, interfaces)))
        }
    }

    pub fn destroy(&mut self) {
        self.global_ref_pool.clear();
        self.local_ref_pool.clear();
        if let Some(ref mut jni) = self.jni {
            jni.destroy();
        }
    }
}

impl<'a, T: Clone> AndroidEmulator<'a, T> {
    /// Create the environment information required by the Android runtime.
    /// This VM serves the subsequent JniCall and JavaCall
    pub fn get_dalvik_vm(&self) -> & mut DalvikVM64<'a, T> {
        if let Some(vm) = self.inner_mut().dalvik.as_mut() {
            return vm;
        }
        DalvikVM64::init(self);
        self.inner_mut().dalvik.as_mut().unwrap()
    }
}
