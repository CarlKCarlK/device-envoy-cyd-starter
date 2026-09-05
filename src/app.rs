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
use embedded_graphics::{
    Drawable,
    geometry::{Point, Size},
    mono_font::{MonoFont, ascii::FONT_9X15_BOLD},
    pixelcolor::Rgb888,
    prelude::Primitive,
    primitives::{Line, PrimitiveStyle, Rectangle},
};

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

pub async fn run<CydDevice, ButtonDevice, FlashBlockDevice, AppError>(
    cyd: &mut CydDevice,
    calibration_button: &ButtonDevice,
    page_flash_block: &mut FlashBlockDevice,
) -> Result<Exit, AppError>
where
    CydDevice: Cyd,
    ButtonDevice: Button,
    FlashBlockDevice: FlashBlock,
    AppError: From<CydDevice::Error> + From<FlashBlockDevice::Error>,
{
    let mut page = page_flash_block.load::<Page>()?.unwrap_or_default();
    let (display, touch) = cyd.parts();
    let mut frame = display.full_frame_mut();
    DrawItem::Bitmap {
        view: page.bitmap(),
        top_left: Point::zero(),
    }
    .draw(&mut frame);
    let mut previous_point = None;
    loop {
        if calibration_button.is_pressed() {
            return Ok(Exit::CalibrationRequested);
        }

        match touch.try_read()? {
            Some(TouchEvent::Down { point }) if PAGE_TURN.contains(point) => {
                previous_point = None;
                page = page.next();
                DrawItem::Bitmap {
                    view: page.bitmap(),
                    top_left: Point::zero(),
                }
                .draw(&mut frame);
                page_flash_block.save(&page)?;
            }
            Some(TouchEvent::Down { point }) => {
                previous_point = Some(point);
            }
            Some(TouchEvent::Move { point }) => {
                if let Some(previous_point) = &mut previous_point {
                    if let Some(color) = frame.pixel(*previous_point) {
                        Line::new(*previous_point, point)
                            .into_styled(PrimitiveStyle::with_stroke(color, BRUSH_WIDTH))
                            .draw(&mut frame)
                            .unwrap_infallible();
                    }
                    *previous_point = point;
                }
            }
            Some(TouchEvent::Up) => {
                previous_point = None;
            }
            None => {}
        }

        frame.flush().await?;
    }
}

#[derive(Clone, Copy, Debug, Default, serde::Deserialize, serde::Serialize)]
pub enum Page {
    #[default]
    CrabBeach,
    DogWalk,
    CaveArt,
}

impl Page {
    pub fn next(self) -> Self {
        match self {
            Self::CrabBeach => Self::DogWalk,
            Self::DogWalk => Self::CaveArt,
            Self::CaveArt => Self::CrabBeach,
        }
    }

    pub fn bitmap(self) -> Image565View {
        match self {
            Self::CrabBeach => CRAB_BEACH,
            Self::DogWalk => DOG_WALK,
            Self::CaveArt => CAVE_ART,
        }
    }
}

#[derive(Debug)]
pub enum Exit {
    CalibrationRequested,
}
