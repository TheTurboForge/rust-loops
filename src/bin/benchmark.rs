//! JSONL benchmark harness for exact CA representation comparisons.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;
use std::time::Instant;

use rust_loops::{
    BylRule, BylSeed, CHUNK_SIDE, CanonicalSeed, ChunkedGrid, ChunkedRunner, DenseGrid,
    DenseRunner, LangtonRule, LocalRule, Neighborhood, Snapshot, SparseFrontierGrid, State,
    StepStats,
};
use serde::Serialize;

const BENCHMARK_SCHEMA_VERSION: u8 = 4;
const LOGICAL_REGION_SIDE: usize = CHUNK_SIDE;
const IDENTITY_RULE_SHA256: &str =
    "e62113ac6654662762d93e3ab5ea38f5e7d6501a5477a2eff9df1187d4728736";
const SYNTHETIC_GENERATOR_SHA256: &str =
    "35214b8326340c22dea33898a9bf849d2066cf162b926855120c115645faef8a";

#[derive(Clone, Copy, Eq, PartialEq)]
enum RuleSelection {
    Langton,
    BylGolly33,
}

#[derive(Clone, Copy)]
enum RepresentationSelection {
    Dense,
    Sparse,
    Chunked,
    Both,
    All,
}

#[derive(Clone, Copy)]
enum Representation {
    Dense,
    Sparse,
    Chunked,
}

impl Representation {
    fn name(self) -> &'static str {
        match self {
            Self::Dense => "dense",
            Self::Sparse => "sparse",
            Self::Chunked => "chunked-32",
        }
    }
}

impl RepresentationSelection {
    fn representations(self) -> &'static [Representation] {
        match self {
            Self::Dense => &[Representation::Dense],
            Self::Sparse => &[Representation::Sparse],
            Self::Chunked => &[Representation::Chunked],
            Self::Both => &[Representation::Dense, Representation::Sparse],
            Self::All => &[
                Representation::Dense,
                Representation::Sparse,
                Representation::Chunked,
            ],
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Layout {
    Uniform,
    Clustered,
}

impl Layout {
    fn name(self) -> &'static str {
        match self {
            Self::Uniform => "uniform",
            Self::Clustered => "clustered",
        }
    }
}

#[derive(Clone, Copy)]
enum Workload {
    Canonical { generation: u64 },
    SyntheticLangton { density_ppm: u32, seed: u64 },
    SyntheticIdentity { density_ppm: u32, seed: u64 },
}

impl Workload {
    fn synthetic_seed(self) -> Option<u64> {
        match self {
            Self::Canonical { .. } => None,
            Self::SyntheticLangton { seed, .. } | Self::SyntheticIdentity { seed, .. } => {
                Some(seed)
            }
        }
    }
}

struct Config {
    rule: RuleSelection,
    workload: Workload,
    world: usize,
    representation: RepresentationSelection,
    repetitions: u32,
    warmup: u64,
    timed_generations: u64,
    layout: Layout,
    execution_order: u32,
}

enum RuleProfile {
    Langton(LangtonRule),
    BylGolly33(BylRule),
    Identity,
}

impl RuleProfile {
    fn load(selection: RuleSelection, workload: Workload) -> Result<Self, String> {
        match workload {
            Workload::SyntheticIdentity { .. } => Ok(Self::Identity),
            Workload::Canonical { .. } | Workload::SyntheticLangton { .. } => match selection {
                RuleSelection::Langton => LangtonRule::canonical()
                    .map(Self::Langton)
                    .map_err(|error| error.to_string()),
                RuleSelection::BylGolly33 => BylRule::golly_3_3_profile()
                    .map(Self::BylGolly33)
                    .map_err(|error| error.to_string()),
            },
        }
    }

    fn id(&self) -> &'static str {
        match self {
            Self::Langton(_) => "langton-1984-canonical",
            Self::BylGolly33(_) => "byl-1989-golly-3.3-executable-reference",
            Self::Identity => "benchmark-identity-v1",
        }
    }

    fn rule_fixture_sha256(&self) -> &'static str {
        match self {
            Self::Langton(_) => "c0ca8e6c9218ddd2603905b6dbc5f3170a7f3f32405b8ad5290b1e396018ee92",
            Self::BylGolly33(_) => {
                "8813815e3af17aa71ce351bfa69358b3eb64ecf38eb44f739142e3f4595d84be"
            }
            Self::Identity => IDENTITY_RULE_SHA256,
        }
    }

    fn seed_fixture_sha256(&self) -> &'static str {
        match self {
            Self::Langton(_) => "46faf1f1c0c4b966f139c59134cc00697b45f4156a1f51a6c4fa2e3d27d5bda1",
            Self::BylGolly33(_) => {
                "0854641da00edc65974ac7a79d79b7c5fabf171946bffdbf1b0ba38a9662892f"
            }
            Self::Identity => SYNTHETIC_GENERATOR_SHA256,
        }
    }
}

