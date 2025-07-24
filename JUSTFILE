default:
    just --list

generate_bindings:
    bindgen c/wrapper.h -o src/wrappers/raw.rs

