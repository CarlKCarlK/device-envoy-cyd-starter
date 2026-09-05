#![no_std]
#![no_main]

use core::{convert::Infallible, fmt};

use device_envoy_core::{
    cyd::{Cyd as _, CydDisplay, display::CydFrame},
    flash_block::FlashBlock as _,
};
use device_envoy_cyd_starter::app::{
    self, BACKGROUND_COLOR, FOREGROUND_COLOR, FRAME_PIXEL_COUNT, ORIENTATION,
};
use device_envoy_esp::{
    Error as DeviceEnvoyError,
    button::PressedTo,
    button_watch,
    cyd::{self, CydEsp, CydStaticEsp, DEFAULT_DISPLAY_SPI_HZ, DEFAULT_FONT},
    flash_block::FlashBlockEsp,
    init_and_start,
};
use embassy_executor::Spawner;
use esp_backtrace as _;
use log::info;

esp_bootloader_esp_idf::esp_app_desc!();

// The app can poll BOOT while the CYD constructor also uses it for calibration.
button_watch! {
    ButtonWatch {
        pin: GPIO0,
    }
}

// TODO000 Should the one-SPI versus two-SPI choice be a feature instead of a
// separate example?

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(never) => match never {},
        Err(error) => panic!("{error:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> Result<Infallible, Error> {
    init_and_start!(p);
    esp_println::logger::init_logger(log::LevelFilter::Info);
    info!("Starting Device Envoy Paint Book on the classic two-SPI CYD");

    let [mut calibration_flash_block, mut page_flash_block] =
        FlashBlockEsp::new_array::<2>(p.FLASH)?;
    let button_watch = ButtonWatch::new(p.GPIO0, PressedTo::Ground, spawner).await?;

    static CYD_STATIC: CydStaticEsp<FRAME_PIXEL_COUNT> = CydEsp::new_static();
    let mut cyd = CydEsp::new(
        &CYD_STATIC,
        // Display SPI and pins (factory classic CYD wiring):
        p.SPI2,
        p.GPIO14,
        p.GPIO13,
        p.GPIO12,
        p.GPIO15,
        p.GPIO2,
        p.GPIO4,
        p.GPIO21,
        DEFAULT_DISPLAY_SPI_HZ,
        // Presentation:
        ORIENTATION,
        BACKGROUND_COLOR,
        FOREGROUND_COLOR,
        &DEFAULT_FONT,
        // Touch SPI and pins (factory classic CYD wiring):
        p.SPI3,
        p.GPIO25,
        p.GPIO32,
        p.GPIO39,
        p.GPIO33,
        p.GPIO36,
        // Calibration storage and recalibration button:
        &mut calibration_flash_block,
        &mut *button_watch,
    )
    .await?;
    info!("CYD initialized; touch coordinates are calibrated and landscape-oriented");

    // TODO000 This is a little unusual. Would it be clearer to return an
    // action from the application and process it here?
    // todo000 is app the right name?
    match app::run(&mut cyd, &*button_watch, &mut page_flash_block).await? {
        app::Exit::CalibrationRequested => {
            calibration_flash_block.clear()?;
            // TODO000 Is displaying this message on the full frame the right
            // user experience for recalibration?
            let mut frame = cyd.display().full_frame_mut();
            frame.clear().write_text("Recalibrating after restart");
            frame.flush()?;
            info!("Cleared touch calibration; the selected paint page remains stored");
            esp_hal::system::software_reset();
        }
    }
}

// Derived Debug reads these payloads at runtime, but dead_code analysis ignores
// derived implementations under -D warnings.
// TODO000 Explain why this application-level error type is needed here.
#[derive(derive_more::From)]
enum Error {
    DeviceEnvoy(DeviceEnvoyError),
    Cyd(cyd::Error),
    App(app::Error<cyd::Error, DeviceEnvoyError>),
}

// TODO00 Is this manual Debug implementation the nicest approach?
impl fmt::Debug for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeviceEnvoy(source) => {
                formatter.debug_tuple("DeviceEnvoy").field(source).finish()
            }
            Self::Cyd(source) => formatter.debug_tuple("Cyd").field(source).finish(),
            Self::App(source) => formatter.debug_tuple("App").field(source).finish(),
        }
    }
}
