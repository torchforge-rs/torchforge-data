# Architecture

> This document captures current design decisions, their justifications, and — explicitly — what remains unknown or unresolved. It is a living document. Decisions marked `[OPEN]` are not yet settled.
>
> **Changelog**:
> - `2026-03-26` — Added FDRL design considerations to `ReplayBuffer`, new `## FDRL Design Considerations` section, and updated `## Out of Scope` to reflect v1.x federation target.
> - `2026-03-26 (v2)` — Added `## Rust ML Ecosystem Context` note from project knowledge section 13; updated `## Out of Scope` GPU entry with ecosystem grounding.

---

## Problem Statement

On-device reinforcement learning requires a data pipeline that:

1. Feeds experience tuples `(state, action, reward, next_state, done)` to a training loop
2. Operates continuously — there is no "epoch end" in online RL
3. Fits within hard memory budgets (target: devices with 256MB–2GB RAM)
4. Does not introduce GC pauses or non-deterministic latency into the control loop
5. Supports random sampling from a replay buffer with configurable capacity

Secondary use case: supervised/offline ML datasets on edge hardware, where streaming from disk is required because the dataset does not fit in RAM.

---

## Core Abstractions

### `Dataset<Item>` trait

The fundamental abstraction. A `Dataset` is anything that can produce items by index. Uses Generic Associated Types (GATs) to enable zero-copy returns — the `Item` carries a lifetime tied to `&self`, allowing implementations to return borrowed data (e.g., slices into a memory-mapped region) without cloning.

```rust
pub trait Dataset {
    type Item<'a> where Self: 'a;
    type Error;

    fn get(&self, index: usize) -> Result<Self::Item<'_>, Self::Error>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool { self.len() == 0 }
}
```

**Decided**: GAT-based `Item<'a>` enables zero-copy on the hot path. This is load-bearing for the core value proposition — without it, every `MmapDataset::get()` would require a copy from the mmap region into an owned allocation.

**Note**: The streaming/unbounded dataset case (no meaningful `len()`) is deferred to v0.5.0 when `AsyncDataLoader` is designed. A separate `StreamDataset` trait will be introduced at that point. For v0.1.0, `Dataset` assumes a known, finite size.

---

### `DataLoader<D, S>`

Wraps a `Dataset`, applies a sampling strategy, batches items, and optionally prefetches.

```rust
pub struct DataLoader<D: Dataset, S: Sampler> {
    dataset: D,
    config: LoaderConfig,
    sampler: S,
}
```

**Decided**: Generic over `Sampler` for monomorphization — no vtable overhead on edge hardware. In-order ARM cores without branch prediction pay a disproportionate cost for vtable indirection. Users who need runtime sampler swapping can use `Box<dyn Sampler>` as the type parameter.

**Parallelism model**: `rayon` for CPU-parallel item loading. Not `tokio` at this layer — item loading is CPU-bound (deserialization, transforms), not I/O-bound.
**[OPEN]**: For network-streamed datasets (S3, NFS), async I/O via `tokio` may be needed. This is a separate `AsyncDataLoader` variant, not the default path. Design deferred to v0.5.0.

**[OPEN]**: Prefetch buffer design. The following questions must be answered before implementing prefetch (v0.2.0):
- Memory budget interaction: each prefetched batch consumes memory — on a 256MB device, prefetch depth must be bounded by available RAM
- Sampler interaction: must sample ahead without consuming the iterator prematurely
- Error handling: what happens when a prefetched load fails? Retry? Propagate on next `next()` call?
- Backpressure: what if the training loop consumes batches slower than the loader produces them?

---

### `ReplayBuffer`

First-class RL primitive. A fixed-capacity circular buffer of experience tuples with uniform random sampling.

```rust
pub struct ReplayBuffer<S, A, R> {
    capacity: usize,
    // internal ring buffer
    // v0.1.0: Vec-backed AoS layout (baseline, known-correct)
    // v0.2.0: benchmark SoA and mmap-backed alternatives
}
```

**Known**: Uniform random sampling from a circular buffer is the standard baseline (DQN, SAC, TD3).
**[OPEN]**: Prioritized Experience Replay (PER) requires a sum-tree data structure. Deferred to v0.3.0.

