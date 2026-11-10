[unix]
set shell := ["sh", "-cu"]

[windows]
set shell := ["powershell.exe", "-NoLogo", "-Command"]

# Quote the comma-containing value so PowerShell passes it as one Cargo argument.
_esp_args := '--target xtensa-esp32-none-elf --no-default-features --features esp32 --release "-Zbuild-std=core,alloc"'

[unix]
_esp_environment := '. "$HOME/export-esp.sh";'

[windows]
_esp_environment := '& "$env:USERPROFILE\export-esp.ps1";'

# Browser shell files that come unmodified from device-envoy-core's www directory.
_shell_files := "cyd-simulator.js cyd-simulator.css case.png desk.jpg"

[unix]
_require_miniserve := "command -v miniserve >/dev/null || { echo 'miniserve is required. Install it with: cargo install --locked miniserve' >&2; exit 1; }"

[windows]
_require_miniserve := "if (-not (Get-Command miniserve -ErrorAction SilentlyContinue)) { Write-Error 'miniserve is required. Install it with: cargo install --locked miniserve'; exit 1 }"

# Check the shared application with the host Rust toolchain.
check-host:
    cargo test --lib --no-default-features

# Check the ESP32 application.
check-esp:
    {{ _esp_environment }} cargo +esp check --bin device-envoy-cyd-starter {{ _esp_args }}

# Check the browser application.
check-wasm:
    cargo check --package device-envoy-cyd-starter-wasm --target wasm32-unknown-unknown

# Check that the browser shell in wasm/www matches the device-envoy-core crate.
[unix]
check-shell:
    #!/bin/sh
    set -eu
    core_dir=$(cargo metadata --format-version 1 --locked | grep -o '"manifest_path":"[^"]*/device-envoy-core\(-[0-9][^/"]*\)\{0,1\}/Cargo.toml"' | head -n 1 | sed 's/^"manifest_path":"//; s#/Cargo.toml"$##')
    test -n "$core_dir" || { echo "device-envoy-core not found in cargo metadata" >&2; exit 1; }
    for file in {{ _shell_files }}; do
        cmp -s "$core_dir/www/$file" "wasm/www/$file" || { echo "wasm/www/$file differs from $core_dir/www/$file; run just sync-shell" >&2; exit 1; }
    done
    echo "wasm/www shell matches $core_dir/www"

[windows]
check-shell:
    $core = (cargo metadata --format-version 1 --locked | ConvertFrom-Json).packages | Where-Object name -eq 'device-envoy-core' | Select-Object -First 1; \
    $coreDir = Split-Path $core.manifest_path; \
    foreach ($file in '{{ _shell_files }}'.Split(' ')) { \
        if ((Get-FileHash "$coreDir/www/$file").Hash -ne (Get-FileHash "wasm/www/$file").Hash) { Write-Error "wasm/www/$file differs from $coreDir/www/$file; run just sync-shell"; exit 1 } \
    }; \
    Write-Output "wasm/www shell matches $coreDir/www"

# Copy the browser shell from the device-envoy-core crate into wasm/www.
[unix]
sync-shell:
    #!/bin/sh
    set -eu
    core_dir=$(cargo metadata --format-version 1 --locked | grep -o '"manifest_path":"[^"]*/device-envoy-core\(-[0-9][^/"]*\)\{0,1\}/Cargo.toml"' | head -n 1 | sed 's/^"manifest_path":"//; s#/Cargo.toml"$##')
    test -n "$core_dir" || { echo "device-envoy-core not found in cargo metadata" >&2; exit 1; }
    for file in {{ _shell_files }}; do cp "$core_dir/www/$file" "wasm/www/$file"; done
    echo "copied the shell from $core_dir/www"

[windows]
sync-shell:
    $core = (cargo metadata --format-version 1 --locked | ConvertFrom-Json).packages | Where-Object name -eq 'device-envoy-core' | Select-Object -First 1; \
    $coreDir = Split-Path $core.manifest_path; \
    foreach ($file in '{{ _shell_files }}'.Split(' ')) { Copy-Item "$coreDir/www/$file" "wasm/www/$file" }; \
    Write-Output "copied the shell from $coreDir/www"

# Check the shared, ESP32, and browser applications, and the browser shell.
check-all: check-host check-esp check-wasm check-shell

# Build the ESP32 application.
build-esp:
    {{ _esp_environment }} cargo +esp build --bin device-envoy-cyd-starter {{ _esp_args }}

# Build the browser application.
build-wasm:
    wasm-pack build wasm --target web --out-dir pkg

# Build the ESP32 and browser applications.
build-all: build-esp build-wasm

# Make the CYD USB serial adapter available to WSL2.
attach-wsl busid="":
    sh scripts/attach-wsl.sh "{{ busid }}"

# Build, flash, and monitor the ESP32 application.
run-esp: _prepare-wsl-usb
    {{ _esp_environment }} cargo +esp run --bin device-envoy-cyd-starter {{ _esp_args }}

# Build and serve the browser application.
run-wasm: _check-miniserve build-wasm
    @echo "Open http://127.0.0.1:8092/ in your browser."
    miniserve --interfaces 127.0.0.1 --port 8092 --index index.html wasm

# Fail before building when the local web server is unavailable.
[private]
_check-miniserve:
    {{ _require_miniserve }}

# Attach the CYD automatically on WSL2 and do nothing on native operating systems.
[private]
_prepare-wsl-usb:
    sh scripts/attach-wsl.sh --if-wsl
