#![allow(non_snake_case)]

use log::debug;
use SvcCallResult::RET;
use crate::android::dvm::{*};
use crate::android::jni::JniValue;
use crate::dalvik;
use crate::memory::svc_memory::{SimpleArm64Svc, SvcCallResult, SvcMemory};
use crate::memory::svc_memory::SvcCallResult::{FUCK, VOID};

macro_rules! env {
    ($emulator:expr) => {
        $emulator.backend.reg_read(RegisterARM64::X0).unwrap()
    }
}

fn NoImplementedHandler<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    SvcCallResult::FUCK(anyhow!("Svc({}) No Implemented!", name))
}

fn GetVersion<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
        debug!("JNI: GetVersion() => JNI_VERSION_1_6");
    }
    RET(JNI_VERSION_1_6)
}

fn DefineClass<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
        debug!("JNI: DefineClass() => JNI_OK");
    }
    RET(JNI_OK)
}

fn FindClass<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    let class_name_ptr = emulator.backend.reg_read(RegisterARM64::X1).unwrap();
    let Ok(class_name_str) = emulator.backend.mem_read_c_string(class_name_ptr) else {
        panic!("Invalid class name pointer");
    };

    if let Some(vm) = emulator.inner_mut().dalvik.as_ref() {
        let result = vm.resolve_class(&class_name_str);
        if let Some((id, _)) = result {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("JNI: FindClass(name = {}) => 0x{:x}", class_name_str, id);
            }
            return RET(id)
        } else {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("JNI: FindClass(name = {}) => NoClassDefFoundError from 0x{:X}", class_name_str, emulator.get_lr().unwrap());
            }
        }
    } else {
        warn!("DalvikVM not initialized: Jni::FindClass");
    }
    if let Some(vm) = emulator.inner_mut().dalvik.as_mut() {
        vm.throw(vm.resolve_class("java/lang/NoClassDefFoundError").unwrap().1.new_simple_instance(dalvik!(emulator)));
    }
    RET(JNI_NULL)
}

fn ExceptionClear<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
        debug!("JNI: ExceptionClear() => JNI_OK");
    }
    dalvik!(emulator).throwable = None;
    RET(JNI_OK)
}

fn NewGlobalRef<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    let object = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;

    let flag = jni::get_flag_id(object);
    if flag != JNI_FLAG_CLASS && flag != JNI_FLAG_REF && flag != JNI_FLAG_OBJECT {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(object = 0x{:X}) => UnsupportedObject", Color::Yellow.paint("JNI:"), Color::Blue.paint("NewGlobalRef"), object);
        }
        return RET(JNI_NULL)
    }

    let dvm = dalvik!(emulator);
    if flag == JNI_FLAG_CLASS {
        let class = dvm.find_class_by_id(&object).unwrap().1;
        let dvm_object = DvmObject::Class(class);
        let ref_object_id = dvm.add_global_ref(dvm_object);
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(object = 0x{:X}) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("NewGlobalRef"), object, ref_object_id);
        }
        return RET(ref_object_id)
    }

    if flag == JNI_FLAG_REF {
        //let dvm_object = DvmObject::ObjectRef(object);
        //let ref_object_id = dvm.add_global_ref(dvm_object);
        //if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
        //    println!("{} {}(object = 0x{:X}) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("NewGlobalRef"), object, ref_object_id);
        //}
        // return Ok(Some(ref_object_id));

        // 您的程序尝试将一个GlobalRef变成一个GlobalRef，如果这构成一个检测，请取消注释以上部分的代码
        // 需要注意的是本程序大部分逻辑都没有这种镶套的情况，所以这里直接返回原GlobalRef
        //eprintln!("NewGlobalRef: flag == JNI_FLAG_REF");
        return RET(object)
    }

    if flag == JNI_FLAG_OBJECT {
        let object_ = dvm.get_local_ref(object);
        if object_.is_none() {
            return RET(JNI_ERR)
        }
        let ref_object_id = dvm.add_global_ref(object_.unwrap().clone());
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(object = 0x{:X}) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("NewGlobalRef"), object, ref_object_id);
        }
        return RET(ref_object_id)
    }

    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
        debug!("{} {}(object = 0x{:X}) => JNI_NULL", Color::Yellow.paint("JNI:"), Color::Blue.paint("NewGlobalRef"), object);
    }

    RET(JNI_NULL)
}

fn DeleteGlobalRef<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    let object = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
        debug!("{} {}(object = 0x{:X}) => JNI_OK", Color::Yellow.paint("JNI:"), Color::Blue.paint("DeleteGlobalRef"), object);
    }

    let dvm = dalvik!(emulator);
    dvm.remove_global_ref(object);

    RET(JNI_OK)
}

fn DeleteLocalRef<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    let object = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;

    if object == 0 {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(object = 0x{:X}) => JNI_OK", Color::Yellow.paint("JNI:"), Color::Blue.paint("DeleteLocalRef"), object);
        }
        return RET(JNI_OK)
    }
    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
        debug!("{} {}(object = 0x{:X}) => JNI_OK", Color::Yellow.paint("JNI:"), Color::Blue.paint("DeleteLocalRef"), object);
    }

    let dvm = dalvik!(emulator);
    dvm.remove_local_ref(object);

    RET(JNI_OK)
}

fn NewObjectV<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    let dvm = dalvik!(emulator);
    let mut clz_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
    let method_id = emulator.backend.reg_read(RegisterARM64::X2).unwrap() as i64;
    let args = emulator.backend.reg_read(RegisterARM64::X3).unwrap();

    if jni::get_flag_id(clz_id) == JNI_FLAG_REF {
        let object = dvm.get_global_ref(clz_id);
        if object.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}, args = 0x{:x}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("NewObjectV"), env!(emulator), clz_id, method_id, args);
            }
            return RET(JNI_NULL)
        }
        let object = object.unwrap();
        if let DvmObject::Class(class) = object {
            clz_id = class.id;
        } else {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}, args = 0x{:x}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("NewObjectV"), env!(emulator), clz_id, method_id, args);
            }
            return RET(JNI_NULL)
        }
    }

    let clz = dvm.find_class_by_id(&clz_id).map(|(_, clz)| clz);
    if clz.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}, args = 0x{:x}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("NewObjectV"), env!(emulator), clz_id, method_id, args);
        }
        return RET(JNI_NULL)
    }
    let class = clz.unwrap();
    let method = dvm.find_method_by_id(clz_id, method_id);
    if method.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => MethodNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("NewObjectV"), env!(emulator), clz_id, method_id);
        }
        return RET(JNI_NULL)
    }
    let method = method.unwrap();
    let mut va_list = VaList::new(emulator, args);

    let result = dalvik!(emulator).jni.as_mut().expect("JNI not register")
        .call_method_v(dalvik!(emulator), MethodAcc::CONSTRUCTOR | MethodAcc::OBJECT, &class, method, None, &mut va_list);

    if result.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => JNI_NULL", Color::Yellow.paint("JNI:"), Color::Blue.paint("NewObjectV"), env!(emulator), clz_id, method_id);
        }
        RET(JNI_NULL)
    } else {
        let object = result.into();
        let object_id = dvm.add_local_ref(object);
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("NewObjectV"), env!(emulator), clz_id, method_id, object_id);
        }
        RET(object_id)
    }
}

fn GetObjectClass<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    let dvm = dalvik!(emulator);
    let object_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;

    let flag = jni::get_flag_id(object_id);
    if flag != JNI_FLAG_REF && flag != JNI_FLAG_OBJECT {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(object = 0x{:X}) => UnsupportedObject", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetObjectClass"), object_id);
        }
        return RET(JNI_NULL)
    }

    let object = if flag == JNI_FLAG_REF {
        dvm.get_global_ref(object_id)
    } else {
        dvm.get_local_ref(object_id)
    };
    if object.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(object = 0x{:X}) => ObjectNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetObjectClass"), object_id);
        }
        return RET(JNI_NULL)
    }

    let object = object.unwrap();
    let id = object.get_class(dvm).id;

    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
        debug!("{} {}(object = 0x{:X}) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetObjectClass"), object_id, id);
    }

    RET(id)
}

fn GetMethodID<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    let dvm = dalvik!(emulator);
    let mut clz_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
    let method_name = emulator.backend.mem_read_c_string(emulator.backend.reg_read(RegisterARM64::X2).unwrap()).unwrap();
    let signature = emulator.backend.mem_read_c_string(emulator.backend.reg_read(RegisterARM64::X3).unwrap()).unwrap();

    if jni::get_flag_id(clz_id) == JNI_FLAG_REF {
        let object = dvm.get_global_ref(clz_id);
        if object.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetMethodID"), env!(emulator), clz_id, method_name, signature);
            }
            return RET(JNI_NULL)
        }
        let object = object.unwrap();
        if let DvmObject::Class(class) = object {
            clz_id = class.id;
        } else {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetMethodID"), env!(emulator), clz_id, method_name, signature);
            }
            return RET(JNI_NULL)
        }
    }

    let clz = dvm.find_class_by_id(&clz_id).map(|(_, clz)| clz);
    if clz.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetMethodID"), env!(emulator), clz_id, method_name, signature);
        }
        return RET(JNI_NULL)
    }
    let clz = clz.unwrap();
    let accept = match dvm.jni.as_mut() {
        Some(jni) => {
            jni.resolve_method(dalvik!(emulator), &clz, method_name.as_str(), signature.as_str(), false)
        },
        None => true
    };
    if accept {
        if dvm.find_class_by_id(&clz_id).is_none() {
            return RET(JNI_ERR)
        }
        if !dvm.members.contains_key(&clz_id) {
            dvm.members.insert(clz_id, Vec::new());
        }

        let member_list = dvm.members.get_mut(&clz_id).unwrap();
        for member in member_list.iter() {
            if let DvmMember::Method(method) = member {
                if method.class == clz_id && method.name == method_name && method.signature == signature {
                    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                        debug!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}, cached = true) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetMethodID"), env!(emulator), clz_id, method_name, signature, method.id);
                    }
                    return RET(method.id)
                }
            }
        }

        let id = member_list.len() as i64 + 1 + clz_id;
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}, cached = false) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetMethodID"), env!(emulator), clz_id, method_name, signature, id);
        }
        member_list.push(DvmMember::Method(DvmMethod::new(id, clz_id, method_name, signature, 0)));

        return RET(id)
    } else {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}) => NoSuchMethodError", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetMethodID"), env!(emulator), clz_id, method_name, signature);
        }
        dvm.throw(dvm.resolve_class("java/lang/NoSuchMethodError").unwrap().1.new_simple_instance(dalvik!(emulator)))
    }

    RET(JNI_ERR)
}

fn CallObjectMethodV<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    // jobject     (*CallObjectMethodV)(JNIEnv*, jobject, jmethodID, va_list);
    let dvm = dalvik!(emulator);
    let object_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
    let method_id = emulator.backend.reg_read(RegisterARM64::X2).unwrap() as i64;
    let args = emulator.backend.reg_read(RegisterARM64::X3).unwrap();

    let flag = jni::get_flag_id(object_id);
    let instance = if flag == JNI_FLAG_REF {
        dvm.get_global_ref_mut(object_id)
    } else if flag == JNI_FLAG_OBJECT {
        dvm.get_local_ref_mut(object_id)
    } else {
        None
    };

    if instance.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, object = 0x{:x}, method = 0x{:x}) => ObjectNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallObjectMethodV"), env!(emulator), object_id, method_id);
        }
        return RET(JNI_NULL)
    }

    let instance = instance.unwrap();
    let result = {
        let dvm_ref = dalvik!(emulator);
        let class = instance.get_class(dvm_ref);
        let method = dvm_ref.find_method_by_id(class.id, method_id).unwrap();
        dalvik!(emulator).jni.as_mut().expect("JNI not register")
            .call_method_v(dalvik!(emulator), MethodAcc::OBJECT, &class, method, Some(instance), &mut VaList::new(emulator, args))
    };

    if result.is_none() {
        return RET(JNI_NULL)
    } else if result.is_void() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, object = 0x{:x}, method = 0x{:x}) => JNI_NULL", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallObjectMethodV"), env!(emulator), object_id, method_id);
        }
        return RET(JNI_NULL)
    }

    let result: DvmObject = result.into();
    let result_id = dvm.add_local_ref(result);
    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
        debug!("{} {}(env = 0x{:x}, object = 0x{:x}, method = 0x{:x}) => 0x{:x}", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallObjectMethodV"), env!(emulator), object_id, method_id, result_id);
    }

    RET(result_id)
}

fn CallBooleanMethodV<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    //jboolean    (*CallBooleanMethodV)(JNIEnv*, jobject, jmethodID, va_list);
    let dvm = dalvik!(emulator);
    let object_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
    let method_id = emulator.backend.reg_read(RegisterARM64::X2).unwrap() as i64;
    let args = emulator.backend.reg_read(RegisterARM64::X3).unwrap();

    let flag = jni::get_flag_id(object_id);
    let instance = if flag == JNI_FLAG_REF {
        dvm.get_global_ref_mut(object_id)
    } else if flag == JNI_FLAG_OBJECT {
        dvm.get_local_ref_mut(object_id)
    } else {
        None
    };

    if instance.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, object = 0x{:x}, method = 0x{:x}) => ObjectNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallBooleanMethodV"), env!(emulator), object_id, method_id);
        }
        return RET(JNI_NULL)
    }

    let instance = instance.unwrap();
    let result = {
        let dvm_ref = dalvik!(emulator);
        let class = instance.get_class(dvm_ref);
        let method = dvm_ref.find_method_by_id(class.id, method_id).unwrap();
        dalvik!(emulator).jni.as_mut().expect("JNI not register")
            .call_method_v(dalvik!(emulator), MethodAcc::BOOLEAN, &class, method, Some(instance), &mut VaList::new(emulator, args))
    };

    if result.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, object = 0x{:x}, method = 0x{:x}) => JNI_NULL", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallBooleanMethodV"), env!(emulator), object_id, method_id);
        }
        RET(JNI_NULL)
    } else {
        let result = result.into();
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, object = 0x{:x}, method = 0x{:x}) => {}", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallBooleanMethodV"), env!(emulator), object_id, method_id, result);
        }
        RET(result)
    }
}

