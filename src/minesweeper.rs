use anyhow::Result;
use itertools::iproduct;
use rand::prelude::*;
use serde::{Deserialize, Serialize};

/// Indicates some sort of error related to initialization and play on the gameboard
#[derive(Debug)]
#[allow(dead_code)]
pub enum Error {
    ExcessiveMines,
    InvalidCoordinates,
    IndexOutOfBounds,
    InvalidCascade,
    UnexpectedResult,
}

/// Represents the type of a square as to the presence of a mine
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SquareType {
    Empty,
    Mine,
}

/// Representation of a single minesweeper square.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Square {
    pub is_revealed: bool,
    pub is_flagged: bool,
    pub square_type: SquareType,
    pub numeral: u32,
}

impl Default for Square {
    fn default() -> Self {
        Square {
            is_revealed: false,
            is_flagged: false,
            numeral: 0,
            square_type: SquareType::Empty,
        }
    }
}

impl Square {
    pub fn default_mine() -> Self {
        Square {
            is_revealed: false,
            is_flagged: false,
            numeral: 0,
            square_type: SquareType::Mine,
        }
    }

    pub fn is_mine(&self) -> bool {
        self.square_type == SquareType::Mine
    }

    #[allow(dead_code)]
    pub fn print(&self) {
        if self.is_flagged {
            print!(" > ");
        } else if !self.is_revealed {
            print!(" - ");
        } else if self.is_mine() {
            print!(" X ");
        } else if self.numeral > 0 {
            print!(" {} ", self.numeral)
        } else {
            print!("   ");
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Default, Deserialize, Serialize)]
pub struct Coordinate {
    pub x: u32,
    pub y: u32,
}

impl From<(u32, u32)> for Coordinate {
    fn from(xy: (u32, u32)) -> Self {
        Coordinate { x: xy.0, y: xy.1 }
    }
}

impl Coordinate {
    #[allow(dead_code)]
    pub fn matches(&self, x: u32, y: u32) -> bool {
        self.x == x && self.y == y
    }

    pub fn distance(&self, coord: &Coordinate) -> f32 {
        ((self.x as f32 - coord.x as f32).powf(2.0) + (self.y as f32 - coord.y as f32).powf(2.0))
            .sqrt()
    }

