# Concepts

## Cellular Automata

A cellular automaton consists of a lattice, a finite cell-state set, a local
neighborhood, and a transition rule applied across the lattice. Canonical
Langton loops use a two-dimensional square lattice, states `0` through `7`, the
center plus four orthogonal neighbors, and synchronous updates.

All next states must be computed from the same current generation. Updating
cells in place would produce a different system.

## Transcription And Translation

The loop's circulating instruction sequence is used in two ways:

- interpreted to construct the daughter loop;
- copied as data into the daughter.

Langton inherited this distinction from von Neumann's analysis of logical
self-reproduction and used it to exclude trivial patterns copied almost
entirely by the background rule.

## Replication And Reproduction

This project uses:

- **self-replication** for exact copies;
- **self-reproduction** for offspring that can carry viable inherited
  variation.

The canonical Langton loop is an engineered exact self-replicator. Evoloops
support inherited variation and natural selection.

## Computation And Construction

Universal computation, universal construction, exact replication, and
evolution are separate properties. The canonical Langton loop intentionally
trades away the universality of von Neumann and Codd systems to become much
smaller.

## Artificial Life

Langton's loop is a classic Artificial Life model because it synthesizes a
lifelike process in an artificial medium. The project does not treat visual
similarity, replication alone, or evolutionary behavior alone as proof that a
system is alive or autopoietic.

## Efficient Intelligence Hypothesis

The broader project asks whether local interaction, persistent state, sparse or
event-driven activity, self-organization, and multi-timescale learning can
produce a better computational economy than architectures dominated by global
communication and repeated context processing.

This is a hypothesis, not a conclusion. Dense CAs can be memory-bandwidth
limited, sparse execution has metadata costs, local signals can require many
hops, and evolution can be vastly more expensive than gradient training.

Neural cellular automata, CA reservoir computing, neuromorphic systems, local
plasticity, and digital evolution are directly relevant research bridges.

## Comparisons

Conway's Game of Life is an important cellular automaton but not part of the
direct loop lineage. Langton's ant shares an author and ALife context but is a
turmite, not a self-replicating loop.
