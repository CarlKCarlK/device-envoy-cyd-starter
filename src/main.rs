#![no_std]
#![no_main]

use core::{convert::Infallible, fmt};

use device_envoy_core::{
    UnwrapInfallible,
    button::Button,
    cyd::{
        Cyd, CydDisplay, CydTouch, SCREEN_PIXELS,
        display::{CydFrame, DrawItem, Image565Fixed, Image565View, Orientation, tga},
        touch::TouchEvent,
    },
    flash_block::FlashBlock,
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
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    Drawable,
    geometry::{Point, Size},
    pixelcolor::{Rgb565, Rgb888},
    prelude::Primitive,
    primitives::{Line, PrimitiveStyle, Rectangle},
};
use esp_backtrace as _;
use log::info;
use serde::{Deserialize, Serialize};

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

async fn inner_main(spawner: Spawner) -> Result<Infallible, MainError> {
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

    let exit = run(&mut cyd, &*button_watch, &mut page_flash_block).await?;
    match exit {
        Exit::CalibrationRequested => {
            info!("Clear touch calibration and reset; the paint page selection remains");
            calibration_flash_block.clear()?;
            esp_hal::system::software_reset();
        }
    }
}

pub const ORIENTATION: Orientation = Orientation::Landscape;
pub const BACKGROUND_COLOR: Rgb888 = Rgb888::new(246, 235, 204); // warm cream
pub const FOREGROUND_COLOR: Rgb888 = Rgb888::new(39, 28, 23); // dark brown
pub const FRAME_PIXEL_COUNT: usize = SCREEN_PIXELS;

const PAGE_TURN: Rectangle = Rectangle::new(Point::new(270, 0), Size::new(50, 45));
const BRUSH_WIDTH: u32 = 7;
const INPUT_POLL_INTERVAL: Duration = Duration::from_millis(16);
// TODO000 Add an FPS meter.

// TODO000 Is too much functionality still concentrated in this file?

// TODO000 (may no longer apply) Keep the cave-art asset name consistent.
const DOG_WALK: Image565View = {
    const IMAGE: Image565Fixed<320, 240, SCREEN_PIXELS> = tga!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/paint-dog-walk.tga"
    ))
    .to_565();
    IMAGE.view()
};
const CRAB_BEACH: Image565View = {
    const IMAGE: Image565Fixed<320, 240, SCREEN_PIXELS> = tga!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/paint-crab-beach.tga"
    ))
    .to_565();
    IMAGE.view()
};
const CAVE_ART: Image565View = {
    const IMAGE: Image565Fixed<320, 240, SCREEN_PIXELS> = tga!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/paint-cave-art.tga"
    ))
    .to_565();
    IMAGE.view()
};
const PAGES: [Image565View; 3] = [CRAB_BEACH, DOG_WALK, CAVE_ART];

async fn run<CydDevice, ButtonDevice, Storage>(
    cyd: &mut CydDevice,
    button: &ButtonDevice,
    page_storage: &mut Storage,
) -> Result<Exit, Error<CydDevice::Error, Storage::Error>>
where
    CydDevice: Cyd,
    ButtonDevice: Button,
    Storage: FlashBlock,
{
    // todo000000 is we really want to presiste the page section? is it a waste of flash lifetime?
    // TODO000 Does the selected page reliably persist across power cycles and
    // recover cleanly from invalid stored data?
    let page = page_storage
        .load::<Page>()
        .map_err(Error::Storage)?
        .filter(|page| usize::from(page.index) < PAGES.len())
        .unwrap_or_default();
    let mut page_index = usize::from(page.index);
    let (display, touch) = cyd.parts();
    // todo0000 we need comments
    let mut frame = display.full_frame_mut();
    // todo0000 shouldn't we use streaming to sent bitmaps?
    DrawItem::Bitmap {
        view: PAGES[page_index],
        top_left: Point::zero(),
    }
    .draw(&mut frame);
    frame.flush().await.map_err(Error::Cyd)?;

    let mut stroke = None;
    loop {
        if button.is_pressed() {
            return Ok(Exit::CalibrationRequested);
        }

        let Some(touch_event) = touch.try_read().map_err(Error::Cyd)? else {
            Timer::after(INPUT_POLL_INTERVAL).await; // todo000 devolve the const
            continue;
        };
        match touch_event {
            TouchEvent::Down { point } if PAGE_TURN.contains(point) => {
                stroke = None;
                page_index = (page_index + 1) % PAGES.len();
                DrawItem::Bitmap {
                    view: PAGES[page_index],
                    top_left: Point::zero(),
                }
                .draw(&mut frame);
                frame.flush().await.map_err(Error::Cyd)?;
                page_storage
                    .save(&Page::new(page_index))
                    .map_err(Error::Storage)?;
            }
            // todo0000 what's up with Stroke? Could we just have a color?
            TouchEvent::Down { point } => {
                stroke = frame.pixel(point).map(|color| Stroke { point, color });
            }
            TouchEvent::Move { point } => {
                if let Some(stroke_ref) = &mut stroke {
                    // todo000 is it weird I'm not using the streaming API for drawing lines?
                    Line::new(stroke_ref.point, point)
                        .into_styled(PrimitiveStyle::with_stroke(stroke_ref.color, BRUSH_WIDTH))
                        .draw(&mut frame)
                        .unwrap_infallible();
                    stroke_ref.point = point;
                    frame.flush().await.map_err(Error::Cyd)?;
                }
            }
            TouchEvent::Up => stroke = None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
struct Page {
    index: u8,
}

impl Page {
    fn new(index: usize) -> Self {
        assert!(index < PAGES.len(), "page index must identify a page");
        Self { index: index as u8 }
    }
}

struct Stroke {
    point: Point,
    color: Rgb565,
}

#[derive(Debug)]
enum Error<CydError, StorageError> {
    Core(device_envoy_core::Error),
    Cyd(CydError),
    Storage(StorageError),
}

impl<CydError, StorageError> From<device_envoy_core::Error> for Error<CydError, StorageError> {
    fn from(error: device_envoy_core::Error) -> Self {
        Self::Core(error)
    }
}

#[derive(Debug)]
enum Exit {
    CalibrationRequested,
}

// Derived Debug reads these payloads at runtime, but dead_code analysis ignores
// derived implementations under -D warnings.
// TODO000 Explain why this application-level error type is needed here.
#[derive(derive_more::From)]
enum MainError {
    DeviceEnvoy(DeviceEnvoyError),
    Cyd(cyd::Error),
    App(Error<cyd::Error, DeviceEnvoyError>),
}

// TODO00 Is this manual Debug implementation the nicest approach?
impl fmt::Debug for MainError {
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
