#![feature(array_chunks)]

mod instructions;
mod disassemble;
mod emulator;

use crate::disassemble::*;
use crate::emulator::*;
use elf::Elf;

fn main() {
    let elf = Elf::load("../test/test").unwrap();
    // println!("{elf:#x?}");

    let code_seg = elf.load_segments.iter().find(|seg| seg.flags.x()).unwrap();

    let entry = (elf.entry - code_seg.load_address) as usize;

    disassemble(elf.entry, &code_seg.data[entry..entry+4*20]);

    let _emu = Emulator::new(&elf);

}