impl LocalRule for RuleProfile {
    fn next_state(&self, neighborhood: Neighborhood) -> State {
        match self {
            Self::Langton(rule) => rule.next_state(neighborhood),
            Self::BylGolly33(rule) => rule.next_state(neighborhood),
            Self::Identity => neighborhood.center,
        }
    }

    fn state_count(&self) -> u8 {
        match self {
            Self::Langton(rule) => rule.state_count(),
            Self::BylGolly33(rule) => rule.state_count(),
            Self::Identity => 8,
        }
    }

    fn coordinate_basis(&self) -> &'static str {
        match self {
            Self::Langton(rule) => rule.coordinate_basis(),
            Self::BylGolly33(rule) => rule.coordinate_basis(),
            Self::Identity => "finite-grid top-left origin",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct SpatialMetrics {
    active_bounds: Option<[usize; 4]>,
    active_bounds_area: usize,
    occupied_logical_regions_32: usize,
    touches_boundary: bool,
}

#[derive(Serialize)]
struct BenchmarkRecord {
    active_cell_updates: u64,
    actual_timed_density_ppm: f64,
    allocated_bytes_estimate: usize,
    allocated_chunks_final: Option<usize>,
    allocated_chunks_peak: Option<usize>,
    allocation_accounting: &'static str,
    baseline_peak_resident_bytes: Option<u64>,
    baseline_resident_bytes: Option<u64>,
    benchmark_schema_version: u8,
    boundary: &'static str,
    cell_evaluations: u64,
    chunk_evaluations: Option<u64>,
    chunk_side: Option<usize>,
    code_revision: String,
    coordinate_basis: &'static str,
    cpu_model: Option<String>,
    current_resident_bytes: Option<u64>,
    current_resident_delta_bytes: Option<u64>,
    data_movement_status: &'static str,
    elapsed_nanoseconds: u128,
    energy_status: String,
    execution_order: u32,
    final_active_cell_count: usize,
    final_generation: u64,
    final_spatial: SpatialMetrics,
    final_state_populations: BTreeMap<String, usize>,
    generations_per_second: f64,
    hardware_counter_status: String,
    in_process_order: usize,
    initial_active_cell_count: usize,
    initial_generation: u64,
    initial_state_hash_sha256: String,
    layout: &'static str,
    logical_region_side: usize,
    logical_storage_bytes: usize,
    os: &'static str,
    output_state_hash_sha256: String,
    peak_resident_bytes: Option<u64>,
    peak_resident_delta_bytes: Option<u64>,
    repetition: u32,
    representation: &'static str,
    representation_fixture_ownership: &'static str,
    resident_delta_saturated: bool,
    resident_memory_method: &'static str,
    rule_fixture_sha256: &'static str,
    rule_profile: &'static str,
    rust_version: String,
    seed_fixture_sha256: &'static str,
    synthetic_seed: Option<u64>,
    timed_active_cell_max: usize,
    timed_active_cell_mean: f64,
    timed_active_cell_min: usize,
    timed_generations: u64,
    timed_start_active_cell_count: usize,
    timed_start_generation: u64,
    timed_start_spatial: SpatialMetrics,
    timed_start_state_hash_sha256: String,
    updates_per_second: f64,
    useful_active_updates_per_second: f64,
    workload: String,
    world: [usize; 2],
}

struct RunResult {
    final_grid: DenseGrid,
    active_cell_updates: u64,
    cell_evaluations: u64,
    chunk_evaluations: Option<u64>,
    elapsed_nanoseconds: u128,
    logical_storage_bytes: usize,
    allocation_accounting: &'static str,
    allocated_chunks_final: Option<usize>,
    allocated_chunks_peak: Option<usize>,
    timed_active_cell_min: usize,
    timed_active_cell_max: usize,
    timed_start_active_cell_count: usize,
    timed_start_spatial: SpatialMetrics,
    timed_start_state_hash_sha256: String,
    current_resident_bytes: Option<u64>,
    peak_resident_bytes: Option<u64>,
}

fn usage() -> &'static str {
    "usage: benchmark --workload canonical|synthetic|identity --world <N> --representation dense|sparse|chunked|both|all --repetitions <N> --warmup <N> --timed-generations <N> [--rule langton|byl-golly-3.3] [--generation <N>] [--density-ppm <N>] [--seed <N>] [--layout uniform|clustered] [--execution-order <N>]"
}

fn parse_arguments() -> Result<Config, String> {
    let mut rule = RuleSelection::Langton;
    let mut workload = None;
    let mut world = None;
    let mut representation = None;
    let mut repetitions = None;
    let mut warmup = None;
    let mut timed_generations = None;
    let mut generation = 0u64;
    let mut density_ppm = None;
    let mut seed = 0x5eed_cafe_u64;
    let mut layout = Layout::Uniform;
    let mut execution_order = 0u32;
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        let mut value = || arguments.next().ok_or_else(|| usage().to_owned());
        match argument.as_str() {
            "--rule" => {
                rule = match value()?.as_str() {
                    "langton" => RuleSelection::Langton,
                    "byl-golly-3.3" => RuleSelection::BylGolly33,
                    _ => return Err(usage().to_owned()),
                }
            }
            "--workload" => workload = Some(value()?),
            "--world" => world = Some(value()?.parse().map_err(|_| usage().to_owned())?),
            "--representation" => {
                representation = Some(match value()?.as_str() {
                    "dense" => RepresentationSelection::Dense,
                    "sparse" => RepresentationSelection::Sparse,
                    "chunked" => RepresentationSelection::Chunked,
                    "both" => RepresentationSelection::Both,
                    "all" => RepresentationSelection::All,
                    _ => return Err(usage().to_owned()),
                })
            }
            "--repetitions" => {
                repetitions = Some(value()?.parse().map_err(|_| usage().to_owned())?)
            }
            "--warmup" => warmup = Some(value()?.parse().map_err(|_| usage().to_owned())?),
            "--timed-generations" => {
                timed_generations = Some(value()?.parse().map_err(|_| usage().to_owned())?)
            }
            "--generation" => generation = value()?.parse().map_err(|_| usage().to_owned())?,
            "--density-ppm" => {
                density_ppm = Some(value()?.parse().map_err(|_| usage().to_owned())?)
            }
            "--seed" => seed = value()?.parse().map_err(|_| usage().to_owned())?,
            "--layout" => {
                layout = match value()?.as_str() {
                    "uniform" => Layout::Uniform,
                    "clustered" => Layout::Clustered,
                    _ => return Err(usage().to_owned()),
                }
            }
            "--execution-order" => {
                execution_order = value()?.parse().map_err(|_| usage().to_owned())?
            }
            "--help" | "-h" => return Err(usage().to_owned()),
            _ => return Err(usage().to_owned()),
        }
    }
    let workload = match workload.as_deref() {
        Some("canonical") => {
            if layout != Layout::Uniform {
                return Err("canonical workload does not accept --layout clustered".to_owned());
            }
            Workload::Canonical { generation }
        }
        Some("synthetic") if rule == RuleSelection::Langton => Workload::SyntheticLangton {
            density_ppm: density_ppm
                .ok_or_else(|| "synthetic workload requires --density-ppm".to_owned())?,
            seed,
        },
        Some("synthetic") => {
            return Err("synthetic workload is defined only for the Langton profile".to_owned());
        }
        Some("identity") if rule == RuleSelection::Langton => Workload::SyntheticIdentity {
            density_ppm: density_ppm
                .ok_or_else(|| "identity workload requires --density-ppm".to_owned())?,
            seed,
        },
        Some("identity") => {
            return Err("identity workload does not accept a scientific rule profile".to_owned());
        }
        _ => return Err(usage().to_owned()),
    };
    let config = Config {
        rule,
        workload,
        world: world.ok_or_else(|| usage().to_owned())?,
        representation: representation.ok_or_else(|| usage().to_owned())?,
        repetitions: repetitions.ok_or_else(|| usage().to_owned())?,
        warmup: warmup.ok_or_else(|| usage().to_owned())?,
        timed_generations: timed_generations.ok_or_else(|| usage().to_owned())?,
        layout,
        execution_order,
    };
    if config.world == 0 || config.repetitions == 0 || config.timed_generations == 0 {
        return Err("world, repetitions, and timed-generations must be positive".to_owned());
    }
    Ok(config)
}

