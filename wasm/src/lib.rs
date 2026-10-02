use riscv_emu_core::{Cpu, Host, Memory, Trap, TIMER_ADDR, UART_ADDR};
use wasm_bindgen::prelude::*;

#[derive(Default)]
struct DoomHost {
    wad: Vec<u8>,
    wad_position: usize,
}

impl Host for DoomHost {
    fn ecall(
        &mut self,
        number: u32,
        regs: &mut [u32; 32],
        memory: &mut Memory,
        heap: &mut u32,
        halted: &mut bool,
    ) -> Result<bool, Trap> {
        match number {
            0 => {
                let old = *heap;
                *heap = heap.wrapping_add(regs[10]);
                if *heap >= regs[2] {
                    return Err(Trap::AccessFault(*heap));
                }
                regs[10] = old;
            }
            1 => {
                self.wad_position = 0;
                regs[10] = 3;
            }
            2 => {
                let address = regs[11];
                let requested = regs[12] as usize;
                let count = requested.min(self.wad.len().saturating_sub(self.wad_position));
                memory.load(
                    address,
                    &self.wad[self.wad_position..self.wad_position + count],
                )?;
                self.wad_position += count;
                regs[10] = count as u32;
            }
            3 => {
                for i in 0..regs[12] {
                    let byte = memory.read_u8(regs[11] + i)?;
                    memory.write_u8(UART_ADDR, byte)?;
                }
                regs[10] = regs[12];
            }
            4 => {
                let base = match regs[12] {
                    0 => 0,
                    1 => self.wad_position as i32,
                    2 => self.wad.len() as i32,
                    _ => -1,
                };
                self.wad_position = base
                    .saturating_add(regs[11] as i32)
                    .max(0)
                    .min(self.wad.len() as i32) as usize;
                regs[10] = self.wad_position as u32;
            }
            5 => regs[10] = 0,
            6 => {
                let ms = memory.read_u32(TIMER_ADDR)?;
                regs[10] = ms / 1000;
                regs[11] = (ms % 1000) * 1000;
                regs[12] = 0;
            }
            7 => regs[10] = 0,
            8 => {
                const VRAM_SIZE: usize = 640 * 400 * 4;
                const VRAM_START: usize = 8 * 1024 * 1024 - 32 - 1 - 1 - 1 - VRAM_SIZE;
                memory
                    .framebuffer
                    .copy_from_slice(&memory.dram[VRAM_START..VRAM_START + VRAM_SIZE]);
            }
            9..=12 => regs[10] = u32::MAX,
            15 => {
                *halted = true;
                return Err(Trap::Halted(regs[10] as i32));
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}

#[wasm_bindgen]
pub struct WasmRiscv {
    cpu: Cpu,
    host: DoomHost,
}

fn error(e: impl std::fmt::Debug) -> JsValue {
    JsValue::from_str(&format!("{e:?}"))
}

#[wasm_bindgen]
impl WasmRiscv {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            cpu: Cpu::new(),
            host: DoomHost::default(),
        }
    }
    pub fn load_elf(&mut self, data: &[u8]) -> Result<(), JsValue> {
        self.cpu.load_elf(data).map_err(error)
    }
    pub fn load_binary(&mut self, address: u32, data: &[u8]) -> Result<(), JsValue> {
        self.cpu.load_binary(address, data).map_err(error)
    }
    pub fn load_wad(&mut self, data: &[u8]) {
        self.host.wad.clear();
        self.host.wad.extend_from_slice(data);
        self.host.wad_position = 0;
    }
    pub fn run(&mut self, count: usize) -> Result<(), JsValue> {
        match self.cpu.run_with_host(&mut self.host, count) {
            Ok(()) => Ok(()),
            Err(Trap::Halted(_)) => Ok(()),
            Err(e) => Err(error(e)),
        }
    }
    pub fn pc(&self) -> u32 {
        self.cpu.pc
    }
    pub fn reg(&self, i: usize) -> u32 {
        self.cpu.regs.get(i).copied().unwrap_or(0)
    }
    pub fn framebuffer_ptr(&self) -> *const u8 {
        self.cpu.mem.framebuffer.as_ptr()
    }
    pub fn framebuffer_size(&self) -> usize {
        self.cpu.mem.framebuffer.len()
    }
    pub fn uart_output(&mut self) -> String {
        self.cpu.mem.take_uart()
    }
    pub fn push_key(&mut self, key: u32) {
        self.cpu.mem.push_key(key);
    }
    pub fn push_key_event(&mut self, key: u32, pressed: bool) {
        self.cpu.mem.push_key_event(key, pressed);
    }
    pub fn set_ticks_ms(&mut self, ticks: u32) {
        self.cpu.mem.set_ticks_ms(ticks);
    }
    pub fn halted(&self) -> bool {
        self.cpu.halted
    }
}

impl Default for WasmRiscv {
    fn default() -> Self {
        Self::new()
    }
}
