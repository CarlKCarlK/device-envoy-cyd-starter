//! Portable paint-book application shared by the CYD and browser simulator.

use device_envoy_core::{
    UnwrapInfallible,
    button::Button,
    cyd::{
        Cyd, CydDisplay, CydTouch, SCREEN_PIXELS,
        display::{CydFrame, Image565Fixed, Orientation, tga},
        touch::TouchEvent,
    },
    flash_block::FlashBlock,
};
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    Drawable,
    geometry::{Point, Size},
    pixelcolor::{Rgb565, Rgb888},
    prelude::Primitive,
    primitives::{Line, PrimitiveStyle, Rectangle},
};
use serde::{Deserialize, Serialize};

pub const ORIENTATION: Orientation = Orientation::Landscape;
pub const BACKGROUND_COLOR: Rgb888 = Rgb888::new(246, 235, 204); // warm cream
pub const FOREGROUND_COLOR: Rgb888 = Rgb888::new(39, 28, 23); // dark brown
pub const FRAME_PIXEL_COUNT: usize = SCREEN_PIXELS;

const PAGE_TURN: Rectangle = Rectangle::new(Point::new(270, 0), Size::new(50, 45));
const BRUSH_WIDTH: u32 = 7;
const INPUT_POLL_INTERVAL: Duration = Duration::from_millis(16);
// TODO000 Add an FPS meter.

// TODO000 Is too much functionality still concentrated in this file?

const DOG_WALK: Image565Fixed<320, 240, SCREEN_PIXELS> = tga!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/paint-dog-walk.tga"
))
.to_565();
const CRAB_BEACH: Image565Fixed<320, 240, SCREEN_PIXELS> = tga!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/paint-crab-beach.tga"
))
.to_565();
// todo00 caveart -> cave_art
const CAVE_ART: Image565Fixed<320, 240, SCREEN_PIXELS> = tga!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/paint-caveart.tga"
))
.to_565();
const PAGES: [&Image565Fixed<320, 240, SCREEN_PIXELS>; 3] = [&CRAB_BEACH, &DOG_WALK, &CAVE_ART];

pub async fn run<CydDevice, ButtonDevice, Storage>(
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
    PAGES[page_index].copy_to(&mut frame)?;
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
                PAGES[page_index].copy_to(&mut frame)?;
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
pub enum Error<CydError, StorageError> {
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
pub enum Exit {
    CalibrationRequested,
}