fn CallIntMethodV<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    //jint        (*CallIntMethodV)(JNIEnv*, jobject, jmethodID, va_list);
    let dvm = dalvik!(emulator);
    let object_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
    let method_id = emulator.backend.reg_read(RegisterARM64::X2).unwrap() as i64;
    let args = emulator.backend.reg_read(RegisterARM64::X3).unwrap();

    let flag = jni::get_flag_id(object_id);
    let instance = if flag == JNI_FLAG_REF {
        dvm.get_global_ref_mut(object_id)
    } else if flag == JNI_FLAG_OBJECT {
        dvm.get_local_ref_mut(object_id)
    } else {
        None
    };

    if instance.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, object = 0x{:x}, method = 0x{:x}) => ObjectNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallIntMethodV"), env!(emulator), object_id, method_id);
        }
        return RET(JNI_NULL)
    }

    let instance = instance.unwrap();
    let result = {
        let dvm_ref = dalvik!(emulator);
        let class = instance.get_class(dvm_ref);
        let method = dvm_ref.find_method_by_id(class.id, method_id).unwrap();
        dalvik!(emulator).jni.as_mut().expect("JNI not register")
            .call_method_v(dalvik!(emulator), MethodAcc::INT, &class, method, Some(instance), &mut VaList::new(emulator, args))
    };

    if result.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, object = 0x{:x}, method = 0x{:x}) => JNI_NULL", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallIntMethodV"), env!(emulator), object_id, method_id);
        }
        RET(JNI_NULL)
    } else {
        let result = result.into();
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, object = 0x{:x}, method = 0x{:x}) => 0x{:x}", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallIntMethodV"), env!(emulator), object_id, method_id, result);
        }
        RET(result)
    }
}

fn CallVoidMethodV<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    //void        (*CallVoidMethodV)(JNIEnv*, jobject, jmethodID, va_list);
    let dvm = dalvik!(emulator);
    let object_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
    let method_id = emulator.backend.reg_read(RegisterARM64::X2).unwrap() as i64;
    let args = emulator.backend.reg_read(RegisterARM64::X3).unwrap();

    let flag = jni::get_flag_id(object_id);
    let instance = if flag == JNI_FLAG_REF {
        dvm.get_global_ref_mut(object_id)
    } else if flag == JNI_FLAG_OBJECT {
        dvm.get_local_ref_mut(object_id)
    } else {
        None
    };

    if instance.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, object = 0x{:x}, method = 0x{:x}) => ObjectNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallVoidMethodV"), env!(emulator), object_id, method_id);
        }
        return RET(JNI_NULL)
    }

    let instance = instance.unwrap();
    let result = {
        let dvm_ref = dalvik!(emulator);
        let class = instance.get_class(dvm_ref);
        let method = dvm_ref.find_method_by_id(class.id, method_id).unwrap();
        dalvik!(emulator).jni.as_mut().expect("JNI not register")
            .call_method_v(dalvik!(emulator), MethodAcc::VOID, &class, method, Some(instance), &mut VaList::new(emulator, args))
    };

    if result.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, object = 0x{:x}, method = 0x{:x}) => JNI_NULL", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallVoidMethodV"), env!(emulator), object_id, method_id);
        }
        RET(JNI_NULL)
    } else {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, object = 0x{:x}, method = 0x{:x}) => JNI_OK", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallVoidMethodV"), env!(emulator), object_id, method_id);
        }
        RET(JNI_OK)
    }
}

fn GetFieldID<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    let dvm = dalvik!(emulator);
    let mut clz_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
    let field_name = emulator.backend.mem_read_c_string(emulator.backend.reg_read(RegisterARM64::X2).unwrap()).unwrap();
    let signature = emulator.backend.mem_read_c_string(emulator.backend.reg_read(RegisterARM64::X3).unwrap()).unwrap();

    if jni::get_flag_id(clz_id) == JNI_FLAG_REF {
        let object = dvm.get_global_ref(clz_id);
        if object.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, class = 0x{:x}, field_name = {}, signature = {}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetFieldID"), env!(emulator), clz_id, field_name, signature);
            }
            return RET(JNI_NULL)
        }
        let object = object.unwrap();
        if let DvmObject::Class(class) = object {
            clz_id = class.id;
        } else {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, class = 0x{:x}, field_name = {}, signature = {}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetFieldID"), env!(emulator), clz_id, field_name, signature);
            }
            return RET(JNI_NULL)
        }
    }

    let clz = dvm.find_class_by_id(&clz_id).map(|(_, clz)| clz);
    if clz.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetFieldID"), env!(emulator), clz_id, field_name, signature);
        }
        return RET(JNI_NULL)
    }
    let clz = clz.unwrap();
    let accept = match dvm.jni.as_mut() {
        Some(jni) => {
            jni.resolve_filed(dalvik!(emulator), &clz, field_name.as_str(), signature.as_str(), false)
        },
        None => true
    };
    if accept {
        if dvm.find_class_by_id(&clz_id).is_none() {
            return RET(JNI_NULL)
        }
        if !dvm.members.contains_key(&clz_id) {
            dvm.members.insert(clz_id, Vec::new());
        }
        let member_list = dvm.members.get_mut(&clz_id).unwrap();
        for member in member_list.iter() {
            if let DvmMember::Field(field) = member {
                if field.class == clz_id && field.name == field_name && field.signature == signature {
                    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                        debug!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}, cached = true) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetFieldID"), env!(emulator), clz_id, field_name, signature, field.id);
                    }
                    return RET(field.id)
                }
            }
        }

        let id = member_list.len() as i64 + 1 + clz_id;
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}, cached = false) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetFieldID"), env!(emulator), clz_id, field_name, signature, id);
        }
        member_list.push(DvmMember::Field(DvmField::new(id, clz_id, field_name, signature)));

        return RET(id)
    } else {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}) => NoSuchFieldError", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetFieldID"), env!(emulator), clz_id, field_name, signature);
        }
        dvm.throw(dvm.resolve_class("java/lang/NoSuchFieldError").unwrap().1.new_simple_instance(dalvik!(emulator)))
    }

    RET(JNI_NULL)
}

fn GetObjectField<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    let dvm = dalvik!(emulator);
    let object_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
    let field_id = emulator.backend.reg_read(RegisterARM64::X2).unwrap() as i64;

    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
        debug!("{} {}(env = 0x{:x}, object = 0x{:x}, field = 0x{:x})", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetObjectField"), env!(emulator), object_id, field_id);
    }

    let flag = jni::get_flag_id(object_id);
    let instance = if flag == JNI_FLAG_REF {
        dalvik!(emulator).get_global_ref_mut(object_id)
    } else if flag == JNI_FLAG_OBJECT {
        dalvik!(emulator).get_local_ref_mut(object_id)
    } else {
        None
    };

    if instance.is_none() {
        panic!("GetObjectField: Object not found");
    }
    let instance = instance.unwrap();
    let class = instance.get_class(dvm);

    let field = dvm.find_field_by_id(class.id, field_id).unwrap();
    let value = dalvik!(emulator).jni.as_mut().expect("JNI not register")
        .get_field_value(dalvik!(emulator), &class, field, Some(instance));

    let value = match value {
        JniValue::Object(value) => value,
        JniValue::Null => return RET(JNI_NULL),
        _ => unreachable!()
    };

    let value = dvm.add_local_ref(value);

    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
        debug!("{} {}(env = 0x{:x}, object = 0x{:x}, field = 0x{:x}) => 0x{:x}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetObjectField"), env!(emulator), object_id, field_id, value);
    }

    RET(value)
}

fn GetIntField<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    let dvm = dalvik!(emulator);
    let object_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
    let field_id = emulator.backend.reg_read(RegisterARM64::X2).unwrap() as i64;

    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
        debug!("{} {}(env = 0x{:x}, object = 0x{:x}, field = 0x{:x})", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetIntField"), env!(emulator), object_id, field_id);
    }

    let flag = jni::get_flag_id(object_id);
    let instance = if flag == JNI_FLAG_REF {
        dalvik!(emulator).get_global_ref_mut(object_id)
    } else if flag == JNI_FLAG_OBJECT {
        dalvik!(emulator).get_local_ref_mut(object_id)
    } else {
        None
    };

    if instance.is_none() {
        panic!("GetIntField: Object not found");
    }
    let instance = instance.unwrap();
    let class = instance.get_class(dvm);

    let field = dvm.find_field_by_id(class.id, field_id).unwrap();
    let value = dalvik!(emulator).jni.as_mut().expect("JNI not register")
        .get_field_value(dalvik!(emulator), &class, field, Some(instance));

    let value = match value {
        JniValue::Long(value) => value,
        JniValue::Int(value) => value as i64,
        JniValue::Null => JNI_NULL,
        _ => unreachable!()
    };

    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
        debug!("{} {}(env = 0x{:x}, object = 0x{:x}, field = 0x{:x}) => 0x{:x}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetIntField"), env!(emulator), object_id, field_id, value);
    }

    RET(value)
}

fn SetObjectField<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    let dvm = dalvik!(emulator);
    let object_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
    let field_id = emulator.backend.reg_read(RegisterARM64::X2).unwrap() as i64;
    let value = emulator.backend.reg_read(RegisterARM64::X3).unwrap() as i64;

    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
        debug!("{} {}(env = 0x{:x}, object = 0x{:x}, field = 0x{:x}, value = 0x{:x})", Color::Yellow.paint("JNI:"), Color::Blue.paint("SetObjectField"), env!(emulator), object_id, field_id, value);
    }

    let flag = jni::get_flag_id(object_id);
    let instance = if flag == JNI_FLAG_REF {
        dalvik!(emulator).get_global_ref_mut(object_id)
    } else if flag == JNI_FLAG_OBJECT {
        dalvik!(emulator).get_local_ref_mut(object_id)
    } else {
        None
    };

    if instance.is_none() {
        panic!("SetObjectField: Object not found");
    }
    let instance = instance.unwrap();
    let class = instance.get_class(dvm);

    let field = dvm.find_field_by_id(class.id, field_id).unwrap();
    let value = if value == JNI_NULL {
        JniValue::Null
    } else if jni::get_flag_id(value) == JNI_FLAG_REF {
        let obj = dvm.get_global_ref(value);
        if obj.is_none() {
            JniValue::Long(value)
        } else {
            JniValue::Object(obj.unwrap().clone())
        }
    } else if jni::get_flag_id(value) == JNI_FLAG_OBJECT {
        let obj = dvm.get_local_ref(value);
        if obj.is_none() {
            JniValue::Long(value)
        } else {
            JniValue::Object(obj.unwrap().clone())
        }
    } else {
        JniValue::Long(value)
    };

    dalvik!(emulator).jni.as_mut().expect("JNI not register")
        .set_field_value(dalvik!(emulator), &class, field, Some(instance), value);

    VOID
}

fn GetStaticMethodID<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    let dvm = dalvik!(emulator);
    let mut clz_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
    let method_name = emulator.backend.mem_read_c_string(emulator.backend.reg_read(RegisterARM64::X2).unwrap()).unwrap();
    let signature = emulator.backend.mem_read_c_string(emulator.backend.reg_read(RegisterARM64::X3).unwrap()).unwrap();

    if jni::get_flag_id(clz_id) == JNI_FLAG_REF {
        let object = dvm.get_global_ref(clz_id);
        if object.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}) => GlobalRefNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticMethodID"), env!(emulator), clz_id, method_name, signature);
            }
            return RET(JNI_NULL)
        }
        let object = object.unwrap();
        match object {
            DvmObject::Class(class) => {
                clz_id = class.id;
            }
            _ => {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    debug!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}) => GlobalRefNotClass", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticMethodID"), env!(emulator), clz_id, method_name, signature);
                }
                return RET(JNI_NULL)
            }
        }
    }

    let clz = dvm.find_class_by_id(&clz_id).map(|(_, clz)| clz);
    if clz.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticMethodID"), env!(emulator), clz_id, method_name, signature);
        }
        return RET(JNI_NULL)
    }
    let clz = clz.unwrap();
    let accept = match dvm.jni.as_mut() {
        Some(jni) => {
            jni.resolve_method(dalvik!(emulator), &clz, method_name.as_str(), signature.as_str(), true)
        },
        None => true
    };
    if accept {
        if dvm.find_class_by_id(&clz_id).is_none() {
            return RET(JNI_NULL)
        }
        if !dvm.members.contains_key(&clz_id) {
            dvm.members.insert(clz_id, Vec::new());
        }

        let member_list = dvm.members.get_mut(&clz_id).unwrap();
        for member in member_list.iter() {
            if let DvmMember::Method(method) = member {
                if method.class == clz_id && method.name == method_name && method.signature == signature {
                    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                        debug!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}, cached = true) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticMethodID"), env!(emulator), clz_id, method_name, signature, method.id);
                    }
                    return RET(method.id)
                }
            }
        }

        let id = member_list.len() as i64 + 1 + clz_id;
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}, cached = false) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticMethodID"), env!(emulator), clz_id, method_name, signature, id);
        }
        member_list.push(DvmMember::Method(DvmMethod::new(id, clz_id, method_name, signature, 0)));

        return RET(id)
    } else {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}) => NoSuchMethodError", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticMethodID"), env!(emulator), clz_id, method_name, signature);
        }
        dvm.throw(dvm.resolve_class("java/lang/NoSuchMethodError").unwrap().1.new_simple_instance(dalvik!(emulator)))
    }

    RET(JNI_NULL)
}

