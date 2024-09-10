use crate::backend::Backend;
use crate::backend::RegisterARM64::CPACR_EL1;

#[cfg(feature = "unicorn_backend")]
pub fn enable_vfp<T: Clone>(backend: &Backend<T>) -> anyhow::Result<()> {
    let mut cpacr_el1 = backend.reg_read(CPACR_EL1)?;
    cpacr_el1 |= 0x300000; // set the FPEN bits
    backend.reg_write(CPACR_EL1, cpacr_el1)
        .expect("failed to enable VFP");

    Ok(())
}