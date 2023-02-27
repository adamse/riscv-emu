use std::io::Write;

use elf::Elf;

use crate::instructions::*;
use crate::disassemble::*;

const TRACE: bool = false;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Perms {
    /// No permissions
    None = 0,

    /// Read
    Read = 0b1,

    /// Write
    Write = 0b10,

    /// Executable
    Exec = 0b100,

    /// Read after write
    ReadAfterWrite = 0b1000,
}

impl Perms {
    fn test(self, byte: u8) -> bool {
        ((self as u8) & byte) != 0
    }
}

#[derive(Debug)]
pub struct Emulator {
    pub pc: u32,
    regs: [u32; 31],
    pub mem: Box<[u8]>,

    // memory permissions
    pub perms: Box<[u8]>,
}

#[derive(Debug)]
pub enum EmulatorExit {
    Syscall,
    Break,
    InvalidInstruction(u32),
    InvalidMemoryAccess {
        perm: Perms,
        addr: u32
    },
}

impl Emulator {
    pub fn new(elf: &Elf) -> Self {
        // 25 mb
        let size = 25 * 1024 * 1024;
        let mem = Box::new_zeroed_slice(size);
        let mut mem = unsafe { mem.assume_init() };

        let perms = Box::new_zeroed_slice(size);
        let mut perms = unsafe { perms.assume_init() };

        for segment in &elf.load_segments {
            let start = segment.load_address as usize;
            let file_end = start + segment.file_size as usize;

            mem[start..file_end].copy_from_slice(&segment.data);

            let perm =
                if segment.flags.r() { Perms::Read as u8 } else { 0 } |
                if segment.flags.w() { Perms::Write as u8 } else { 0 } |
                if segment.flags.x() { Perms::Exec as u8 } else { 0 };

            let mem_end = start + segment.size as usize;
            // align up to next word
            let mem_end = (mem_end + 4) & !3;

            for i in start..mem_end {
                perms[i] = perm;
            }

            println!("loading segment: {:08x}-{:08x}-{:08x} {:?}", start, file_end, mem_end, segment.flags);
        }

        let mut regs = [0; 31];

        // allocate an initial stack
        // 2M at the end of memory
        regs[1] = (mem.len() - 2*1024*1024) as u32;

        Emulator {
            pc: elf.entry,
            regs,
            mem,
            perms
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

    /// write current instruction and register state to the trace file
    ///
    /// A trace record is (1 + 1 + 31) * 4 bytes long (instruction, pc, general purpose registers)
    fn trace_binary(&self, pc: u32, file: &mut std::fs::File) {
        // write the instruction to the trace
        let instr = &self.mem[pc as usize..][..4];
        file.write_all(instr);

        // write pc to the trace
        let pc = pc.to_le_bytes();
        file.write_all(&pc[..]);

        // write all the other registers to the trace
        let regs = &self.regs as *const u32 as *const u8;
        let regs = unsafe {
            std::slice::from_raw_parts(regs, self.regs.len() * std::mem::size_of::<u32>())
        };
        file.write_all(regs);
    }

    fn trace_print2(&self, pc: u32) {
        print!("  pc {pc:#010x}");
        for i in 1u8..4 {
            let reg = Reg(i);
            print!(" {:>3} {:#010x}", reg.abi_name(), self.read_reg(reg));
        }
        println!();
        for i in (4u8..31).step_by(4) {
            for j in 0..4 {
                let reg = Reg(i + j);
                print!(" {:>3} {:#010x}", reg.abi_name(), self.read_reg(reg));
            }
            println!();
        }
    }

    fn trace_print(&self, pc: u32) {
        println!(" pc {:#010x}  x1 {:#010x}  x2 {:010x}  x3 {:#010x}",
            pc, self.regs[0], self.regs[1], self.regs[2]);
        println!(" x4 {:#010x}  x5 {:#010x}  x6 {:#010x}  x7 {:#010x}",
            self.regs[3], self.regs[4], self.regs[5], self.regs[6]);
        println!(" x8 {:#010x}  x9 {:#010x} x10 {:#010x} x11 {:#010x}",
            self.regs[7], self.regs[8], self.regs[9], self.regs[10]);
        println!("x12 {:#010x} x13 {:#010x} x14 {:#010x} x15 {:#010x}",
            self.regs[11], self.regs[12], self.regs[13], self.regs[14]);
        println!("x16 {:#010x} x17 {:#010x} x18 {:#010x} x19 {:#010x}",
            self.regs[15], self.regs[16], self.regs[17], self.regs[18]);
        println!("x20 {:#010x} x21 {:#010x} x22 {:#010x} x23 {:#010x}",
            self.regs[19], self.regs[20], self.regs[21], self.regs[22]);
        println!("x24 {:#010x} x25 {:#010x} x26 {:#010x} x27 {:#010x}",
            self.regs[23], self.regs[24], self.regs[25], self.regs[26]);
        println!("x28 {:#010x} x29 {:#010x} x30 {:#010x} x31 {:#010x}",
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
        'main: loop {

            macro_rules! read_mem {
                ($addr:expr, $ty:ty) => {{
                    const SIZE: usize = std::mem::size_of::<$ty>();

                    // check permissions
                    for i in 0..SIZE {
                        if !Perms::Read.test(self.perms[$addr + i]) {
                            ret = EmulatorExit::InvalidMemoryAccess {
                                perm: Perms::Read,
                                addr: ($addr + i) as u32
                            };
                            break 'main;
                        }
                    }

                    <$ty>::from_le_bytes(self.mem[$addr..$addr + SIZE].try_into().unwrap())
                }}
            }

            macro_rules! write_mem {
                ($addr:expr, $ty:ty, $data:expr) => {{
                    const SIZE: usize = std::mem::size_of::<$ty>();

                    // check permissions
                    for i in 0..SIZE {
                        if !Perms::Write.test(self.perms[$addr + i]) {
                            ret = EmulatorExit::InvalidMemoryAccess {
                                perm: Perms::Write,
                                addr: ($addr + i) as u32
                            };
                            break 'main;
                        }
                    }
                    self.mem[$addr..$addr + SIZE].copy_from_slice(&<$ty>::to_le_bytes($data));
                }}
            }

            let instr = self.mem[pc as usize..pc as usize + 4].try_into().unwrap();
            let instr = u32::from_le_bytes(instr);

            if TRACE {
                self.trace_print(pc);
                disassemble_one(pc as u32, instr, true);
            }
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
                    // offset is in multiples of 2 bytes ??
                    pc = pc.wrapping_add(typ.imm);
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
                    pc = self.read_reg(typ.rs1).wrapping_add(typ.imm);
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
                                pc = pc.wrapping_add(typ.imm);
                                continue;
                            }
                        },
                        // BNE
                        0b001 => {
                            if self.read_reg(typ.rs1) != self.read_reg(typ.rs2) {
                                pc = pc.wrapping_add(typ.imm);
                                continue;
                            }
                        },
                        // BLT
                        0b100 => {
                            if (self.read_reg(typ.rs1) as i32) < self.read_reg(typ.rs2) as i32 {
                                pc = pc.wrapping_add(typ.imm);
                                continue;
                            }
                        },
                        // BGE
                        0b101 => {
                            if self.read_reg(typ.rs1) as i32 >= self.read_reg(typ.rs2) as i32 {
                                pc = pc.wrapping_add(typ.imm);
                                continue;
                            }
                        },
                        // BLTU
                        0b110 => {
                            if self.read_reg(typ.rs1) < self.read_reg(typ.rs2) {
                                pc = pc.wrapping_add(typ.imm);
                                continue;
                            }
                        },
                        // BGEU
                        0b111 => {
                            if self.read_reg(typ.rs1) >= self.read_reg(typ.rs2) {
                                pc = pc.wrapping_add(typ.imm);
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
                            let addr = self.read_reg(typ.rs1).wrapping_add(typ.imm);
                            let addr = addr as usize;
                            let data = read_mem!(addr, i8);
                            self.write_reg(typ.rd, data as i32 as u32);
                        },
                        // LH
                        0b001 => {
                            let addr = self.read_reg(typ.rs1).wrapping_add(typ.imm);
                            let addr = addr as usize;
                            let data = read_mem!(addr, i16);
                            self.write_reg(typ.rd, data as i32 as u32);
                        },
                        // LW
                        0b010 => {
                            let addr = self.read_reg(typ.rs1).wrapping_add(typ.imm);
                            let addr = addr as usize;
                            let data = read_mem!(addr, u32);
                            self.write_reg(typ.rd, data);
                        },
                        // LBU
                        0b100 => {
                            let addr = self.read_reg(typ.rs1).wrapping_add(typ.imm);
                            let addr = addr as usize;
                            let data = read_mem!(addr, u8);
                            self.write_reg(typ.rd, data as u32);
                        },
                        // LHU
                        0b101 => {
                            let addr = self.read_reg(typ.rs1).wrapping_add(typ.imm);
                            let addr = addr as usize;
                            let data = read_mem!(addr, u16);
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

                    let addr = self.read_reg(typ.rs1) + typ.imm;
                    let addr = addr as usize;

                    match typ.funct3 {
                        // SB
                        0b000 => {
                            let data = self.read_reg(typ.rs2) as u8;
                            write_mem!(addr, u8, data);
                        },
                        // SH
                        0b001 => {
                            let data = self.read_reg(typ.rs2) as u16;
                            write_mem!(addr, u16, data);
                        },
                        // SW
                        0b010 => {
                            let data = self.read_reg(typ.rs2);
                            write_mem!(addr, u32, data);
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
                            let data = self.read_reg(typ.rs1).wrapping_add(typ.imm);
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
                            self.write_reg(typ.rd, rs1.wrapping_add(rs2));
                        },
                        // SUB
                        (0b000, 0b0100000) => {
                            let rs1 = self.read_reg(typ.rs1);
                            let rs2 = self.read_reg(typ.rs2);
                            self.write_reg(typ.rd, rs1.wrapping_add(rs2));
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
