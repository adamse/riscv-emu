// use std::io::Write;
use std::ops::Range;

use elf::Elf;

use rangeset::RangeSet;

use crate::instructions::*;
use crate::disassemble::*;

const TRACE: bool = false;

pub const PERM_NONE: u8 = 0b0;

pub const PERM_READ: u8 = 0b1;

pub const PERM_WRITE: u8 = 0b10;

pub const PERM_EXEC: u8 = 0b100;

/// Read after write
pub const PERM_RAW: u8 = 0b1000;

fn test_perm(permission: u8, byte: u8) -> bool {
    (permission & byte) != 0
}

#[derive(Debug)]
pub enum MemoryError {
    BadPermissions {
        addr: Range<u32>,
        access: u8,
        perms: u8,
    },
    OutOfBounds {
        addr: Range<u32>
    },
    OutOfMemory {
        err: rangeset::Error,
    },
}

impl MemoryError {
    pub fn from_range_error(err: rangeset::Error) -> Self {
        MemoryError::OutOfMemory { err }
    }
}

#[derive(Debug)]
pub struct Memory {
    pub mem: Box<[u8]>,
    pub perms: Box<[u8]>,
    pub free: RangeSet,
}

macro_rules! readu_impl {
    ($name:ident, $ty:ty) => {
        pub fn $name(&self, addr: u32, perm: u8) -> Result<u32, MemoryError> {
            const SIZE: usize = std::mem::size_of::<$ty>();

            let slice = self.read(addr..addr+(SIZE as u32), perm)?;

            Ok(<$ty>::from_le_bytes(slice.try_into().unwrap()) as u32)
        }
    }
}

macro_rules! readi_impl {
    ($name:ident, $ty:ty) => {
        pub fn $name(&self, addr: u32, perm: u8) -> Result<u32, MemoryError> {
            const SIZE: usize = std::mem::size_of::<$ty>();

            let slice = self.read(addr..addr+(SIZE as u32), perm)?;

            Ok(<$ty>::from_le_bytes(slice.try_into().unwrap()) as i32 as u32)
        }
    }
}

macro_rules! write_impl {
    ($name:ident, $ty:ty) => {
        pub fn $name(&mut self, addr: u32, perm: u8, val: $ty) -> Result<(), MemoryError> {
            self.write(addr, perm, &<$ty>::to_le_bytes(val))
        }
    }
}

impl Memory {
    pub fn new(size: u32) -> Self {
        let mem = Box::new_zeroed_slice(size as usize);
        let mem = unsafe { mem.assume_init() };

        let perms = Box::new_zeroed_slice(size as usize);
        let perms = unsafe { perms.assume_init() };

        Memory {
            mem,
            perms,
            free: RangeSet::new(0, size),
        }
    }

    pub fn allocate(&mut self, size: u32, perms: u8) -> Result<(u32, u32), MemoryError> {
        let (start, end) = self.free.remove_first_fit(size).map_err(MemoryError::from_range_error)?;
        self.set_permissions(start..end, perms)?;
        Ok((start, end))
    }

    fn check_bounds(&self, range: Range<u32>) -> Result<(), MemoryError> {
        if range.start as usize >= self.mem.len() || range.end as usize >= self.mem.len() {
            return Err(MemoryError::OutOfBounds {
                addr: range
            });
        }

        Ok(())
    }

    pub fn set_permissions(&mut self, range: Range<u32>, perm: u8) -> Result<(), MemoryError> {
        self.check_bounds(range.clone())?;

        for ii in range {
            self.perms[ii as usize] = perm;
        }

        Ok(())
    }

    pub fn check_permission(&self, range: Range<u32>, perm: u8) -> Result<(), MemoryError> {
        for addr in range.clone() {
            let byte = self.perms[addr as usize];
            if !test_perm(perm, byte) {
                return Err(MemoryError::BadPermissions {
                    addr: range,
                    access: perm as u8,
                    perms: byte,
                });
            }
        }

        Ok(())
    }