fn xorshift64(value: &mut u64) -> u64 {
    *value ^= *value << 13;
    *value ^= *value >> 7;
    *value ^= *value << 17;
    *value
}

fn uniform_synthetic_grid(
    density_ppm: u32,
    mut seed: u64,
    world: usize,
) -> Result<DenseGrid, String> {
    if density_ppm > 1_000_000 {
        return Err("density-ppm must be <= 1000000".to_owned());
    }
    let mut grid = DenseGrid::new(world, world).map_err(|error| error.to_string())?;
    for y in 0..world {
        for x in 0..world {
            let random = xorshift64(&mut seed);
            if random % 1_000_000 < u64::from(density_ppm) {
                grid.set(
                    x,
                    y,
                    State::try_from((random % 7 + 1) as u8).expect("range is 1..=7"),
                )
                .map_err(|error| error.to_string())?;
            }
        }
    }
    Ok(grid)
}

fn clustered_from_uniform(uniform: &DenseGrid) -> Result<DenseGrid, String> {
    let mut states = Vec::with_capacity(uniform.active_cell_count());
    for y in 0..uniform.height() {
        for x in 0..uniform.width() {
            let state = uniform.get(x, y).map_err(|error| error.to_string())?;
            if state != State::QUIESCENT {
                states.push(state);
            }
        }
    }
    let mut clustered =
        DenseGrid::new(uniform.width(), uniform.height()).map_err(|error| error.to_string())?;
    if states.is_empty() {
        return Ok(clustered);
    }
    let mut cluster_width = 1usize;
    while cluster_width.saturating_mul(cluster_width) < states.len() {
        cluster_width += 1;
    }
    cluster_width = cluster_width.min(uniform.width());
    let cluster_height = states.len().div_ceil(cluster_width);
    if cluster_height > uniform.height() {
        return Err("cluster does not fit world".to_owned());
    }
    let origin_x = (uniform.width() - cluster_width) / 2;
    let origin_y = (uniform.height() - cluster_height) / 2;
    for (index, state) in states.into_iter().enumerate() {
        clustered
            .set(
                origin_x + index % cluster_width,
                origin_y + index / cluster_width,
                state,
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(clustered)
}

fn initial_grid(
    workload: Workload,
    layout: Layout,
    world: usize,
    rule: &RuleProfile,
) -> Result<(DenseGrid, String, u64), String> {
    match workload {
        Workload::Canonical { generation } => {
            let initial = match rule {
                RuleProfile::Langton(_) => {
                    let seed = CanonicalSeed::canonical().map_err(|error| error.to_string())?;
                    let (width, height) = seed.dimensions();
                    if world < width || world < height {
                        return Err("world is smaller than the Langton seed".to_owned());
                    }
                    seed.place_in(world, world, (world - width) / 2, (world - height) / 2)
                        .map_err(|error| error.to_string())?
                }
                RuleProfile::BylGolly33(_) => {
                    let seed = BylSeed::golly_3_3_profile().map_err(|error| error.to_string())?;
                    let (width, height) = seed.dimensions();
                    if world < width || world < height {
                        return Err("world is smaller than the Byl seed".to_owned());
                    }
                    seed.place_in(world, world, (world - width) / 2, (world - height) / 2)
                        .map_err(|error| error.to_string())?
                }
                RuleProfile::Identity => {
                    return Err("identity profile has no canonical seed".to_owned());
                }
            };
            let name = match rule {
                RuleProfile::Langton(_) => format!("canonical-generation-{generation}"),
                RuleProfile::BylGolly33(_) => {
                    format!("byl-golly-3.3-generation-{generation}")
                }
                RuleProfile::Identity => unreachable!("rejected above"),
            };
            Ok((initial.run(rule, generation), name, generation))
        }
        Workload::SyntheticLangton { density_ppm, seed } => {
            let uniform = uniform_synthetic_grid(density_ppm, seed, world)?;
            let grid = if layout == Layout::Clustered {
                clustered_from_uniform(&uniform)?
            } else {
                uniform
            };
            Ok((
                grid,
                format!("synthetic-{density_ppm}ppm-{}-seed-{seed}", layout.name()),
                0,
            ))
        }
        Workload::SyntheticIdentity { density_ppm, seed } => {
            let uniform = uniform_synthetic_grid(density_ppm, seed, world)?;
            let grid = if layout == Layout::Clustered {
                clustered_from_uniform(&uniform)?
            } else {
                uniform
            };
            Ok((
                grid,
                format!("identity-{density_ppm}ppm-{}-seed-{seed}", layout.name()),
                0,
            ))
        }
    }
}

fn spatial_metrics(grid: &DenseGrid) -> SpatialMetrics {
    let mut min_x = usize::MAX;
    let mut min_y = usize::MAX;
    let mut max_x = 0usize;
    let mut max_y = 0usize;
    let mut active = 0usize;
    let mut regions = BTreeSet::new();
    let mut touches_boundary = false;
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if grid.get(x, y).expect("coordinates are in bounds") != State::QUIESCENT {
                active += 1;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
                regions.insert((x / LOGICAL_REGION_SIDE, y / LOGICAL_REGION_SIDE));
                touches_boundary |=
                    x == 0 || y == 0 || x + 1 == grid.width() || y + 1 == grid.height();
            }
        }
    }
    if active == 0 {
        SpatialMetrics {
            active_bounds: None,
            active_bounds_area: 0,
            occupied_logical_regions_32: 0,
            touches_boundary: false,
        }
    } else {
        let width = max_x - min_x + 1;
        let height = max_y - min_y + 1;
        SpatialMetrics {
            active_bounds: Some([min_x, min_y, width, height]),
            active_bounds_area: width * height,
            occupied_logical_regions_32: regions.len(),
            touches_boundary,
        }
    }
}

