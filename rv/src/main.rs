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

    let heap_size = 2 * 1024 * 1024;
    let (heap_start, heap_end) = emu.mem.allocate(heap_size, PERM_READ | PERM_WRITE).unwrap();

    let mut current_brk = heap_start;

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
                        // However, the actual Linux system call returns the new program break  on
                        // success.   On  failure, the system call returns the current break.

                        let arg0 = emu.read_reg(Reg(10));
                        println!("brk {arg0:08x}");

                        if arg0 == 0 {
                        } else if arg0 > heap_end {
                            // todo do something about oom?
                        } else {
                            current_brk = arg0;
                        };

                        emu.write_reg(Reg(10), current_brk);

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
                let pc = emu.pc;
                panic!("Invalid instruction: 0x{pc:08x} {instr:#010x}");
            },
            EmulatorExit::InvalidMemoryAccess(err) => {
                panic!("Invalid memory access: {err:08x?}");
            }
        };
    }

}
