#[derive(Debug, Clone, Copy)]
pub struct Decoded {
    pub raw: u32,
    pub opcode: u8,
    pub rd: usize,
    pub rs1: usize,
    pub rs2: usize,
    pub funct3: u8,
    pub funct7: u8,
    pub imm: i32,
}
fn sext(v: u32, bits: u8) -> i32 {
    ((v << (32 - bits)) as i32) >> (32 - bits)
}
pub fn decode(i: u32) -> Decoded {
    let opcode = (i & 0x7f) as u8;
    let imm = match opcode {
        0x03 | 0x13 | 0x67 | 0x73 => sext(i >> 20, 12),
        0x23 => sext(((i >> 25) << 5) | ((i >> 7) & 31), 12),
        0x63 => sext(
            ((i >> 31) << 12)
                | (((i >> 7) & 1) << 11)
                | (((i >> 25) & 0x3f) << 5)
                | (((i >> 8) & 15) << 1),
            13,
        ),
        0x17 | 0x37 => (i & 0xfffff000) as i32,
        0x6f => sext(
            ((i >> 31) << 20)
                | (((i >> 12) & 0xff) << 12)
                | (((i >> 20) & 1) << 11)
                | (((i >> 21) & 0x3ff) << 1),
            21,
        ),
        _ => 0,
    };
    Decoded {
        raw: i,
        opcode,
        rd: ((i >> 7) & 31) as usize,
        funct3: ((i >> 12) & 7) as u8,
        rs1: ((i >> 15) & 31) as usize,
        rs2: ((i >> 20) & 31) as usize,
        funct7: ((i >> 25) & 0x7f) as u8,
        imm,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn immediates() {
        assert_eq!(decode(0xfff00093).imm, -1);
        assert_eq!(decode(0x0080006f).imm, 8);
    }
}
