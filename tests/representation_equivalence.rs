use rust_loops::{
    CanonicalSeed, DenseGrid, DenseRunner, LangtonRule, Snapshot, SparseFrontierGrid, State,
};

fn assert_equal(dense: &DenseGrid, sparse: &SparseFrontierGrid, generation: u64) {
    assert_eq!(
        Snapshot::from_grid(dense, generation),
        Snapshot::from_grid(&sparse.to_dense(), generation),
        "state mismatch at generation {generation}"
    );
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
fn sparse_matches_dense_at_every_canonical_milestone() {
    let rule = LangtonRule::canonical().unwrap();
    let seed = CanonicalSeed::canonical().unwrap();
    let mut dense = seed.place_in(128, 128, 32, 32).unwrap();
    let mut sparse = SparseFrontierGrid::from_dense(&dense);
    for generation in 0..=302 {
        if [0, 1, 25, 75, 151, 302].contains(&generation) {
            assert_equal(&dense, &sparse, generation);
        }
        dense = dense.step(&rule);
        sparse = sparse.step(&rule).0;
    }
}

#[test]
fn sparse_matches_dense_for_deterministic_multistate_fields() {
    let rule = LangtonRule::canonical().unwrap();
    let mut dense = deterministic_field(79, 61, 0x5eed_cafe);
    let mut sparse = SparseFrontierGrid::from_dense(&dense);
    for generation in 0..20 {
        assert_equal(&dense, &sparse, generation);
        dense = dense.step(&rule);
        sparse = sparse.step(&rule).0;
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
    let (next_sparse, stats) = sparse.step(&rule);
    assert!(stats.cell_evaluations < dense.width() * dense.height());
    assert_equal(&dense.step(&rule), &next_sparse, 1);
}

#[test]
fn empty_sparse_world_remains_empty() {
    let rule = LangtonRule::canonical().unwrap();
    let dense = DenseGrid::new(19, 19).unwrap();
    let sparse = SparseFrontierGrid::from_dense(&dense);
    let (next, stats) = sparse.step(&rule);
    assert_eq!(stats.cell_evaluations, 0);
    assert_eq!(next.active_cell_count(), 0);
    assert_equal(&dense, &next, 1);
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
}
