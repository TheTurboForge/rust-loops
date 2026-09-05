use rust_loops::{
    BYL_COORDINATE_BASIS, BylRule, BylSeed, CANONICAL_COORDINATE_BASIS, CANONICAL_STATE_COUNT,
    CanonicalSeed, ChunkedGrid, ChunkedRunner, DenseGrid, DenseRunner, LangtonRule, LocalRule,
    Neighborhood, RuleError, Snapshot, SparseFrontierGrid, State,
};

fn assert_equal<R: LocalRule>(
    dense: &DenseGrid,
    sparse: &SparseFrontierGrid,
    chunked: &ChunkedGrid,
    rule: &R,
    generation: u64,
) {
    let expected = Snapshot::from_grid_for_rule(dense, generation, rule);
    assert_eq!(
        expected,
        Snapshot::from_grid_for_rule(&sparse.to_dense(), generation, rule),
        "sparse state mismatch at generation {generation}"
    );
    assert_eq!(
        expected,
        Snapshot::from_grid_for_rule(&chunked.to_dense(), generation, rule),
        "chunked state mismatch at generation {generation}"
    );
}

#[derive(Clone, Copy)]
struct IdentityRule;

impl LocalRule for IdentityRule {
    fn next_state(&self, neighborhood: Neighborhood) -> State {
        neighborhood.center
    }

    fn state_count(&self) -> u8 {
        CANONICAL_STATE_COUNT
    }

    fn coordinate_basis(&self) -> &'static str {
        CANONICAL_COORDINATE_BASIS
    }
}

#[derive(Clone, Copy)]
struct ActiveBackgroundRule;

impl LocalRule for ActiveBackgroundRule {
    fn next_state(&self, _neighborhood: Neighborhood) -> State {
        State::try_from(1).unwrap()
    }

    fn state_count(&self) -> u8 {
        CANONICAL_STATE_COUNT
    }

    fn coordinate_basis(&self) -> &'static str {
        CANONICAL_COORDINATE_BASIS
    }
}

fn deterministic_field(width: usize, height: usize, seed: u64) -> DenseGrid {
    let mut value = seed;
    let mut grid = DenseGrid::new(width, height).unwrap();
    for y in 0..height {
        for x in 0..width {
            value ^= value << 13;
            value ^= value >> 7;
            value ^= value << 17;
            if value.is_multiple_of(11) {
                grid.set(x, y, State::try_from((value % 7 + 1) as u8).unwrap())
                    .unwrap();
            }
        }
    }
    grid
}

#[test]
fn all_representations_match_every_langton_generation_through_302() {
    let rule = LangtonRule::canonical().unwrap();
    let seed = CanonicalSeed::canonical().unwrap();
    let mut dense = seed.place_in(128, 128, 32, 32).unwrap();
    let mut sparse = SparseFrontierGrid::from_dense(&dense);
    let mut chunked = ChunkedGrid::from_dense(&dense);
    for generation in 0..=302 {
        assert_equal(&dense, &sparse, &chunked, &rule, generation);
        dense = dense.step(&rule);
        sparse = sparse.step(&rule).0;
        chunked = chunked.step(&rule).0;
    }
}

#[test]
fn all_representations_match_every_byl_generation_through_165() {
    let rule = BylRule::golly_3_3_profile().unwrap();
    let seed = BylSeed::golly_3_3_profile().unwrap();
    let mut dense = seed.place_in(256, 256, 96, 96).unwrap();
    let mut sparse = SparseFrontierGrid::from_dense(&dense);
    let mut chunked = ChunkedGrid::from_dense(&dense);
    for generation in 0..=165 {
        assert_equal(&dense, &sparse, &chunked, &rule, generation);
        dense = dense.step(&rule);
        sparse = sparse.step(&rule).0;
        chunked = chunked.step(&rule).0;
    }
    assert_eq!(
        Snapshot::from_grid_for_rule(&dense, 166, &rule).coordinate_basis,
        BYL_COORDINATE_BASIS
    );
}

#[test]
fn chunked_matches_dense_for_deterministic_multistate_fields() {
    let rule = LangtonRule::canonical().unwrap();
    let mut dense = deterministic_field(79, 61, 0x5eed_cafe);
    let mut sparse = SparseFrontierGrid::from_dense(&dense);
    let mut chunked = ChunkedGrid::from_dense(&dense);
    for generation in 0..20 {
        assert_equal(&dense, &sparse, &chunked, &rule, generation);
        dense = dense.step(&rule);
        sparse = sparse.step(&rule).0;
        chunked = chunked.step(&rule).0;
    }
}

#[test]
fn sparse_frontier_handles_edges_and_deduplicates_candidates() {
    let rule = LangtonRule::canonical().unwrap();
    let mut dense = DenseGrid::new(7, 7).unwrap();
    for (x, y, state) in [(0, 0, 1), (1, 0, 2), (0, 1, 7), (6, 6, 4), (5, 6, 2)] {
        dense.set(x, y, State::try_from(state).unwrap()).unwrap();
    }
    let sparse = SparseFrontierGrid::from_dense(&dense);
    let chunked = ChunkedGrid::from_dense(&dense);
    let (next_sparse, stats) = sparse.step(&rule);
    assert!(stats.cell_evaluations < dense.width() * dense.height());
    let next_chunked = chunked.step(&rule).0;
    assert_equal(&dense.step(&rule), &next_sparse, &next_chunked, &rule, 1);
}