    pub fn read(&self, range: Range<u32>, perm: u8) -> Result<&[u8], MemoryError> {
        self.check_bounds(range.clone())?;

        if !matches!(perm, PERM_NONE) {
            self.check_permission(range.clone(), perm)?;
        }

        // println!("{:08x?}", range.clone());
        // dbg!(&self.mem[range.start as usize..range.end as usize]);
        Ok(&self.mem[range.start as usize..range.end as usize])
    }

    readu_impl!(read_u8, u8);
    readu_impl!(read_u16, u16);
    readu_impl!(read_u32, u32);
    readi_impl!(read_i8, i8);
    readi_impl!(read_i16, i16);

    pub fn write(&mut self, addr: u32, perm: u8, data: &[u8]) -> Result<(), MemoryError> {
        let range = addr..addr+data.len() as u32;

        self.check_bounds(range.clone())?;

        if perm != 0 {
            self.check_permission(range.clone(), perm)?;
        }

        // reset the RAW bit and set the READ bit
        for ii in range {
            let mut perm = self.perms[ii as usize];
            if perm & PERM_RAW != 0 {
                perm &= !PERM_RAW;
                perm |= PERM_READ;
                self.perms[ii as usize] = perm;
            }
        }

        self.mem[addr as usize..addr as usize + data.len()].copy_from_slice(data);

        Ok(())
    }

    write_impl!(write_u8, u8);
    write_impl!(write_u16, u16);
    write_impl!(write_u32, u32);

}

#[derive(Debug)]
pub struct Emulator {
    pub pc: u32,
    pub regs: [u32; 31],
    pub mem: Memory,
}

#[derive(Debug)]
pub enum EmulatorExit {
    Syscall,
    Break,
    InvalidInstruction(u32),
    InvalidMemoryAccess(MemoryError),
}

impl Emulator {
    pub fn new(memory_size: u32) -> Self {


        Emulator {
            pc: 0,
            regs: [0; 31],
            mem: Memory::new(memory_size),
        }
    }

    pub fn load(&mut self, elf: &Elf) -> Result<(), MemoryError> {
        self.pc = elf.entry;

        for segment in &elf.load_segments {
            let start = segment.load_address;
            let file_end = start + segment.file_size;

            self.mem.write(start, PERM_NONE, &segment.data)?;

            let perm =
                if segment.flags.r() { PERM_READ } else { 0 } |
                if segment.flags.w() { PERM_WRITE } else { 0 } |
                if segment.flags.x() { PERM_EXEC } else { 0 };

            let mem_end = start + segment.size;
            // align up to next word
            let mem_end = (mem_end + 4) & !3;

            // remove this range from free memory
            self.mem.free.remove(start, mem_end).map_err(MemoryError::from_range_error)?;

            // set the permissions as the elf specifies
            self.mem.set_permissions(start..mem_end, perm)?;

            println!("loading segment: {:08x}-{:08x}-{:08x} {:?}", start, file_end, mem_end, segment.flags);
        }

        Ok(())
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

    /*
    /// write current instruction and register state to the trace file
    ///
    /// A trace record is (1 + 1 + 31) * 4 bytes long (instruction, pc, general purpose registers)
    fn trace_binary(&self, pc: u32, file: &mut std::fs::File) {
        // write the instruction to the trace
        let instr = &self.mem[pc as usize..][..4];
        file.write_all(instr).unwrap();

        // write pc to the trace
        let pc = pc.to_le_bytes();
        file.write_all(&pc[..]).unwrap();

        // write all the other registers to the trace
        let regs = &self.regs as *const u32 as *const u8;
        let regs = unsafe {
            std::slice::from_raw_parts(regs, self.regs.len() * std::mem::size_of::<u32>())
        };
        file.write_all(regs).unwrap();
    }
    */

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
            ($ret:expr) => {{
                ret = $ret;
                break;
            }}
        }

