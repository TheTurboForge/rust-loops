use std::env;
use std::process::ExitCode;

use rust_loops::{
    BylRule, BylSeed, CanonicalSeed, DenseGrid, EvoloopRule, EvoloopSeed, LangtonRule, LocalRule,
    SdsrRule, SdsrSeed, Snapshot, ToroidalDenseRunner, ToroidalSnapshot,
};
use serde::Serialize;

#[derive(Serialize)]
struct RunOutput {
    boundary: &'static str,
    engine_version: &'static str,
    origin: [usize; 2],
    rule_profile: &'static str,
    rule_fixture_sha256: &'static str,
    seed_fixture_sha256: &'static str,
    snapshot: SnapshotOutput,
    world: [usize; 2],
}

#[derive(Serialize)]
#[serde(untagged)]
enum SnapshotOutput {
    Fixed(Snapshot),
    Toroidal(ToroidalSnapshot),
}

fn usage() -> &'static str {
    "usage: rust-loops --generations <N> [--rule langton|byl-golly-3.3|sdsr-golly-3.3|evoloop-golly-3.3] [--boundary fixed-quiescent|toroidal] [--width <N> --height <N> --origin-x <N> --origin-y <N>]"
}

fn run_profile<R: LocalRule + ?Sized>(grid: DenseGrid, rule: &R, generations: u64) -> Snapshot {
    Snapshot::from_grid_for_rule(&grid.run(rule, generations), generations, rule)
}

fn run_toroidal_profile<R: LocalRule + ?Sized>(
    grid: DenseGrid,
    rule: &R,
    generations: u64,
) -> ToroidalSnapshot {
    let mut runner = ToroidalDenseRunner::new(grid);
    runner.run(rule, generations);
    ToroidalSnapshot::from_grid_for_rule(runner.grid(), generations, rule)
}

