# Loop Family

| System | Main Contribution |
| --- | --- |
| von Neumann automaton | Universal construction and explicit description copying. |
| Codd automaton | Eight-state simplification retaining construction universality. |
| Langton loop | Compact, fully simulatable exact replication using a circulating genome. |
| Byl loop | Six-state, 12-cell, 25-update minimal loop model. |
| Chou–Reggia loops | Unsheathed, very small self-directed replicators. |
| Tempesti loop | Adds construction and computation beside replication. |
| Perrier–Sipper–Zahnd | Reproduces a loop, Turing program, and data, then executes the program. |
| SDSR loop | Adds structural dissolution and resource turnover. |
| Evoloop | Adds robust inherited variation and intrinsic natural selection. |
| Sexyloop | Adds genetic exchange and studies sexual reproduction's evolutionary effects. |

Secondary summaries sometimes list Byl's loop as seven-state. Byl's original
paper explicitly specifies six states, which is the value this project uses.
The implemented package is more narrowly identified as the Golly 3.3
executable-reference profile: its trajectory matches every printed Figure 3
configuration, while a literal transcription of the paper's visible Table II
does not. The conflicting sources remain separately identified.

Variants will be implemented as separate named rule packages with their own
sources, seeds, update semantics, and verification oracles.

The implemented `sdsr-golly-3.3` package follows that rule: its nine-state
direct function is exact to three mutually agreeing executable sources and six
independent trajectory milestones through generation 2,000. The project does
not promote structural dissolution to a self-repair claim until controlled
interventions pass.
