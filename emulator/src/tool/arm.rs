use std::mem;
use bytes::{BufMut, Bytes, BytesMut};
use log::warn;
use crate::backend::RegisterARM64;
use crate::emulator::AndroidEmulator;

#[derive(Clone, Debug, PartialEq)]
pub enum UnicornArg {
    I32(i32),
    I64(i64),
    U32(u32),
    U64(u64),
    F32(f32),
    F64(f64),
    Ptr(u64),
    Str(String),
    Vec(Vec<u8>),
}

pub fn init_args<T: Clone>(emulator: &AndroidEmulator<T>, pending: bool, arguments: Vec<UnicornArg>) {
    static ARM64_REG_ARGS: [RegisterARM64; 8] = [
        RegisterARM64::X0,
        RegisterARM64::X1,
        RegisterARM64::X2,
        RegisterARM64::X3,
        RegisterARM64::X4,
        RegisterARM64::X5,
        RegisterARM64::X6,
        RegisterARM64::X7
    ];

    let backend = emulator.backend.clone();
    let mut args_list = Vec::new();
    let mut reg_vector = RegisterARM64::Q0;
    for arg in arguments {
        match arg {
            UnicornArg::F32(f) => {
                let mut buf = BytesMut::with_capacity(16);
                buf.put_f32_le(f);
                let data = buf.to_vec();
                backend.reg_write_long(reg_vector, data.as_slice())
                    .expect("failed to reg_write_vector: float");
                reg_vector = RegisterARM64::from_i32(reg_vector.value() + 1);
                continue
            }
            UnicornArg::F64(d) => {
                let mut buf = BytesMut::with_capacity(16);
                buf.put_f64_le(d);
                let data = buf.to_vec();
                backend.reg_write_long(reg_vector, data.as_slice())
                    .expect("failed to reg_write_vector: float");
                reg_vector = RegisterARM64::from_i32(reg_vector.value() + 1);
                continue
            }
            UnicornArg::I32(i) => args_list.push(unsafe { mem::transmute::<i64, u64>(i as i64) }),
            UnicornArg::I64(l) => args_list.push(unsafe { mem::transmute::<i64, u64>(l) }),
            UnicornArg::U32(i) => args_list.push(i as u64),
            UnicornArg::U64(l) => args_list.push(l),
            UnicornArg::Ptr(p) => args_list.push(p),
            UnicornArg::Str(s) => {
                let pointer = emulator.inner_mut()
                    .memory
                    .write_stack_string(s)
                    .expect("failed to allocate stack: write_stack_string");
                args_list.push(pointer.addr);
            }
            UnicornArg::Vec(v) => {
                let pointer = emulator
                    .inner_mut()
                    .memory
                    .write_stack_bytes(Bytes::from(v))
                    .expect("failed to allocate stack: write_stack_bytes");
                args_list.push(pointer.addr);
            }
            _ => panic!("Invalid argument type")
        }
    }

    for i in 0..args_list.len() {
        let reg = ARM64_REG_ARGS[i];
        emulator.backend.reg_write(reg, args_list[i])
            .expect("failed to reg_write: args_list");
    }

    let reversed = args_list.iter().rev().collect::<Vec<_>>();

    if reversed.len() % 2 != 0 {
        emulator
            .inner_mut()
            .memory
            .allocate_stack(8);
    }

    for i in 0..reversed.len() {
        let p = reversed[i];
        let pointer = emulator.inner_mut()
            .memory
            .allocate_stack(8);
        if pointer.addr % 8 != 0 {
            warn!("initArgs pointer={}", pointer.addr);
        }
        backend.mem_write(pointer.addr, &p.to_le_bytes())
            .expect("failed to mem_write: args_list");
    }
}