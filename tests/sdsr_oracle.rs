use rust_loops::{
    ChunkedGrid, DenseGrid, ExtendedSrControlRule, LocalRule, Neighborhood, SDSR_COORDINATE_BASIS,
    SDSR_STATE_COUNT, SdsrRule, SdsrSeed, Snapshot, SparseFrontierGrid, State,
    verify_sdsr_golly_profile,
};
use serde::Deserialize;
use serde_json::Value;
use std::process::Command;

#[derive(Deserialize)]
struct OracleDocument {
    profile: String,
    snapshots: Vec<Snapshot>,
}

fn oracles() -> OracleDocument {
    serde_json::from_str(include_str!(
        "../data/sdsr-golly-3.3/oracles-golly-3.3.json"
    ))
    .expect("valid frozen Golly SDSR fixture")
}

fn state(value: u8) -> State {
    State::try_from(value).unwrap()
}

fn rotate_clockwise(neighborhood: Neighborhood) -> Neighborhood {
    Neighborhood {
        center: neighborhood.center,
        north: neighborhood.west,
        east: neighborhood.north,
        south: neighborhood.east,
        west: neighborhood.south,
    }
}

#[test]
fn sdsr_profile_matches_every_independent_golly_oracle() {
    let document = oracles();
    assert_eq!(document.profile, "Golly 3.3 executable SDSR-Loop reference");
    assert_eq!(
        document
            .snapshots
            .iter()
            .map(|snapshot| snapshot.generation)
            .collect::<Vec<_>>(),
        [0, 1, 151, 302, 1_000, 2_000]
    );
    for expected in &document.snapshots {
        verify_sdsr_golly_profile(expected.generation, expected).unwrap();
    }
}

#[test]
fn sdsr_fixture_invariants_and_dissolver_transition_are_explicit() {
    let rule = SdsrRule::golly_3_3_profile().unwrap();
    let seed = SdsrSeed::golly_3_3_profile().unwrap();
    assert_eq!(rule.state_count(), SDSR_STATE_COUNT);
    assert_eq!(rule.coordinate_basis(), SDSR_COORDINATE_BASIS);
    assert_eq!(rule.direct_transition_count(), 9usize.pow(5));
    assert_eq!(seed.active_cell_count(), 86);
    assert_eq!(seed.dimensions(), (15, 10));
    assert!(seed.place_in(20, 20, 6, 6).is_err());

    let mut neighborhood = Neighborhood {
        center: state(1),
        north: state(1),
        east: state(1),
        south: state(5),
        west: state(2),
    };
    for _ in 0..4 {
        assert_eq!(rule.next_state(neighborhood), state(8));
        neighborhood = rotate_clockwise(neighborhood);
    }
}

#[test]
fn sdsr_dense_sparse_and_chunked_match_through_second_replication() {
    let rule = SdsrRule::golly_3_3_profile().unwrap();
    let seed = SdsrSeed::golly_3_3_profile().unwrap();
    let mut dense = seed.place_in(256, 256, 64, 64).unwrap();
    let mut sparse = SparseFrontierGrid::from_dense(&dense);
    let mut chunked = ChunkedGrid::from_dense(&dense);
    for generation in 0..=302 {
        let expected = Snapshot::from_grid_for_rule(&dense, generation, &rule);
        assert_eq!(
            expected,
            Snapshot::from_grid_for_rule(&sparse.to_dense(), generation, &rule),
            "sparse mismatch at SDSR generation {generation}"
        );
        assert_eq!(
            expected,
            Snapshot::from_grid_for_rule(&chunked.to_dense(), generation, &rule),
            "chunked mismatch at SDSR generation {generation}"
        );
        dense = dense.step(&rule);
        sparse = sparse.step(&rule).0;
        chunked = chunked.step(&rule).0;
    }
}

#[test]
fn extended_sr_control_preserves_the_unperturbed_early_trajectory() {
    let sdsr = SdsrRule::golly_3_3_profile().unwrap();
    let control = ExtendedSrControlRule::project_control().unwrap();
    let seed = SdsrSeed::golly_3_3_profile().unwrap();
    let initial = seed.place_in(256, 256, 64, 64).unwrap();
    let sdsr_snapshot = Snapshot::from_grid_for_rule(&initial.run(&sdsr, 302), 302, &sdsr);
    let control_snapshot = Snapshot::from_grid_for_rule(&initial.run(&control, 302), 302, &control);
    assert_eq!(sdsr_snapshot.cells, control_snapshot.cells);
    assert_eq!(sdsr_snapshot.bounds, control_snapshot.bounds);
    assert_eq!(sdsr_snapshot.active_cell_count, 343);
    assert_eq!(sdsr_snapshot.state_populations["8"], 0);
}

#[test]
fn sdsr_quiescent_world_is_stable_and_other_profiles_reject_state_eight() {
    let rule = SdsrRule::golly_3_3_profile().unwrap();
    let grid = DenseGrid::new(5, 5).unwrap();
    assert_eq!(grid, grid.step(&rule));

    let state_eight = Neighborhood {
        center: state(8),
        north: state(0),
        east: state(0),
        south: state(0),
        west: state(0),
    };
    assert!(
        std::panic::catch_unwind(|| {
            rust_loops::LangtonRule::canonical()
                .unwrap()
                .next_state(state_eight)
        })
        .is_err()
    );
    assert!(
        std::panic::catch_unwind(|| {
            rust_loops::BylRule::golly_3_3_profile()
                .unwrap()
                .next_state(state_eight)
        })
        .is_err()
    );
}

#[test]
fn public_cli_emits_the_named_sdsr_profile() {
    let output = Command::new(env!("CARGO_BIN_EXE_rust-loops"))
        .args(["--rule", "sdsr-golly-3.3", "--generations", "151"])
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
        "sdsr-golly-3.3-executable-reference"
    );
    assert_eq!(document["snapshot"]["generation"], 151);
    assert_eq!(
        document["snapshot"]["state_hash_sha256"],
        "3ba16165709a79862010bdbde104035a9cfebdc0449fa7b0613bf0c45d29f246"
    );
}

#[test]
#[ignore = "release-mode exhaustive representation gate"]
fn all_representations_match_generation_by_generation_through_2000() {
    let rule = SdsrRule::golly_3_3_profile().unwrap();
    let seed = SdsrSeed::golly_3_3_profile().unwrap();
    let mut dense = seed.place_in(768, 768, 256, 256).unwrap();
    let mut sparse = SparseFrontierGrid::from_dense(&dense);
    let mut chunked = ChunkedGrid::from_dense(&dense);
    for generation in 0..=2_000 {
        assert!(
            sparse.state_equals_dense(&dense),
            "sparse generation {generation}"
        );
        assert!(
            chunked.state_equals_dense(&dense),
            "chunked generation {generation}"
        );
        if generation < 2_000 {
            dense = dense.step(&rule);
            sparse = sparse.step(&rule).0;
            chunked = chunked.step(&rule).0;
        }
    }
    let expected = oracles().snapshots.pop().unwrap();
    assert_eq!(Snapshot::from_grid_for_rule(&dense, 2_000, &rule), expected);
}