fn main() -> ExitCode {
    let mut generations = None;
    let mut width = 128usize;
    let mut height = 128usize;
    let mut origin_x = 32usize;
    let mut origin_y = 32usize;
    let mut rule_profile = "langton";
    let mut boundary = "fixed-quiescent";
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        let value = match argument.as_str() {
            "--generations" => &mut generations,
            "--rule" => {
                rule_profile = match arguments.next().as_deref() {
                    Some("langton") => "langton",
                    Some("byl-golly-3.3") => "byl-golly-3.3",
                    Some("sdsr-golly-3.3") => "sdsr-golly-3.3",
                    Some("evoloop-golly-3.3") => "evoloop-golly-3.3",
                    _ => {
                        eprintln!("{}", usage());
                        return ExitCode::from(2);
                    }
                };
                continue;
            }
            "--boundary" => {
                boundary = match arguments.next().as_deref() {
                    Some("fixed-quiescent") => "fixed-quiescent",
                    Some("toroidal") => "toroidal",
                    _ => {
                        eprintln!("{}", usage());
                        return ExitCode::from(2);
                    }
                };
                continue;
            }
            "--width" => {
                width = match arguments.next().and_then(|value| value.parse().ok()) {
                    Some(value) => value,
                    None => {
                        eprintln!("{}", usage());
                        return ExitCode::from(2);
                    }
                };
                continue;
            }
            "--height" => {
                height = match arguments.next().and_then(|value| value.parse().ok()) {
                    Some(value) => value,
                    None => {
                        eprintln!("{}", usage());
                        return ExitCode::from(2);
                    }
                };
                continue;
            }
            "--origin-x" => {
                origin_x = match arguments.next().and_then(|value| value.parse().ok()) {
                    Some(value) => value,
                    None => {
                        eprintln!("{}", usage());
                        return ExitCode::from(2);
                    }
                };
                continue;
            }
            "--origin-y" => {
                origin_y = match arguments.next().and_then(|value| value.parse().ok()) {
                    Some(value) => value,
                    None => {
                        eprintln!("{}", usage());
                        return ExitCode::from(2);
                    }
                };
                continue;
            }
            "--help" | "-h" => {
                println!("{}", usage());
                return ExitCode::SUCCESS;
            }
            _ => {
                eprintln!("{}", usage());
                return ExitCode::from(2);
            }
        };
        *value = match arguments.next().and_then(|value| value.parse().ok()) {
            Some(value) => Some(value),
            None => {
                eprintln!("{}", usage());
                return ExitCode::from(2);
            }
        };
    }
    let Some(generations) = generations else {
        eprintln!("{}", usage());
        return ExitCode::from(2);
    };
    let result = (|| {
        let (profile, rule_fixture_sha256, seed_fixture_sha256, grid, rule): (
            _,
            _,
            _,
            _,
            Box<dyn LocalRule>,
        ) = match rule_profile {
            "langton" => {
                let rule = LangtonRule::canonical().map_err(|error| error.to_string())?;
                let seed = CanonicalSeed::canonical().map_err(|error| error.to_string())?;
                let grid = seed
                    .place_in(width, height, origin_x, origin_y)
                    .map_err(|error| error.to_string())?;
                (
                    "langton-1984-canonical",
                    "c0ca8e6c9218ddd2603905b6dbc5f3170a7f3f32405b8ad5290b1e396018ee92",
                    "46faf1f1c0c4b966f139c59134cc00697b45f4156a1f51a6c4fa2e3d27d5bda1",
                    grid,
                    Box::new(rule) as Box<dyn LocalRule>,
                )
            }
            "byl-golly-3.3" => {
                let rule = BylRule::golly_3_3_profile().map_err(|error| error.to_string())?;
                let seed = BylSeed::golly_3_3_profile().map_err(|error| error.to_string())?;
                let grid = seed
                    .place_in(width, height, origin_x, origin_y)
                    .map_err(|error| error.to_string())?;
                (
                    "byl-1989-golly-3.3-executable-reference",
                    "8813815e3af17aa71ce351bfa69358b3eb64ecf38eb44f739142e3f4595d84be",
                    "0854641da00edc65974ac7a79d79b7c5fabf171946bffdbf1b0ba38a9662892f",
                    grid,
                    Box::new(rule) as Box<dyn LocalRule>,
                )
            }
            "sdsr-golly-3.3" => {
                let rule = SdsrRule::golly_3_3_profile().map_err(|error| error.to_string())?;
                let seed = SdsrSeed::golly_3_3_profile().map_err(|error| error.to_string())?;
                let grid = seed
                    .place_in(width, height, origin_x, origin_y)
                    .map_err(|error| error.to_string())?;
                (
                    "sdsr-golly-3.3-executable-reference",
                    SdsrRule::LOOKUP_SHA256,
                    SdsrSeed::SEED_SHA256,
                    grid,
                    Box::new(rule) as Box<dyn LocalRule>,
                )
            }
            "evoloop-golly-3.3" => {
                let rule = EvoloopRule::golly_3_3_profile().map_err(|error| error.to_string())?;
                let seed = EvoloopSeed::golly_3_3_profile().map_err(|error| error.to_string())?;
                let grid = seed
                    .place_in(width, height, origin_x, origin_y)
                    .map_err(|error| error.to_string())?;
                (
                    "evoloop-golly-3.3-executable-reference",
                    EvoloopRule::LOOKUP_SHA256,
                    EvoloopSeed::SEED_SHA256,
                    grid,
                    Box::new(rule) as Box<dyn LocalRule>,
                )
            }
            _ => unreachable!("validated rule profile"),
        };
        let snapshot = if boundary == "toroidal" {
            SnapshotOutput::Toroidal(run_toroidal_profile(grid, rule.as_ref(), generations))
        } else {
            SnapshotOutput::Fixed(run_profile(grid, rule.as_ref(), generations))
        };
        Ok::<_, String>(RunOutput {
            boundary,
            engine_version: env!("CARGO_PKG_VERSION"),
            origin: [origin_x, origin_y],
            rule_profile: profile,
            rule_fixture_sha256,
            seed_fixture_sha256,
            snapshot,
            world: [width, height],
        })
    })();
    match result {
        Ok(snapshot) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&snapshot).expect("serializable snapshot")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(1)
        }
    }
}
