//! Deterministic, bounded calibration engine for canonical Langton loops.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const CANONICAL_STATE_COUNT: u8 = 8;
pub const CANONICAL_COORDINATE_BASIS: &str = "Golly RLE active-bounds origin";

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct State(u8);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateError(pub u8);

impl fmt::Display for StateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "state {} is outside canonical range 0..=7",
            self.0
        )
    }
}

impl std::error::Error for StateError {}

impl TryFrom<u8> for State {
    type Error = StateError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value < CANONICAL_STATE_COUNT {
            Ok(Self(value))
        } else {
            Err(StateError(value))
        }
    }
}

impl State {
    pub const QUIESCENT: Self = Self(0);
    pub fn value(self) -> u8 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Neighborhood {
    pub center: State,
    pub north: State,
    pub east: State,
    pub south: State,
    pub west: State,
}

impl Neighborhood {
    fn clockwise(self) -> Self {
        Self {
            center: self.center,
            north: self.west,
            east: self.north,
            south: self.east,
            west: self.south,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LangtonRule {
    expanded: BTreeMap<Neighborhood, State>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuleError {
    InvalidState(u8),
    DuplicateBaseTransition {
        neighborhood: Neighborhood,
    },
    ConflictingRotation {
        neighborhood: Neighborhood,
        previous: State,
        next: State,
    },
    InvalidTransitionCount(usize),
    Parse(String),
}

impl fmt::Display for RuleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}
impl std::error::Error for RuleError {}

#[derive(Deserialize)]
struct RuleDocument {
    transitions: Vec<TransitionDocument>,
}
#[derive(Deserialize)]
struct TransitionDocument {
    center: u8,
    north: u8,
    east: u8,
    south: u8,
    west: u8,
    next: u8,
}

impl LangtonRule {
    pub fn canonical() -> Result<Self, RuleError> {
        let document: RuleDocument =
            serde_json::from_str(include_str!("../data/langton/transitions-cneswc.json"))
                .map_err(|error| RuleError::Parse(error.to_string()))?;
        if document.transitions.len() != 219 {
            return Err(RuleError::InvalidTransitionCount(
                document.transitions.len(),
            ));
        }
        let transitions = document
            .transitions
            .into_iter()
            .map(|item| {
                Ok((
                    Neighborhood {
                        center: State::try_from(item.center)
                            .map_err(|error| RuleError::InvalidState(error.0))?,
                        north: State::try_from(item.north)
                            .map_err(|error| RuleError::InvalidState(error.0))?,
                        east: State::try_from(item.east)
                            .map_err(|error| RuleError::InvalidState(error.0))?,
                        south: State::try_from(item.south)
                            .map_err(|error| RuleError::InvalidState(error.0))?,
                        west: State::try_from(item.west)
                            .map_err(|error| RuleError::InvalidState(error.0))?,
                    },
                    State::try_from(item.next).map_err(|error| RuleError::InvalidState(error.0))?,
                ))
            })
            .collect::<Result<Vec<_>, RuleError>>()?;
        Self::from_base_transitions(transitions)
    }

    pub fn from_base_transitions(
        transitions: Vec<(Neighborhood, State)>,
    ) -> Result<Self, RuleError> {
        let mut expanded = BTreeMap::new();
        let mut declared = BTreeMap::new();
        for (base, next) in transitions {
            if declared.insert(base, next).is_some() {
                return Err(RuleError::DuplicateBaseTransition { neighborhood: base });
            }
            let mut rotated = base;
            for _ in 0..4 {
                if let Some(previous) = expanded.insert(rotated, next)
                    && previous != next
                {
                    return Err(RuleError::ConflictingRotation {
                        neighborhood: rotated,
                        previous,
                        next,
                    });
                }
                rotated = rotated.clockwise();
            }
        }
        Ok(Self { expanded })
    }

    pub fn next_state(&self, neighborhood: Neighborhood) -> State {
        self.expanded
            .get(&neighborhood)
            .copied()
            .unwrap_or(State::QUIESCENT)
    }

    pub fn expanded_transition_count(&self) -> usize {
        self.expanded.len()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DenseGrid {
    width: usize,
    height: usize,
    cells: Vec<State>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GridError {
    InvalidDimensions,
    OutOfBounds { x: usize, y: usize },
}
impl fmt::Display for GridError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}
impl std::error::Error for GridError {}

impl DenseGrid {
    pub fn new(width: usize, height: usize) -> Result<Self, GridError> {
        if width == 0 || height == 0 {
            return Err(GridError::InvalidDimensions);
        }
        Ok(Self {
            width,
            height,
            cells: vec![
                State::QUIESCENT;
                width
                    .checked_mul(height)
                    .ok_or(GridError::InvalidDimensions)?
            ],
        })
    }
    pub fn width(&self) -> usize {
        self.width
    }
    pub fn height(&self) -> usize {
        self.height
    }
    fn index(&self, x: usize, y: usize) -> Result<usize, GridError> {
        if x < self.width && y < self.height {
            Ok(y * self.width + x)
        } else {
            Err(GridError::OutOfBounds { x, y })
        }
    }
    pub fn get(&self, x: usize, y: usize) -> Result<State, GridError> {
        Ok(self.cells[self.index(x, y)?])
    }
    pub fn set(&mut self, x: usize, y: usize, state: State) -> Result<(), GridError> {
        let index = self.index(x, y)?;
        self.cells[index] = state;
        Ok(())
    }
    fn at_or_quiescent(&self, x: isize, y: isize) -> State {
        if x < 0 || y < 0 {
            return State::QUIESCENT;
        }
        self.get(x as usize, y as usize).unwrap_or(State::QUIESCENT)
    }
    pub fn step(&self, rule: &LangtonRule) -> Self {
        let mut next = Self::new(self.width, self.height).expect("existing dimensions are valid");
        for y in 0..self.height {
            for x in 0..self.width {
                let x = x as isize;
                let y = y as isize;
                let neighborhood = Neighborhood {
                    center: self.at_or_quiescent(x, y),
                    north: self.at_or_quiescent(x, y - 1),
                    east: self.at_or_quiescent(x + 1, y),
                    south: self.at_or_quiescent(x, y + 1),
                    west: self.at_or_quiescent(x - 1, y),
                };
                next.cells[(y as usize) * self.width + x as usize] = rule.next_state(neighborhood);
            }
        }
        next
    }
    pub fn run(&self, rule: &LangtonRule, generations: u64) -> Self {
        let mut current = self.clone();
        for _ in 0..generations {
            current = current.step(rule);
        }
        current
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Cell {
    pub state: u8,
    pub x: usize,
    pub y: usize,
}
#[derive(Deserialize)]
struct SeedDocument {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
}

#[derive(Clone, Debug)]
pub struct CanonicalSeed {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
}

impl CanonicalSeed {
    pub fn canonical() -> Result<Self, RuleError> {
        let document: SeedDocument =
            serde_json::from_str(include_str!("../data/langton/seed.json"))
                .map_err(|error| RuleError::Parse(error.to_string()))?;
        if document.width != 15 || document.height != 10 || document.cells.len() != 86 {
            return Err(RuleError::Parse("canonical seed invariants failed".into()));
        }
        for cell in &document.cells {
            State::try_from(cell.state).map_err(|error| RuleError::InvalidState(error.0))?;
            if cell.state == 0 || cell.x >= document.width || cell.y >= document.height {
                return Err(RuleError::Parse("invalid canonical seed cell".into()));
            }
        }
        Ok(Self {
            width: document.width,
            height: document.height,
            cells: document.cells,
        })
    }
    pub fn active_cell_count(&self) -> usize {
        self.cells.len()
    }
    pub fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }
    pub fn place_in(
        &self,
        width: usize,
        height: usize,
        origin_x: usize,
        origin_y: usize,
    ) -> Result<DenseGrid, GridError> {
        let mut grid = DenseGrid::new(width, height)?;
        for cell in &self.cells {
            grid.set(
                origin_x
                    .checked_add(cell.x)
                    .ok_or(GridError::InvalidDimensions)?,
                origin_y
                    .checked_add(cell.y)
                    .ok_or(GridError::InvalidDimensions)?,
                State::try_from(cell.state).expect("validated canonical seed"),
            )?;
        }
        Ok(grid)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Snapshot {
    pub active_cell_count: usize,
    pub bounds: Bounds,
    pub cells: Vec<Cell>,
    pub coordinate_basis: String,
    pub generation: u64,
    pub state_hash_sha256: String,
    pub state_populations: BTreeMap<String, usize>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Bounds {
    pub min_x: usize,
    pub min_y: usize,
    pub width: usize,
    pub height: usize,
}

impl Snapshot {
    pub fn from_grid(grid: &DenseGrid, generation: u64) -> Self {
        let active = (0..grid.height)
            .flat_map(|y| {
                (0..grid.width).filter_map(move |x| {
                    let state = grid.cells[y * grid.width + x];
                    (state != State::QUIESCENT).then_some((x, y, state))
                })
            })
            .collect::<Vec<_>>();
        let min_x = active.iter().map(|(x, _, _)| *x).min().unwrap_or(0);
        let min_y = active.iter().map(|(_, y, _)| *y).min().unwrap_or(0);
        let max_x = active.iter().map(|(x, _, _)| *x).max().unwrap_or(min_x);
        let max_y = active.iter().map(|(_, y, _)| *y).max().unwrap_or(min_y);
        let mut cells = active
            .into_iter()
            .map(|(x, y, state)| Cell {
                state: state.value(),
                x: x - min_x,
                y: y - min_y,
            })
            .collect::<Vec<_>>();
        cells.sort_by_key(|cell| (cell.y, cell.x, cell.state));
        let mut populations = BTreeMap::new();
        for state in 1..CANONICAL_STATE_COUNT {
            populations.insert(
                state.to_string(),
                cells.iter().filter(|cell| cell.state == state).count(),
            );
        }
        let bounds = Bounds {
            min_x: 0,
            min_y: 0,
            width: if cells.is_empty() {
                0
            } else {
                max_x - min_x + 1
            },
            height: if cells.is_empty() {
                0
            } else {
                max_y - min_y + 1
            },
        };
        let mut snapshot = Self {
            active_cell_count: cells.len(),
            bounds,
            cells,
            coordinate_basis: CANONICAL_COORDINATE_BASIS.into(),
            generation,
            state_hash_sha256: String::new(),
            state_populations: populations,
        };
        snapshot.state_hash_sha256 = snapshot.state_hash();
        snapshot
    }
    pub fn canonical_state_json(&self) -> String {
        let cells = self
            .cells
            .iter()
            .map(|cell| {
                format!(
                    "    {{\n      \"state\": {},\n      \"x\": {},\n      \"y\": {}\n    }}",
                    cell.state, cell.x, cell.y
                )
            })
            .collect::<Vec<_>>()
            .join(",\n");
        format!(
            "{{\n  \"cells\": [\n{}\n  ],\n  \"coordinate_basis\": \"{}\"\n}}\n",
            cells, CANONICAL_COORDINATE_BASIS
        )
    }
    pub fn state_hash(&self) -> String {
        format!(
            "{:x}",
            Sha256::digest(self.canonical_state_json().as_bytes())
        )
    }
}

pub fn verify_canonical(generation: u64, expected: &Snapshot) -> Result<(), String> {
    let rule = LangtonRule::canonical().map_err(|error| error.to_string())?;
    let seed = CanonicalSeed::canonical().map_err(|error| error.to_string())?;
    let grid = seed
        .place_in(128, 128, 32, 32)
        .map_err(|error| error.to_string())?;
    let actual = Snapshot::from_grid(&grid.run(&rule, generation), generation);
    if actual == *expected {
        Ok(())
    } else {
        Err(format!(
            "canonical mismatch at generation {generation}: expected hash {}, actual hash {}; expected {} cells, actual {}",
            expected.state_hash_sha256,
            actual.state_hash_sha256,
            expected.active_cell_count,
            actual.active_cell_count
        ))
    }
}
