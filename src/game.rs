//! Small fixed-capacity Snake rules engine shared by hardware and WASM.

use embedded_graphics::{
    geometry::{Point, Size},
    primitives::Rectangle,
};

pub const CELL_SIZE: i32 = 10;
pub const PLAYFIELD: Rectangle = Rectangle::new(Point::new(4, 33), Size::new(312, 141));
const BORDER_SIZE: i32 = 5;
pub const COLUMN_COUNT: i16 = ((PLAYFIELD.size.width as i32 - BORDER_SIZE * 2) / CELL_SIZE) as i16;
pub const ROW_COUNT: i16 = ((PLAYFIELD.size.height as i32 - BORDER_SIZE * 2) / CELL_SIZE) as i16;
const MAX_CELL_COUNT: usize = COLUMN_COUNT as usize * ROW_COUNT as usize;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cell {
    pub column: i16,
    pub row: i16,
}

impl Cell {
    pub const fn new(column: i16, row: i16) -> Self {
        Self { column, row }
    }

    pub fn rectangle(self) -> Rectangle {
        Rectangle::new(
            Point::new(
                PLAYFIELD.top_left.x + BORDER_SIZE + self.column as i32 * CELL_SIZE,
                PLAYFIELD.top_left.y + BORDER_SIZE + self.row as i32 * CELL_SIZE,
            ),
            Size::new(CELL_SIZE as u32, CELL_SIZE as u32),
        )
    }

    fn stepped(self, direction: Direction) -> Self {
        match direction {
            Direction::Up => Self::new(self.column, self.row - 1),
            Direction::Down => Self::new(self.column, self.row + 1),
            Direction::Left => Self::new(self.column - 1, self.row),
            Direction::Right => Self::new(self.column + 1, self.row),
        }
    }

    fn is_inside(self) -> bool {
        self.column >= 0 && self.column < COLUMN_COUNT && self.row >= 0 && self.row < ROW_COUNT
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    fn is_opposite(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::Up, Self::Down)
                | (Self::Down, Self::Up)
                | (Self::Left, Self::Right)
                | (Self::Right, Self::Left)
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    Running,
    Paused,
    GameOver,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tick {
    Moved { old_tail: Cell },
    Ate,
    GameOver,
}

pub struct Game {
    body: [Cell; MAX_CELL_COUNT],
    length: usize,
    direction: Direction,
    queued_direction: Direction,
    food: Cell,
    score: u32,
    random_state: u32,
    phase: Phase,
}

impl Game {
    pub fn new() -> Self {
        let center = Cell::new(COLUMN_COUNT / 2, ROW_COUNT / 2);
        let mut body = [center; MAX_CELL_COUNT];
        body[1] = Cell::new(center.column - 1, center.row);
        body[2] = Cell::new(center.column - 2, center.row);
        let mut game = Self {
            body,
            length: 3,
            direction: Direction::Right,
            queued_direction: Direction::Right,
            food: Cell::new(0, 0),
            score: 0,
            random_state: 0x51A7_7EED,
            phase: Phase::Running,
        };
        game.place_food();
        game
    }

    pub fn restart(&mut self) {
        let random_state = self.random_state;
        *self = Self::new();
        self.random_state ^= random_state.rotate_left(13);
        self.place_food();
    }

    pub const fn phase(&self) -> Phase {
        self.phase
    }

    pub const fn score(&self) -> u32 {
        self.score
    }

    pub const fn food(&self) -> Cell {
        self.food
    }

    pub fn body(&self) -> impl Iterator<Item = Cell> + '_ {
        self.body[..self.length].iter().copied()
    }

    pub fn set_direction(&mut self, direction: Direction) {
        if !direction.is_opposite(self.direction) {
            self.queued_direction = direction;
        }
    }

    pub fn toggle_pause(&mut self) {
        self.phase = match self.phase {
            Phase::Running => Phase::Paused,
            Phase::Paused => Phase::Running,
            Phase::GameOver => Phase::GameOver,
        };
    }

    pub fn tick(&mut self) -> Tick {
        if self.phase != Phase::Running {
            return Tick::GameOver;
        }

        self.direction = self.queued_direction;
        let next_head = self.body[0].stepped(self.direction);
        if !next_head.is_inside() || self.body[..self.length].contains(&next_head) {
            self.phase = Phase::GameOver;
            return Tick::GameOver;
        }

        let ate = next_head == self.food;
        let old_tail = self.body[self.length - 1];
        if ate && self.length < MAX_CELL_COUNT {
            self.length += 1;
        }
        self.body.copy_within(0..self.length - 1, 1);
        self.body[0] = next_head;

        if ate {
            self.score = self.score.saturating_add(10);
            self.place_food();
            Tick::Ate
        } else {
            Tick::Moved { old_tail }
        }
    }

    fn place_food(&mut self) {
        for _attempt in 0..MAX_CELL_COUNT {
            self.random_state = self
                .random_state
                .wrapping_mul(1_664_525)
                .wrapping_add(1_013_904_223);
            let index = self.random_state as usize % MAX_CELL_COUNT;
            let candidate = Cell::new(
                (index % COLUMN_COUNT as usize) as i16,
                (index / COLUMN_COUNT as usize) as i16,
            );
            if !self.body[..self.length].contains(&candidate) {
                self.food = candidate;
                return;
            }
        }
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playfield_and_grid_share_one_boundary_definition() {
        assert_eq!(COLUMN_COUNT, 30);
        assert_eq!(ROW_COUNT, 13);
        assert!(Cell::new(0, 0).rectangle().top_left.x > PLAYFIELD.top_left.x);
        assert!(!Cell::new(-1, 0).is_inside());
        assert!(!Cell::new(COLUMN_COUNT, 0).is_inside());
    }

    #[test]
    fn reverse_direction_is_ignored() {
        let mut game = Game::new();
        game.set_direction(Direction::Left);
        assert!(matches!(game.tick(), Tick::Moved { .. }));
        assert_eq!(
            game.body().next(),
            Some(Cell::new(COLUMN_COUNT / 2 + 1, ROW_COUNT / 2))
        );
    }
}
