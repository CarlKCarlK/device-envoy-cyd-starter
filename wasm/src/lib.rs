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
    let mut page_flash_block = FlashBlockWasm::new("device-envoy/cyd-paint-book/page")?;

    let exit = run(&mut cyd, &button, &mut page_flash_block).await?;
    match exit {
        Exit::CalibrationRequested => Ok(cyd_web::Command::CalibrationNotNeeded),
    }
}

async fn run<CydDevice, ButtonDevice, FlashBlockDevice>(
    cyd: &mut CydDevice,
    button: &ButtonDevice,
    page_flash_block: &mut FlashBlockDevice,
) -> Result<Exit, AppError<CydDevice::Error, FlashBlockDevice::Error>>
where
    CydDevice: Cyd,
    ButtonDevice: Button,
    FlashBlockDevice: flash_block::FlashBlock,
{
    let mut page_index = page_flash_block
        .load::<PageIndex>()
        .map_err(AppError::Storage)?
        .unwrap_or_default();
    let (display, touch) = cyd.parts();
    let mut frame = display.full_frame_mut();
    DrawItem::Bitmap {
        view: page_index.image(),
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
                page_index = page_index.next();
                DrawItem::Bitmap {
                    view: page_index.image(),
                    top_left: Point::zero(),
                }
                .draw(&mut frame);
                frame.flush().await.map_err(AppError::Cyd)?;
                page_flash_block
                    .save(&page_index)
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
enum PageIndex {
    #[default]
    CrabBeach,
    DogWalk,
    CaveArt,
}

impl PageIndex {
    fn next(self) -> Self {
        match self {
            Self::CrabBeach => Self::DogWalk,
            Self::DogWalk => Self::CaveArt,
            Self::CaveArt => Self::CrabBeach,
        }
    }

    fn image(self) -> Image565View {
        match self {
            Self::CrabBeach => CRAB_BEACH,
            Self::DogWalk => DOG_WALK,
            Self::CaveArt => CAVE_ART,
        }
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
