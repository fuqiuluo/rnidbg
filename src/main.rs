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

mod jni;
mod vm;
mod utils;

fn main() {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();


}
