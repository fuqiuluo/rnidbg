use std::collections::{HashMap, VecDeque};
use std::fmt::format;
use std::fs::File;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use log::{error, info};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::{fs, result, thread};
use std::thread::{sleep, sleep_ms};
use std::time::{Duration, Instant};
use bytes::Buf;
use emulator::android::dvm::class_resolver::ClassResolver;
use emulator::android::virtual_library::libc::Libc;
use emulator::android::jni::{Jni, MethodAcc, VaList, JniValue};
use emulator::android::dvm::object::DvmObject;
use emulator::android::dvm::DalvikVM64;
use emulator::AndroidEmulator;
use emulator::linux::file_system::{FileIO, FileIOTrait, StMode};
use emulator::linux::fs::linux_file::LinuxFileIO;
use emulator::linux::fs::ByteArrayFileIO;
use emulator::linux::errno::Errno;
use emulator::linux::fs::direction::Direction;
use emulator::linux::structs::OFlag;
use emulator::linux::fs::direction::DirectionEntry;
use generic_array::typenum::op;
use generic_array::typenum::private::IsEqualPrivate;
use log::__private_api::loc;
use once_cell::sync::Lazy;

const PID: u32 = 2667;
const PPID: u32 = 2427;

// pub fn create_vm<T: Clone + Send>(
//     base_path: &str,
//     data: T
// ) -> (Arc<Mutex<AndroidEmulator<'static, T>>>, Arc<AtomicBool>) {
//     let emulator = AndroidEmulator::create_arm64(PID, PPID, "com.tencent.mobileqq:MSF", data);
//     let memory = emulator.memory();
//
//     let mut libc = Box::new(Libc::new());
//     libc.set_system_property_service(Rc::new(Box::new(move |name| {
//         match name {
//             "ro.build.version.sdk" => Some(device_info.build_version.clone()),
//             "persist.sys.timezone" => Some("Asia/Shanghai".to_string()),
//             "ro.build.fingerprint" => Some(format!(
//                 "{}/{}/{}:{}/{}/{}:user/release-keys",
//                 device_info.manufacturer,
//                 device_info.code,
//                 device_info.code,
//                 device_info.os_version,
//                 device_info.fingerprint,
//                 device_info.rom_version
//             )),
//             "ro.product.name" | "ro.product.device" => Some(device_info.code.clone()),
//             "ro.product.manufacturer" | "ro.product.brand" | "ro.product.model" => Some(device_info.manufacturer.clone()),
//             "ro.hardware" => Some("unknown".to_string()),
//             "ro.product.cpu.abi" | "ro.product.cpu.abilist" => Some("arm64-v8a".to_string()),
//             "ro.boot.hardware" => Some("qcom".to_string()),
//             "ro.bluetooth.address" => Some("".to_string()),
//             "ro.recovery_id" => Some("0x11451419".to_string()),
//             "gsm.version.baseband" => Some("MOLY.LR12A.R3.MP.V79.P10".into()),
//             _ => {
//                 panic!("name={}", name)
//             }
//         }
//     })));
//
//     memory.add_hook_listeners(libc);
//     init_filesystem(base_path.to_string(), &emulator);
//
//     let vm = emulator.get_dalvik_vm();
//     vm.set_class_resolver(ClassResolver::new(vec![
//         "com/tencent/mobileqq/qsec/qsecprotocol/ByteData",
//         "com/tencent/mobileqq/qsec/qsecdandelionsdk/Dandelion",
//         "com/tencent/mobileqq/qsec/qseccodec/SecCipher",
//         "com/tencent/mobileqq/qsec/qsecurity/QSec",
//         "com/tencent/mobileqq/sign/QQSecuritySign$SignResult",
//         "com/tencent/mobileqq/sign/QQSecuritySign",
//         "com/tencent/mobileqq/channel/ChannelManager",
//         "com/tencent/mobileqq/dt/app/Dtc",
//         "com/tencent/mobileqq/fe/IFEKitLog",
//         "com/tencent/mobileqq/dt/model/FEBound",
//         "com/tencent/mobileqq/fe/utils/DeepSleepDetector",
//         "com/tencent/mobileqq/dt/Dtn",
//         "com/tencent/mobileqq/qsec/qsecest/QsecEst",
//         "com/tencent/mobileqq/fe/EventCallback",
//         "java/lang/Thread",
//         "java/lang/StackTraceElement",
//         "android/content/Context",
//         "com/tencent/mobileqq/channel/ChannelProxy",
//         "android/content/ContentResolver",
//         "com/tencent/mobileqq/qsec/qsecurity/QSecConfig",
//         "android/os/Build$VERSION",
//         "android/content/pm/ApplicationInfo",
//         "java/io/File",
//         "java/util/UUID",
//         "com/tencent/qqprotect/qsec/QSecFramework",
//         "android/provider/Settings$System",
//         "java/lang/ClassLoader",
//         "android/os/Process",
//         "dalvik/system/BaseDexClassLoader",
//         "android/content/pm/IPackageManager$Stub",
//         "android/content/pm/IPackageManager",
//         "android/os/ServiceManager",
//         "android/os/IBinder",
//         "android/content/pm/PackageManager",
//         "android/content/pm/PackageManager$NameNotFoundException"
//     ]));
//
//     let locked_emulator = Arc::new(Mutex::new(emulator.clone()));
//     let mut jni = Box::new(TestJni::new(business.clone(), device.clone()));
//     let is_sign = jni.is_sign.clone();
//     vm.set_jni(jni);
//
//     let dm = vm.load_library(emulator.clone(), &format!("{}/libfekit.so", base_path), true)
//         .map_err(|e| eprintln!("failed to load_library: {}", e))
//         .unwrap();
//
//     let dm = unsafe { &*dm.get() };
//     if let Err(e) = vm.call_jni_onload(emulator.clone(), &dm) {
//         info!("failed to call_jni_onload: {}", e);
//         panic!("failed to call_jni_onload");
//     }
//
//     let _ = memory;
//     let _ = vm;
//     let _ = dm;
//
//     if let Ok(mut lock) = locked_emulator.lock() {
//         let emulator = lock.deref_mut();
//         let vm = emulator.get_dalvik_vm();
//
//         init_fekit(business.uin.clone(), business.qua.clone(), device.q36.clone(), device.guid.clone(), device, emulator, vm);
//
//         lock.get_current_pid();
//     } else {
//         panic!("failed to lock emulator");
//     }
//
//     (locked_emulator, is_sign)
// }
//
// fn init_filesystem<T: Clone>(base_path: String, emulator: &AndroidEmulator<T>) {
//     let system_base_path = emulator.get_base_path().to_string();
//     let file_system = emulator.get_file_system();
//
//     let _ = fs::remove_file(format!("{}/6FAcBa17D93747A5", base_path));
//     let _ = fs::remove_file(format!("{}/android_lq", base_path));
//
//     file_system.set_file_resolver(Box::new(move |file_system, path, flags, mode| {
//         if path == "/data/user/0/com.tencent.mobileqq/files/5463306EE50FE3AA/6FAcBa17D93747A5" {
//             if flags == OFlag::O_RDONLY {
//                 if let Ok(file) = File::open(format!("{}/6FAcBa17D93747A5", base_path)) {
//                     return Some(FileIO::File(LinuxFileIO::new_with_file(
//                         file,
//                         path,
//                         flags.bits(),
//                         12345,
//                         StMode::APP_FILE
//                     )));
//                 }
//                 return None;
//             } else if flags == (OFlag::O_WRONLY | OFlag::O_CREAT | OFlag::O_TRUNC) {
//                 let file = File::create(format!("{}/6FAcBa17D93747A5", base_path)).unwrap();
//                 return Some(FileIO::File(LinuxFileIO::new_with_file(
//                     file,
//                     path,
//                     flags.bits(),
//                     12345,
//                     StMode::APP_FILE
//                 )));
//             } else {
//                 panic!("unsupported flags: {:?}", flags);
//             }
//         }
//         if path == "/sdcard/Android/.android_lq" {
//             return None
//         }
//         if path == "/data/user/0/com.tencent.mobileqq/files/.android_lq" {
//             if flags == OFlag::O_RDONLY {
//                 if let Ok(file) = File::open(format!("{}/android_lq", base_path)) {
//                     return Some(FileIO::File(LinuxFileIO::new_with_file(
//                         file,
//                         path,
//                         flags.bits(),
//                         12345,
//                         StMode::APP_FILE
//                     )));
//                 }
//                 return None;
//             } else if flags == (OFlag::O_RDWR | OFlag::O_CREAT) {
//                 return if let Ok(file) = File::options()
//                     .read(true)
//                     .write(true)
//                     .open(format!("{}/android_lq", base_path)) {
//                     Some(FileIO::File(LinuxFileIO::new_with_file(
//                         file,
//                         path,
//                         flags.bits(),
//                         12345,
//                         StMode::APP_FILE
//                     )))
//                 } else {
//                     let file = File::create(format!("{}/android_lq", base_path)).unwrap();
//                     Some(FileIO::File(LinuxFileIO::new_with_file(
//                         file,
//                         path,
//                         flags.bits(),
//                         12345,
//                         StMode::APP_FILE
//                     )))
//                 }
//             } else if flags == OFlag::O_RDWR {
//                 // open for read and write
//                 if let Ok(file) = File::options()
//                     .read(true)
//                     .write(true)
//                     .open(format!("{}/android_lq", base_path)) {
//                     return Some(FileIO::File(LinuxFileIO::new_with_file(
//                         file,
//                         path,
//                         flags.bits(),
//                         12345,
//                         StMode::APP_FILE
//                     )));
//                 }
//                 return None;
//             } else {
//                 return None;
//             }
//         }
//
//
//         if path == "/sdcard/Android/" {
//             return Some(FileIO::Direction(Direction::new(VecDeque::new(), path)))
//         }
//
//
//         if path == "/data/app/~~vbcRLwPxS0GyVfqT-nCYrQ==/com.tencent.mobileqq-xJKJPVp9lorkCgR_w5zhyA==/base.apk" {
//             return Some(FileIO::File(LinuxFileIO::new(
//                 &format!("{}/base.apk", base_path),
//                 path,
//                 flags.bits(),
//                 12345,
//                 StMode::APP_FILE
//             )))
//         }
//
//         if path == "/proc/self/cmdline" || path == format!("/proc/{}/cmdline", PID) {
//             return Some(FileIO::Bytes(ByteArrayFileIO::new(b"com.tencent.mobileqq:MSF\0".to_vec(), path.to_string(), 12345, flags.bits(), StMode::APP_FILE)))
//         }
//
//
//         if path == "/data/app/~~vbcRLwPxS0GyVfqT-nCYrQ==/com.tencent.mobileqq-xJKJPVp9lorkCgR_w5zhyA==/lib/arm64/libfekit.so" {
//             return Some(FileIO::File(LinuxFileIO::new(
//                 &format!("{}/libfekit.so", base_path),
//                 path,
//                 flags.bits(),
//                 12345,
//                 StMode::APP_FILE
//             )))
//         }
//
//
//
//         if path == "/data/data/com.tencent.mobileqq" {
//             let mut deque = VecDeque::new();
//             deque.push_back(DirectionEntry::new(false, "files"));
//             deque.push_back(DirectionEntry::new(false, "shared_prefs"));
//             deque.push_back(DirectionEntry::new(false, "cache"));
//             deque.push_back(DirectionEntry::new(false, "code_cache"));
//             return Some(FileIO::Direction(Direction::new(deque, path)))
//         }
//
//
//         if path == "/proc" {
//             let mut deque = VecDeque::new();
//             deque.push_back(DirectionEntry::new(false, &PID.to_string()));
//             deque.push_back(DirectionEntry::new(false, &PPID.to_string()));
//             deque.push_back(DirectionEntry::new(false, "self"));
//             return Some(FileIO::Direction(Direction::new(deque, path)))
//         }
//
//          if path == "/sys/devices/soc0/serial_number" {
//              return FileIO::Bytes(ByteArrayFileIO::new(
//                  b"0x0000043be8571339".to_vec(),
//                  path.to_string(),
//                  0,
//                  0,
//                  StMode::S_IRUSR | StMode::S_IRGRP | StMode::S_IROTH,
//              )).into()
//          }
//
//         if path == "/proc/self/attr/prev" {
//             return FileIO::Bytes(ByteArrayFileIO::new(
//                 b"u:r:untrusted app:s0:07,c257,c512,c768".to_vec(),
//                 path.to_string(),
//                 0,
//                 flags.bits(),
//                 StMode::SYSTEM_FILE,
//             )).into()
//         }
//
//         panic!("unresolved file_resolver: path={}, flags={:?}, mode={}", path, flags, mode);
//     }));
// }
//
// fn init_fekit<T: Clone>(
//     uin: String,
//     qua: String,
//     qimei36: String,
//     guid: String,
//     device: Rc<Device>,
//     emulator: &AndroidEmulator<T>,
//     vm: &mut DalvikVM64<T>
// ) {
//     let version = "6.100.248".to_string();
//     let android_os = device.os_version.clone();
//     let brand = device.manufacturer.clone();
//     let model = device.model.clone();
//
//     let qq_security_sign = vm.resolve_class_unchecked("com/tencent/mobileqq/sign/QQSecuritySign").1
//         .new_simple_instance(vm);
//     qq_security_sign.call_method(emulator, vm, "initSafeMode", "(Z)V", vec![false.into()]);
//
//     let event_callback = vm.resolve_class_unchecked("com/tencent/mobileqq/fe/EventCallback").1
//         .new_simple_instance(vm);
//     qq_security_sign.call_method(emulator, vm, "dispatchEvent", "(Ljava/lang/String;Ljava/lang/String;Lcom/tencent/mobileqq/fe/EventCallback;)V", vec![
//         "Kicked".into(), uin.clone().into(), JniValue::Object(event_callback)
//     ]);
//
//     let context = vm.resolve_class_unchecked("android/content/Context").1
//         .new_simple_instance(vm);
//
//     let dtn = vm.resolve_class_unchecked("com/tencent/mobileqq/dt/Dtn").1
//         .new_simple_instance(vm);
//     dtn.call_method(emulator, vm, "initContext", "(Landroid/content/Context;Ljava/lang/String;)V", vec![
//         JniValue::Object(context.clone()),
//         "/data/user/0/com.tencent.mobileqq/files/5463306EE50FE3AA".into()
//     ]);
//
//     let fekit_log = vm.resolve_class_unchecked("com/tencent/mobileqq/fe/IFEKitLog").1
//         .new_simple_instance(vm);
//     dtn.call_method(emulator, vm, "initLog", "(Lcom/tencent/mobileqq/fe/IFEKitLog;)V", vec![
//         JniValue::Object(fekit_log)
//     ]);
//     dtn.call_method(emulator, vm, "initUin", "(Ljava/lang/String;)V", vec![
//         uin.clone().into()
//     ]);
//
//     let qsec = vm.resolve_class_unchecked("com/tencent/mobileqq/qsec/qsecurity/QSec").1
//         .new_simple_instance(vm);
//
//     qsec.call_method(emulator, vm, "doSomething", "(Landroid/content/Context;I)I", vec![
//         JniValue::Object(context.clone()),
//         1.into()
//     ]);
//
//     let channel_proxy = vm.resolve_class_unchecked("com/tencent/mobileqq/channel/ChannelProxy").1
//         .new_simple_instance(vm);
//     let channel_manager = vm.resolve_class_unchecked("com/tencent/mobileqq/channel/ChannelManager").1
//         .new_simple_instance(vm);
//     channel_manager.call_method(emulator, vm, "setChannelProxy", "(Lcom/tencent/mobileqq/channel/ChannelProxy;)V", vec![
//         JniValue::Object(channel_proxy)
//     ]);
//
//     channel_manager.call_method(emulator, vm, "initReport", "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V", vec![
//         qua.clone().into(),
//         version.clone().into(),
//         android_os.clone().into(),
//         (brand.clone() + model.as_str()).into(),
//         qimei36.clone().into(),
//         guid.clone().into()
//     ]);
//
//     if let DvmObject::ObjectRef(ref_id) = qq_security_sign {
//         vm.remove_global_ref(ref_id);
//     }
//     if let DvmObject::ObjectRef(ref_id) = qsec {
//         vm.remove_global_ref(ref_id);
//     }
//     if let DvmObject::ObjectRef(ref_id) = channel_manager {
//         vm.remove_global_ref(ref_id);
//     }
// }
