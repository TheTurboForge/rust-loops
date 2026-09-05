use std::process::{Command, Output};

use serde_json::Value;

fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_benchmark"))
        .args(arguments)
        .env("RUST_LOOPS_GIT_REVISION", "test-revision")
        .output()
        .expect("benchmark process starts")
}

fn records(output: &Output) -> Vec<Value> {
    assert!(
        output.status.success(),
        "benchmark failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone())
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn byl_benchmark_is_profiled_and_exact_across_representations() {
    let output = run(&[
        "--rule",
        "byl-golly-3.3",
        "--workload",
        "canonical",
        "--generation",
        "0",
        "--world",
        "32",
        "--representation",
        "both",
        "--repetitions",
        "1",
        "--warmup",
        "0",
        "--timed-generations",
        "1",
        "--execution-order",
        "7",
    ]);
    let records = records(&output);
    assert_eq!(records.len(), 2);
    assert_eq!(records[0]["representation"], "dense");
    assert_eq!(records[1]["representation"], "sparse");
    for record in &records {
        assert_eq!(record["benchmark_schema_version"], 3);
        assert_eq!(
            record["rule_profile"],
            "byl-1989-golly-3.3-executable-reference"
        );
        assert_eq!(record["initial_generation"], 0);
        assert_eq!(record["timed_start_generation"], 0);
        assert_eq!(record["final_generation"], 1);
        assert_eq!(record["execution_order"], 7);
        assert_eq!(
            record["initial_state_hash_sha256"],
            "1bc12780715f6b52ed939f7de7abb6805a9a658cc4aebbf3ceff5ccddb3fad8a"
        );
        assert_eq!(
            record["output_state_hash_sha256"],
            "a24a38b173dbdee0167b6db9fd8cc705ec5da534594a9dc48246268bc5522dc6"
        );
        assert_eq!(record["final_active_cell_count"], 13);
        assert_eq!(record["final_spatial"]["touches_boundary"], false);
    }
    assert_eq!(
        records[0]["output_state_hash_sha256"],
        records[1]["output_state_hash_sha256"]
    );
}

#[test]
fn omitted_rule_preserves_langton_benchmark_compatibility() {
    let output = run(&[
        "--workload",
        "canonical",
        "--generation",
        "0",
        "--world",
        "32",
        "--representation",
        "dense",
        "--repetitions",
        "1",
        "--warmup",
        "0",
        "--timed-generations",
        "1",
    ]);
    let records = records(&output);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["rule_profile"], "langton-1984-canonical");
    assert_eq!(records[0]["workload"], "canonical-generation-0");
}

#[test]
fn byl_rejects_the_langton_only_synthetic_workload() {
    let output = run(&[
        "--rule",
        "byl-golly-3.3",
        "--workload",
        "synthetic",
        "--density-ppm",
        "1000",
        "--world",
        "32",
        "--representation",
        "dense",
        "--repetitions",
        "1",
        "--warmup",
        "0",
        "--timed-generations",
        "1",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("synthetic workload is defined only for the Langton profile")
    );
}

#[test]
fn too_small_world_is_an_error_not_an_underflow() {
    let output = run(&[
        "--rule",
        "byl-golly-3.3",
        "--workload",
        "canonical",
        "--world",
        "3",
        "--representation",
        "dense",
        "--repetitions",
        "1",
        "--warmup",
        "0",
        "--timed-generations",
        "1",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("world is smaller than the Byl seed"));
}
