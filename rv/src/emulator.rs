use elf::Elf;

use crate::instructions::*;
use crate::disassemble::*;

const TRACE: bool = true;

#[derive(Debug)]
pub struct Emulator {
    pub pc: u32,
    regs: [u32; 31],
    pub mem: Vec<u8>,

}

#[derive(Debug)]
pub enum EmulatorExit {
    Syscall,
    Break,
    InvalidInstruction(u32),
}

impl Emulator {
    pub fn new(elf: &Elf) -> Self {
        // 25 mb
        let mut mem = vec![0u8; 25 * 1024 * 1024];

        for segment in &elf.load_segments {
            let start = segment.load_address as usize;
            let end = start + segment.file_size as usize;
            mem[start..end].copy_from_slice(&segment.data);
            // TODO: set permissions
        }

        // allocate an initial stack
        let mut regs = [0; 31];
        regs[1] = (mem.len() - 4096) as u32;

        Emulator {
            pc: elf.entry,
            regs,
            mem,
        }
    }

    pub fn write_reg(&mut self, reg: Reg, val: u32) {
        if reg.0 != 0 {
            self.regs[reg.0 as usize - 1] = val;
        }
    }

    pub fn read_reg(&self, reg: Reg) -> u32 {
        if reg.0 != 0 {
            self.regs[reg.0 as usize - 1]
        } else {
            0
        }
    }

    fn trace_print(&self, pc: u32) {
        println!(" pc={:#08x}  x1={:#08x}  x2={:08x}  x3={:#08x}",
            pc, self.regs[0], self.regs[1], self.regs[2]);
        println!(" x4={:#08x}  x5={:#08x}  x6={:#08x}  x7={:#08x}",
            self.regs[3], self.regs[4], self.regs[5], self.regs[6]);
        println!(" x8={:#08x}  x9={:#08x} x10={:#08x} x11={:#08x}",
            self.regs[7], self.regs[8], self.regs[9], self.regs[10]);
        println!("x12={:#08x} x13={:#08x} x14={:#08x} x15={:#08x}",
            self.regs[11], self.regs[12], self.regs[13], self.regs[14]);
        println!("x16={:#08x} x17={:#08x} x18={:#08x} x19={:#08x}",
            self.regs[15], self.regs[16], self.regs[17], self.regs[18]);
        println!("x20={:#08x} x21={:#08x} x22={:#08x} x23={:#08x}",
            self.regs[19], self.regs[20], self.regs[21], self.regs[22]);
        println!("x24={:#08x} x25={:#08x} x26={:#08x} x27={:#08x}",
            self.regs[23], self.regs[24], self.regs[25], self.regs[26]);
        println!("x28={:#08x} x29={:#08x} x30={:#08x} x31={:#08x}",
            self.regs[27], self.regs[28], self.regs[29], self.regs[30]);
    }

