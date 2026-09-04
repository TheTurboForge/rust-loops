//! Deterministic, bounded calibration engine for canonical Langton loops.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const CANONICAL_STATE_COUNT: u8 = 8;
pub const CANONICAL_COORDINATE_BASIS: &str = "Golly RLE active-bounds origin";
pub const CHUNK_SIDE: usize = 32;
const CHUNK_AREA: usize = CHUNK_SIDE * CHUNK_SIDE;
const HALO_SIDE: usize = CHUNK_SIDE + 2;
const EDGE_NORTH: u8 = 1;
const EDGE_EAST: u8 = 2;
const EDGE_SOUTH: u8 = 4;
const EDGE_WEST: u8 = 8;

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

#[derive(Clone, Debug)]
pub struct LangtonRule {
    lookup: Box<[State]>,
    expanded_transition_count: usize,
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
    NonQuiescentBackground(State),
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
        let mut lookup = vec![State::QUIESCENT; 8usize.pow(5)];
        for (neighborhood, next) in &expanded {
            lookup[neighborhood.lookup_index()] = *next;
        }
        let quiescent = Neighborhood {
            center: State::QUIESCENT,
            north: State::QUIESCENT,
            east: State::QUIESCENT,
            south: State::QUIESCENT,
            west: State::QUIESCENT,
        };
        let background_next = lookup[quiescent.lookup_index()];
        if background_next != State::QUIESCENT {
            return Err(RuleError::NonQuiescentBackground(background_next));
        }
        Ok(Self {
            lookup: lookup.into_boxed_slice(),
            expanded_transition_count: expanded.len(),
        })
    }

    pub fn next_state(&self, neighborhood: Neighborhood) -> State {
        self.lookup[neighborhood.lookup_index()]
    }

    pub fn expanded_transition_count(&self) -> usize {
        self.expanded_transition_count
    }
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
    pub fn step(&self, rule: &LangtonRule) -> Self {
        let mut runner = DenseRunner::new(self.clone());
        runner.step(rule);
        runner.into_grid()
    }
    pub fn run(&self, rule: &LangtonRule, generations: u64) -> Self {
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

    pub fn step(&mut self, rule: &LangtonRule) -> StepStats {
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
    pub fn logical_storage_bytes(&self) -> usize {
        (self.current.cells.capacity() + self.next.cells.capacity()) * std::mem::size_of::<State>()
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

    /// Lower bound containing only stored key/state payloads, not B-tree node
    /// metadata, allocator overhead, or the temporary candidate set.
    pub fn logical_storage_bytes_lower_bound(&self) -> usize {
        self.cells.len() * (std::mem::size_of::<usize>() + std::mem::size_of::<State>())
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

    pub fn step(&self, rule: &LangtonRule) -> (Self, StepStats) {
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

    pub fn run(&self, rule: &LangtonRule, generations: u64) -> Self {
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

#[derive(Clone, Debug)]
struct Chunk {
    cells: Box<[State]>,
    active_cell_count: usize,
    active_edges: u8,
}

impl Chunk {
    fn empty(cells: Box<[State]>) -> Self {
        Self {
            cells,
            active_cell_count: 0,
            active_edges: 0,
        }
    }
}

/// Exact finite grid with sparse allocation of dense 32×32 chunks.
///
/// Chunks are directory-addressed rather than stored in an ordered per-cell
/// map. Only chunks containing active cells are retained between generations.
#[derive(Clone, Debug)]
pub struct ChunkedGrid {
    width: usize,
    height: usize,
    chunks_wide: usize,
    chunks_high: usize,
    chunks: Vec<Option<Chunk>>,
    active_cell_count: usize,
}

/// Additional accounting emitted by an exact chunked step.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub struct ChunkStepStats {
    pub step: StepStats,
    pub chunk_evaluations: usize,
    pub allocated_chunks_after_step: usize,
}

/// Reusable synchronous runner for [`ChunkedGrid`].
#[derive(Clone, Debug)]
pub struct ChunkedRunner {
    current: ChunkedGrid,
    next: ChunkedGrid,
    candidates: Vec<bool>,
    halo: Vec<State>,
    chunk_pool: Vec<Box<[State]>>,
}

impl ChunkedGrid {
    fn empty(width: usize, height: usize) -> Result<Self, GridError> {
        if width == 0 || height == 0 {
            return Err(GridError::InvalidDimensions);
        }
        let chunks_wide = width.div_ceil(CHUNK_SIDE);
        let chunks_high = height.div_ceil(CHUNK_SIDE);
        Ok(Self {
            width,
            height,
            chunks_wide,
            chunks_high,
            chunks: vec![None; chunks_wide * chunks_high],
            active_cell_count: 0,
        })
    }

    pub fn from_dense(grid: &DenseGrid) -> Self {
        let mut chunked = Self::empty(grid.width, grid.height).expect("dense dimensions are valid");
        for y in 0..grid.height {
            for x in 0..grid.width {
                let state = grid.cells[y * grid.width + x];
                if state == State::QUIESCENT {
                    continue;
                }
                let chunk_x = x / CHUNK_SIDE;
                let chunk_y = y / CHUNK_SIDE;
                let chunk_index = chunk_y * chunked.chunks_wide + chunk_x;
                let local_x = x % CHUNK_SIDE;
                let local_y = y % CHUNK_SIDE;
                let chunk = chunked.chunks[chunk_index].get_or_insert_with(|| {
                    Chunk::empty(vec![State::QUIESCENT; CHUNK_AREA].into_boxed_slice())
                });
                chunk.cells[local_y * CHUNK_SIDE + local_x] = state;
                chunk.active_cell_count += 1;
                chunk.active_edges |= edge_bits(local_x, local_y);
                chunked.active_cell_count += 1;
            }
        }
        chunked
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn active_cell_count(&self) -> usize {
        self.active_cell_count
    }

    pub fn allocated_chunk_count(&self) -> usize {
        self.chunks.iter().flatten().count()
    }

    pub fn logical_storage_bytes(&self) -> usize {
        self.chunks.capacity() * std::mem::size_of::<Option<Chunk>>()
            + self.allocated_chunk_count() * CHUNK_AREA * std::mem::size_of::<State>()
    }

    fn chunk_index(&self, chunk_x: usize, chunk_y: usize) -> usize {
        chunk_y * self.chunks_wide + chunk_x
    }

    fn valid_chunk_width(&self, chunk_x: usize) -> usize {
        (self.width - chunk_x * CHUNK_SIDE).min(CHUNK_SIDE)
    }

    fn valid_chunk_height(&self, chunk_y: usize) -> usize {
        (self.height - chunk_y * CHUNK_SIDE).min(CHUNK_SIDE)
    }

    fn state_at(&self, x: usize, y: usize) -> State {
        let chunk_x = x / CHUNK_SIDE;
        let chunk_y = y / CHUNK_SIDE;
        self.chunks[self.chunk_index(chunk_x, chunk_y)]
            .as_ref()
            .map(|chunk| chunk.cells[(y % CHUNK_SIDE) * CHUNK_SIDE + x % CHUNK_SIDE])
            .unwrap_or(State::QUIESCENT)
    }

    pub fn step(&self, rule: &LangtonRule) -> (Self, StepStats) {
        let mut runner = ChunkedRunner::new(self.clone());
        let stats = runner.step(rule).step;
        (runner.into_grid(), stats)
    }

    pub fn run(&self, rule: &LangtonRule, generations: u64) -> Self {
        let mut runner = ChunkedRunner::new(self.clone());
        for _ in 0..generations {
            runner.step(rule);
        }
        runner.into_grid()
    }

    pub fn to_dense(&self) -> DenseGrid {
        let mut grid =
            DenseGrid::new(self.width, self.height).expect("stored dimensions are valid");
        for y in 0..self.height {
            for x in 0..self.width {
                grid.cells[y * self.width + x] = self.state_at(x, y);
            }
        }
        grid
    }
}

fn edge_bits(local_x: usize, local_y: usize) -> u8 {
    let mut edges = 0;
    if local_y == 0 {
        edges |= EDGE_NORTH;
    }
    if local_x + 1 == CHUNK_SIDE {
        edges |= EDGE_EAST;
    }
    if local_y + 1 == CHUNK_SIDE {
        edges |= EDGE_SOUTH;
    }
    if local_x == 0 {
        edges |= EDGE_WEST;
    }
    edges
}

impl ChunkedRunner {
    pub fn new(current: ChunkedGrid) -> Self {
        let next = ChunkedGrid::empty(current.width, current.height)
            .expect("existing dimensions are valid");
        Self {
            candidates: vec![false; current.chunks.len()],
            halo: vec![State::QUIESCENT; HALO_SIDE * HALO_SIDE],
            current,
            next,
            chunk_pool: Vec::new(),
        }
    }

    fn recycle_next(&mut self) {
        for slot in &mut self.next.chunks {
            if let Some(chunk) = slot.take() {
                self.chunk_pool.push(chunk.cells);
            }
        }
        self.next.active_cell_count = 0;
        self.candidates.fill(false);
    }

    fn mark_candidates(&mut self) {
        for chunk_y in 0..self.current.chunks_high {
            for chunk_x in 0..self.current.chunks_wide {
                let index = self.current.chunk_index(chunk_x, chunk_y);
                let Some(chunk) = self.current.chunks[index].as_ref() else {
                    continue;
                };
                self.candidates[index] = true;
                if chunk.active_edges & EDGE_NORTH != 0 && chunk_y > 0 {
                    self.candidates[index - self.current.chunks_wide] = true;
                }
                if chunk.active_edges & EDGE_EAST != 0 && chunk_x + 1 < self.current.chunks_wide {
                    self.candidates[index + 1] = true;
                }
                if chunk.active_edges & EDGE_SOUTH != 0 && chunk_y + 1 < self.current.chunks_high {
                    self.candidates[index + self.current.chunks_wide] = true;
                }
                if chunk.active_edges & EDGE_WEST != 0 && chunk_x > 0 {
                    self.candidates[index - 1] = true;
                }
            }
        }
    }

    fn fill_halo(&mut self, chunk_x: usize, chunk_y: usize) {
        self.halo.fill(State::QUIESCENT);
        let copy_chunk = |halo: &mut [State], chunk: &Chunk, width: usize, height: usize| {
            for local_y in 0..height {
                let source = local_y * CHUNK_SIDE;
                let target = (local_y + 1) * HALO_SIDE + 1;
                halo[target..target + width].copy_from_slice(&chunk.cells[source..source + width]);
            }
        };
        let center_index = self.current.chunk_index(chunk_x, chunk_y);
        let valid_width = self.current.valid_chunk_width(chunk_x);
        let valid_height = self.current.valid_chunk_height(chunk_y);
        if let Some(center) = self.current.chunks[center_index].as_ref() {
            copy_chunk(&mut self.halo, center, valid_width, valid_height);
        }
        if chunk_y > 0 {
            let north_index = center_index - self.current.chunks_wide;
            if let Some(north) = self.current.chunks[north_index].as_ref() {
                let source_row = (CHUNK_SIDE - 1) * CHUNK_SIDE;
                self.halo[1..1 + valid_width]
                    .copy_from_slice(&north.cells[source_row..source_row + valid_width]);
            }
        }
        if chunk_y + 1 < self.current.chunks_high {
            let south_index = center_index + self.current.chunks_wide;
            if let Some(south) = self.current.chunks[south_index].as_ref() {
                let target = (valid_height + 1) * HALO_SIDE + 1;
                self.halo[target..target + valid_width]
                    .copy_from_slice(&south.cells[..valid_width]);
            }
        }
        if chunk_x > 0 {
            let west_index = center_index - 1;
            if let Some(west) = self.current.chunks[west_index].as_ref() {
                for local_y in 0..valid_height {
                    self.halo[(local_y + 1) * HALO_SIDE] =
                        west.cells[local_y * CHUNK_SIDE + CHUNK_SIDE - 1];
                }
            }
        }
        if chunk_x + 1 < self.current.chunks_wide {
            let east_index = center_index + 1;
            if let Some(east) = self.current.chunks[east_index].as_ref() {
                for local_y in 0..valid_height {
                    self.halo[(local_y + 1) * HALO_SIDE + valid_width + 1] =
                        east.cells[local_y * CHUNK_SIDE];
                }
            }
        }
    }

    fn empty_chunk_cells(&mut self) -> Box<[State]> {
        let mut cells = self
            .chunk_pool
            .pop()
            .unwrap_or_else(|| vec![State::QUIESCENT; CHUNK_AREA].into_boxed_slice());
        cells.fill(State::QUIESCENT);
        cells
    }

    fn evaluate_chunk(&mut self, rule: &LangtonRule, chunk_index: usize) -> usize {
        let chunk_x = chunk_index % self.current.chunks_wide;
        let chunk_y = chunk_index / self.current.chunks_wide;
        let valid_width = self.current.valid_chunk_width(chunk_x);
        let valid_height = self.current.valid_chunk_height(chunk_y);
        self.fill_halo(chunk_x, chunk_y);
        let mut chunk = Chunk::empty(self.empty_chunk_cells());
        for local_y in 0..valid_height {
            for local_x in 0..valid_width {
                let halo_index = (local_y + 1) * HALO_SIDE + local_x + 1;
                let next = rule.next_state(Neighborhood {
                    center: self.halo[halo_index],
                    north: self.halo[halo_index - HALO_SIDE],
                    east: self.halo[halo_index + 1],
                    south: self.halo[halo_index + HALO_SIDE],
                    west: self.halo[halo_index - 1],
                });
                chunk.cells[local_y * CHUNK_SIDE + local_x] = next;
                if next != State::QUIESCENT {
                    chunk.active_cell_count += 1;
                    chunk.active_edges |= edge_bits(local_x, local_y);
                }
            }
        }
        if chunk.active_cell_count == 0 {
            self.chunk_pool.push(chunk.cells);
        } else {
            self.next.active_cell_count += chunk.active_cell_count;
            self.next.chunks[chunk_index] = Some(chunk);
        }
        valid_width * valid_height
    }

    fn step_ordered(&mut self, rule: &LangtonRule, reverse: bool) -> ChunkStepStats {
        self.recycle_next();
        self.mark_candidates();
        let active_before = self.current.active_cell_count;
        let mut cell_evaluations = 0usize;
        let mut chunk_evaluations = 0usize;
        if reverse {
            for chunk_index in (0..self.candidates.len()).rev() {
                if self.candidates[chunk_index] {
                    cell_evaluations += self.evaluate_chunk(rule, chunk_index);
                    chunk_evaluations += 1;
                }
            }
        } else {
            for chunk_index in 0..self.candidates.len() {
                if self.candidates[chunk_index] {
                    cell_evaluations += self.evaluate_chunk(rule, chunk_index);
                    chunk_evaluations += 1;
                }
            }
        }
        std::mem::swap(&mut self.current, &mut self.next);
        ChunkStepStats {
            step: StepStats {
                active_cells_before_step: active_before,
                cell_evaluations,
            },
            chunk_evaluations,
            allocated_chunks_after_step: self.current.allocated_chunk_count(),
        }
    }

    pub fn step(&mut self, rule: &LangtonRule) -> ChunkStepStats {
        self.step_ordered(rule, false)
    }

    pub fn grid(&self) -> &ChunkedGrid {
        &self.current
    }

    pub fn logical_storage_bytes(&self) -> usize {
        self.current.logical_storage_bytes()
            + self.next.logical_storage_bytes()
            + self.chunk_pool.capacity() * std::mem::size_of::<Box<[State]>>()
            + self.chunk_pool.len() * CHUNK_AREA * std::mem::size_of::<State>()
            + self.candidates.capacity() * std::mem::size_of::<bool>()
            + self.halo.capacity() * std::mem::size_of::<State>()
    }

    pub fn into_grid(self) -> ChunkedGrid {
        self.current
    }
}

#[cfg(test)]
mod chunked_tests {
    use super::*;

    #[test]
    fn candidate_traversal_order_does_not_change_output() {
        let rule = LangtonRule::canonical().unwrap();
        let mut dense = DenseGrid::new(97, 71).unwrap();
        for (x, y, state) in [
            (0, 0, 1),
            (31, 16, 2),
            (32, 16, 3),
            (63, 32, 4),
            (64, 32, 5),
            (96, 70, 7),
        ] {
            dense.set(x, y, State::try_from(state).unwrap()).unwrap();
        }
        let initial = ChunkedGrid::from_dense(&dense);
        let mut forward = ChunkedRunner::new(initial.clone());
        let mut reverse = ChunkedRunner::new(initial);
        forward.step_ordered(&rule, false);
        reverse.step_ordered(&rule, true);
        assert_eq!(forward.grid().to_dense(), reverse.grid().to_dense());
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
