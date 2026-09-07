stil# Device Envoy CYD starter

> **Pre-release:** This starter is still being prepared for its first release.

The Cheap Yellow Display (CYD) is an inexpensive ESP32 development board with
a built-in color touchscreen. This repository is a ready-to-run Rust starter
for the classic `ESP32-2432S028R` CYD.

[Device Envoy](https://crates.io/crates/device-envoy-core) is the Rust library
that connects the portable application to the CYD's display, touchscreen,
button, and storage—or to browser simulations of those devices.

This starter project initially runs a touchscreen paint-book application. First
run it unchanged in your browser or on the physical board, then replace the
portable application in [`src/app.rs`](src/app.rs) with whatever you want to
build. The hardware setup in [`src/main.rs`](src/main.rs) can stay unchanged.

<!-- TODO000 Add a short GIF showing color pickup, painting, and a page turn. -->

## Try it in your browser

**[Open the live CYD paint book](https://carlkcarlk.github.io/device-envoy-cyd-starter/)**

The browser simulator runs the same application code as the physical board.
Click or drag on its touchscreen to use it. No hardware or software installation
is required for the live version.

To build and test the web version locally on your own computer, follow
[Set up your computer](#set-up-your-computer) and [Get the source](#get-the-source),
then run:

```sh
just run-wasm
```

Open <http://127.0.0.1:8092/> in your browser. Press
<kbd>Ctrl</kbd>+<kbd>C</kbd> in the terminal to stop the local server.

## What the paint book does

Start a stroke on a color and drag it into the picture. The paint brush picks
up the color under your first touch, so anything you have already painted can
become a new color source. There is no separate palette.

Touch the folded corner to switch between the crab-beach, dog-walk, and cave-art
pages. The selected page is saved and survives a restart. On the physical CYD,
the first boot also guides you through touchscreen calibration.

The paint book is deliberately small enough to replace. It demonstrates:

- drawing into and reading from a full-screen frame;
- calibrated, orientation-correct touch input;
- bitmap assets compiled into the program;
- saving a typed value in flash storage; and
- sharing one portable application between ESP32 hardware and WebAssembly.

## Get the supported CYD

This starter supports the classic **2.8-inch `ESP32-2432S028R`** with:

- the original ESP32 chip;
- an ILI9341 display controller; and
- XPT2046 resistive touch.

The panel is physically 240×320 pixels; this starter uses it in 320×240
landscape orientation.

Before ordering, check both the listing and the model printed on the back of the
board. The original single-Micro-USB revision is the safest choice.

These are ordinary product links for reference. They are not affiliate links,
and neither this project nor its authors sell these products.

- [2.8-inch ESP32-2432S028R board used for this starter](https://www.amazon.com/dp/B0BVFXR313)
- [Optional acrylic case for the 2.8-inch ESP32-2432S028R](https://www.amazon.com/dp/B0D9JQ6GRC?th=1)

Amazon listings and the hardware supplied under them can change. Confirm the
model number, display controller, and touch controller before ordering.

> **Not currently supported:** two-port `CYD2USB` boards, ESP32-S3 or ESP32-C3
> variants, ST7789 displays, capacitive-touch or no-touch models, `JC2432W328`
> boards, and other screen sizes.

<!-- TODO000 Add front-and-back photos of the supported board, with the printed
ESP32-2432S028R model number highlighted. -->

## Set up your computer

The commands in this README support Linux, macOS, and Windows PowerShell. On
Windows, run them in PowerShell rather than Command Prompt.

Install Rust 1.93 or newer [using rustup](https://rustup.rs/) if `cargo` and
`rustup` are not already available. You will also need Git and a data-capable
USB cable for the physical board.

### Install the command runner

[`just`](https://github.com/casey/just#installation) gives this repository short,
consistent development commands. Install it with Cargo:

```sh
cargo install just
```

`just` is not part of Rust or Device Envoy; it runs the Cargo, flashing, and web
server commands defined in this repository's `justfile`.

### Install browser tools

To build and serve the browser simulator locally, install `wasm-pack`, the WASM
Rust target, and the cross-platform Rust web server
[`miniserve`](https://github.com/svenstaro/miniserve):

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --locked --version 0.15.0
cargo install --locked miniserve
```

### Install ESP32 tools

The classic CYD uses the Xtensa-based original ESP32, which needs Espressif's
Rust toolchain and a flashing tool:

```sh
cargo install espup
espup install
cargo install espflash
```

The `just` ESP commands automatically load the environment file created by
`espup`. If another terminal command needs the Xtensa tools directly, load it
with the command for your operating system.

On Linux or macOS:

```sh
source "$HOME/export-esp.sh"
```

On Windows PowerShell:

```powershell
& "$env:USERPROFILE\export-esp.ps1"
```

## Get the source

```sh
git clone https://github.com/CarlKCarlK/device-envoy-cyd-starter.git
cd device-envoy-cyd-starter
```

## Run it on the CYD

Connect the board with a data-capable Micro-USB cable, then run:

```sh
just run-esp
```

This builds the release binary, flashes it to the board, and opens the serial
monitor. Follow the instructions on the CYD to calibrate its touchscreen the
first time it starts.

Press the **BOOT** button on the back of the board at any time to request a new
touch calibration. The saved calibration is cleared and calibration runs again
after the board restarts. Your selected paint-book page is kept.

If the flasher cannot open the serial port on Linux, check your USB device
permissions. If no serial port appears at all, first try another cable; many USB
cables supply power but do not carry data.

## Make it your own

Once the paint book works, edit [`src/app.rs`](src/app.rs). It contains the
portable application state, touch loop, images, and drawing behavior. Keep
[`src/main.rs`](src/main.rs) unchanged at first; it constructs the display,
touchscreen, BOOT button, and flash storage for the factory-wired CYD.

Use the browser for a quick development loop:

```sh
just run-wasm
```

Then test the same application on the board:

```sh
just run-esp
```

You can replace the paint loop, page state, and assets with your own game,
instrument display, controller, or other touchscreen application. Remove the
example images under `assets/` when your application no longer uses them.

<!-- TODO000 Add a small diagram showing src/app.rs feeding both the browser
simulator and the physical CYD. -->

<!-- todo000 test on windows. -->
<!-- todo000 read this carefully -->

## Commands you need

| Command | What it does |
| --- | --- |
| `just run-wasm` | Builds the browser application and starts a local server |
| `just run-esp` | Builds, flashes, and monitors the physical CYD |
| `just check-wasm` | Checks the browser application |
| `just check-esp` | Checks the ESP32 application |
| `just check-all` | Checks the shared, browser, and ESP32 applications |
| `just build-wasm` | Builds the browser application |
| `just build-esp` | Builds the ESP32 application |
| `just build-all` | Builds the browser and ESP32 applications |

Run the full local verification before sharing changes:

```sh
just check-all
```

## Repository layout

```text
src/app.rs   portable application state, touch loop, images, and drawing
src/lib.rs   library entry point that exposes the portable application
src/main.rs  factory classic-CYD hardware construction
wasm/        browser launcher and CYD simulator shell
assets/      320×240 TGA pages and editable PNG sources
```

The browser launcher and ESP32 binary both call the application in `src/app.rs`.
Platform-specific setup stays outside that file, making it the natural place to
start building your own application.

## Acknowledgments

Thank you to:

- Jeff B. for inspiring the idea of a finger-paint simulator.
- John K. for the idea behind the Cave Art picture.
