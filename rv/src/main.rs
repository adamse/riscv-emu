#![feature(array_chunks)]

mod instructions;
mod disassemble;
mod emulator;

use crate::disassemble::*;
use elf::Elf;

fn main() {
    let elf = Elf::load("../test/test").unwrap();
    // println!("{elf:#x?}");

    let code_seg = elf.load_segments.iter().find(|seg| seg.flags.x()).unwrap();

    let mut code = Vec::from(code_seg.data.clone());
    code.resize(code_seg.size as usize, 0);

    let entry = (elf.entry - code_seg.load_address) as usize;

    disassemble(elf.entry, &code[entry..entry+4*20]);

}
