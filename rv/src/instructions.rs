
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
/// Representing an x0-x31 register
pub struct Reg(pub u8);

impl Reg {
    /// get the abi name of the register
    pub fn abi_name(self) -> &'static str {
        const NAMES: [&str; 32] = [
            "zero",
            // return address
            "ra",
            // stack pointer
            "sp",
            // global pointer
            "gp",
            // thread pointer
            "tp",
            // temporaries
            "t0", "t1", "t2",
            // frame pointer, alternative name s0
            "fp",
            // saved register
            "s1",
            // function args (all)/return values (a0, a1)
            "a0", "a1", "a2", "a3", "a4", "a5", "a6", "a7",
            // saved register
            "s2", "s3", "s4", "s5", "s6", "s7", "s8", "s9", "s10", "s11",
            // temporary registers
            "t3", "t4", "t5", "t6",
        ];

        NAMES[self.0 as usize]
    }
}

#[derive(Debug, Clone, Copy)]
pub struct UType {
    pub imm: u32,
    pub rd: Reg,
}

impl UType {
    pub fn parse(instr: u32) -> Self {
        // instr[31:12] into imm[31:12]
        let imm = instr & !((1 << 12) - 1);

        // instr[11:7]
        let rd = ((instr & ((1 << 12) - 1)) >> 7) as u8;
        let rd = Reg(rd);
        UType {
            imm,
            rd,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct JType {
    pub imm: u32,
    pub rd: Reg,
}

impl JType {
    pub fn parse(instr: u32) -> Self {
        let imm: u32 =
            // instr[31] -> imm[20]
            ((instr & (1 << 31)) >> 31) << 20 |
            // instr[30:21] -> imm [10:1]
            ((instr & ((1 << 31) - 1)) >> 21) << 1 |
            // instr[20] -> imm[11]
            ((instr & ((1 << 21) - 1)) >> 20) << 11 |
            // instr[19:12] -> imm [19:12]
            ((instr & ((1 << 20) - 1)) >> 12) << 12;

        // instr[11:7]
        let rd: u8 =
            ((instr & ((1 << 12) - 1)) >> 7) as u8;
        let rd = Reg(rd);
        JType {
            imm,
            rd,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct IType {
    pub imm: u32,
    pub rs1: Reg,
    pub funct3: u8,
    pub rd: Reg,
}

impl IType {
    pub fn parse(instr: u32) -> Self{
        // instr[31:20] -> imm[11:0], sign extended
        let imm =
            ((instr as i32) >> 20) as u32;

        // instr[19:15]
        let rs1 = ((instr & ((1 << 20) - 1)) >> 15) as u8;
        let rs1 = Reg(rs1);

        // instr[14:12]
        let funct3 = ((instr & ((1 << 15) - 1)) >> 12) as u8;

        // instr[11:7]
        let rd = ((instr & ((1 << 12) - 1)) >> 7) as u8;
        let rd = Reg(rd);

        IType {
            imm,
            rs1,
            funct3,
            rd,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BType {
    pub imm: u32,
    pub rs2: Reg,
    pub rs1: Reg,
    pub funct3: u8,
}

impl BType {
    pub fn parse(instr: u32) -> Self {
        let imm =
            // instr[31] -> imm[12], sign extended
            (((instr as i32 >> 31) as u32) << 12) |
            // instr[30:25] -> imm[10:5]
            (((instr & ((1 << 31) - 1)) >> 25) << 5) |
            // instr[11:8] -> imm[4:1]
            (((instr & ((1 << 12) - 1)) >> 8) << 1) |
            // instr[7] -> imm[11]
            (((instr & ((1 << 8) - 1)) >> 7) << 11);

        // instr[24:20]
        let rs2 = ((instr & ((1 << 25) - 1)) >> 20) as u8;
        let rs2 = Reg(rs2);

        // instr[19:15]
        let rs1 = ((instr & ((1 << 20) - 1)) >> 15) as u8;
        let rs1 = Reg(rs1);

        // instr[14:12]
        let funct3 = ((instr & ((1 << 15) - 1)) >> 12) as u8;

        BType {
            imm,
            rs2,
            rs1,
            funct3,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SType {
    pub imm: u32,
    pub rs2: Reg,
    pub rs1: Reg,
    pub funct3: u8,
}

impl SType {
    pub fn parse(instr: u32) -> Self {
        let imm =
            // instr[31:25] -> imm[11:5], sign extended
            (((instr as i32 >> 25) as u32) << 5) |
            // instr[11:7] -> imm[4:0]
            ((instr & ((1 << 12) - 1)) >> 7);

        // instr[24:20]
        let rs2 = ((instr & ((1 << 25) - 1)) >> 20) as u8;
        let rs2 = Reg(rs2);

        // instr[19:15]
        let rs1 = ((instr & ((1 << 20) - 1)) >> 15) as u8;
        let rs1 = Reg(rs1);

        // instr[14:12]
        let funct3 = ((instr & ((1 << 15) - 1)) >> 12) as u8;

        SType {
            imm,
            rs2,
            rs1,
            funct3,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RType {
    pub funct7: u8,
    pub rs2: Reg,
    pub rs1: Reg,
    pub funct3: u8,
    pub rd: Reg,
}

impl RType {
    pub fn parse(instr: u32) -> Self {
        // instr[31:25]
        let funct7 = (instr >> 25) as u8;

        // instr[24:20]
        let rs2 = ((instr & ((1 << 25) - 1)) >> 20) as u8;
        let rs2 = Reg(rs2);

        // instr[19:15]
        let rs1 = ((instr & ((1 << 20) - 1)) >> 15) as u8;
        let rs1 = Reg(rs1);

        // instr[14:12]
        let funct3 = ((instr & ((1 << 15) - 1)) >> 12) as u8;

        // instr[11:7]
        let rd = ((instr & ((1 << 12) - 1)) >> 7) as u8;
        let rd = Reg(rd);

        RType {
            funct7,
            rs2,
            rs1,
            funct3,
            rd,
        }
    }
}
