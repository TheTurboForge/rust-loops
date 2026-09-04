use rust_loops::{
    CanonicalSeed, ChunkedGrid, ChunkedRunner, DenseGrid, DenseRunner, LangtonRule, Neighborhood,
    Snapshot, SparseFrontierGrid, State,
};

fn assert_equal(
    dense: &DenseGrid,
    sparse: &SparseFrontierGrid,
    chunked: &ChunkedGrid,
    generation: u64,
) {
    let expected = Snapshot::from_grid(dense, generation);
    assert_eq!(
        expected,
        Snapshot::from_grid(&sparse.to_dense(), generation),
        "sparse state mismatch at generation {generation}"
    );
    assert_eq!(
        expected,
        Snapshot::from_grid(&chunked.to_dense(), generation),
        "chunked state mismatch at generation {generation}"
    );
}

fn identity_rule() -> LangtonRule {
    let mut transitions = Vec::with_capacity(8usize.pow(5));
    for center in 0..8 {
        for north in 0..8 {
            for east in 0..8 {
                for south in 0..8 {
                    for west in 0..8 {
                        transitions.push((
                            Neighborhood {
                                center: State::try_from(center).unwrap(),
                                north: State::try_from(north).unwrap(),
                                east: State::try_from(east).unwrap(),
                                south: State::try_from(south).unwrap(),
                                west: State::try_from(west).unwrap(),
                            },
                            State::try_from(center).unwrap(),
                        ));
                    }
                }
            }
        }
    }
    LangtonRule::from_base_transitions(transitions).unwrap()
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
fn all_representations_match_every_canonical_generation_through_302() {
    let rule = LangtonRule::canonical().unwrap();
    let seed = CanonicalSeed::canonical().unwrap();
    let mut dense = seed.place_in(128, 128, 32, 32).unwrap();
    let mut sparse = SparseFrontierGrid::from_dense(&dense);
    let mut chunked = ChunkedGrid::from_dense(&dense);
    for generation in 0..=302 {
        assert_equal(&dense, &sparse, &chunked, generation);
        dense = dense.step(&rule);
        sparse = sparse.step(&rule).0;
        chunked = chunked.step(&rule).0;
    }
}

#[test]
fn sparse_matches_dense_for_deterministic_multistate_fields() {
    let rule = LangtonRule::canonical().unwrap();
    let mut dense = deterministic_field(79, 61, 0x5eed_cafe);
    let mut sparse = SparseFrontierGrid::from_dense(&dense);
    let mut chunked = ChunkedGrid::from_dense(&dense);
    for generation in 0..20 {
        assert_equal(&dense, &sparse, &chunked, generation);
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
    assert_equal(&dense.step(&rule), &next_sparse, &next_chunked, 1);
}

#[test]
fn empty_sparse_world_remains_empty() {
    let rule = LangtonRule::canonical().unwrap();
    let dense = DenseGrid::new(19, 19).unwrap();
    let sparse = SparseFrontierGrid::from_dense(&dense);
    let chunked = ChunkedGrid::from_dense(&dense);
    let (next, stats) = sparse.step(&rule);
    let (next_chunked, chunked_stats) = chunked.step(&rule);
    assert_eq!(stats.cell_evaluations, 0);
    assert_eq!(chunked_stats.cell_evaluations, 0);
    assert_eq!(next.active_cell_count(), 0);
    assert_equal(&dense, &next, &next_chunked, 1);
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
        assert_equal(&dense, &sparse, &chunked, generation);
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
    let rule = identity_rule();
    let dense = deterministic_field(70, 67, 0x1357_9bdf);
    let sparse = SparseFrontierGrid::from_dense(&dense);
    let chunked = ChunkedGrid::from_dense(&dense);
    assert_equal(&dense, &sparse.run(&rule, 5), &chunked.run(&rule, 5), 5);
}