**v0.1.0 memory layout**: `Vec<Transition>` (Array-of-Structs). This is the simplest correct implementation but has a **known performance limitation**: random batch sampling touches non-contiguous cache lines because `Transition` structs are interleaved. On edge ARM chips with small L1/L2 caches (32–256KB), this means nearly every sample is a cache miss. This is acceptable for v0.1.0 as a correctness baseline; the v0.2.0 benchmark suite will quantify the overhead and compare against SoA and mmap-backed alternatives.

**[OPEN]**: Memory layout (v0.2.0 research). Options:
- Structure-of-Arrays (SoA) — better cache behavior for batch sampling, more complex
- mmap-backed — enables buffers larger than RAM, unknown latency profile on edge hardware

This is a **genuine open research question** for the on-device RL case. We will prototype and benchmark all three before deciding.

<!-- v2: 2026-03-26 — FDRL federation boundary consideration -->
**FDRL federation boundary** *(added 2026-03-26)*: In the federated RL use case (torchforge-federated, v1.x), each edge device runs a local `ReplayBuffer` and trains a local policy. What gets shared with the federation coordinator is **gradient updates**, not raw experience tuples. This distinction has concrete implications for v0.x design:

- The `ReplayBuffer` API must not assume it is the only consumer of experience data. A future `GradientBuffer` or federation adapter will sit alongside it, not replace it.
- The `Transition` type must remain serialization-friendly. Avoid embedding non-serializable types (e.g., raw pointers, `Rc`) in `Transition` fields.
- Capacity and memory layout decisions made at v0.2.0 should be evaluated against the federation scenario: in FDRL, buffer capacity per device is bounded by that device's RAM, not by an aggregate fleet budget.

These are forward compatibility constraints, not v0.x implementation tasks. No FDRL code ships before v1.x. But the buffer interface designed at v0.1.0–v0.2.0 must not require a breaking change to accommodate federation at v1.x.

---

### `Sampler` trait

Decouples index generation from data loading. Returns an iterator to avoid per-batch `Vec<usize>` allocation on the hot path — in continuous RL with no epoch boundary, `sample()` is called thousands of times per second.

```rust
pub trait Sampler {
    fn sample(&mut self, dataset_len: usize, batch_size: usize) -> impl Iterator<Item = usize>;
}
```

**Decided**: Iterator-based return type eliminates allocation pressure. Indices are consumed lazily and compose naturally with `rayon` and batch collation.

Implementations planned:
- `SequentialSampler`
- `RandomSampler` (with/without replacement)
- `WeightedSampler`

---

### Memory-Mapped Dataset (`MmapDataset`)

For datasets that do not fit in RAM. Uses `memmap2` crate for OS-managed demand paging.

**Known**: `memmap2` is actively maintained (v0.9.x as of 2025), soundness-reviewed, and used in production Rust projects.
**Known**: mmap provides zero-copy reads on the hot path — the OS page cache handles caching transparently. With the GAT-based `Dataset` trait, `MmapDataset::get()` can return a `&'a [u8]` slice directly into the mapped region.
**[OPEN]**: Behavior under memory pressure on embedded Linux (page eviction latency) is not characterized. Needs measurement on target hardware.
**[OPEN]**: File format. Raw binary with a sidecar index file is the v0.1.0 baseline (simplest viable format). Candidates for v0.4.0:
- Raw binary with a sidecar index file — simplest
- Apache Arrow IPC — interoperable with Python, higher complexity
- Custom format — full control, highest maintenance burden

Decision deferred until the first concrete benchmark target is chosen.

---

## Dependency Decisions

| Crate | Version | Purpose | Justification |
|---|---|---|---|
| `memmap2` | latest stable | Memory-mapped I/O | Actively maintained, soundness-reviewed |
| `rayon` | latest stable | Data-parallel loading | Standard, proven, no unsafe in user code |
| `rand` | latest stable | Sampling | Standard |
| `thiserror` | latest stable | Error types | Idiomatic Rust error handling |

**Not yet decided:**
- Arrow/columnar format support: deferred until format decision is made
- Async I/O: deferred until `AsyncDataLoader` design begins
- Serialization: deferred until file format decision is made

---

## What We Explicitly Do Not Know

