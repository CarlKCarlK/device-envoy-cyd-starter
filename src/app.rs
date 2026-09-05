//! Portable Snake application loop, drawing, calibrated input, and persistence.

use core::{
    convert::Infallible,
    fmt::{self, Write},
};

use device_envoy_core::{
    UnwrapInfallible,
    button::Button,
    cyd::{
        Cyd, CydDisplay, CydTouch,
        display::{CydFrame, DrawItem, Image565Fixed, Image565View, Orientation, tga},
        touch::TouchEvent,
    },
    flash_block::FlashBlock,
};
use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::{
    Drawable,
    geometry::{Point, Size},
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::{Rgb565, Rgb888, RgbColor},
    prelude::Primitive,
    primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle},
    text::{Alignment, Baseline, Text, TextStyleBuilder},
};

use crate::{
    game::{Direction, Game, PLAYFIELD, Phase, Tick},
    persistence::{HighScore, load_high_score},
};

pub const ORIENTATION: Orientation = Orientation::Landscape;
pub const BACKGROUND_COLOR: Rgb888 = Rgb888::new(8, 12, 18); // blue-black
pub const FOREGROUND_COLOR: Rgb888 = Rgb888::new(238, 244, 236); // warm white

/// Largest buffered composition region: the 180×90 pause/game-over modal.
///
/// The 320×240 background is streamed directly and normal ticks update only
/// changed cells. A full-screen RGB565 frame would use 153,600 bytes; this
/// bounded frame uses 32,400 bytes and is also reused by smaller UI regions.
pub const FRAME_PIXEL_COUNT: usize = 180 * 90;

// TODO0 (may no longer apply) Replace this generated background if final Snake
// artwork is supplied. Keep the exact 320×240 uncompressed TGA format.
const BACKGROUND_FIXED: Image565Fixed<320, 240, { 320 * 240 }> = tga!(
    concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/snake-background-v2.tga"
    ),
    320,
    240
)
.to_565();
const BACKGROUND: Image565View = BACKGROUND_FIXED.view();

const SCORE_RECTANGLE: Rectangle = Rectangle::new(Point::new(5, 4), Size::new(268, 27));
pub const MODAL_RECTANGLE: Rectangle = Rectangle::new(Point::new(70, 72), Size::new(180, 90));
const MODAL_ACTION_RECTANGLE: Rectangle = Rectangle::new(Point::new(105, 128), Size::new(110, 26));
const TICK_INTERVAL: Duration = Duration::from_millis(155);
const INPUT_POLL_INTERVAL: Duration = Duration::from_millis(20);

const BUTTONS: [(Control, Rectangle, &str); 5] = [
    (
        Control::Up,
        Rectangle::new(Point::new(130, 175), Size::new(60, 20)),
        "^",
    ),
    (
        Control::Left,
        Rectangle::new(Point::new(78, 190), Size::new(53, 27)),
        "<",
    ),
    (
        Control::Right,
        Rectangle::new(Point::new(189, 190), Size::new(53, 27)),
        ">",
    ),
    (
        Control::Down,
        Rectangle::new(Point::new(130, 209), Size::new(60, 27)),
        "v",
    ),
    (
        Control::Pause,
        Rectangle::new(Point::new(276, 4), Size::new(32, 27)),
        "",
    ),
];

//todo000 too much in this file?

const SNAKE_HEAD: Rgb565 = Rgb565::new(8, 63, 12); // vivid lime green
const SNAKE_BODY: Rgb565 = Rgb565::new(3, 44, 12); // leaf green
const FOOD: Rgb565 = Rgb565::new(31, 8, 4); // tomato red
const OUTLINE: Rgb565 = Rgb565::new(10, 35, 22); // blue-gray
const BUTTON_FILL: Rgb565 = Rgb565::new(2, 10, 16); // navy
const BUTTON_PRESSED: Rgb565 = Rgb565::new(4, 30, 25); // teal
const MODAL_FILL: Rgb565 = Rgb565::new(2, 7, 12); // near-black blue

