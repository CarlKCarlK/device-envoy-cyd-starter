#![no_std]
#![no_main]

use core::convert::Infallible;

use device_envoy_cyd_starter::app::{self, BACKGROUND_COLOR, FONT, FOREGROUND_COLOR, ORIENTATION};
use device_envoy_esp::{
    Error as DeviceEnvoyError,
    button::PressedTo,
    button_watch,
    cyd::{self, CydEsp, CydStaticEsp, DEFAULT_DISPLAY_SPI_HZ, NoDisplayReset},
    flash_block::{FlashBlock, FlashBlockEsp},
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

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(never) => match never {},
        Err(Error::DeviceEnvoy(source)) => panic!("DeviceEnvoy({source:?})"),
        Err(Error::Cyd(source)) => panic!("Cyd({source:?})"),
    }
}

async fn inner_main(spawner: Spawner) -> Result<Infallible, Error> {
    init_and_start!(p);
    esp_println::logger::init_logger(log::LevelFilter::Info);
    info!("Starting Device Envoy Paint Book on the classic two-SPI CYD");

    let [mut calibration_flash_block, mut page_flash_block] =
        FlashBlockEsp::new_array::<2>(p.FLASH)?;
    let calibration_button_watch = ButtonWatch::new(p.GPIO0, PressedTo::Ground, spawner).await?;

    static CYD_STATIC: CydStaticEsp<{ CydEsp::SCREEN_PIXELS }> = CydEsp::new_static();
    // The factory CYD uses separate SPI resources for display and touch.
    // Device Envoy also offers CydEspOneSpi::new for custom shared-bus hardware.
    let mut cyd = CydEsp::new(
        &CYD_STATIC,
        // Display SPI and pins (factory classic CYD wiring):
        p.SPI2,
        p.GPIO14,
        p.GPIO13,
        p.GPIO12,
        p.GPIO15,
        p.GPIO2,
        NoDisplayReset,
        p.GPIO21,
        DEFAULT_DISPLAY_SPI_HZ,
        // Presentation:
        ORIENTATION,
        BACKGROUND_COLOR,
        FOREGROUND_COLOR,
        &FONT,
        // Touch SPI and pins (factory classic CYD wiring):
        p.SPI3,
        p.GPIO25,
        p.GPIO32,
        p.GPIO39,
        p.GPIO33,
        p.GPIO36,
        // Calibration storage and recalibration button:
        &mut calibration_flash_block,
        &mut *calibration_button_watch,
    )
    .await?;
    info!("CYD initialized; touch coordinates are calibrated and landscape-oriented");

    let exit =
        app::run::<_, _, _, Error>(&mut cyd, &*calibration_button_watch, &mut page_flash_block)
            .await?;
    match exit {
        app::Exit::CalibrationRequested => {
            info!("Clear touch calibration and reset; the paint page selection remains");
            calibration_flash_block.clear()?;
            esp_hal::system::software_reset();
        }
    }
}

#[derive(Debug, derive_more::From)]
enum Error {
    DeviceEnvoy(DeviceEnvoyError),
    Cyd(cyd::Error),
}