1. **Optimal replay buffer memory layout for on-device RL** — SoA vs AoS vs mmap. Requires benchmarks on real edge hardware.
2. **Deterministic latency profile of mmap under memory pressure** — unknown without measurement.
3. **Whether `rayon` parallelism degrades gracefully on single-core edge targets** — needs verification.
4. **File format for edge-native datasets** — no clear winner; requires use-case-driven decision.
5. **Allocator choice for edge targets** — musl libc's allocator and glibc's ptmalloc2 perform poorly under the continuous small alloc/dealloc patterns of an RL training loop. jemalloc adds ~300KB to binary size and may not be available on all edge targets (e.g., RISC-V with musl). Needs benchmarking on target hardware.

These are not gaps to paper over. They are the research questions this project exists to answer.

---

<!-- v2: 2026-03-26 — FDRL section added -->
## FDRL Design Considerations *(added 2026-03-26)*

Federated Deep Reinforcement Learning (FDRL) is the v1.x target for the torchforge ecosystem. Each edge device trains a local DRL agent on its own environment data; only gradient updates — not raw experience tuples — are shared with a coordination layer. A global policy is aggregated (FedAvg baseline) and pushed back to devices.

This section captures how the v0.x data layer design must account for that future, without implementing it prematurely.

**What FDRL requires from torchforge-data at v1.x:**
- A `ReplayBuffer` API that a federation adapter can sit alongside without requiring a rewrite
- `Transition` types that are serialization-friendly (no non-serializable fields)
- Clear separation between local experience storage (this crate) and gradient communication (torchforge-federated, v1.x)
- Memory layout decisions that hold under per-device RAM constraints, not aggregate fleet budgets

**What this means for v0.x decisions:**
- Do not embed non-serializable types in `Transition`. Use owned, plain-data fields.
- Keep the `ReplayBuffer` interface narrow and side-effect-free — a future gradient-extraction step needs to compose with it, not wrap it
- File format decision (v0.4.0) should consider whether the chosen format can represent gradient checkpoints, not just raw experience tuples — this informs the decision between raw binary and Arrow IPC

**What FDRL does NOT require from v0.x:**
- No federation protocol code
- No gradient serialization
- No network communication
- No multi-device coordination

The federated crate (torchforge-federated) is a v1.x milestone. The data layer built at v0.x is the foundation it runs on. The constraint is interface compatibility, not implementation.

**Historical grounding**: FDRL is an established academic term (IEEE DySPAN 2021, arXiv 2412.12543, arXiv 2505.12153). No production Rust tooling for FDRL exists as of March 2026. torchforge-federated will be the first.

---

## Rust ML Ecosystem Context *(added 2026-03-26)*

Project knowledge section 13 establishes where torchforge sits in the Rust ML stack. Reproduced here for orientation — the data layer does not choose a neural network backend, but its design must be compatible with the framework that does.

- `burn` — Native Rust framework. No C++ dependency. Modular backends: `ndarray`, `wgpu`, `candle`, CUDA. **The training framework** torchforge builds on for on-device RL.
- `candle` — HuggingFace. Best for inference + LLMs. Not the primary training target.
- `tch-rs` — ~800MB libtorch. Explicitly excluded — not viable for edge.
- `torchforge` — **builds ON TOP of burn/candle** for the edge RL training use case.

**Implication for this crate**: torchforge-data provides the data pipeline that feeds a `burn`-backed training loop. The `Transition` type and `ReplayBuffer` API must be compatible with how `burn`'s tensor operations consume batched data. No burn dependency in this crate — the interface boundary is raw slices and owned data. The compatibility requirement is semantic, not a crate dependency.

---

## Out of Scope (v0.x)

- GPU-side data loading / CUDA pinned memory — `burn` supports CUDA and `wgpu` backends; if GPU-side loading becomes relevant at v1.x, the interface boundary is a `burn` tensor, not a raw slice. Design deferred.
- Distributed data loading across multiple devices — *deferred to v1.x as torchforge-federated (FDRL); explicitly not forgotten, see `## FDRL Design Considerations`*
- Video / audio streaming pipelines
- Python interop / PyO3 bindings

These may become in-scope as the ecosystem matures. They are explicitly deferred, not forgotten.
