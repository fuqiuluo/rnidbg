use std::collections::HashMap;
use log::error;
use crate::emulator::{AndroidEmulator, RcUnsafeCell};
use crate::linux::LinuxModule;
use crate::memory::svc_memory::{Arm64Svc, SvcCallResult};

const SO_NAME: &'static str = "libjnigraphics.so";

struct AndroidBitmapGetInfo; //AndroidBitmap_getInfo
struct AndroidBitmapLockPixels; //AndroidBitmap_lockPixels
struct AndroidBitmapUnlockPixels; //AndroidBitmap_unlockPixels

impl<T: Clone> Arm64Svc<T> for AndroidBitmapGetInfo {
    fn name(&self) -> &str {
        "AndroidBitmapGetInfo"
    }

    fn handle(&self, emu: &AndroidEmulator<T>) -> SvcCallResult {
        panic!("AndroidBitmapGetInfo unimplemented");
    }
}

impl<T: Clone> Arm64Svc<T> for AndroidBitmapLockPixels {
    fn name(&self) -> &str {
        "AndroidBitmapLockPixels"
    }

    fn handle(&self, emu: &AndroidEmulator<T>) -> SvcCallResult {
        panic!("AndroidBitmapLockPixels unimplemented");
    }
}

impl<T: Clone> Arm64Svc<T> for AndroidBitmapUnlockPixels {
    fn name(&self) -> &str {
        "AndroidBitmapUnlockPixels"
    }

    fn handle(&self, emu: &AndroidEmulator<T>) -> SvcCallResult {
        panic!("AndroidBitmapUnlockPixels unimplemented");
    }
}

pub fn register_jni_graphics<'a, T: Clone>(emu: &mut AndroidEmulator<'a, T>) -> RcUnsafeCell<LinuxModule<'a, T>> {
    let svc = &mut emu.inner_mut().svc_memory;
    let mut symbol = HashMap::new();
    symbol.insert("AndroidBitmap_getInfo".to_string(), svc.register_svc(Box::new(AndroidBitmapGetInfo)));
    symbol.insert("AndroidBitmap_lockPixels".to_string(), svc.register_svc(Box::new(AndroidBitmapLockPixels)));
    symbol.insert("AndroidBitmap_unlockPixels".to_string(), svc.register_svc(Box::new(AndroidBitmapUnlockPixels)));
    emu.inner_mut().memory
        .load_virtual_module(SO_NAME.to_string(), symbol)
}