    pub fn near(&self, coord: &Coordinate) -> bool {
        self.distance(coord) <= 1.5
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
pub enum RevealType {
    #[default]
    Reveal,
    RevealChord,
    Chord,
    Flag,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum PlayResult {
    Flagged(bool),
    Explosion(Coordinate), // Loss
    NoChange,
    Revealed(Coordinate),
    CascadedReveal(Vec<PlayResult>),
}

#[derive(Debug, Clone)]
/// Representation of a minesweeper game board
pub struct GameBoard {
    pub width: u32,
    pub height: u32,
    pub num_mines: u32,
    pub squares: Vec<Square>,
    pub is_populated: bool,
}

impl GameBoard {
    pub fn new(width: u32, height: u32) -> Self {
        GameBoard {
            width,
            height,
            num_mines: 0,
            squares: (0..width * height).map(|_| Square::default()).collect(),
            is_populated: false,
        }
    }

    #[allow(dead_code)]
    pub fn new_populated(width: u32, height: u32, num_mines: u32) -> Result<GameBoard, Error> {
        let mut gb = Self::new(width, height);
        gb.populate_mines(num_mines)?;
        gb.populate_numerals()?;
        Ok(gb)
    }

    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.squares = (0..self.width * self.height)
            .map(|_| Square::default())
            .collect();
    }

    #[allow(dead_code)]
    pub fn new_populated_around(
        width: u32,
        height: u32,
        num_mines: u32,
        keep_clear: Coordinate,
    ) -> Result<GameBoard, Error> {
        let mut gb = Self::new(width, height);
        gb.populate_mines_around(num_mines, Some(keep_clear))?;
        gb.populate_numerals()?;
        Ok(gb)
    }

    /// Convert x, y coordinate to vector index
    fn xy_to_idx(&self, x: u32, y: u32) -> u32 {
        y * self.width + x
    }

    fn coordinate_to_idx(&self, coord: &Coordinate) -> u32 {
        self.xy_to_idx(coord.x, coord.y)
    }

    #[allow(dead_code)]
    fn idx_to_xy(&self, idx: u32) -> Result<Coordinate, Error> {
        if idx as usize > self.squares.len() - 1 {
            return Err(Error::IndexOutOfBounds);
        }

        Ok(Coordinate {
            x: idx % self.width,
            y: idx / self.width,
        })
    }

    fn get_square_by_idx(&self, idx: u32) -> Result<Square, Error> {
        if idx as usize >= self.squares.len() {
            Err(Error::InvalidCoordinates)
        } else {
            Ok(self.squares[idx as usize])
        }
    }

    pub fn get_square(&self, x: u32, y: u32) -> Result<Square, Error> {
        if x >= self.width || y >= self.height {
            Err(Error::InvalidCoordinates)
        } else {
            self.get_square_by_idx(self.xy_to_idx(x, y))
        }
    }

    pub fn get_square_by_coordinate(&self, coord: &Coordinate) -> Result<Square, Error> {
        self.get_square(coord.x, coord.y)
    }

    /// Determines whether a square contains a mine, allowing for negative
    /// and invalid coordinates.
    fn is_mine_protected(&self, x: i32, y: i32) -> bool {
        if x < 0 {
            return false;
        }
        if y < 0 {
            return false;
        }

        match self.get_square(x as u32, y as u32) {
            Ok(sqr) => sqr.is_mine(),
            _ => false,
        }
    }

    fn is_flagged_protected(&self, x: i32, y: i32) -> bool {
        if x < 0 {
            return false;
        }
        if y < 0 {
            return false;
        }

        match self.get_square(x as u32, y as u32) {
            Ok(sqr) => sqr.is_flagged,
            _ => false,
        }
    }

    fn flagged_neighbor_count(&self, x: u32, y: u32) -> Result<u32, Error> {
        if x >= self.width || y >= self.height {
            Err(Error::InvalidCoordinates)
        } else {
            Ok(iproduct!(-1_i32..2_i32, -1_i32..2_i32)
                .map(|(dx, dy)| {
                    if self.is_flagged_protected(x as i32 + dx, y as i32 + dy) {
                        1
                    } else {
                        0
                    }
                })
                .collect::<Vec<u32>>()
                .into_iter()
                .sum())
        }
    }

    /// Determine how many mines a given square touches.
    fn mined_neighbor_count(&self, x: u32, y: u32) -> Result<u32, Error> {
        if x >= self.width || y >= self.height {
            Err(Error::InvalidCoordinates)
        } else {
            Ok(iproduct!(-1_i32..2_i32, -1_i32..2_i32)
                .map(|(dx, dy)| {
                    if self.is_mine_protected(x as i32 + dx, y as i32 + dy) {
                        1
                    } else {
                        0
                    }
                })
                .collect::<Vec<u32>>()
                .into_iter()
                .sum())
        }
    }

    fn gen_random_square_coordinates(&self) -> Coordinate {
        Coordinate {
            x: rand::thread_rng().gen_range(0..self.width),
            y: rand::thread_rng().gen_range(0..self.height),
        }
    }

    pub fn populate_mines_around(
        &mut self,
        num_mines: u32,
        keep_clear: Option<Coordinate>,
    ) -> Result<(), Error> {
        if num_mines > self.width * self.height {
            Err(Error::ExcessiveMines)
        } else {
            self.num_mines = num_mines;

            let mut mines_placed = 0;
            while mines_placed < num_mines {
                let random_coord = self.gen_random_square_coordinates();

                if let Some(kc) = &keep_clear {
                    let sqr = self.get_square_by_coordinate(&random_coord)?;
                    if !kc.near(&random_coord) && !sqr.is_mine() {
                        let idx = self.coordinate_to_idx(&random_coord);
                        self.squares[idx as usize] = Square::default_mine();
                        mines_placed += 1;
                    }
                } else {
                    let idx = self.coordinate_to_idx(&random_coord);
                    self.squares[idx as usize] = Square::default_mine();
                    mines_placed += 1;
                }
            }
            self.is_populated = true;
            Ok(())
        }
    }

    pub fn populate_mines(&mut self, num_mines: u32) -> Result<(), Error> {
        self.populate_mines_around(num_mines, None)
    }

    pub fn populate_numerals(&mut self) -> Result<(), Error> {
        iproduct!(0..self.width, 0..self.height).for_each(|(x, y)| {
            let idx = self.xy_to_idx(x, y);
            self.squares[idx as usize].numeral = self.mined_neighbor_count(x, y).unwrap_or(0);
        });

        Ok(())
    }

    #[allow(dead_code)]
    pub fn print(&self) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.squares[self.xy_to_idx(x, y) as usize].print();
            }
            println!();
        }
    }

