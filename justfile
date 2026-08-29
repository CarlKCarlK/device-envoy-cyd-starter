set shell := ["bash", "-cu"]

check-host:
    cargo test --lib --no-default-features --target x86_64-unknown-linux-gnu

check-esp:
    cargo +esp check --bin device-envoy-cyd-starter --target xtensa-esp32-none-elf --features esp32 --release -Zbuild-std=core,alloc

check-one-spi:
    cargo +esp check --example one_spi --target xtensa-esp32-none-elf --features esp32 --release -Zbuild-std=core,alloc

build-wasm:
    wasm-pack build wasm --target web --out-dir pkg

serve-wasm:
    python3 -m http.server 8092 --directory wasm

check-all: check-host check-esp check-one-spi build-wasm
