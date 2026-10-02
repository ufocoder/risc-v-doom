use crate::{cpu::Cpu, host::Host, memory::Trap};

pub fn syscall<H: Host>(c: &mut Cpu, host: &mut H) -> Result<(), Trap> {
    let n = c.regs[17];
    if host.ecall(n, &mut c.regs, &mut c.mem, &mut c.heap, &mut c.halted)? {
        return Ok(());
    }
    match n {
        64 => {
            let addr = c.regs[11];
            let len = c.regs[12];
            for i in 0..len {
                let ch = c.mem.read_u8(addr + i)?;
                c.mem.write_u8(crate::UART_ADDR, ch)?;
            }
            c.regs[10] = len;
        }
        63 => c.regs[10] = 0,
        214 => {
            let inc = c.regs[10];
            let old = c.heap;
            c.heap = c.heap.checked_add(inc).ok_or(Trap::AccessFault(c.heap))?;
            c.regs[10] = old;
        }
        57 | 62 | 80 => c.regs[10] = 0,
        169 => {
            let p = c.regs[10];
            let ms = c.mem.read_u32(crate::TIMER_ADDR)?;
            c.mem.write_u32(p, ms / 1000)?;
            c.mem.write_u32(p + 4, (ms % 1000) * 1000)?;
            c.regs[10] = 0;
        }
        93 => {
            c.halted = true;
            return Err(Trap::Halted(c.regs[10] as i32));
        }
        _ => return Err(Trap::Ecall(n)),
    }
    Ok(())
}
