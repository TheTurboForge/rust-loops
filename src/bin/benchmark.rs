//! JSONL benchmark harness for exact CA representation comparisons.

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::process::ExitCode;
use std::time::Instant;

use rust_loops::{
    CHUNK_SIDE, CanonicalSeed, ChunkedGrid, ChunkedRunner, DenseGrid, DenseRunner, LangtonRule,
    Neighborhood, Snapshot, SparseFrontierGrid, State, StepStats,
};
use serde::Serialize;

const BENCHMARK_SCHEMA_VERSION: u8 = 2;

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
    fn rule_id(self) -> &'static str {
        match self {
            Self::SyntheticIdentity { .. } => "benchmark-identity-v1",
            Self::Canonical { .. } | Self::SyntheticLangton { .. } => "langton-canonical",
        }
    }
}

struct Config {
    workload: Workload,
    world: usize,
    representation: RepresentationSelection,
    repetitions: u32,
    warmup: u64,
    timed_generations: u64,
    layout: Layout,
    execution_order: u32,
}

#[derive(Serialize)]
struct BenchmarkRecord {
    active_cell_updates: u64,
    actual_timed_density_ppm: f64,
    allocated_bytes_estimate: usize,
    allocated_chunks_final: Option<usize>,
    allocated_chunks_peak: Option<usize>,
    allocation_accounting: &'static str,
    benchmark_schema_version: u8,
    boundary: &'static str,
    cell_evaluations: u64,
    chunk_evaluations: Option<u64>,
    chunk_side: Option<usize>,
    code_revision: String,
    cpu_model: Option<String>,
    current_resident_bytes: Option<u64>,
    data_movement_status: String,
    elapsed_nanoseconds: u128,
    energy_status: String,
    execution_order: u32,
    final_active_cell_count: usize,
    generations_per_second: f64,
    hardware_counter_status: String,
    in_process_order: usize,
    initial_active_cell_count: usize,
    layout: &'static str,
    logical_storage_bytes: usize,
    os: &'static str,
    output_state_hash_sha256: String,
    peak_resident_bytes: Option<u64>,
    perf_status: String,
    repetition: u32,
    representation: &'static str,
    resident_memory_method: &'static str,
    rule_id: &'static str,
    rust_version: String,
    state_populations: BTreeMap<String, usize>,
    timed_active_cell_max: usize,
    timed_active_cell_min: usize,
    timed_active_cell_mean: f64,
    timed_generations: u64,
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
    current_resident_bytes: Option<u64>,
    peak_resident_bytes: Option<u64>,
}

fn usage() -> &'static str {
    "usage: benchmark --workload canonical|synthetic|synthetic-identity --world <N> --representation dense|sparse|chunked|both|all --repetitions <N> --warmup <N> --timed-generations <N> [--generation <N>] [--density-ppm <N>] [--seed <N>] [--layout uniform|clustered] [--execution-order <N>]"
}

fn parse_arguments() -> Result<Config, String> {
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
        Some("synthetic") => Workload::SyntheticLangton {
            density_ppm: density_ppm
                .ok_or_else(|| "synthetic workload requires --density-ppm".to_owned())?,
            seed,
        },
        Some("synthetic-identity") => Workload::SyntheticIdentity {
            density_ppm: density_ppm
                .ok_or_else(|| "synthetic-identity workload requires --density-ppm".to_owned())?,
            seed,
        },
        _ => return Err(usage().to_owned()),
    };
    let config = Config {
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
    langton_rule: &LangtonRule,
) -> Result<(DenseGrid, String), String> {
    match workload {
        Workload::Canonical { generation } => {
            let seed = CanonicalSeed::canonical().map_err(|error| error.to_string())?;
            let (seed_width, seed_height) = seed.dimensions();
            if world < seed_width || world < seed_height {
                return Err("world is smaller than canonical seed".to_owned());
            }
            let initial = seed
                .place_in(
                    world,
                    world,
                    (world - seed_width) / 2,
                    (world - seed_height) / 2,
                )
                .map_err(|error| error.to_string())?;
            Ok((
                initial.run(langton_rule, generation),
                format!("canonical-generation-{generation}"),
            ))
        }
        Workload::SyntheticLangton { density_ppm, seed } => {
            let uniform = uniform_synthetic_grid(density_ppm, seed, world)?;
            let grid = if layout == Layout::Clustered {
                clustered_from_uniform(&uniform)?
            } else {
                uniform
            };
            let workload_name = if layout == Layout::Uniform {
                format!("synthetic-{density_ppm}ppm-seed-{seed}")
            } else {
                format!("synthetic-{density_ppm}ppm-clustered-seed-{seed}")
            };
            Ok((grid, workload_name))
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
            ))
        }
    }
}

