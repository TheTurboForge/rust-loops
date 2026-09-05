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
        "all",
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
    assert_eq!(records.len(), 3);
    assert_eq!(records[0]["representation"], "dense");
    assert_eq!(records[1]["representation"], "sparse");
    assert_eq!(records[2]["representation"], "chunked-32");
    for record in &records {
        assert_eq!(record["benchmark_schema_version"], 4);
        assert_eq!(
            record["rule_profile"],
            "byl-1989-golly-3.3-executable-reference"
        );
        assert_eq!(record["initial_generation"], 0);
        assert_eq!(record["timed_start_generation"], 0);
        assert_eq!(record["final_generation"], 1);
        assert_eq!(record["execution_order"], 7);
        assert_eq!(record["layout"], "uniform");
        assert_eq!(record["synthetic_seed"], Value::Null);
        assert_eq!(
            record["representation_fixture_ownership"],
            "owned; baseline-before-fixture; dense-dropped-after-conversion"
        );
        assert!(record["baseline_resident_bytes"].is_number());
        assert!(record["current_resident_delta_bytes"].is_number());
        assert!(record["peak_resident_delta_bytes"].is_number());
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
    assert_eq!(
        records[0]["output_state_hash_sha256"],
        records[2]["output_state_hash_sha256"]
    );
    assert_eq!(records[2]["chunk_side"], 32);
    assert!(records[2]["chunk_evaluations"].is_number());
    assert!(records[2]["allocated_chunks_peak"].is_number());
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

#[test]
fn clustered_identity_is_preserved_across_all_representations() {
    let output = run(&[
        "--workload",
        "identity",
        "--density-ppm",
        "50000",
        "--seed",
        "1592639710",
        "--layout",
        "clustered",
        "--world",
        "64",
        "--representation",
        "all",
        "--repetitions",
        "1",
        "--warmup",
        "2",
        "--timed-generations",
        "3",
    ]);
    let records = records(&output);
    assert_eq!(records.len(), 3);
    for record in &records {
        assert_eq!(record["benchmark_schema_version"], 4);
        assert_eq!(record["rule_profile"], "benchmark-identity-v1");
        assert_eq!(record["layout"], "clustered");
        assert_eq!(record["synthetic_seed"], 1_592_639_710u64);
        assert_eq!(record["initial_generation"], 0);
        assert_eq!(record["timed_start_generation"], 2);
        assert_eq!(record["final_generation"], 5);
        assert_eq!(
            record["initial_state_hash_sha256"],
            record["timed_start_state_hash_sha256"]
        );
        assert_eq!(
            record["initial_state_hash_sha256"],
            record["output_state_hash_sha256"]
        );
        assert_eq!(
            record["initial_active_cell_count"],
            record["final_active_cell_count"]
        );
    }
}

#[test]
fn identity_rejects_a_scientific_rule_profile() {
    let output = run(&[
        "--rule",
        "byl-golly-3.3",
        "--workload",
        "identity",
        "--density-ppm",
        "1000",
        "--world",
        "32",
        "--representation",
        "chunked",
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
            .contains("identity workload does not accept a scientific rule profile")
    );
}
