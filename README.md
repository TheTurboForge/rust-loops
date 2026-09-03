# rust-loops

`rust-loops` is a Rust laboratory for cellular automata, Artificial Life, and
local persistent computation. Its first executable artifact is a deliberately
small canonical Langton-loop calibration engine.

The long-term research question is whether artificial substrates built from
cheap local interactions can support learning or evolution that discovers
increasingly capable information processing—and whether that computation can
be fundamentally more efficient than today's globally communicating neural
architectures.

The repository is in its canonical-correctness phase. It is not yet a general
CA framework, a viewer, a performance claim, or a learning/evolution system.

## Intended First Milestone

Langton's loop is the first calibration organism, not the final target. The
first implementation milestone is a faithful canonical loop:

- eight states;
- a five-cell von Neumann neighborhood;
- deterministic synchronous updates;
- fourfold rotational rule symmetry;
- the canonical circulating instruction sequence; and
- independently verified daughter-loop reproduction after 151 updates.

The project will implement the cellular transition rules themselves. It will
not fake reproduction with an object-level animation that merely resembles the
published behavior.

Later research stages may cover fixed-rule CA reservoir computing, neural
cellular automata, local plasticity and credit assignment, evolution of learning
rules, ecological selection for useful information processing, and hardware-
level efficiency. Each step is a separate empirical burden.

## Scope

The direct research lineage includes:

- von Neumann and Codd self-reproducing cellular automata;
- Langton's canonical loop;
- Byl and Chou–Reggia minimal loops;
- Tempesti and Perrier capability-oriented loops;
- Sayama's SDSR loop and Evoloop; and
- Oros and Nehaniv's Sexyloop systems.

See [Concepts](docs/concepts.md), [Loop family](docs/loop-family.md),
[Reference model](docs/reference-model.md), and [References](docs/references.md).

## Run The Canonical Engine

```sh
cargo run -- --generations 151
cargo test
```

The command emits a normalized active-cell snapshot with population, bounds,
generation, and a SHA-256 state-set hash. The integration suite compares
generations `0`, `1`, `25`, `75`, and `151` exactly against fixtures generated
by Golly 3.3's headless `bgolly` RuleLoader executor. The reference fixtures
are normalized state data, not raw Golly assets.

The first reference result is 171 non-quiescent cells in a `26×15` active
bounds box at generation 151. This is consistent with Langton's published
first-reproduction timing; it is not a claim about intelligence or evolution.

## Current Status

- GPL-3.0-or-later Cargo workspace, bounded dense engine, CLI, normalized
  canonical rule/seed fixtures, and Golly differential tests are present.
- `benchmark` is a single-threaded CPU JSONL harness that compares an exact
  double-buffered dense runner with an exact sparse-frontier runner. It emits
  timing, state hashes, update counts, approximate allocation, run metadata,
  and an explicit hardware-counter status; it makes no efficiency claim.
- The canonical generation-151 reference trajectory is independently frozen
  and checked in the private research archive.
- UI, sparse/GPU execution, parallelism, performance benchmarking, and other
  rule families remain intentionally deferred.

## Licensing And Provenance

The project is GPL-3.0-or-later. See [NOTICE.md](NOTICE.md) and
[data/README.md](data/README.md) for the source chain and boundary around the
normalized fixture data.

External papers, raw Golly rule tables/patterns, package bytes, and images are
not redistributed by this repository.

## Benchmark Harness

Build release mode and run one declared case:

```sh
RUSTC_VERSION="$(rustc --version)" cargo build --release --bin benchmark
RUST_LOOPS_GIT_REVISION="$(git rev-parse HEAD)" \
  target/release/benchmark --workload canonical --generation 151 --world 512 \
  --representation both --repetitions 30 --warmup 15 --timed-generations 100
```

Each line is one run record. Dense and sparse output hashes must match inside a
`both` run or the process fails. Wall-clock time is required; hardware counters
are reported only where host policy permits them.

## Epistemic Position

Emergence is not intelligence. Replication is not intelligence. Turing
completeness is not useful learning. Evolution does not guarantee open-ended
complexity, and logical locality does not automatically imply physical energy
efficiency. The project is designed to test those gaps rather than assume them
away.
