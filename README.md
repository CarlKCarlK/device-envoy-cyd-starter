# Device Envoy CYD Snake starter

A real downstream-style Device Envoy application: one portable Snake game and
UI runs on a classic ESP32 Cheap Yellow Display and in a browser CYD simulator.

## Run the browser demo

From this repository, build and serve the demo with:

```sh
just demo-wasm
```

Open <http://127.0.0.1:8092/>. Press Ctrl-C to stop the server.

## Run on a CYD

Connect a factory-wired classic ESP32 CYD over USB, then build, flash, and run
the demo with:

```sh
just demo-cyd
```

TODO0 This development repository intentionally uses local Device Envoy path
dependencies while the application exercises the evolving APIs. Replace every
local path with released crate versions and regenerate `Cargo.lock` before the
starter is released.

## What it demonstrates

- calibrated, orientation-correct `CydTouch` input;
- persistent touch calibration as device configuration;
- a separately persisted application high score;
- the same game/UI code on ESP32 and WASM;
- a compile-time RGB565 TGA background;
- layered program-drawn controls and graphics;
- incremental low-memory updates without tiling or a full-screen framebuffer.

The static background is streamed directly. Normal ticks restore only the old
tail cell from a zero-copy bitmap crop, draw the new snake state and refresh the
overlaid controls. The largest buffered composition is the 180×90 modal, so
`FRAME_PIXEL_COUNT` is 16,200 pixels (32,400 bytes). A 320×240 RGB565 frame
would consume 153,600 bytes.

## Repository layout

```text
src/game.rs          fixed-capacity Snake rules
src/app.rs           portable CYD input, rendering, loop, and persistence use
src/persistence.rs   typed high-score data
src/main.rs          classic CYD, factory two-SPI construction
examples/one_spi.rs  alternate physically shared-SPI construction
wasm/                thin cyd_web launcher and canonical browser shell
assets/              compile-time 320×240 TGA background
```

## Toolchain

Install Rust 1.93 or newer. Then install the Xtensa ESP Rust toolchain:

```sh
cargo install espup
espup install
```

`just demo-cyd` activates the installed ESP environment automatically. Before
running ESP Cargo commands directly, activate it in your current shell:

```sh
source "$HOME/export-esp.sh"
```

Install the flashing tool:

```sh
cargo install espflash
```

The browser demo additionally requires `wasm32-unknown-unknown` and
`wasm-pack`. The adjacent local Device Envoy checkout is currently expected at
`../device-envoy`.

TODO0 Replace this local-checkout instruction with crates.io dependency setup.

## Classic CYD: default two-SPI build

The default target is the original ESP32 CYD in landscape orientation. To build
without flashing:

```sh
cargo +esp build --release --target xtensa-esp32-none-elf \
    --no-default-features --features esp32 -Zbuild-std=core,alloc
```

To build, flash, and run:

```sh
just demo-cyd
```

The constructor uses the common factory wiring:

| Function | GPIO |
| --- | ---: |
| Display SCK / MOSI / MISO | 14 / 13 / 12 |
| Display CS / DC / reset / backlight | 15 / 2 / 4 / 21 |
| Touch SCK / MOSI / MISO | 25 / 32 / 39 |
| Touch CS / IRQ | 33 / 36 |
| BOOT/recalibration | 0 |

TODO0 Verify this exact table on the intended classic CYD hardware revision
before release; clone boards sometimes differ.

On first boot Device Envoy runs touch calibration and saves it in the first
flash block. Application code receives only calibrated, landscape-oriented
events through `CydTouch`; it performs no coordinate remapping. Press BOOT to
clear calibration and restart. The second flash block stores `HighScore`, and
is written only when a completed game's score exceeds the stored value.

## One-SPI variant

```sh
cargo build --release --example one_spi --target xtensa-esp32-none-elf
```

`examples/one_spi.rs` constructs `CydEspOneSpi` and shares all application
logic with the default program. Selecting this example does **not** change the
factory wiring. The display and touch controller must actually be modified or
wired to share GPIO14/GPIO13/GPIO12, with their independent CS pins retained.

TODO0 Document and photograph the validated physical one-SPI modification.

## Browser/WASM

```sh
just demo-wasm
```

The recipe builds the WASM package, prints the browser URL, and starts the local
server. The launcher constructs `CydWasm`; the canonical Device Envoy browser
shell forwards pointer input into its touch source. The portable application
still reads only `CydTouch`. `FlashBlockWasm` stores the high score in browser
local storage under
`device-envoy/cyd-snake/high-score`.

TODO0 Replace the copied local `cyd-simulator.js` and `.css` development assets
with the final supported downstream distribution mechanism, if Device Envoy
adds one before release.

## Verification

```sh
just check-all
```

TODO0 Add browser interaction tests for direction pressed states, pause/resume,
game over/play again, and high-score persistence before release.
todo0 background should be fanicer
todo0 seems slow to respond
does high score persist?
does reset button work?
