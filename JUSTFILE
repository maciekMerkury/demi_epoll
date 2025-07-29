install_prefix := "/usr/local"

default:
    just --list

rust_bindings:
    bindgen c/wrapper.h -o src/wrappers/raw.rs

c_header: build
    cbindgen src/bindings/mod.rs -c cbindgen.toml -o c/dpoll.h

build: rust_bindings
    cargo build --release

install: 
    sudo mkdir -p {{install_prefix}}/include/demi_epoll {{install_prefix}}/lib
    sudo cp c/dpoll.h {{install_prefix}}/include/demi_epoll/
    sudo cp target/release/libdemi_epoll.so {{install_prefix}}/lib/

all: build install
