.PHONY: help
help:
	@echo "Commands:"
	@echo
	@echo "toolchain -- download, build and install the rv32i newlib toolchain"

.PHONY: toolchain
toolchain:
	git clone https://github.com/riscv/riscv-gnu-toolchain
	cd riscv-gnu-toolchain
	./configure --prefix=$(ROOT)/riscv-rv32i/ --with-arch=rv32i --with-abi=ilp32
	$(MAKE) -j5

