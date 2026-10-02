use std::collections::VecDeque;

pub const DRAM_BASE: u32 = 0x8000_0000;
pub const DRAM_SIZE: usize = 8 * 1024 * 1024;
pub const FRAMEBUFFER_ADDR: u32 = 0x8020_0000;
pub const FRAMEBUFFER_WIDTH: usize = 640;
pub const FRAMEBUFFER_HEIGHT: usize = 400;
pub const FRAMEBUFFER_SIZE: usize = FRAMEBUFFER_WIDTH * FRAMEBUFFER_HEIGHT * 4;
pub const UART_ADDR: u32 = 0x1000_0000;
pub const KEYBOARD_ADDR: u32 = 0x1000_0004;
pub const TIMER_ADDR: u32 = 0x1000_0008;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trap {
    AccessFault(u32),
    Misaligned(u32),
    IllegalInstruction(u32),
    Breakpoint,
    Ecall(u32),
    Halted(i32),
}

pub struct Memory {
    pub dram: Vec<u8>,
    pub framebuffer: Vec<u8>,
    pub uart_buffer: String,
    keys: VecDeque<u32>,
    ticks_ms: u32,
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}
impl Memory {
    pub fn new() -> Self {
        Self {
            dram: vec![0; DRAM_SIZE],
            framebuffer: vec![0; FRAMEBUFFER_SIZE],
            uart_buffer: String::new(),
            keys: VecDeque::new(),
            ticks_ms: 0,
        }
    }
    pub fn set_ticks_ms(&mut self, value: u32) {
        self.ticks_ms = value;
    }
    pub fn push_key(&mut self, key: u32) {
        self.push_key_event(key, true);
    }
    pub fn push_key_event(&mut self, key: u32, pressed: bool) {
        self.keys.push_back(key);
        const QUEUE_START: usize = DRAM_SIZE - 32 - 1;
        const READ_INDEX: usize = QUEUE_START - 1;
        const WRITE_INDEX: usize = READ_INDEX - 1;
        let index = (self.dram[WRITE_INDEX] % 16) as usize;
        let event = ((pressed as u16) << 8) | (key as u16 & 0xff);
        self.dram[QUEUE_START + index * 2..QUEUE_START + index * 2 + 2]
            .copy_from_slice(&event.to_le_bytes());
        self.dram[WRITE_INDEX] = ((index + 1) % 16) as u8;
    }
    pub fn take_uart(&mut self) -> String {
        std::mem::take(&mut self.uart_buffer)
    }

    fn dram_offset(&self, addr: u32, len: usize) -> Result<usize, Trap> {
        let off = if addr < DRAM_SIZE as u32 {
            addr as usize
        } else {
            addr.checked_sub(DRAM_BASE).ok_or(Trap::AccessFault(addr))? as usize
        };
        if off
            .checked_add(len)
            .filter(|&end| end <= self.dram.len())
            .is_none()
        {
            return Err(Trap::AccessFault(addr));
        }
        Ok(off)
    }
    pub fn read_u8(&mut self, addr: u32) -> Result<u32, Trap> {
        match addr {
            KEYBOARD_ADDR => Ok(self.keys.pop_front().unwrap_or(0)),
            TIMER_ADDR => Ok(self.ticks_ms),
            _ => Ok(self.dram[self.dram_offset(addr, 1)?] as u32),
        }
    }
    pub fn read_u16(&mut self, addr: u32) -> Result<u32, Trap> {
        let o = self.dram_offset(addr, 2)?;
        Ok(u16::from_le_bytes([self.dram[o], self.dram[o + 1]]) as u32)
    }
    pub fn read_u32(&mut self, addr: u32) -> Result<u32, Trap> {
        if addr == KEYBOARD_ADDR {
            return Ok(self.keys.pop_front().unwrap_or(0));
        }
        if addr == TIMER_ADDR {
            return Ok(self.ticks_ms);
        }
        let o = self.dram_offset(addr, 4)?;
        Ok(u32::from_le_bytes(self.dram[o..o + 4].try_into().unwrap()))
    }
    pub fn write_u8(&mut self, addr: u32, value: u32) -> Result<(), Trap> {
        if addr == UART_ADDR {
            self.uart_buffer.push(value as u8 as char);
            return Ok(());
        }
        if (FRAMEBUFFER_ADDR..FRAMEBUFFER_ADDR + FRAMEBUFFER_SIZE as u32).contains(&addr) {
            self.framebuffer[(addr - FRAMEBUFFER_ADDR) as usize] = value as u8;
            return Ok(());
        }
        let o = self.dram_offset(addr, 1)?;
        self.dram[o] = value as u8;
        Ok(())
    }
    pub fn write_u16(&mut self, addr: u32, value: u32) -> Result<(), Trap> {
        for (i, b) in (value as u16).to_le_bytes().iter().enumerate() {
            self.write_u8(addr + i as u32, *b as u32)?;
        }
        Ok(())
    }
    pub fn write_u32(&mut self, addr: u32, value: u32) -> Result<(), Trap> {
        if addr == UART_ADDR {
            self.uart_buffer.push(value as u8 as char);
            return Ok(());
        }
        for (i, b) in value.to_le_bytes().iter().enumerate() {
            self.write_u8(addr + i as u32, *b as u32)?;
        }
        Ok(())
    }
    pub fn load(&mut self, addr: u32, data: &[u8]) -> Result<(), Trap> {
        let o = self.dram_offset(addr, data.len())?;
        self.dram[o..o + data.len()].copy_from_slice(data);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn little_endian() {
        let mut m = Memory::new();
        m.write_u32(DRAM_BASE, 0x12345678).unwrap();
        assert_eq!(&m.dram[..4], &[0x78, 0x56, 0x34, 0x12]);
        assert_eq!(m.read_u32(DRAM_BASE).unwrap(), 0x12345678);
    }
    #[test]
    fn bounds_and_unaligned_access() {
        let mut m = Memory::new();
        m.write_u32(DRAM_BASE + 1, 0x1234_5678).unwrap();
        assert_eq!(m.read_u32(DRAM_BASE + 1).unwrap(), 0x1234_5678);
        assert!(matches!(
            m.read_u8(DRAM_BASE + DRAM_SIZE as u32),
            Err(Trap::AccessFault(_))
        ));
    }
    #[test]
    fn mmio() {
        let mut m = Memory::new();
        m.write_u8(UART_ADDR, b'A' as u32).unwrap();
        m.push_key(42);
        m.set_ticks_ms(99);
        assert_eq!(m.uart_buffer, "A");
        assert_eq!(m.read_u32(KEYBOARD_ADDR).unwrap(), 42);
        assert_eq!(m.read_u32(TIMER_ADDR).unwrap(), 99);
    }
    #[test]
    fn keyboard_ring_buffer_records_press_and_release() {
        let mut m = Memory::new();
        const QUEUE_START: usize = DRAM_SIZE - 32 - 1;
        m.push_key_event(3, true);
        m.push_key_event(3, false);
        assert_eq!(
            u16::from_le_bytes([m.dram[QUEUE_START], m.dram[QUEUE_START + 1]]),
            0x0103
        );
        assert_eq!(
            u16::from_le_bytes([m.dram[QUEUE_START + 2], m.dram[QUEUE_START + 3]]),
            0x0003
        );
    }
}