pub async fn run<CydDevice, ButtonDevice, Storage>(
    cyd: &mut CydDevice,
    button: &mut ButtonDevice,
    high_score_storage: &mut Storage,
) -> Result<Exit, Error<CydDevice::Error, Storage::Error>>
where
    CydDevice: Cyd,
    ButtonDevice: Button,
    Storage: FlashBlock,
{
    let mut high_score = load_high_score(high_score_storage).map_err(Error::Storage)?;
    // todo000 is Game needed?
    let mut game = Game::new();
    let (display, touch) = cyd.parts();
    draw_complete_scene(display, &game, high_score, None).await?;

    let mut pressed = None;
    let mut previous_tick = Instant::now();
    loop {
        if button.is_pressed() {
            return Ok(Exit::CalibrationRequested);
        }

        if let Some(touch_event) = touch.try_read().map_err(RenderError::Cyd)? {
            handle_touch(display, touch_event, &mut pressed, &mut game, high_score).await?;
        }

        let now = Instant::now();
        if game.phase() == Phase::Running
            && now.saturating_duration_since(previous_tick) >= TICK_INTERVAL
        {
            match game.tick() {
                Tick::Moved { old_tail } => {
                    restore_background(display, old_tail.rectangle()).map_err(RenderError::Cyd)?;
                    draw_snake(display, &game).map_err(RenderError::Cyd)?;
                    draw_buttons(display, pressed).await?;
                }
                Tick::Ate => {
                    draw_snake(display, &game).map_err(RenderError::Cyd)?;
                    draw_food(display, &game).map_err(RenderError::Cyd)?;
                    draw_score(display, &game, high_score).await?;
                    draw_buttons(display, pressed).await?;
                }
                Tick::GameOver => {
                    high_score
                        .record_if_higher(game.score(), high_score_storage)
                        .map_err(Error::Storage)?;
                    draw_score(display, &game, high_score).await?;
                    draw_modal(display, &game).await?;
                }
            }
            previous_tick = now;
        }
        Timer::after(INPUT_POLL_INTERVAL).await;
    }
}

