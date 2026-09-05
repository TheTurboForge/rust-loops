//! Deterministic, bounded calibration engine for independently verified CA loops.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const CANONICAL_STATE_COUNT: u8 = 8;
pub const CANONICAL_COORDINATE_BASIS: &str = "Golly RLE active-bounds origin";
pub const BYL_STATE_COUNT: u8 = 6;
pub const BYL_COORDINATE_BASIS: &str = "active-bounds origin";

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

    fn lookup_index(self) -> usize {
        ((((usize::from(self.center.value()) * 8 + usize::from(self.north.value())) * 8
            + usize::from(self.east.value()))
            * 8
            + usize::from(self.south.value()))
            * 8)
            + usize::from(self.west.value())
    }
}

pub trait LocalRule {
    fn next_state(&self, neighborhood: Neighborhood) -> State;
    fn state_count(&self) -> u8;
    fn coordinate_basis(&self) -> &'static str;
}

#[derive(Clone, Debug)]
struct TransitionLookup {
    lookup: Box<[State]>,
    expanded_transition_count: usize,
}

#[derive(Clone, Debug)]
pub struct LangtonRule {
    transitions: TransitionLookup,
}

#[derive(Clone, Debug)]
pub struct BylRule {
    transitions: TransitionLookup,
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
    InvalidTransitionCount {
        expected: usize,
        actual: usize,
    },
    StateOutsideRule {
        state: u8,
        state_count: u8,
    },
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
            return Err(RuleError::InvalidTransitionCount {
                expected: 219,
                actual: document.transitions.len(),
            });
        }
        let transitions = parse_transitions(document, CANONICAL_STATE_COUNT)?;
        Self::from_base_transitions(transitions)
    }

    pub fn from_base_transitions(
        transitions: Vec<(Neighborhood, State)>,
    ) -> Result<Self, RuleError> {
        Ok(Self {
            transitions: TransitionLookup::from_base_transitions(
                transitions,
                CANONICAL_STATE_COUNT,
                |_| State::QUIESCENT,
            )?,
        })
    }

    pub fn next_state(&self, neighborhood: Neighborhood) -> State {
        LocalRule::next_state(self, neighborhood)
    }

    pub fn expanded_transition_count(&self) -> usize {
        self.transitions.expanded_transition_count
    }
}

impl LocalRule for LangtonRule {
    fn next_state(&self, neighborhood: Neighborhood) -> State {
        self.transitions.lookup[neighborhood.lookup_index()]
    }

    fn state_count(&self) -> u8 {
        CANONICAL_STATE_COUNT
    }

    fn coordinate_basis(&self) -> &'static str {
        CANONICAL_COORDINATE_BASIS
    }
}

impl BylRule {
    pub fn golly_3_3_profile() -> Result<Self, RuleError> {
        let document: RuleDocument = serde_json::from_str(include_str!(
            "../data/byl-golly-3.3/transitions-cneswc.json"
        ))
        .map_err(|error| RuleError::Parse(error.to_string()))?;
        if document.transitions.len() != 144 {
            return Err(RuleError::InvalidTransitionCount {
                expected: 144,
                actual: document.transitions.len(),
            });
        }
        Self::from_base_transitions(parse_transitions(document, BYL_STATE_COUNT)?)
    }

    pub fn from_base_transitions(
        transitions: Vec<(Neighborhood, State)>,
    ) -> Result<Self, RuleError> {
        Ok(Self {
            transitions: TransitionLookup::from_base_transitions(
                transitions,
                BYL_STATE_COUNT,
                |center| center,
            )?,
        })
    }

    pub fn next_state(&self, neighborhood: Neighborhood) -> State {
        LocalRule::next_state(self, neighborhood)
    }

    pub fn expanded_transition_count(&self) -> usize {
        self.transitions.expanded_transition_count
    }
}

impl LocalRule for BylRule {
    fn next_state(&self, neighborhood: Neighborhood) -> State {
        for state in [
            neighborhood.center,
            neighborhood.north,
            neighborhood.east,
            neighborhood.south,
            neighborhood.west,
        ] {
            assert!(
                state.value() < BYL_STATE_COUNT,
                "state {} is outside Byl profile range 0..=5",
                state.value()
            );
        }
        self.transitions.lookup[neighborhood.lookup_index()]
    }

    fn state_count(&self) -> u8 {
        BYL_STATE_COUNT
    }

    fn coordinate_basis(&self) -> &'static str {
        BYL_COORDINATE_BASIS
    }
}