#[test]
fn empty_sparse_and_chunked_worlds_remain_empty() {
    let rule = LangtonRule::canonical().unwrap();
    let dense = DenseGrid::new(19, 19).unwrap();
    let sparse = SparseFrontierGrid::from_dense(&dense);
    let chunked = ChunkedGrid::from_dense(&dense);
    let (next_sparse, sparse_stats) = sparse.step(&rule);
    let (next_chunked, chunked_stats) = chunked.step(&rule);
    assert_eq!(sparse_stats.cell_evaluations, 0);
    assert_eq!(chunked_stats.cell_evaluations, 0);
    assert_eq!(next_sparse.active_cell_count(), 0);
    assert_eq!(next_chunked.active_cell_count(), 0);
    assert_equal(&dense, &next_sparse, &next_chunked, &rule, 1);
}

#[test]
fn reusable_dense_runner_matches_repeated_immutable_steps() {
    let rule = LangtonRule::canonical().unwrap();
    let start = deterministic_field(43, 41, 0x1234_5678);
    let mut runner = DenseRunner::new(start.clone());
    let mut reference = start;
    for generation in 1..=12 {
        let stats = runner.step(&rule);
        assert_eq!(stats.cell_evaluations, 43 * 41);
        reference = reference.step(&rule);
        assert_eq!(runner.grid(), &reference, "generation {generation}");
    }
    assert_eq!(runner.logical_storage_bytes(), 43 * 41 * 2);
}

#[test]
fn chunked_halos_cover_all_edges_and_partial_chunks() {
    let rule = LangtonRule::canonical().unwrap();
    let mut dense = DenseGrid::new(65, 67).unwrap();
    for (x, y, state) in [
        (31, 9, 1),
        (32, 9, 2),
        (9, 31, 3),
        (9, 32, 4),
        (63, 40, 5),
        (64, 40, 6),
        (40, 63, 7),
        (40, 64, 1),
        (64, 66, 2),
    ] {
        dense.set(x, y, State::try_from(state).unwrap()).unwrap();
    }
    let mut sparse = SparseFrontierGrid::from_dense(&dense);
    let mut chunked = ChunkedGrid::from_dense(&dense);
    for generation in 0..8 {
        assert_equal(&dense, &sparse, &chunked, &rule, generation);
        dense = dense.step(&rule);
        sparse = sparse.step(&rule).0;
        chunked = chunked.step(&rule).0;
    }
}

#[test]
fn chunked_runner_recycles_without_stale_cells() {
    let rule = LangtonRule::from_base_transitions(Vec::new()).unwrap();
    let mut dense = DenseGrid::new(96, 96).unwrap();
    dense.set(1, 1, State::try_from(7).unwrap()).unwrap();
    dense.set(65, 65, State::try_from(6).unwrap()).unwrap();
    let mut runner = ChunkedRunner::new(ChunkedGrid::from_dense(&dense));
    runner.step(&rule);
    assert_eq!(runner.grid().active_cell_count(), 0);
    runner.step(&rule);
    assert_eq!(runner.grid().to_dense(), DenseGrid::new(96, 96).unwrap());
}

#[test]
fn identity_dynamics_preserve_every_representation() {
    let rule = IdentityRule;
    let dense = deterministic_field(70, 67, 0x1357_9bdf);
    let sparse = SparseFrontierGrid::from_dense(&dense);
    let chunked = ChunkedGrid::from_dense(&dense);
    assert_equal(
        &dense,
        &sparse.run(&rule, 5),
        &chunked.run(&rule, 5),
        &rule,
        5,
    );
}

#[test]
fn rule_constructors_reject_an_active_quiescent_background() {
    let zero = State::QUIESCENT;
    let active = State::try_from(1).unwrap();
    let neighborhood = Neighborhood {
        center: zero,
        north: zero,
        east: zero,
        south: zero,
        west: zero,
    };
    assert_eq!(
        LangtonRule::from_base_transitions(vec![(neighborhood, active)]).unwrap_err(),
        RuleError::NonQuiescentBackground(active)
    );
    assert_eq!(
        BylRule::from_base_transitions(vec![(neighborhood, active)]).unwrap_err(),
        RuleError::NonQuiescentBackground(active)
    );
}

#[test]
fn sparse_representations_refuse_external_rules_with_active_backgrounds() {
    let dense = DenseGrid::new(8, 8).unwrap();
    let sparse = SparseFrontierGrid::from_dense(&dense);
    let chunked = ChunkedGrid::from_dense(&dense);
    assert!(std::panic::catch_unwind(|| sparse.step(&ActiveBackgroundRule)).is_err());
    assert!(std::panic::catch_unwind(|| chunked.step(&ActiveBackgroundRule)).is_err());
}
