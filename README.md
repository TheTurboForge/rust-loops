# rust-loops

`rust-loops` is a Rust laboratory for cellular automata, Artificial Life, and
local persistent computation. Its first executable artifacts are deliberately
small, independently verified Langton and Byl loop calibration profiles.

The long-term research question is whether artificial substrates built from
cheap local interactions can support learning or evolution that discovers
increasingly capable information processing—and whether that computation can
be fundamentally more efficient than today's globally communicating neural
architectures.

The repository has completed two independent loop-family correctness gates and
its first substrate-characterization cycles. It is not yet a general CA
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

## Current Status

- GPL-3.0-or-later Cargo workspace, bounded dense engine, CLI, normalized
  Langton and Byl rule/seed fixtures, and Golly differential tests are present.
- Rust `1.97.0`, Rustfmt, and Clippy are pinned for reproducible checks.
- `benchmark` is a single-threaded CPU JSONL harness that compares an exact
  double-buffered dense runner with an exact sparse-frontier runner. It emits
  timing, state hashes, update counts, approximate allocation, run metadata,
  and an explicit hardware-counter status; it makes no efficiency claim.
- A deliberately narrow chunked candidate was evaluated under a preregistered
  gate and not retained in the current API. Automatic representation selection
  remains deferred until evidence exists from another rule family.
- The Langton generation-151 and Byl generation-25 reference trajectories are
  independently frozen and checked in the private research archive.
- UI, GPU execution, parallelism, and additional rule families remain
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
