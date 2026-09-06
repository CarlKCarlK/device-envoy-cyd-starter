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
use embassy_time::Timer;
use embedded_graphics::{
    Drawable,
    geometry::{Point, Size},
    mono_font::{MonoFont, ascii::FONT_9X15_BOLD},
    pixelcolor::Rgb888,
    prelude::{Primitive, WebColors},
    primitives::{Line, PrimitiveStyle, Rectangle},
};

pub const ORIENTATION: Orientation = Orientation::Landscape;
pub const BACKGROUND_COLOR: Rgb888 = Rgb888::CSS_BLANCHED_ALMOND;
pub const FOREGROUND_COLOR: Rgb888 = Rgb888::new(39, 28, 23); // dark brown
pub const FONT: MonoFont<'static> = FONT_9X15_BOLD;

const PAGE_TURN_RECTANGLE: Rectangle = Rectangle::new(Point::new(270, 0), Size::new(50, 45));
const BRUSH_WIDTH: u32 = 7;

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
    // Split the CYD into its display and touch components.
    let (display, touch) = cyd.parts();

    // For ease of use, create a full frame buffer for the display.
    // (When memory is tighter, the library supports smaller buffers, tiling, and streaming pixels.)
    let mut frame = display.full_frame_mut();

    // Which coloring book page should we start with? Read a variant from the flash block.
    // If empty or the flash block contains the wrong type, default to the first page.
    let mut page = page_flash_block.load::<Page>()?.unwrap_or_default();


    // Draw the first page's bitmap to the frame buffer and flush it to the display.
    DrawItem::Bitmap {
        view: page.bitmap(),
        top_left: Point::zero(),
    }
    .draw(&mut frame);
    frame.flush().await?;


    // Keep track of the previous touch point for drawing lines.
    let mut previous_point = None;

    // Start the "game" loop.
    loop {

        // User must always be able to recalibrate the touch screen.
        // If the user presses the button on the back of the CYD, exit this function
        // and return to the hardware-specific caller.
        if calibration_button.is_pressed() {
            return Ok(Exit::CalibrationRequested);
        }

        // Read the next touch event if any. This never blocks; it returns immediately.
        // Use a match statement to handle the different types of touch events.
        match touch.try_read()? {
            // When there is no new touch input,
            None => {
                // Avoid repeatedly flushing the display while idle.
                Timer::after_millis(16).await;
                continue;
            }
            // When a touch begins in the page-turn area,
            Some(TouchEvent::Down { point }) if PAGE_TURN_RECTANGLE.contains(point) => {
                // clear the previous touch point and advance to the next page.
                previous_point = None;
                page = page.next();
                DrawItem::Bitmap {
                    view: page.bitmap(),
                    top_left: Point::zero(),
                }
                .draw(&mut frame);
                page_flash_block.save(&page)?;
            }
            // When a touch begins elsewhere on the page,
            Some(TouchEvent::Down { point }) => {
                // remember the touched point.
                previous_point = Some(point);
            }
            // When the touch moves,
            Some(TouchEvent::Move { point }) => {
                // draw a line from the previous point (if any) using the color beneath it.
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
            // When the touch is released,
            Some(TouchEvent::Up) => {
                // clear the previous touch point.
                previous_point = None;
            }
        }

        // Flush the frame buffer to the display.
        frame.flush().await?;
    }
}

#[derive(Clone, Copy, Debug, Default, serde::Deserialize, serde::Serialize)]
enum Page {
    #[default]
    CrabBeach,
    DogWalk,
    CaveArt,
}

impl Page {
    fn next(self) -> Self {
        match self {
            Self::CrabBeach => Self::DogWalk,
            Self::DogWalk => Self::CaveArt,
            Self::CaveArt => Self::CrabBeach,
        }
    }

    fn bitmap(self) -> Image565View {
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