fn update_activity(
    stats: StepStats,
    active: &mut u64,
    evaluations: &mut u64,
    minimum: &mut usize,
    maximum: &mut usize,
) {
    *active += stats.active_cells_before_step as u64;
    *evaluations += stats.cell_evaluations as u64;
    *minimum = (*minimum).min(stats.active_cells_before_step);
    *maximum = (*maximum).max(stats.active_cells_before_step);
}

fn process_memory_bytes() -> (Option<u64>, Option<u64>) {
    let Ok(status) = fs::read_to_string("/proc/self/status") else {
        return (None, None);
    };
    let parse = |label: &str| {
        status.lines().find_map(|line| {
            let value = line.strip_prefix(label)?.trim();
            value
                .strip_suffix(" kB")?
                .trim()
                .parse::<u64>()
                .ok()
                .map(|kilobytes| kilobytes * 1024)
        })
    };
    (parse("VmRSS:"), parse("VmHWM:"))
}

fn timed_start<R: LocalRule + ?Sized>(
    grid: &DenseGrid,
    generation: u64,
    rule: &R,
) -> (usize, SpatialMetrics, String) {
    (
        grid.active_cell_count(),
        spatial_metrics(grid),
        Snapshot::from_grid_for_rule(grid, generation, rule).state_hash_sha256,
    )
}

