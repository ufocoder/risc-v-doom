use crate::{Memory, Trap};

/// Platform services invoked by guest ECALL instructions.
///
/// Returning `Ok(true)` means that the call was handled. Returning
/// `Ok(false)` lets the core try its built-in, platform-neutral calls.
pub trait Host {
    fn ecall(
        &mut self,
        number: u32,
        regs: &mut [u32; 32],
        memory: &mut Memory,
        heap: &mut u32,
        halted: &mut bool,
    ) -> Result<bool, Trap>;
}

#[derive(Default)]
pub struct NullHost;

impl Host for NullHost {
    fn ecall(
        &mut self,
        _number: u32,
        _regs: &mut [u32; 32],
        _memory: &mut Memory,
        _heap: &mut u32,
        _halted: &mut bool,
    ) -> Result<bool, Trap> {
        Ok(false)
    }
}
