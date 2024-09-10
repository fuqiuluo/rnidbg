use std::fmt::{Display, Formatter};
use std::rc::Rc;
use crate::emulator::AndroidEmulator;

#[derive(Clone)]
pub enum LibraryFile {
    Elf(ElfLibraryFile)
}

pub trait LibraryFileTrait {
    fn name(&self) -> String;

    fn map_region_name(&self) -> String;

    fn resolve_library(&self, so_name: &str) -> anyhow::Result<LibraryFile>;

    fn map_buffer(&self) -> anyhow::Result<Vec<u8>>;

    fn path(&self) -> String;

    fn real_path(&self) -> String;

    fn file_size(&self) -> u64;
}

#[derive(Clone)]
pub struct ElfLibraryFile {
    buffer: Rc<Vec<u8>>,
    /// path: /etc/data/libfekit.so
    /// name: libfekit.so
    /// region_name: /system/bin/lib64/libfekit.so
    /// path: /system/bin/lib64/libfekit.so
    path: String,
    system: bool,
    package_name: Option<String>
}

impl ElfLibraryFile {
    pub fn new(buffer: Vec<u8>, path: String) -> ElfLibraryFile {
        ElfLibraryFile {
            buffer: Rc::new(buffer),
            path,
            system: true,
            package_name: None
        }
    }

    pub fn new_application_library(buffer: Vec<u8>, path: String, package_name: String) -> ElfLibraryFile {
        ElfLibraryFile {
            buffer: Rc::new(buffer),
            path,
            system: false,
            package_name: Some(package_name)
        }
    }

    pub fn name(&self) -> String {
        self.path.split('/').last().unwrap().to_string()
    }

    pub fn resolve_library(&self, so_name: &str) -> anyhow::Result<LibraryFile> {
        let mut path = self.path.clone();
        let mut path = path.split('/').collect::<Vec<&str>>();
        path.pop();
        path.push(so_name);
        let path = path.join("/");
        let buffer = std::fs::read(&path)?;
        Ok(LibraryFile::Elf(ElfLibraryFile::new(buffer, path)))
    }

    pub fn map_buffer(&self) -> anyhow::Result<Vec<u8>> {
        Ok(Vec::clone(self.buffer.as_ref()))
    }

    pub fn vm_path(&self) -> String {
        if self.system {
            format!("/system/lib64/{}", self.name())
        } else {
            format!("/data/app/~~YuanShenZhenChaoHaoWan/{}-0/lib/arm64/{}", self.package_name.as_ref().unwrap(), self.name())
        }
    }
}

impl LibraryFileTrait for LibraryFile {
    fn name(&self) -> String {
        match self {
            LibraryFile::Elf(elf) => elf.name()
        }
    }

    fn map_region_name(&self) -> String {
        match self {
            LibraryFile::Elf(elf) => elf.vm_path()
        }
    }

    fn resolve_library(&self, so_name: &str) -> anyhow::Result<LibraryFile> {
        match self {
            LibraryFile::Elf(elf) => elf.resolve_library(so_name)
        }
    }

    fn map_buffer(&self) -> anyhow::Result<Vec<u8>> {
        match self {
            LibraryFile::Elf(elf) => elf.map_buffer()
        }
    }

    fn path(&self) -> String {
        match self {
            LibraryFile::Elf(elf) => elf.vm_path()
        }
    }

    fn real_path(&self) -> String {
        match self {
            LibraryFile::Elf(elf) => elf.path.clone()
        }
    }

    fn file_size(&self) -> u64 {
        match self {
            LibraryFile::Elf(elf) => elf.buffer.len() as u64
        }
    }
}