OUTPUT_DIR := build
ASM_DIR := asm
OUTPUT_DIR_KEEP := $(OUTPUT_DIR)/.keep
IMG := $(OUTPUT_DIR)/haribote.img

default:
	make img

$(OUTPUT_DIR)/ipl.bin: $(ASM_DIR)/ipl.asm Makefile $(OUTPUT_DIR_KEEP)
$(OUTPUT_DIR)/asmhead.bin: $(ASM_DIR)/asmhead.asm Makefile $(OUTPUT_DIR_KEEP)

$(OUTPUT_DIR)/%.bin: $(ASM_DIR)/%.asm Makefile $(OUTPUT_DIR_KEEP)
	nasm $< -o $@

$(OUTPUT_DIR)/haribote.sys : $(OUTPUT_DIR)/asmhead.bin $(OUTPUT_DIR)/kernel.bin
	cat $^ > $@

$(IMG) : $(OUTPUT_DIR)/ipl.bin $(OUTPUT_DIR)/haribote.sys Makefile
	mformat -f 1440 -C -B $< -i $@ ::
	mcopy $(OUTPUT_DIR)/haribote.sys -i $@ ::

asm :
	make $(OUTPUT_DIR)/ipl.bin

img :
	make $(IMG)

run :
	make img
	qemu-system-i386 -fda $(IMG)

debug :
	make img
	qemu-system-i386 -fda $(IMG) -gdb tcp::10000 -S

clean :
	rm -rf $(OUTPUT_DIR)/*

$(OUTPUT_DIR)/kernel.bin: $(OUTPUT_DIR)/libharibote_os.a $(OUTPUT_DIR_KEEP)
	ld -v -nostdlib -m elf_i386 -Tdata=0x00310000 -Tkernel.ld $<  -o $@

$(OUTPUT_DIR)/libharibote_os.a: $(OUTPUT_DIR_KEEP)
	cargo xbuild --target-dir $(OUTPUT_DIR)
	cp $(OUTPUT_DIR)/i686-haribote/debug/libharibote_os.a $(OUTPUT_DIR)/

$(OUTPUT_DIR_KEEP):
	mkdir -p $(OUTPUT_DIR)
	touch $@