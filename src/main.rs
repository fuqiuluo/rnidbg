use std::collections::HashMap;
use std::ffi::{c_char, CStr, CString};
use std::ops::Deref;
use std::rc::Rc;
use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep_ms;
use std::time::Instant;
use emulator::android::dvm::DalvikVM64;
use emulator::AndroidEmulator;
use log::{error, info, warn};
use once_cell::sync::Lazy;
use crate::business::BusinessConfig;
use crate::device::Device;

mod business;
mod jni;
mod qsec;
mod device;
mod vm;
mod utils;

fn main() {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();

    for i in 0..100 {
        let qq = 123456;
        let business = BusinessConfig::new("123456", "V1_AND_SQ_9.0.71_6702_YYB_D", "9.0.71", "6702", 537228658);
        let qimei36 = "0".repeat(36);
        let android_id = "0".repeat(16);
        let guid = "0".repeat(32);
        let device = Device::new(
            "elish",
            "Xiaomi",
            "23013RK75C",
            "29",
            "V14.0.23.5.29",
            "10",
            29,
            "RKQ1.211001.001",
            "fba9eda78468632f",
            &qimei36,
            &android_id,
            &guid,
        );
        let instance = vm::create_vm("./txlib/9.0.90", business.clone(), device, ());

        for j in 0..100 {
            test_get_sign(&instance, &business, qq);
        }

        if let Ok(emu) = instance.0.lock() {
            emu.destroy();
        }
    }


    sleep_ms(15_000);
}

fn test_get_sign<T: Clone>(instance: &(Arc<Mutex<AndroidEmulator<T>>>, Arc<AtomicBool>), business: &BusinessConfig, qq: u64) {
    instance.1.store(true, Ordering::SeqCst);
    let Ok(lock) = instance.0.lock() else {
        panic!("lock failed");
    };
    let start = Instant::now();
    let vm = lock.get_dalvik_vm();
    let Ok((sign, token, extra)) = qsec::get_sign(lock.deref(), vm, business.qua.as_str(), &qq.to_string(), "wtlogin.login", 1000, vec![0x01, 0x02, 0x03]).map_err(|e| {
        eprintln!("get_sign error: {:?}", e);
    }) else {
        panic!("get_sign failed");
    };
    info!("sign: {}, token: {}, extra: {}", sign.len(), token.len(), extra.len());
    info!("get_sign cost: {:?}", start.elapsed());
}