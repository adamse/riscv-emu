mod instructions;
mod disassemble;
mod emulator;

use crate::disassemble::*;

fn main() {
    // lui a0,0x11
    disassemble(0x00011537);
    // auipc   gp,0x5
    disassemble(0x00005197);
    // jal	ra,1049c <__call_exitprocs>
    disassemble(0x3f4000ef);
    // jalr	ra,0(a5)
    disassemble(0x000780e7);
    // jalr	ra,-96(a3)
    disassemble(0xfa0680e7);
    // beq	a5,zero,100bc <exit+0x28>
    disassemble(0x00078463);
    // bne	a5,zero,1015c <__do_global_dtors_aux+0x34>
    disassemble(0x02079263);
    // blt	a4,zero,103c0 <_puts_r+0x8c>
    disassemble(0x02074263);
    // bge	zero,a0,10cb0 <__sfvwrite_r+0x304>
    disassemble(0x06a05863);
    // bltu	a3,a5,10d78 <__sfvwrite_r+0x3cc>
    disassemble(0x04f6e463);
    // bgeu	s5,s3,10d48 <__sfvwrite_r+0x39c>
    disassemble(0x013af463);
    // lb ...
    // lh	a1,14(a1)
    disassemble(0x00e59583);
    // lw	a5,80(s0)
    disassemble(0x05042783);
    // lbu	a3,-4(a4)
    disassemble(0xffc74683);
    // lhu	a5,12(s0)
    disassemble(0x00c45783);
    // sb	a5,88(gp) # 14ed8 <completed.1>
    disassemble(0x04f18c23);
    // sh	a5,12(a1)
    disassemble(0x00f59623);
    // sw	a4,100(a1)
    disassemble(0x06e5a223);
    // addi	sp,sp,-16
    disassemble(0xff010113);
    // sltiu	a0,a0,1
    disassemble(0x00153513);
    // xori	s5,s5,-1024
    disassemble(0xc00aca93);
    // ori	a5,a5,128
    disassemble(0x0807e793);
    // andi	a5,a5,-129
    disassemble(0xf7f7f793);
    // slli	a3,a1,0x3
    disassemble(0x00359693);
    // srli	a2,a5,0x5
    disassemble(0x0057d613);
    // srai	a5,a1,0x2
    disassemble(0x4025d793);
    // add	a3,a3,a4
    disassemble(0x00e686b3);
    // sub	a3,t1,a2
    disassemble(0x40c306b3);
    // sll	a4,s5,s0
    disassemble(0x008a9733);
    // sltu	a0,zero,a0
    disassemble(0x00a03533);
    // xor	a4,a1,a4
    disassemble(0x00e5c733);
    // sra	a4,a4,a3
    disassemble(0x40d75733);
    // or	a5,a5,a0
    disassemble(0x00a7e7b3);
    // and	s2,s2,a5
    disassemble(0x00f97933);
    // ecall
    disassemble(0x00000073);
}