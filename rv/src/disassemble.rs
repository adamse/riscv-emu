use crate::instructions::*;

pub fn disassemble(addr: u32, instrs: &[u8]) {
    for (ii, instr) in instrs.array_chunks::<4>().enumerate() {
        disassemble_one(addr + (ii * 4) as u32, u32::from_le_bytes(*instr), false);
    }
}

pub fn disassemble_one(addr: u32, instr: u32, abi_name: bool) {

    // first 7 bits are the opcode
    let opcode: u32 = instr & ((1 << 7) - 1);

    // follow the table on page 130 in the riscv spec
    match opcode {
        // LUI
        0b0110111 => {
            let typ = UType::parse(instr);
            println!("lui {}, imm={:#08x}", typ.rd.name2(abi_name), typ.imm);
        },
        // AUIPC
        0b0010111 => {
            let typ = UType::parse(instr);
            println!("auipc {}, imm={:#08x}", typ.rd.name2(abi_name), typ.imm);
        },
        // JAL
        0b1101111 => {
            let typ = JType::parse(instr);
            println!("jal {}, rel={}, abs={:#08x}",
                typ.rd.name2(abi_name),
                typ.imm,
                (addr as i32 + typ.imm as i32) as u32);
        },
        // JALR
        0b1100111 => {
            let typ = IType::parse(instr);

            assert!(typ.funct3 == 0,
                "JALR should have funct3 == 0");

            println!("jalr {}, {}, rel={}",
                typ.rd.name2(abi_name),
                typ.rs1.name2(abi_name),
                typ.imm as i32)
        },

        // BRANCH
        0b1100011 => {
            let typ = BType::parse(instr);
            match typ.funct3 {
                // BEQ
                0b000 => {
                    println!("beq {}, {}, rel={}, abs={:#08x}",
                        typ.rs1.name2(abi_name),
                        typ.rs2.name2(abi_name),
                        typ.imm,
                        (addr as i32 + typ.imm as i32) as u32);
                },
                // BNE
                0b001 => {
                    println!("bne {}, {}, rel={}, abs={:#08x}",
                        typ.rs1.name2(abi_name),
                        typ.rs2.name2(abi_name),
                        typ.imm,
                        (addr as i32 + typ.imm as i32) as u32);
                },
                // BLT
                0b100 => {
                    println!("blt {}, {}, rel={}, abs={:#08x}",
                        typ.rs1.name2(abi_name),
                        typ.rs2.name2(abi_name),
                        typ.imm,
                        (addr as i32 + typ.imm as i32) as u32);
                },
                // BGE
                0b101 => {
                    println!("bge {}, {}, rel={}, abs={:#08x}",
                        typ.rs1.name2(abi_name),
                        typ.rs2.name2(abi_name),
                        typ.imm,
                        (addr as i32 + typ.imm as i32) as u32);
                },
                // BLTU
                0b110 => {
                    println!("bltu {}, {}, rel={}, abs={:#08x}",
                        typ.rs1.name2(abi_name),
                        typ.rs2.name2(abi_name),
                        typ.imm as i32,
                        (addr as i32 + typ.imm as i32) as u32);
                },
                // BGEU
                0b111 => {
                    println!("bgeu {}, {}, rel={}, abs={:#08x}",
                        typ.rs1.name2(abi_name),
                        typ.rs2.name2(abi_name),
                        typ.imm,
                        (addr as i32 + typ.imm as i32) as u32);
                },
                funct3 => {
                    panic!("Unknown BRANCH: {funct3:#03b}");
                },
            };
        },

        // LOAD
        0b0000011 => {
            let typ = IType::parse(instr);
            match typ.funct3 {
                // LB
                0b000 => {
                    println!("lb {}, {}, rel={}",
                        typ.rd.name2(abi_name),
                        typ.rs1.name2(abi_name),
                        typ.imm as i32);
                },
                // LH
                0b001 => {
                    println!("lh {}, {}, rel={}",
                        typ.rd.name2(abi_name),
                        typ.rs1.name2(abi_name),
                        typ.imm as i32);
                },
                // LW
                0b010 => {
                    println!("lw {}, {}, rel={}",
                        typ.rd.name2(abi_name),
                        typ.rs1.name2(abi_name),
                        typ.imm as i32);
                },
                // LBU
                0b100 => {
                    println!("lbu {}, {}, rel={}",
                        typ.rd.name2(abi_name),
                        typ.rs1.name2(abi_name),
                        typ.imm as i32);
                },
                // LHU
                0b101 => {
                    println!("lhu {}, {}, rel={}",
                        typ.rd.name2(abi_name),
                        typ.rs1.name2(abi_name),
                        typ.imm as i32);
                },
                funct3 => {
                    panic!("Uknown LOAD: {funct3:#03b}");
                },
            };
        },

        // STORE
        0b0100011 => {
            let typ = SType::parse(instr);
            match typ.funct3 {
                // SB
                0b000 => {
                    println!("sb {}, {}, rel={}",
                        typ.rs2.name2(abi_name),
                        typ.rs1.name2(abi_name),
                        typ.imm as i32);
                },
                // SH
                0b001 => {
                    println!("sh {}, {}, rel={}",
                        typ.rs2.name2(abi_name),
                        typ.rs1.name2(abi_name),
                        typ.imm as i32);
                },
                // SW
                0b010 => {
                    println!("sw {}, {}, rel={}",
                        typ.rs2.name2(abi_name),
                        typ.rs1.name2(abi_name),
                        typ.imm as i32);
                },
                funct3 => {
                    panic!("Uknown LOAD: {funct3:#03b}");
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
                    println!("addi {}, {}, {}",
                        typ.rd.name2(abi_name),
                        typ.rs1.name2(abi_name),
                        typ.imm as i32);
                },
                // SLTI
                0b010 => {
                    println!("slti {}, {}, {}",
                        typ.rd.name2(abi_name),
                        typ.rs1.name2(abi_name),
                        typ.imm as i32);
                },
                // SLTIU
                0b011 => {
                    println!("sltiu {}, {}, {}",
                        typ.rd.name2(abi_name),
                        typ.rs1.name2(abi_name),
                        typ.imm);
                },
                // XORI
                0b100 => {
                    println!("xori {}, {}, {:#08x}",
                        typ.rd.name2(abi_name),
                        typ.rs1.name2(abi_name),
                        typ.imm);
                },
                // ORI
                0b110 => {
                    println!("ori {}, {}, {:#08x}",
                        typ.rd.name2(abi_name),
                        typ.rs1.name2(abi_name),
                        typ.imm);
                },
                // ANDI
                0b111 => {
                    println!("andi {}, {}, {:#08x}",
                        typ.rd.name2(abi_name),
                        typ.rs1.name2(abi_name),
                        typ.imm);
                },
                // SLLI
                0b001 => {
                    assert!(arithmetic == 0b0);
                    println!("slli {}, {}, {}",
                        typ.rd.name2(abi_name),
                        typ.rs1.name2(abi_name),
                        shamt);
                },
                // SRLI & SRAI
                0b101 => {
                    match arithmetic {
                        // SRLI
                        0b0 => {
                            println!("srli {}, {}, {}",
                                typ.rd.name2(abi_name),
                                typ.rs1.name2(abi_name),
                                shamt);
                                },
                        // SRAI
                        0b0100000 => {
                            println!("srai {}, {}, {}",
                                typ.rd.name2(abi_name),
                                typ.rs1.name2(abi_name),
                                shamt);
                                },
                        _ => {
                            panic!("Uknown SRLI/SRAI: {arithmetic:#07b}");
                        },
                    };
                },
                funct3 => {
                    panic!("Uknown OP-IMM: {funct3:#03b}");
                },
            };
        },

        // OP
        0b0110011 => {
            let typ = RType::parse(instr);

            match (typ.funct3, typ.funct7) {
                // ADD
                (0b000, 0b0000000) => {
                    println!("add {}, {}, {}",
                        typ.rd.name2(abi_name),
                        typ.rs1.name2(abi_name),
                        typ.rs2.name2(abi_name));
                },
                // SUB
                (0b000, 0b0100000) => {
                    println!("sub {}, {}, {}",
                        typ.rd.name2(abi_name),
                        typ.rs1.name2(abi_name),
                        typ.rs2.name2(abi_name));
                },
                // SLL
                (0b001, 0b0000000) => {
                    println!("sll {}, {}, {}",
                        typ.rd.name2(abi_name),
                        typ.rs1.name2(abi_name),
                        typ.rs2.name2(abi_name));
                },
                // SLT
                (0b010, 0b0000000) => {
                    println!("slt {}, {}, {}",
                        typ.rd.name2(abi_name),
                        typ.rs1.name2(abi_name),
                        typ.rs2.name2(abi_name));
                },
                // SLTU
                (0b011, 0b0000000) => {
                    println!("sltu {}, {}, {}",
                        typ.rd.name2(abi_name),
                        typ.rs1.name2(abi_name),
                        typ.rs2.name2(abi_name));
                },
                // XOR
                (0b100, 0b0000000) => {
                    println!("xor {}, {}, {}",
                        typ.rd.name2(abi_name),
                        typ.rs1.name2(abi_name),
                        typ.rs2.name2(abi_name));
                },
                // SRL
                (0b101, 0b0000000) => {
                    println!("slr {}, {}, {}",
                        typ.rd.name2(abi_name),
                        typ.rs1.name2(abi_name),
                        typ.rs2.name2(abi_name));
                },
                // SRA
                (0b101, 0b0100000) => {
                    println!("sra {}, {}, {}",
                        typ.rd.name2(abi_name),
                        typ.rs1.name2(abi_name),
                        typ.rs2.name2(abi_name));
                },
                // OR
                (0b110, 0b0000000) => {
                    println!("or {}, {}, {}",
                        typ.rd.name2(abi_name),
                        typ.rs1.name2(abi_name),
                        typ.rs2.name2(abi_name));
                },
                // AND
                (0b111, 0b0000000) => {
                    println!("and {}, {}, {}",
                        typ.rd.name2(abi_name),
                        typ.rs1.name2(abi_name),
                        typ.rs2.name2(abi_name));
                },
                (funct3, funct7) => {
                    panic!("Uknown OP-IMM: funct3={funct3:#03b}, funct7={funct7:#07b}");
                },
            };
        }

        // MISC-MEM
        0b0001111 => {
            let typ = IType::parse(instr);
            // FENCE
            assert!(typ.funct3 == 0b000,
                "FENCE must have funct3=0b00, found {:#03b}", typ.funct3);
            // TODO: more junk to print?
            println!("fence");
        },

        // SYSTEM
        0b1110011 => {
            let typ = IType::parse(instr);
            assert!(typ.rs1.0 == 0,
                "rs1 must be 0 for SYSTEM instruction, found {:#02x}", typ.rs1.0);
            assert!(typ.rd.0 == 0,
                "rd must be 0 for SYSTEM instruction, found {:#02x}", typ.rd.0);
            assert!(typ.funct3 == 0,
                "funct3 must be 0 for SYSTEM instruction, found {:#03b}", typ.funct3);
            match typ.imm {
                // ECALL
                0b0 => {
                    println!("ecall");
                },
                0b1 => {
                    println!("ebreak");
                },
                imm => {
                    panic!("unknown SYSTEM instruction {imm:#011b}");
                },
            }
        },

        _ => { panic!("Unknown opcode: {opcode:032b}"); },
    }
}
