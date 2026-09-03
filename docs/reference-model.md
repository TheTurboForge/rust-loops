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

## Experiment Metadata

Saved results should identify engine version, rule and seed hashes, dimensions,
boundary policy, update schedule, generation, palette, and randomness seed if a
later variant uses randomness.
