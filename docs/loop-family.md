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

Variants will be implemented as separate named rule packages with their own
sources, seeds, update semantics, and verification oracles.
