use core::fmt;

use device_envoy_core::{
    flash_block,
    wasm::{self, FlashBlockWasm, cyd_web},
};
use device_envoy_cyd_starter::app::{APP_FONT, BACKGROUND_COLOR, FOREGROUND_COLOR, ORIENTATION};
use wasm_bindgen::prelude::wasm_bindgen;

const WEB_APP: cyd_web::Config = cyd_web::Config::new(
    "device-envoy/cyd-paint-book",
    ORIENTATION,
    BACKGROUND_COLOR,
    FOREGROUND_COLOR,
    &APP_FONT,
);

const PAGE_INFO: cyd_web::PageInfo = cyd_web::PageInfo::new(
    "Device Envoy Paint Book",
    "Drag colors into pictures in the same application that runs on an ESP32 CYD.",
    "A concise Device Envoy starter with shared touch, framebuffer, bitmap, and persistence code.",
    "Start each stroke on the color you want to carry. Tap the folded corner for a fresh page.",
    "https://github.com/CarlKCarlK/device-envoy-cyd-starter/blob/main/src/main.rs",
);

#[wasm_bindgen]
pub fn start(canvas_id: &str) -> Result<cyd_web::Handle, wasm_bindgen::JsValue> {
    cyd_web::start(canvas_id, WEB_APP, PAGE_INFO, inner_main)
}

async fn inner_main(capabilities: cyd_web::Capabilities) -> Result<cyd_web::Command, Error> {
    let mut cyd = capabilities.cyd;
    let button = capabilities.button;
    let mut page_flash_block = FlashBlockWasm::new("device-envoy/cyd-paint-book/page")?;

    let exit = device_envoy_cyd_starter::app::run::<_, _, _, Error>(
        &mut cyd,
        &button,
        &mut page_flash_block,
    )
    .await?;
    match exit {
        device_envoy_cyd_starter::app::Exit::CalibrationRequested => {
            Ok(cyd_web::Command::CalibrationNotNeeded)
        }
    }
}

#[derive(derive_more::From)]
enum Error {
    Wasm(wasm::Error),
    Cyd(core::convert::Infallible),
    Storage(flash_block::Error<wasm::Error>),
}

impl fmt::Debug for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wasm(source) => formatter.debug_tuple("Wasm").field(source).finish(),
            Self::Cyd(source) => formatter.debug_tuple("Cyd").field(source).finish(),
            Self::Storage(source) => formatter.debug_tuple("Storage").field(source).finish(),
        }
    }
}
