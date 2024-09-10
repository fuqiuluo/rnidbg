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
use crate::business::BusinessConfig;
use crate::device::Device;

pub struct QSecJni<'a, T: Clone> {
    pub business: BusinessConfig,
    pub device: Rc<Device>,
    pub is_sign: Arc<AtomicBool>,
    pub o3did: String,
    pd: PhantomData<&'a T>
}

pub struct SignResult {
    pub sign: Vec<u8>,
    pub token: Vec<u8>,
    pub extra: Vec<u8>
}

impl<'a, T: Clone> QSecJni<'a, T> {
    pub fn new(
        business: BusinessConfig,
        device: Rc<Device>
    ) -> QSecJni<'a, T> {
        QSecJni {
            device,
            business,
            is_sign: Arc::new(AtomicBool::new(false)),
            o3did: String::new(),
            pd: PhantomData
        }
    }
}

impl<T: Clone> Jni<T> for QSecJni<'_, T> {
    fn resolve_method(&mut self, vm: &mut DalvikVM64<T>, class: &Rc<DvmClass>, name: &str, signature: &str, is_static: bool) -> bool {
        if class.name == "com/tencent/mobileqq/qsec/qsecest/QsecEst" {
            return false
        }
        if class.name == "com/tencent/mobileqq/qsec/qsecurity/QSec" { 
            if name == "updateO3DID" && signature == "(Ljava/lang/String;)V" && !is_static {
                return true
            }
        }
        if class.name == "com/tencent/mobileqq/sign/QQSecuritySign$SignResult" {
            if name == "<init>" && signature == "()V" && !is_static {
                return true
            }
        }
        if class.name == "com/tencent/mobileqq/dt/model/FEBound" {
            if name == "transform" && signature == "(I[B)[B" && is_static {
                return true
            }
        }
        if class.name == "com/tencent/mobileqq/qsec/qsecdandelionsdk/Dandelion" {
            if name == "getInstance" && signature == "()Lcom/tencent/mobileqq/qsec/qsecdandelionsdk/Dandelion;" && is_static {
                return true
            }
        }
        if class.name == "java/lang/String" {
            if name == "<init>" && signature == "([BLjava/lang/String;)V" && !is_static {
                return true
            }
            if name == "hashCode" && signature == "()I" && !is_static {
                return true
            }
        }
        if class.name == "java/lang/Thread" {
            if name == "getStackTrace" && signature == "()[Ljava/lang/StackTraceElement;" && !is_static {
                return true
            }
            if name == "currentThread" && signature == "()Ljava/lang/Thread;" && is_static {
                return true
            }
            if name == "getClassName" && signature == "()Ljava/lang/String;" && !is_static {
                return true
            }
            if name == "getMethodName" && signature == "()Ljava/lang/String;" && !is_static {
                return true
            }
        }
        if class.name == "com/tencent/mobileqq/dt/app/Dtc" {
            if name == "systemGetsafe" && signature == "(Ljava/lang/String;)Ljava/lang/String" && !is_static {
                return false
            }

            if name == "getPropSafe" && signature == "(Ljava/lang/String;)Ljava/lang/String;" && is_static {
                return true
            }

            if name == "getAppVersionName" && signature == "(Ljava/lang/String;)Ljava/lang/String;" {
                return true
            }

            if name == "getAppVersionCode" && signature == "(Ljava/lang/String;)Ljava/lang/String;" {
                return true
            }

            if name == "getAppInstallTime" && signature == "(Ljava/lang/String;)Ljava/lang/String;" {
                return true
            }

            if name == "getDensity" && signature == "(Ljava/lang/String;)Ljava/lang/String;" {
                return true
            }

            if name == "getFontDpi" && signature == "(Ljava/lang/String;)Ljava/lang/String;" {
                return true
            }

            if name == "getScreenSize" && signature == "(Ljava/lang/String;)Ljava/lang/String;" {
                return true
            }

            if name == "saveList" && signature == "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V" {
                return true
            }

            if name == "mmKVValue" && signature == "(Ljava/lang/String;)Ljava/lang/String;" {
                return true
            }

            if name == "getStorage" && signature == "(Ljava/lang/String;)Ljava/lang/String;" {
                return true
            }

            if name == "systemGetSafe" && signature == "(Ljava/lang/String;)Ljava/lang/String;" && is_static {
                return true
            }

            if name == "getIME" && signature == "(Ljava/lang/String;)Ljava/lang/String;" && is_static {
                return true
            }

            if name == "mmKVSaveValue" && signature == "(Ljava/lang/String;Ljava/lang/String;)V" {
                return true
            }

            if name == "mmQsecKVValue" && signature == "(Ljava/lang/String;)Ljava/lang/String;" {
                return true;
            }

            if name == "mmKVQsecSaveValue" && signature == "(Ljava/lang/String;Ljava/lang/String;)V"
            {
                return true;
            }

            if name == "getUid"
                && signature == "(Ljava/lang/String;)Ljava/lang/String;"
                && is_static
            {
                return true;
            }

            if name == "getApkPath"
                && signature == "(Ljava/lang/String;)Ljava/lang/String;"
                && is_static
            {
                return true;
            }
        }

        if class.name == "com/tencent/mobileqq/fe/IFEKitLog" {
            if name == "e" && signature == "(Ljava/lang/String;ILjava/lang/String;)V" && !is_static {
                return true
            }
            if name == "i" && signature == "(Ljava/lang/String;ILjava/lang/String;)V" && !is_static {
                return true
            }
        }

        if class.name == "com/tencent/mobileqq/channel/ChannelProxy" {
            if name == "sendMessage" && signature == "(Ljava/lang/String;[BJ)V" && !is_static {
                return true
            }
        }

        if class.name == "android/content/Context" {
            if name == "getContentResolver" && signature == "()Landroid/content/ContentResolver;" && !is_static {
                return true
            }

            if name == "getPackageResourcePath" && signature == "()Ljava/lang/String;" && !is_static {
                return true
            }
        }
        
        if class.name == "android/content/ContentResolver" {
            if name == "acquireContentProviderClient" && signature == "(Ljava/lang/String;)Landroid/content/ContentProviderClient;" {
                return true
            }
        }
        
        if class.name == "android/content/Context" { 
            if name == "getApplicationInfo" && signature == "()Landroid/content/pm/ApplicationInfo;" {
                return true
            }

            if name == "getFilesDir" && signature == "()Ljava/io/File;" {
                return true
            }

            if name == "getPackageName" && signature == "()Ljava/lang/String;" {
                return true
            }

            if name == "getPackageManager" && signature == "()Landroid/content/pm/PackageManager;" {
                return true
            }
        }

        if class.name == "java/io/File" {
            if name == "getAbsolutePath" && signature == "()Ljava/lang/String;" {
                return true
            }

            if name == "<init>" && signature == "(Ljava/lang/String;)V" {
                return true
            }

            if name == "canRead" && signature == "()Z" {
                return true
            }
        }

        if class.name == "com/tencent/mobileqq/fe/utils/DeepSleepDetector" {
            if name == "getCheckResult" && signature == "()Ljava/lang/String;" {
                return true
            }
            
            if name == "stopCheck" && signature == "()V" {
                return true
            }
        }

        if class.name == "com/tencent/mobileqq/fe/EventCallback" {
            if name == "onResult" && signature == "(I[B)V" {
                return true
            }
        }

        if class.name == "java/util/UUID" {
            if name == "randomUUID" && signature == "()Ljava/util/UUID;" && is_static {
                return true
            }

            if name == "toString" && signature == "()Ljava/lang/String;" && !is_static {
                return true
            }
        }

        if class.name == "android/os/ServiceManager" {
            if name == "getService" && signature == "(Ljava/lang/String;)Landroid/os/IBinder;" && is_static {
                return true
            }
        }

        if class.name == "android/content/pm/IPackageManager$Stub" {
            if name == "asInterface" && signature == "(Landroid/os/IBinder;)Landroid/content/pm/IPackageManager;" && is_static {
                return true
            }
        }

        if class.name == "android/content/pm/IPackageManager" {
            if name == "getPackageGids" && signature == "(Ljava/lang/String;II)[I" {
                return true
            }
        }

        if class.name == "android/content/pm/PackageManager" {
            if name == "getPackageGids" && signature == "(Ljava/lang/String;)[I" {
                return true
            }
        }

        if class.name == "android/provider/Settings$System" {
            if name == "getString" && signature == "(Landroid/content/ContentResolver;Ljava/lang/String;)Ljava/lang/String;" {
                return true
            }
        }

        if class.name == "android/os/Process" {
            if name == "myUid" && signature == "()I" && is_static {
                return true
            }
        }


        panic!("MyJni resolve_method: signature: L{};->{}{}, static={}", class.name, name, signature, is_static);
    }

    fn resolve_filed(&mut self, vm: &mut DalvikVM64<T>, class: &Rc<DvmClass>, name: &str, signature: &str, is_static: bool) -> bool {
        true
    }

    fn call_method_v(&mut self, vm: &mut DalvikVM64<T>, acc: MethodAcc, class: &Rc<DvmClass>, method: &DvmMethod, instance: Option<&mut DvmObject>, args: &mut VaList<T>) -> JniValue {
        if acc.contains(MethodAcc::CONSTRUCTOR) {
            if class.name == "java/lang/String" {
                if method.name == "<init>" && method.signature == "([BLjava/lang/String;)V" {
                    let result = String::from_utf8(args.get::<Vec<u8>>(vm)).unwrap();
                    let charset = args.get::<String>(vm);
                    return DvmObject::String(result).into()
                }
            }
        }

        if !acc.contains(MethodAcc::STATIC) {
            if class.name == "java/lang/String" && method.name == "hashCode" && method.signature == "()I" {
                let this = instance.unwrap();
                let data = match this {
                    DvmObject::String(data) => data,
                    _ => unreachable!()
                };
                let mut h = 0i32;
                for v in data.as_bytes() {
                    let b = i8::from_le_bytes(v.to_le_bytes());
                    h = h.overflowing_mul(31).0.overflowing_add(b as i32).0;
                }
                return h.into();
            }

            if class.name == "java/lang/Thread" {
                if method.name == "getStackTrace" && method.signature == "()[Ljava/lang/StackTraceElement;" {
                    let is_sign = self.is_sign.load(Ordering::Relaxed);
                    return if is_sign {
                        DvmObject::ObjectArray(vm.resolve_class_unchecked("java/lang/StackTraceElement").1, vec![
                            DvmObject::DataMutInstance(class.clone(), Rc::new(UnsafeCell::new(Box::new("dalvik.system.VMStack:getThreadStackTrace")))).into(),
                            DvmObject::DataMutInstance(class.clone(), Rc::new(UnsafeCell::new(Box::new("java.lang.Thread:getStackTrace")))).into(),
                            DvmObject::DataMutInstance(class.clone(), Rc::new(UnsafeCell::new(Box::new("com.tencent.mobileqq.sign.QQSecuritySign:getSign")))).into(),
                            DvmObject::DataMutInstance(class.clone(), Rc::new(UnsafeCell::new(Box::new("com.tencent.mobileqq.sign.QQSecuritySign:getSign")))).into(),
                            DvmObject::DataMutInstance(class.clone(), Rc::new(UnsafeCell::new(Box::new("com.tencent.mobileqq.fe.FEKit:getSign")))).into(),
                            DvmObject::DataMutInstance(class.clone(), Rc::new(UnsafeCell::new(Box::new("com.tencent.mobileqq.msf.core.d0.a:c")))).into(),
                            DvmObject::DataMutInstance(class.clone(), Rc::new(UnsafeCell::new(Box::new("com.tencent.mobileqq.msf.core.r:a")))).into(),
                            DvmObject::DataMutInstance(class.clone(), Rc::new(UnsafeCell::new(Box::new("com.tencent.mobileqq.msf.core.r:b")))).into(),
                            DvmObject::DataMutInstance(class.clone(), Rc::new(UnsafeCell::new(Box::new("com.tencent.mobileqq.msf.core.r:d")))).into(),
                            DvmObject::DataMutInstance(class.clone(), Rc::new(UnsafeCell::new(Box::new("com.tencent.mobileqq.msf.core.r:a")))).into(),
                            DvmObject::DataMutInstance(class.clone(), Rc::new(UnsafeCell::new(Box::new("com.tencent.mobileqq.msf.core.r$d:run")))).into(),
                        ]).into()
                    } else {
                        // 替换为 Java 代码中的第二个条件
                        DvmObject::ObjectArray(vm.resolve_class_unchecked("java/lang/StackTraceElement").1, vec![
                            DvmObject::DataMutInstance(class.clone(), Rc::new(UnsafeCell::new(Box::new("dalvik.system.VMStack:getThreadStackTrace")))).into(),
                            DvmObject::DataMutInstance(class.clone(), Rc::new(UnsafeCell::new(Box::new("java.lang.Thread:getStackTrace")))).into(),
                            DvmObject::DataMutInstance(class.clone(), Rc::new(UnsafeCell::new(Box::new("com.tencent.mobileqq.qsec.qsecdandelionsdk.Dandelion:energy")))).into(),
                            DvmObject::DataMutInstance(class.clone(), Rc::new(UnsafeCell::new(Box::new("com.tencent.mobileqq.qsec.qsecdandelionsdk.Dandelion:fly")))).into(),
                            DvmObject::DataMutInstance(class.clone(), Rc::new(UnsafeCell::new(Box::new("com.tencent.mobileqq.qsecurity.QSec:getLiteSign")))).into(),
                            DvmObject::DataMutInstance(class.clone(), Rc::new(UnsafeCell::new(Box::new("oicq.wlogin_sdk.tlv_type.tlv_t544:get_tlv_544")))).into(),
                            DvmObject::DataMutInstance(class.clone(), Rc::new(UnsafeCell::new(Box::new("oicq.wlogin_sdk.request.j:a")))).into(),
                            DvmObject::DataMutInstance(class.clone(), Rc::new(UnsafeCell::new(Box::new("oicq.wlogin_sdk.request.j:a")))).into(),
                            DvmObject::DataMutInstance(class.clone(), Rc::new(UnsafeCell::new(Box::new("oicq.wlogin_sdk.request.WtloginHelper:GetStWithPasswd")))).into(),
                            DvmObject::DataMutInstance(class.clone(), Rc::new(UnsafeCell::new(Box::new("oicq.wlogin_sdk.request.WtloginHelper:access$500")))).into(),
                            DvmObject::DataMutInstance(class.clone(), Rc::new(UnsafeCell::new(Box::new("oicq.wlogin_sdk.request.WtloginHelper$HelperThread:run")))).into(),
                        ]).into()
                    };
                }
                if method.name == "getClassName" && method.signature == "()Ljava/lang/String;" {
                    let instance = instance.unwrap();
                    match instance {
                        DvmObject::DataMutInstance(_, value) => {
                            let value = unsafe { &mut *value.get() };
                            match value.downcast_mut::<&str>() {
                                Some(value) => {
                                    let class_name = value.split(":").next().unwrap();
                                    return DvmObject::String(class_name.to_string()).into()
                                }
                                None => unreachable!()
                            }
                        }
                        _ => unreachable!()
                    }
                }
                if method.name == "getMethodName" && method.signature == "()Ljava/lang/String;" {
                    let instance = instance.unwrap();
                    match instance {
                        DvmObject::DataMutInstance(_, value) => {
                            let value = unsafe { &mut *value.get() };
                            match value.downcast_mut::<&str>() {
                                Some(value) => {
                                    let method_name = value.split(":").last().unwrap();
                                    return DvmObject::String(method_name.to_string()).into()
                                }
                                None => unreachable!()
                            }
                        }
                        _ => unreachable!()
                    }
                }
            }
        }
        
        if acc.contains(MethodAcc::STATIC) && class.name == "java/lang/Thread" && method.name == "currentThread" && method.signature == "()Ljava/lang/Thread;" {
            return vm.resolve_class_unchecked("java/lang/Thread").1
                .new_simple_instance(vm).into()
        }

        if !acc.contains(MethodAcc::STATIC) && class.name == "android/content/Context" {
            if method.name == "getContentResolver" && method.signature == "()Landroid/content/ContentResolver;" {
                return vm.resolve_class_unchecked("android/content/ContentResolver").1
                    .new_simple_instance(vm).into()
            }
        }

        if class.name == "android/content/ContentResolver" {
            if method.name == "acquireContentProviderClient" && method.signature == "(Ljava/lang/String;)Landroid/content/ContentProviderClient;" {
                return JniValue::Null
            }
        }

        if class.name == "com/tencent/mobileqq/sign/QQSecuritySign$SignResult" {
            if method.name == "<init>" && method.signature == "()V" {
                return DvmObject::DataMutInstance(class.clone(), Rc::new(UnsafeCell::new(Box::new(SignResult {
                    sign: vec![],
                    token: vec![],
                    extra: vec![],
                })))).into()
            }
        }
        
        if class.name == "android/content/Context" {
            if method.name == "getApplicationInfo" && method.signature == "()Landroid/content/pm/ApplicationInfo;" {
                return vm.resolve_class_unchecked("android/content/pm/ApplicationInfo").1.new_simple_instance(vm).into()
            }
            
            if method.name == "getFilesDir" && method.signature == "()Ljava/io/File;" {
                let class = vm.resolve_class_unchecked("java/io/File").1;
                return DvmObject::DataInstance(class, Rc::new(Box::new("/data/user/0/com.tencent.mobileqq/files"))).into()
            }

            if method.name == "getPackageName" && method.signature == "()Ljava/lang/String;" {
                return "com.tencent.mobileqq".into()
            }

            if method.name == "getPackageResourcePath" && method.signature == "()Ljava/lang/String;" {
                return "/data/app/~~vbcRLwPxS0GyVfqT-nCYrQ==/com.tencent.mobileqq-xJKJPVp9lorkCgR_w5zhyA==/base.apk".into()
            }

            if method.name == "getPackageManager" && method.signature == "()Landroid/content/pm/PackageManager;" {
                return vm.resolve_class_unchecked("android/content/pm/PackageManager").1.new_simple_instance(vm).into()
            }
        }

        if class.name == "java/io/File" {
            if method.name == "getAbsolutePath" && method.signature == "()Ljava/lang/String;" {
                let this = instance.unwrap();
                match this {
                    DvmObject::DataInstance(_, value) => {
                        match value.downcast_ref::<&str>() {
                            Some(value) => {
                                return DvmObject::String(value.to_string()).into()
                            }
                            None => unreachable!()
                        }
                    }
                    _ => unreachable!()
                }
            }

            if method.name == "<init>" && method.signature == "(Ljava/lang/String;)V" {
                let path = args.get::<String>(vm);
                let class = vm.resolve_class_unchecked("java/io/File").1;
                return DvmObject::DataInstance(class, Rc::new(Box::new(path))).into();
            }

            if method.name == "canRead" && method.signature == "()Z" {
                let file = instance.unwrap();
                let path = match file {
                    DvmObject::DataInstance(_, value) => match value.downcast_ref::<String>() {
                        Some(value) => value,
                        None => unreachable!(),
                    },
                    _ => unreachable!(),
                };

                if path == "/data/data/com.tencent.mobileqq/.." {
                    return false.into();
                } else {

                    panic!("canRead: path={}", path);
                }
            }
        }
        
        if class.name == "com/tencent/mobileqq/fe/utils/DeepSleepDetector" {
            if method.name == "getCheckResult" && method.signature == "()Ljava/lang/String;" {
                return "0.8".into()
            }
            if method.name == "stopCheck" && method.signature == "()V" {
                return JniValue::Null
            }
        }

        if class.name == "com/tencent/mobileqq/fe/EventCallback" {
            if method.name == "onResult" && method.signature == "(I[B)V" {
                //let code = args.get::<i32>(vm);
                //let data = args.get::<Vec<u8>>(vm);
                return JniValue::Void
            }
        }

        if class.name == "com/tencent/mobileqq/qsec/qsecurity/QSec" {
            if method.name == "updateO3DID" && method.signature == "(Ljava/lang/String;)V" {
                let o3did = args.get::<String>(vm);

                self.o3did = o3did;
                return JniValue::Void
            }
        }

        if class.name == "java/util/UUID" {
            if method.name == "randomUUID" && method.signature == "()Ljava/util/UUID;" {
                return class.new_simple_instance(vm).into()
            }

            if method.name == "toString" && method.signature == "()Ljava/lang/String;" {
                return "efa95389-fe66-4c60-a24a-444e47bdfd6c".into()
            }
        }

        if class.name == "android/os/ServiceManager" {
            if method.name == "getService" && method.signature == "(Ljava/lang/String;)Landroid/os/IBinder;" {
                let name = args.get::<String>(vm);
                match name.as_str() {
                    "package" => return vm.resolve_class_unchecked("android/os/IBinder").1
                        .new_simple_instance(vm).into(),
                    _ => {

                        panic!("unknown service: {}", name)
                    }
                }
            }
        }

        if class.name == "android/content/pm/IPackageManager$Stub" {
            if method.name == "asInterface" && method.signature == "(Landroid/os/IBinder;)Landroid/content/pm/IPackageManager;" {
                return vm.resolve_class_unchecked("android/content/pm/IPackageManager").1
                    .new_simple_instance(vm).into()
            }
        }

        if class.name == "android/content/pm/IPackageManager" || class.name == "android/content/pm/PackageManager" {
            if method.name == "getPackageGids" && (method.signature == "(Ljava/lang/String;II)[I" || method.signature == "(Ljava/lang/String;)[I") {
                let name = args.get::<String>(vm);
                if name == "moe.fuqiuluo.shamrock" {
                    let exception_class = vm.resolve_class_unchecked("android/content/pm/PackageManager$NameNotFoundException").1;
                    let exception = exception_class.new_simple_instance(vm);
                    vm.throw(exception);
                    return JniValue::Void
                }

                panic!("getPackageGids: name={}", name)
            }
        }

        if class.name == "android/provider/Settings$System" {
            if method.name == "getString" && method.signature == "(Landroid/content/ContentResolver;Ljava/lang/String;)Ljava/lang/String;" {
                let resolver = args.get::<DvmObject>(vm);
                let key = args.get::<String>(vm);
                if key == "android_id" {
                    return self.device.android_id.as_str().into()
                }
                panic!("getString: key={}", key)
            }
        }

        if class.name == "android/os/Process" && method.name == "myUid" && method.signature == "()I" {
            return 10261.into();
        }

        unreachable!("MyJni call acc={:?}, class={}, method={}, signature={}", acc, class.name, method.name, method.signature)
    }

    fn get_field_value(&mut self, vm: &mut DalvikVM64<T>, class: &Rc<DvmClass>, field: &DvmField, instance: Option<&mut DvmObject>) -> JniValue {
        if class.name == "com/tencent/mobileqq/qsec/qsecurity/QSecConfig" {
            if field.name == "business_uin" && field.signature == "Ljava/lang/String;" {
                return self.business.uin.as_str().into()
            }
            if field.name == "business_seed" && field.signature == "Ljava/lang/String;" {
                return "".into()
            }
            if field.name == "business_guid" && field.signature == "Ljava/lang/String;" {
                return self.device.guid.as_str().into()
            }
            if field.name == "business_o3did" && field.signature == "Ljava/lang/String;" {
                return self.o3did.as_str().into()
            }
            if field.name == "business_q36" && field.signature == "Ljava/lang/String;" {
                return self.device.q36.as_str().into()
            }
            if field.name == "business_qua" && field.signature == "Ljava/lang/String;" {
                return self.business.qua.as_str().into()
            }
        }

        if class.name == "android/os/Build$VERSION" && field.name == "SDK_INT" && field.signature == "I" {
            return (self.device.os_sdk as i32).into()
        }

        if class.name == "android/content/pm/ApplicationInfo" {
            if field.name == "targetSdkVersion" && field.signature == "I" {
                return 31.into()
            }

            if field.name == "nativeLibraryDir" && field.signature == "Ljava/lang/String;" {
                return "/data/app/~~vbcRLwPxS0GyVfqT-nCYrQ==/com.tencent.mobileqq-xJKJPVp9lorkCgR_w5zhyA==/lib/arm64".into()
            }
        }


        panic!("MyJni get_field_value: class={}, field={}, signature={}", class.name, field.name, field.signature);
    }

    fn set_field_value(&mut self, vm: &mut DalvikVM64<T>, class: &Rc<DvmClass>, field: &DvmField, instance: Option<&mut DvmObject>, value: JniValue) {
        if class.name == "com/tencent/mobileqq/sign/QQSecuritySign$SignResult" {
            let result = match instance.unwrap() {
                DvmObject::DataMutInstance(_, value) => unsafe { &mut *value.get() }.downcast_mut::<SignResult>().unwrap(),
                _ => unreachable!()
            };
            let value = match value {
                JniValue::Object(object) => object,
                _ => unreachable!()
            };
            if field.name == "token" {
                result.token = match value {
                    DvmObject::ByteArray(bytes) => bytes,
                    _ => unreachable!()
                };
                return;
            }
            if field.name == "extra" {
                result.extra = match value {
                    DvmObject::ByteArray(bytes) => bytes,
                    _ => unreachable!()
                };
                return;
            }
            if field.name == "sign" {
                result.sign = match value {
                    DvmObject::ByteArray(bytes) => bytes,
                    _ => unreachable!()
                };
                return;
            }
        }

        panic!("MyJni set_field_value: class={}, field={}, signature={}", class.name, field.name, field.signature);
    }

    fn destroy(&mut self) {
    }
}
