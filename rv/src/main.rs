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
    let elf = Elf::load("../test/test2").unwrap();

    let mut emu = Emulator::new(25*1024*1024);
    emu.load(&elf).unwrap();

    // allocate an initial stack

    // TODO: alignment??
    let stack_size = 1 * 1024 * 1096;
    let (stack_start, stack_end) = emu.mem.allocate(stack_size, PERM_RAW | PERM_WRITE).unwrap();

    println!("allocated stack: {:08x}-{:08x}", stack_start, stack_end);
    let mut sp = stack_end;

    // stack layout:
    // progname\0
    // aux vector, null terminated
    // env vector, null terminated
    // arg vector, null terminated
    // argc

    macro_rules! push {
        ($val:expr) => {{
            // allocate space
            sp -= $val.len() as u32;
            // write data
            emu.mem.write(sp, PERM_WRITE, &$val[..]).unwrap();
            println!("{sp:08x}: {:02x?}", $val);
            sp
        }}
    }

    let progname = b"test\0";
    let progname = push!(progname);

    // aux vector terminator
    push!(u32::to_le_bytes(0));
    push!(u32::to_le_bytes(0));
    // env vector terminator
    push!(u32::to_le_bytes(0));
    // argv vector
    push!(u32::to_le_bytes(0));
    push!(u32::to_le_bytes(progname));
    // argc
    push!(u32::to_le_bytes(1));
    emu.write_reg(RegName::Sp.as_reg(), sp);

    // allocate a heap

    let heap_size = 2 * 1024 * 1024;
    let (heap_start, heap_end) = emu.mem.allocate(heap_size, PERM_RAW | PERM_WRITE).unwrap();

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
                let ret = match syscall_no {
                    // long sys_close(unsigned int fd);
                    57 => {
                        let fd = emu.read_reg(Reg(10));
                        println!("close({fd})");

                        // return ok
                        0
                    },
                    // long sys_write(unsigned int fd, const char __user *buf, size_t count);
                    64 => {
                        let fd = emu.read_reg(Reg(10));
                        let buf = emu.read_reg(Reg(11));
                        let count = emu.read_reg(Reg(12));

                        println!("write({fd}, {buf:08x}, {count})");

                        if fd == 1 || fd == 2 {
                            // stdout or stderr
                            let bytes = emu.mem.read(buf..buf+count, PERM_READ).unwrap();
                            let string = String::from_utf8_lossy(bytes);
                            println!("output: {bytes:x?}");
                            println!("output: {string}");
                            0
                        } else {
                            !1 // TODO: right return for write ??
                        }
                    }
                    // fstat / newfstat(unsigned int fd, struct stat __user *statbuf)
                    80 => {
                        let fd = emu.read_reg(Reg(10));
                        let buf = emu.read_reg(Reg(11));
                        println!("fstat({fd}, {buf:08x})");
                        // size of kernel stat struct is 128 bytes
                        emu.mem.write(buf, PERM_WRITE, &[0; 128]).unwrap();

                        // just return ok
                        0
                    },
                    // long sys_exit(int error_code);
                    93 => {
                        let code = emu.read_reg(Reg(10)) as i32;
                        println!("exit({code})");

                        return;
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
                        }

                        current_brk
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
                };

                println!("ret = {ret}");
                // set return value
                emu.write_reg(Reg(10), ret);

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