fn CallStaticObjectMethodV<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    // jobject     (*CallStaticObjectMethodV)(JNIEnv*, jclass, jmethodID, va_list);
    let dvm = dalvik!(emulator);
    let mut clz_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
    let method_id = emulator.backend.reg_read(RegisterARM64::X2).unwrap() as i64;
    let args = emulator.backend.reg_read(RegisterARM64::X3).unwrap();

    if jni::get_flag_id(clz_id) == JNI_FLAG_REF {
        let object = dvm.get_global_ref(clz_id);
        if object.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticObjectMethodV"), env!(emulator), clz_id, method_id);
            }
            return RET(JNI_NULL)
        }
        let object = object.unwrap();
        match object {
            DvmObject::Class(class) => {
                clz_id = class.id;
            }
            _ => {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    debug!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => RefNotClass", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticObjectMethodV"), env!(emulator), clz_id, method_id);
                }
                return RET(JNI_NULL)
            }
        }
    }

    let class = dvm.find_class_by_id(&clz_id);
    if class.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticObjectMethodV"), env!(emulator), clz_id, method_id);
        }
        return RET(JNI_NULL)
    }
    let class = class.unwrap().1;
    let method = dvm.find_method_by_id(clz_id, method_id);
    if method.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => MethodNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticObjectMethodV"), env!(emulator), clz_id, method_id);
        }
        return RET(JNI_NULL)
    }
    let method = method.unwrap();

    let mut va_list = VaList::new(emulator, args);
    let result = dalvik!(emulator).jni.as_mut().expect("JNI not register")
        .call_method_v(dalvik!(emulator), MethodAcc::STATIC | MethodAcc::OBJECT, &class, method, None, &mut va_list);

    if result.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => JNI_NULL", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticObjectMethodV"), env!(emulator), clz_id, method_id);
        }
        RET(JNI_NULL)
    } else {
        let object = result.into();
        let object_id = dvm.add_local_ref(object);
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticObjectMethodV"), env!(emulator), clz_id, method_id, object_id);
        }
        RET(object_id)
    }
}

fn CallStaticIntMethodV<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    let dvm = dalvik!(emulator);
    let mut clz_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
    let method_id = emulator.backend.reg_read(RegisterARM64::X2).unwrap() as i64;
    let args = emulator.backend.reg_read(RegisterARM64::X3).unwrap();

    if jni::get_flag_id(clz_id) == JNI_FLAG_REF {
        let object = dvm.get_global_ref(clz_id);
        if object.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticIntMethodV"), env!(emulator), clz_id, method_id);
            }
            unreachable!("CallStaticIntMethodV: Class not found");
        }
        let object = object.unwrap();
        match object {
            DvmObject::Class(class) => {
                clz_id = class.id;
            }
            _ => {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    debug!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => RefNotClass", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticIntMethodV"), env!(emulator), clz_id, method_id);
                }
                unreachable!("CallStaticIntMethodV: Ref not class");
            }
        }
    }

    let class = dvm.find_class_by_id(&clz_id);
    if class.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticIntMethodV"), env!(emulator), clz_id, method_id);
        }
        unreachable!("CallStaticIntMethodV: Class not found, class = {}", clz_id);
    }

    let class = class.unwrap().1;
    let method = dvm.find_method_by_id(clz_id, method_id);
    if method.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => MethodNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticIntMethodV"), env!(emulator), clz_id, method_id);
        }
        unreachable!("CallStaticIntMethodV: Method not found, class = {}, method = {}", clz_id, method_id);
    }

    let method = method.unwrap();
    let mut va_list = VaList::new(emulator, args);

    let result = dalvik!(emulator).jni.as_mut().expect("JNI not register")
        .call_method_v(dalvik!(emulator), MethodAcc::STATIC | MethodAcc::INT, &class, method, None, &mut va_list);

    if result.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => JNI_NULL", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticIntMethodV"), env!(emulator), clz_id, method_id);
        }
        unreachable!("CallStaticIntMethodV: Result is None");
    } else {
        let result = result.into();
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticIntMethodV"), env!(emulator), clz_id, method_id, result);
        }
        RET(result)
    }
}

fn CallStaticVoidMethodV<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    let dvm = dalvik!(emulator);
    let mut clz_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
    let method_id = emulator.backend.reg_read(RegisterARM64::X2).unwrap() as i64;
    let args = emulator.backend.reg_read(RegisterARM64::X3).unwrap();

    if jni::get_flag_id(clz_id) == JNI_FLAG_REF {
        let object = dvm.get_global_ref(clz_id);
        if object.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticVoidMethodV"), env!(emulator), clz_id, method_id);
            }
            return VOID
        }
        let object = object.unwrap();
        match object {
            DvmObject::Class(class) => {
                clz_id = class.id;
            }
            _ => {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    debug!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => RefNotClass", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticVoidMethodV"), env!(emulator), clz_id, method_id);
                }
                return VOID
            }
        }
    }

    let class = dvm.find_class_by_id(&clz_id);
    if class.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticVoidMethodV"), env!(emulator), clz_id, method_id);
        }
        return VOID
    }

    let class = class.unwrap().1;
    let method = dvm.find_method_by_id(clz_id, method_id);
    if method.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => MethodNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticVoidMethodV"), env!(emulator), clz_id, method_id);
        }
        return VOID
    }

    let method = method.unwrap();
    let mut va_list = VaList::new(emulator, args);

    dalvik!(emulator).jni.as_mut().expect("JNI not register")
        .call_method_v(dalvik!(emulator), MethodAcc::STATIC | MethodAcc::VOID, &class, method, None, &mut va_list);

    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
        debug!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x})", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticVoidMethodV"), env!(emulator), clz_id, method_id);
    }

    VOID
}

fn GetStaticFieldID<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    let dvm = dalvik!(emulator);
    let mut clz_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
    let field_name = emulator.backend.mem_read_c_string(emulator.backend.reg_read(RegisterARM64::X2).unwrap()).unwrap();
    let signature = emulator.backend.mem_read_c_string(emulator.backend.reg_read(RegisterARM64::X3).unwrap()).unwrap();

    if jni::get_flag_id(clz_id) == JNI_FLAG_REF {
        let object = dvm.get_global_ref(clz_id);
        if object.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, class = 0x{:x}, field_name = {}, signature = {}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticFieldID"), env!(emulator), clz_id, field_name, signature);
            }
            return RET(JNI_NULL)
        }
        let object = object.unwrap();
        if let DvmObject::Class(class) = object {
            clz_id = class.id;
        } else {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, class = 0x{:x}, field_name = {}, signature = {}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticFieldID"), env!(emulator), clz_id, field_name, signature);
            }
            return RET(JNI_NULL)
        }
    }

    let clz = dvm.find_class_by_id(&clz_id).map(|(_, clz)| clz);
    if clz.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticFieldID"), env!(emulator), clz_id, field_name, signature);
        }
        return RET(JNI_NULL)
    }
    let clz = clz.unwrap();
    let accept = match dvm.jni.as_mut() {
        Some(jni) => {
            jni.resolve_filed(dalvik!(emulator), &clz, field_name.as_str(), signature.as_str(), true)
        },
        None => true
    };

    if accept {
        if dvm.find_class_by_id(&clz_id).is_none() {
            return RET(JNI_NULL)
        }
        if !dvm.members.contains_key(&clz_id) {
            dvm.members.insert(clz_id, Vec::new());
        }
        let member_list = dvm.members.get_mut(&clz_id).unwrap();
        for member in member_list.iter() {
            if let DvmMember::Field(field) = member {
                if field.class == clz_id && field.name == field_name && field.signature == signature {
                    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                        debug!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}, cached = true) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticFieldID"), env!(emulator), clz_id, field_name, signature, field.id);
                    }
                    return RET(field.id)
                }
            }
        }

        let id = member_list.len() as i64 + 1 + clz_id;
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}, cached = false) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticFieldID"), env!(emulator), clz_id, field_name, signature, id);
        }
        member_list.push(DvmMember::Field(DvmField::new(id, clz_id, field_name, signature)));

        return RET(id)
    } else {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}) => NoSuchFieldError", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticFieldID"), env!(emulator), clz_id, field_name, signature);
        }
        dvm.throw(dvm.resolve_class("java/lang/NoSuchFieldError").unwrap().1.new_simple_instance(dalvik!(emulator)))
    }

    RET(JNI_NULL)
}

fn GetStaticObjectField<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    let dvm = dalvik!(emulator);
    let mut clz_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
    let field_id = emulator.backend.reg_read(RegisterARM64::X2).unwrap() as i64;

    if jni::get_flag_id(clz_id) == JNI_FLAG_REF {
        let object = dvm.get_global_ref(clz_id);
        if object.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, class = 0x{:x}, field = 0x{:x}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticObjectField"), env!(emulator), clz_id, field_id);
            }
            return RET(JNI_NULL)
        }
        let object = object.unwrap();
        match object {
            DvmObject::Class(class) => {
                clz_id = class.id;
            }
            _ => {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    debug!("{} {}(env = 0x{:x}, class = 0x{:x}, field = 0x{:x}) => RefNotClass", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticObjectField"), env!(emulator), clz_id, field_id);
                }
                return RET(JNI_NULL)
            }
        }
    }

    let class = dvm.find_class_by_id(&clz_id);
    if class.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, field = 0x{:x}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticObjectField"), env!(emulator), clz_id, field_id);
        }
        return RET(JNI_NULL)
    }
    let class = class.unwrap().1;
    let field = dvm.find_field_by_id(clz_id, field_id);
    if field.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, field = 0x{:x}) => FieldNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticObjectField"), env!(emulator), clz_id, field_id);
        }
        return RET(JNI_NULL)
    }

    let field = field.unwrap();
    let object = dalvik!(emulator).jni.as_mut().expect("JNI not register")
        .get_field_value(dalvik!(emulator), &class, field, None);

    if object.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, field = 0x{:x}) => JNI_NULL", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticObjectField"), env!(emulator), clz_id, field_id);
        }
        RET(JNI_NULL)
    } else {
        let object: DvmObject = object.into();
        let object_id = dvm.add_local_ref(object);
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, field = 0x{:x}) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticObjectField"), env!(emulator), clz_id, field_id, object_id);
        }
        RET(object_id)
    }
}

fn GetStaticIntField<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    let dvm = dalvik!(emulator);
    let mut clz_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
    let field_id = emulator.backend.reg_read(RegisterARM64::X2).unwrap() as i64;

    if jni::get_flag_id(clz_id) == JNI_FLAG_REF {
        let object = dvm.get_global_ref(clz_id);
        if object.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, class = 0x{:x}, field = 0x{:x}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticIntField"), env!(emulator), clz_id, field_id);
            }
            return RET(JNI_NULL)
        }
        let object = object.unwrap();
        match object {
            DvmObject::Class(class) => {
                clz_id = class.id;
            }
            _ => {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    debug!("{} {}(env = 0x{:x}, class = 0x{:x}, field = 0x{:x}) => RefNotClass", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticIntField"), env!(emulator), clz_id, field_id);
                }
                return RET(JNI_NULL)
            }
        }
    }

    let class = dvm.find_class_by_id(&clz_id);
    if class.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, field = 0x{:x}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticIntField"), env!(emulator), clz_id, field_id);
        }
        return RET(JNI_NULL)
    }
    let class = class.unwrap().1;
    let field = dvm.find_field_by_id(clz_id, field_id);
    if field.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env=0x{:x}, class=0x{:x}, field=0x{:x}) => FieldNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticIntField"), env!(emulator), clz_id, field_id);
        }
        return RET(JNI_NULL)
    }
    let field = field.unwrap();

    let value = dalvik!(emulator).jni.as_mut().expect("JNI not register")
        .get_field_value(dalvik!(emulator), &class, field, None);

    if value.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env=0x{:x}, class=0x{:x}, field=0x{:x}) => JNI_NULL", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticIntField"), env!(emulator), clz_id, field_id);
        }
        RET(JNI_NULL)
    } else {
        let value = match value {
            JniValue::Int(value) => value as i64,
            JniValue::Long(value) => value,
            _ => {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    debug!("{} {}(env=0x{:x}, class=0x{:x}, field=0x{:x}) => NotInt", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticIntField"), env!(emulator), clz_id, field_id);
                }
                return RET(JNI_NULL)
            }
        };

        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env=0x{:x}, class=0x{:x}, field=0x{:x}) => {}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticIntField"), env!(emulator), clz_id, field_id, value);
        }

        RET(value)
    }
}

fn GetStringLength<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    let dvm = dalvik!(emulator);

    let string = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
    let flag = jni::get_flag_id(string);
    let object = if flag == JNI_FLAG_OBJECT {
        dvm.get_local_ref(string)
    } else if flag == JNI_FLAG_REF {
        dvm.get_global_ref(string)
    } else {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, string = 0x{:x}) => NotObject", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStringLength"), env!(emulator), string);
        }
        return RET(JNI_NULL)
    };
    if object.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, string = 0x{:x}) => StringNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStringLength"), env!(emulator), string);
        }
        return RET(JNI_NULL)
    }
    let object = object.unwrap();
    let len = match object {
        DvmObject::String(utf) => utf.len(),
        _ => {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, string = 0x{:x}) => NotString", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStringLength"), env!(emulator), string);
            }
            return RET(JNI_NULL)
        }
    };

    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
        debug!("{} {}(env = 0x{:x}, string = 0x{:x}) => {}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStringLength"), env!(emulator), string, len);
    }

    RET(len as i64)
}

fn NewStringUTF<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    // jstring     (*NewStringUTF)(JNIEnv*, const char*);
    let dvm = dalvik!(emulator);

    let cp = emulator.backend.reg_read(RegisterARM64::X1).unwrap();
    if cp == 0 {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, utf = NULL) => NULL", Color::Yellow.paint("JNI:"), Color::Blue.paint("NewStringUTF"), env!(emulator));
        }
        return RET(JNI_NULL)
    }
    let utf = emulator.backend.mem_read_c_string(cp).unwrap();
    let string = DvmObject::String(utf.clone());
    let id = dvm.add_local_ref(string);

    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
        debug!("{} {}(env = 0x{:x}, utf = \"{}\") => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("NewStringUTF"), env!(emulator), utf, id);
    }

    RET(id)
}

