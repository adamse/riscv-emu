#![feature(array_chunks)]
#![feature(new_uninit)]

mod instructions;
mod disassemble;
mod emulator;

// use crate::disassemble::*;
use crate::emulator::*;
use crate::instructions::*;
use elf::Elf;

fn main() {
    let elf = Elf::load("../test/test").unwrap();

    let mut emu = Emulator::new(&elf);

    // find the first free memory address after all the segments loaded from the elf
    let end_of_elf_loads =
        elf.load_segments.iter()
        .map(|segment| segment.load_address + segment.size)
        .max().unwrap_or(0);

    let mut brk = end_of_elf_loads;

    loop {
        let res = emu.run();

        match res {
            EmulatorExit::Syscall => {
                // sycall no is in a7/x17
                let syscall_no = emu.read_reg(Reg(17));
                println!("Syscall no: {syscall_no}");
                match syscall_no {
                    // fstat / newfstat(unsigned int fd, struct stat __user *statbuf)
                    80 => {
                        // just return ok
                            // panic!("fstat");
                        emu.write_reg(Reg(10), 0);
                    },
                    // brk / long sys_brk(unsigned long brk)
                    214 => {
                        let arg0 = emu.read_reg(Reg(10));
                        println!("brk {arg0}");
                        if arg0 == 0 {
                            // return current brk
                            emu.write_reg(Reg(10), brk);
                            panic!("brk: {arg0}");
                        } else {
                            // update brk if possible
                            brk = arg0;
                            emu.write_reg(Reg(10), brk);
                            panic!("brk: {arg0}");
                        }
                    },
                    x => {
                        let arg0 = emu.read_reg(Reg(10));
                        let arg1 = emu.read_reg(Reg(11));
                        let arg2 = emu.read_reg(Reg(12));
                        let arg3 = emu.read_reg(Reg(13));
                        let arg4 = emu.read_reg(Reg(14));
                        let arg5 = emu.read_reg(Reg(15));
                        panic!("Unhandled syscall no: {x} {arg0:08x} {arg1:08x} {arg2:08x} {arg3:08x} {arg4:08x} {arg5:08x}");
                    }
                }

                // update pc to next instruction
                emu.pc = emu.pc + 4;
            },
            EmulatorExit::Break => {
                panic!("Unhandled break");
            },
            EmulatorExit::InvalidInstruction(instr) => {
                panic!("Invalid instruction: {instr:#010x}");
            },
            EmulatorExit::InvalidMemoryAccess { perm, addr } => {
                panic!("Invalid memory access: couldn't {perm:?} at {addr:#010x}");
            }
        };
    }

}
