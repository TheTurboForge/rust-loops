//! JSONL benchmark harness for exact dense-versus-sparse CA comparisons.

use std::env;
use std::fs;
use std::process::ExitCode;
use std::time::Instant;

use rust_loops::{
    CanonicalSeed, DenseGrid, DenseRunner, LangtonRule, Snapshot, SparseFrontierGrid, State,
    StepStats,
};
use serde::Serialize;

#[derive(Clone, Copy)]
enum Representation {
    Dense,
    Sparse,
    Both,
}

#[derive(Clone, Copy)]
enum Workload {
    Canonical { generation: u64 },
    Synthetic { density_ppm: u32, seed: u64 },
}

#[derive(Serialize)]
struct BenchmarkRecord {
    active_cell_updates: u64,
    allocated_bytes_estimate: usize,
    boundary: &'static str,
    cell_evaluations: u64,
    code_revision: String,
    cpu_model: Option<String>,
    elapsed_nanoseconds: u128,
    final_active_cell_count: usize,
    generations_per_second: f64,
    initial_active_cell_count: usize,
    os: &'static str,
    output_state_hash_sha256: String,
    perf_status: String,
    repetition: u32,
    representation: &'static str,
    rust_version: String,
    timed_generations: u64,
    updates_per_second: f64,
    useful_active_updates_per_second: f64,
    workload: String,
    world: [usize; 2],
}

struct RecordInput<'a> {
    representation: &'static str,
    workload: &'a str,
    world: usize,
    repetition: u32,
    initial_active: usize,
    final_grid: &'a DenseGrid,
    active: u64,
    evaluations: u64,
    elapsed: u128,
    generations: u64,
}

fn usage() -> &'static str {
    "usage: benchmark --workload canonical|synthetic --world <N> --representation dense|sparse|both --repetitions <N> --warmup <N> --timed-generations <N> [--generation <N>] [--density-ppm <N>] [--seed <N>]"
}

