todo0000 This is a pre-release version

# Device Envoy CYD Paint Book starter

A small Device Envoy application for the classic ESP32 Cheap Yellow Display.
The same application code runs on real hardware and in the browser CYD
simulator.

Start a stroke on a color and drag it into the picture. The color is sampled
from the framebuffer at the start of the stroke, so painted areas intentionally
become new color sources. There is no separate fixed palette. Touch the folded
corner to switch between the crab-beach, dog-walk, and cave-art pages. The
selected page survives a restart.

## Run it

In a browser:

```sh
just demo-wasm
```

The current `main` branch is also published at
<https://carlkcarlk.github.io/device-envoy-cyd-starter/>. The GitHub Pages
workflow builds and replaces that single site on every push to `main`. Select
**GitHub Actions** as the repository's Pages source once under **Settings →
Pages**.

On a factory-wired classic CYD:

```sh
just demo-cyd
```

The first hardware boot walks through touch calibration and saves it. Press the
BOOT button at any time to discard that calibration and repeat it after the
device restarts.

## What the starter demonstrates

- calibrated, orientation-correct touch input;
- a full-screen RGB565 frame that can be read, drawn into, and flushed;
- compile-time TGA-to-RGB565 bitmap assets;
- typed flash storage for the current page;
- one portable application shared by ESP32 and WASM;
- explicit hardware construction for the factory CYD wiring.

The framebuffer itself is the painting. On touch-down, the app reads that
pixel's color; touch movement draws a thick line with the carried color. This
deliberately makes the canvas both the rendered image and the color source for
future strokes. It keeps the portable application in
[`src/app.rs`](src/app.rs) concise: there is no separate palette, scene graph,
modal state, dirty-region renderer, or game engine.

## Repository layout

```text
src/app.rs   shared state, touch loop, bitmap pages, and drawing
src/main.rs  factory classic-CYD hardware construction
wasm/        thin browser launcher and simulator shell
assets/      320x240 TGA pages and editable PNG sources
```

TODO0 Explain whether this file split is the clearest structure for a starter.

## Hardware wiring

The default binary uses the common factory two-SPI wiring:

| Function | GPIO |
| --- | ---: |
| Display SCK / MOSI / MISO | 14 / 13 / 12 |
| Display CS / DC / reset / backlight | 15 / 2 / 4 / 21 |
| Touch SCK / MOSI / MISO | 25 / 32 / 39 |
| Touch CS / IRQ | 33 / 36 |
| BOOT/recalibration | 0 |

TODO0 Verify this exact table on the intended classic CYD hardware revision
before release; clone boards sometimes differ.

## Toolchain

Install Rust 1.93 or newer, the Xtensa ESP Rust toolchain, and the flashing
tool:

```sh
cargo install espup
espup install
cargo install espflash
```

The browser demo additionally requires `wasm32-unknown-unknown` and
`wasm-pack`. This development checkout currently expects Device Envoy at
`../device-envoy`.

TODO0 Replace the local Device Envoy paths with released crate versions and
regenerate `Cargo.lock` before release.

## Verification

```sh
just check-all
```

TODO0 Replace the copied local `cyd-simulator.js` and `.css` development assets
with the final supported downstream distribution mechanism, if Device Envoy
adds one before release.
