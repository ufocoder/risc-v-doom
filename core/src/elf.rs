use crate::memory::{Memory, Trap};
#[derive(Debug)]
pub enum ElfError {
    TooShort,
    BadMagic,
    Unsupported(&'static str),
    Bounds,
    Memory(Trap),
}
impl From<Trap> for ElfError {
    fn from(v: Trap) -> Self {
        Self::Memory(v)
    }
}
fn u16le(d: &[u8], o: usize) -> Result<u16, ElfError> {
    Ok(u16::from_le_bytes(
        d.get(o..o + 2).ok_or(ElfError::Bounds)?.try_into().unwrap(),
    ))
}
fn u32le(d: &[u8], o: usize) -> Result<u32, ElfError> {
    Ok(u32::from_le_bytes(
        d.get(o..o + 4).ok_or(ElfError::Bounds)?.try_into().unwrap(),
    ))
}
pub fn load_elf(mem: &mut Memory, d: &[u8]) -> Result<(u32, u32), ElfError> {
    if d.len() < 52 {
        return Err(ElfError::TooShort);
    }
    if &d[..4] != b"\x7fELF" {
        return Err(ElfError::BadMagic);
    }
    if d[4] != 1 || d[5] != 1 {
        return Err(ElfError::Unsupported("ELF32 little-endian required"));
    }
    if u16le(d, 18)? != 243 {
        return Err(ElfError::Unsupported("RISC-V machine required"));
    }
    let entry = u32le(d, 24)?;
    let phoff = u32le(d, 28)? as usize;
    let entsz = u16le(d, 42)? as usize;
    let count = u16le(d, 44)? as usize;
    let mut image_end = 0u32;
    for i in 0..count {
        let p = phoff + i * entsz;
        if u32le(d, p)? != 1 {
            continue;
        }
        let off = u32le(d, p + 4)? as usize;
        let va = u32le(d, p + 8)?;
        let filesz = u32le(d, p + 16)? as usize;
        let memsz = u32le(d, p + 20)? as usize;
        image_end = image_end.max(va.checked_add(memsz as u32).ok_or(ElfError::Bounds)?);
        let bytes = d.get(off..off + filesz).ok_or(ElfError::Bounds)?;
        mem.load(va, bytes)?;
        if memsz > filesz {
            mem.load(va + filesz as u32, &vec![0; memsz - filesz])?
        }
    }
    Ok((entry, image_end))
}