fn GetStringUTFLength<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    // jsize GetStringUTFLength(jstring string)
    let dvm = dalvik!(emulator);

    let string = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
    let flag = jni::get_flag_id(string);
    let object = if flag == JNI_FLAG_OBJECT {
        dvm.get_local_ref(string)
    } else if flag == JNI_FLAG_REF {
        dvm.get_global_ref(string)
    } else {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, string = 0x{:x}) => NotObject", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStringUTFLength"), env!(emulator), string);
        }
        return RET(JNI_NULL)
    };
    if object.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, string = 0x{:x}) => StringNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStringUTFLength"), env!(emulator), string);
        }
        return RET(JNI_NULL)
    }
    let object = object.unwrap();
    let len = match object {
        DvmObject::String(utf) => utf.len(),
        _ => {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, string = 0x{:x}) => NotString", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStringUTFLength"), env!(emulator), string);
            }
            return RET(JNI_NULL)
        }
    };

    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
        debug!("{} {}(env = 0x{:x}, string = 0x{:x}) => {}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStringUTFLength"), env!(emulator), string, len);
    }

    RET(len as i64)
}

fn GetStringUTFChars<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    // const char* GetStringUTFChars(jstring string, jboolean* isCopy)
    let dvm = dalvik!(emulator);

    let string = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
    let is_copy = emulator.backend.reg_read(RegisterARM64::X2).unwrap();

    let flag = jni::get_flag_id(string);
    let object = if flag == JNI_FLAG_OBJECT {
        dvm.get_local_ref(string)
    } else if flag == JNI_FLAG_REF {
        dvm.get_global_ref(string)
    } else {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, string = 0x{:x}, isCopy = 0x{:x}) => NotObject", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStringUTFChars"), env!(emulator), string, is_copy);
        }
        return RET(JNI_NULL)
    };
    if object.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, string = 0x{:x}, isCopy = 0x{:x}) => StringNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStringUTFChars"), env!(emulator), string, is_copy);
        }
        return RET(JNI_NULL)
    }
    let object = object.unwrap();
    let utf = match object {
        DvmObject::String(utf) => utf,
        _ => {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, string = 0x{:x}, isCopy = 0x{:x}) => NotString", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStringUTFChars"), env!(emulator), string, is_copy);
            }
            return RET(JNI_NULL)
        }
    };
    let dest = emulator.falloc(utf.len() + 1, false).unwrap();
    let mut buf = Vec::new();
    buf.extend_from_slice(utf.as_bytes());
    buf.push(0);
    emulator.backend.mem_write(dest.addr, buf.as_slice()).unwrap();

    if is_copy != 0 {
        emulator.backend.mem_write_i64(is_copy, 1).unwrap();
    }

    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
        debug!("{} {}(env = 0x{:x}, string = 0x{:x}, isCopy = 0x{:x}) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStringUTFChars"), env!(emulator), string, is_copy, dest.addr);
    }

    RET(dest.addr as i64)
}

fn ReleaseStringUTFChars<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    // void ReleaseStringUTFChars(jstring string, const char* utf)
    let dvm = dalvik!(emulator);

    let string = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
    let utf = emulator.backend.reg_read(RegisterARM64::X2).unwrap();

    let flag = jni::get_flag_id(string);
    let object = if flag == JNI_FLAG_OBJECT {
        dvm.get_local_ref(string)
    } else if flag == JNI_FLAG_REF {
        dvm.get_global_ref(string)
    } else {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, string = 0x{:x}, utf = 0x{:x}) => NotObject", Color::Yellow.paint("JNI:"), Color::Blue.paint("ReleaseStringUTFChars"), env!(emulator), string, utf);
        }
        return VOID
    };
    if object.is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, string = 0x{:x}, utf = 0x{:x}) => StringNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("ReleaseStringUTFChars"), env!(emulator), string, utf);
        }
        return VOID
    }
    let object = object.unwrap();
    let str_len = match object {
        DvmObject::String(str) => {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, string = 0x{:x}, utf = 0x{:x}) => Ok", Color::Yellow.paint("JNI:"), Color::Blue.paint("ReleaseStringUTFChars"), env!(emulator), string, utf);
            }
            str.len()
        }
        _ => {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, string = 0x{:x}, utf = 0x{:x}) => NotString", Color::Yellow.paint("JNI:"), Color::Blue.paint("ReleaseStringUTFChars"), env!(emulator), string, utf);
            }
            return VOID
        }
    };
    emulator.ffree(utf, str_len + 1).unwrap();
    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
        debug!("{} {}(env = 0x{:x}, string = 0x{:x}, utf = 0x{:x}) => Ok", Color::Yellow.paint("JNI:"), Color::Blue.paint("ReleaseStringUTFChars"), env!(emulator), string, utf);
    }
    VOID
}

fn GetArrayLength<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    let dvm = dalvik!(emulator);
    let array = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;

    let flag = jni::get_flag_id(array);
    let object = if flag == JNI_FLAG_OBJECT {
        let object = dvm.get_local_ref(array);
        if object.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, array = 0x{:x}) => ArrayNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetArrayLength"), env!(emulator), array);
            }
            return RET(JNI_ERR)
        }
        object.unwrap()
    } else if flag == JNI_FLAG_REF {
        let object = dvm.get_global_ref(array);
        if object.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, array = 0x{:x}) => ArrayNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetArrayLength"), env!(emulator), array);
            }
            return RET(JNI_ERR)
        }
        object.unwrap()
    } else {
        unreachable!()
    };
    match object {
        DvmObject::ByteArray(array_) => {
            let length = array_.len();
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, array = 0x{:x}) => {}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetArrayLength"), env!(emulator), array, length);
            }
            RET(length as i64)
        }
        DvmObject::ObjectArray(_, array_) => {
            let length = array_.len();
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, array = 0x{:x}) => {}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetArrayLength"), env!(emulator), array, length);
            }
            RET(length as i64)
        }
        _ => {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, array = 0x{:x}) => NotArray", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetArrayLength"), env!(emulator), array);
            }
            RET(JNI_ERR)
        }
    }
}

fn GetObjectArrayElement<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    let dvm = dalvik!(emulator);
    let array_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
    let index = emulator.backend.reg_read(RegisterARM64::X2).unwrap();

    let flag = jni::get_flag_id(array_id);
    let object = if flag == JNI_FLAG_OBJECT {
        let object = dvm.get_local_ref(array_id);
        if object.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, array = 0x{:x}, index = {}) => ArrayNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetObjectArrayElement"), env!(emulator), array_id, index);
            }
            return RET(JNI_NULL)
        }
        object.unwrap()
    } else if flag == JNI_FLAG_REF {
        let object = dvm.get_global_ref(array_id);
        if object.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, array = 0x{:x}, index = {}) => ArrayNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetObjectArrayElement"), env!(emulator), array_id, index);
            }
            return RET(JNI_NULL)
        }
        object.unwrap()
    } else {
        unreachable!()
    };
    match object {
        DvmObject::ObjectArray(_, array) => {
            if index < 0 || index as usize >= array.len() {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    debug!("{} {}(env = 0x{:x}, array = 0x{:x}, index = {}) => ArrayIndexOutOfBounds", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetObjectArrayElement"), env!(emulator), array_id, index);
                }
                return RET(JNI_NULL)
            }
            if let Some(object) = array.get(index as usize) {
                if let Some(object) = object {
                    let id = dvm.add_local_ref(object.clone());
                    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                        debug!("{} {}(env = 0x{:x}, array = 0x{:x}, index = {}) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetObjectArrayElement"), env!(emulator), array_id, index, id);
                    }
                    RET(id)
                } else {
                    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                        debug!("{} {}(env = 0x{:x}, array = 0x{:x}, index = {}) => NULL", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetObjectArrayElement"), env!(emulator), array_id, index);
                    }
                    RET(JNI_NULL)
                }
            } else {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    debug!("{} {}(env = 0x{:x}, array = 0x{:x}, index = {}) => ArrayIndexOutOfBounds", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetObjectArrayElement"), env!(emulator), array_id, index);
                }
                RET(JNI_NULL)
            }
        }
        _ => {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, array = 0x{:x}, index = {}) => NotArray", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetObjectArrayElement"), env!(emulator), array_id, index);
            }
            RET(JNI_NULL)
        }
    }
}

fn NewByteArray<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    let dvm = dalvik!(emulator);
    let size = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as usize;

    let array = DvmObject::ByteArray(vec![0u8; size]);
    let id = dvm.add_local_ref(array);
    if option_env!("PRINT_JNI_CALLS") == Some("1") {
        debug!("{} {}(env = 0x{:x}, size = {}) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("NewByteArray"), env!(emulator), size, id);
    }

    RET(id)
}

fn GetByteArrayElements<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    let dvm = dalvik!(emulator);
    let array = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
    let is_copy = emulator.backend.reg_read(RegisterARM64::X2).unwrap();

    let flag = jni::get_flag_id(array);
    let object = if flag == JNI_FLAG_OBJECT {
        let object = dvm.get_local_ref(array);
        if object.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, array = 0x{:x}) => ArrayNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetByteArrayElements"), env!(emulator), array);
            }
            return RET(JNI_ERR)
        }
        object.unwrap()
    } else if flag == JNI_FLAG_REF {
        let object = dvm.get_global_ref(array);
        if object.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, array = 0x{:x}) => ArrayNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetByteArrayElements"), env!(emulator), array);
            }
            return RET(JNI_ERR)
        }
        object.unwrap()
    } else {
        unreachable!()
    };
    match object {
        DvmObject::ByteArray(data) => {
            let dest = emulator.falloc(data.len(), false).unwrap();
            dest.write_data(data.as_slice()).unwrap();
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, array = 0x{:x}, is_copy = 0x{:X}) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetByteArrayElements"), env!(emulator), array, is_copy, dest.addr as i64);
            }

            if is_copy != 0 {
                emulator.backend.mem_write_i64(is_copy, 1).unwrap();
            }

            RET(dest.addr as i64)
        },
        _ => unreachable!()
    }
}

fn ReleaseByteArrayElements<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    let dvm = dalvik!(emulator);
    let array = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
    let elems = emulator.backend.reg_read(RegisterARM64::X2).unwrap();
    let mode = emulator.backend.reg_read(RegisterARM64::X3).unwrap() as i32;

    let flag = jni::get_flag_id(array);
    let object = if flag == JNI_FLAG_REF {
        let obj = dvm.get_global_ref_mut(array);
        if obj.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, array = 0x{:x}, elems = 0x{:x}, mode = {}) => ArrayNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("ReleaseByteArrayElements"), env!(emulator), array, elems, mode);
            }
            return RET(JNI_ERR)
        }
        obj.unwrap()
    } else if flag == JNI_FLAG_OBJECT {
        let obj = dvm.get_local_ref_mut(array);
        if obj.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, array = 0x{:x}, elems = 0x{:x}, mode = {}) => ArrayNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("ReleaseByteArrayElements"), env!(emulator), array, elems, mode);
            }
            return RET(JNI_ERR)
        }
        obj.unwrap()
    } else {
        unreachable!()
    };

    let array_object = match object {
        DvmObject::ByteArray(array) => array,
        _ => unreachable!()
    };

    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
        debug!("{} {}(env = 0x{:x}, array = 0x{:x}, elems = 0x{:x}, mode = {}) => OK", Color::Yellow.paint("JNI:"), Color::Blue.paint("ReleaseByteArrayElements"), env!(emulator), array, elems, mode);
    }

    match mode {
        0 => {
            let new_data = emulator.backend.mem_read_as_vec(elems, array_object.len()).unwrap();
            array_object.copy_from_slice(new_data.as_slice());
            emulator.ffree(elems, array_object.len()).unwrap();
        }
        1 => {
            let new_data = emulator.backend.mem_read_as_vec(elems, array_object.len()).unwrap();
            array_object.copy_from_slice(new_data.as_slice());
        }
        2 => {
            emulator.ffree(elems, array_object.len()).unwrap();
        }
        _ => unreachable!()
    };

    RET(JNI_OK)
}

fn GetByteArrayRegion<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    //void        (*GetByteArrayRegion)(JNIEnv*, jbyteArray, jsize, jsize, jbyte*);
    let dvm = dalvik!(emulator);
    let array = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
    let start = emulator.backend.reg_read(RegisterARM64::X2).unwrap() as usize;
    let length = emulator.backend.reg_read(RegisterARM64::X3).unwrap() as usize;
    let buf = emulator.backend.reg_read(RegisterARM64::X4).unwrap();

    let flag = jni::get_flag_id(array);
    if flag != JNI_FLAG_REF && flag != JNI_FLAG_OBJECT {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, array = 0x{:x}, start = {}, length = {}, buf = 0x{:x}) => InvalidArray", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetByteArrayRegion"), env!(emulator), array, start, length, buf);
        }
        return FUCK(anyhow!("_GetByteArrayRegion: input array pointer is invalid"))
    }

    let data = match flag {
        JNI_FLAG_REF => {
            let ref_data = dvm.get_global_ref(array);
            if ref_data.is_none() {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    debug!("{} {}(env = 0x{:x}, array = 0x{:x}, start = {}, length = {}, buf = 0x{:x}) => GlobalRefNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetByteArrayRegion"), env!(emulator), array, start, length, buf);
                }
                return FUCK(anyhow!("_GetByteArrayRegion: input array pointer is not found in global ref pool"))
            }
            match ref_data.unwrap() {
                DvmObject::ByteArray(bytes) => bytes,
                _ => unreachable!()
            }
        }
        JNI_FLAG_OBJECT => {
            let ref_data = dvm.get_local_ref(array);
            if ref_data.is_none() {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    debug!("{} {}(env = 0x{:x}, array = 0x{:x}, start = {}, length = {}, buf = 0x{:x}) => LocalRefNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetByteArrayRegion"), env!(emulator), array, start, length, buf);
                }
                return FUCK(anyhow!("_GetByteArrayRegion: input array pointer is not found in local ref pool"))
            }
            match ref_data.unwrap() {
                DvmObject::ByteArray(bytes) => bytes,
                _ => unreachable!()
            }
        }
        _ => unreachable!()
    };

    let data = data[start..start + length].to_vec();
    emulator.backend.mem_write(buf, &data).unwrap();

    if option_env!("PRINT_JNI_CALLS_EX").unwrap_or("") == "1" {
        debug!("{} {}(env = 0x{:x}, array = 0x{:x}, start = {}, length = {}, buf = 0x{:x})", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetByteArrayRegion"), env!(emulator), array, start, length, buf);
        debug!("  data = {} | {:?}", String::from_utf8_lossy(data.as_slice()), hex::encode(&data));
    } else if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
        debug!("{} {}(env = 0x{:x}, array = 0x{:x}, start = {}, length = {}, buf = 0x{:x})", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetByteArrayRegion"), env!(emulator), array, start, length, buf);
    }

    VOID
}

