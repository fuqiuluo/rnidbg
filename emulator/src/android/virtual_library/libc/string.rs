use std::cmp::min;
use anyhow::anyhow;
use log::debug;
use crate::android::virtual_library::libc::system_properties::SystemPropertyGet;
use crate::backend::RegisterARM64;
use crate::emulator::AndroidEmulator;
use crate::memory::svc_memory::{Arm64Svc, SvcCallResult};
use crate::memory::svc_memory::SvcCallResult::{FUCK, RET};

pub(super) struct StrCmp;
pub(super) struct StrNCmp;
pub(super) struct StrCaseCmp;
pub(super) struct StrNCasCmp;

impl<T: Clone> Arm64Svc<T> for StrCmp {
    fn name(&self) -> &str { "strcmp" }

    fn handle(&self, emu: &AndroidEmulator<T>) -> SvcCallResult {
        let backend = &emu.backend;
        let Ok(ps1) = backend.reg_read(RegisterARM64::X0) else {
            return FUCK(anyhow!("unable to get s1 when strcmp"))
        };
        let Ok(s1) = backend.mem_read_c_string(ps1) else {
            return FUCK(anyhow!("unable to fetch s1 when strcmp"))
        };
        let Ok(ps2) = backend.reg_read(RegisterARM64::X1) else {
            return FUCK(anyhow!("unable to get s2 when strcmp"))
        };
        let Ok(s2) = backend.mem_read_c_string(ps2) else {
            return FUCK(anyhow!("unable to fetch s2 when strcmp"))
        };

        if option_env!("PRINT_STRING_LOG") == Some("1") {
            debug!("strcmp({}, {})", s1, s2);
        }

        let result = s1.cmp(&s2);

        RET(result as i64)
    }
}

impl<T: Clone> Arm64Svc<T> for StrNCmp {
    fn name(&self) -> &str { "strncmp" }

    fn handle(&self, emu: &AndroidEmulator<T>) -> SvcCallResult {
        let backend = &emu.backend;
        let Ok(ps1) = backend.reg_read(RegisterARM64::X0) else {
            return FUCK(anyhow!("unable to get s1 when strncmp"))
        };
        let Ok(s1) = backend.mem_read_c_string(ps1) else {
            return FUCK(anyhow!("unable to fetch s1 when strncmp"))
        };
        let Ok(ps2) = backend.reg_read(RegisterARM64::X1) else {
            return FUCK(anyhow!("unable to get s2 when strncmp"))
        };
        let Ok(s2) = backend.mem_read_c_string(ps2) else {
            return FUCK(anyhow!("unable to fetch s2 when strncmp"))
        };

        let n = backend.reg_read(RegisterARM64::X2).unwrap() as usize;

        let result = s1[..min(n, s1.len())].cmp(&s2[..min(n, s2.len())]);

        if option_env!("PRINT_STRING_LOG") == Some("1") {
            debug!("strncmp({}, {}, {}) => {:?}", s1, s2, n, result);
        }

        RET(result as i8 as i64)
    }
}

impl<T: Clone> Arm64Svc<T> for StrCaseCmp {
    fn name(&self) -> &str { "strcasecmp" }

    fn handle(&self, emu: &AndroidEmulator<T>) -> SvcCallResult {
        let backend = &emu.backend;
        let Ok(ps1) = backend.reg_read(RegisterARM64::X0) else {
            return FUCK(anyhow!("unable to get s1 when strcasecmp"))
        };
        let Ok(s1) = backend.mem_read_c_string(ps1) else {
            return FUCK(anyhow!("unable to fetch s1 when strcasecmp"))
        };
        let Ok(ps2) = backend.reg_read(RegisterARM64::X1) else {
            return FUCK(anyhow!("unable to get s2 when strcasecmp"))
        };
        let Ok(s2) = backend.mem_read_c_string(ps2) else {
            return FUCK(anyhow!("unable to fetch s2 when strcasecmp"))
        };
        if option_env!("PRINT_STRING_LOG") == Some("1") {
            debug!("strcasecmp({}, {})", s1, s2);
        }

        let result = s1.to_lowercase().cmp(&s2.to_lowercase());

        RET(result as i64)
    }
}

impl<T: Clone> Arm64Svc<T> for StrNCasCmp {
    fn name(&self) -> &str { "strncasecmp" }

    fn handle(&self, emu: &AndroidEmulator<T>) -> SvcCallResult {
        let backend = &emu.backend;
        let Ok(ps1) = backend.reg_read(RegisterARM64::X0) else {
            return FUCK(anyhow!("unable to get s1 when strncasecmp"))
        };
        let Ok(s1) = backend.mem_read_c_string(ps1) else {
            return FUCK(anyhow!("unable to fetch s1 when strncasecmp"))
        };
        let Ok(ps2) = backend.reg_read(RegisterARM64::X1) else {
            return FUCK(anyhow!("unable to get s2 when strncasecmp"))
        };
        let Ok(s2) = backend.mem_read_c_string(ps2) else {
            return FUCK(anyhow!("unable to fetch s2 when strncasecmp"))
        };
        let n = backend.reg_read(RegisterARM64::X2).unwrap() as usize;

        if option_env!("PRINT_STRING_LOG") == Some("1") {
            debug!("strncasecmp({}, {}, {})", s1, s2, n);
        }

        let result = s1.to_lowercase()[..min(n, s1.len())].cmp(&s2.to_lowercase()[..min(n, s2.len())]);

        RET(result as i64)
    }
}