fn run_dense(
    initial: DenseGrid,
    rule: &RuleProfile,
    initial_generation: u64,
    warmup: u64,
    generations: u64,
) -> RunResult {
    let mut runner = DenseRunner::new(initial);
    for _ in 0..warmup {
        runner.step(rule);
    }
    let (timed_start_active_cell_count, timed_start_spatial, timed_start_state_hash_sha256) =
        timed_start(runner.grid(), initial_generation + warmup, rule);
    let start = Instant::now();
    let mut active = 0u64;
    let mut evaluations = 0u64;
    let mut minimum = usize::MAX;
    let mut maximum = 0usize;
    for _ in 0..generations {
        update_activity(
            runner.step(rule),
            &mut active,
            &mut evaluations,
            &mut minimum,
            &mut maximum,
        );
    }
    let elapsed_nanoseconds = start.elapsed().as_nanos();
    let logical_storage_bytes = runner.logical_storage_bytes();
    let (current_resident_bytes, peak_resident_bytes) = process_memory_bytes();
    RunResult {
        final_grid: runner.into_grid(),
        active_cell_updates: active,
        cell_evaluations: evaluations,
        chunk_evaluations: None,
        elapsed_nanoseconds,
        logical_storage_bytes,
        allocation_accounting: "exact-dense-state-capacity",
        allocated_chunks_final: None,
        allocated_chunks_peak: None,
        timed_active_cell_min: minimum,
        timed_active_cell_max: maximum,
        timed_start_active_cell_count,
        timed_start_spatial,
        timed_start_state_hash_sha256,
        current_resident_bytes,
        peak_resident_bytes,
    }
}