impl TransitionLookup {
    fn from_base_transitions(
        transitions: Vec<(Neighborhood, State)>,
        state_count: u8,
        fallback: impl Fn(State) -> State,
    ) -> Result<Self, RuleError> {
        let mut expanded = BTreeMap::new();
        let mut declared = BTreeMap::new();
        for (base, next) in transitions {
            for state in [
                base.center,
                base.north,
                base.east,
                base.south,
                base.west,
                next,
            ] {
                if state.value() >= state_count {
                    return Err(RuleError::StateOutsideRule {
                        state: state.value(),
                        state_count,
                    });
                }
            }
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
        let mut lookup = (0..8usize.pow(5))
            .map(|index| {
                let center = State((index / 8usize.pow(4)) as u8);
                fallback(center)
            })
            .collect::<Vec<_>>();
        for (neighborhood, next) in &expanded {
            lookup[neighborhood.lookup_index()] = *next;
        }
        Ok(Self {
            lookup: lookup.into_boxed_slice(),
            expanded_transition_count: expanded.len(),
        })
    }
}

fn parse_transitions(
    document: RuleDocument,
    state_count: u8,
) -> Result<Vec<(Neighborhood, State)>, RuleError> {
    let state = |value: u8| -> Result<State, RuleError> {
        let state = State::try_from(value).map_err(|error| RuleError::InvalidState(error.0))?;
        if value >= state_count {
            Err(RuleError::StateOutsideRule {
                state: value,
                state_count,
            })
        } else {
            Ok(state)
        }
    };
    document
        .transitions
        .into_iter()
        .map(|item| {
            Ok((
                Neighborhood {
                    center: state(item.center)?,
                    north: state(item.north)?,
                    east: state(item.east)?,
                    south: state(item.south)?,
                    west: state(item.west)?,
                },
                state(item.next)?,
            ))
        })
        .collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DenseGrid {
    width: usize,
    height: usize,
    cells: Vec<State>,
}

/// Exact synchronous runner that owns two reusable dense buffers.
#[derive(Clone, Debug)]
pub struct DenseRunner {
    current: DenseGrid,
    next: DenseGrid,
    active_cell_count: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub struct StepStats {
    pub active_cells_before_step: usize,
    pub cell_evaluations: usize,
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
    pub fn step<R: LocalRule + ?Sized>(&self, rule: &R) -> Self {
        let mut runner = DenseRunner::new(self.clone());
        runner.step(rule);
        runner.into_grid()
    }
    pub fn run<R: LocalRule + ?Sized>(&self, rule: &R, generations: u64) -> Self {
        let mut runner = DenseRunner::new(self.clone());
        for _ in 0..generations {
            runner.step(rule);
        }
        runner.into_grid()
    }

    pub fn active_cell_count(&self) -> usize {
        self.cells
            .iter()
            .filter(|state| **state != State::QUIESCENT)
            .count()
    }
}

impl DenseRunner {
    pub fn new(current: DenseGrid) -> Self {
        let active_cell_count = current.active_cell_count();
        let next =
            DenseGrid::new(current.width, current.height).expect("existing dimensions are valid");
        Self {
            current,
            next,
            active_cell_count,
        }
    }

    pub fn step<R: LocalRule + ?Sized>(&mut self, rule: &R) -> StepStats {
        let mut next_active = 0usize;
        for y in 0..self.current.height {
            let row = y * self.current.width;
            for x in 0..self.current.width {
                let index = row + x;
                let north = if y == 0 {
                    State::QUIESCENT
                } else {
                    self.current.cells[index - self.current.width]
                };
                let east = if x + 1 == self.current.width {
                    State::QUIESCENT
                } else {
                    self.current.cells[index + 1]
                };
                let south = if y + 1 == self.current.height {
                    State::QUIESCENT
                } else {
                    self.current.cells[index + self.current.width]
                };
                let west = if x == 0 {
                    State::QUIESCENT
                } else {
                    self.current.cells[index - 1]
                };
                let next = rule.next_state(Neighborhood {
                    center: self.current.cells[index],
                    north,
                    east,
                    south,
                    west,
                });
                next_active += usize::from(next != State::QUIESCENT);
                self.next.cells[index] = next;
            }
        }
        std::mem::swap(&mut self.current, &mut self.next);
        let stats = StepStats {
            active_cells_before_step: self.active_cell_count,
            cell_evaluations: self.current.cells.len(),
        };
        self.active_cell_count = next_active;
        stats
    }

    pub fn grid(&self) -> &DenseGrid {
        &self.current
    }
    pub fn into_grid(self) -> DenseGrid {
        self.current
    }
}

/// Exact sparse-frontier representation for a finite quiescent world.
/// Every active cell and its von Neumann neighbors is evaluated each generation;
/// all remaining neighborhoods are quiescent and therefore remain quiescent.
#[derive(Clone, Debug)]
pub struct SparseFrontierGrid {
    width: usize,
    height: usize,
    cells: BTreeMap<usize, State>,
}

impl SparseFrontierGrid {
    pub fn from_dense(grid: &DenseGrid) -> Self {
        let cells = grid
            .cells
            .iter()
            .enumerate()
            .filter_map(|(index, state)| (*state != State::QUIESCENT).then_some((index, *state)))
            .collect();
        Self {
            width: grid.width,
            height: grid.height,
            cells,
        }
    }

    pub fn active_cell_count(&self) -> usize {
        self.cells.len()
    }

    fn at_or_quiescent(&self, x: isize, y: isize) -> State {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            return State::QUIESCENT;
        }
        self.cells
            .get(&(y as usize * self.width + x as usize))
            .copied()
            .unwrap_or(State::QUIESCENT)
    }

    fn candidates(&self) -> BTreeSet<usize> {
        let mut candidates = BTreeSet::new();
        for index in self.cells.keys().copied() {
            let x = index % self.width;
            let y = index / self.width;
            candidates.insert(index);
            if x > 0 {
                candidates.insert(index - 1);
            }
            if x + 1 < self.width {
                candidates.insert(index + 1);
            }
            if y > 0 {
                candidates.insert(index - self.width);
            }
            if y + 1 < self.height {
                candidates.insert(index + self.width);
            }
        }
        candidates
    }

    pub fn step<R: LocalRule + ?Sized>(&self, rule: &R) -> (Self, StepStats) {
        let candidates = self.candidates();
        let mut cells = BTreeMap::new();
        for index in &candidates {
            let x = index % self.width;
            let y = index / self.width;
            let next = rule.next_state(Neighborhood {
                center: self.at_or_quiescent(x as isize, y as isize),
                north: self.at_or_quiescent(x as isize, y as isize - 1),
                east: self.at_or_quiescent(x as isize + 1, y as isize),
                south: self.at_or_quiescent(x as isize, y as isize + 1),
                west: self.at_or_quiescent(x as isize - 1, y as isize),
            });
            if next != State::QUIESCENT {
                cells.insert(*index, next);
            }
        }
        (
            Self {
                width: self.width,
                height: self.height,
                cells,
            },
            StepStats {
                active_cells_before_step: self.cells.len(),
                cell_evaluations: candidates.len(),
            },
        )
    }

    pub fn run<R: LocalRule + ?Sized>(&self, rule: &R, generations: u64) -> Self {
        let mut current = self.clone();
        for _ in 0..generations {
            current = current.step(rule).0;
        }
        current
    }

    pub fn to_dense(&self) -> DenseGrid {
        let mut grid =
            DenseGrid::new(self.width, self.height).expect("stored dimensions are valid");
        for (index, state) in &self.cells {
            grid.cells[*index] = *state;
        }
        grid
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

#[derive(Clone, Debug)]
pub struct BylSeed {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
}

impl BylSeed {
    pub fn golly_3_3_profile() -> Result<Self, RuleError> {
        let document: SeedDocument =
            serde_json::from_str(include_str!("../data/byl-golly-3.3/seed.json"))
                .map_err(|error| RuleError::Parse(error.to_string()))?;
        if document.width != 4 || document.height != 4 || document.cells.len() != 12 {
            return Err(RuleError::Parse("Byl seed invariants failed".into()));
        }
        let mut states = BTreeSet::new();
        let mut positions = BTreeSet::new();
        for cell in &document.cells {
            State::try_from(cell.state).map_err(|error| RuleError::InvalidState(error.0))?;
            if cell.state == 0
                || cell.state >= BYL_STATE_COUNT
                || cell.x >= document.width
                || cell.y >= document.height
                || !positions.insert((cell.x, cell.y))
            {
                return Err(RuleError::Parse("invalid Byl seed cell".into()));
            }
            states.insert(cell.state);
        }
        if states != BTreeSet::from([1, 2, 3, 4, 5]) {
            return Err(RuleError::Parse(
                "unexpected Byl seed state signature".into(),
            ));
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
                State::try_from(cell.state).expect("validated Byl seed"),
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
        Self::from_grid_with_profile(
            grid,
            generation,
            CANONICAL_STATE_COUNT,
            CANONICAL_COORDINATE_BASIS,
        )
    }

    pub fn from_grid_for_rule<R: LocalRule + ?Sized>(
        grid: &DenseGrid,
        generation: u64,
        rule: &R,
    ) -> Self {
        Self::from_grid_with_profile(
            grid,
            generation,
            rule.state_count(),
            rule.coordinate_basis(),
        )
    }

    fn from_grid_with_profile(
        grid: &DenseGrid,
        generation: u64,
        state_count: u8,
        coordinate_basis: &str,
    ) -> Self {
        assert!((2..=CANONICAL_STATE_COUNT).contains(&state_count));
        assert!(
            grid.cells.iter().all(|state| state.value() < state_count),
            "grid contains a state outside the selected rule profile"
        );
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
        for state in 1..state_count {
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
            coordinate_basis: coordinate_basis.into(),
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
            cells, self.coordinate_basis
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

pub fn verify_byl_golly_profile(generation: u64, expected: &Snapshot) -> Result<(), String> {
    let rule = BylRule::golly_3_3_profile().map_err(|error| error.to_string())?;
    let seed = BylSeed::golly_3_3_profile().map_err(|error| error.to_string())?;
    let grid = seed
        .place_in(128, 128, 32, 32)
        .map_err(|error| error.to_string())?;
    let actual = Snapshot::from_grid_for_rule(&grid.run(&rule, generation), generation, &rule);
    if actual == *expected {
        Ok(())
    } else {
        Err(format!(
            "Byl Golly 3.3 profile mismatch at generation {generation}: expected hash {}, actual hash {}; expected {} cells, actual {}",
            expected.state_hash_sha256,
            actual.state_hash_sha256,
            expected.active_cell_count,
            actual.active_cell_count
        ))
    }
}
