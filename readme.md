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
