set shell := ["bash", "-cu"]

check-host:
    cargo test --lib --no-default-features --target x86_64-unknown-linux-gnu

check-esp:
    cargo +esp check --bin device-envoy-cyd-starter --target xtensa-esp32-none-elf --features esp32 --release -Zbuild-std=core,alloc

demo-cyd:
    #!/usr/bin/env bash
    set -euo pipefail
    esp_export="${HOME}/export-esp.sh"
    if [[ ! -f "${esp_export}" ]]; then
        echo "Missing ${esp_export}. Install the Xtensa toolchain with 'cargo install espup' and 'espup install'." >&2
        exit 1
    fi
    source "${esp_export}"
    cargo +esp run --release --target xtensa-esp32-none-elf --no-default-features --features esp32 -Zbuild-std=core,alloc

build-wasm:
    wasm-pack build wasm --target web --out-dir pkg

demo-wasm: build-wasm
    @echo "Open http://127.0.0.1:8092/ in your browser."
    python3 -m http.server 8092 --directory wasm

serve-wasm:
    python3 -m http.server 8092 --directory wasm

check-all: check-host check-esp build-wasm
