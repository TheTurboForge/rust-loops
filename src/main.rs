use std::env;
use std::process::ExitCode;

use rust_loops::{
    BylRule, BylSeed, CanonicalSeed, DenseGrid, LangtonRule, LocalRule, SdsrRule, SdsrSeed,
    Snapshot,
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
    snapshot: Snapshot,
    world: [usize; 2],
}

fn usage() -> &'static str {
    "usage: rust-loops --generations <N> [--rule langton|byl-golly-3.3|sdsr-golly-3.3] [--width <N> --height <N> --origin-x <N> --origin-y <N>]"
}

fn run_profile<R: LocalRule>(grid: DenseGrid, rule: &R, generations: u64) -> Snapshot {
    Snapshot::from_grid_for_rule(&grid.run(rule, generations), generations, rule)
}

fn main() -> ExitCode {
    let mut generations = None;
    let mut width = 128usize;
    let mut height = 128usize;
    let mut origin_x = 32usize;
    let mut origin_y = 32usize;
    let mut rule_profile = "langton";
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        let value = match argument.as_str() {
            "--generations" => &mut generations,
            "--rule" => {
                rule_profile = match arguments.next().as_deref() {
                    Some("langton") => "langton",
                    Some("byl-golly-3.3") => "byl-golly-3.3",
                    Some("sdsr-golly-3.3") => "sdsr-golly-3.3",
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
        let (profile, rule_fixture_sha256, seed_fixture_sha256, snapshot) = match rule_profile {
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
                    run_profile(grid, &rule, generations),
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
                    run_profile(grid, &rule, generations),
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
                    run_profile(grid, &rule, generations),
                )
            }
            _ => unreachable!("validated rule profile"),
        };
        Ok::<_, String>(RunOutput {
            boundary: "fixed-quiescent",
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
