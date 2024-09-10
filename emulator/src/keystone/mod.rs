use anyhow::anyhow;
use bytes::BytesMut;
use keystone_engine::{Arch, Keystone, Mode, OptionType, OptionValue};
use crate::emulator::POST_CALLBACK_SYSCALL_NUMBER;
use crate::keystone;

pub fn assemble_no_check(asm: &str) -> Vec<u8> {
    let engine =
        Keystone::new(Arch::ARM64, Mode::LITTLE_ENDIAN)
            .unwrap();

    let result = engine
        .asm(asm.to_string(), 0)
        .unwrap();

    result.bytes
}

pub fn assemble_no_check_v2(asm: &str, addr: u64) -> Vec<u8> {
    let engine =
        Keystone::new(Arch::ARM64, Mode::LITTLE_ENDIAN)
            .unwrap();

    let result = engine
        .asm(asm.to_string(), addr)
        .unwrap();

    result.bytes
}

pub fn assemble(asm: &str) -> anyhow::Result<Vec<u8>> {
    let engine = Keystone::new(Arch::ARM64, Mode::LITTLE_ENDIAN)?;

    let result = engine
        .asm(asm.to_string(), 0)
        .map_err(|e| anyhow!("\"{}\" assemble failed: {}", asm, e))?;

    Ok(result.bytes)
}

#[test]
fn test_assemble() {
    let code = [
        "sub sp, sp, #0x10",
        "stp x29, x30, [sp]",
        &format!("svc #0x{:X}", 0xfff),
        "ldr x13, [sp]",
        "add sp, sp, #0x8",
        "cmp x13, #0",
        "b.eq #0x58",
        "ldr x0, [sp]",
        "add sp, sp, #0x8",
        "ldr x1, [sp]",
        "add sp, sp, #0x8",
        "ldr x2, [sp]",
        "add sp, sp, #0x8",
        "blr x13",
        "cmp w0, #0",
        "b.eq #0xc",
        "ldr x13, [sp]",
        "add sp, sp, #0x8",
        "cmp x13, #0",
        "b.eq #0x58",
        "add sp, sp, #0x18",
        "b 0x40",
        "mov x8, #0",
        &format!("mov x12, #0x{:x}", 0xfff),
        &format!("mov x16, #0x{:x}", POST_CALLBACK_SYSCALL_NUMBER),
        "svc #0",
        "ldp x29, x30, [sp]",
        "add sp, sp, #0x10",
        "ret"
    ];
    let code = code.map(|code|
        assemble(code).map_err(|e| eprintln!("{}", e)).unwrap()
    );

    let mut merged = BytesMut::new();
    for c in code {
        merged.extend_from_slice(&c);
    }

    let code = merged.freeze();

    println!("Code len: {}", code.len());

    assert_eq!(code.len(), 29 * 4);
}