use crate::android::dvm::{*};
use crate::android::jni::JniValue;
use crate::dalvik;
use crate::memory::svc_memory::{SimpleArm64Svc, SvcMemory};

macro_rules! env {
    ($emulator:expr) => {
        $emulator.backend.reg_read(RegisterARM64::X0).unwrap()
    }
}

pub fn initialize_env<T: Clone>(svc_memory: &mut SvcMemory<T>) -> u64 {
    let java_env = svc_memory.allocate(8, "_JNIEnv");
    let imp = svc_memory.allocate(0x748 + 8, "_JNIEnv.impl");

    let _get_version = svc_memory.register_svc(SimpleArm64Svc::new("_GetVersion", |emulator| {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            println!("JNI: GetVersion() => JNI_VERSION_1_6");
        }
        Ok(Some(JNI_VERSION_1_6))
    }));
    let _define_class = svc_memory.register_svc(SimpleArm64Svc::new("_DefineClass", |emulator| {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            println!("{} {}() => JNI_OK", Color::Yellow.paint("JNI:"), Color::Blue.paint("DefineClass"));
        }
        Ok(Some(JNI_OK))
    }));
    let _find_class = svc_memory.register_svc(SimpleArm64Svc::new("_FindClass", |emulator| {
        let class_name_ptr = emulator.backend.reg_read(RegisterARM64::X1).unwrap();
        let class_name_str = emulator.backend.mem_read_c_string(class_name_ptr).unwrap();

        if let Some(vm) = emulator.inner_mut().dalvik.as_ref() {
            let result = vm.resolve_class(&class_name_str);
            if let Some((id, _)) = result {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(name = {}) => 0x{:x}", Color::Yellow.paint("JNI:"), Color::Blue.paint("FindClass"), Color::Green.paint(class_name_str), id);
                }
                return Ok(Some(id))
            } else {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    //emulator.backend.dump_context(0, 0);
                    println!("{} {}(name = {}) => NoClassDefFoundError from 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("FindClass"), Color::Green.paint(class_name_str), emulator.get_lr().unwrap());
                }
            }
        } else {
            warn!("Dalvik vm not initialized: Jni::FindClass");
        }
        if let Some(vm) = emulator.inner_mut().dalvik.as_mut() {
            vm.throw(vm.resolve_class("java/lang/NoClassDefFoundError").unwrap().1.new_simple_instance(dalvik!(emulator)));
        }
        Ok(Some(JNI_NULL))
    }));
    let _from_reflected_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("FromReflectedMethod not implemented");
    }));
    let _from_reflected_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("FromReflectedField not implemented");
    }));
    let _to_reflected_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("ToReflectedMethod not implemented");
    }));
    let _get_superclass = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetSuperclass not implemented");
    }));
    let _is_assignable_from = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("IsAssignableFrom not implemented");
    }));
    let _to_reflected_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("ToReflectedField not implemented");
    }));
    let _throw = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("Throw not implemented");
    }));
    let _throw_new = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("ThrowNew not implemented");
    }));
    let _exception_occurred = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("ExceptionOccurred not implemented");
    }));
    let _exception_describe = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("ExceptionDescribe not implemented");
    }));
    let _exception_clear = svc_memory.register_svc(SimpleArm64Svc::new("ExceptionClear", |emulator| {
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            println!("{} {}() => JNI_OK", Color::Yellow.paint("JNI:"), Color::Blue.paint("ExceptionClear"));
        }
        let dvm = dalvik!(emulator);
        dvm.throwable = None;
        Ok(Some(JNI_OK))
    }));
    let _fatal_error = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("FatalError not implemented");
    }));
    let _push_local_frame = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("PushLocalFrame not implemented");
    }));
    let _pop_local_frame = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("PopLocalFrame not implemented");
    }));
    let _new_global_ref = svc_memory.register_svc(SimpleArm64Svc::new("_NewGlobalRef", |emulator| {
        let object = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;

        let flag = jni::get_flag_id(object);
        if flag != JNI_FLAG_CLASS && flag != JNI_FLAG_REF && flag != JNI_FLAG_OBJECT {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(object = 0x{:X}) => UnsupportedObject", Color::Yellow.paint("JNI:"), Color::Blue.paint("NewGlobalRef"), object);
            }
            return Ok(Some(JNI_NULL))
        }

        let dvm = dalvik!(emulator);
        if flag == JNI_FLAG_CLASS {
            let class = dvm.find_class_by_id(&object).unwrap().1;
            let dvm_object = DvmObject::Class(class);
            let ref_object_id = dvm.add_global_ref(dvm_object);
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(object = 0x{:X}) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("NewGlobalRef"), object, ref_object_id);
            }
            return Ok(Some(ref_object_id))
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
            return Ok(Some(object))
        }

        if flag == JNI_FLAG_OBJECT {
            let object_ = dvm.get_local_ref(object);
            if object_.is_none() {
                return Ok(Some(JNI_ERR))
            }
            let ref_object_id = dvm.add_global_ref(object_.unwrap().clone());
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(object = 0x{:X}) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("NewGlobalRef"), object, ref_object_id);
            }
            return Ok(Some(ref_object_id))
        }

        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            println!("{} {}(object = 0x{:X}) => JNI_NULL", Color::Yellow.paint("JNI:"), Color::Blue.paint("NewGlobalRef"), object);
        }

        return Ok(Some(JNI_NULL))
    }));
    let _delete_global_ref = svc_memory.register_svc(SimpleArm64Svc::new("_DeleteGlobalRef", |emulator| {
        let object = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            println!("{} {}(object = 0x{:X}) => JNI_OK", Color::Yellow.paint("JNI:"), Color::Blue.paint("DeleteGlobalRef"), object);
        }

        let dvm = dalvik!(emulator);
        dvm.remove_global_ref(object);

        Ok(Some(JNI_OK))
    }));
    let _delete_local_ref = svc_memory.register_svc(SimpleArm64Svc::new("_DeleteLocalRef", |emulator| {
        let object = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;

        if object == 0 {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(object = 0x{:X}) => JNI_OK", Color::Yellow.paint("JNI:"), Color::Blue.paint("DeleteLocalRef"), object);
            }
            return Ok(Some(JNI_OK))
        }
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            println!("{} {}(object = 0x{:X}) => JNI_OK", Color::Yellow.paint("JNI:"), Color::Blue.paint("DeleteLocalRef"), object);
        }

        let dvm = dalvik!(emulator);
        dvm.remove_local_ref(object);

        Ok(Some(JNI_OK))
    }));
    let _is_same_object = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("IsSameObject not implemented");
    }));
    let _new_local_ref = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("NewLocalRef not implemented");
    }));
    let _ensure_local_capacity = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("EnsureLocalCapacity not implemented");
    }));
    let _alloc_object = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("AllocObject not implemented");
    }));
    let _new_object = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("NewObject not implemented");
    }));
    let _new_object_v = svc_memory.register_svc(SimpleArm64Svc::new("_NewObjectV", |emulator| {
        let dvm = dalvik!(emulator);
        let mut clz_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
        let method_id = emulator.backend.reg_read(RegisterARM64::X2).unwrap() as i64;
        let args = emulator.backend.reg_read(RegisterARM64::X3).unwrap();

        if jni::get_flag_id(clz_id) == JNI_FLAG_REF {
            let object = dvm.get_global_ref(clz_id);
            if object.is_none() {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}, args = 0x{:x}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("NewObjectV"), env!(emulator), clz_id, method_id, args);
                }
                return Ok(Some(JNI_NULL))
            }
            let object = object.unwrap();
            if let DvmObject::Class(class) = object {
                clz_id = class.id;
            } else {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}, args = 0x{:x}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("NewObjectV"), env!(emulator), clz_id, method_id, args);
                }
                return Ok(Some(JNI_NULL))
            }
        }

        let clz = dvm.find_class_by_id(&clz_id).map(|(_, clz)| clz);
        if clz.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}, args = 0x{:x}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("NewObjectV"), env!(emulator), clz_id, method_id, args);
            }
            return Ok(Some(JNI_NULL))
        }
        let class = clz.unwrap();
        let method = dvm.find_method_by_id(clz_id, method_id);
        if method.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => MethodNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("NewObjectV"), env!(emulator), clz_id, method_id);
            }
            return Ok(Some(JNI_NULL));
        }
        let method = method.unwrap();
        let mut va_list = VaList::new(emulator, args);

        let result = dalvik!(emulator).jni.as_mut().expect("JNI not register")
            .call_method_v(dalvik!(emulator), MethodAcc::CONSTRUCTOR | MethodAcc::OBJECT, &class, method, None, &mut va_list);

        if result.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => JNI_NULL", Color::Yellow.paint("JNI:"), Color::Blue.paint("NewObjectV"), env!(emulator), clz_id, method_id);
            }
            return Ok(Some(JNI_NULL))
        } else {
            let object = result.into();
            let object_id = dvm.add_local_ref(object);
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("NewObjectV"), env!(emulator), clz_id, method_id, object_id);
            }
            Ok(Some(object_id))
        }
    }));
    let _new_object_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("NewObjectA not implemented");
    }));
    let _get_object_class = svc_memory.register_svc(SimpleArm64Svc::new("_GetObjectClass", |emulator| {
        let dvm = dalvik!(emulator);
        let object_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;

        let flag = jni::get_flag_id(object_id);
        if flag != JNI_FLAG_REF && flag != JNI_FLAG_OBJECT {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(object = 0x{:X}) => UnsupportedObject", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetObjectClass"), object_id);
            }
            return Ok(Some(JNI_NULL))
        }

        let object = if flag == JNI_FLAG_REF {
            dvm.get_global_ref(object_id)
        } else {
            dvm.get_local_ref(object_id)
        };
        if object.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(object = 0x{:X}) => ObjectNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetObjectClass"), object_id);
            }
            return Ok(Some(JNI_NULL))
        }

        let object = object.unwrap();
        let id = object.get_class(dvm).id;

        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            println!("{} {}(object = 0x{:X}) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetObjectClass"), object_id, id);
        }

        Ok(Some(id))
    }));
    let _is_instance_of = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("IsInstanceOf not implemented");
    }));
    let _get_method_id = svc_memory.register_svc(SimpleArm64Svc::new("_GetMethodID", |emulator| {
        let dvm = dalvik!(emulator);
        let mut clz_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
        let method_name = emulator.backend.mem_read_c_string(emulator.backend.reg_read(RegisterARM64::X2).unwrap()).unwrap();
        let signature = emulator.backend.mem_read_c_string(emulator.backend.reg_read(RegisterARM64::X3).unwrap()).unwrap();

        if jni::get_flag_id(clz_id) == JNI_FLAG_REF {
            let object = dvm.get_global_ref(clz_id);
            if object.is_none() {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetMethodID"), env!(emulator), clz_id, method_name, signature);
                }
                return Ok(Some(JNI_NULL));
            }
            let object = object.unwrap();
            if let DvmObject::Class(class) = object {
                clz_id = class.id;
            } else {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetMethodID"), env!(emulator), clz_id, method_name, signature);
                }
                return Ok(Some(JNI_NULL));
            }
        }

        let clz = dvm.find_class_by_id(&clz_id).map(|(_, clz)| clz);
        if clz.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetMethodID"), env!(emulator), clz_id, method_name, signature);
            }
            return Ok(Some(JNI_NULL));
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
                return Ok(Some(JNI_ERR));
            }
            if !dvm.members.contains_key(&clz_id) {
                dvm.members.insert(clz_id, Vec::new());
            }

            let member_list = dvm.members.get_mut(&clz_id).unwrap();
            for member in member_list.iter() {
                if let DvmMember::Method(method) = member {
                    if method.class == clz_id && method.name == method_name && method.signature == signature {
                        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                            println!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}, cached = true) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetMethodID"), env!(emulator), clz_id, method_name, signature, method.id);
                        }
                        return Ok(Some(method.id))
                    }
                }
            }

            let id = member_list.len() as i64 + 1 + clz_id;
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}, cached = false) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetMethodID"), env!(emulator), clz_id, method_name, signature, id);
            }
            member_list.push(DvmMember::Method(DvmMethod::new(id, clz_id, method_name, signature, 0)));

            return Ok(Some(id))
        } else {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}) => NoSuchMethodError", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetMethodID"), env!(emulator), clz_id, method_name, signature);
            }
            dvm.throw(dvm.resolve_class("java/lang/NoSuchMethodError").unwrap().1.new_simple_instance(dalvik!(emulator)))
        }

        Ok(Some(JNI_NULL))
    }));
    let _call_object_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallObjectMethod not implemented");
    }));
    let _call_object_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallObjectMethodV", |emulator| {
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
                println!("{} {}(env = 0x{:x}, object = 0x{:x}, method = 0x{:x}) => ObjectNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallObjectMethodV"), env!(emulator), object_id, method_id);
            }
            return Ok(Some(JNI_NULL));
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
            return Ok(Some(JNI_NULL))
        } else if result.is_void() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, object = 0x{:x}, method = 0x{:x}) => JNI_NULL", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallObjectMethodV"), env!(emulator), object_id, method_id);
            }
            return Ok(Some(JNI_NULL))
        }

        let result: DvmObject = result.into();
        let result_id = dvm.add_local_ref(result);
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            println!("{} {}(env = 0x{:x}, object = 0x{:x}, method = 0x{:x}) => 0x{:x}", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallObjectMethodV"), env!(emulator), object_id, method_id, result_id);
        }

        Ok(Some(result_id))
    }));
    let _call_object_method_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallObjectMethodA not implemented");
    }));
    let _call_boolean_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallBooleanMethod not implemented");
    }));
    let _call_boolean_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallBooleanMethodV", |emulator| {
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
                println!("{} {}(env = 0x{:x}, object = 0x{:x}, method = 0x{:x}) => ObjectNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallBooleanMethodV"), env!(emulator), object_id, method_id);
            }
            return Ok(Some(JNI_NULL));
        }

        let instance = instance.unwrap();
        let result = {
            let dvm_ref = dalvik!(emulator);
            let class = instance.get_class(dvm_ref);
            let method = dvm_ref.find_method_by_id(class.id, method_id).unwrap();
            dalvik!(emulator).jni.as_mut().expect("JNI not register")
                .call_method_v(dalvik!(emulator), MethodAcc::BOOLEAN, &class, method, Some(instance), &mut VaList::new(emulator, args))
        };

        return if result.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, object = 0x{:x}, method = 0x{:x}) => JNI_NULL", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallBooleanMethodV"), env!(emulator), object_id, method_id);
            }
            Ok(Some(JNI_NULL))
        } else {
            let result = result.into();
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, object = 0x{:x}, method = 0x{:x}) => {}", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallBooleanMethodV"), env!(emulator), object_id, method_id, result);
            }
            Ok(Some(result))
        }
    }));
    let _call_boolean_method_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallBooleanMethodA not implemented");
    }));
    let _call_byte_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallByteMethod not implemented");
    }));
    let _call_byte_method_v = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallByteMethodV not implemented");
    }));
    let _call_byte_method_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallByteMethodA not implemented");
    }));
    let _call_char_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallCharMethod not implemented");
    }));
    let _call_char_method_v = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallCharMethodV not implemented");
    }));
    let _call_char_method_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallCharMethodA not implemented");
    }));
    let _call_short_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallShortMethod not implemented");
    }));
    let _call_short_method_v = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallShortMethodV not implemented");
    }));
    let _call_short_method_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallShortMethodA not implemented");
    }));
    let _call_int_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallIntMethod not implemented");
    }));
    let _call_int_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallIntMethodV", |emulator| {
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
                println!("{} {}(env = 0x{:x}, object = 0x{:x}, method = 0x{:x}) => ObjectNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallIntMethodV"), env!(emulator), object_id, method_id);
            }
            return Ok(Some(JNI_NULL));
        }

        let instance = instance.unwrap();
        let result = {
            let dvm_ref = dalvik!(emulator);
            let class = instance.get_class(dvm_ref);
            let method = dvm_ref.find_method_by_id(class.id, method_id).unwrap();
            dalvik!(emulator).jni.as_mut().expect("JNI not register")
                .call_method_v(dalvik!(emulator), MethodAcc::INT, &class, method, Some(instance), &mut VaList::new(emulator, args))
        };

        return if result.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, object = 0x{:x}, method = 0x{:x}) => JNI_NULL", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallIntMethodV"), env!(emulator), object_id, method_id);
            }
            Ok(Some(JNI_NULL))
        } else {
            let result = result.into();
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, object = 0x{:x}, method = 0x{:x}) => 0x{:x}", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallIntMethodV"), env!(emulator), object_id, method_id, result);
            }
            Ok(Some(result))
        }
    }));
    let _call_int_method_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallIntMethodA not implemented");
    }));
    let _call_long_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallLongMethod not implemented");
    }));
    let _call_long_method_v = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallLongMethodV not implemented");
    }));
    let _call_long_method_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallLongMethodA not implemented");
    }));
    let _call_float_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallFloatMethod not implemented");
    }));
    let _call_float_method_v = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallFloatMethodV not implemented");
    }));
    let _call_float_method_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallFloatMethodA not implemented");
    }));
    let _call_double_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallDoubleMethod not implemented");
    }));
    let _call_double_method_v = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallDoubleMethodV not implemented");
    }));
    let _call_double_method_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallDoubleMethodA not implemented");
    }));
    let _call_void_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallVoidMethod not implemented");
    }));
    let _call_void_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallVoidMethodV", |emulator| {
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
                println!("{} {}(env = 0x{:x}, object = 0x{:x}, method = 0x{:x}) => ObjectNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallVoidMethodV"), env!(emulator), object_id, method_id);
            }
            return Ok(Some(JNI_NULL));
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
                println!("{} {}(env = 0x{:x}, object = 0x{:x}, method = 0x{:x}) => JNI_NULL", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallVoidMethodV"), env!(emulator), object_id, method_id);
            }
            Ok(Some(JNI_NULL))
        } else {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, object = 0x{:x}, method = 0x{:x}) => JNI_OK", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallVoidMethodV"), env!(emulator), object_id, method_id);
            }
            Ok(Some(JNI_OK))
        }
    }));
    let _call_void_method_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallVoidMethodA not implemented");
    }));
    let _call_nonvirtual_object_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallNonvirtualObjectMethod not implemented");
    }));
    let _call_nonvirtual_object_method_v = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallNonvirtualObjectMethodV not implemented");
    }));
    let _call_nonvirtual_object_method_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallNonvirtualObjectMethodA not implemented");
    }));
    let _call_nonvirtual_boolean_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallNonvirtualBooleanMethod not implemented");
    }));
    let _call_nonvirtual_boolean_method_v = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallNonvirtualBooleanMethodV not implemented");
    }));
    let _call_nonvirtual_boolean_method_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallNonvirtualBooleanMethodA not implemented");
    }));
    let _call_nonvirtual_byte_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallNonvirtualByteMethod not implemented");
    }));
    let _call_nonvirtual_byte_method_v = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallNonvirtualByteMethodV not implemented");
    }));
    let _call_nonvirtual_byte_method_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallNonvirtualByteMethodA not implemented");
    }));
    let _call_nonvirtual_char_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallNonvirtualCharMethod not implemented");
    }));
    let _call_nonvirtual_char_method_v = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallNonvirtualCharMethodV not implemented");
    }));
    let _call_nonvirtual_char_method_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallNonvirtualCharMethodA not implemented");
    }));
    let _call_nonvirtual_short_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallNonvirtualShortMethod not implemented");
    }));
    let _call_nonvirtual_short_method_v = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallNonvirtualShortMethodV not implemented");
    }));
    let _call_nonvirtual_short_method_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallNonvirtualShortMethodA not implemented");
    }));
    let _call_nonvirtual_int_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallNonvirtualIntMethod not implemented");
    }));
    let _call_nonvirtual_int_method_v = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallNonvirtualIntMethodV not implemented");
    }));
    let _call_nonvirtual_int_method_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallNonvirtualIntMethodA not implemented");
    }));
    let _call_nonvirtual_long_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallNonvirtualLongMethod not implemented");
    }));
    let _call_nonvirtual_long_method_v = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallNonvirtualLongMethodV not implemented");
    }));
    let _call_nonvirtual_long_method_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallNonvirtualLongMethodA not implemented");
    }));
    let _call_nonvirtual_float_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallNonvirtualFloatMethod not implemented");
    }));
    let _call_nonvirtual_float_method_v = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallNonvirtualFloatMethodV not implemented");
    }));
    let _call_nonvirtual_float_method_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallNonvirtualFloatMethodA not implemented");
    }));
    let _call_nonvirtual_double_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallNonvirtualDoubleMethod not implemented");
    }));
    let _call_nonvirtual_double_method_v = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallNonvirtualDoubleMethodV not implemented");
    }));
    let _call_nonvirtual_double_method_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallNonvirtualDoubleMethodA not implemented");
    }));
    let _call_nonvirtual_void_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallNonvirtualVoidMethod not implemented");
    }));
    let _call_nonvirtual_void_method_v = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallNonvirtualVoidMethodV not implemented");
    }));
    let _call_nonvirtual_void_method_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallNonvirtualVoidMethodA not implemented");
    }));
    let _get_field_id = svc_memory.register_svc(SimpleArm64Svc::new("_GetFieldID", |emulator| {
        let dvm = dalvik!(emulator);
        let mut clz_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
        let field_name = emulator.backend.mem_read_c_string(emulator.backend.reg_read(RegisterARM64::X2).unwrap()).unwrap();
        let signature = emulator.backend.mem_read_c_string(emulator.backend.reg_read(RegisterARM64::X3).unwrap()).unwrap();

        if jni::get_flag_id(clz_id) == JNI_FLAG_REF {
            let object = dvm.get_global_ref(clz_id);
            if object.is_none() {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, class = 0x{:x}, field_name = {}, signature = {}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetFieldID"), env!(emulator), clz_id, field_name, signature);
                }
                return Ok(Some(JNI_NULL))
            }
            let object = object.unwrap();
            if let DvmObject::Class(class) = object {
                clz_id = class.id;
            } else {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, class = 0x{:x}, field_name = {}, signature = {}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetFieldID"), env!(emulator), clz_id, field_name, signature);
                }
                return Ok(Some(JNI_NULL))
            }
        }

        let clz = dvm.find_class_by_id(&clz_id).map(|(_, clz)| clz);
        if clz.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetFieldID"), env!(emulator), clz_id, field_name, signature);
            }
            return Ok(Some(JNI_NULL));
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
                return Ok(Some(JNI_NULL));
            }
            if !dvm.members.contains_key(&clz_id) {
                dvm.members.insert(clz_id, Vec::new());
            }
            let member_list = dvm.members.get_mut(&clz_id).unwrap();
            for member in member_list.iter() {
                if let DvmMember::Field(field) = member {
                    if field.class == clz_id && field.name == field_name && field.signature == signature {
                        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                            println!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}, cached = true) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetFieldID"), env!(emulator), clz_id, field_name, signature, field.id);
                        }
                        return Ok(Some(field.id))
                    }
                }
            }

            let id = member_list.len() as i64 + 1 + clz_id;
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}, cached = false) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetFieldID"), env!(emulator), clz_id, field_name, signature, id);
            }
            member_list.push(DvmMember::Field(DvmField::new(id, clz_id, field_name, signature)));

            return Ok(Some(id))
        } else {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}) => NoSuchFieldError", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetFieldID"), env!(emulator), clz_id, field_name, signature);
            }
            dvm.throw(dvm.resolve_class("java/lang/NoSuchFieldError").unwrap().1.new_simple_instance(dalvik!(emulator)))
        }

        Ok(Some(JNI_NULL))
    }));
    let _get_object_field = svc_memory.register_svc(SimpleArm64Svc::new("_GetObjectField", |emulator| {
        let dvm = dalvik!(emulator);
        let object_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
        let field_id = emulator.backend.reg_read(RegisterARM64::X2).unwrap() as i64;

        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            println!("{} {}(env = 0x{:x}, object = 0x{:x}, field = 0x{:x})", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetObjectField"), env!(emulator), object_id, field_id);
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
            JniValue::Null => return Ok(Some(JNI_NULL)),
            _ => unreachable!()
        };

        let value = dvm.add_local_ref(value);

        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            println!("{} {}(env = 0x{:x}, object = 0x{:x}, field = 0x{:x}) => 0x{:x}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetObjectField"), env!(emulator), object_id, field_id, value);
        }

        Ok(Some(value))
    }));
    let _get_boolean_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetBooleanField not implemented");
    }));
    let _get_byte_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetByteField not implemented");
    }));
    let _get_char_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetCharField not implemented");
    }));
    let _get_short_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetShortField not implemented");
    }));
    let _get_int_field = svc_memory.register_svc(SimpleArm64Svc::new("_GetIntField", |emulator| {
        let dvm = dalvik!(emulator);
        let object_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
        let field_id = emulator.backend.reg_read(RegisterARM64::X2).unwrap() as i64;

        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            println!("{} {}(env = 0x{:x}, object = 0x{:x}, field = 0x{:x})", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetIntField"), env!(emulator), object_id, field_id);
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
            println!("{} {}(env = 0x{:x}, object = 0x{:x}, field = 0x{:x}) => 0x{:x}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetIntField"), env!(emulator), object_id, field_id, value);
        }

        Ok(Some(value))
    }));
    let _get_long_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetLongField not implemented");
    }));
    let _get_float_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetFloatField not implemented");
    }));
    let _get_double_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetDoubleField not implemented");
    }));
    let _set_object_field = svc_memory.register_svc(SimpleArm64Svc::new("_SetObjectField", |emulator| {
        let dvm = dalvik!(emulator);
        let object_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
        let field_id = emulator.backend.reg_read(RegisterARM64::X2).unwrap() as i64;
        let value = emulator.backend.reg_read(RegisterARM64::X3).unwrap() as i64;

        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            println!("{} {}(env = 0x{:x}, object = 0x{:x}, field = 0x{:x}, value = 0x{:x})", Color::Yellow.paint("JNI:"), Color::Blue.paint("SetObjectField"), env!(emulator), object_id, field_id, value);
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

        Ok(None)
    }));
    let _set_boolean_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("SetBooleanField not implemented");
    }));
    let _set_byte_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("SetByteField not implemented");
    }));
    let _set_char_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("SetCharField not implemented");
    }));
    let _set_short_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("SetShortField not implemented");
    }));
    let _set_int_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("SetIntField not implemented");
    }));
    let _set_long_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("SetLongField not implemented");
    }));
    let _set_float_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("SetFloatField not implemented");
    }));
    let _set_double_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("SetDoubleField not implemented");
    }));
    let _get_static_method_id = svc_memory.register_svc(SimpleArm64Svc::new("_GetStaticMethodID", |emulator| {
        let dvm = dalvik!(emulator);
        let mut clz_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
        let method_name = emulator.backend.mem_read_c_string(emulator.backend.reg_read(RegisterARM64::X2).unwrap()).unwrap();
        let signature = emulator.backend.mem_read_c_string(emulator.backend.reg_read(RegisterARM64::X3).unwrap()).unwrap();

        if jni::get_flag_id(clz_id) == JNI_FLAG_REF {
            let object = dvm.get_global_ref(clz_id);
            if object.is_none() {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}) => GlobalRefNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticMethodID"), env!(emulator), clz_id, method_name, signature);
                }
                return Ok(Some(JNI_NULL))
            }
            let object = object.unwrap();
            match object {
                DvmObject::Class(class) => {
                    clz_id = class.id;
                }
                _ => {
                    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                        println!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}) => GlobalRefNotClass", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticMethodID"), env!(emulator), clz_id, method_name, signature);
                    }
                    return Ok(Some(JNI_NULL));
                }
            }
        }

        let clz = dvm.find_class_by_id(&clz_id).map(|(_, clz)| clz);
        if clz.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticMethodID"), env!(emulator), clz_id, method_name, signature);
            }
            return Ok(Some(JNI_NULL));
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
                return Ok(Some(JNI_NULL));
            }
            if !dvm.members.contains_key(&clz_id) {
                dvm.members.insert(clz_id, Vec::new());
            }

            let member_list = dvm.members.get_mut(&clz_id).unwrap();
            for member in member_list.iter() {
                if let DvmMember::Method(method) = member {
                    if method.class == clz_id && method.name == method_name && method.signature == signature {
                        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                            println!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}, cached = true) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticMethodID"), env!(emulator), clz_id, method_name, signature, method.id);
                        }
                        return Ok(Some(method.id))
                    }
                }
            }

            let id = member_list.len() as i64 + 1 + clz_id;
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}, cached = false) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticMethodID"), env!(emulator), clz_id, method_name, signature, id);
            }
            member_list.push(DvmMember::Method(DvmMethod::new(id, clz_id, method_name, signature, 0)));

            return Ok(Some(id))
        } else {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}) => NoSuchMethodError", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticMethodID"), env!(emulator), clz_id, method_name, signature);
            }
            dvm.throw(dvm.resolve_class("java/lang/NoSuchMethodError").unwrap().1.new_simple_instance(dalvik!(emulator)))
        }

        Ok(Some(JNI_NULL))
    }));
    let _call_static_object_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallStaticObjectMethod not implemented");
    }));
    let _call_static_object_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticObjectMethodV", |emulator| {
        // jobject     (*CallStaticObjectMethodV)(JNIEnv*, jclass, jmethodID, va_list);
        let dvm = dalvik!(emulator);
        let mut clz_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
        let method_id = emulator.backend.reg_read(RegisterARM64::X2).unwrap() as i64;
        let args = emulator.backend.reg_read(RegisterARM64::X3).unwrap();

        if jni::get_flag_id(clz_id) == JNI_FLAG_REF {
            let object = dvm.get_global_ref(clz_id);
            if object.is_none() {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticObjectMethodV"), env!(emulator), clz_id, method_id);
                }
                return Ok(Some(JNI_NULL))
            }
            let object = object.unwrap();
            match object {
                DvmObject::Class(class) => {
                    clz_id = class.id;
                }
                _ => {
                    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                        println!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => RefNotClass", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticObjectMethodV"), env!(emulator), clz_id, method_id);
                    }
                    return Ok(Some(JNI_NULL));
                }
            }
        }

        let class = dvm.find_class_by_id(&clz_id);
        if class.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticObjectMethodV"), env!(emulator), clz_id, method_id);
            }
            return Ok(Some(JNI_NULL));
        }
        let class = class.unwrap().1;
        let method = dvm.find_method_by_id(clz_id, method_id);
        if method.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => MethodNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticObjectMethodV"), env!(emulator), clz_id, method_id);
            }
            return Ok(Some(JNI_NULL));
        }
        let method = method.unwrap();

        let mut va_list = VaList::new(emulator, args);
        let result = dalvik!(emulator).jni.as_mut().expect("JNI not register")
            .call_method_v(dalvik!(emulator), MethodAcc::STATIC | MethodAcc::OBJECT, &class, method, None, &mut va_list);

        return if result.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => JNI_NULL", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticObjectMethodV"), env!(emulator), clz_id, method_id);
            }
            Ok(Some(JNI_NULL))
        } else {
            let object = result.into();
            let object_id = dvm.add_local_ref(object);
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticObjectMethodV"), env!(emulator), clz_id, method_id, object_id);
            }
            Ok(Some(object_id))
        }
    }));
    let _call_static_object_method_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallStaticObjectMethodA not implemented");
    }));
    let _call_static_boolean_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallStaticBooleanMethod not implemented");
    }));
    let _call_static_boolean_method_v = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallStaticBooleanMethodV not implemented");
    }));
    let _call_static_boolean_method_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallStaticBooleanMethodA not implemented");
    }));
    let _call_static_byte_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallStaticByteMethod not implemented");
    }));
    let _call_static_byte_method_v = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallStaticByteMethodV not implemented");
    }));
    let _call_static_byte_method_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallStaticByteMethodA not implemented");
    }));
    let _call_static_char_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallStaticCharMethod not implemented");
    }));
    let _call_static_char_method_v = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallStaticCharMethodV not implemented");
    }));
    let _call_static_char_method_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallStaticCharMethodA not implemented");
    }));
    let _call_static_short_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallStaticShortMethod not implemented");
    }));
    let _call_static_short_method_v = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallStaticShortMethodV not implemented");
    }));
    let _call_static_short_method_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallStaticShortMethodA not implemented");
    }));
    let _call_static_int_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallStaticIntMethod not implemented");
    }));
    let _call_static_int_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticIntMethodV", |emulator| {
        let dvm = dalvik!(emulator);
        let mut clz_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
        let method_id = emulator.backend.reg_read(RegisterARM64::X2).unwrap() as i64;
        let args = emulator.backend.reg_read(RegisterARM64::X3).unwrap();

        if jni::get_flag_id(clz_id) == JNI_FLAG_REF {
            let object = dvm.get_global_ref(clz_id);
            if object.is_none() {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticIntMethodV"), env!(emulator), clz_id, method_id);
                }
                return Ok(None)
            }
            let object = object.unwrap();
            match object {
                DvmObject::Class(class) => {
                    clz_id = class.id;
                }
                _ => {
                    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                        println!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => RefNotClass", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticIntMethodV"), env!(emulator), clz_id, method_id);
                    }
                    return Ok(None);
                }
            }
        }

        let class = dvm.find_class_by_id(&clz_id);
        if class.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticIntMethodV"), env!(emulator), clz_id, method_id);
            }
            return Ok(None);
        }

        let class = class.unwrap().1;
        let method = dvm.find_method_by_id(clz_id, method_id);
        if method.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => MethodNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticIntMethodV"), env!(emulator), clz_id, method_id);
            }
            return Ok(None);
        }

        let method = method.unwrap();
        let mut va_list = VaList::new(emulator, args);

        let result = dalvik!(emulator).jni.as_mut().expect("JNI not register")
            .call_method_v(dalvik!(emulator), MethodAcc::STATIC | MethodAcc::INT, &class, method, None, &mut va_list);

        if result.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => JNI_NULL", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticIntMethodV"), env!(emulator), clz_id, method_id);
            }
            return Ok(None);
        } else {
            let result = result.into();
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticIntMethodV"), env!(emulator), clz_id, method_id, result);
            }
            return Ok(Some(result));
        }
    }));
    let _call_static_int_method_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallStaticIntMethodA not implemented");
    }));
    let _call_static_long_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallStaticLongMethod not implemented");
    }));
    let _call_static_long_method_v = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallStaticLongMethodV not implemented");
    }));
    let _call_static_long_method_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallStaticLongMethodA not implemented");
    }));
    let _call_static_float_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallStaticFloatMethod not implemented");
    }));
    let _call_static_float_method_v = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallStaticFloatMethodV not implemented");
    }));
    let _call_static_float_method_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallStaticFloatMethodA not implemented");
    }));
    let _call_static_double_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallStaticDoubleMethod not implemented");
    }));
    let _call_static_double_method_v = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallStaticDoubleMethodV not implemented");
    }));
    let _call_static_double_method_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallStaticDoubleMethodA not implemented");
    }));
    let _call_static_void_method = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallStaticVoidMethod not implemented");
    }));
    let _call_static_void_method_v = svc_memory.register_svc(SimpleArm64Svc::new("_CallStaticVoidMethodV", |emulator| {
        let dvm = dalvik!(emulator);
        let mut clz_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
        let method_id = emulator.backend.reg_read(RegisterARM64::X2).unwrap() as i64;
        let args = emulator.backend.reg_read(RegisterARM64::X3).unwrap();

        if jni::get_flag_id(clz_id) == JNI_FLAG_REF {
            let object = dvm.get_global_ref(clz_id);
            if object.is_none() {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticVoidMethodV"), env!(emulator), clz_id, method_id);
                }
                return Ok(None)
            }
            let object = object.unwrap();
            match object {
                DvmObject::Class(class) => {
                    clz_id = class.id;
                }
                _ => {
                    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                        println!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => RefNotClass", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticVoidMethodV"), env!(emulator), clz_id, method_id);
                    }
                    return Ok(None);
                }
            }
        }

        let class = dvm.find_class_by_id(&clz_id);
        if class.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticVoidMethodV"), env!(emulator), clz_id, method_id);
            }
            return Ok(None);
        }

        let class = class.unwrap().1;
        let method = dvm.find_method_by_id(clz_id, method_id);
        if method.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x}) => MethodNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticVoidMethodV"), env!(emulator), clz_id, method_id);
            }
            return Ok(None);
        }

        let method = method.unwrap();
        let mut va_list = VaList::new(emulator, args);

        dalvik!(emulator).jni.as_mut().expect("JNI not register")
            .call_method_v(dalvik!(emulator), MethodAcc::STATIC | MethodAcc::VOID, &class, method, None, &mut va_list);

        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            println!("{} {}(env = 0x{:x}, class = 0x{:x}, method = 0x{:x})", Color::Yellow.paint("JNI:"), Color::Blue.paint("CallStaticVoidMethodV"), env!(emulator), clz_id, method_id);
        }

        Ok(None)
    }));
    let _call_static_void_method_a = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("CallStaticVoidMethodA not implemented");
    }));
    let _get_static_field_id = svc_memory.register_svc(SimpleArm64Svc::new("_GetStaticFieldID", |emulator| {
        let dvm = dalvik!(emulator);
        let mut clz_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
        let field_name = emulator.backend.mem_read_c_string(emulator.backend.reg_read(RegisterARM64::X2).unwrap()).unwrap();
        let signature = emulator.backend.mem_read_c_string(emulator.backend.reg_read(RegisterARM64::X3).unwrap()).unwrap();

        if jni::get_flag_id(clz_id) == JNI_FLAG_REF {
            let object = dvm.get_global_ref(clz_id);
            if object.is_none() {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, class = 0x{:x}, field_name = {}, signature = {}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticFieldID"), env!(emulator), clz_id, field_name, signature);
                }
                return Ok(Some(JNI_NULL))
            }
            let object = object.unwrap();
            if let DvmObject::Class(class) = object {
                clz_id = class.id;
            } else {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, class = 0x{:x}, field_name = {}, signature = {}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticFieldID"), env!(emulator), clz_id, field_name, signature);
                }
                return Ok(Some(JNI_NULL))
            }
        }

        let clz = dvm.find_class_by_id(&clz_id).map(|(_, clz)| clz);
        if clz.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticFieldID"), env!(emulator), clz_id, field_name, signature);
            }
            return Ok(Some(JNI_NULL));
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
                return Ok(Some(JNI_NULL));
            }
            if !dvm.members.contains_key(&clz_id) {
                dvm.members.insert(clz_id, Vec::new());
            }
            let member_list = dvm.members.get_mut(&clz_id).unwrap();
            for member in member_list.iter() {
                if let DvmMember::Field(field) = member {
                    if field.class == clz_id && field.name == field_name && field.signature == signature {
                        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                            println!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}, cached = true) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticFieldID"), env!(emulator), clz_id, field_name, signature, field.id);
                        }
                        return Ok(Some(field.id))
                    }
                }
            }

            let id = member_list.len() as i64 + 1 + clz_id;
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}, cached = false) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticFieldID"), env!(emulator), clz_id, field_name, signature, id);
            }
            member_list.push(DvmMember::Field(DvmField::new(id, clz_id, field_name, signature)));

            return Ok(Some(id))
        } else {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}) => NoSuchFieldError", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticFieldID"), env!(emulator), clz_id, field_name, signature);
            }
            dvm.throw(dvm.resolve_class("java/lang/NoSuchFieldError").unwrap().1.new_simple_instance(dalvik!(emulator)))
        }

        Ok(Some(JNI_NULL))
    }));
    let _get_static_object_field = svc_memory.register_svc(SimpleArm64Svc::new("_GetStaticObjectField", |emulator| {
        let dvm = dalvik!(emulator);
        let mut clz_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
        let field_id = emulator.backend.reg_read(RegisterARM64::X2).unwrap() as i64;

        if jni::get_flag_id(clz_id) == JNI_FLAG_REF {
            let object = dvm.get_global_ref(clz_id);
            if object.is_none() {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, class = 0x{:x}, field = 0x{:x}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticObjectField"), env!(emulator), clz_id, field_id);
                }
                return Ok(Some(JNI_NULL))
            }
            let object = object.unwrap();
            match object {
                DvmObject::Class(class) => {
                    clz_id = class.id;
                }
                _ => {
                    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                        println!("{} {}(env = 0x{:x}, class = 0x{:x}, field = 0x{:x}) => RefNotClass", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticObjectField"), env!(emulator), clz_id, field_id);
                    }
                    return Ok(Some(JNI_NULL));
                }
            }
        }

        let class = dvm.find_class_by_id(&clz_id);
        if class.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, field = 0x{:x}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticObjectField"), env!(emulator), clz_id, field_id);
            }
            return Ok(Some(JNI_NULL));
        }
        let class = class.unwrap().1;
        let field = dvm.find_field_by_id(clz_id, field_id);
        if field.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, field = 0x{:x}) => FieldNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticObjectField"), env!(emulator), clz_id, field_id);
            }
            return Ok(Some(JNI_NULL));
        }

        let field = field.unwrap();
        let object = dalvik!(emulator).jni.as_mut().expect("JNI not register")
            .get_field_value(dalvik!(emulator), &class, field, None);

        return if object.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, field = 0x{:x}) => JNI_NULL", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticObjectField"), env!(emulator), clz_id, field_id);
            }
            Ok(Some(JNI_NULL))
        } else {
            let object: DvmObject = object.into();
            let object_id = dvm.add_local_ref(object);
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, field = 0x{:x}) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticObjectField"), env!(emulator), clz_id, field_id, object_id);
            }
            Ok(Some(object_id))
        }
    }));
    let _get_static_boolean_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetStaticBooleanField not implemented");
    }));
    let _get_static_byte_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetStaticByteField not implemented");
    }));
    let _get_static_char_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetStaticCharField not implemented");
    }));
    let _get_static_short_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetStaticShortField not implemented");
    }));
    let _get_static_int_field = svc_memory.register_svc(SimpleArm64Svc::new("_GetStaticIntField", |emulator| {
        let dvm = dalvik!(emulator);
        let mut clz_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
        let field_id = emulator.backend.reg_read(RegisterARM64::X2).unwrap() as i64;

        if jni::get_flag_id(clz_id) == JNI_FLAG_REF {
            let object = dvm.get_global_ref(clz_id);
            if object.is_none() {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, class = 0x{:x}, field = 0x{:x}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticIntField"), env!(emulator), clz_id, field_id);
                }
                return Ok(Some(JNI_NULL))
            }
            let object = object.unwrap();
            match object {
                DvmObject::Class(class) => {
                    clz_id = class.id;
                }
                _ => {
                    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                        println!("{} {}(env = 0x{:x}, class = 0x{:x}, field = 0x{:x}) => RefNotClass", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticIntField"), env!(emulator), clz_id, field_id);
                    }
                    return Ok(Some(JNI_NULL));
                }
            }
        }

        let class = dvm.find_class_by_id(&clz_id);
        if class.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, field = 0x{:x}) => ClassNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticIntField"), env!(emulator), clz_id, field_id);
            }
            return Ok(Some(JNI_NULL));
        }
        let class = class.unwrap().1;
        let field = dvm.find_field_by_id(clz_id, field_id);
        if field.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env=0x{:x}, class=0x{:x}, field=0x{:x}) => FieldNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticIntField"), env!(emulator), clz_id, field_id);
            }
            return Ok(Some(JNI_NULL));
        }
        let field = field.unwrap();

        let value = dalvik!(emulator).jni.as_mut().expect("JNI not register")
            .get_field_value(dalvik!(emulator), &class, field, None);

        return if value.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env=0x{:x}, class=0x{:x}, field=0x{:x}) => JNI_NULL", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticIntField"), env!(emulator), clz_id, field_id);
            }
            Ok(Some(JNI_NULL))
        } else {
            let value = match value {
                JniValue::Int(value) => value as i64,
                JniValue::Long(value) => value,
                _ => {
                    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                        println!("{} {}(env=0x{:x}, class=0x{:x}, field=0x{:x}) => NotInt", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticIntField"), env!(emulator), clz_id, field_id);
                    }
                    return Ok(Some(JNI_NULL));
                }
            };

            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env=0x{:x}, class=0x{:x}, field=0x{:x}) => {}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStaticIntField"), env!(emulator), clz_id, field_id, value);
            }

            Ok(Some(value))
        }
    }));
    let _get_static_long_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetStaticLongField not implemented");
    }));
    let _get_static_float_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetStaticFloatField not implemented");
    }));
    let _get_static_double_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetStaticDoubleField not implemented");
    }));
    let _set_static_object_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("SetStaticObjectField not implemented");
    }));
    let _set_static_boolean_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("SetStaticBooleanField not implemented");
    }));
    let _set_static_byte_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("SetStaticByteField not implemented");
    }));
    let _set_static_char_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("SetStaticCharField not implemented");
    }));
    let _set_static_short_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("SetStaticShortField not implemented");
    }));
    let _set_static_int_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("SetStaticIntField not implemented");
    }));
    let _set_static_long_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("SetStaticLongField not implemented");
    }));
    let _set_static_float_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("SetStaticFloatField not implemented");
    }));
    let _set_static_double_field = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("SetStaticDoubleField not implemented");
    }));
    let _new_string = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("NewString not implemented");
    }));
    let _get_string_length = svc_memory.register_svc(SimpleArm64Svc::new("_GetStringLength", |emulator| {
        let dvm = dalvik!(emulator);

        let string = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
        let flag = jni::get_flag_id(string);
        let object = if flag == JNI_FLAG_OBJECT {
            dvm.get_local_ref(string)
        } else if flag == JNI_FLAG_REF {
            dvm.get_global_ref(string)
        } else {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, string = 0x{:x}) => NotObject", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStringLength"), env!(emulator), string);
            }
            return Ok(Some(JNI_NULL));
        };
        if object.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, string = 0x{:x}) => StringNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStringLength"), env!(emulator), string);
            }
            return Ok(Some(JNI_NULL));
        }
        let object = object.unwrap();
        let len = match object {
            DvmObject::String(utf) => utf.len(),
            _ => {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, string = 0x{:x}) => NotString", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStringLength"), env!(emulator), string);
                }
                return Ok(Some(JNI_NULL));
            }
        };

        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            println!("{} {}(env = 0x{:x}, string = 0x{:x}) => {}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStringLength"), env!(emulator), string, len);
        }

        Ok(Some(len as i64))
    }));
    let _get_string_chars = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetStringChars not implemented");
    }));
    let _release_string_chars = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("ReleaseStringChars not implemented");
    }));
    let _new_string_utf = svc_memory.register_svc(SimpleArm64Svc::new("_NewStringUTF", |emulator| {
        // jstring     (*NewStringUTF)(JNIEnv*, const char*);
        let dvm = dalvik!(emulator);

        let cp = emulator.backend.reg_read(RegisterARM64::X1).unwrap();
        if cp == 0 {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, utf = NULL) => NULL", Color::Yellow.paint("JNI:"), Color::Blue.paint("NewStringUTF"), env!(emulator));
            }
            return Ok(Some(JNI_NULL));
        }
        let utf = emulator.backend.mem_read_c_string(cp).unwrap();
        let string = DvmObject::String(utf.clone());
        let id = dvm.add_local_ref(string);

        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            println!("{} {}(env = 0x{:x}, utf = \"{}\") => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("NewStringUTF"), env!(emulator), utf, id);
        }

        Ok(Some(id))
    }));
    let _get_string_utf_length = svc_memory.register_svc(SimpleArm64Svc::new("_GetStringUTFLength", |emulator| {
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
                println!("{} {}(env = 0x{:x}, string = 0x{:x}) => NotObject", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStringUTFLength"), env!(emulator), string);
            }
            return Ok(Some(JNI_NULL));
        };
        if object.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, string = 0x{:x}) => StringNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStringUTFLength"), env!(emulator), string);
            }
            return Ok(Some(JNI_NULL));
        }
        let object = object.unwrap();
        let len = match object {
            DvmObject::String(utf) => utf.len(),
            _ => {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, string = 0x{:x}) => NotString", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStringUTFLength"), env!(emulator), string);
                }
                return Ok(Some(JNI_NULL));
            }
        };

        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            println!("{} {}(env = 0x{:x}, string = 0x{:x}) => {}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStringUTFLength"), env!(emulator), string, len);
        }

        Ok(Some(len as i64))
    }));
    let _get_string_utf_chars = svc_memory.register_svc(SimpleArm64Svc::new("_GetStringUTFChars", |emulator| {
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
                println!("{} {}(env = 0x{:x}, string = 0x{:x}, isCopy = 0x{:x}) => NotObject", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStringUTFChars"), env!(emulator), string, is_copy);
            }
            return Ok(Some(JNI_NULL));
        };
        if object.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, string = 0x{:x}, isCopy = 0x{:x}) => StringNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStringUTFChars"), env!(emulator), string, is_copy);
            }
            return Ok(Some(JNI_NULL));
        }
        let object = object.unwrap();
        let utf = match object {
            DvmObject::String(utf) => utf,
            _ => {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, string = 0x{:x}, isCopy = 0x{:x}) => NotString", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStringUTFChars"), env!(emulator), string, is_copy);
                }
                return Ok(Some(JNI_NULL));
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
            println!("{} {}(env = 0x{:x}, string = 0x{:x}, isCopy = 0x{:x}) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetStringUTFChars"), env!(emulator), string, is_copy, dest.addr);
        }

        return Ok(Some(dest.addr as i64))
    }));
    let _release_string_utf_chars = svc_memory.register_svc(SimpleArm64Svc::new("_ReleaseStringUTFChars", |emulator| {
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
                println!("{} {}(env = 0x{:x}, string = 0x{:x}, utf = 0x{:x}) => NotObject", Color::Yellow.paint("JNI:"), Color::Blue.paint("ReleaseStringUTFChars"), env!(emulator), string, utf);
            }
            return Ok(None);
        };
        if object.is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, string = 0x{:x}, utf = 0x{:x}) => StringNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("ReleaseStringUTFChars"), env!(emulator), string, utf);
            }
            return Ok(None);
        }
        let object = object.unwrap();
        let str_len = match object {
            DvmObject::String(str) => {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, string = 0x{:x}, utf = 0x{:x}) => Ok", Color::Yellow.paint("JNI:"), Color::Blue.paint("ReleaseStringUTFChars"), env!(emulator), string, utf);
                }
                str.len()
            }
            _ => {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, string = 0x{:x}, utf = 0x{:x}) => NotString", Color::Yellow.paint("JNI:"), Color::Blue.paint("ReleaseStringUTFChars"), env!(emulator), string, utf);
                }
                return Ok(None)
            }
        };
        emulator.ffree(utf, str_len + 1).unwrap();
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            println!("{} {}(env = 0x{:x}, string = 0x{:x}, utf = 0x{:x}) => Ok", Color::Yellow.paint("JNI:"), Color::Blue.paint("ReleaseStringUTFChars"), env!(emulator), string, utf);
        }
        Ok(None)
    }));
    let _get_array_length = svc_memory.register_svc(SimpleArm64Svc::new("_GetArrayLength", |emulator| {
        let dvm = dalvik!(emulator);
        let array = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;

        let flag = jni::get_flag_id(array);
        let object = if flag == JNI_FLAG_OBJECT {
            let object = dvm.get_local_ref(array);
            if object.is_none() {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, array = 0x{:x}) => ArrayNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetArrayLength"), env!(emulator), array);
                }
                return Ok(Some(JNI_ERR));
            }
            object.unwrap()
        } else if flag == JNI_FLAG_REF {
            let object = dvm.get_global_ref(array);
            if object.is_none() {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, array = 0x{:x}) => ArrayNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetArrayLength"), env!(emulator), array);
                }
                return Ok(Some(JNI_ERR));
            }
            object.unwrap()
        } else {
            unreachable!()
        };
        return match object {
            DvmObject::ByteArray(array_) => {
                let length = array_.len();
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, array = 0x{:x}) => {}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetArrayLength"), env!(emulator), array, length);
                }
                Ok(Some(length as i64))
            }
            DvmObject::ObjectArray(_, array_) => {
                let length = array_.len();
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, array = 0x{:x}) => {}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetArrayLength"), env!(emulator), array, length);
                }
                Ok(Some(length as i64))
            }
            _ => {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, array = 0x{:x}) => NotArray", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetArrayLength"), env!(emulator), array);
                }
                Ok(Some(JNI_ERR))
            }
        }
    }));
    let _new_object_array = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("NewObjectArray not implemented");
    }));
    let _get_object_array_element = svc_memory.register_svc(SimpleArm64Svc::new("_GetObjectArrayElement", |emulator| {
        let dvm = dalvik!(emulator);
        let array_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
        let index = emulator.backend.reg_read(RegisterARM64::X2).unwrap();

        let flag = jni::get_flag_id(array_id);
        let object = if flag == JNI_FLAG_OBJECT {
            let object = dvm.get_local_ref(array_id);
            if object.is_none() {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, array = 0x{:x}, index = {}) => ArrayNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetObjectArrayElement"), env!(emulator), array_id, index);
                }
                return Ok(Some(JNI_NULL));
            }
            object.unwrap()
        } else if flag == JNI_FLAG_REF {
            let object = dvm.get_global_ref(array_id);
            if object.is_none() {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, array = 0x{:x}, index = {}) => ArrayNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetObjectArrayElement"), env!(emulator), array_id, index);
                }
                return Ok(Some(JNI_NULL));
            }
            object.unwrap()
        } else {
            unreachable!()
        };
        match object {
            DvmObject::ObjectArray(_, array) => {
                if index < 0 || index as usize >= array.len() {
                    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                        println!("{} {}(env = 0x{:x}, array = 0x{:x}, index = {}) => ArrayIndexOutOfBounds", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetObjectArrayElement"), env!(emulator), array_id, index);
                    }
                    return Ok(Some(JNI_NULL));
                }
                return if let Some(object) = array.get(index as usize) {
                    if let Some(object) = object {
                        let id = dvm.add_local_ref(object.clone());
                        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                            println!("{} {}(env = 0x{:x}, array = 0x{:x}, index = {}) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetObjectArrayElement"), env!(emulator), array_id, index, id);
                        }
                        Ok(Some(id))
                    } else {
                        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                            println!("{} {}(env = 0x{:x}, array = 0x{:x}, index = {}) => NULL", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetObjectArrayElement"), env!(emulator), array_id, index);
                        }
                        Ok(Some(JNI_NULL))
                    }
                } else {
                    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                        println!("{} {}(env = 0x{:x}, array = 0x{:x}, index = {}) => ArrayIndexOutOfBounds", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetObjectArrayElement"), env!(emulator), array_id, index);
                    }
                    Ok(Some(JNI_NULL))
                }
            }
            _ => {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, array = 0x{:x}, index = {}) => NotArray", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetObjectArrayElement"), env!(emulator), array_id, index);
                }
                Ok(Some(JNI_NULL))
            }
        }
    }));
    let _set_object_array_element = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("SetObjectArrayElement not implemented");
    }));
    let _new_boolean_array = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("NewBooleanArray not implemented");
    }));
    let _new_byte_array = svc_memory.register_svc(SimpleArm64Svc::new("_NewByteArray", |emulator| {
        let dvm = dalvik!(emulator);
        let size = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as usize;

        let array = DvmObject::ByteArray(vec![0u8; size]);
        let id = dvm.add_local_ref(array);
        if option_env!("PRINT_JNI_CALLS") == Some("1") {
            println!("{} {}(env = 0x{:x}, size = {}) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("NewByteArray"), env!(emulator), size, id);
        }

        Ok(Some(id))
    }));
    let _new_char_array = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("NewCharArray not implemented");
    }));
    let _new_short_array = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("NewShortArray not implemented");
    }));
    let _new_int_array = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("NewIntArray not implemented");
    }));
    let _new_long_array = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("NewLongArray not implemented");
    }));
    let _new_float_array = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("NewFloatArray not implemented");
    }));
    let _new_double_array = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("NewDoubleArray not implemented");
    }));
    let _get_boolean_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetBooleanArrayElements not implemented");
    }));
    let _get_byte_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("_GetByteArrayElements", |emulator| {
        let dvm = dalvik!(emulator);
        let array = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
        let is_copy = emulator.backend.reg_read(RegisterARM64::X2).unwrap();

        let flag = jni::get_flag_id(array);
        let object = if flag == JNI_FLAG_OBJECT {
            let object = dvm.get_local_ref(array);
            if object.is_none() {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, array = 0x{:x}) => ArrayNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetByteArrayElements"), env!(emulator), array);
                }
                return Ok(Some(JNI_ERR));
            }
            object.unwrap()
        } else if flag == JNI_FLAG_REF {
            let object = dvm.get_global_ref(array);
            if object.is_none() {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, array = 0x{:x}) => ArrayNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetByteArrayElements"), env!(emulator), array);
                }
                return Ok(Some(JNI_ERR));
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
                    println!("{} {}(env = 0x{:x}, array = 0x{:x}, is_copy = 0x{:X}) => 0x{:X}", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetByteArrayElements"), env!(emulator), array, is_copy, dest.addr as i64);
                }

                if is_copy != 0 {
                    emulator.backend.mem_write_i64(is_copy, 1).unwrap();
                }

                return Ok(Some(dest.addr as i64))
            },
            _ => unreachable!()
        }
    }));
    let _get_char_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetCharArrayElements not implemented");
    }));
    let _get_short_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetShortArrayElements not implemented");
    }));
    let _get_int_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetIntArrayElements not implemented");
    }));
    let _get_long_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetLongArrayElements not implemented");
    }));
    let _get_float_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetFloatArrayElements not implemented");
    }));
    let _get_double_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetDoubleArrayElements not implemented");
    }));
    let _release_boolean_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("ReleaseBooleanArrayElements not implemented");
    }));
    let _release_byte_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("_ReleaseByteArrayElements", |emulator| {
        let dvm = dalvik!(emulator);
        let array = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
        let elems = emulator.backend.reg_read(RegisterARM64::X2).unwrap();
        let mode = emulator.backend.reg_read(RegisterARM64::X3).unwrap() as i32;

        let flag = jni::get_flag_id(array);
        let object = if flag == JNI_FLAG_REF {
            let obj = dvm.get_global_ref_mut(array);
            if obj.is_none() {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, array = 0x{:x}, elems = 0x{:x}, mode = {}) => ArrayNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("ReleaseByteArrayElements"), env!(emulator), array, elems, mode);
                }
                return Ok(Some(JNI_ERR));
            }
            obj.unwrap()
        } else if flag == JNI_FLAG_OBJECT {
            let obj = dvm.get_local_ref_mut(array);
            if obj.is_none() {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, array = 0x{:x}, elems = 0x{:x}, mode = {}) => ArrayNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("ReleaseByteArrayElements"), env!(emulator), array, elems, mode);
                }
                return Ok(Some(JNI_ERR));
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
            println!("{} {}(env = 0x{:x}, array = 0x{:x}, elems = 0x{:x}, mode = {}) => OK", Color::Yellow.paint("JNI:"), Color::Blue.paint("ReleaseByteArrayElements"), env!(emulator), array, elems, mode);
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

        Ok(Some(JNI_OK))
    }));
    let _release_char_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("ReleaseCharArrayElements not implemented");
    }));
    let _release_short_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("ReleaseShortArrayElements not implemented");
    }));
    let _release_int_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("ReleaseIntArrayElements not implemented");
    }));
    let _release_long_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("ReleaseLongArrayElements not implemented");
    }));
    let _release_float_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("ReleaseFloatArrayElements not implemented");
    }));
    let _release_double_array_elements = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("ReleaseDoubleArrayElements not implemented");
    }));
    let _get_boolean_array_region = svc_memory.register_svc(SimpleArm64Svc::new("_GetBooleanArrayRegion", |emulator| {
        panic!()
    }));
    let _get_byte_array_region = svc_memory.register_svc(SimpleArm64Svc::new("_GetByteArrayRegion", |emulator| {
        //void        (*GetByteArrayRegion)(JNIEnv*, jbyteArray, jsize, jsize, jbyte*);
        let dvm = dalvik!(emulator);
        let array = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
        let start = emulator.backend.reg_read(RegisterARM64::X2).unwrap() as usize;
        let length = emulator.backend.reg_read(RegisterARM64::X3).unwrap() as usize;
        let buf = emulator.backend.reg_read(RegisterARM64::X4).unwrap();

        let flag = jni::get_flag_id(array);
        if flag != JNI_FLAG_REF && flag != JNI_FLAG_OBJECT {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, array = 0x{:x}, start = {}, length = {}, buf = 0x{:x}) => InvalidArray", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetByteArrayRegion"), env!(emulator), array, start, length, buf);
            }
            return Err(anyhow!("_GetByteArrayRegion: input array pointer is invalid"))
        }

        let data = match flag {
            JNI_FLAG_REF => {
                let ref_data = dvm.get_global_ref(array);
                if ref_data.is_none() {
                    if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                        println!("{} {}(env = 0x{:x}, array = 0x{:x}, start = {}, length = {}, buf = 0x{:x}) => GlobalRefNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetByteArrayRegion"), env!(emulator), array, start, length, buf);
                    }
                    return Err(anyhow!("_GetByteArrayRegion: input array pointer is not found in global ref pool"))
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
                        println!("{} {}(env = 0x{:x}, array = 0x{:x}, start = {}, length = {}, buf = 0x{:x}) => LocalRefNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetByteArrayRegion"), env!(emulator), array, start, length, buf);
                    }
                    return Err(anyhow!("_GetByteArrayRegion: input array pointer is not found in local ref pool"))
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
            println!("{} {}(env = 0x{:x}, array = 0x{:x}, start = {}, length = {}, buf = 0x{:x})", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetByteArrayRegion"), env!(emulator), array, start, length, buf);
            println!("  data = {} | {:?}", String::from_utf8_lossy(data.as_slice()), hex::encode(&data));
        } else if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            println!("{} {}(env = 0x{:x}, array = 0x{:x}, start = {}, length = {}, buf = 0x{:x})", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetByteArrayRegion"), env!(emulator), array, start, length, buf);
        }

        return Ok(None)
    }));
    let _get_char_array_region = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetCharArrayRegion not implemented");
    }));
    let _get_short_array_region = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetShortArrayRegion not implemented");
    }));
    let _get_int_array_region = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetIntArrayRegion not implemented");
    }));
    let _get_long_array_region = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetLongArrayRegion not implemented");
    }));
    let _get_float_array_region = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetFloatArrayRegion not implemented");
    }));
    let _get_double_array_region = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetDoubleArrayRegion not implemented");
    }));
    let _set_boolean_array_region = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("SetBooleanArrayRegion not implemented");
    }));
    let _set_byte_array_region = svc_memory.register_svc(SimpleArm64Svc::new("_SetByteArrayRegion", |emulator| { // VOID METHOD
        let dvm = dalvik!(emulator);
        let array = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
        let start = emulator.backend.reg_read(RegisterARM64::X2).unwrap() as usize;
        let length = emulator.backend.reg_read(RegisterARM64::X3).unwrap() as usize;
        let buf = emulator.backend.reg_read(RegisterARM64::X4).unwrap();

        let data = emulator.backend.mem_read_as_vec(buf, length).unwrap();
        if option_env!("PRINT_JNI_CALLS_EX").unwrap_or("") == "1" {
            println!("{} {}(env = 0x{:x}, array = 0x{:x}, start = {}, length = {}, buf = 0x{:x})", Color::Yellow.paint("JNI:"), Color::Blue.paint("SetByteArrayRegion"), env!(emulator), array, start, length, buf);
            println!("  data = {} | {:?}", String::from_utf8_lossy(data.as_slice()), hex::encode(&data));
        } else if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            println!("{} {}(env = 0x{:x}, array = 0x{:x}, start = {}, length = {}, buf = 0x{:x})", Color::Yellow.paint("JNI:"), Color::Blue.paint("SetByteArrayRegion"), env!(emulator), array, start, length, buf);
        }

        let flag = jni::get_flag_id(array);
        if flag != JNI_FLAG_REF && flag != JNI_FLAG_OBJECT {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, array = 0x{:x}, start = {}, length = {}, buf = 0x{:x}) => InvalidArray", Color::Yellow.paint("JNI:"), Color::Blue.paint("SetByteArrayRegion"), env!(emulator), array, start, length, buf);
            }
            return Err(anyhow!("_SetByteArrayRegion: input array pointer is invalid"))
        }

        if flag == JNI_FLAG_REF {
            let ref_data = dvm.get_global_ref_mut(array);
            if ref_data.is_none() {
                if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                    println!("{} {}(env = 0x{:x}, array = 0x{:x}, start = {}, length = {}, buf = 0x{:x}) => GlobalRefNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("SetByteArrayRegion"), env!(emulator), array, start, length, buf);
                }
                return Err(anyhow!("_SetByteArrayRegion: input array pointer is not found in global ref pool"))
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
                    println!("{} {}(env = 0x{:x}, array = 0x{:x}, start = {}, length = {}, buf = 0x{:x}) => LocalRefNotFound", Color::Yellow.paint("JNI:"), Color::Blue.paint("SetByteArrayRegion"), env!(emulator), array, start, length, buf);
                }
                return Err(anyhow!("_SetByteArrayRegion: input array pointer is not found in local ref pool"))
            }
            match ref_data.unwrap() {
                DvmObject::ByteArray(bytes) => {
                    bytes.splice(start..start + length, data.iter().cloned());
                }
                _ => unreachable!()
            }
        }

        return Ok(None)
    }));
    let _set_char_array_region = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("SetCharArrayRegion not implemented");
    }));
    let _set_short_array_region = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("SetShortArrayRegion not implemented");
    }));
    let _set_int_array_region = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("SetIntArrayRegion not implemented");
    }));
    let _set_long_array_region = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("SetLongArrayRegion not implemented");
    }));
    let _set_float_array_region = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("SetFloatArrayRegion not implemented");
    }));
    let _set_double_array_region = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("SetDoubleArrayRegion not implemented");
    }));
    let _register_natives = svc_memory.register_svc(SimpleArm64Svc::new("_RegisterNatives", |emulator| {
        let dvm = dalvik!(emulator);

        let clz_id = emulator.backend.reg_read(RegisterARM64::X1).unwrap() as i64;
        let methods = emulator.backend.reg_read(RegisterARM64::X2).unwrap();
        let n_methods = emulator.backend.reg_read(RegisterARM64::X3).unwrap();

        if n_methods == 0 {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}) => NoAnyMethod", Color::Yellow.paint("JNI:"), Color::Blue.paint("RegisterNatives"), env!(emulator), clz_id);
            }
            return Ok(Some(JNI_OK));
        }

        if dvm.find_class_by_id(&clz_id).is_none() {
            if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
                println!("{} {}(env = 0x{:x}, class = 0x{:x}) => ClassNotFOunt", Color::Yellow.paint("JNI:"), Color::Blue.paint("RegisterNatives"), env!(emulator), clz_id);
            }
            return Ok(Some(JNI_ERR));
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
                println!("{} {}(env = 0x{:x}, class = 0x{:x}, name = {}, signature = {}, fn_ptr = 0x{:X})", Color::Yellow.paint("JNI:"), Color::Blue.paint("RegisterNatives"), env!(emulator), clz_id, name, signature, fn_ptr);
            }
            start_id += 1;
            members_list.push(DvmMember::Method(DvmMethod::new(clz_id + start_id, clz_id, name, signature, fn_ptr)));
        }

        Ok(Some(JNI_OK))
    }));
    let _unregister_natives = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("UnregisterNatives not implemented");
    }));
    let _monitor_enter = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("MonitorEnter not implemented");
    }));
    let _monitor_exit = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("MonitorExit not implemented");
    }));
    let _get_java_vm = svc_memory.register_svc(SimpleArm64Svc::new("_GetJavaVM", |emulator| {
        let dvm = emulator.inner_mut().dalvik.as_ref().unwrap();
        let vm_ptr = emulator.backend.reg_read(RegisterARM64::X1).unwrap();
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            println!("{} {}(env = 0x{:x}, vm = 0x{:x})", Color::Yellow.paint("JNI:"), Color::Blue.paint("GetJavaVM"), env!(emulator), vm_ptr);
        }
        let vm = dvm.java_vm;
        emulator.backend.mem_write(vm_ptr, &vm.to_le_bytes()).unwrap();
        Ok(Some(JNI_OK))
    }));
    let _get_string_region = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetStringRegion not implemented");
    }));
    let _get_string_utf_region = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetStringUTFRegion not implemented");
    }));
    let _get_primitive_array_critical = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetPrimitiveArrayCritical not implemented");
    }));
    let _release_primitive_array_critical = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("ReleasePrimitiveArrayCritical not implemented");
    }));
    let _get_string_critical = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetStringCritical not implemented");
    }));
    let _release_string_critical = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("ReleaseStringCritical not implemented");
    }));
    let _new_weak_global_ref = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("NewWeakGlobalRef not implemented");
    }));
    let _delete_weak_global_ref = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("DeleteWeakGlobalRef not implemented");
    }));
    let _exception_check = svc_memory.register_svc(SimpleArm64Svc::new("_ExceptionCheck", |emulator| {
        let dvm = emulator.inner_mut().dalvik.as_ref().unwrap();
        let ret = if dvm.throwable.is_some() { JNI_TRUE } else { JNI_FALSE };
        if option_env!("PRINT_JNI_CALLS").unwrap_or("") == "1" {
            println!("{} {}(env = 0x{:x}) => {}", Color::Yellow.paint("JNI:"), Color::Blue.paint("ExceptionCheck"), env!(emulator), ret);
        }
        return Ok(Some(ret));
    }));
    let _new_direct_byte_buffer = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("NewDirectByteBuffer not implemented");
    }));
    let _get_direct_buffer_address = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetDirectBufferAddress not implemented");
    }));
    let _get_direct_buffer_capacity = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetDirectBufferCapacity not implemented");
    }));
    let _get_object_ref_type = svc_memory.register_svc(SimpleArm64Svc::new("Jni-3", |emulator| {
        panic!("GetObjectRefType not implemented");
    }));

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