fn parse_arguments() -> Result<(Workload, usize, Representation, u32, u64, u64), String> {
    let mut workload = None;
    let mut world = None;
    let mut representation = None;
    let mut repetitions = None;
    let mut warmup = None;
    let mut timed_generations = None;
    let mut generation = 0u64;
    let mut density_ppm = None;
    let mut seed = 0x5eed_cafe_u64;
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        let mut value = || arguments.next().ok_or_else(|| usage().to_owned());
        match argument.as_str() {
            "--workload" => workload = Some(value()?),
            "--world" => world = Some(value()?.parse().map_err(|_| usage().to_owned())?),
            "--representation" => {
                representation = Some(match value()?.as_str() {
                    "dense" => Representation::Dense,
                    "sparse" => Representation::Sparse,
                    "both" => Representation::Both,
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
            "--help" | "-h" => return Err(usage().to_owned()),
            _ => return Err(usage().to_owned()),
        }
    }
    let workload = match workload.as_deref() {
        Some("canonical") => Workload::Canonical { generation },
        Some("synthetic") => Workload::Synthetic {
            density_ppm: density_ppm
                .ok_or_else(|| "synthetic workload requires --density-ppm".to_owned())?,
            seed,
        },
        _ => return Err(usage().to_owned()),
    };
    Ok((
        workload,
        world.ok_or_else(|| usage().to_owned())?,
        representation.ok_or_else(|| usage().to_owned())?,
        repetitions.ok_or_else(|| usage().to_owned())?,
        warmup.ok_or_else(|| usage().to_owned())?,
        timed_generations.ok_or_else(|| usage().to_owned())?,
    ))
}

fn xorshift64(value: &mut u64) -> u64 {
    *value ^= *value << 13;
    *value ^= *value >> 7;
    *value ^= *value << 17;
    *value
}

fn initial_grid(
    workload: Workload,
    world: usize,
    rule: &LangtonRule,
) -> Result<(DenseGrid, String), String> {
    if world == 0 {
        return Err("world must be positive".to_owned());
    }
    match workload {
        Workload::Canonical { generation } => {
            let seed = CanonicalSeed::canonical().map_err(|error| error.to_string())?;
            let (seed_width, seed_height) = seed.dimensions();
            let initial = seed
                .place_in(
                    world,
                    world,
                    (world - seed_width) / 2,
                    (world - seed_height) / 2,
                )
                .map_err(|error| error.to_string())?;
            Ok((
                initial.run(rule, generation),
                format!("canonical-generation-{generation}"),
            ))
        }
        Workload::Synthetic {
            density_ppm,
            mut seed,
        } => {
            if density_ppm > 1_000_000 {
                return Err("density-ppm must be <= 1000000".to_owned());
            }
            let initial_seed = seed;
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
            Ok((
                grid,
                format!("synthetic-{density_ppm}ppm-seed-{initial_seed}"),
            ))
        }
    }
}

fn run_dense(
    initial: &DenseGrid,
    rule: &LangtonRule,
    warmup: u64,
    generations: u64,
) -> (DenseGrid, u64, u64, u128) {
    let mut runner = DenseRunner::new(initial.clone());
    for _ in 0..warmup {
        runner.step(rule);
    }
    let start = Instant::now();
    let mut active = 0u64;
    let mut evaluations = 0u64;
    for _ in 0..generations {
        let StepStats {
            active_cells_before_step,
            cell_evaluations,
        } = runner.step(rule);
        active += active_cells_before_step as u64;
        evaluations += cell_evaluations as u64;
    }
    (
        runner.into_grid(),
        active,
        evaluations,
        start.elapsed().as_nanos(),
    )
}

fn run_sparse(
    initial: &DenseGrid,
    rule: &LangtonRule,
    warmup: u64,
    generations: u64,
) -> (SparseFrontierGrid, u64, u64, u128) {
    let mut current = SparseFrontierGrid::from_dense(initial);
    for _ in 0..warmup {
        current = current.step(rule).0;
    }
    let start = Instant::now();
    let mut active = 0u64;
    let mut evaluations = 0u64;
    for _ in 0..generations {
        let (next, stats) = current.step(rule);
        active += stats.active_cells_before_step as u64;
        evaluations += stats.cell_evaluations as u64;
        current = next;
    }
    (current, active, evaluations, start.elapsed().as_nanos())
}

fn cpu_model() -> Option<String> {
    fs::read_to_string("/proc/cpuinfo")
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix("model name\t: ").map(str::to_owned))
}

fn perf_status() -> String {
    match fs::read_to_string("/proc/sys/kernel/perf_event_paranoid") {
        Ok(value) => format!("not-collected: perf_event_paranoid={}", value.trim()),
        Err(error) => format!("not-collected: perf status unavailable ({error})"),
    }
}

fn rust_version() -> String {
    option_env!("RUSTC_VERSION")
        .unwrap_or("not-embedded; record cargo/rustc externally")
        .to_owned()
}

fn record(input: RecordInput<'_>) -> BenchmarkRecord {
    let elapsed_seconds = input.elapsed as f64 / 1_000_000_000.0;
    let snapshot = Snapshot::from_grid(input.final_grid, 0);
    let dense_bytes = input.world * input.world * std::mem::size_of::<State>() * 2;
    BenchmarkRecord {
        active_cell_updates: input.active,
        allocated_bytes_estimate: if input.representation == "dense" {
            dense_bytes
        } else {
            input.final_grid.active_cell_count()
                * (std::mem::size_of::<usize>() + std::mem::size_of::<State>())
        },
        boundary: "fixed-quiescent",
        cell_evaluations: input.evaluations,
        code_revision: env::var("RUST_LOOPS_GIT_REVISION").unwrap_or_else(|_| "unknown".to_owned()),
        cpu_model: cpu_model(),
        elapsed_nanoseconds: input.elapsed,
        final_active_cell_count: input.final_grid.active_cell_count(),
        generations_per_second: if input.elapsed == 0 {
            0.0
        } else {
            input.generations as f64 / elapsed_seconds
        },
        initial_active_cell_count: input.initial_active,
        os: env::consts::OS,
        output_state_hash_sha256: snapshot.state_hash_sha256,
        perf_status: perf_status(),
        repetition: input.repetition,
        representation: input.representation,
        rust_version: rust_version(),
        timed_generations: input.generations,
        updates_per_second: if input.elapsed == 0 {
            0.0
        } else {
            input.evaluations as f64 / elapsed_seconds
        },
        useful_active_updates_per_second: if input.elapsed == 0 {
            0.0
        } else {
            input.active as f64 / elapsed_seconds
        },
        workload: input.workload.to_owned(),
        world: [input.world, input.world],
    }
}

fn main() -> ExitCode {
    let (workload, world, representation, repetitions, warmup, generations) =
        match parse_arguments() {
            Ok(config) => config,
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::from(2);
            }
        };
    let rule = match LangtonRule::canonical() {
        Ok(rule) => rule,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::from(1);
        }
    };
    let (initial, workload_name) = match initial_grid(workload, world, &rule) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::from(1);
        }
    };
    for repetition in 0..repetitions {
        let initial_active = initial.active_cell_count();
        let dense = matches!(representation, Representation::Dense | Representation::Both)
            .then(|| run_dense(&initial, &rule, warmup, generations));
        let sparse = matches!(
            representation,
            Representation::Sparse | Representation::Both
        )
        .then(|| run_sparse(&initial, &rule, warmup, generations));
        if let (Some((dense_grid, _, _, _)), Some((sparse_grid, _, _, _))) = (&dense, &sparse)
            && Snapshot::from_grid(dense_grid, 0) != Snapshot::from_grid(&sparse_grid.to_dense(), 0)
        {
            eprintln!("error: dense/sparse mismatch for {workload_name}, repetition {repetition}");
            return ExitCode::from(1);
        }
        for (name, grid, active, evaluations, elapsed) in dense
            .into_iter()
            .map(|(grid, active, evaluations, elapsed)| {
                ("dense", grid, active, evaluations, elapsed)
            })
            .chain(
                sparse
                    .into_iter()
                    .map(|(grid, active, evaluations, elapsed)| {
                        ("sparse", grid.to_dense(), active, evaluations, elapsed)
                    }),
            )
        {
            let output = record(RecordInput {
                representation: name,
                workload: &workload_name,
                world,
                repetition,
                initial_active,
                final_grid: &grid,
                active,
                evaluations,
                elapsed,
                generations,
            });
            println!(
                "{}",
                serde_json::to_string(&output).expect("serializable benchmark record")
            );
        }
    }
    ExitCode::SUCCESS
}