        'next_instruction: loop {

            let instr =
                self.mem.read_u32(pc, PERM_EXEC);
            let instr = match instr {
                Err(memerr) => exit!(EmulatorExit::InvalidMemoryAccess(memerr)),
                Ok(instr) => instr,
            };

            if TRACE {
                self.trace_print2(pc);
                disassemble_one(pc as u32, instr, true);
                println!("");
            }

            // before bzero bss
            if pc == 0x000100ec {
                println!("start: {:08x}, end: {:08x}, len: {:}",
                    self.read_reg(RegName::A0.as_reg()),
                    self.read_reg(RegName::A2.as_reg()),
                    self.read_reg(RegName::A2.as_reg()) - self.read_reg(RegName::A0.as_reg())
                )
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
                    continue 'next_instruction;
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
                    continue 'next_instruction;

                },

                // BRANCH
                0b1100011 => {
                    let typ = BType::parse(instr);

                    let take_branch = match typ.funct3 {
                        // BEQ
                        0b000 => self.read_reg(typ.rs1) == self.read_reg(typ.rs2),

                        // BNE
                        0b001 => self.read_reg(typ.rs1) != self.read_reg(typ.rs2),

                        // BLT
                        0b100 => (self.read_reg(typ.rs1) as i32) < self.read_reg(typ.rs2) as i32,

                        // BGE
                        0b101 => self.read_reg(typ.rs1) as i32 >= self.read_reg(typ.rs2) as i32,

                        // BLTU
                        0b110 => self.read_reg(typ.rs1) < self.read_reg(typ.rs2),

                        // BGEU
                        0b111 => self.read_reg(typ.rs1) >= self.read_reg(typ.rs2),

                        _ => exit!(EmulatorExit::InvalidInstruction(instr)),
                    };

                    if take_branch {
                        pc = pc.wrapping_add(typ.imm);
                        continue 'next_instruction;
                    }
                },

                // LOAD
                0b0000011 => {
                    let typ = IType::parse(instr);

                    let addr = self.read_reg(typ.rs1).wrapping_add(typ.imm);

                    let data = match typ.funct3 {
                        // LB
                        0b000 => self.mem.read_i8(addr, PERM_READ),
                        // LH
                        0b001 => self.mem.read_i16(addr, PERM_READ),
                        // LW
                        0b010 => self.mem.read_u32(addr, PERM_READ),
                        // LBU
                        0b100 => self.mem.read_u8(addr, PERM_READ),
                        // LHU
                        0b101 => self.mem.read_u16(addr, PERM_READ),
                        _ => {
                            exit!(EmulatorExit::InvalidInstruction(instr));
                        },
                    };
                    match data {
                        Err(memerr) =>
                            exit!(EmulatorExit::InvalidMemoryAccess(memerr)),
                        Ok(data) =>
                            self.write_reg(typ.rd, data as i32 as u32),
                    }
                },

                // STORE
                0b0100011 => {
                    let typ = SType::parse(instr);

                    let addr = self.read_reg(typ.rs1) + typ.imm;
                    let data = self.read_reg(typ.rs2);

                    let res = match typ.funct3 {
                        // SB
                        0b000 => self.mem.write_u8(addr, PERM_WRITE, data as u8),

                        // SH
                        0b001 => self.mem.write_u16(addr, PERM_WRITE, data as u16),

                        // SW
                        0b010 => self.mem.write_u32(addr, PERM_WRITE, data as u32),

                        _ => exit!(EmulatorExit::InvalidInstruction(instr)),
                    };

                    match res {
                        Err(memerr) => exit!(EmulatorExit::InvalidMemoryAccess(memerr)),
                        Ok(()) => (),
                    }
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
                            let data = self.read_reg(typ.rs1).wrapping_add_signed(typ.imm as i32);
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
                            self.write_reg(typ.rd, rs1.wrapping_sub(rs2));
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