fn run_sparse(
    initial: DenseGrid,
    rule: &RuleProfile,
    initial_generation: u64,
    warmup: u64,
    generations: u64,
) -> RunResult {
    let mut current = SparseFrontierGrid::from_dense(&initial);
    drop(initial);
    for _ in 0..warmup {
        current = current.step(rule).0;
    }
    let (timed_start_active_cell_count, timed_start_spatial, timed_start_state_hash_sha256) = {
        let grid = current.to_dense();
        timed_start(&grid, initial_generation + warmup, rule)
    };
    let start = Instant::now();
    let mut active = 0u64;
    let mut evaluations = 0u64;
    let mut minimum = usize::MAX;
    let mut maximum = 0usize;
    for _ in 0..generations {
        let (next, stats) = current.step(rule);
        update_activity(
            stats,
            &mut active,
            &mut evaluations,
            &mut minimum,
            &mut maximum,
        );
        current = next;
    }
    let elapsed_nanoseconds = start.elapsed().as_nanos();
    let logical_storage_bytes = current.logical_storage_bytes_lower_bound();
    let (current_resident_bytes, peak_resident_bytes) = process_memory_bytes();
    let final_grid = current.to_dense();
    RunResult {
        final_grid,
        active_cell_updates: active,
        cell_evaluations: evaluations,
        chunk_evaluations: None,
        elapsed_nanoseconds,
        logical_storage_bytes,
        allocation_accounting: "exact-logical-sparse-key-state-payload; excludes-btree-overhead",
        allocated_chunks_final: None,
        allocated_chunks_peak: None,
        timed_active_cell_min: minimum,
        timed_active_cell_max: maximum,
        timed_start_active_cell_count,
        timed_start_spatial,
        timed_start_state_hash_sha256,
        current_resident_bytes,
        peak_resident_bytes,
    }
}

fn run_chunked(
    initial: DenseGrid,
    rule: &RuleProfile,
    initial_generation: u64,
    warmup: u64,
    generations: u64,
) -> RunResult {
    let chunked = ChunkedGrid::from_dense(&initial);
    drop(initial);
    let mut runner = ChunkedRunner::new(chunked);
    let mut peak_chunks = runner.grid().allocated_chunk_count();
    for _ in 0..warmup {
        let stats = runner.step(rule);
        peak_chunks = peak_chunks.max(stats.allocated_chunks_after_step);
    }
    let (timed_start_active_cell_count, timed_start_spatial, timed_start_state_hash_sha256) = {
        let grid = runner.grid().to_dense();
        timed_start(&grid, initial_generation + warmup, rule)
    };
    let start = Instant::now();
    let mut active = 0u64;
    let mut evaluations = 0u64;
    let mut chunk_evaluations = 0u64;
    let mut minimum = usize::MAX;
    let mut maximum = 0usize;
    for _ in 0..generations {
        let stats = runner.step(rule);
        update_activity(
            stats.step,
            &mut active,
            &mut evaluations,
            &mut minimum,
            &mut maximum,
        );
        chunk_evaluations += stats.chunk_evaluations as u64;
        peak_chunks = peak_chunks.max(stats.allocated_chunks_after_step);
    }
    let elapsed_nanoseconds = start.elapsed().as_nanos();
    let logical_storage_bytes = runner.logical_storage_bytes();
    let allocated_chunks_final = runner.grid().allocated_chunk_count();
    let (current_resident_bytes, peak_resident_bytes) = process_memory_bytes();
    let final_grid = runner.into_grid().to_dense();
    RunResult {
        final_grid,
        active_cell_updates: active,
        cell_evaluations: evaluations,
        chunk_evaluations: Some(chunk_evaluations),
        elapsed_nanoseconds,
        logical_storage_bytes,
        allocation_accounting: "exact-owned-chunk-directory-state-scratch-capacity",
        allocated_chunks_final: Some(allocated_chunks_final),
        allocated_chunks_peak: Some(peak_chunks),
        timed_active_cell_min: minimum,
        timed_active_cell_max: maximum,
        timed_start_active_cell_count,
        timed_start_spatial,
        timed_start_state_hash_sha256,
        current_resident_bytes,
        peak_resident_bytes,
    }
}

fn cpu_model() -> Option<String> {
    fs::read_to_string("/proc/cpuinfo")
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix("model name\t: ").map(str::to_owned))
}

fn hardware_counter_status() -> String {
    match fs::read_to_string("/proc/sys/kernel/perf_event_paranoid") {
        Ok(value) => format!("not-collected: perf_event_paranoid={}", value.trim()),
        Err(error) => format!("not-collected: perf status unavailable ({error})"),
    }
}

fn energy_status() -> String {
    let candidates = [
        "/sys/class/powercap/intel-rapl:0/energy_uj",
        "/sys/class/powercap/amd-rapl:0/energy_uj",
    ];
    if candidates.iter().any(|path| fs::File::open(path).is_ok()) {
        "not-collected: readable interface detection requires experiment driver".to_owned()
    } else {
        "not-collected: no readable powercap energy_uj interface".to_owned()
    }
}

fn rust_version() -> String {
    option_env!("RUSTC_VERSION")
        .unwrap_or("not-embedded; record cargo/rustc externally")
        .to_owned()
}

fn memory_delta(after: Option<u64>, before: Option<u64>) -> (Option<u64>, bool) {
    match (after, before) {
        (Some(after), Some(before)) => (Some(after.saturating_sub(before)), after < before),
        _ => (None, false),
    }
}

