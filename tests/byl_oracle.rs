use rust_loops::{
    BylRule, BylSeed, DenseGrid, LocalRule, Neighborhood, RuleError, Snapshot, SparseFrontierGrid,
    State, verify_byl_golly_profile,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct OracleDocument {
    profile: String,
    snapshots: Vec<Snapshot>,
}

fn oracles() -> OracleDocument {
    serde_json::from_str(include_str!("../data/byl-golly-3.3/oracles-golly-3.3.json"))
        .expect("valid frozen Golly Byl fixture")
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
fn byl_profile_matches_every_independent_golly_oracle() {
    let document = oracles();
    assert_eq!(document.profile, "Golly 3.3 executable Byl-Loop reference");
    assert_eq!(document.snapshots.len(), 29);
    assert_eq!(
        document
            .snapshots
            .iter()
            .map(|snapshot| snapshot.generation)
            .collect::<Vec<_>>(),
        (0..=27).chain([50]).collect::<Vec<_>>()
    );

    for expected in &document.snapshots {
        verify_byl_golly_profile(expected.generation, expected).unwrap();
    }
}

#[test]
fn byl_generation_25_is_the_frozen_first_replication_milestone() {
    let expected = oracles()
        .snapshots
        .into_iter()
        .find(|snapshot| snapshot.generation == 25)
        .unwrap();
    assert_eq!(expected.active_cell_count, 25);
    assert_eq!((expected.bounds.width, expected.bounds.height), (9, 4));
    assert_eq!(expected.state_populations["1"], 2);
    assert_eq!(expected.state_populations["2"], 15);
    assert_eq!(expected.state_populations["3"], 4);
    assert_eq!(expected.state_populations["4"], 2);
    assert_eq!(expected.state_populations["5"], 2);
    assert_eq!(
        expected.state_hash_sha256,
        "3f46c0896951a1d9a53395efe812a44570a8170659131765a6e04caa34419ef4"
    );
}

#[test]
fn byl_fixture_invariants_are_explicit() {
    let rule = BylRule::golly_3_3_profile().unwrap();
    let seed = BylSeed::golly_3_3_profile().unwrap();
    assert_eq!(rule.state_count(), 6);
    assert_eq!(rule.expanded_transition_count(), 561);
    assert_eq!(seed.active_cell_count(), 12);
    assert_eq!(seed.dimensions(), (4, 4));
    assert!(seed.place_in(35, 35, 32, 32).is_err());
}

#[test]
fn byl_uses_fourfold_rotation_and_center_preserving_fallback() {
    let rule = BylRule::golly_3_3_profile().unwrap();
    let mut neighborhood = Neighborhood {
        center: state(0),
        north: state(0),
        east: state(0),
        south: state(0),
        west: state(3),
    };
    for _ in 0..4 {
        assert_eq!(rule.next_state(neighborhood), state(1));
        neighborhood = rotate_clockwise(neighborhood);
    }

    let unmatched = Neighborhood {
        center: state(1),
        north: state(0),
        east: state(0),
        south: state(0),
        west: state(2),
    };
    assert_eq!(rule.next_state(unmatched), state(1));
}

#[test]
fn byl_rejects_rotational_conflicts_and_out_of_profile_rules() {
    let base = Neighborhood {
        center: state(0),
        north: state(1),
        east: state(2),
        south: state(3),
        west: state(4),
    };
    let conflict =
        BylRule::from_base_transitions(vec![(base, state(1)), (rotate_clockwise(base), state(2))]);
    assert!(matches!(
        conflict,
        Err(RuleError::ConflictingRotation { .. })
    ));

    let outside = BylRule::from_base_transitions(vec![(
        Neighborhood {
            center: state(6),
            north: state(0),
            east: state(0),
            south: state(0),
            west: state(0),
        },
        state(1),
    )]);
    assert_eq!(
        outside.unwrap_err(),
        RuleError::StateOutsideRule {
            state: 6,
            state_count: 6
        }
    );
}

#[test]
fn byl_dense_and_sparse_trajectories_are_exact_through_generation_50() {
    let rule = BylRule::golly_3_3_profile().unwrap();
    let seed = BylSeed::golly_3_3_profile().unwrap();
    let mut dense = seed.place_in(128, 128, 32, 32).unwrap();
    let mut sparse = SparseFrontierGrid::from_dense(&dense);
    for generation in 0..=50 {
        assert_eq!(
            Snapshot::from_grid_for_rule(&dense, generation, &rule),
            Snapshot::from_grid_for_rule(&sparse.to_dense(), generation, &rule),
            "representation mismatch at Byl generation {generation}"
        );
        dense = dense.step(&rule);
        sparse = sparse.step(&rule).0;
    }
}

#[test]
fn quiescent_world_is_stable_under_byl_profile() {
    let rule = BylRule::golly_3_3_profile().unwrap();
    let grid = DenseGrid::new(5, 5).unwrap();
    assert_eq!(grid, grid.step(&rule));
}

#[test]
fn byl_execution_refuses_states_six_and_seven() {
    let rule = BylRule::golly_3_3_profile().unwrap();
    for invalid in [6, 7] {
        let result = std::panic::catch_unwind(|| {
            rule.next_state(Neighborhood {
                center: state(invalid),
                north: state(0),
                east: state(0),
                south: state(0),
                west: state(0),
            });
        });
        assert!(result.is_err(), "Byl accepted state {invalid}");
    }
}
