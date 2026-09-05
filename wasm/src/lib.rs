use core::fmt;

use device_envoy_core::{
    UnwrapInfallible,
    button::Button,
    cyd::{
        Cyd, CydDisplay, CydTouch, SCREEN_PIXELS,
        display::{CydFrame, DrawItem, Image565Fixed, Image565View, Orientation, tga},
        touch::TouchEvent,
    },
    flash_block,
    wasm::{self, FlashBlockWasm, cyd_web},
};
use embassy_time::{Duration, Timer};
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::{
    Drawable,
    geometry::{Point, Size},
    pixelcolor::{Rgb565, Rgb888},
    prelude::Primitive,
    primitives::{Line, PrimitiveStyle, Rectangle},
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::wasm_bindgen;

const BACKGROUND_COLOR: Rgb888 = Rgb888::new(246, 235, 204); // warm cream
const FOREGROUND_COLOR: Rgb888 = Rgb888::new(39, 28, 23); // dark brown
const PAGE_TURN: Rectangle = Rectangle::new(Point::new(270, 0), Size::new(50, 45));
const BRUSH_WIDTH: u32 = 7;
const INPUT_POLL_INTERVAL: Duration = Duration::from_millis(16);
const DOG_WALK: Image565View = {
    const IMAGE: Image565Fixed<320, 240, SCREEN_PIXELS> = tga!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../assets/paint-dog-walk.tga"
    ))
    .to_565();
    IMAGE.view()
};
const CRAB_BEACH: Image565View = {
    const IMAGE: Image565Fixed<320, 240, SCREEN_PIXELS> = tga!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../assets/paint-crab-beach.tga"
    ))
    .to_565();
    IMAGE.view()
};
const CAVE_ART: Image565View = {
    const IMAGE: Image565Fixed<320, 240, SCREEN_PIXELS> = tga!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../assets/paint-cave-art.tga"
    ))
    .to_565();
    IMAGE.view()
};
const PAGES: [Image565View; 3] = [CRAB_BEACH, DOG_WALK, CAVE_ART];

const WEB_APP: cyd_web::Config = cyd_web::Config::new(
    "device-envoy/cyd-paint-book",
    Orientation::Landscape,
    BACKGROUND_COLOR,
    FOREGROUND_COLOR,
    &FONT_6X10,
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
    let mut page_storage = FlashBlockWasm::new("device-envoy/cyd-paint-book/page")?;

    let exit = run(&mut cyd, &button, &mut page_storage).await?;
    match exit {
        Exit::CalibrationRequested => Ok(cyd_web::Command::CalibrationNotNeeded),
    }
}

async fn run<CydDevice, ButtonDevice, Storage>(
    cyd: &mut CydDevice,
    button: &ButtonDevice,
    page_storage: &mut Storage,
) -> Result<Exit, AppError<CydDevice::Error, Storage::Error>>
where
    CydDevice: Cyd,
    ButtonDevice: Button,
    Storage: flash_block::FlashBlock,
{
    let page = page_storage
        .load::<Page>()
        .map_err(AppError::Storage)?
        .filter(|page| usize::from(page.index) < PAGES.len())
        .unwrap_or_default();
    let mut page_index = usize::from(page.index);
    let (display, touch) = cyd.parts();
    let mut frame = display.full_frame_mut();
    DrawItem::Bitmap {
        view: PAGES[page_index],
        top_left: Point::zero(),
    }
    .draw(&mut frame);
    frame.flush().await.map_err(AppError::Cyd)?;

    let mut stroke = None;
    loop {
        if button.is_pressed() {
            return Ok(Exit::CalibrationRequested);
        }
        let Some(touch_event) = touch.try_read().map_err(AppError::Cyd)? else {
            Timer::after(INPUT_POLL_INTERVAL).await;
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
                frame.flush().await.map_err(AppError::Cyd)?;
                page_storage
                    .save(&Page::new(page_index))
                    .map_err(AppError::Storage)?;
            }
            TouchEvent::Down { point } => {
                stroke = frame.pixel(point).map(|color| Stroke { point, color });
            }
            TouchEvent::Move { point } => {
                if let Some(stroke_ref) = &mut stroke {
                    Line::new(stroke_ref.point, point)
                        .into_styled(PrimitiveStyle::with_stroke(stroke_ref.color, BRUSH_WIDTH))
                        .draw(&mut frame)
                        .unwrap_infallible();
                    stroke_ref.point = point;
                    frame.flush().await.map_err(AppError::Cyd)?;
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
enum Exit {
    CalibrationRequested,
}

#[derive(Debug)]
enum AppError<CydError, StorageError> {
    Core(device_envoy_core::Error),
    Cyd(CydError),
    Storage(StorageError),
}

impl<CydError, StorageError> From<device_envoy_core::Error> for AppError<CydError, StorageError> {
    fn from(error: device_envoy_core::Error) -> Self {
        Self::Core(error)
    }
}

#[derive(derive_more::From)]
enum Error {
    Wasm(wasm::Error),
    App(AppError<core::convert::Infallible, flash_block::Error<wasm::Error>>),
}

impl fmt::Debug for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wasm(source) => formatter.debug_tuple("Wasm").field(source).finish(),
            Self::App(source) => formatter.debug_tuple("App").field(source).finish(),
        }
    }
}