struct RecordContext<'a> {
    config: &'a Config,
    rule: &'a RuleProfile,
    workload: &'a str,
    repetition: u32,
    in_process_order: usize,
    representation: Representation,
    initial_active_cell_count: usize,
    initial_generation: u64,
    initial_state_hash_sha256: &'a str,
    baseline_resident_bytes: Option<u64>,
    baseline_peak_resident_bytes: Option<u64>,
}

fn record(context: RecordContext<'_>, result: &RunResult) -> BenchmarkRecord {
    let elapsed_seconds = result.elapsed_nanoseconds as f64 / 1_000_000_000.0;
    let final_generation =
        context.initial_generation + context.config.warmup + context.config.timed_generations;
    let final_snapshot =
        Snapshot::from_grid_for_rule(&result.final_grid, final_generation, context.rule);
    let world_cells = context.config.world * context.config.world;
    let (current_resident_delta_bytes, current_saturated) = memory_delta(
        result.current_resident_bytes,
        context.baseline_resident_bytes,
    );
    let (peak_resident_delta_bytes, peak_saturated) = memory_delta(
        result.peak_resident_bytes,
        context.baseline_peak_resident_bytes,
    );
    BenchmarkRecord {
        active_cell_updates: result.active_cell_updates,
        actual_timed_density_ppm: result.active_cell_updates as f64
            / (world_cells as f64 * context.config.timed_generations as f64)
            * 1_000_000.0,
        allocated_bytes_estimate: result.logical_storage_bytes,
        allocated_chunks_final: result.allocated_chunks_final,
        allocated_chunks_peak: result.allocated_chunks_peak,
        allocation_accounting: result.allocation_accounting,
        baseline_peak_resident_bytes: context.baseline_peak_resident_bytes,
        baseline_resident_bytes: context.baseline_resident_bytes,
        benchmark_schema_version: BENCHMARK_SCHEMA_VERSION,
        boundary: "fixed-quiescent",
        cell_evaluations: result.cell_evaluations,
        chunk_evaluations: result.chunk_evaluations,
        chunk_side: matches!(context.representation, Representation::Chunked).then_some(CHUNK_SIDE),
        code_revision: env::var("RUST_LOOPS_GIT_REVISION").unwrap_or_else(|_| "unknown".to_owned()),
        coordinate_basis: context.rule.coordinate_basis(),
        cpu_model: cpu_model(),
        current_resident_bytes: result.current_resident_bytes,
        current_resident_delta_bytes,
        data_movement_status: "logical-storage-and-evaluations-only; physical-traffic-not-measured",
        elapsed_nanoseconds: result.elapsed_nanoseconds,
        energy_status: energy_status(),
        execution_order: context.config.execution_order,
        final_active_cell_count: final_snapshot.active_cell_count,
        final_generation,
        final_spatial: spatial_metrics(&result.final_grid),
        final_state_populations: final_snapshot.state_populations,
        generations_per_second: context.config.timed_generations as f64 / elapsed_seconds,
        hardware_counter_status: hardware_counter_status(),
        in_process_order: context.in_process_order,
        initial_active_cell_count: context.initial_active_cell_count,
        initial_generation: context.initial_generation,
        initial_state_hash_sha256: context.initial_state_hash_sha256.to_owned(),
        layout: context.config.layout.name(),
        logical_region_side: LOGICAL_REGION_SIDE,
        logical_storage_bytes: result.logical_storage_bytes,
        os: env::consts::OS,
        output_state_hash_sha256: final_snapshot.state_hash_sha256,
        peak_resident_bytes: result.peak_resident_bytes,
        peak_resident_delta_bytes,
        repetition: context.repetition,
        representation: context.representation.name(),
        representation_fixture_ownership: "owned; baseline-before-fixture; dense-dropped-after-conversion",
        resident_delta_saturated: current_saturated || peak_saturated,
        resident_memory_method: "Linux /proc/self/status VmRSS/VmHWM; process-level",
        rule_fixture_sha256: context.rule.rule_fixture_sha256(),
        rule_profile: context.rule.id(),
        rust_version: rust_version(),
        seed_fixture_sha256: context.rule.seed_fixture_sha256(),
        synthetic_seed: context.config.workload.synthetic_seed(),
        timed_active_cell_max: result.timed_active_cell_max,
        timed_active_cell_mean: result.active_cell_updates as f64
            / context.config.timed_generations as f64,
        timed_active_cell_min: result.timed_active_cell_min,
        timed_generations: context.config.timed_generations,
        timed_start_active_cell_count: result.timed_start_active_cell_count,
        timed_start_generation: context.initial_generation + context.config.warmup,
        timed_start_spatial: result.timed_start_spatial.clone(),
        timed_start_state_hash_sha256: result.timed_start_state_hash_sha256.clone(),
        updates_per_second: result.cell_evaluations as f64 / elapsed_seconds,
        useful_active_updates_per_second: result.active_cell_updates as f64 / elapsed_seconds,
        workload: context.workload.to_owned(),
        world: [context.config.world, context.config.world],
    }
}

