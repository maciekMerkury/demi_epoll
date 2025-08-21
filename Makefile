INSTALL_PREFIX?="/usr/local"

lib_path:=$(INSTALL_PREFIX)/lib

include_path:=$(INSTALL_PREFIX)/include/demi_epoll

default: build install

.PHONY: install build default

rust_bindings: c/wrapper.h
	bindgen c/wrapper.h -o src/wrappers/raw.rs

update_c_header: src/bindings/mod.rs
	cbindgen src/bindings/mod.rs -c cbindgen.toml -o c/updated_dpoll.h

build:
	cargo build --release

install:
	mkdir -p $(lib_path) $(include_path)
	cp c/dpoll.h $(include_path)/
	cp target/release/libdemi_epoll.so $(lib_path)/
