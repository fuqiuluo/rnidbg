use anyhow::anyhow;
use bytes::{BufMut, Bytes, BytesMut};
use crate::backend::Backend;
use crate::emulator::AndroidEmulator;

#[derive(Clone)]
pub struct VMPointer<'a, T: Clone> {
    pub addr: u64,
    pub size: usize,
    pub backend: Option<Backend<'a, T>>,
}

impl<'a, T: Clone> VMPointer<'a, T> {
    pub fn null() -> Self {
        VMPointer {
            addr: 0,
            size: 0,
            backend: None,
        }
    }

    pub fn new(addr: u64, size: usize, backend: Backend<'a, T>) -> VMPointer<'a, T> {
        VMPointer {
            addr,
            size,
            backend: Some(backend),
        }
    }

    pub fn share(&self, offset: i64) -> Self {
        Self {
            addr: (self.addr as i64 + offset) as u64,
            size: 0,
            backend: self.backend.clone()
        }
    }

    pub fn share_with_size(&self, offset: i64, size: usize) -> Self {
        Self {
            addr: (self.addr as i64 + offset) as u64,
            size,
            backend: self.backend.clone()
        }
    }

    pub fn from_svc_string(emu: &AndroidEmulator<'a, T>, s: &str) -> anyhow::Result<VMPointer<'a, T>> {
        let bytes = s.as_bytes();
        let mut buf = BytesMut::with_capacity(bytes.len());
        buf.put(bytes);
        let ptr = emu.inner_mut()
            .svc_memory
            .allocate(bytes.len(), &format!("Module.path: {}", s));
        ptr.write_bytes(buf.freeze())?;
        Ok(ptr)
    }

    /// 填充内存
    pub fn fill_memory(&self, emu: &mut AndroidEmulator<'a, T>, offset: u64, length: usize, v: u8) -> anyhow::Result<()> {
        let mut buf = BytesMut::with_capacity(length);
        buf.fill(v);
        emu.backend.mem_write(self.addr + offset, &buf)
            .map_err(|e| anyhow!("failed to set_memory to memory: {:?}", e))?;
        Ok(())
    }

    pub fn read_i32_with_offset(&self, offset: i64) -> anyhow::Result<i32> {
        let mut buf = [0u8; 4];
        if let Some(backend) = &self.backend {
           backend.mem_read((self.addr as i64 + offset) as u64, &mut buf)
                .map_err(|e| anyhow!("failed to read_i32_with_offset to memory: {:?}", e))?;
            return Ok(i32::from_le_bytes(buf));
        }
        Err(anyhow!("failed to read_i32_with_offset to memory: backend is None"))
    }

    pub fn read_i32_with_offset_ne(&self, offset: i64) -> anyhow::Result<i32> {
        let mut buf = [0u8; 4];
        if let Some(backend) = &self.backend {
            backend.mem_read((self.addr as i64 + offset) as u64, &mut buf)
                .map_err(|e| anyhow!("failed to read_i32_with_offset_ne to memory: {:?}", e))?;
            return Ok(i32::from_ne_bytes(buf));
        }
        Err(anyhow!("failed to read_i32_with_offset_ne to memory: backend is None"))
    }

    pub fn read_u32_with_offset(&self, offset: i64) -> anyhow::Result<u32> {
        let mut buf = [0u8; 4];
        if let Some(backend) = &self.backend {
            backend.mem_read((self.addr as i64 + offset) as u64, &mut buf)
                .map_err(|e| anyhow!("failed to read_u32_with_offset to memory: {:?}", e))?;
            return Ok(u32::from_le_bytes(buf));
        }
        Err(anyhow!("failed to read_u32_with_offset to memory: backend is None"))
    }

    pub fn read_u64_with_offset(&self, offset: i64) -> anyhow::Result<u64> {
        let mut buf = [0u8; 8];
        if let Some(backend) = &self.backend {
            backend.mem_read((self.addr as i64 + offset) as u64, &mut buf)
                .map_err(|e| anyhow!("failed to read_u64_with_offset to memory: {:?}", e))?;
            return Ok(u64::from_le_bytes(buf));
        }
        Err(anyhow!("failed to read_u64_with_offset to memory: backend is None"))
    }

    pub fn read_f32_with_offset(&self, offset: i64) -> anyhow::Result<f32> {
        let mut buf = [0u8; 4];
        if let Some(backend) = &self.backend {
            backend.mem_read((self.addr as i64 + offset) as u64, &mut buf)
                .map_err(|e| anyhow!("failed to read_f32_with_offset to memory: {:?}", e))?;
            return Ok(f32::from_le_bytes(buf));
        }
        Err(anyhow!("failed to read_f32_with_offset to memory: backend is None"))
    }

    pub fn read_f64_with_offset(&self, offset: i64) -> anyhow::Result<f64> {
        let mut buf = [0u8; 8];
        if let Some(backend) = &self.backend {
            backend.mem_read((self.addr as i64 + offset) as u64, &mut buf)
                .map_err(|e| anyhow!("failed to read_f64_with_offset to memory: {:?}", e))?;
            return Ok(f64::from_le_bytes(buf));
        }
        Err(anyhow!("failed to read_f64_with_offset to memory: backend is None"))
    }

    pub fn read_i64_with_offset(&self, offset: i64) -> anyhow::Result<i64> {
        let mut buf = [0u8; 8];
        if let Some(backend) = &self.backend {
            backend.mem_read((self.addr as i64 + offset) as u64, &mut buf)
                .map_err(|e| anyhow!("failed to read_i64_with_offset to memory: {:?}", e))?;
            return Ok(i64::from_le_bytes(buf));
        }
        Err(anyhow!("failed to read_i64_with_offset to memory: backend is None"))
    }

    pub fn read_bytes(&self) -> anyhow::Result<Vec<u8>> {
        if let Some(backend) = &self.backend {
            let mut buf = vec![0u8; self.size];
            backend.mem_read(self.addr, &mut buf)
                .map_err(|e| anyhow!("failed to read_bytes from memory: {:?}", e))?;
            return Ok(buf);
        }
        Err(anyhow!("failed to read_bytes from memory: backend is None"))
    }

    pub fn read_bytes_with_len(&self, len: usize) -> anyhow::Result<Vec<u8>> {
        if let Some(backend) = &self.backend {
            let mut buf = vec![0u8; len];
            backend.mem_read(self.addr, &mut buf)
                .map_err(|e| anyhow!("failed to read_bytes_with_len from memory: {:?}", e))?;
            return Ok(buf);
        }
        Err(anyhow!("failed to read_bytes_with_len from memory: backend is None"))
    }

    pub fn read_string(&self) -> anyhow::Result<String> {
        let mut buf = BytesMut::new();
        let mut offset = 0;
        let mut b = vec![0u8; 1];
        let backend = self.backend.as_ref().unwrap();
        loop {
            backend.mem_read(self.addr + offset, &mut b)
                .map_err(|e| anyhow!("failed to read_string from memory: {:?}", e))?;
            if b[0] == 0 {
                break;
            }
            buf.extend_from_slice(&b);
            offset += 1;
        }
        unsafe { Ok(String::from_utf8_unchecked(buf.to_vec())) }
    }

    pub fn write_string(&self, s: &str) -> anyhow::Result<()> {
        let bytes = s.as_bytes();
        let mut buf = BytesMut::with_capacity(bytes.len());
        buf.put(bytes);
        if let Some(unicorn) = &self.backend {
            unicorn.mem_write(self.addr, &buf)
                .map_err(|e| anyhow!("failed to write_string to memory: {:?}", e))?;
        } else {
            return Err(anyhow!("failed to write_string to memory: backend is None"));
        }
        Ok(())
    }

    pub fn write_c_string(&self, s: &str) -> anyhow::Result<()> {
        let bytes = s.as_bytes();
        let mut buf = BytesMut::with_capacity(bytes.len() + 1);
        buf.put(bytes);
        buf.put_u8(0);
        if let Some(unicorn) = &self.backend {
            unicorn.mem_write(self.addr, &buf)
                .map_err(|e| anyhow!("failed to write_c_string to memory: {:?}", e))?;
        } else {
            return Err(anyhow!("failed to write_c_string to memory: backend is None"));
        }
        Ok(())
    }

    pub fn write_u16_with_offset(&self, offset: u64, value: u16) -> anyhow::Result<()> {
        if let Some(unicorn) = &self.backend {
            unicorn.mem_write(self.addr + offset, &value.to_le_bytes())
                .map_err(|e| anyhow!("failed to write_u16 to memory: {:?}", e))?;
            Ok(())
        } else {
            Err(anyhow!("failed to write_u16 to memory: backend is None"))
        }
    }

    pub fn write_i32_with_offset(&self, offset: u64, value: i32) -> anyhow::Result<()> {
        if let Some(unicorn) = &self.backend {
            unicorn.mem_write(self.addr + offset, &value.to_le_bytes())
                .map_err(|e| anyhow!("failed to write_i32 to memory: {:?}", e))?;
        } else {
            return Err(anyhow!("failed to write_i32 to memory: backend is None"));
        }
        Ok(())
    }

    pub fn write_u64(&self, value: u64) -> anyhow::Result<()> {
        if let Some(unicorn) = &self.backend {
            unicorn.mem_write(self.addr, &value.to_le_bytes())
                .map_err(|e| anyhow!("failed to write_u64 to memory: {:?}", e))?;
        } else {
            return Err(anyhow!("failed to write_u64 to memory: backend is None"));
        }
        Ok(())
    }

    pub fn write_u64_with_offset(&self, offset: u64, value: u64) -> anyhow::Result<()> {
        if let Some(unicorn) = &self.backend {
            unicorn.mem_write(self.addr + offset, &value.to_le_bytes())
                .map_err(|e| anyhow!("failed to write_u64 to memory: {:?}", e))?;
        } else {
            return Err(anyhow!("failed to write_u64 to memory: backend is None"));
        }
        Ok(())
    }

    pub fn write_bytes(&self, bytes: Bytes) -> anyhow::Result<()> {
        if let Some(unicorn) = &self.backend {
            unicorn.mem_write(self.addr, &bytes)
                .map_err(|e| anyhow!("failed to write_bytes to memory: {:?}", e))?;
        } else {
            return Err(anyhow!("failed to write_bytes to memory: backend is None"));
        }
        Ok(())
    }

    pub fn write_buf(&self, buf: Vec<u8>) -> anyhow::Result<()> {
        if let Some(unicorn) = &self.backend {
            unicorn.mem_write(self.addr, buf.as_slice())
                .map_err(|e| anyhow!("failed to write_buf to memory: {:?}", e))?;
        } else {
            return Err(anyhow!("failed to write_buf to memory: backend is None"));
        }
        Ok(())
    }

    pub fn write_data(&self, buf: &[u8]) -> anyhow::Result<()> {
        if let Some(unicorn) = &self.backend {
            unicorn.mem_write(self.addr, buf)
                .map_err(|e| anyhow!("failed to write_data to memory: {:?}", e))?;
        } else {
            return Err(anyhow!("failed to write_data to memory: backend is None"));
        }
        Ok(())
    }

    pub fn write_bytes_with_offset(&self, offset: u64, bytes: Bytes) -> anyhow::Result<()> {
        if let Some(unicorn) = &self.backend {
            unicorn.mem_write(self.addr + offset, &bytes)
                .map_err(|e| anyhow!("failed to write_bytes to memory: {:?}", e))?;
        } else {
            return Err(anyhow!("failed to write_bytes to memory: backend is None"));
        }
        Ok(())
    }

    pub fn dump(&self) -> anyhow::Result<()> {
        let mut buf = self.read_bytes()?;

        for chunk in buf.chunks(4).collect::<Vec<&[u8]>>() {
            println!("[{}]", hex::encode(chunk));
        };

        Ok(())
    }
}