fn SetByteArrayRegion<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    let dvm = dalvik!(emulator);
    let array = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
    let start = emulator.backend.reg_read(RegisterARM64::X2).unwrap() as usize;
    let length = emulator.backend.reg_read(RegisterARM64::X3).unwrap() as usize;
    let buf = emulator.backend.reg_read(RegisterARM64::X4).unwrap();

    let data = emulator.backend.mem_read_as_vec(buf, length).unwrap();
    if option_env!("PRINT_JNI_CALLS_EX").unwrap_or("") == "1" {
        debug!("{} {}(env = 0x{:x}, array = 0x{:x}, start = {}, length = {}, buf = 0x{:x})", Color::Yellow.paint("JNI:"), Color::Blue.paint("SetByteArrayRegion"), env!(emulator), array, start, length, buf);
        debug!("  data = {} | {:?}", String::from_utf8_lossy(data.as_slice()), hex::encode(&data));
    } else if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
        println!("{} {}(env = 0x{:x}, array = 0x{:x}, start = {}, length = {}, buf = 0x{:x})", Color::Yellow.paint("JNI:"), Color::Blue.paint("SetByteArrayRegion"), env!(emulator), array, start, length, buf);
    }

    let flag = jni::get_flag_id(array);
    if flag != JNI_FLAG_REF && flag != JNI_FLAG_OBJECT {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, array = 0x{:x}, start = {}, length = {}, buf = 0x{:x}) => InvalidArray", Color::Yellow.paint("JNI:"), Color::Blue.paint("SetByteArrayRegion"), env!(emulator), array, start, length, buf);
        }
        return FUCK(anyhow!("_SetByteArrayRegion: input array pointer is invalid"))
    }

    if flag == JNI_FLAG_REF {
        let ref_data = dvm.get_global_ref_mut(array);
        if ref_data.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, array = 0x{:x}, start = {}, length = {}, buf = 0x{:x}) => GlobalRefNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("SetByteArrayRegion"), env!(emulator), array, start, length, buf);
            }
            return FUCK(anyhow!("_SetByteArrayRegion: input array pointer is not found in global ref pool"))
        }
        match ref_data.unwrap() {
            DvmObject::ByteArray(bytes) => {
                bytes.splice(start..start + length, data.iter().cloned());
            }
            _ => unreachable!()
        }
    } else if flag == JNI_FLAG_OBJECT {
        let ref_data = dvm.get_local_ref_mut(array);
        if ref_data.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                debug!("{} {}(env = 0x{:x}, array = 0x{:x}, start = {}, length = {}, buf = 0x{:x}) => LocalRefNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("SetByteArrayRegion"), env!(emulator), array, start, length, buf);
            }
            return FUCK(anyhow!("_SetByteArrayRegion: input array pointer is not found in local ref pool"))
        }
        match ref_data.unwrap() {
            DvmObject::ByteArray(bytes) => {
                bytes.splice(start..start + length, data.iter().cloned());
            }
            _ => unreachable!()
        }
    }
    VOID
}

fn MicroGetJavaVM<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    //let dvm = emulator.inner_mut().dalvik.as_ref().unwrap();
    let dvm = dalvik!(emulator);
    let vm_ptr = emulator.backend.reg_read(RegisterARM64::X1).unwrap();
    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
        debug!("{} {}(env = 0x{:x}, vm = 0x{:x})", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetJavaVM"), env!(emulator), vm_ptr);
    }
    let vm = dvm.java_vm;
    emulator.backend.mem_write(vm_ptr, &vm.to_le_bytes()).unwrap();
    RET(JNI_OK)
}

fn RegisterNatives<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    let dvm = dalvik!(emulator);

    let clz_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
    let methods = emulator.backend.reg_read(RegisterARM64::X2).unwrap();
    let n_methods = emulator.backend.reg_read(RegisterARM64::X3).unwrap();

    if n_methods == 0 {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}) => NoAnyMethod", Color::Yellow.paint("JNI:"), Color::Blue.paint("RegisterNatives"), env!(emulator), clz_id);
        }
        return RET(JNI_OK)
    }

    if dvm.find_class_by_id(&clz_id).is_none() {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}) => ClassNotFOunt", Color::Yellow.paint("JNI:"), Color::Blue.paint("RegisterNatives"), env!(emulator), clz_id);
        }
        return RET(JNI_ERR)
    }

    if !dvm.members.contains_key(&clz_id) {
        dvm.members.insert(clz_id, Vec::new());
    }
    let members_list = dvm.members.get_mut(&clz_id).unwrap();

    const JNINATIVE_METHOD_SIZE: usize = mem::size_of::<JNINativeMethod>();
    let mut start_id = members_list.len() as i64;
    let mut buf = [0u8; JNINATIVE_METHOD_SIZE];
    for i in 0..n_methods {
        emulator.backend.mem_read(methods + i * JNINATIVE_METHOD_SIZE as u64, &mut buf).unwrap();
        let method = unsafe { &*(buf.as_ptr() as *const JNINativeMethod) };
        let name = emulator.backend.mem_read_c_string(method.name).unwrap();
        let signature = emulator.backend.mem_read_c_string(method.signature).unwrap();
        let fn_ptr = method.fn_ptr;
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            debug!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}, fn_ptr = 0x{:X})", Color::Yellow.paint("JNI:"), Color::Blue.paint("RegisterNatives"), env!(emulator), clz_id, name, signature, fn_ptr);
        }
        start_id += 1;
        members_list.push(DvmMember::Method(DvmMethod::new(clz_id + start_id, clz_id, name, signature, fn_ptr)));
    }

    RET(JNI_OK)
}

fn ExceptionCheck<T: Clone>(name: &str, emulator: &AndroidEmulator<T>) -> SvcCallResult {
    let dvm = emulator.inner_mut().dalvik.as_ref().unwrap();
    let ret = if dvm.throwable.is_some() { JNI_TRUE } else { JNI_FALSE };
    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
        debug!("{} {}(env = 0x{:x}) => {}", Color::Yellow.paint("JNI:"), Color::Blue.paint("ExceptionCheck"), env!(emulator), ret);
    }
    RET(ret)
}

