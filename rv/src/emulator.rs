use elf::Elf;
use crate::instructions::Reg;

pub struct Emulator {
    pub pc: u32,
    pub regs: [u32; 31],
    pub mem: Vec<u8>,

}

impl Emulator {
    pub fn new(elf: &Elf) -> Self {
        // 25 mb
        let mut mem = vec![0u8; 25 * 1024 * 1024];

        for segment in &elf.load_segments {
            let start = segment.load_address as usize;
            let end = start + segment.file_size as usize;
            mem[start..end].copy_from_slice(&segment.data);
        }

        Emulator {
            pc: elf.entry,
            regs: [0; 31],
            mem,
        }
    }

    fn set_reg(&mut self, reg: Reg, val: u32) {
        if reg.0 != 0 {
            self.regs[reg.0 as usize - 1] = val;
        }
    }

    fn get_reg(&self, reg: Reg) -> u32 {
        if reg.0 != 0 {
            self.regs[reg.0 as usize - 1]
        } else {
            0
        }
    }
}
