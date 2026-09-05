use rust_loops::{
    ChunkedGrid, DenseGrid, EVOLOOP_COORDINATE_BASIS, EVOLOOP_STATE_COUNT, EvoloopRule,
    EvoloopSeed, LocalRule, Neighborhood, Snapshot, SparseFrontierGrid, State, ToroidalDenseRunner,
    ToroidalSnapshot, verify_evoloop_golly_profile,
};
use serde::Deserialize;
use serde_json::Value;
use std::process::Command;

#[derive(Deserialize)]
struct OracleDocument {
    profile: String,
    snapshots: Vec<Snapshot>,
}

#[derive(Deserialize)]
struct ToroidalOracleDocument {
    profile: String,
    snapshots: Vec<ToroidalSnapshot>,
}

fn oracles() -> OracleDocument {
    serde_json::from_str(include_str!(
        "../data/evoloop-golly-3.3/oracles-golly-3.3.json"
    ))
    .expect("valid frozen Golly Evoloop fixture")
}

fn toroidal_oracles() -> ToroidalOracleDocument {
    serde_json::from_str(include_str!(
        "../data/evoloop-golly-3.3/oracles-t200x200-golly-3.3.json"
    ))
    .expect("valid frozen Golly Evoloop torus fixture")
}

#[test]
fn evoloop_profile_matches_every_independent_unbounded_golly_oracle() {
    let document = oracles();
    assert_eq!(document.profile, "Golly 3.3 executable Evoloop reference");
    assert_eq!(
        document
            .snapshots
            .iter()
            .map(|snapshot| snapshot.generation)
            .collect::<Vec<_>>(),
        [0, 1, 151, 302, 500, 1_000, 2_000]
    );
    for expected in &document.snapshots {
        verify_evoloop_golly_profile(expected.generation, expected).unwrap();
    }
}

#[test]
fn evoloop_fixture_invariants_are_explicit() {
    let rule = EvoloopRule::golly_3_3_profile().unwrap();
    let seed = EvoloopSeed::golly_3_3_profile().unwrap();
    assert_eq!(rule.state_count(), EVOLOOP_STATE_COUNT);
    assert_eq!(rule.coordinate_basis(), EVOLOOP_COORDINATE_BASIS);
    assert_eq!(rule.direct_transition_count(), 9usize.pow(5));
    assert_eq!(seed.active_cell_count(), 149);
    assert_eq!(seed.dimensions(), (17, 17));
    assert!(seed.place_in(20, 20, 4, 4).is_err());
}

#[test]
fn evoloop_dense_sparse_and_chunked_match_through_generation_302() {
    let rule = EvoloopRule::golly_3_3_profile().unwrap();
    let seed = EvoloopSeed::golly_3_3_profile().unwrap();
    let mut dense = seed.place_in(512, 512, 128, 128).unwrap();
    let mut sparse = SparseFrontierGrid::from_dense(&dense);
    let mut chunked = ChunkedGrid::from_dense(&dense);
    for generation in 0..=302 {
        let expected = Snapshot::from_grid_for_rule(&dense, generation, &rule);
        assert_eq!(
            expected,
            Snapshot::from_grid_for_rule(&sparse.to_dense(), generation, &rule),
            "sparse mismatch at Evoloop generation {generation}"
        );
        assert_eq!(
            expected,
            Snapshot::from_grid_for_rule(&chunked.to_dense(), generation, &rule),
            "chunked mismatch at Evoloop generation {generation}"
        );
        dense = dense.step(&rule);
        sparse = sparse.step(&rule).0;
        chunked = chunked.step(&rule).0;
    }
}

#[derive(Clone, Copy)]
struct CopyNorth;

impl LocalRule for CopyNorth {
    fn next_state(&self, neighborhood: Neighborhood) -> State {
        neighborhood.north
    }

    fn state_count(&self) -> u8 {
        2
    }

    fn coordinate_basis(&self) -> &'static str {
        "test"
    }
}

#[derive(Clone, Copy)]
struct AnyNeighbor;

impl LocalRule for AnyNeighbor {
    fn next_state(&self, neighborhood: Neighborhood) -> State {
        State::try_from(u8::from(
            [
                neighborhood.north,
                neighborhood.east,
                neighborhood.south,
                neighborhood.west,
            ]
            .into_iter()
            .any(|state| state != State::QUIESCENT),
        ))
        .unwrap()
    }

    fn state_count(&self) -> u8 {
        2
    }

    fn coordinate_basis(&self) -> &'static str {
        "test"
    }
}

