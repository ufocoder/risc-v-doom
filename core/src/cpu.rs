use crate::{
    decode::decode,
    elf::{load_elf, ElfError},
    execute::execute_with_host,
    host::{Host, NullHost},
    memory::{Memory, Trap, DRAM_BASE},
};

#[derive(Default)]
pub struct Csr {
    pub cycle: u64,
    pub instret: u64,
}
pub struct Cpu {
    pub regs: [u32; 32],
    pub pc: u32,
    pub mem: Memory,
    pub csr: Csr,
    pub heap: u32,
    pub halted: bool,
}
impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}
impl Cpu {
    pub fn new() -> Self {
        Self {
            regs: [0; 32],
            pc: DRAM_BASE,
            mem: Memory::new(),
            csr: Csr::default(),
            heap: DRAM_BASE + 0x0040_0000,
            halted: false,
        }
    }
    pub fn step(&mut self) -> Result<(), Trap> {
        self.step_with_host(&mut NullHost)
    }
    pub fn step_with_host<H: Host>(&mut self, host: &mut H) -> Result<(), Trap> {
        if self.halted {
            return Err(Trap::Halted(self.regs[10] as i32));
        }
        if self.pc & 3 != 0 {
            return Err(Trap::Misaligned(self.pc));
        }
        let raw = self.mem.read_u32(self.pc)?;
        let d = decode(raw);
        execute_with_host(self, d, host)?;
        self.regs[0] = 0;
        self.csr.cycle += 1;
        self.csr.instret += 1;
        Ok(())
    }
    pub fn run(&mut self, n: usize) -> Result<(), Trap> {
        self.run_with_host(&mut NullHost, n)
    }
    pub fn run_with_host<H: Host>(&mut self, host: &mut H, n: usize) -> Result<(), Trap> {
        for _ in 0..n {
            self.step_with_host(host)?;
        }
        Ok(())
    }
    pub fn load_binary(&mut self, addr: u32, data: &[u8]) -> Result<(), Trap> {
        self.mem.load(addr, data)?;
        self.pc = addr;
        Ok(())
    }
    pub fn load_elf(&mut self, data: &[u8]) -> Result<(), ElfError> {
        let (entry, image_end) = load_elf(&mut self.mem, data)?;
        self.pc = entry;
        if entry < DRAM_BASE {
            self.heap = image_end.checked_add(15).ok_or(ElfError::Bounds)? & !15;
            self.regs[2] = 8 * 1024 * 1024 - (640 * 400 * 4) - 35;
        }
        Ok(())
    }
}
