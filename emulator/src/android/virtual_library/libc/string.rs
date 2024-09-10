use std::cmp::min;
use crate::android::virtual_library::libc::system_properties::SystemPropertyGet;
use crate::backend::RegisterARM64;
use crate::emulator::AndroidEmulator;
use crate::memory::svc_memory::Arm64Svc;

pub(super) struct StrCmp;
pub(super) struct StrNCmp;
pub(super) struct StrCaseCmp;
pub(super) struct StrNCasCmp;

impl<T: Clone> Arm64Svc<T> for StrCmp {
    #[cfg(feature = "show_svc_name")]
    fn name(&self) -> &str { "strcmp" }

    fn handle(&self, emu: &AndroidEmulator<T>) -> anyhow::Result<Option<i64>> {
        let backend = &emu.backend;
        let s1 = backend.mem_read_c_string(backend.reg_read(RegisterARM64::X0)?)?;
        let s2 = backend.mem_read_c_string(backend.reg_read(RegisterARM64::X1)?)?;

        if option_env!("PRINT_STRING_LOG") == Some("1") {
            println!("strcmp({}, {})", s1, s2);
        }

        let result = s1.cmp(&s2);

        Ok(Some(result as i64))
    }
}

impl<T: Clone> Arm64Svc<T> for StrNCmp {
    #[cfg(feature = "show_svc_name")]
    fn name(&self) -> &str { "strncmp" }

    fn handle(&self, emu: &AndroidEmulator<T>) -> anyhow::Result<Option<i64>> {
        let backend = &emu.backend;
        let s1 = backend.mem_read_c_string(backend.reg_read(RegisterARM64::X0)?)?;
        let s2 = backend.mem_read_c_string(backend.reg_read(RegisterARM64::X1)?)?;
        let n = backend.reg_read(RegisterARM64::X2)? as usize;

        let result = s1[..min(n, s1.len())].cmp(&s2[..min(n, s2.len())]);

        if option_env!("PRINT_STRING_LOG") == Some("1") {
            println!("strncmp({}, {}, {}) => {:?}", s1, s2, n, result);
        }

        Ok(Some(result as i8 as i64))
    }
}

impl<T: Clone> Arm64Svc<T> for StrCaseCmp {
    #[cfg(feature = "show_svc_name")]
    fn name(&self) -> &str { "strcasecmp" }

    fn handle(&self, emu: &AndroidEmulator<T>) -> anyhow::Result<Option<i64>> {
        let backend = &emu.backend;
        let s1 = backend.mem_read_c_string(backend.reg_read(RegisterARM64::X0)?)?;
        let s2 = backend.mem_read_c_string(backend.reg_read(RegisterARM64::X1)?)?;

        if option_env!("PRINT_STRING_LOG") == Some("1") {
            println!("strcasecmp({}, {})", s1, s2);
        }

        let result = s1.to_lowercase().cmp(&s2.to_lowercase());

        Ok(Some(result as i64))
    }
}

impl<T: Clone> Arm64Svc<T> for StrNCasCmp {
    #[cfg(feature = "show_svc_name")]
    fn name(&self) -> &str { "strncasecmp" }

    fn handle(&self, emu: &AndroidEmulator<T>) -> anyhow::Result<Option<i64>> {
        let backend = &emu.backend;
        let s1 = backend.mem_read_c_string(backend.reg_read(RegisterARM64::X0)?)?;
        let s2 = backend.mem_read_c_string(backend.reg_read(RegisterARM64::X1)?)?;
        let n = backend.reg_read(RegisterARM64::X2)? as usize;

        if option_env!("PRINT_STRING_LOG") == Some("1") {
            println!("strncasecmp({}, {}, {})", s1, s2, n);
        }

        let result = s1.to_lowercase()[..min(n, s1.len())].cmp(&s2.to_lowercase()[..min(n, s2.len())]);

        Ok(Some(result as i64))
    }
}

