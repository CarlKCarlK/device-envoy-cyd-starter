//! Paint Book on a deliberately shared display/touch SPI bus.
//!
//! This is not a software switch for an untouched classic CYD. The factory
//! board routes the display and XPT2046 touch controller to different signal
//! pins. Use this example only after wiring/modifying the board so both devices
//! genuinely share SCK, MOSI, and MISO while retaining independent CS pins.

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
    cyd::{self, CydEspOneSpi, CydStaticEsp, DEFAULT_DISPLAY_SPI_HZ, DEFAULT_FONT},
    flash_block::FlashBlockEsp,
    init_and_start,
};
use embassy_executor::Spawner;
use esp_backtrace as _;
use log::info;

esp_bootloader_esp_idf::esp_app_desc!();

button_watch! {
    ButtonWatch {
        pin: GPIO0,
    }
}

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
    info!("Starting Device Envoy Paint Book on a physically shared SPI bus");

    let [mut calibration_flash_block, mut page_flash_block] =
        FlashBlockEsp::new_array::<2>(p.FLASH)?;
    let button_watch = ButtonWatch::new(p.GPIO0, PressedTo::Ground, spawner).await?;

    static CYD_STATIC: CydStaticEsp<FRAME_PIXEL_COUNT> = CydEspOneSpi::new_static();
    let mut cyd = CydEspOneSpi::new(
        &CYD_STATIC,
        // Shared display/touch SPI signals after the required wiring change:
        p.SPI2,
        p.GPIO14,
        p.GPIO13,
        p.GPIO12,
        // Display-only pins:
        p.GPIO15,
        p.GPIO2,
        p.GPIO4,
        p.GPIO21,
        DEFAULT_DISPLAY_SPI_HZ,
        // Touch-only pins:
        p.GPIO33,
        p.GPIO36,
        // Presentation:
        ORIENTATION,
        BACKGROUND_COLOR,
        FOREGROUND_COLOR,
        &DEFAULT_FONT,
        // Calibration storage and recalibration button:
        &mut calibration_flash_block,
        &mut *button_watch,
    )
    .await?;

    match app::run(&mut cyd, &*button_watch, &mut page_flash_block).await? {
        app::Exit::CalibrationRequested => {
            calibration_flash_block.clear()?;
            let mut frame = cyd.display().full_frame_mut();
            frame.clear().write_text("Recalibrating after restart");
            frame.flush()?;
            esp_hal::system::software_reset();
        }
    }
}

// Derived Debug reads these payloads at runtime, but dead_code analysis ignores
// derived implementations under -D warnings.
#[derive(derive_more::From)]
enum Error {
    DeviceEnvoy(DeviceEnvoyError),
    Cyd(cyd::Error),
    App(app::Error<cyd::Error, DeviceEnvoyError>),
}

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
