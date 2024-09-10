use emulator::android::dvm::DalvikVM64;
use emulator::android::dvm::object::DvmObject;
use emulator::android::jni::JniValue;
use emulator::AndroidEmulator;
use crate::jni::SignResult;

pub fn get_sign<T: Clone>(
    emulator: &AndroidEmulator<T>,
    vm: &mut DalvikVM64<T>,
    qua: &str,
    uin: &str,
    cmd: &str,
    seq: i32,
    data: Vec<u8>
) -> anyhow::Result<(Vec<u8>, Vec<u8>, Vec<u8>)> {
    let qq_security_sign = vm.resolve_class_unchecked("com/tencent/mobileqq/sign/QQSecuritySign").1
        .new_simple_instance(vm);
    let qsec = vm.resolve_class_unchecked("com/tencent/mobileqq/qsec/qsecurity/QSec").1
        .new_simple_instance(vm);
    let seq = seq.to_ne_bytes().to_vec();
    let result = qq_security_sign.call_method(emulator, vm, "getSign", "(Lcom/tencent/mobileqq/qsec/qsecurity/QSec;Ljava/lang/String;Ljava/lang/String;[B[BLjava/lang/String;)Lcom/tencent/mobileqq/sign/QQSecuritySign$SignResult;", vec![
        qsec.clone().into(),
        qua.into(),
        cmd.into(),
        seq.into(),
        data.into(),
        uin.into()
    ]);
    if let DvmObject::ObjectRef(ref_id) = qq_security_sign {
        vm.remove_global_ref(ref_id);
    }
    if let DvmObject::ObjectRef(ref_id) = qsec {
        vm.remove_global_ref(ref_id);
    }

    let JniValue::Object(object) = result else {
        return Err(anyhow::anyhow!("result not a object"));
    };
    let DvmObject::DataMutInstance(_, result) = object else {
        return Err(anyhow::anyhow!("result not a DataMutInstance"));
    };
    let result = unsafe { &mut *result.get() };
    let Some(result) = result.downcast_ref::<SignResult>() else {
        return Err(anyhow::anyhow!("result not a SignResult"));
    };

    Ok((result.sign.clone(), result.token.clone(), result.extra.clone()))
}

pub fn energy<T: Clone>(
    emulator: &AndroidEmulator<T>,
    vm: &mut DalvikVM64<T>,
    data: &str,
    salt: Vec<u8>
) -> anyhow::Result<Vec<u8>> {
    let dandelion = vm.resolve_class_unchecked("com/tencent/mobileqq/qsec/qsecdandelionsdk/Dandelion").1;
    let result = dandelion.call_static_method(emulator, vm, "energy", "(Ljava/lang/Object;Ljava/lang/Object;)[B", vec![
        data.into(),
        salt.into()
    ]);
    let bytes = if let JniValue::Object(object) = result {
        if let DvmObject::ByteArray(bytes) = object {
            bytes
        } else {
            return Err(anyhow::anyhow!("result not a ByteArray"));
        }
    } else {
        return Err(anyhow::anyhow!("result not a object"));
    };
    Ok(bytes)
}