fn identity_rule() -> Result<LangtonRule, String> {
    let mut transitions = Vec::with_capacity(8usize.pow(5));
    for center in 0u8..8 {
        for north in 0u8..8 {
            for east in 0u8..8 {
                for south in 0u8..8 {
                    for west in 0u8..8 {
                        transitions.push((
                            Neighborhood {
                                center: State::try_from(center).expect("state range"),
                                north: State::try_from(north).expect("state range"),
                                east: State::try_from(east).expect("state range"),
                                south: State::try_from(south).expect("state range"),
                                west: State::try_from(west).expect("state range"),
                            },
                            State::try_from(center).expect("state range"),
                        ));
                    }
                }
            }
        }
    }
    LangtonRule::from_base_transitions(transitions).map_err(|error| error.to_string())
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

fn run_dense(initial: &DenseGrid, rule: &LangtonRule, warmup: u64, generations: u64) -> RunResult {
    let mut runner = DenseRunner::new(initial.clone());
    for _ in 0..warmup {
        runner.step(rule);
    }
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
    let elapsed = start.elapsed().as_nanos();
    let logical_storage_bytes = runner.logical_storage_bytes();
    let (current_resident_bytes, peak_resident_bytes) = process_memory_bytes();
    RunResult {
        final_grid: runner.into_grid(),
        active_cell_updates: active,
        cell_evaluations: evaluations,
        chunk_evaluations: None,
        elapsed_nanoseconds: elapsed,
        logical_storage_bytes,
        allocation_accounting: "exact-dense-state-capacity",
        allocated_chunks_final: None,
        allocated_chunks_peak: None,
        timed_active_cell_min: minimum,
        timed_active_cell_max: maximum,
        current_resident_bytes,
        peak_resident_bytes,
    }
}

fn run_sparse(initial: &DenseGrid, rule: &LangtonRule, warmup: u64, generations: u64) -> RunResult {
    let mut current = SparseFrontierGrid::from_dense(initial);
    for _ in 0..warmup {
        current = current.step(rule).0;
    }
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
    let elapsed = start.elapsed().as_nanos();
    let logical_storage_bytes = current.logical_storage_bytes_lower_bound();
    let (current_resident_bytes, peak_resident_bytes) = process_memory_bytes();
    RunResult {
        final_grid: current.to_dense(),
        active_cell_updates: active,
        cell_evaluations: evaluations,
        chunk_evaluations: None,
        elapsed_nanoseconds: elapsed,
        logical_storage_bytes,
        allocation_accounting: "lower-bound-sparse-key-state-payload",
        allocated_chunks_final: None,
        allocated_chunks_peak: None,
        timed_active_cell_min: minimum,
        timed_active_cell_max: maximum,
        current_resident_bytes,
        peak_resident_bytes,
    }
}

fn run_chunked(
    initial: &DenseGrid,
    rule: &LangtonRule,
    warmup: u64,
    generations: u64,
) -> RunResult {
    let mut runner = ChunkedRunner::new(ChunkedGrid::from_dense(initial));
    let mut peak_chunks = runner.grid().allocated_chunk_count();
    for _ in 0..warmup {
        let stats = runner.step(rule);
        peak_chunks = peak_chunks.max(stats.allocated_chunks_after_step);
    }
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
    let elapsed = start.elapsed().as_nanos();
    let logical_storage_bytes = runner.logical_storage_bytes();
    let allocated_chunks_final = runner.grid().allocated_chunk_count();
    let (current_resident_bytes, peak_resident_bytes) = process_memory_bytes();
    RunResult {
        final_grid: runner.into_grid().to_dense(),
        active_cell_updates: active,
        cell_evaluations: evaluations,
        chunk_evaluations: Some(chunk_evaluations),
        elapsed_nanoseconds: elapsed,
        logical_storage_bytes,
        allocation_accounting: "owned-chunk-directory-state-and-scratch-capacity",
        allocated_chunks_final: Some(allocated_chunks_final),
        allocated_chunks_peak: Some(peak_chunks),
        timed_active_cell_min: minimum,
        timed_active_cell_max: maximum,
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

struct RecordInput<'a> {
    representation: Representation,
    workload: &'a str,
    rule_id: &'static str,
    layout: Layout,
    world: usize,
    repetition: u32,
    initial_active: usize,
    result: &'a RunResult,
    generations: u64,
    execution_order: u32,
    in_process_order: usize,
}

fn record(input: RecordInput<'_>) -> BenchmarkRecord {
    let elapsed_seconds = input.result.elapsed_nanoseconds as f64 / 1_000_000_000.0;
    let snapshot = Snapshot::from_grid(&input.result.final_grid, 0);
    let timed_active_cell_mean = input.result.active_cell_updates as f64 / input.generations as f64;
    let counter_status = hardware_counter_status();
    BenchmarkRecord {
        active_cell_updates: input.result.active_cell_updates,
        actual_timed_density_ppm: timed_active_cell_mean / (input.world * input.world) as f64
            * 1_000_000.0,
        allocated_bytes_estimate: input.result.logical_storage_bytes,
        allocated_chunks_final: input.result.allocated_chunks_final,
        allocated_chunks_peak: input.result.allocated_chunks_peak,
        allocation_accounting: input.result.allocation_accounting,
        benchmark_schema_version: BENCHMARK_SCHEMA_VERSION,
        boundary: "fixed-quiescent",
        cell_evaluations: input.result.cell_evaluations,
        chunk_evaluations: input.result.chunk_evaluations,
        chunk_side: matches!(input.representation, Representation::Chunked).then_some(CHUNK_SIDE),
        code_revision: env::var("RUST_LOOPS_GIT_REVISION").unwrap_or_else(|_| "unknown".to_owned()),
        cpu_model: cpu_model(),
        current_resident_bytes: input.result.current_resident_bytes,
        data_movement_status: "not-collected: logical evaluations are not physical traffic"
            .to_owned(),
        elapsed_nanoseconds: input.result.elapsed_nanoseconds,
        energy_status: energy_status(),
        execution_order: input.execution_order,
        final_active_cell_count: input.result.final_grid.active_cell_count(),
        generations_per_second: input.generations as f64 / elapsed_seconds,
        hardware_counter_status: counter_status.clone(),
        in_process_order: input.in_process_order,
        initial_active_cell_count: input.initial_active,
        layout: input.layout.name(),
        logical_storage_bytes: input.result.logical_storage_bytes,
        os: env::consts::OS,
        output_state_hash_sha256: snapshot.state_hash_sha256,
        peak_resident_bytes: input.result.peak_resident_bytes,
        perf_status: counter_status,
        repetition: input.repetition,
        representation: input.representation.name(),
        resident_memory_method: "Linux /proc/self/status VmRSS/VmHWM; process-level",
        rule_id: input.rule_id,
        rust_version: rust_version(),
        state_populations: snapshot.state_populations,
        timed_active_cell_max: input.result.timed_active_cell_max,
        timed_active_cell_min: input.result.timed_active_cell_min,
        timed_active_cell_mean,
        timed_generations: input.generations,
        updates_per_second: input.result.cell_evaluations as f64 / elapsed_seconds,
        useful_active_updates_per_second: input.result.active_cell_updates as f64 / elapsed_seconds,
        workload: input.workload.to_owned(),
        world: [input.world, input.world],
    }
}

fn representations(selection: RepresentationSelection) -> &'static [Representation] {
    match selection {
        RepresentationSelection::Dense => &[Representation::Dense],
        RepresentationSelection::Sparse => &[Representation::Sparse],
        RepresentationSelection::Chunked => &[Representation::Chunked],
        RepresentationSelection::Both => &[Representation::Dense, Representation::Sparse],
        RepresentationSelection::All => &[
            Representation::Dense,
            Representation::Sparse,
            Representation::Chunked,
        ],
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
    let langton_rule = match LangtonRule::canonical() {
        Ok(rule) => rule,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::from(1);
        }
    };
    let identity_rule_value;
    let rule = match config.workload {
        Workload::SyntheticIdentity { .. } => {
            identity_rule_value = match identity_rule() {
                Ok(rule) => rule,
                Err(error) => {
                    eprintln!("error: {error}");
                    return ExitCode::from(1);
                }
            };
            &identity_rule_value
        }
        Workload::Canonical { .. } | Workload::SyntheticLangton { .. } => &langton_rule,
    };
    let (initial, workload_name) =
        match initial_grid(config.workload, config.layout, config.world, &langton_rule) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("error: {error}");
                return ExitCode::from(1);
            }
        };
    for repetition in 0..config.repetitions {
        let initial_active = initial.active_cell_count();
        let mut results = Vec::new();
        for representation in representations(config.representation) {
            let result = match representation {
                Representation::Dense => {
                    run_dense(&initial, rule, config.warmup, config.timed_generations)
                }
                Representation::Sparse => {
                    run_sparse(&initial, rule, config.warmup, config.timed_generations)
                }
                Representation::Chunked => {
                    run_chunked(&initial, rule, config.warmup, config.timed_generations)
                }
            };
            results.push((*representation, result));
        }
        if let Some((_, first)) = results.first() {
            let expected = Snapshot::from_grid(&first.final_grid, 0);
            for (representation, result) in results.iter().skip(1) {
                if Snapshot::from_grid(&result.final_grid, 0) != expected {
                    eprintln!(
                        "error: {} mismatch for {workload_name}, repetition {repetition}",
                        representation.name()
                    );
                    return ExitCode::from(1);
                }
            }
        }
        for (in_process_order, (representation, result)) in results.iter().enumerate() {
            let output = record(RecordInput {
                representation: *representation,
                workload: &workload_name,
                rule_id: config.workload.rule_id(),
                layout: config.layout,
                world: config.world,
                repetition,
                initial_active,
                result,
                generations: config.timed_generations,
                execution_order: config.execution_order,
                in_process_order,
            });
            println!(
                "{}",
                serde_json::to_string(&output).expect("serializable benchmark record")
            );
        }
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_rule_preserves_a_multistate_field() {
        let rule = identity_rule().unwrap();
        let initial = uniform_synthetic_grid(100_000, 1592639710, 41).unwrap();
        assert_eq!(initial, initial.run(&rule, 4));
    }

    #[test]
    fn uniform_and_clustered_layouts_preserve_count_and_state_multiset() {
        let uniform = uniform_synthetic_grid(50_000, 1592639710, 97).unwrap();
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
}
