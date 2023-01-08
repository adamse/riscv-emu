struct Regs {
    /// x1-x31 registers
    regs: [u32; 31],
}

/// Creates a getter and a setter for a register
///
/// `macro_rules!(reg getter name, reg setter name, register index)`
macro_rules! make_register_get_set {
    ($regget:ident, $regset:ident, $regidx:literal) => {
        fn $regget(&self) -> u32 {
            self.regs[$regidx]
        }
        fn $regset(&mut self, reg: u32) {
            self.regs[$regidx] = reg;
        }
    };
}

impl Regs {
    /// read X0
    fn x0(&self) -> u32 { 0 }
}

use crate::instructions::*;

struct Emulator {
    mem: Vec<u8>,
    pc: Reg,
    regs: Regs,
}

impl Emulator {
}