#[test]
fn toroidal_runner_wraps_without_changing_fixed_boundary_semantics() {
    let mut initial = DenseGrid::new(3, 3).unwrap();
    initial.set(1, 2, State::try_from(1).unwrap()).unwrap();
    let fixed = initial.step(&CopyNorth);
    assert_eq!(fixed.active_cell_count(), 0);

    let mut toroidal = ToroidalDenseRunner::new(initial);
    let stats = toroidal.step(&CopyNorth);
    assert_eq!(stats.cell_evaluations, 9);
    assert_eq!(toroidal.grid().active_cell_count(), 1);
    assert_eq!(toroidal.grid().get(1, 0).unwrap().value(), 1);
    let snapshot = ToroidalSnapshot::from_grid_for_rule(toroidal.grid(), 1, &CopyNorth);
    assert_eq!(snapshot.world, [3, 3]);
    assert_eq!(snapshot.cells[0].x, 1);
    assert_eq!(snapshot.cells[0].y, 0);
}

#[test]
fn toroidal_runner_wraps_all_four_edges_on_a_nonsquare_world() {
    let mut initial = DenseGrid::new(4, 3).unwrap();
    initial.set(0, 0, State::try_from(1).unwrap()).unwrap();
    let mut runner = ToroidalDenseRunner::new(initial);
    runner.step(&AnyNeighbor);
    let snapshot = ToroidalSnapshot::from_grid_for_rule(runner.grid(), 1, &AnyNeighbor);
    assert_eq!(
        snapshot.cells,
        [
            rust_loops::Cell {
                state: 1,
                x: 1,
                y: 0
            },
            rust_loops::Cell {
                state: 1,
                x: 3,
                y: 0
            },
            rust_loops::Cell {
                state: 1,
                x: 0,
                y: 1
            },
            rust_loops::Cell {
                state: 1,
                x: 0,
                y: 2
            },
        ]
    );
}

#[test]
fn public_cli_emits_the_named_evoloop_profile() {
    let output = Command::new(env!("CARGO_BIN_EXE_rust-loops"))
        .args(["--rule", "evoloop-golly-3.3", "--generations", "151"])
        .output()
        .expect("rust-loops CLI starts");
    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        document["rule_profile"],
        "evoloop-golly-3.3-executable-reference"
    );
    assert_eq!(document["snapshot"]["generation"], 151);
    assert_eq!(
        document["snapshot"]["state_hash_sha256"],
        "7663533720bd41909c6243976ec9806944f8330c7adcdb1047fa2cbe5c4d9cbc"
    );
}

#[test]
fn public_cli_emits_an_explicit_toroidal_snapshot() {
    let output = Command::new(env!("CARGO_BIN_EXE_rust-loops"))
        .args([
            "--rule",
            "evoloop-golly-3.3",
            "--boundary",
            "toroidal",
            "--generations",
            "0",
            "--width",
            "200",
            "--height",
            "200",
            "--origin-x",
            "92",
            "--origin-y",
            "92",
        ])
        .output()
        .expect("rust-loops CLI starts");
    assert!(output.status.success());
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["boundary"], "toroidal");
    assert_eq!(document["snapshot"]["world"], serde_json::json!([200, 200]));
    assert_eq!(
        document["snapshot"]["state_hash_sha256"],
        "fbcaabaa24c80b7f025bcdff9be61eabce8ce63ca93164ed343b4fda1fc7c081"
    );
}

#[test]
#[ignore = "release-mode 50,000-generation independent Golly torus gate"]
fn toroidal_trajectory_matches_every_independent_golly_milestone() {
    let document = toroidal_oracles();
    assert_eq!(
        document.profile,
        "Golly 3.3 executable Evoloop T200,200 reference"
    );
    let rule = EvoloopRule::golly_3_3_profile().unwrap();
    let seed = EvoloopSeed::golly_3_3_profile().unwrap();
    let initial = seed.place_in(200, 200, 92, 92).unwrap();
    let mut runner = ToroidalDenseRunner::new(initial);
    let mut generation = 0u64;
    for expected in &document.snapshots {
        runner.run(&rule, expected.generation - generation);
        generation = expected.generation;
        let actual = ToroidalSnapshot::from_grid_for_rule(runner.grid(), generation, &rule);
        assert_eq!(
            actual, *expected,
            "toroidal mismatch at generation {generation}"
        );
    }
}

#[test]
#[ignore = "release-mode exhaustive representation gate"]
fn all_representations_match_generation_by_generation_through_2000() {
    let rule = EvoloopRule::golly_3_3_profile().unwrap();
    let seed = EvoloopSeed::golly_3_3_profile().unwrap();
    let mut dense = seed.place_in(768, 768, 256, 256).unwrap();
    let mut sparse = SparseFrontierGrid::from_dense(&dense);
    let mut chunked = ChunkedGrid::from_dense(&dense);
    for generation in 0..=2_000 {
        assert_eq!(dense, sparse.to_dense(), "sparse generation {generation}");
        assert_eq!(dense, chunked.to_dense(), "chunked generation {generation}");
        dense = dense.step(&rule);
        sparse = sparse.step(&rule).0;
        chunked = chunked.step(&rule).0;
    }
}
