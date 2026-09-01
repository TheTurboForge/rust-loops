# rust-loops

`rust-loops` is a planned high-performance Rust laboratory for cellular
automata, Artificial Life, and local persistent computation.

The long-term research question is whether artificial substrates built from
cheap local interactions can support learning or evolution that discovers
increasingly capable information processing—and whether that computation can
be fundamentally more efficient than today's globally communicating neural
architectures.

The repository is currently in its documentation and verification-design
phase. No simulator has been implemented yet.

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

## Current Status

- Repository and research scope established.
- Canonical specifications and verification targets documented.
- Efficient-intelligence hypothesis and staged experiment program documented
  privately in the research control plane.
- Rust workspace, rule data, simulator, and UI not yet implemented.
- Reference artifact licensing and attribution review still open.

## Licensing Status

No project license has been selected yet. The absence of a license does not
grant permission to copy, modify, or redistribute the repository's contents.

External papers, rule tables, patterns, and images retain their respective
rights. No third-party rule or pattern data has been imported at this stage.

## Epistemic Position

Emergence is not intelligence. Replication is not intelligence. Turing
completeness is not useful learning. Evolution does not guarantee open-ended
complexity, and logical locality does not automatically imply physical energy
efficiency. The project is designed to test those gaps rather than assume them
away.
