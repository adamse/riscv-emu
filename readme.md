# riscv emulator and things

## riscv newlib toolchain

https://github.com/riscv-collab/riscv-gnu-toolchain

ubuntu 22.04 build
https://github.com/riscv-collab/riscv-gnu-toolchain/releases/download/2023.01.04/riscv32-elf-ubuntu-22.04-nightly-2023.01.04-nightly.tar.gz

build

```
./configure --prefix=$HOME/rv/riscv-rv32i/ --with-arch=rv32i --with-abi=ilp32
make -j5
```

## riscv syscall convention

syscall no: a7 (x17)
syscall args: a0-a5 (x10-x15)
return: a0 (x10)

numbers: https://jborza.com/post/2021-05-11-riscv-linux-syscalls/

porting newlib guide: <https://www.embecosm.com/appnotes/ean9/ean9-howto-newlib-1.0.html#sec_sbrk>