    /// Toggles the flagged state of a square.
    /// Returns the updated flagged state of the square.
    ///
    /// A revealed square cannot be flagged
    ///
    pub fn flag(&mut self, x: u32, y: u32) -> Result<PlayResult, Error> {
        if x >= self.width || y >= self.height {
            Err(Error::InvalidCoordinates)
        } else {
            let idx = self.xy_to_idx(x, y);
            let sqr = self.get_square_by_idx(idx)?;
            if !sqr.is_revealed {
                self.squares[idx as usize].is_flagged = !sqr.is_flagged;
                Ok(PlayResult::Flagged(self.squares[idx as usize].is_flagged))
            } else {
                Ok(PlayResult::NoChange) // Maybe return false instead?
            }
        }
    }

    pub fn cascade_from(&mut self, x: u32, y: u32) -> Result<PlayResult, Error> {
        if x >= self.width || y >= self.height {
            return Err(Error::InvalidCoordinates);
        }

        let idx = self.xy_to_idx(x, y);

        if self.squares[idx as usize].is_mine()
            || self.squares[idx as usize].is_flagged
            || self.squares[idx as usize].numeral > 0
        {
            return Err(Error::InvalidCascade);
        }
        self.squares[idx as usize].is_revealed = true;

        let results = iproduct!(-1_i32..2_i32, -1_i32..2_i32)
            .map(|(dx, dy)| self.reveal_protected(x as i32 + dx, y as i32 + dy))
            .collect::<Vec<PlayResult>>();

        Ok(PlayResult::CascadedReveal(results))
    }

    // Defines a single square reveal
    pub fn reveal(&mut self, x: u32, y: u32) -> Result<PlayResult, Error> {
        if x >= self.width || y >= self.height {
            Err(Error::InvalidCoordinates)
        } else {
            let idx = self.xy_to_idx(x, y);
            let sqr = self.get_square_by_idx(idx)?;

            if sqr.is_mine() && !sqr.is_flagged {
                // If the square is a mine and it's not flagged (unprotected)
                self.squares[idx as usize].is_revealed = true;
                Ok(PlayResult::Explosion(Coordinate::from((x, y))))
            } else if !sqr.is_mine() && !sqr.is_flagged && !sqr.is_revealed {
                // if the square is not a mine, is unflagged, and is unrevealed
                if self.squares[idx as usize].numeral == 0 {
                    // If it's a non-numeral square, we can auto-chord it
                    self.cascade_from(x, y)
                } else {
                    // Otherwise, reveal the single square, and set it as so
                    self.squares[idx as usize].is_revealed = true;
                    Ok(PlayResult::Revealed(Coordinate::from((x, y))))
                }
            } else {
                // Otherwise no change (user tried to reveal an already revealed square)
                Ok(PlayResult::NoChange)
            }
        }
    }

    fn reveal_protected(&mut self, x: i32, y: i32) -> PlayResult {
        if x < 0 {
            return PlayResult::NoChange;
        }
        if y < 0 {
            return PlayResult::NoChange;
        }

        self.reveal(x as u32, y as u32)
            .unwrap_or(PlayResult::NoChange)
    }

