# rust-loops

`rust-loops` is a Rust laboratory for cellular automata, Artificial Life, and
local persistent computation. Its first executable artifacts are deliberately
small, independently verified Langton, Byl, SDSR, and Evoloop calibration
profiles.

The long-term research question is whether artificial substrates built from
cheap local interactions can support learning or evolution that discovers
increasingly capable information processing—and whether that computation can
be fundamentally more efficient than today's globally communicating neural
architectures.

The repository has completed four independent loop-profile correctness gates
and its first substrate-characterization cycles. It is not yet a general CA
framework, a viewer, a performance claim, or a learning/evolution system.

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

## Run The Verified Profiles

```sh
cargo run --bin rust-loops -- --generations 151
cargo run --bin rust-loops -- --rule byl-golly-3.3 --generations 25
cargo run --bin rust-loops -- --rule sdsr-golly-3.3 --generations 151
cargo run --release --bin rust-loops -- --rule evoloop-golly-3.3 \
  --boundary toroidal --width 200 --height 200 --origin-x 92 --origin-y 92 \
  --generations 5000
cargo test
```

The command emits either a finite normalized snapshot or an explicitly
toroidal absolute snapshot with population, generation, and a SHA-256 state
hash. The integration suite compares
generations `0`, `1`, `25`, `75`, and `151` exactly against fixtures generated
by Golly 3.3's headless `bgolly` RuleLoader executor. The reference fixtures
are normalized state data, not raw Golly assets.

The first reference result is 171 non-quiescent cells in a `26×15` active
bounds box at generation 151. This is consistent with Langton's published
first-reproduction timing; it is not a claim about intelligence or evolution.

The Byl executable-reference profile has six states and a 12-cell seed. Its
generation-25 state contains 25 active cells in `9×4` bounds and matches the
paper's printed first-replication configuration. The suite compares every
generation from `0` through `27`, plus generation `50`, with independently
executed Golly 3.3 states.

This profile is deliberately called `byl-golly-3.3`, not simply “canonical
Byl.” The paper's transition-count statement, its visible transition table,
and the table needed to reproduce its printed trajectory do not fully agree.
The project preserves that conflict; it does not synthesize an undocumented
resolution.

The nine-state `sdsr-golly-3.3` profile adds structural dissolution state `8`.
Its complete 59,049-entry direct function agrees across three archived
executable channels, and Rust matches independently executed Golly snapshots
through generation 2,000. That establishes the named transition profile and
trajectory; causal dissolution and recovery claims remain a separate
intervention gate.

The nine-state `evoloop-golly-3.3` profile uses a 149-cell 2-Evoloop species-13
seed. Fixed-quiescent Rust execution matches independently run Golly through
generation 2,000. The explicit dense toroidal runner matches absolute Golly
states in a `200×200` world at every frozen milestone through generation
50,000. Boundary selection is explicit; existing finite-grid APIs remain
quiescent and never switch topology implicitly. Evolutionary interpretation is
a separate observer-and-controls gate.

## Current Status

- GPL-3.0-or-later Cargo workspace, bounded dense engine, CLI, normalized
  Langton, Byl, SDSR, and Evoloop rule/seed fixtures, and Golly differential
  tests are present.
- Rust `1.97.0`, Rustfmt, and Clippy are pinned for reproducible checks.
- `benchmark` is a single-threaded CPU JSONL harness for exact dense,
  sparse-frontier, and explicit `32×32` chunked representations under either
  benchmark-enabled Langton or Byl profile. Schema version 4 emits timing,
  provenance, exact output hashes, actual activity/locality, logical capacity,
  fixture-owned process-RSS deltas, update counts, and explicit counter/energy
  status; it makes no efficiency claim.
- The first chunked candidate was killed under its preregistered aggregate
  timeout gate. A second rule-generic candidate passed a separately
  preregistered private cross-rule gate and is retained only as an explicit
  manually selected option for localized `512²/2048²` workloads. Dense remains
  the declared high-density choice; no automatic dispatcher exists.
- The Langton generation-151, Byl generation-25, SDSR-through-2,000, and
  periodic Evoloop-through-50,000 reference trajectories are independently
  frozen and checked in the private research archive.
- UI, GPU execution, parallelism, and further rule families remain
  intentionally deferred.

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

RUST_LOOPS_GIT_REVISION="$(git rev-parse HEAD)" \
  target/release/benchmark --rule byl-golly-3.3 --workload canonical \
  --generation 25 --world 512 --representation all --repetitions 30 \
  --warmup 15 --timed-generations 100

RUST_LOOPS_GIT_REVISION="$(git rev-parse HEAD)" \
  target/release/benchmark --workload identity --density-ppm 10000 \
  --layout clustered --seed 1592639710 --world 512 --representation chunked \
  --repetitions 1 --warmup 15 --timed-generations 100
```

Each line is one run record. All selected representation output hashes must
match or the process fails. `both` retains dense/sparse compatibility; `all`
adds the explicit chunked option. Use chunked only when locality is known; use
dense for uniform high-density fields. The identity workload is a
stable-density storage/kernel control, not a loop result. The default
scientific rule remains Langton for command compatibility. Wall-clock time is
required; hardware counters and energy are reported only where the host and
experiment provide them. Logical capacity, process RSS deltas, and `32×32`
occupancy summaries are not allocator-exact ownership, physical memory
traffic, or energy.

## Epistemic Position

Emergence is not intelligence. Replication is not intelligence. Turing
completeness is not useful learning. Evolution does not guarantee open-ended
complexity, and logical locality does not automatically imply physical energy
efficiency. The project is designed to test those gaps rather than assume them
away.
