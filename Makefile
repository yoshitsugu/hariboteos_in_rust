OUTPUT_DIR := build
ASM_DIR := asm
APPS_DIR := apps
OUTPUT_DIR_KEEP := $(OUTPUT_DIR)/.keep
IMG := $(OUTPUT_DIR)/haribote.img

default:
	make img

$(OUTPUT_DIR)/%.bin: $(ASM_DIR)/%.asm Makefile $(OUTPUT_DIR_KEEP)
	nasm  $< -o $@

$(OUTPUT_DIR)/haribote.sys : $(OUTPUT_DIR)/asmhead.bin $(OUTPUT_DIR)/kernel.bin
	cat $^ > $@

$(IMG) : $(OUTPUT_DIR)/ipl.bin $(OUTPUT_DIR)/haribote.sys fonts/nihongo.fnt $(OUTPUT_DIR)/prim.hrb $(OUTPUT_DIR)/lines.hrb $(OUTPUT_DIR)/timer.hrb $(OUTPUT_DIR)/beepdown.hrb $(OUTPUT_DIR)/color.hrb $(OUTPUT_DIR)/iroha.hrb $(OUTPUT_DIR)/cat.hrb $(OUTPUT_DIR)/chklang.hrb $(OUTPUT_DIR)/notrec.hrb $(OUTPUT_DIR)/bball.hrb $(OUTPUT_DIR)/invader.hrb Makefile
	mformat -f 1440 -C -B $< -i $@ ::
	mcopy $(OUTPUT_DIR)/haribote.sys -i $@ ::
	mcopy $(OUTPUT_DIR)/lines.hrb -i $@ ::
	mcopy $(OUTPUT_DIR)/timer.hrb -i $@ ::
	mcopy $(OUTPUT_DIR)/beepdown.hrb -i $@ ::
	mcopy $(OUTPUT_DIR)/color.hrb -i $@ ::
	mcopy $(OUTPUT_DIR)/prim.hrb -i $@ ::
	mcopy $(OUTPUT_DIR)/iroha.hrb -i $@ ::
	mcopy $(OUTPUT_DIR)/cat.hrb -i $@ ::
	mcopy $(OUTPUT_DIR)/chklang.hrb -i $@ ::
	mcopy $(OUTPUT_DIR)/notrec.hrb -i $@ ::
	mcopy $(OUTPUT_DIR)/bball.hrb -i $@ ::
	mcopy $(OUTPUT_DIR)/invader.hrb -i $@ ::
	mcopy texts/euc.txt -i $@ ::
	mcopy fonts/nihongo.fnt -i $@ ::

asm :
	make $(OUTPUT_DIR)/ipl.bin 

img :
	make $(IMG)

run :
	make img
	qemu-system-i386 -m 32 -fda $(IMG) -no-reboot

debug :
	make img
	qemu-system-i386 -fda $(IMG) -gdb tcp::10000 -S

clean :
	rm -rf $(OUTPUT_DIR)/*

$(OUTPUT_DIR)/asmfunc.o: $(ASM_DIR)/asmfunc.asm Makefile $(OUTPUT_DIR_KEEP)
	nasm -f elf $< -o $@

$(OUTPUT_DIR)/kernel.bin: $(OUTPUT_DIR)/libharibote_os.a $(OUTPUT_DIR)/asmfunc.o $(OUTPUT_DIR_KEEP)
	ld -v -nostdlib -m elf_i386 -Tdata=0x00310000 -Tkernel.ld $< $(OUTPUT_DIR)/asmfunc.o -o $@

$(OUTPUT_DIR)/libharibote_os.a: $(OUTPUT_DIR_KEEP)
	cargo xbuild --target-dir $(OUTPUT_DIR)
	cp $(OUTPUT_DIR)/i686-haribote/debug/libharibote_os.a $(OUTPUT_DIR)/

$(OUTPUT_DIR_KEEP):
	mkdir -p $(OUTPUT_DIR)
	touch $@

$(OUTPUT_DIR)/%.a: $(APPS_DIR)/%
	cd $(APPS_DIR)/$* && cargo xbuild --target-dir ../../$(OUTPUT_DIR)
	cp $(OUTPUT_DIR)/i686-haribote/debug/lib$*.a $@

$(OUTPUT_DIR)/app_asmfunc.o: apps/asmfunc.asm Makefile $(OUTPUT_DIR_KEEP)
	nasm -f elf $< -o $@

$(OUTPUT_DIR)/crack7.hrb: $(ASM_DIR)/crack7.asm $(OUTPUT_DIR)/app_asmfunc.o $(OUTPUT_DIR_KEEP)
	nasm -f elf $< -o $(OUTPUT_DIR)/crack7.o
	ld -v -nostdlib -m elf_i386 -Tdata=0x00310000 -Tkernel.ld $(OUTPUT_DIR)/crack7.o $(OUTPUT_DIR)/app_asmfunc.o -o $@

$(OUTPUT_DIR)/%.hrb: $(OUTPUT_DIR)/%.a $(OUTPUT_DIR)/app_asmfunc.o $(OUTPUT_DIR_KEEP)
	ld -v -nostdlib -m elf_i386 -Tdata=0x00310000 -Tkernel.ld $< $(OUTPUT_DIR)/app_asmfunc.o -o $@
