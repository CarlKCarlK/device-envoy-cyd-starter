use core::fmt;

use device_envoy_core::{
    cyd::display::Orientation,
    flash_block,
    wasm::{self, FlashBlockWasm, cyd_web},
};
use device_envoy_cyd_starter::app::{self, BACKGROUND_COLOR, FOREGROUND_COLOR};
use embedded_graphics::mono_font::ascii::FONT_6X10;
use wasm_bindgen::prelude::wasm_bindgen;

const WEB_APP: cyd_web::Config = cyd_web::Config::new(
    "device-envoy/cyd-snake",
    Orientation::Landscape,
    BACKGROUND_COLOR,
    FOREGROUND_COLOR,
    &FONT_6X10,
);

const PAGE_INFO: cyd_web::PageInfo = cyd_web::PageInfo::new(
    "Device Envoy Snake",
    "Play the same stylus-driven Snake application that runs on an ESP32 CYD.",
    "A low-memory Device Envoy demo with shared game, UI, touch, bitmap, and persistence code.",
    "Use the on-screen direction and pause buttons. The browser stores your high score locally.",
    "https://github.com/CarlKCarlK/device-envoy-cyd-starter/blob/main/src/app.rs",
);

#[wasm_bindgen]
pub fn start(canvas_id: &str) -> Result<cyd_web::Handle, wasm_bindgen::JsValue> {
    cyd_web::start(canvas_id, WEB_APP, PAGE_INFO, inner_main)
}

async fn inner_main(capabilities: cyd_web::Capabilities) -> Result<cyd_web::Command, Error> {
    let mut cyd = capabilities.cyd;
    let mut button = capabilities.button;
    let mut high_score_storage = FlashBlockWasm::new("device-envoy/cyd-snake/high-score")?;

    match app::run(&mut cyd, &mut button, &mut high_score_storage).await? {
        app::Exit::CalibrationRequested => Ok(cyd_web::Command::CalibrationNotNeeded),
    }
}

#[derive(derive_more::From)]
enum Error {
    Wasm(wasm::Error),
    App(app::Error<core::convert::Infallible, flash_block::Error<wasm::Error>>),
}

impl fmt::Debug for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wasm(source) => formatter.debug_tuple("Wasm").field(source).finish(),
            Self::App(source) => formatter.debug_tuple("App").field(source).finish(),
        }
    }
}
