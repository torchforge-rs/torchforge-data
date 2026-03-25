# TODO

> Roadmap aligned to SemVer. `v0.x` = pre-alpha/alpha, no stability guarantees. `v1.0.0` = first stable API.
>
> Items marked `[RESEARCH]` require benchmarking or prototyping before implementation.
> Items marked `[DECISION]` are blocked on an architectural choice documented in [ARCHITECTURE.md](ARCHITECTURE.md).
> Items marked `[BLOCKED]` depend on another item being completed first.

---

## v0.1.0 — Core Primitives

**Goal**: Minimal working `Dataset` + `DataLoader` + `ReplayBuffer`. Enough to run a basic DQN training loop on a toy environment.

### Infrastructure (do these first)
- [ ] CI via GitHub Actions (stable + nightly Rust, `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`)
- [ ] `#![deny(missing_docs)]` enforced from v0.1.0 — all public items documented as they are written
- [ ] `proptest` in `[dev-dependencies]` for property-based testing
- [ ] `criterion` in `[dev-dependencies]` for micro-benchmarks, create `benches/` directory
- [ ] `cargo-fuzz` target for `MmapDataset` file parsing (malformed files, truncated files, corrupt index)

### Traits & Core Types
- [ ] Define `Dataset` trait with GAT-based `Item<'a>` for zero-copy returns
- [ ] Define `Sampler` trait with iterator-based return type
- [ ] Define `LoaderConfig` struct with builder pattern
- [ ] Define error types via `thiserror`
- [ ] `SequentialSampler`
- [ ] `RandomSampler` (without replacement)

### DataLoader
- [ ] `DataLoader<D, S>` struct generic over `Dataset` + `Sampler`
- [ ] Sequential iteration (no parallelism yet)
- [ ] Batch collation for primitive types (`f32`, `i64` arrays)
- [ ] `[RESEARCH]` Verify `rayon`-based parallel item loading degrades gracefully on single-core targets before enabling

### ReplayBuffer
- [ ] `ReplayBuffer` with `Vec`-backed AoS layout (baseline, known-correct)
- [ ] Fixed capacity circular buffer (ring buffer semantics)
- [ ] Uniform random sampling
- [ ] `push()` and `sample()` API
- [ ] `len()`, `capacity()`, `is_ready(min_samples)` helpers

### MmapDataset
- [ ] `MmapDataset` backed by `memmap2`
- [ ] Raw binary format with sidecar index (simplest viable format)
- [ ] `[RESEARCH]` Measure mmap page eviction latency under memory pressure on at least one edge target before marking stable

### Testing
- [ ] Unit tests for all core types
- [ ] Property-based tests for `ReplayBuffer` correctness (`proptest`), testing these invariants:
  - Capacity invariant: `len() <= capacity` always holds after any sequence of `push()` calls
  - FIFO overwrite: after `capacity + n` pushes, the oldest `n` items are evicted and the newest `capacity` are retained in order
  - Sample correctness: `sample(k)` returns exactly `k` items, all of which are currently in the buffer
  - Sample distribution: over many calls, uniform sampling approximates uniform distribution (statistical test)
  - Empty buffer: `sample()` on empty/underfilled buffer returns error, not panic
- [ ] Fuzz testing for `MmapDataset::open()` and `MmapDataset::get()` via `cargo-fuzz`

### Benchmarks
- [ ] `criterion` micro-benchmarks for:
  - `ReplayBuffer::push()` throughput
  - `ReplayBuffer::sample()` latency (various batch sizes)
  - `Dataset::get()` for in-memory and mmap-backed datasets
  - `DataLoader` end-to-end iteration throughput
- [ ] Establish performance baseline for v0.2.0 comparisons

### Documentation
- [ ] Doc comments on all public API items (enforced by `#![deny(missing_docs)]`)
- [ ] At least one working example in `examples/`

---

## v0.2.0 — Parallelism & Performance

**Goal**: Demonstrate measurable throughput advantage over Python DataLoader on a documented benchmark.

- [ ] `rayon`-parallel `DataLoader` (after single-core degradation is verified — see v0.1.0)
- [ ] Prefetch buffer with configurable depth (see [OPEN] design questions in ARCHITECTURE.md)
- [ ] `[RESEARCH]` SoA layout for `ReplayBuffer` — prototype and benchmark vs. AoS baseline from v0.1.0
- [ ] `[RESEARCH]` mmap-backed `ReplayBuffer` — prototype and benchmark vs. heap
- [ ] `[DECISION]` Adopt winning memory layout from research above
- [ ] `[RESEARCH]` Allocator comparison: system allocator vs. jemalloc vs. mimalloc on edge targets
- [ ] Benchmark suite: throughput (samples/sec), memory usage, latency distribution
- [ ] Publish benchmark results in `benches/` with reproducible methodology
- [ ] `WeightedSampler`
- [ ] `RandomSampler` with replacement

---

## v0.3.0 — RL Depth

**Goal**: Support Prioritized Experience Replay and enough infrastructure for a full SAC/PPO loop.

- [ ] `[RESEARCH]` Sum-tree data structure for Prioritized Experience Replay (PER)
- [ ] `PrioritizedReplayBuffer` backed by sum-tree
- [ ] Priority update API (`update_priorities()`)
- [ ] Importance sampling weights for PER correction
- [ ] N-step return buffer
- [ ] Episode boundary tracking

---

## v0.4.0 — Format & Interop

**Goal**: Read datasets produced by Python tooling. Enable hybrid Python-train / Rust-serve workflows.

- [ ] `[DECISION]` File format decision: raw binary vs. Arrow IPC vs. custom
- [ ] Reader for chosen format
- [ ] Writer for chosen format (to generate test fixtures from Python)
- [ ] Validation tooling: verify written files are readable without corruption

---

## v0.5.0 — Async I/O

**Goal**: Support streaming from network sources (S3, HTTP) for non-edge use cases.

- [ ] `StreamDataset` trait for unbounded/streaming datasets (no `len()` requirement)
- [ ] `AsyncDataLoader` design (separate from sync `DataLoader`)
- [ ] `tokio`-based async item loading
- [ ] Backpressure handling

---

## v1.0.0 — Stable API

**Gate criteria** (all must be met):
- [ ] All v0.x public API items are stable and documented
- [ ] At least one published benchmark showing advantage over Python DataLoader on edge hardware
- [ ] At least one real RL training loop using `torchforge-data` end-to-end
- [ ] Zero known soundness issues
- [ ] MSRV (Minimum Supported Rust Version) declared and tested

---

## Ongoing

- [ ] Keep dependencies on latest stable versions
- [ ] `cargo audit` clean at all times
- [ ] `cargo clippy -- -D warnings` clean at all times
- [ ] All public items documented (`#![deny(missing_docs)]` enforced from v0.1.0)
- [ ] CHANGELOG.md maintained per [Keep a Changelog](https://keepachangelog.com/)
