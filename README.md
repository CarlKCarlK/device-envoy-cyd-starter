todo0000 This is a pre-release version

# Device Envoy CYD Paint Book starter

Classic CYD (`ESP32-2432S028R`, 2.8-inch 240×320 ILI9341 display, and XPT2046
resistive touch) paint-book starter for Device Envoy. The same application code
runs on real hardware and in the browser CYD simulator.

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

## Hardware compatibility and purchasing

This starter targets the classic CYD described above, using an original ESP32
rather than an ESP32-S3 or ESP32-C3. Before buying or flashing a board, check
that its listing and the model printed on its back identify an
`ESP32-2432S028R` with an ILI9341 display and XPT2046 resistive touch. The
original single-micro-USB revision is the safest choice.

The two-port `CYD2USB`, ST7789 display revisions, capacitive-touch and no-touch
models, `JC2432W328` boards, other screen sizes, and boards using another ESP32
variant are not supported by this starter as currently configured.

The items ordered for this starter are:

- [2.8-inch ESP32-2432S028R board](https://www.amazon.com/dp/B0BVFXR313) — its
  listing identifies an ILI9341 display and resistive touch;
- [optional acrylic case](https://www.amazon.com/dp/B0D9JQ6GRC?th=1) — listed
  for the 2.8-inch ESP32-2432S028R.

Amazon listings and fulfilled hardware can change, so confirm those identifying
details before ordering.

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
src/lib.rs   library entry point that exposes the portable application
src/app.rs   shared state, touch loop, bitmap pages, and drawing
src/main.rs  factory classic-CYD hardware construction
wasm/        thin browser launcher and simulator shell
assets/      320x240 TGA pages and editable PNG sources
```

This keeps the application in one platform-neutral module while the ESP32 and
browser launchers each contain only their platform setup. `src/lib.rs` makes
that shared application available to both launchers.

## Hardware wiring

The default binary targets the classic ESP32-2432S028R and uses its factory
two-SPI wiring:

| Function | GPIO |
| --- | ---: |
| Display SCK / MOSI / MISO | 14 / 13 / 12 |
| Display CS / DC / backlight | 15 / 2 / 21 |
| Display reset | Board reset / EN (no dedicated GPIO) |
| Touch SCK / MOSI / MISO | 25 / 32 / 39 |
| Touch CS / IRQ | 33 / 36 |
| BOOT/recalibration | 0 |

This mapping follows the
[ESP32-2432S028R schematic](https://www.ardboard.com/downloads/ESP32_2432S028R_Schematics.pdf),
including the display reset connection to the board reset/EN net. Other
CYD-branded boards and clone revisions may use different wiring.

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

TODO00 add thanks for Jeff B. and John
