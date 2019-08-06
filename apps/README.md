### アプリケーション作成手順

1. 他のアプリをコピー

```bash
$ cp -r lines xxx
```

2. ファイル名を書き換え

```toml
# Cargo.toml
[package]
name = "(app_name)"
version = "0.1.0"
authors = [""]
edition = "2018"

[dependencies]

[profile.dev]
opt-level = 2
lto = true
panic = "abort"

[profile.release]
opt-level = 2
lto = true
panic = "abort"

[lib]
name = "(app_name)"
crate-type = ["staticlib"]
```

3. `cargo clean` しておく

```bash
$ cd apps/xxx
$ cargo clean
```

4. Makefileに追加

```Makefile
# xxx.hrbの部分を追加
$(IMG) : $(OUTPUT_DIR)/ipl.bin $(OUTPUT_DIR)/haribote.sys $(OUTPUT_DIR)/lines.hrb $(OUTPUT_DIR)/xxx.hrb $(OUTPUT_DIR)/xxx.hrb Makefile
	mformat -f 1440 -C -B $< -i $@ ::
	mcopy $(OUTPUT_DIR)/haribote.sys -i $@ ::
	mcopy $(OUTPUT_DIR)/lines.hrb -i $@ ::
	mcopy $(OUTPUT_DIR)/xxx.hrb -i $@ ::
```