pub fn initialize_env<T: Clone>(svc_memory: &mut SvcMemory<T>) -> u64 {
    let java_env = svc_memory.allocate(8, "_JNIEnv");
    let imp = svc_memory.allocate(0x748 + 8, "_JNIEnv.impl");

    let _get_version = svc_memory.register_svc(SimpleArm64Svc::new("_GetVersion", GetVersion));
    let _define_class = svc_memory.register_svc(SimpleArm64Svc::new("_DefineClass", DefineClass));
    let _find_class = svc_memory.register_svc(SimpleArm64Svc::new("_FindClass", FindClass));
    let _from_reflected_method = svc_memory.register_svc(SimpleArm64Svc::new("_FromReflectedMethod", NoImplementedHandler));
    let _from_reflected_field = svc_memory.register_svc(SimpleArm64Svc::new("_FromReflectedField", NoImplementedHandler));
    let _to_reflected_method = svc_memory.register_svc(SimpleArm64Svc::new("_ToReflectedMethod", NoImplementedHandler));
    let _get_superclass = svc_memory.register_svc(SimpleArm64Svc::new("_GetSuperclass", NoImplementedHandler));
    let _is_assignable_from = svc_memory.register_svc(SimpleArm64Svc::new("_IsAssignableFrom", NoImplementedHandler));
    let _to_reflected_field = svc_memory.register_svc(SimpleArm64Svc::new("_ToReflectedField", NoImplementedHandler));
    let _throw = svc_memory.register_svc(SimpleArm64Svc::new("_Throw", NoImplementedHandler));
    let _throw_new = svc_memory.register_svc(SimpleArm64Svc::new("_ThrowNew", NoImplementedHandler));
    let _exception_occurred = svc_memory.register_svc(SimpleArm64Svc::new("_ExceptionOccurred", NoImplementedHandler));
    let _exception_describe = svc_memory.register_svc(SimpleArm64Svc::new("_ExceptionDescribe", NoImplementedHandler));
    let _exception_clear = svc_memory.register_svc(SimpleArm64Svc::new("_ExceptionClear", ExceptionClear));
    let _fatal_error = svc_memory.register_svc(SimpleArm64Svc::new("_FatalError", NoImplementedHandler));
    let _push_local_frame = svc_memory.register_svc(SimpleArm64Svc::new("_PushLocalFrame", NoImplementedHandler));
    let _pop_local_frame = svc_memory.register_svc(SimpleArm64Svc::new("_PopLocalFrame", NoImplementedHandler));
    let _new_global_ref = svc_memory.register_svc(SimpleArm64Svc::new("_NewGlobalRef", NewGlobalRef));
    let _delete_global_ref = svc_memory.register_svc(SimpleArm64Svc::new("_DeleteGlobalRef", DeleteGlobalRef));
    let _delete_local_ref = svc_memory.register_svc(SimpleArm64Svc::new("_DeleteLocalRef", DeleteLocalRef));
    let _is_same_object = svc_memory.register_svc(SimpleArm64Svc::new("_IsSameObject", NoImplementedHandler));
    let _new_local_ref = svc_memory.register_svc(SimpleArm64Svc::new("_NewLocalRef", NoImplementedHandler));
    let _ensure_local_capacity = svc_memory.register_svc(SimpleArm64Svc::new("_EnsureLocalCapacity", NoImplementedHandler));
    let _alloc_object = svc_memory.register_svc(SimpleArm64Svc::new("_AllocObject", NoImplementedHandler));
    let _new_object = svc_memory.register_svc(SimpleArm64Svc::new("_NewObject", NoImplementedHandler));
    let _new_object_v = svc_memory.register_svc(SimpleArm64Svc::new("_NewObjectV", NewObjectV));
    let _new_object_a = svc_memory.register_svc(SimpleArm64Svc::new("_NewObjectA", NoImplementedHandler));
    let _get_object_class = svc_memory.register_svc(SimpleArm64Svc::new("_GetObjectClass", GetObjectClass));
    let _is_instance_of = svc_memory.register_svc(SimpleArm64Svc::new("_IsInstanceOf", NoImplementedHandler));
    let _get_method_id = svc_memory.register_svc(SimpleArm64Svc::new("_GetMethodID", GetMethodID));
    let _call_object_method = svc_memory.register_svc(SimpleArm64Svc::new("_CallObjectMethod", NoImplementedHandler));
    let _call_object_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallObjectMethodV", CallObjectMethodV));
    let _call_object_method_a = svc_memory.register_svc(SimpleArm64Svc::new("_CallObjectMethodA", NoImplementedHandler));
    let _call_boolean_method = svc_memory.register_svc(SimpleArm64Svc::new("_CallBooleanMethod", NoImplementedHandler));
    let _call_boolean_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallBooleanMethodV", CallBooleanMethodV));
    let _call_boolean_method_a = svc_memory.register_svc(SimpleArm64Svc::new("_CallBooleanMethodA", NoImplementedHandler));
    let _call_byte_method = svc_memory.register_svc(SimpleArm64Svc::new("_CallByteMethod", NoImplementedHandler));
    let _call_byte_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallByteMethodV", NoImplementedHandler));
    let _call_byte_method_a = svc_memory.register_svc(SimpleArm64Svc::new("_CallByteMethodA", NoImplementedHandler));
    let _call_char_method = svc_memory.register_svc(SimpleArm64Svc::new("_CallCharMethod", NoImplementedHandler));
    let _call_char_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallCharMethodV", NoImplementedHandler));
    let _call_char_method_a = svc_memory.register_svc(SimpleArm64Svc::new("_CallCharMethodA", NoImplementedHandler));
    let _call_short_method = svc_memory.register_svc(SimpleArm64Svc::new("_CallShortMethod", NoImplementedHandler));
    let _call_short_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallShortMethodV", NoImplementedHandler));
    let _call_short_method_a = svc_memory.register_svc(SimpleArm64Svc::new("_CallShortMethodA", NoImplementedHandler));
    let _call_int_method = svc_memory.register_svc(SimpleArm64Svc::new("_CallIntMethod", NoImplementedHandler));
    let _call_int_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallIntMethodV", CallIntMethodV));
    let _call_int_method_a = svc_memory.register_svc(SimpleArm64Svc::new("_CallIntMethodA", NoImplementedHandler));
    let _call_long_method = svc_memory.register_svc(SimpleArm64Svc::new("_CallLongMethod", NoImplementedHandler));
    let _call_long_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallLongMethodV", NoImplementedHandler));
    let _call_long_method_a = svc_memory.register_svc(SimpleArm64Svc::new("_CallLongMethodA", NoImplementedHandler));
    let _call_float_method = svc_memory.register_svc(SimpleArm64Svc::new("_CallFloatMethod", NoImplementedHandler));
    let _call_float_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallFloatMethodV", NoImplementedHandler));
    let _call_float_method_a = svc_memory.register_svc(SimpleArm64Svc::new("_CallFloatMethodA", NoImplementedHandler));
    let _call_double_method = svc_memory.register_svc(SimpleArm64Svc::new("_CallDoubleMethod", NoImplementedHandler));
    let _call_double_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallDoubleMethodV", NoImplementedHandler));
    let _call_double_method_a = svc_memory.register_svc(SimpleArm64Svc::new("_CallDoubleMethodA", NoImplementedHandler));
    let _call_void_method = svc_memory.register_svc(SimpleArm64Svc::new("_CallVoidMethod", NoImplementedHandler));
    let _call_void_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallVoidMethodV", CallVoidMethodV));
    let _call_void_method_a = svc_memory.register_svc(SimpleArm64Svc::new("_CallVoidMethodA", NoImplementedHandler));
    let _call_nonvirtual_object_method = svc_memory.register_svc(SimpleArm64Svc::new("_CallNonvirtualObjectMethod", NoImplementedHandler));
    let _call_nonvirtual_object_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallNonvirtualObjectMethodV", NoImplementedHandler));
    let _call_nonvirtual_object_method_a = svc_memory.register_svc(SimpleArm64Svc::new("_CallNonvirtualObjectMethodA", NoImplementedHandler));
    let _call_nonvirtual_boolean_method = svc_memory.register_svc(SimpleArm64Svc::new("_CallNonvirtualBooleanMethod", NoImplementedHandler));
    let _call_nonvirtual_boolean_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallNonvirtualBooleanMethodV", NoImplementedHandler));
    let _call_nonvirtual_boolean_method_a = svc_memory.register_svc(SimpleArm64Svc::new("_CallNonvirtualBooleanMethodA", NoImplementedHandler));
    let _call_nonvirtual_byte_method = svc_memory.register_svc(SimpleArm64Svc::new("_CallNonvirtualByteMethod", NoImplementedHandler));
    let _call_nonvirtual_byte_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallNonvirtualByteMethodV", NoImplementedHandler));
    let _call_nonvirtual_byte_method_a = svc_memory.register_svc(SimpleArm64Svc::new("_CallNonvirtualByteMethodA", NoImplementedHandler));
    let _call_nonvirtual_char_method = svc_memory.register_svc(SimpleArm64Svc::new("_CallNonvirtualCharMethod", NoImplementedHandler));
    let _call_nonvirtual_char_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallNonvirtualCharMethodV", NoImplementedHandler));
    let _call_nonvirtual_char_method_a = svc_memory.register_svc(SimpleArm64Svc::new("_CallNonvirtualCharMethodA", NoImplementedHandler));
    let _call_nonvirtual_short_method = svc_memory.register_svc(SimpleArm64Svc::new("_CallNonvirtualShortMethod", NoImplementedHandler));
    let _call_nonvirtual_short_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallNonvirtualShortMethodV", NoImplementedHandler));
    let _call_nonvirtual_short_method_a = svc_memory.register_svc(SimpleArm64Svc::new("_CallNonvirtualShortMethodA", NoImplementedHandler));
    let _call_nonvirtual_int_method = svc_memory.register_svc(SimpleArm64Svc::new("_CallNonvirtualIntMethod", NoImplementedHandler));
    let _call_nonvirtual_int_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallNonvirtualIntMethodV", NoImplementedHandler));
    let _call_nonvirtual_int_method_a = svc_memory.register_svc(SimpleArm64Svc::new("_CallNonvirtualIntMethodA", NoImplementedHandler));
    let _call_nonvirtual_long_method = svc_memory.register_svc(SimpleArm64Svc::new("_CallNonvirtualLongMethod", NoImplementedHandler));
    let _call_nonvirtual_long_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallNonvirtualLongMethodV", NoImplementedHandler));
    let _call_nonvirtual_long_method_a = svc_memory.register_svc(SimpleArm64Svc::new("_CallNonvirtualLongMethodA", NoImplementedHandler));
    let _call_nonvirtual_float_method = svc_memory.register_svc(SimpleArm64Svc::new("_CallNonvirtualFloatMethod", NoImplementedHandler));
    let _call_nonvirtual_float_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallNonvirtualFloatMethodV", NoImplementedHandler));
    let _call_nonvirtual_float_method_a = svc_memory.register_svc(SimpleArm64Svc::new("_CallNonvirtualFloatMethodA", NoImplementedHandler));
    let _call_nonvirtual_double_method = svc_memory.register_svc(SimpleArm64Svc::new("_CallNonvirtualDoubleMethod", NoImplementedHandler));
    let _call_nonvirtual_double_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallNonvirtualDoubleMethodV", NoImplementedHandler));
    let _call_nonvirtual_double_method_a = svc_memory.register_svc(SimpleArm64Svc::new("_CallNonvirtualDoubleMethodA", NoImplementedHandler));
    let _call_nonvirtual_void_method = svc_memory.register_svc(SimpleArm64Svc::new("_CallNonvirtualVoidMethod", NoImplementedHandler));
    let _call_nonvirtual_void_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallNonvirtualVoidMethodV", NoImplementedHandler));
    let _call_nonvirtual_void_method_a = svc_memory.register_svc(SimpleArm64Svc::new("_CallNonvirtualVoidMethodA", NoImplementedHandler));
    let _get_field_id = svc_memory.register_svc(SimpleArm64Svc::new("_GetFieldID", GetFieldID));
    let _get_object_field = svc_memory.register_svc(SimpleArm64Svc::new("_GetObjectField", GetObjectField));
    let _get_boolean_field = svc_memory.register_svc(SimpleArm64Svc::new("_GetBooleanField", NoImplementedHandler));
    let _get_byte_field = svc_memory.register_svc(SimpleArm64Svc::new("_GetByteField", NoImplementedHandler));
    let _get_char_field = svc_memory.register_svc(SimpleArm64Svc::new("_GetCharField", NoImplementedHandler));
    let _get_short_field = svc_memory.register_svc(SimpleArm64Svc::new("_GetShortField", NoImplementedHandler));
    let _get_int_field = svc_memory.register_svc(SimpleArm64Svc::new("_GetIntField", GetIntField));
    let _get_long_field = svc_memory.register_svc(SimpleArm64Svc::new("_GetLongField", NoImplementedHandler));
    let _get_float_field = svc_memory.register_svc(SimpleArm64Svc::new("_GetFloatField", NoImplementedHandler));
    let _get_double_field = svc_memory.register_svc(SimpleArm64Svc::new("_GetDoubleField", NoImplementedHandler));
    let _set_object_field = svc_memory.register_svc(SimpleArm64Svc::new("_SetObjectField", SetObjectField));
    let _set_boolean_field = svc_memory.register_svc(SimpleArm64Svc::new("_SetBooleanField", NoImplementedHandler));
    let _set_byte_field = svc_memory.register_svc(SimpleArm64Svc::new("_SetByteField", NoImplementedHandler));
    let _set_char_field = svc_memory.register_svc(SimpleArm64Svc::new("_SetCharField", NoImplementedHandler));
    let _set_short_field = svc_memory.register_svc(SimpleArm64Svc::new("_SetShortField", NoImplementedHandler));
    let _set_int_field = svc_memory.register_svc(SimpleArm64Svc::new("_SetIntField", NoImplementedHandler));
    let _set_long_field = svc_memory.register_svc(SimpleArm64Svc::new("_SetLongField", NoImplementedHandler));
    let _set_float_field = svc_memory.register_svc(SimpleArm64Svc::new("_SetFloatField", NoImplementedHandler));
    let _set_double_field = svc_memory.register_svc(SimpleArm64Svc::new("_SetDoubleField", NoImplementedHandler));
    let _get_static_method_id = svc_memory.register_svc(SimpleArm64Svc::new("_GetStaticMethodID", GetStaticMethodID));
    let _call_static_object_method = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticObjectMethod", NoImplementedHandler));
    let _call_static_object_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticObjectMethodV", CallStaticObjectMethodV));
    let _call_static_object_method_a = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticObjectMethodA", NoImplementedHandler));
    let _call_static_boolean_method = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticBooleanMethod", NoImplementedHandler));
    let _call_static_boolean_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticBooleanMethodV", NoImplementedHandler));
    let _call_static_boolean_method_a = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticBooleanMethodA", NoImplementedHandler));
    let _call_static_byte_method = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticByteMethod", NoImplementedHandler));
    let _call_static_byte_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticByteMethodV", NoImplementedHandler));
    let _call_static_byte_method_a = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticByteMethodA", NoImplementedHandler));
    let _call_static_char_method = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticCharMethod", NoImplementedHandler));
    let _call_static_char_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticCharMethodV", NoImplementedHandler));
    let _call_static_char_method_a = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticCharMethodA", NoImplementedHandler));
    let _call_static_short_method = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticShortMethod", NoImplementedHandler));
    let _call_static_short_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticShortMethodV", NoImplementedHandler));
    let _call_static_short_method_a = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticShortMethodA", NoImplementedHandler));
    let _call_static_int_method = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticIntMethod", NoImplementedHandler));
    let _call_static_int_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticIntMethodV", CallStaticIntMethodV));
    let _call_static_int_method_a = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticIntMethodA", NoImplementedHandler));
    let _call_static_long_method = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticLongMethod", NoImplementedHandler));
    let _call_static_long_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticLongMethodV", NoImplementedHandler));
    let _call_static_long_method_a = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticLongMethodA", NoImplementedHandler));
    let _call_static_float_method = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticFloatMethod", NoImplementedHandler));
    let _call_static_float_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticFloatMethodV", NoImplementedHandler));
    let _call_static_float_method_a = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticFloatMethodA", NoImplementedHandler));
    let _call_static_double_method = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticDoubleMethod", NoImplementedHandler));
    let _call_static_double_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticDoubleMethodV", NoImplementedHandler));
    let _call_static_double_method_a = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticDoubleMethodA", NoImplementedHandler));
    let _call_static_void_method = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticVoidMethod", NoImplementedHandler));
    let _call_static_void_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticVoidMethodV", CallStaticVoidMethodV));
    let _call_static_void_method_a = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticVoidMethodA", NoImplementedHandler));
    let _get_static_field_id = svc_memory.register_svc(SimpleArm64Svc::new("_GetStaticFieldID", GetStaticFieldID));
    let _get_static_object_field = svc_memory.register_svc(SimpleArm64Svc::new("_GetStaticObjectField", GetStaticObjectField));
    let _get_static_boolean_field = svc_memory.register_svc(SimpleArm64Svc::new("_GetStaticBooleanField", NoImplementedHandler));
    let _get_static_byte_field = svc_memory.register_svc(SimpleArm64Svc::new("_GetStaticByteField", NoImplementedHandler));
    let _get_static_char_field = svc_memory.register_svc(SimpleArm64Svc::new("_GetStaticCharField", NoImplementedHandler));
    let _get_static_short_field = svc_memory.register_svc(SimpleArm64Svc::new("_GetStaticShortField", NoImplementedHandler));
    let _get_static_int_field = svc_memory.register_svc(SimpleArm64Svc::new("_GetStaticIntField", GetStaticIntField));
    let _get_static_long_field = svc_memory.register_svc(SimpleArm64Svc::new("_GetStaticLongField", NoImplementedHandler));
    let _get_static_float_field = svc_memory.register_svc(SimpleArm64Svc::new("_GetStaticFloatField", NoImplementedHandler));
    let _get_static_double_field = svc_memory.register_svc(SimpleArm64Svc::new("_GetStaticDoubleField", NoImplementedHandler));
    let _set_static_object_field = svc_memory.register_svc(SimpleArm64Svc::new("_SetStaticObjectField", NoImplementedHandler));
    let _set_static_boolean_field = svc_memory.register_svc(SimpleArm64Svc::new("_SetStaticBooleanField", NoImplementedHandler));
    let _set_static_byte_field = svc_memory.register_svc(SimpleArm64Svc::new("_SetStaticByteField", NoImplementedHandler));
    let _set_static_char_field = svc_memory.register_svc(SimpleArm64Svc::new("_SetStaticCharField", NoImplementedHandler));
    let _set_static_short_field = svc_memory.register_svc(SimpleArm64Svc::new("_SetStaticShortField", NoImplementedHandler));
    let _set_static_int_field = svc_memory.register_svc(SimpleArm64Svc::new("_SetStaticIntField", NoImplementedHandler));
    let _set_static_long_field = svc_memory.register_svc(SimpleArm64Svc::new("_SetStaticLongField", NoImplementedHandler));
    let _set_static_float_field = svc_memory.register_svc(SimpleArm64Svc::new("_SetStaticFloatField", NoImplementedHandler));
    let _set_static_double_field = svc_memory.register_svc(SimpleArm64Svc::new("_SetStaticDoubleField", NoImplementedHandler));
    let _new_string = svc_memory.register_svc(SimpleArm64Svc::new("_NewString", NoImplementedHandler));
    let _get_string_length = svc_memory.register_svc(SimpleArm64Svc::new("_GetStringLength", GetStringLength));
    let _get_string_chars = svc_memory.register_svc(SimpleArm64Svc::new("_GetStringChars", NoImplementedHandler));
    let _release_string_chars = svc_memory.register_svc(SimpleArm64Svc::new("_ReleaseStringChars", NoImplementedHandler));
    let _new_string_utf = svc_memory.register_svc(SimpleArm64Svc::new("_NewStringUTF", NewStringUTF));
    let _get_string_utf_length = svc_memory.register_svc(SimpleArm64Svc::new("_GetStringUTFLength", GetStringUTFLength));
    let _get_string_utf_chars = svc_memory.register_svc(SimpleArm64Svc::new("_GetStringUTFChars", GetStringUTFChars));
    let _release_string_utf_chars = svc_memory.register_svc(SimpleArm64Svc::new("_ReleaseStringUTFChars", ReleaseStringUTFChars));
    let _get_array_length = svc_memory.register_svc(SimpleArm64Svc::new("_GetArrayLength", GetArrayLength));
    let _new_object_array = svc_memory.register_svc(SimpleArm64Svc::new("_NewObjectArray", NoImplementedHandler));
    let _get_object_array_element = svc_memory.register_svc(SimpleArm64Svc::new("_GetObjectArrayElement", GetObjectArrayElement));
    let _set_object_array_element = svc_memory.register_svc(SimpleArm64Svc::new("_SetObjectArrayElement", NoImplementedHandler));
    let _new_boolean_array = svc_memory.register_svc(SimpleArm64Svc::new("_NewBooleanArray", NoImplementedHandler));
    let _new_byte_array = svc_memory.register_svc(SimpleArm64Svc::new("_NewByteArray", NewByteArray));
    let _new_char_array = svc_memory.register_svc(SimpleArm64Svc::new("_NewCharArray", NoImplementedHandler));
    let _new_short_array = svc_memory.register_svc(SimpleArm64Svc::new("_NewShortArray", NoImplementedHandler));
    let _new_int_array = svc_memory.register_svc(SimpleArm64Svc::new("_NewIntArray", NoImplementedHandler));
    let _new_long_array = svc_memory.register_svc(SimpleArm64Svc::new("_NewLongArray", NoImplementedHandler));
    let _new_float_array = svc_memory.register_svc(SimpleArm64Svc::new("_NewFloatArray", NoImplementedHandler));
    let _new_double_array = svc_memory.register_svc(SimpleArm64Svc::new("_NewDoubleArray", NoImplementedHandler));
    let _get_boolean_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("_GetBooleanArrayElements", NoImplementedHandler));
    let _get_byte_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("_GetByteArrayElements", GetByteArrayElements));
    let _get_char_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("_GetCharArrayElements", NoImplementedHandler));
    let _get_short_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("_GetShortArrayElements", NoImplementedHandler));
    let _get_int_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("_GetIntArrayElements", NoImplementedHandler));
    let _get_long_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("_GetLongArrayElements", NoImplementedHandler));
    let _get_float_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("_GetFloatArrayElements", NoImplementedHandler));
    let _get_double_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("_GetDoubleArrayElements", NoImplementedHandler));
    let _release_boolean_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("_ReleaseBooleanArrayElements", NoImplementedHandler));
    let _release_byte_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("_ReleaseByteArrayElements", ReleaseByteArrayElements));
    let _release_char_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("_ReleaseCharArrayElements", NoImplementedHandler));
    let _release_short_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("_ReleaseShortArrayElements", NoImplementedHandler));
    let _release_int_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("_ReleaseIntArrayElements", NoImplementedHandler));
    let _release_long_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("_ReleaseLongArrayElements", NoImplementedHandler));
    let _release_float_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("_ReleaseFloatArrayElements", NoImplementedHandler));
    let _release_double_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("_ReleaseDoubleArrayElements", NoImplementedHandler));
    let _get_boolean_array_region = svc_memory.register_svc(SimpleArm64Svc::new("_GetBooleanArrayRegion", NoImplementedHandler));
    let _get_byte_array_region = svc_memory.register_svc(SimpleArm64Svc::new("_GetByteArrayRegion", GetByteArrayRegion));
    let _get_char_array_region = svc_memory.register_svc(SimpleArm64Svc::new("_GetCharArrayRegion", NoImplementedHandler));
    let _get_short_array_region = svc_memory.register_svc(SimpleArm64Svc::new("_GetShortArrayRegion", NoImplementedHandler));
    let _get_int_array_region = svc_memory.register_svc(SimpleArm64Svc::new("_GetIntArrayRegion", NoImplementedHandler));
    let _get_long_array_region = svc_memory.register_svc(SimpleArm64Svc::new("_GetLongArrayRegion", NoImplementedHandler));
    let _get_float_array_region = svc_memory.register_svc(SimpleArm64Svc::new("_GetFloatArrayRegion", NoImplementedHandler));
    let _get_double_array_region = svc_memory.register_svc(SimpleArm64Svc::new("_GetDoubleArrayRegion", NoImplementedHandler));
    let _set_boolean_array_region = svc_memory.register_svc(SimpleArm64Svc::new("_SetBooleanArrayRegion", NoImplementedHandler));
    let _set_byte_array_region = svc_memory.register_svc(SimpleArm64Svc::new("_SetByteArrayRegion", SetByteArrayRegion));
    let _set_char_array_region = svc_memory.register_svc(SimpleArm64Svc::new("_SetCharArrayRegion", NoImplementedHandler));
    let _set_short_array_region = svc_memory.register_svc(SimpleArm64Svc::new("_SetShortArrayRegion", NoImplementedHandler));
    let _set_int_array_region = svc_memory.register_svc(SimpleArm64Svc::new("_SetIntArrayRegion", NoImplementedHandler));
    let _set_long_array_region = svc_memory.register_svc(SimpleArm64Svc::new("_SetLongArrayRegion", NoImplementedHandler));
    let _set_float_array_region = svc_memory.register_svc(SimpleArm64Svc::new("_SetFloatArrayRegion", NoImplementedHandler));
    let _set_double_array_region = svc_memory.register_svc(SimpleArm64Svc::new("_SetDoubleArrayRegion", NoImplementedHandler));
    let _register_natives = svc_memory.register_svc(SimpleArm64Svc::new("_RegisterNatives", RegisterNatives));
    let _unregister_natives = svc_memory.register_svc(SimpleArm64Svc::new("_UnregisterNatives", NoImplementedHandler));
    let _monitor_enter = svc_memory.register_svc(SimpleArm64Svc::new("_MonitorEnter", NoImplementedHandler));
    let _monitor_exit = svc_memory.register_svc(SimpleArm64Svc::new("_MonitorExit", NoImplementedHandler));
    let _get_java_vm = svc_memory.register_svc(SimpleArm64Svc::new("_GetJavaVM", MicroGetJavaVM));
    let _get_string_region = svc_memory.register_svc(SimpleArm64Svc::new("_GetStringRegion", NoImplementedHandler));
    let _get_string_utf_region = svc_memory.register_svc(SimpleArm64Svc::new("_GetStringUTFRegion", NoImplementedHandler));
    let _get_primitive_array_critical = svc_memory.register_svc(SimpleArm64Svc::new("_GetPrimitiveArrayCritical", NoImplementedHandler));
    let _release_primitive_array_critical = svc_memory.register_svc(SimpleArm64Svc::new("_ReleasePrimitiveArrayCritical", NoImplementedHandler));
    let _get_string_critical = svc_memory.register_svc(SimpleArm64Svc::new("_GetStringCritical", NoImplementedHandler));
    let _release_string_critical = svc_memory.register_svc(SimpleArm64Svc::new("_ReleaseStringCritical", NoImplementedHandler));
    let _new_weak_global_ref = svc_memory.register_svc(SimpleArm64Svc::new("_NewWeakGlobalRef", NoImplementedHandler));
    let _delete_weak_global_ref = svc_memory.register_svc(SimpleArm64Svc::new("_DeleteWeakGlobalRef", NoImplementedHandler));
    let _exception_check = svc_memory.register_svc(SimpleArm64Svc::new("_ExceptionCheck", ExceptionCheck));
    let _new_direct_byte_buffer = svc_memory.register_svc(SimpleArm64Svc::new("_NewDirectByteBuffer", NoImplementedHandler));
    let _get_direct_buffer_address = svc_memory.register_svc(SimpleArm64Svc::new("_GetDirectBufferAddress", NoImplementedHandler));
    let _get_direct_buffer_capacity = svc_memory.register_svc(SimpleArm64Svc::new("_GetDirectBufferCapacity", NoImplementedHandler));
    let _get_object_ref_type = svc_memory.register_svc(SimpleArm64Svc::new("_GetObjectRefType", NoImplementedHandler));

    {
        let _ = imp.write_u64_with_offset(0 * 8, 0);
        let _ = imp.write_u64_with_offset(1 * 8, 0);
        let _ = imp.write_u64_with_offset(2 * 8, 0);
        let _ = imp.write_u64_with_offset(3 * 8, 0);
        let _ = imp.write_u64_with_offset(4 * 8, _get_version);
        let _ = imp.write_u64_with_offset(5 * 8, _define_class);
        let _ = imp.write_u64_with_offset(6 * 8, _find_class);
        let _ = imp.write_u64_with_offset(7 * 8, _from_reflected_method);
        let _ = imp.write_u64_with_offset(8 * 8, _from_reflected_field);
        let _ = imp.write_u64_with_offset(9 * 8, _to_reflected_method);
        let _ = imp.write_u64_with_offset(10 * 8, _get_superclass);
        let _ = imp.write_u64_with_offset(11 * 8, _is_assignable_from);
        let _ = imp.write_u64_with_offset(12 * 8, _to_reflected_field);
        let _ = imp.write_u64_with_offset(13 * 8, _throw);
        let _ = imp.write_u64_with_offset(14 * 8, _throw_new);
        let _ = imp.write_u64_with_offset(15 * 8, _exception_occurred);
        let _ = imp.write_u64_with_offset(16 * 8, _exception_describe);
        let _ = imp.write_u64_with_offset(17 * 8, _exception_clear);
        let _ = imp.write_u64_with_offset(18 * 8, _fatal_error);
        let _ = imp.write_u64_with_offset(19 * 8, _push_local_frame);
        let _ = imp.write_u64_with_offset(20 * 8, _pop_local_frame);
        let _ = imp.write_u64_with_offset(21 * 8, _new_global_ref);
        let _ = imp.write_u64_with_offset(22 * 8, _delete_global_ref);
        let _ = imp.write_u64_with_offset(23 * 8, _delete_local_ref);
        let _ = imp.write_u64_with_offset(24 * 8, _is_same_object);
        let _ = imp.write_u64_with_offset(25 * 8, _new_local_ref);
        let _ = imp.write_u64_with_offset(26 * 8, _ensure_local_capacity);
        let _ = imp.write_u64_with_offset(27 * 8, _alloc_object);
        let _ = imp.write_u64_with_offset(28 * 8, _new_object);
        let _ = imp.write_u64_with_offset(29 * 8, _new_object_v);
        let _ = imp.write_u64_with_offset(30 * 8, _new_object_a);
        let _ = imp.write_u64_with_offset(31 * 8, _get_object_class);
        let _ = imp.write_u64_with_offset(32 * 8, _is_instance_of);
        let _ = imp.write_u64_with_offset(33 * 8, _get_method_id);
        let _ = imp.write_u64_with_offset(34 * 8, _call_object_method);
        let _ = imp.write_u64_with_offset(35 * 8, _call_object_method_v);
        let _ = imp.write_u64_with_offset(36 * 8, _call_object_method_a);
        let _ = imp.write_u64_with_offset(37 * 8, _call_boolean_method);
        let _ = imp.write_u64_with_offset(38 * 8, _call_boolean_method_v);
        let _ = imp.write_u64_with_offset(39 * 8, _call_boolean_method_a);
        let _ = imp.write_u64_with_offset(40 * 8, _call_byte_method);
        let _ = imp.write_u64_with_offset(41 * 8, _call_byte_method_v);
        let _ = imp.write_u64_with_offset(42 * 8, _call_byte_method_a);
        let _ = imp.write_u64_with_offset(43 * 8, _call_char_method);
        let _ = imp.write_u64_with_offset(44 * 8, _call_char_method_v);
        let _ = imp.write_u64_with_offset(45 * 8, _call_char_method_a);
        let _ = imp.write_u64_with_offset(46 * 8, _call_short_method);
        let _ = imp.write_u64_with_offset(47 * 8, _call_short_method_v);
        let _ = imp.write_u64_with_offset(48 * 8, _call_short_method_a);
        let _ = imp.write_u64_with_offset(49 * 8, _call_int_method);
        let _ = imp.write_u64_with_offset(50 * 8, _call_int_method_v);
        let _ = imp.write_u64_with_offset(51 * 8, _call_int_method_a);
        let _ = imp.write_u64_with_offset(52 * 8, _call_long_method);
        let _ = imp.write_u64_with_offset(53 * 8, _call_long_method_v);
        let _ = imp.write_u64_with_offset(54 * 8, _call_long_method_a);
        let _ = imp.write_u64_with_offset(55 * 8, _call_float_method);
        let _ = imp.write_u64_with_offset(56 * 8, _call_float_method_v);
        let _ = imp.write_u64_with_offset(57 * 8, _call_float_method_a);
        let _ = imp.write_u64_with_offset(58 * 8, _call_double_method);
        let _ = imp.write_u64_with_offset(59 * 8, _call_double_method_v);
        let _ = imp.write_u64_with_offset(60 * 8, _call_double_method_a);
        let _ = imp.write_u64_with_offset(61 * 8, _call_void_method);
        let _ = imp.write_u64_with_offset(62 * 8, _call_void_method_v);
        let _ = imp.write_u64_with_offset(63 * 8, _call_void_method_a);
        let _ = imp.write_u64_with_offset(64 * 8, _call_nonvirtual_object_method);
        let _ = imp.write_u64_with_offset(65 * 8, _call_nonvirtual_object_method_v);
        let _ = imp.write_u64_with_offset(66 * 8, _call_nonvirtual_object_method_a);
        let _ = imp.write_u64_with_offset(67 * 8, _call_nonvirtual_boolean_method);
        let _ = imp.write_u64_with_offset(68 * 8, _call_nonvirtual_boolean_method_v);
        let _ = imp.write_u64_with_offset(69 * 8, _call_nonvirtual_boolean_method_a);
        let _ = imp.write_u64_with_offset(70 * 8, _call_nonvirtual_byte_method);
        let _ = imp.write_u64_with_offset(71 * 8, _call_nonvirtual_byte_method_v);
        let _ = imp.write_u64_with_offset(72 * 8, _call_nonvirtual_byte_method_a);
        let _ = imp.write_u64_with_offset(73 * 8, _call_nonvirtual_char_method);
        let _ = imp.write_u64_with_offset(74 * 8, _call_nonvirtual_char_method_v);
        let _ = imp.write_u64_with_offset(75 * 8, _call_nonvirtual_char_method_a);
        let _ = imp.write_u64_with_offset(76 * 8, _call_nonvirtual_short_method);
        let _ = imp.write_u64_with_offset(77 * 8, _call_nonvirtual_short_method_v);
        let _ = imp.write_u64_with_offset(78 * 8, _call_nonvirtual_short_method_a);
        let _ = imp.write_u64_with_offset(79 * 8, _call_nonvirtual_int_method);
        let _ = imp.write_u64_with_offset(80 * 8, _call_nonvirtual_int_method_v);
        let _ = imp.write_u64_with_offset(81 * 8, _call_nonvirtual_int_method_a);
        let _ = imp.write_u64_with_offset(82 * 8, _call_nonvirtual_long_method);
        let _ = imp.write_u64_with_offset(83 * 8, _call_nonvirtual_long_method_v);
        let _ = imp.write_u64_with_offset(84 * 8, _call_nonvirtual_long_method_a);
        let _ = imp.write_u64_with_offset(85 * 8, _call_nonvirtual_float_method);
        let _ = imp.write_u64_with_offset(86 * 8, _call_nonvirtual_float_method_v);
        let _ = imp.write_u64_with_offset(87 * 8, _call_nonvirtual_float_method_a);
        let _ = imp.write_u64_with_offset(88 * 8, _call_nonvirtual_double_method);
        let _ = imp.write_u64_with_offset(89 * 8, _call_nonvirtual_double_method_v);
        let _ = imp.write_u64_with_offset(90 * 8, _call_nonvirtual_double_method_a);
        let _ = imp.write_u64_with_offset(91 * 8, _call_nonvirtual_void_method);
        let _ = imp.write_u64_with_offset(92 * 8, _call_nonvirtual_void_method_v);
        let _ = imp.write_u64_with_offset(93 * 8, _call_nonvirtual_void_method_a);
        let _ = imp.write_u64_with_offset(94 * 8, _get_field_id);
        let _ = imp.write_u64_with_offset(95 * 8, _get_object_field);
        let _ = imp.write_u64_with_offset(96 * 8, _get_boolean_field);
        let _ = imp.write_u64_with_offset(97 * 8, _get_byte_field);
        let _ = imp.write_u64_with_offset(98 * 8, _get_char_field);
        let _ = imp.write_u64_with_offset(99 * 8, _get_short_field);
        let _ = imp.write_u64_with_offset(100 * 8, _get_int_field);
        let _ = imp.write_u64_with_offset(101 * 8, _get_long_field);
        let _ = imp.write_u64_with_offset(102 * 8, _get_float_field);
        let _ = imp.write_u64_with_offset(103 * 8, _get_double_field);
        let _ = imp.write_u64_with_offset(104 * 8, _set_object_field);
        let _ = imp.write_u64_with_offset(105 * 8, _set_boolean_field);
        let _ = imp.write_u64_with_offset(106 * 8, _set_byte_field);
        let _ = imp.write_u64_with_offset(107 * 8, _set_char_field);
        let _ = imp.write_u64_with_offset(108 * 8, _set_short_field);
        let _ = imp.write_u64_with_offset(109 * 8, _set_int_field);
        let _ = imp.write_u64_with_offset(110 * 8, _set_long_field);
        let _ = imp.write_u64_with_offset(111 * 8, _set_float_field);
        let _ = imp.write_u64_with_offset(112 * 8, _set_double_field);
        let _ = imp.write_u64_with_offset(113 * 8, _get_static_method_id);
        let _ = imp.write_u64_with_offset(114 * 8, _call_static_object_method);
        let _ = imp.write_u64_with_offset(115 * 8, _call_static_object_method_v);
        let _ = imp.write_u64_with_offset(116 * 8, _call_static_object_method_a);
        let _ = imp.write_u64_with_offset(117 * 8, _call_static_boolean_method);
        let _ = imp.write_u64_with_offset(118 * 8, _call_static_boolean_method_v);
        let _ = imp.write_u64_with_offset(119 * 8, _call_static_boolean_method_a);
        let _ = imp.write_u64_with_offset(120 * 8, _call_static_byte_method);
        let _ = imp.write_u64_with_offset(121 * 8, _call_static_byte_method_v);
        let _ = imp.write_u64_with_offset(122 * 8, _call_static_byte_method_a);
        let _ = imp.write_u64_with_offset(123 * 8, _call_static_char_method);
        let _ = imp.write_u64_with_offset(124 * 8, _call_static_char_method_v);
        let _ = imp.write_u64_with_offset(125 * 8, _call_static_char_method_a);
        let _ = imp.write_u64_with_offset(126 * 8, _call_static_short_method);
        let _ = imp.write_u64_with_offset(127 * 8, _call_static_short_method_v);
        let _ = imp.write_u64_with_offset(128 * 8, _call_static_short_method_a);
        let _ = imp.write_u64_with_offset(129 * 8, _call_static_int_method);
        let _ = imp.write_u64_with_offset(130 * 8, _call_static_int_method_v);
        let _ = imp.write_u64_with_offset(131 * 8, _call_static_int_method_a);
        let _ = imp.write_u64_with_offset(132 * 8, _call_static_long_method);
        let _ = imp.write_u64_with_offset(133 * 8, _call_static_long_method_v);
        let _ = imp.write_u64_with_offset(134 * 8, _call_static_long_method_a);
        let _ = imp.write_u64_with_offset(135 * 8, _call_static_float_method);
        let _ = imp.write_u64_with_offset(136 * 8, _call_static_float_method_v);
        let _ = imp.write_u64_with_offset(137 * 8, _call_static_float_method_a);
        let _ = imp.write_u64_with_offset(138 * 8, _call_static_double_method);
        let _ = imp.write_u64_with_offset(139 * 8, _call_static_double_method_v);
        let _ = imp.write_u64_with_offset(140 * 8, _call_static_double_method_a);
        let _ = imp.write_u64_with_offset(141 * 8, _call_static_void_method);
        let _ = imp.write_u64_with_offset(142 * 8, _call_static_void_method_v);
        let _ = imp.write_u64_with_offset(143 * 8, _call_static_void_method_a);
        let _ = imp.write_u64_with_offset(144 * 8, _get_static_field_id);
        let _ = imp.write_u64_with_offset(145 * 8, _get_static_object_field);
        let _ = imp.write_u64_with_offset(146 * 8, _get_static_boolean_field);
        let _ = imp.write_u64_with_offset(147 * 8, _get_static_byte_field);
        let _ = imp.write_u64_with_offset(148 * 8, _get_static_char_field);
        let _ = imp.write_u64_with_offset(149 * 8, _get_static_short_field);
        let _ = imp.write_u64_with_offset(150 * 8, _get_static_int_field);
        let _ = imp.write_u64_with_offset(151 * 8, _get_static_long_field);
        let _ = imp.write_u64_with_offset(152 * 8, _get_static_float_field);
        let _ = imp.write_u64_with_offset(153 * 8, _get_static_double_field);
        let _ = imp.write_u64_with_offset(154 * 8, _set_static_object_field);
        let _ = imp.write_u64_with_offset(155 * 8, _set_static_boolean_field);
        let _ = imp.write_u64_with_offset(156 * 8, _set_static_byte_field);
        let _ = imp.write_u64_with_offset(157 * 8, _set_static_char_field);
        let _ = imp.write_u64_with_offset(158 * 8, _set_static_short_field);
        let _ = imp.write_u64_with_offset(159 * 8, _set_static_int_field);
        let _ = imp.write_u64_with_offset(160 * 8, _set_static_long_field);
        let _ = imp.write_u64_with_offset(161 * 8, _set_static_float_field);
        let _ = imp.write_u64_with_offset(162 * 8, _set_static_double_field);
        let _ = imp.write_u64_with_offset(163 * 8, _new_string);
        let _ = imp.write_u64_with_offset(164 * 8, _get_string_length);
        let _ = imp.write_u64_with_offset(165 * 8, _get_string_chars);
        let _ = imp.write_u64_with_offset(166 * 8, _release_string_chars);
        let _ = imp.write_u64_with_offset(167 * 8, _new_string_utf);
        let _ = imp.write_u64_with_offset(168 * 8, _get_string_utf_length);
        let _ = imp.write_u64_with_offset(169 * 8, _get_string_utf_chars);
        let _ = imp.write_u64_with_offset(170 * 8, _release_string_utf_chars);
        let _ = imp.write_u64_with_offset(171 * 8, _get_array_length);
        let _ = imp.write_u64_with_offset(172 * 8, _new_object_array);
        let _ = imp.write_u64_with_offset(173 * 8, _get_object_array_element);
        let _ = imp.write_u64_with_offset(174 * 8, _set_object_array_element);
        let _ = imp.write_u64_with_offset(175 * 8, _new_boolean_array);
        let _ = imp.write_u64_with_offset(176 * 8, _new_byte_array);
        let _ = imp.write_u64_with_offset(177 * 8, _new_char_array);
        let _ = imp.write_u64_with_offset(178 * 8, _new_short_array);
        let _ = imp.write_u64_with_offset(179 * 8, _new_int_array);
        let _ = imp.write_u64_with_offset(180 * 8, _new_long_array);
        let _ = imp.write_u64_with_offset(181 * 8, _new_float_array);
        let _ = imp.write_u64_with_offset(182 * 8, _new_double_array);
        let _ = imp.write_u64_with_offset(183 * 8, _get_boolean_array_elements);
        let _ = imp.write_u64_with_offset(184 * 8, _get_byte_array_elements);
        let _ = imp.write_u64_with_offset(185 * 8, _get_char_array_elements);
        let _ = imp.write_u64_with_offset(186 * 8, _get_short_array_elements);
        let _ = imp.write_u64_with_offset(187 * 8, _get_int_array_elements);
        let _ = imp.write_u64_with_offset(188 * 8, _get_long_array_elements);
        let _ = imp.write_u64_with_offset(189 * 8, _get_float_array_elements);
        let _ = imp.write_u64_with_offset(190 * 8, _get_double_array_elements);
        let _ = imp.write_u64_with_offset(191 * 8, _release_boolean_array_elements);
        let _ = imp.write_u64_with_offset(192 * 8, _release_byte_array_elements);
        let _ = imp.write_u64_with_offset(193 * 8, _release_char_array_elements);
        let _ = imp.write_u64_with_offset(194 * 8, _release_short_array_elements);
        let _ = imp.write_u64_with_offset(195 * 8, _release_int_array_elements);
        let _ = imp.write_u64_with_offset(196 * 8, _release_long_array_elements);
        let _ = imp.write_u64_with_offset(197 * 8, _release_float_array_elements);
        let _ = imp.write_u64_with_offset(198 * 8, _release_double_array_elements);
        let _ = imp.write_u64_with_offset(199 * 8, _get_boolean_array_region);
        let _ = imp.write_u64_with_offset(200 * 8, _get_byte_array_region);
        let _ = imp.write_u64_with_offset(201 * 8, _get_char_array_region);
        let _ = imp.write_u64_with_offset(202 * 8, _get_short_array_region);
        let _ = imp.write_u64_with_offset(203 * 8, _get_int_array_region);
        let _ = imp.write_u64_with_offset(204 * 8, _get_long_array_region);
        let _ = imp.write_u64_with_offset(205 * 8, _get_float_array_region);
        let _ = imp.write_u64_with_offset(206 * 8, _get_double_array_region);
        let _ = imp.write_u64_with_offset(207 * 8, _set_boolean_array_region);
        let _ = imp.write_u64_with_offset(208 * 8, _set_byte_array_region);
        let _ = imp.write_u64_with_offset(209 * 8, _set_char_array_region);
        let _ = imp.write_u64_with_offset(210 * 8, _set_short_array_region);
        let _ = imp.write_u64_with_offset(211 * 8, _set_int_array_region);
        let _ = imp.write_u64_with_offset(212 * 8, _set_long_array_region);
        let _ = imp.write_u64_with_offset(213 * 8, _set_float_array_region);
        let _ = imp.write_u64_with_offset(214 * 8, _set_double_array_region);
        let _ = imp.write_u64_with_offset(215 * 8, _register_natives);
        let _ = imp.write_u64_with_offset(216 * 8, _unregister_natives);
        let _ = imp.write_u64_with_offset(217 * 8, _monitor_enter);
        let _ = imp.write_u64_with_offset(218 * 8, _monitor_exit);
        let _ = imp.write_u64_with_offset(219 * 8, _get_java_vm);
        let _ = imp.write_u64_with_offset(220 * 8, _get_string_region);
        let _ = imp.write_u64_with_offset(221 * 8, _get_string_utf_region);
        let _ = imp.write_u64_with_offset(222 * 8, _get_primitive_array_critical);
        let _ = imp.write_u64_with_offset(223 * 8, _release_primitive_array_critical);
        let _ = imp.write_u64_with_offset(224 * 8, _get_string_critical);
        let _ = imp.write_u64_with_offset(225 * 8, _release_string_critical);
        let _ = imp.write_u64_with_offset(226 * 8, _new_weak_global_ref);
        let _ = imp.write_u64_with_offset(227 * 8, _delete_weak_global_ref);
        let _ = imp.write_u64_with_offset(228 * 8, _exception_check);
        let _ = imp.write_u64_with_offset(229 * 8, _new_direct_byte_buffer);
        let _ = imp.write_u64_with_offset(230 * 8, _get_direct_buffer_address);
        let _ = imp.write_u64_with_offset(231 * 8, _get_direct_buffer_capacity);
        let _ = imp.write_u64_with_offset(232 * 8, _get_object_ref_type);
    }

    java_env.write_u64_with_offset(0, imp.addr).expect("write_u64_with_offset failed: _JNIEnv.impl");

    java_env.addr
}