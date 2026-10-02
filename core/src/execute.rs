use crate::{cpu::Cpu, decode::Decoded, host::Host, memory::Trap, syscall::syscall};
fn wr(c: &mut Cpu, r: usize, v: u32) {
    if r != 0 {
        c.regs[r] = v
    }
}
fn high(a: u32, b: u32) -> u32 {
    (((a as u64) * (b as u64)) >> 32) as u32
}
pub fn execute_with_host<H: Host>(c: &mut Cpu, d: Decoded, host: &mut H) -> Result<(), Trap> {
    let a = c.regs[d.rs1];
    let b = c.regs[d.rs2];
    let mut next = c.pc.wrapping_add(4);
    match d.opcode {
        0x37 => wr(c, d.rd, d.imm as u32),
        0x17 => wr(c, d.rd, c.pc.wrapping_add(d.imm as u32)),
        0x6f => {
            wr(c, d.rd, next);
            next = c.pc.wrapping_add(d.imm as u32)
        }
        0x67 => {
            if d.funct3 != 0 {
                return Err(Trap::IllegalInstruction(d.raw));
            }
            wr(c, d.rd, next);
            next = a.wrapping_add(d.imm as u32) & !1
        }
        0x63 => {
            let take = match d.funct3 {
                0 => a == b,
                1 => a != b,
                4 => (a as i32) < (b as i32),
                5 => (a as i32) >= (b as i32),
                6 => a < b,
                7 => a >= b,
                _ => return Err(Trap::IllegalInstruction(d.raw)),
            };
            if take {
                next = c.pc.wrapping_add(d.imm as u32)
            }
        }
        0x03 => {
            let ad = a.wrapping_add(d.imm as u32);
            let v = match d.funct3 {
                0 => c.mem.read_u8(ad)? as u8 as i8 as i32 as u32,
                1 => c.mem.read_u16(ad)? as u16 as i16 as i32 as u32,
                2 => c.mem.read_u32(ad)?,
                4 => c.mem.read_u8(ad)?,
                5 => c.mem.read_u16(ad)?,
                _ => return Err(Trap::IllegalInstruction(d.raw)),
            };
            wr(c, d.rd, v)
        }
        0x23 => {
            let ad = a.wrapping_add(d.imm as u32);
            match d.funct3 {
                0 => c.mem.write_u8(ad, b)?,
                1 => c.mem.write_u16(ad, b)?,
                2 => c.mem.write_u32(ad, b)?,
                _ => return Err(Trap::IllegalInstruction(d.raw)),
            }
        }
        0x13 => {
            let sh = (d.raw >> 20) & 31;
            let v = match d.funct3 {
                0 => a.wrapping_add(d.imm as u32),
                2 => ((a as i32) < d.imm) as u32,
                3 => (a < (d.imm as u32)) as u32,
                4 => a ^ (d.imm as u32),
                6 => a | (d.imm as u32),
                7 => a & (d.imm as u32),
                1 if d.funct7 == 0 => a << sh,
                5 if d.funct7 == 0 => a >> sh,
                5 if d.funct7 == 0x20 => ((a as i32) >> sh) as u32,
                _ => return Err(Trap::IllegalInstruction(d.raw)),
            };
            wr(c, d.rd, v)
        }
        0x33 => {
            let sh = b & 31;
            let v = if d.funct7 == 1 {
                match d.funct3 {
                    0 => a.wrapping_mul(b),
                    1 => (((a as i32 as i64) * (b as i32 as i64)) >> 32) as u32,
                    2 => (((a as i32 as i64) * (b as u64 as i64)) >> 32) as u32,
                    3 => high(a, b),
                    4 => {
                        if b == 0 {
                            u32::MAX
                        } else if a == 0x80000000 && b == u32::MAX {
                            a
                        } else {
                            ((a as i32) / (b as i32)) as u32
                        }
                    }
                    5 => {
                        if b == 0 {
                            u32::MAX
                        } else {
                            a / b
                        }
                    }
                    6 => {
                        if b == 0 {
                            a
                        } else if a == 0x80000000 && b == u32::MAX {
                            0
                        } else {
                            ((a as i32) % (b as i32)) as u32
                        }
                    }
                    7 => {
                        if b == 0 {
                            a
                        } else {
                            a % b
                        }
                    }
                    _ => unreachable!(),
                }
            } else {
                match (d.funct7, d.funct3) {
                    (0, 0) => a.wrapping_add(b),
                    (0x20, 0) => a.wrapping_sub(b),
                    (0, 1) => a << sh,
                    (0, 2) => ((a as i32) < (b as i32)) as u32,
                    (0, 3) => (a < b) as u32,
                    (0, 4) => a ^ b,
                    (0, 5) => a >> sh,
                    (0x20, 5) => ((a as i32) >> sh) as u32,
                    (0, 6) => a | b,
                    (0, 7) => a & b,
                    _ => return Err(Trap::IllegalInstruction(d.raw)),
                }
            };
            wr(c, d.rd, v)
        }
        0x0f => {}
        0x73 => {
            if d.raw == 0x00100073 {
                return Err(Trap::Breakpoint);
            }
            if d.raw == 0x00000073 {
                syscall(c, host)?
            } else {
                return Err(Trap::IllegalInstruction(d.raw));
            }
        }
        _ => return Err(Trap::IllegalInstruction(d.raw)),
    }
    c.pc = next;
    c.regs[0] = 0;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::decode;
    use crate::NullHost;
    #[test]
    fn arithmetic() {
        let mut c = Cpu::new();
        c.regs[1] = 7;
        c.regs[2] = 3;
        execute_with_host(&mut c, decode(0x002081b3), &mut NullHost).unwrap();
        assert_eq!(c.regs[3], 10);
        execute_with_host(&mut c, decode(0x02208233), &mut NullHost).unwrap();
        assert_eq!(c.regs[4], 21);
    }
    #[test]
    fn x0() {
        let mut c = Cpu::new();
        execute_with_host(&mut c, decode(0x00100013), &mut NullHost).unwrap();
        assert_eq!(c.regs[0], 0);
    }
}
