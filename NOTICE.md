# Provenance and Attribution

`data/langton/` and `data/byl-golly-3.3/` are project-authored normalized
representations of transition, seed, and reference-state data. They are used
as executable fixture data by this GPL-3.0-or-later project; they do not
redistribute Golly source code, Golly rule files, RLE files, binaries, or the
source papers.

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

The normalized fixture content hashes and the independently executed Golly 3.3
trajectory hashes are documented in `data/README.md`. The Byl paper/table
discrepancy is retained explicitly rather than resolved by assumption. Raw
evidence remains private and is not part of this repository.
