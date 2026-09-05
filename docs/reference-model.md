# Reference Model

## Canonical Langton Loop

- Two-dimensional square lattice.
- Eight states numbered `0` through `7`.
- Center/north/east/south/west neighborhood.
- Deterministic synchronous generations.
- Fourfold rotational transition symmetry.
- Quiescent state `0`.
- Canonical genome: `70-70-70-70-70-70-40-40`.
- Canonical initial active population: 86 cells under the published counting
  convention.
- Exact daughter-loop reproduction after 151 updates.

The genome encodes six straight extensions and a left turn. Fourfold symmetry
allows one side-and-corner description to be reused to construct a complete
loop.

## Required Verification

The implementation will need to prove:

- exact transition and rotation behavior;
- synchronous, traversal-order-independent stepping;
- quiescent stability;
- canonical seed population;
- selected independently sourced intermediate snapshots;
- exact parent/daughter state at generation 151;
- continued reproduction after the first daughter; and
- documented collision and blocked-colony behavior.

Final visual resemblance alone is not sufficient evidence.

## Implemented Calibration Boundary

The initial engine has one explicit boundary policy: a fixed finite grid with
quiescent cells outside the grid. The canonical runner uses a padded `128×128`
world and places the `15×10` seed at `(32, 32)`. Result snapshots normalize
their active bounding box to `(0, 0)` so they can be compared with the Golly
reference format without exposing an arbitrary simulation-world offset.

The public oracle fixtures cover generations `0`, `1`, `25`, `75`, and `151`.
They were independently executed by pinned Golly 3.3 `bgolly` with RuleLoader;
Rust does not generate its expected states. A generation-302 continuation
fixture is retained privately for the next calibration extension.

## Byl Golly 3.3 Executable-Reference Profile

- Six states numbered `0` through `5`.
- Center/north/east/south/west neighborhood.
- Deterministic synchronous generations and fourfold rotational symmetry.
- A `4×4`, 12-cell initial pattern.
- 144 active source rows expanding to 561 distinct neighborhoods.
- Unlisted neighborhoods preserve the center state.
- First completed daughter configuration at generation `25`.

The public oracle covers every generation from `0` through `27`, plus `50`.
The Rust runner matches those independently executed Golly 3.3 states exactly,
including cells, state populations, bounds, and hashes. Generation `25` has 25
active cells, `9×4` bounds, and hash
`3f46c0896951a1d9a53395efe812a44570a8170659131765a6e04caa34419ef4`.

This is not presented as an uncontested global Byl transition table. The paper
reports 57 rules, its visible Table II contains 50 explicit and six default
rules, and a literal transcription diverges from Figure 3 at generation `1`.
The pinned Golly rule matches all 28 printed Figure 3 configurations. That
source conflict remains part of the reference model.

## Representation-Economics Boundary

The benchmark supports exact finite-world, single-threaded CPU comparison of
reusable double-buffered dense storage, an ordered-map sparse frontier, and an
explicit rule-generic candidate using a sparse directory of dense `32×32`
chunks. Each must yield the same rule-aware normalized state snapshot for a
declared run. The chunked representation requires a stable quiescent
background and uses active-edge candidate propagation. Its second
preregistered private gate retained it only for explicit manual selection on
localized natural and centered-clustered workloads at `512²/2048²`. Uniform
high-density controls favor dense storage, which remains the declared choice
for that regime.

Records identify the selected rule and fixtures, distinguish total evaluated
cells from active-cell updates, and report logical capacity separately from
process-level RSS deltas. They do not treat reduced logical work, RSS deltas,
or logical `32×32` occupancy as allocator-exact ownership, physical traffic,
energy, or hardware-general superiority. The benchmark-only identity rule
holds occupancy constant; it is not a scientific loop profile.

The historical first chunked candidate was killed by aggregate timeouts. Its
raw negative evidence remains valid and is not overwritten by the narrower
per-process cross-rule result. No automatic representation dispatcher exists;
one would require a separately preregistered selection policy and evidence
across structurally different workloads, not merely another related loop rule.

## SDSR Golly 3.3 Executable-Reference Profile

- Nine states numbered `0` through `8`, with state `8` used for structural
  dissolution.
- Center/north/east/south/west neighborhoods, synchronous generations,
  fourfold rotational rule-table semantics, and quiescent state `0`.
- A complete base-9 direct function of 59,049 neighborhoods rather than an
  implicit wildcard implementation.
- The canonical `15×10`, 86-active-cell Langton seed.

Archived Bachmutsky Java, pinned maintained rule tables, and Golly 3.3 expand
to the same direct function. The public Rust runner matches independently
executed Golly snapshots at generations `0`, `1`, `151`, `302`, `1,000`, and
`2,000`. The profile retains ordinary reproduction, but exactness is not itself
evidence that dissolution causes recovery. Obstruction, deletion, and collision
interventions are evaluated separately against a declared extended-SR control.

## Experiment Metadata

Saved results should identify engine version, rule and seed hashes, dimensions,
boundary policy, update schedule, generation, palette, and randomness seed if a
later variant uses randomness.