    pub fn run(&mut self) -> EmulatorExit {

        let ret;

        let mut pc = self.pc;

        macro_rules! exit {
            ($ret:expr) => {
                ret = $ret;
                break;
            }
        }

        loop {
            let instr =self.mem[pc as usize..pc as usize + 4].try_into().unwrap();
            let instr = u32::from_le_bytes(instr);

            if TRACE {
                self.trace_print(pc);
                disassemble_one(pc as u32, instr);
            }

            // first 7 bits are the opcode
            let opcode: u32 = instr & ((1 << 7) - 1);

            // follow the table on page 130 in the riscv spec
            match opcode {
                // LUI
                0b0110111 => {
                    let typ = UType::parse(instr);
                    self.write_reg(typ.rd, typ.imm);
                },
                // AUIPC
                0b0010111 => {
                    let typ = UType::parse(instr);
                    self.write_reg(typ.rd, pc + typ.imm);
                },
                // JAL
                0b1101111 => {
                    let typ = JType::parse(instr);
                    let old_pc = pc;
                    // offset is in multiples of 2 bytes
                    pc = pc.overflowing_add(typ.imm).0;
                    self.write_reg(typ.rd, old_pc + 4);
                    continue;
                },
                // JALR
                0b1100111 => {
                    let typ = IType::parse(instr);

                    if typ.funct3 != 0 {
                        exit!(EmulatorExit::InvalidInstruction(instr));
                    }

                    let old_pc = pc;
                    pc = self.read_reg(typ.rs1).overflowing_add(typ.imm).0;
                    self.write_reg(typ.rd, old_pc + 4);
                    continue;

                },

                // BRANCH
                0b1100011 => {
                    let typ = BType::parse(instr);
                    match typ.funct3 {
                        // BEQ
                        0b000 => {
                            if self.read_reg(typ.rs1) == self.read_reg(typ.rs2) {
                                pc = pc.overflowing_add(typ.imm).0;
                                continue;
                            }
                        },
                        // BNE
                        0b001 => {
                            if self.read_reg(typ.rs1) != self.read_reg(typ.rs2) {
                                pc = pc.overflowing_add(typ.imm).0;
                                continue;
                            }
                        },
                        // BLT
                        0b100 => {
                            if (self.read_reg(typ.rs1) as i32) < self.read_reg(typ.rs2) as i32 {
                                pc = pc.overflowing_add(typ.imm).0;
                                continue;
                            }
                        },
                        // BGE
                        0b101 => {
                            if self.read_reg(typ.rs1) as i32 >= self.read_reg(typ.rs2) as i32 {
                                pc = pc.overflowing_add(typ.imm).0;
                                continue;
                            }
                        },
                        // BLTU
                        0b110 => {
                            if self.read_reg(typ.rs1) < self.read_reg(typ.rs2) {
                                pc = pc.overflowing_add(typ.imm).0;
                                continue;
                            }
                        },
                        // BGEU
                        0b111 => {
                            if self.read_reg(typ.rs1) >= self.read_reg(typ.rs2) {
                                pc = pc.overflowing_add(typ.imm).0;
                                pc = pc + typ.imm;
                                continue;
                            }
                        },
                        _ => {
                            exit!(EmulatorExit::InvalidInstruction(instr));
                        },
                    };
                },

                // LOAD
                0b0000011 => {
                    let typ = IType::parse(instr);
                    match typ.funct3 {
                        // LB
                        0b000 => {
                            let addr = self.read_reg(typ.rs1).overflowing_add(typ.imm).0;
                            let addr = addr as usize;
                            let data = self.mem[addr] as i8;
                            self.write_reg(typ.rd, data as i32 as u32);
                        },
                        // LH
                        0b001 => {
                            let addr = self.read_reg(typ.rs1).overflowing_add(typ.imm).0;
                            let addr = addr as usize;
                            let data = &self.mem[addr..addr+2];
                            let data = i16::from_le_bytes(data.try_into().unwrap());
                            self.write_reg(typ.rd, data as i32 as u32);
                        },
                        // LW
                        0b010 => {
                            let addr = self.read_reg(typ.rs1).overflowing_add(typ.imm).0;
                            let addr = addr as usize;
                            let data = &self.mem[addr..addr+4];
                            let data = u32::from_le_bytes(data.try_into().unwrap());
                            self.write_reg(typ.rd, data);
                        },
                        // LBU
                        0b100 => {
                            let addr = self.read_reg(typ.rs1).overflowing_add(typ.imm).0;
                            let addr = addr as usize;
                            let data = self.mem[addr];
                            self.write_reg(typ.rd, data as u32);
                        },
                        // LHU
                        0b101 => {
                            let addr = self.read_reg(typ.rs1).overflowing_add(typ.imm).0;
                            let addr = addr as usize;
                            let data = &self.mem[addr..addr+2];
                            let data = u16::from_le_bytes(data.try_into().unwrap());
                            self.write_reg(typ.rd, data as u32);
                        },
                        _ => {
                            exit!(EmulatorExit::InvalidInstruction(instr));
                        },
                    };
                },

                // STORE
                0b0100011 => {
                    let typ = SType::parse(instr);
                    match typ.funct3 {
                        // SB
                        0b000 => {
                            let addr = self.read_reg(typ.rs1) + typ.imm;
                            let addr = addr as usize;
                            self.mem[addr] = self.read_reg(typ.rs2) as u8;
                        },
                        // SH
                        0b001 => {
                            let addr = self.read_reg(typ.rs1) + typ.imm;
                            let addr = addr as usize;
                            let data = self.read_reg(typ.rs2) as u16;
                            self.mem[addr..addr+2].copy_from_slice(&u16::to_le_bytes(data));
                        },
                        // SW
                        0b010 => {
                            let addr = self.read_reg(typ.rs1) + typ.imm;
                            let addr = addr as usize;
                            let data = self.read_reg(typ.rs2);
                            self.mem[addr..addr+4].copy_from_slice(&u32::to_le_bytes(data));
                        },
                        _ => {
                            exit!(EmulatorExit::InvalidInstruction(instr));
                        },
                    };
                }

                // OP-IMM
                0b0010011 => {
                    let typ = IType::parse(instr);

                    // arithmetic right shift?
                    // imm[11:5]
                    let arithmetic = (typ.imm & ((1 << 12) - 1)) >> 5;

                    // imm[4:0]
                    let shamt = typ.imm & 0b11111;

                    match typ.funct3 {
                        // ADDI
                        0b000 => {
                            let data = self.read_reg(typ.rs1).overflowing_add(typ.imm).0;
                            self.write_reg(typ.rd, data);
                        },
                        // SLTI
                        0b010 => {
                            if (self.read_reg(typ.rs1) as i32) < typ.imm as i32 {
                                self.write_reg(typ.rd, 1);
                            } else {
                                self.write_reg(typ.rd, 0);
                            }
                        },
                        // SLTIU
                        0b011 => {
                            if self.read_reg(typ.rs1) < typ.imm {
                                self.write_reg(typ.rd, 1);
                            } else {
                                self.write_reg(typ.rd, 0);
                            }
                        },
                        // XORI
                        0b100 => {
                            let data = self.read_reg(typ.rs1) ^ typ.imm;
                            self.write_reg(typ.rd, data);
                        },
                        // ORI
                        0b110 => {
                            let data = self.read_reg(typ.rs1) | typ.imm;
                            self.write_reg(typ.rd, data);
                        },
                        // ANDI
                        0b111 => {
                            let data = self.read_reg(typ.rs1) & typ.imm;
                            self.write_reg(typ.rd, data);
                        },
                        // SLLI
                        0b001 => {
                            if arithmetic != 0b0 {
                                exit!(EmulatorExit::InvalidInstruction(instr));
                            }

                            let data = self.read_reg(typ.rs1) << shamt;
                            self.write_reg(typ.rd, data);
                        },
                        // SRLI & SRAI
                        0b101 => {
                            match arithmetic {
                                // SRLI
                                0b0 => {
                                    let data = self.read_reg(typ.rs1) >> shamt;
                                    self.write_reg(typ.rd, data);
                                },
                                // SRAI
                                0b0100000 => {
                                    let data = self.read_reg(typ.rs1) as i32 >> shamt;
                                    self.write_reg(typ.rd, data as u32);
                                },
                                _ => {
                                    exit!(EmulatorExit::InvalidInstruction(instr));
                                },
                            };
                        },
                        _ => {
                            exit!(EmulatorExit::InvalidInstruction(instr));
                        },
                    };
                },

                // OP
                0b0110011 => {
                    let typ = RType::parse(instr);

                    match (typ.funct3, typ.funct7) {
                        // ADD
                        (0b000, 0b0000000) => {
                            let rs1 = self.read_reg(typ.rs1);
                            let rs2 = self.read_reg(typ.rs2);
                            self.write_reg(typ.rd, rs1.overflowing_add(rs2).0);
                        },
                        // SUB
                        (0b000, 0b0100000) => {
                            let rs1 = self.read_reg(typ.rs1);
                            let rs2 = self.read_reg(typ.rs2);
                            self.write_reg(typ.rd, rs1.overflowing_sub(rs2).0);
                        },
                        // SLL
                        (0b001, 0b0000000) => {
                            let rs1 = self.read_reg(typ.rs1);
                            let rs2 = self.read_reg(typ.rs2);
                            let shamt = rs2 & 0b11111;
                            self.write_reg(typ.rd, rs1 << shamt);
                        },
                        // SLT
                        (0b010, 0b0000000) => {
                            let rs1 = self.read_reg(typ.rs1) as i32;
                            let rs2 = self.read_reg(typ.rs2) as i32;
                            self.write_reg(typ.rd, if rs1 < rs2 { 1 } else { 0 });
                        },
                        // SLTU
                        (0b011, 0b0000000) => {
                            let rs1 = self.read_reg(typ.rs1);
                            let rs2 = self.read_reg(typ.rs2);
                            self.write_reg(typ.rd, if rs1 < rs2 { 1 } else { 0 });
                        },
                        // XOR
                        (0b100, 0b0000000) => {
                            let rs1 = self.read_reg(typ.rs1);
                            let rs2 = self.read_reg(typ.rs2);
                            self.write_reg(typ.rd, rs1 ^ rs2);
                        },
                        // SRL
                        (0b101, 0b0000000) => {
                            let rs1 = self.read_reg(typ.rs1);
                            let rs2 = self.read_reg(typ.rs2);
                            let shamt = rs2 & 0b11111;
                            self.write_reg(typ.rd, rs1 >> shamt);
                        },
                        // SRA
                        (0b101, 0b0100000) => {
                            let rs1 = self.read_reg(typ.rs1);
                            let rs2 = self.read_reg(typ.rs2);
                            let shamt = rs2 & 0b11111;
                            self.write_reg(typ.rd, (rs1 as i32 >> shamt) as u32);
                        },
                        // OR
                        (0b110, 0b0000000) => {
                            let rs1 = self.read_reg(typ.rs1);
                            let rs2 = self.read_reg(typ.rs2);
                            self.write_reg(typ.rd, rs1 | rs2);
                        },
                        // AND
                        (0b111, 0b0000000) => {
                            let rs1 = self.read_reg(typ.rs1);
                            let rs2 = self.read_reg(typ.rs2);
                            self.write_reg(typ.rd, rs1 & rs2);
                        },
                        _ => {
                            exit!(EmulatorExit::InvalidInstruction(instr));
                        },
                    };
                }

                // MISC-MEM
                0b0001111 => {
                    let typ = IType::parse(instr);
                    // FENCE
                    if typ.funct3 != 0b000 {
                        exit!(EmulatorExit::InvalidInstruction(instr));
                    }
                },

                // SYSTEM
                0b1110011 => {
                    let typ = IType::parse(instr);

                    if typ.rs1.0 != 0 || typ.rd.0 != 0 || typ.funct3 != 0 {
                        exit!(EmulatorExit::InvalidInstruction(instr));
                    }

                    match typ.imm {
                        // ECALL
                        0b0 => {
                            ret = EmulatorExit::Syscall;
                            break;
                        },
                        0b1 => {
                            ret = EmulatorExit::Break;
                            break;
                        },
                        _ => {
                            exit!(EmulatorExit::InvalidInstruction(instr));
                        },
                    }
                },

                _ => {
                    exit!(EmulatorExit::InvalidInstruction(instr));
                },
            }

            // update pc for next instruction
            pc = pc + 4;
        }

        self.pc = pc as u32;

        return ret;
    }
}