    /// Determine whether a given square can be chorded.
    ///
    /// Has a zero numeral: yes
    /// Has same number of neighbors flagged as numeral: yes
    /// Does *not* determine if the square can be *safely* chorded
    /// If the number of flagged neighbors is greated than the numeral, then
    ///     there is an abiguity and the square cannot be chorded.
    pub fn can_chord_square(&self, x: u32, y: u32) -> Result<bool, Error> {
        if x >= self.width || y >= self.height {
            return Err(Error::InvalidCoordinates);
        }
        let sqr = self.get_square(x, y)?;

        // Is it a blank square or does the numeral match the number of flagged neighbors
        if sqr.numeral == 0 || sqr.numeral == self.flagged_neighbor_count(x, y)? {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Executes a 'chord' reveal on the requested square.
    pub fn chord(&mut self, x: u32, y: u32) -> Result<PlayResult, Error> {
        if x >= self.width || y >= self.height {
            Err(Error::InvalidCoordinates)
        } else if !self.can_chord_square(x, y)? {
            Ok(PlayResult::NoChange)
        } else {
            let results = iproduct!(-1_i32..2_i32, -1_i32..2_i32)
                .map(|(dx, dy)| self.reveal_protected(x as i32 + dx, y as i32 + dy))
                .collect::<Vec<PlayResult>>();

            Ok(PlayResult::CascadedReveal(results))
        }
    }

    /// Performs a unified reveal then chord in the same coordinate
    pub fn revealchord(&mut self, x: u32, y: u32) -> Result<PlayResult, Error> {
        let rv = self.reveal(x, y)?;
        let rc = self.chord(x, y)?;

        if let PlayResult::CascadedReveal(mut v) = rc {
            v.push(rv);
            Ok(PlayResult::CascadedReveal(v))
        } else if PlayResult::NoChange == rc {
            Ok(PlayResult::CascadedReveal(vec![rv]))
        } else {
            Err(Error::UnexpectedResult)
        }
    }

    /// Determine if the board is in a winning configuration.
    ///
    /// Conditions
    /// - All non-mine squares are revealed (mined need not be flagged)
    #[allow(dead_code)]
    pub fn is_win_configuration(&self) -> bool {
        self.squares
            .clone()
            .into_iter()
            .map(|s| if !s.is_mine() && !s.is_revealed { 1 } else { 0 })
            .collect::<Vec<u32>>()
            .into_iter()
            .sum::<u32>()
            == 0_u32
    }

    #[allow(dead_code)]
    pub fn is_loss_configuration(&self) -> bool {
        self.squares
            .clone()
            .into_iter()
            .map(|s| if s.is_mine() && s.is_revealed { 1 } else { 0 })
            .collect::<Vec<u32>>()
            .into_iter()
            .sum::<u32>()
            > 0_u32
    }

    pub fn play(&mut self, x: u32, y: u32, reveal_type: RevealType) -> Result<PlayResult, Error> {
        match reveal_type {
            RevealType::Flag => self.flag(x, y),
            RevealType::Reveal => self.reveal(x, y),
            RevealType::Chord => self.chord(x, y),
            RevealType::RevealChord => self.revealchord(x, y),
        }
    }

    pub fn num_flags(&self) -> u32 {
        self.squares
            .clone()
            .into_iter()
            .map(|s| if s.is_flagged { 1 } else { 0 })
            .collect::<Vec<u32>>()
            .into_iter()
            .sum::<u32>()
    }

    pub fn num_revealed(&self) -> u32 {
        self.squares
            .clone()
            .into_iter()
            .map(|s| if s.is_revealed { 1 } else { 0 })
            .collect::<Vec<u32>>()
            .into_iter()
            .sum::<u32>()
    }

    // Don't cheat
    #[allow(dead_code)]
    pub fn flag_all_mines(&mut self) {
        for sqr in self.squares.iter_mut() {
            sqr.is_flagged = sqr.is_mine();
        }
    }

    #[allow(dead_code)]
    pub fn reset_existing(&mut self) {
        for sqr in self.squares.iter_mut() {
            sqr.is_flagged = false;
            sqr.is_revealed = false;
        }
    }
}
