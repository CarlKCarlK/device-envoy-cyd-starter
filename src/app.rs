use core::fmt::Write as _;

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
use embassy_time::{Instant, Timer};
use embedded_graphics::{
    Drawable,
    geometry::{Point, Size},
    mono_font::{MonoFont, ascii::FONT_9X15_BOLD},
    pixelcolor::{Rgb565, Rgb888},
    prelude::Primitive,
    primitives::{Line, PrimitiveStyle, Rectangle},
};

const FPS_STATUS_RECTANGLE: Rectangle = Rectangle::new(Point::zero(), Size::new(90, 20));

pub const ORIENTATION: Orientation = Orientation::Landscape;
pub const BACKGROUND_COLOR: Rgb888 = Rgb888::new(246, 235, 204); // warm cream
pub const FOREGROUND_COLOR: Rgb888 = Rgb888::new(39, 28, 23); // dark brown
pub const APP_FONT: MonoFont<'static> = FONT_9X15_BOLD;
pub const FRAME_PIXEL_COUNT: usize = SCREEN_PIXELS;
pub const PAGE_TURN: Rectangle = Rectangle::new(Point::new(270, 0), Size::new(50, 45));
pub const BRUSH_WIDTH: u32 = 7;
pub const INPUT_POLL_INTERVAL: embassy_time::Duration = embassy_time::Duration::from_millis(16);

pub const DOG_WALK: Image565View = {
    const IMAGE: Image565Fixed<320, 240, SCREEN_PIXELS> = tga!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/paint-dog-walk.tga"
    ))
    .to_565();
    IMAGE.view()
};
pub const CRAB_BEACH: Image565View = {
    const IMAGE: Image565Fixed<320, 240, SCREEN_PIXELS> = tga!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/paint-crab-beach.tga"
    ))
    .to_565();
    IMAGE.view()
};
pub const CAVE_ART: Image565View = {
    const IMAGE: Image565Fixed<320, 240, SCREEN_PIXELS> = tga!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/paint-cave-art.tga"
    ))
    .to_565();
    IMAGE.view()
};

#[derive(Clone, Copy, Debug, Default, serde::Deserialize, serde::Serialize)]
pub enum PageIndex {
    #[default]
    CrabBeach,
    DogWalk,
    CaveArt,
}

impl PageIndex {
    pub fn next(self) -> Self {
        match self {
            Self::CrabBeach => Self::DogWalk,
            Self::DogWalk => Self::CaveArt,
            Self::CaveArt => Self::CrabBeach,
        }
    }

    pub fn image(self) -> Image565View {
        match self {
            Self::CrabBeach => CRAB_BEACH,
            Self::DogWalk => DOG_WALK,
            Self::CaveArt => CAVE_ART,
        }
    }
}

pub async fn run<CydDevice, ButtonDevice, FlashBlockDevice>(
    cyd: &mut CydDevice,
    calibration_button: &ButtonDevice,
    page_flash_block: &mut FlashBlockDevice,
) -> Result<Exit, Error<CydDevice::Error, FlashBlockDevice::Error>>
where
    CydDevice: Cyd,
    ButtonDevice: Button,
    FlashBlockDevice: FlashBlock,
{
    let mut page_index = page_flash_block
        .load::<PageIndex>()
        .map_err(Error::Storage)?
        .unwrap_or_default();
    let (display, touch) = cyd.parts();
    let mut frame = display.full_frame_mut();
    DrawItem::Bitmap {
        view: page_index.image(),
        top_left: Point::zero(),
    }
    .draw(&mut frame);
    draw_fps(&mut frame, BACKGROUND_COLOR.into(), None);
    frame.flush().await?;

    let mut stroke = None;
    let mut previous_tick = Instant::now();
    loop {
        if calibration_button.is_pressed() {
            return Ok(Exit::CalibrationRequested);
        }

        if let Some(touch_event) = touch.try_read()? {
            match touch_event {
                TouchEvent::Down { point } if PAGE_TURN.contains(point) => {
                    stroke = None;
                    page_index = page_index.next();
                    DrawItem::Bitmap {
                        view: page_index.image(),
                        top_left: Point::zero(),
                    }
                    .draw(&mut frame);
                    page_flash_block.save(&page_index).map_err(Error::Storage)?;
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
                    }
                }
                TouchEvent::Up => stroke = None,
            }
        }

        let current_tick = Instant::now();
        let elapsed_micros = current_tick
            .saturating_duration_since(previous_tick)
            .as_micros();
        draw_fps(
            &mut frame,
            BACKGROUND_COLOR.into(),
            fps_from_elapsed_micros(elapsed_micros),
        );
        frame.flush().await?;
        previous_tick = current_tick;
        Timer::after(INPUT_POLL_INTERVAL).await;
    }
}

#[derive(Debug)]
pub enum Exit {
    CalibrationRequested,
}

#[derive(Debug, derive_more::From)]
pub enum Error<CydError, StorageError> {
    Cyd(CydError),
    // `StorageError` is explicit because deriving both generic conversions would
    // overlap when a device uses the same error type for display and storage.
    #[from(ignore)]
    Storage(StorageError),
}

struct Stroke {
    point: Point,
    color: Rgb565,
}

pub fn fps_from_elapsed_micros(elapsed_micros: u64) -> Option<(u64, u64)> {
    (elapsed_micros != 0).then(|| {
        let fps_tenths = 10_000_000_u64.saturating_add(elapsed_micros / 2) / elapsed_micros;
        let fps_tenths = fps_tenths.min(999);
        (fps_tenths / 10, fps_tenths % 10)
    })
}

pub fn draw_fps<Frame>(frame: &mut Frame, background_color: Rgb565, fps: Option<(u64, u64)>)
where
    Frame: CydFrame,
{
    FPS_STATUS_RECTANGLE
        .into_styled(PrimitiveStyle::with_fill(background_color))
        .draw(frame)
        .unwrap_infallible();

    match fps {
        Some((whole, fraction)) => {
            let mut text = heapless::String::<16>::new();
            if write!(&mut text, "FPS {whole:>3}.{fraction}").is_ok() {
                frame.write_text(&text);
            } else {
                frame.write_text("FPS ERROR");
            }
        }
        None => {
            frame.write_text("FPS   --.-");
        }
    }
}
