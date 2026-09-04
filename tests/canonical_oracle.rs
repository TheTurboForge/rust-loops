use rust_loops::{
    CanonicalSeed, DenseGrid, LangtonRule, Neighborhood, Snapshot, State, verify_canonical,
};

fn oracle(generation: u16) -> Snapshot {
    serde_json::from_str(match generation {
        0 => include_str!("../data/oracles/golly-3.3/generation-000.json"),
        1 => include_str!("../data/oracles/golly-3.3/generation-001.json"),
        25 => include_str!("../data/oracles/golly-3.3/generation-025.json"),
        75 => include_str!("../data/oracles/golly-3.3/generation-075.json"),
        151 => include_str!("../data/oracles/golly-3.3/generation-151.json"),
        _ => panic!("unknown oracle generation"),
    })
    .expect("valid frozen Golly fixture")
}

#[test]
fn canonical_matches_independent_golly_oracles() {
    for generation in [0, 1, 25, 75, 151] {
        verify_canonical(generation, &oracle(generation as u16)).unwrap();
    }
}

#[test]
fn canonical_seed_is_complete_and_fits() {
    let seed = CanonicalSeed::canonical().unwrap();
    assert_eq!(seed.active_cell_count(), 86);
    assert_eq!(seed.dimensions(), (15, 10));
    assert!(seed.place_in(46, 46, 32, 32).is_err());
}

#[test]
fn state_validation_rejects_out_of_range_values() {
    assert!(State::try_from(7).is_ok());
    assert!(State::try_from(8).is_err());
}

#[test]
fn quiescent_world_is_stable() {
    let rule = LangtonRule::canonical().unwrap();
    let grid = DenseGrid::new(5, 5).unwrap();
    assert_eq!(grid, grid.step(&rule));
}

#[test]
fn rotations_produce_the_same_transition() {
    let rule = LangtonRule::canonical().unwrap();
    let n = |value| State::try_from(value).unwrap();
    let base = Neighborhood {
        center: n(0),
        north: n(0),
        east: n(0),
        south: n(0),
        west: n(1),
    };
    let expected = rule.next_state(base);
    let clockwise = Neighborhood {
        center: base.center,
        north: base.west,
        east: base.north,
        south: base.east,
        west: base.south,
    };
    assert_eq!(expected, rule.next_state(clockwise));
}

#[test]
fn conflicting_rotations_are_rejected() {
    let n = |value| State::try_from(value).unwrap();
    let base = Neighborhood {
        center: n(0),
        north: n(1),
        east: n(2),
        south: n(3),
        west: n(4),
    };
    let rotated = Neighborhood {
        center: base.center,
        north: base.west,
        east: base.north,
        south: base.east,
        west: base.south,
    };
    assert!(LangtonRule::from_base_transitions(vec![(base, n(1)), (rotated, n(2))]).is_err());
}

#[test]
fn duplicate_base_transitions_are_rejected() {
    let n = |value| State::try_from(value).unwrap();
    let base = Neighborhood {
        center: n(0),
        north: n(1),
        east: n(2),
        south: n(3),
        west: n(4),
    };
    assert!(LangtonRule::from_base_transitions(vec![(base, n(1)), (base, n(1))]).is_err());
}
