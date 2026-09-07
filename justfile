[unix]
set shell := ["sh", "-cu"]

[windows]
set shell := ["powershell.exe", "-NoLogo", "-Command"]

_esp_args := "--target xtensa-esp32-none-elf --no-default-features --features esp32 --release -Zbuild-std=core,alloc"

[unix]
_esp_environment := '. "$HOME/export-esp.sh";'

[windows]
_esp_environment := '& "$env:USERPROFILE\export-esp.ps1";'

# Check the shared application with the host Rust toolchain.
check-host:
    cargo test --lib --no-default-features

# Check the ESP32 application.
check-esp:
    {{_esp_environment}} cargo +esp check --bin device-envoy-cyd-starter {{_esp_args}}

# Check the browser application.
check-wasm:
    cargo check --package device-envoy-cyd-starter-wasm --target wasm32-unknown-unknown

# Check the shared, ESP32, and browser applications.
check-all: check-host check-esp check-wasm

# Build the ESP32 application.
build-esp:
    {{_esp_environment}} cargo +esp build --bin device-envoy-cyd-starter {{_esp_args}}

# Build the browser application.
build-wasm:
    wasm-pack build wasm --target web --out-dir pkg

# Build the ESP32 and browser applications.
build-all: build-esp build-wasm

# Build, flash, and monitor the ESP32 application.
run-esp:
    {{_esp_environment}} cargo +esp run --bin device-envoy-cyd-starter {{_esp_args}}

# Build and serve the browser application.
run-wasm: build-wasm
    @echo "Open http://127.0.0.1:8092/ in your browser."
    miniserve --interfaces 127.0.0.1 --port 8092 --index index.html wasm
