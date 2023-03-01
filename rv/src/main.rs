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

    let mut emu = Emulator::new(25*1024*1024);
    emu.load(&elf).unwrap();

    // allocate an initial stack

    let stack_size = 1 * 1024 * 1096;
    // TODO: make the stack read-after-write
    let (stack_start, stack_end) = emu.mem.allocate(stack_size, PERM_READ | PERM_WRITE).unwrap();

    emu.write_reg(RegName::Sp.as_reg(), stack_start as u32);

    println!("allocated stack: {:08x}-{:08x}", stack_start, stack_end);


    // allocate a heap

    let heap_size = 2 * 1024 * 1024;
    let (heap_start, heap_end) = emu.mem.allocate(heap_size, PERM_READ | PERM_WRITE).unwrap();

    println!("allocated heap:  {heap_start:08x}-{heap_end:08x}");

    // store the current "end of heap" as the program believes it to be
    let mut current_brk = heap_start;


    loop {
        let res = emu.run();

        match res {
            EmulatorExit::Syscall => {
                // sycall no is in a7/x17
                let syscall_no = emu.read_reg(Reg(17));
                println!("syscall: {syscall_no}");
                match syscall_no {
                    // read long sys_write(unsigned int fd, const char __user *buf, size_t count);
                    64 => {
                        let fd = emu.read_reg(Reg(10));
                        let buf = emu.read_reg(Reg(11));
                        let count = emu.read_reg(Reg(12));

                        println!("write({fd}, {buf:08x}, {count})");

                        let ret = if fd == 1 || fd == 2 {
                            // stdout or stderr
                            let bytes = emu.mem.read(buf..buf+count, PERM_READ).unwrap();
                            let string = String::from_utf8_lossy(bytes);
                            println!("output: {bytes:x?}");
                            println!("output: {string}");
                            0
                        } else {
                            !1 // TODO: right return for write ??
                        };

                        emu.write_reg(Reg(10), ret);
                    }
                    // fstat / newfstat(unsigned int fd, struct stat __user *statbuf)
                    80 => {
                        let fd = emu.read_reg(Reg(10));
                        let buf = emu.read_reg(Reg(11));
                        println!("fstat({fd}, {buf:08x})");
                        // just return ok
                        emu.write_reg(Reg(10), 0);
                    },
                    // brk / long sys_brk(unsigned long brk)
                    214 => {
                        // However, the actual Linux system call returns the new program break  on
                        // success.   On  failure, the system call returns the current break.

                        let new_brk = emu.read_reg(Reg(10));
                        println!("brk({new_brk:08x})");

                        if new_brk == 0 {
                        } else if new_brk > heap_end {
                            // todo do something about oom?
                        } else {
                            current_brk = new_brk;
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
