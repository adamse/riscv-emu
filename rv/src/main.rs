#![feature(array_chunks)]

mod instructions;
mod disassemble;
mod emulator;

use crate::disassemble::*;
use crate::emulator::*;
use elf::Elf;

fn main() {
    let elf = Elf::load("../test/test").unwrap();

    let code_seg = elf.load_segments.iter().find(|seg| seg.flags.x()).unwrap();

    let mut emu = Emulator::new(&elf);
    let res = emu.run();
    println!("emulator exited with: {res:?}");

}