fn main() -> ExitCode {
    let config = match parse_arguments() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    let rule = match RuleProfile::load(config.rule, config.workload) {
        Ok(rule) => rule,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::from(1);
        }
    };
    for repetition in 0..config.repetitions {
        let mut expected = None;
        for (in_process_order, representation) in config
            .representation
            .representations()
            .iter()
            .copied()
            .enumerate()
        {
            let (baseline_resident_bytes, baseline_peak_resident_bytes) = process_memory_bytes();
            let (initial, workload_name, initial_generation) =
                match initial_grid(config.workload, config.layout, config.world, &rule) {
                    Ok(value) => value,
                    Err(error) => {
                        eprintln!("error: {error}");
                        return ExitCode::from(1);
                    }
                };
            let initial_snapshot =
                Snapshot::from_grid_for_rule(&initial, initial_generation, &rule);
            let result = match representation {
                Representation::Dense => run_dense(
                    initial,
                    &rule,
                    initial_generation,
                    config.warmup,
                    config.timed_generations,
                ),
                Representation::Sparse => run_sparse(
                    initial,
                    &rule,
                    initial_generation,
                    config.warmup,
                    config.timed_generations,
                ),
                Representation::Chunked => run_chunked(
                    initial,
                    &rule,
                    initial_generation,
                    config.warmup,
                    config.timed_generations,
                ),
            };
            let output = record(
                RecordContext {
                    config: &config,
                    rule: &rule,
                    workload: &workload_name,
                    repetition,
                    in_process_order,
                    representation,
                    initial_active_cell_count: initial_snapshot.active_cell_count,
                    initial_generation,
                    initial_state_hash_sha256: &initial_snapshot.state_hash_sha256,
                    baseline_resident_bytes,
                    baseline_peak_resident_bytes,
                },
                &result,
            );
            println!(
                "{}",
                serde_json::to_string(&output).expect("serializable benchmark record")
            );
            let final_snapshot = Snapshot::from_grid_for_rule(&result.final_grid, 0, &rule);
            if let Some(expected) = &expected {
                if &final_snapshot != expected {
                    eprintln!(
                        "error: {} mismatch for {workload_name}, repetition {repetition}",
                        representation.name()
                    );
                    return ExitCode::from(1);
                }
            } else {
                expected = Some(final_snapshot);
            }
        }
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    #[test]
    fn identity_rule_preserves_a_multistate_field() {
        let rule = RuleProfile::Identity;
        let initial = uniform_synthetic_grid(100_000, 1_592_639_710, 41).unwrap();
        assert_eq!(initial, initial.run(&rule, 4));
    }

    #[test]
    fn uniform_and_clustered_layouts_preserve_count_and_state_multiset() {
        let uniform = uniform_synthetic_grid(50_000, 1_592_639_710, 97).unwrap();
        let clustered = clustered_from_uniform(&uniform).unwrap();
        let uniform_snapshot = Snapshot::from_grid(&uniform, 0);
        let clustered_snapshot = Snapshot::from_grid(&clustered, 0);
        assert_eq!(
            uniform_snapshot.active_cell_count,
            clustered_snapshot.active_cell_count
        );
        assert_eq!(
            uniform_snapshot.state_populations,
            clustered_snapshot.state_populations
        );
    }

    #[test]
    fn memory_deltas_are_nonnegative_and_flag_saturation() {
        assert_eq!(memory_delta(Some(12), Some(10)), (Some(2), false));
        assert_eq!(memory_delta(Some(8), Some(10)), (Some(0), true));
        assert_eq!(memory_delta(None, Some(10)), (None, false));
    }

    #[test]
    fn benchmark_fixture_hashes_are_current() {
        let identity = Sha256::digest(include_bytes!("../../data/benchmark/identity-rule.json"));
        let generator = Sha256::digest(include_bytes!(
            "../../data/benchmark/synthetic-generator.json"
        ));
        assert_eq!(format!("{identity:x}"), IDENTITY_RULE_SHA256);
        assert_eq!(format!("{generator:x}"), SYNTHETIC_GENERATOR_SHA256);
    }
}
