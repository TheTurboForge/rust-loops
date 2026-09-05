# Provenance and Attribution

`data/langton/`, `data/byl-golly-3.3/`, `data/sdsr-golly-3.3/`,
`data/evoloop-golly-3.3/`, and `data/benchmark/` are
project-authored normalized representations of transition, seed,
reference-state, and deterministic benchmark-control data. They are used as
executable fixture data by this GPL-3.0-or-later project; they do not
redistribute Golly source code, Golly rule files, RLE files, binaries, or the
source papers. The benchmark identity rule and synthetic generator are
original project controls rather than claims about an external scientific
model.

Sources consulted and recorded in the private research archive:

- C. G. Langton, “Self-reproduction in cellular automata,” *Physica D* 10
  (1984), 135–144, doi:10.1016/0167-2789(84)90256-2.
- Golly Rule Table Repository, `Langtons-Loops.table`, pinned commit
  `3646a185c6049180980de2989b27662a10bb3b86`.
- Golly 3.3-1build1 packaged `Langtons-Loops` rule and seed.
- J. Byl, “Self-Reproduction in Small Cellular Automata,” *Physica D* 34
  (1989), 295–299, doi:10.1016/0167-2789(89)90242-X.
- Golly Rule Table Repository, `Byl-Loop.table`, pinned commit
  `3646a185c6049180980de2989b27662a10bb3b86`.
- Golly 3.3-1build1 packaged `Byl-Loop` rule and seed.
- H. Sayama, “Introduction of Structural Dissolution into Langton's
  Self-Reproducing Loop,” *Artificial Life VI*, 1998.
- H. Sayama's archived SDSR/Evoloop Java implementation by Andrew Bachmutsky.
- Golly Rule Table Repository, `SDSR-Loop.table`, pinned commit
  `3646a185c6049180980de2989b27662a10bb3b86`.
- Golly 3.3-1build1 packaged `SDSR-Loop` rule and seed.
- H. Sayama, “A New Structurally Dissolvable Self-Reproducing Loop Evolving in
  a Cellular Automata Space,” *Artificial Life* 5(4), 1999, 343–365,
  doi:10.1162/106454699568773.
- Golly Rule Table Repository, `Evoloop.table`, pinned commit
  `3646a185c6049180980de2989b27662a10bb3b86`.
- Golly 3.3-1build1 packaged `Evoloop` rule and seed.

The normalized fixture content hashes and the independently executed Golly 3.3
trajectory hashes are documented in `data/README.md`. The Byl paper/table
discrepancy is retained explicitly rather than resolved by assumption. The
SDSR and Evoloop direct functions were independently reconciled across three
executable channels before normalization. Raw evidence remains private and is
not part of this repository.
