use std::env;
use std::process::ExitCode;

use rust_loops::{CanonicalSeed, LangtonRule, Snapshot};
use serde::Serialize;

#[derive(Serialize)]
struct RunOutput {
    boundary: &'static str,
    engine_version: &'static str,
    origin: [usize; 2],
    rule_fixture_sha256: &'static str,
    seed_fixture_sha256: &'static str,
    snapshot: Snapshot,
    world: [usize; 2],
}

fn usage() -> &'static str {
    "usage: rust-loops --generations <N> [--width <N> --height <N> --origin-x <N> --origin-y <N>]"
}

fn main() -> ExitCode {
    let mut generations = None;
    let mut width = 128usize;
    let mut height = 128usize;
    let mut origin_x = 32usize;
    let mut origin_y = 32usize;
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        let value = match argument.as_str() {
            "--generations" => &mut generations,
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
        let rule = LangtonRule::canonical().map_err(|error| error.to_string())?;
        let seed = CanonicalSeed::canonical().map_err(|error| error.to_string())?;
        let grid = seed
            .place_in(width, height, origin_x, origin_y)
            .map_err(|error| error.to_string())?;
        Ok::<_, String>(RunOutput {
            boundary: "fixed-quiescent",
            engine_version: env!("CARGO_PKG_VERSION"),
            origin: [origin_x, origin_y],
            rule_fixture_sha256: "c0ca8e6c9218ddd2603905b6dbc5f3170a7f3f32405b8ad5290b1e396018ee92",
            seed_fixture_sha256: "46faf1f1c0c4b966f139c59134cc00697b45f4156a1f51a6c4fa2e3d27d5bda1",
            snapshot: Snapshot::from_grid(&grid.run(&rule, generations), generations),
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