async fn handle_touch<Display>(
    display: &mut Display,
    touch_event: TouchEvent,
    pressed: &mut Option<Control>,
    game: &mut Game,
    high_score: HighScore,
) -> Result<(), RenderError<Display::Error>>
where
    Display: CydDisplay,
{
    match touch_event {
        TouchEvent::Down { point } | TouchEvent::Move { point } => {
            let next_pressed = control_at(point, game.phase());
            if next_pressed != *pressed {
                *pressed = next_pressed;
                draw_buttons(display, *pressed).await?;
                if game.phase() != Phase::Running {
                    draw_modal(display, game).await?;
                }
            }
            if matches!(touch_event, TouchEvent::Down { .. }) {
                match next_pressed {
                    Some(Control::Up) => game.set_direction(Direction::Up),
                    Some(Control::Down) => game.set_direction(Direction::Down),
                    Some(Control::Left) => game.set_direction(Direction::Left),
                    Some(Control::Right) => game.set_direction(Direction::Right),
                    Some(Control::Pause) => {
                        game.toggle_pause();
                        draw_modal(display, game).await?;
                    }
                    Some(Control::ModalAction) => match game.phase() {
                        Phase::Paused => {
                            game.toggle_pause();
                            redraw_region(display, MODAL_RECTANGLE, game, high_score, *pressed)
                                .await?;
                        }
                        Phase::GameOver => {
                            game.restart();
                            draw_complete_scene(display, game, high_score, *pressed).await?;
                        }
                        Phase::Running => {}
                    },
                    None => {}
                }
            }
        }
        TouchEvent::Up => {
            *pressed = None;
            draw_buttons(display, None).await?;
            if game.phase() != Phase::Running {
                draw_modal(display, game).await?;
            }
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Control {
    Up,
    Down,
    Left,
    Right,
    Pause,
    ModalAction,
}

fn control_at(point: Point, phase: Phase) -> Option<Control> {
    if phase != Phase::Running && MODAL_ACTION_RECTANGLE.contains(point) {
        return Some(Control::ModalAction);
    }
    BUTTONS
        .iter()
        .find(|(_, rectangle, _)| rectangle.contains(point))
        .map(|(control, _, _)| *control)
}

async fn draw_complete_scene<Display>(
    display: &mut Display,
    game: &Game,
    high_score: HighScore,
    pressed: Option<Control>,
) -> Result<(), RenderError<Display::Error>>
where
    Display: CydDisplay,
{
    display
        .fill_contiguous_full(BACKGROUND.rgb565_iter())
        .map_err(RenderError::Cyd)?;
    draw_playfield_outline(display).map_err(RenderError::Cyd)?;
    draw_snake(display, game).map_err(RenderError::Cyd)?;
    draw_food(display, game).map_err(RenderError::Cyd)?;
    draw_score(display, game, high_score).await?;
    draw_buttons(display, pressed).await?;
    if game.phase() != Phase::Running {
        draw_modal(display, game).await?;
    }
    Ok(())
}

fn draw_playfield_outline<Display>(display: &mut Display) -> Result<(), Display::Error>
where
    Display: CydDisplay,
{
    let top_left = PLAYFIELD.top_left;
    let width = PLAYFIELD.size.width;
    let height = PLAYFIELD.size.height;
    display.fill_rectangle(Rectangle::new(top_left, Size::new(width, 2)), OUTLINE)?;
    display.fill_rectangle(
        Rectangle::new(
            Point::new(top_left.x, top_left.y + height as i32 - 2),
            Size::new(width, 2),
        ),
        OUTLINE,
    )?;
    display.fill_rectangle(Rectangle::new(top_left, Size::new(2, height)), OUTLINE)?;
    display.fill_rectangle(
        Rectangle::new(
            Point::new(top_left.x + width as i32 - 2, top_left.y),
            Size::new(2, height),
        ),
        OUTLINE,
    )
}

fn draw_snake<Display>(display: &mut Display, game: &Game) -> Result<(), Display::Error>
where
    Display: CydDisplay,
{
    for (body_index, cell) in game.body().enumerate() {
        display.fill_rectangle(
            cell.rectangle(),
            if body_index == 0 {
                SNAKE_HEAD
            } else {
                SNAKE_BODY
            },
        )?;
    }
    Ok(())
}

fn draw_food<Display>(display: &mut Display, game: &Game) -> Result<(), Display::Error>
where
    Display: CydDisplay,
{
    let rectangle = game.food().rectangle();
    display.draw_items::<1>(
        rectangle,
        display.background_565(),
        [DrawItem::Circle {
            center: (rectangle.center().x as f32, rectangle.center().y as f32),
            pixel_radius: 4.0,
            color: Rgb888::from(FOOD),
        }],
    )
}

fn restore_background<Display>(
    display: &mut Display,
    rectangle: Rectangle,
) -> Result<(), Display::Error>
where
    Display: CydDisplay,
{
    let crop = BACKGROUND_FIXED.view_rect(rectangle);
    display.fill_contiguous(rectangle, crop.rgb565_iter())
}

async fn draw_score<Display>(
    display: &mut Display,
    game: &Game,
    high_score: HighScore,
) -> Result<(), RenderError<Display::Error>>
where
    Display: CydDisplay,
{
    let mut frame = display.frame_mut(SCORE_RECTANGLE);
    DrawItem::Bitmap {
        view: BACKGROUND,
        top_left: Point::zero(),
    }
    .draw(&mut frame);
    draw_scores(&mut frame, game, high_score)?;
    frame.flush().await.map_err(RenderError::Cyd)
}

async fn draw_buttons<Display>(
    display: &mut Display,
    pressed: Option<Control>,
) -> Result<(), RenderError<Display::Error>>
where
    Display: CydDisplay,
{
    for (control, rectangle, label) in BUTTONS {
        draw_button(display, control, rectangle, label, pressed == Some(control)).await?;
    }
    Ok(())
}

async fn draw_button<Display>(
    display: &mut Display,
    control: Control,
    rectangle: Rectangle,
    label: &str,
    pressed: bool,
) -> Result<(), RenderError<Display::Error>>
where
    Display: CydDisplay,
{
    let mut frame = display.frame_mut(rectangle);
    DrawItem::Bitmap {
        view: BACKGROUND,
        top_left: Point::zero(),
    }
    .draw(&mut frame);
    if pressed {
        rectangle
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .stroke_color(BUTTON_PRESSED)
                    .stroke_width(2)
                    .build(),
            )
            .draw(&mut frame)
            .unwrap_infallible();
    }
    draw_control_symbol(&mut frame, control, rectangle, label);
    frame.flush().await.map_err(RenderError::Cyd)
}

async fn draw_modal<Display>(
    display: &mut Display,
    game: &Game,
) -> Result<(), RenderError<Display::Error>>
where
    Display: CydDisplay,
{
    let mut frame = display.frame_mut(MODAL_RECTANGLE);
    frame.fill(MODAL_FILL);
    MODAL_RECTANGLE
        .into_styled(PrimitiveStyle::with_stroke(OUTLINE, 2))
        .draw(&mut frame)
        .unwrap_infallible();
    let (title, action) = match game.phase() {
        Phase::Paused => ("PAUSED", "RESUME"),
        Phase::GameOver => ("GAME OVER", "PLAY AGAIN"),
        Phase::Running => return Ok(()),
    };
    draw_text(&mut frame, title, Point::new(160, 88), Alignment::Center);
    MODAL_ACTION_RECTANGLE
        .into_styled(PrimitiveStyle::with_fill(BUTTON_FILL))
        .draw(&mut frame)
        .unwrap_infallible();
    draw_text(&mut frame, action, Point::new(160, 136), Alignment::Center);
    frame.flush().await.map_err(RenderError::Cyd)
}

async fn redraw_region<Display>(
    display: &mut Display,
    rectangle: Rectangle,
    game: &Game,
    high_score: HighScore,
    pressed: Option<Control>,
) -> Result<(), RenderError<Display::Error>>
where
    Display: CydDisplay,
{
    let mut frame = display.frame_mut(rectangle);
    DrawItem::Bitmap {
        view: BACKGROUND,
        top_left: Point::zero(),
    }
    .draw(&mut frame);
    draw_scene_into_frame(&mut frame, game, high_score, pressed)?;
    frame.flush().await.map_err(RenderError::Cyd)
}

fn draw_scene_into_frame<Frame>(
    frame: &mut Frame,
    game: &Game,
    high_score: HighScore,
    pressed: Option<Control>,
) -> fmt::Result
where
    Frame: CydFrame,
{
    PLAYFIELD
        .into_styled(PrimitiveStyle::with_stroke(OUTLINE, 2))
        .draw(frame)
        .unwrap_infallible();
    for (body_index, cell) in game.body().enumerate() {
        cell.rectangle()
            .into_styled(PrimitiveStyle::with_fill(if body_index == 0 {
                SNAKE_HEAD
            } else {
                SNAKE_BODY
            }))
            .draw(frame)
            .unwrap_infallible();
    }
    game.food()
        .rectangle()
        .into_styled(PrimitiveStyle::with_fill(FOOD))
        .draw(frame)
        .unwrap_infallible();
    draw_scores(frame, game, high_score)?;
    for (control, rectangle, label) in BUTTONS {
        if pressed == Some(control) {
            rectangle
                .into_styled(PrimitiveStyle::with_stroke(BUTTON_PRESSED, 2))
                .draw(frame)
                .unwrap_infallible();
        }
        draw_control_symbol(frame, control, rectangle, label);
    }
    Ok(())
}

fn draw_scores<Target>(target: &mut Target, game: &Game, high_score: HighScore) -> fmt::Result
where
    Target: embedded_graphics::draw_target::DrawTarget<Color = Rgb565, Error = Infallible>,
{
    let mut score_text = heapless::String::<16>::new();
    write!(score_text, "SCORE {:04}", game.score())?;
    draw_text(
        target,
        score_text.as_str(),
        Point::new(74, 12),
        Alignment::Center,
    );

    let mut high_score_text = heapless::String::<16>::new();
    write!(high_score_text, "HIGH {:04}", high_score.value())?;
    draw_text(
        target,
        high_score_text.as_str(),
        Point::new(210, 12),
        Alignment::Center,
    );
    Ok(())
}

fn draw_control_symbol<Target>(
    target: &mut Target,
    control: Control,
    rectangle: Rectangle,
    label: &str,
) where
    Target: embedded_graphics::draw_target::DrawTarget<Color = Rgb565, Error = Infallible>,
{
    if control == Control::Pause {
        let center = rectangle.center();
        for offset_x in [-4, 2] {
            Rectangle::new(
                Point::new(center.x + offset_x, center.y - 5),
                Size::new(3, 11),
            )
            .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
            .draw(target)
            .unwrap_infallible();
        }
    } else {
        draw_text(
            target,
            label,
            rectangle.center() - Point::new(0, 4),
            Alignment::Center,
        );
    }
}

fn draw_text<Target>(target: &mut Target, text: &str, position: Point, alignment: Alignment)
where
    Target: embedded_graphics::draw_target::DrawTarget<Color = Rgb565, Error = Infallible>,
{
    Text::with_text_style(
        text,
        position,
        MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE),
        TextStyleBuilder::new()
            .alignment(alignment)
            .baseline(Baseline::Top)
            .build(),
    )
    .draw(target)
    .unwrap_infallible();
}

#[derive(Debug)]
pub enum Error<CydError, StorageError> {
    Render(RenderError<CydError>),
    Storage(StorageError),
}

impl<CydError, StorageError> From<RenderError<CydError>> for Error<CydError, StorageError> {
    fn from(error: RenderError<CydError>) -> Self {
        Self::Render(error)
    }
}

#[derive(Debug)]
pub enum RenderError<CydError> {
    Cyd(CydError),
    Text(fmt::Error),
}

impl<CydError> From<fmt::Error> for RenderError<CydError> {
    fn from(error: fmt::Error) -> Self {
        Self::Text(error)
    }
}

#[derive(Debug)]
pub enum Exit {
    CalibrationRequested,
}
