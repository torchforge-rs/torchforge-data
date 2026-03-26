# TODO

> Roadmap aligned to SemVer. `v0.x` = pre-alpha/alpha, no stability guarantees. `v1.0.0` = first stable API.
>
> Items marked `[RESEARCH]` require benchmarking or prototyping before implementation.
> Items marked `[DECISION]` are blocked on an architectural choice documented in [ARCHITECTURE.md](ARCHITECTURE.md).
> Items marked `[BLOCKED]` depend on another item being completed first.
>
> **Changelog**:
> - `2026-03-26` — Added FDRL forward-compatibility constraints to `ReplayBuffer` items; added `## v1.x — torchforge-federated Foundation` section.
> - `2026-03-26 (v2)` — Reviewed against project knowledge section 13 (Rust ML ecosystem position). No substantive TODO changes required — NN backend choice is a torchforge-bench concern. `Transition: Send + Sync` constraint (added in v1) remains the data layer's interface commitment to the burn-backed training loop.

---

## Phase 0 — Project Foundation

**Goal**: Establish the repository as a credible, contribution-ready OSS project before any functional code ships. These items are prerequisites for v0.1.0 — nothing is merged to `main` until Phase 0 is complete.

### Repository Structure
- [x] Initialize repository with standard layout:
  ```
  .github/
    workflows/
    ISSUE_TEMPLATE/
    PULL_REQUEST_TEMPLATE.md
    CODEOWNERS
  benches/
  examples/
  src/
  tests/
  ARCHITECTURE.md
  CHANGELOG.md
  CODE_OF_CONDUCT.md
  CONTRIBUTING.md
  LICENSE
  README.md
  SECURITY.md
  TODO.md
  ```
- [x] `Cargo.toml` with correct metadata: `name`, `version = "0.0.1"`, `edition = "2024"`, `rust-version = "1.85"`, `license = "Apache-2.0"`, `repository`, `homepage`, `description`, `keywords`, `categories`
- [x] `Cargo.toml` `[badges]` section: crates.io version, docs.rs, license
- [x] `.gitignore` (standard Rust + editor artifacts)
- [ ] `.cargo/config.toml` if any workspace-level flags are needed (e.g., target-specific linker flags)

### License
- [x] `LICENSE` — Apache-2.0 full text
- [x] SPDX identifier `Apache-2.0` in `Cargo.toml`
- [x] License header policy: decide whether source files carry per-file SPDX headers (document the decision in `CONTRIBUTING.md`)

### Governance Documents
- [x] `CODE_OF_CONDUCT.md` — Contributor Covenant v2.1 (standard, widely recognized)
- [x] `CONTRIBUTING.md` — must cover:
  - Prerequisites (Rust version, `cargo` toolchain, `protoc` if needed)
  - How to build and run tests locally
  - Branching model (`main` is always release-ready; feature branches via PR)
  - PR process: issue first, then PR; no unsolicited implementation PRs while design is unstable
  - Commit message format (Conventional Commits recommended — enables automated CHANGELOG)
  - Code style: `cargo fmt` enforced, `cargo clippy -- -D warnings` must pass
  - What "ready to merge" means (all CI green, docs on all public items, CHANGELOG entry)
  - How to report a bug vs. propose a design change (issue templates)
- [x] `SECURITY.md` — must cover:
  - Supported versions (currently: latest `v0.x` only, no backports)
  - How to report a vulnerability (private channel: GitHub Security Advisories or email)
  - Response SLA commitment (e.g., acknowledge within 72 hours, triage within 7 days)
  - What is in scope (soundness issues in `unsafe` blocks, supply chain issues via `cargo audit`)
  - What is out of scope (theoretical only, requires physical device access, etc.)

### GitHub Templates
- [x] `.github/PULL_REQUEST_TEMPLATE.md` — checklist: description of change, linked issue, tests added/updated, docs updated, CHANGELOG entry, `cargo clippy` clean
- [x] `.github/ISSUE_TEMPLATE/bug_report.md` — Rust version, OS, reproduction steps, actual vs. expected behavior
- [x] `.github/ISSUE_TEMPLATE/feature_request.md` — problem being solved, proposed API sketch, alternatives considered
- [x] `.github/ISSUE_TEMPLATE/design_question.md` — for pre-implementation architecture discussion
- [x] `CODEOWNERS` — assign owners to `ARCHITECTURE.md`, `SECURITY.md`, `Cargo.toml` (forces review of critical files)

### CI — GitHub Actions
- [x] `ci.yml` — runs on every push and PR to `main`:
  - `cargo fmt --check`
  - `cargo clippy -- -D warnings`
  - `cargo test`
  - `cargo doc --no-deps`
  - Matrix: `[stable, nightly]` × `[ubuntu-latest]` (add macOS/Windows if cross-platform support is intended)
- [x] `audit.yml` — runs on push to `main` and on a daily schedule:
  - `cargo audit` — fail on any RUSTSEC advisory
  - `cargo deny check` (licenses, bans, advisories) — enforces dependency policy
- [x] `bench.yml` — manual trigger only (`workflow_dispatch`):
  - `cargo bench` — runs criterion benchmarks, uploads results as workflow artifact
  - Not a CI gate (benchmarks are hardware-sensitive); manual when needed
- [x] Cache `~/.cargo/registry` and `target/` across workflow runs (standard GHA cargo caching)
- [ ] Branch protection on `main`: require all `ci.yml` checks to pass, require at least one review

### Changelog
- [x] `CHANGELOG.md` — initialized per [Keep a Changelog](https://keepachangelog.com/) format:
  ```
  # Changelog
  All notable changes to this project will be documented in this file.
  ...
  ## [Unreleased]
  ```
- [x] Document the policy: every PR that changes behavior, adds a feature, or fixes a bug requires a CHANGELOG entry under `[Unreleased]`
- [x] On release: `[Unreleased]` becomes `[x.y.z] — YYYY-MM-DD`, a new empty `[Unreleased]` is added

### Supply Chain
- [x] `deny.toml` for `cargo deny`:
  - Licenses: allowlist `Apache-2.0`, `MIT`, `MIT-0`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Unicode-DFS-2016`; deny everything else
  - Bans: deny duplicate versions of the same crate where avoidable
  - Advisories: mirror `cargo audit` — deny all known vulnerabilities
- [x] `rust-toolchain.toml` pinning `stable` channel (ensures reproducible builds across contributors and CI)

### README Polish
- [x] Badges rendering correctly: crates.io version, docs.rs, license, CI status
- [x] "Why" section grounded in verifiable data (already present — verify links are live)
- [x] "Status" section clearly states pre-alpha, no stable API
- [x] "Planned Usage" clearly marked illustrative/unstable
- [x] Link to `CONTRIBUTING.md` and `CODE_OF_CONDUCT.md`

---

## v0.1.0 — Core Primitives

**Goal**: Minimal working `Dataset` + `DataLoader` + `ReplayBuffer`. Enough to run a basic DQN training loop on a toy environment.

### Infrastructure (do these first)
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
- [ ] *(added 2026-03-26)* `Transition` type fields must be plain, owned, serialization-friendly data — no `Rc`, raw pointers, or non-`Send` types; enforced by `Transition: Send + Sync` bound. Required for FDRL forward compatibility (see ARCHITECTURE.md `## FDRL Design Considerations`).

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
- [ ] Prefetch buffer with configurable depth (see `[OPEN]` design questions in ARCHITECTURE.md)
- [ ] `[RESEARCH]` SoA layout for `ReplayBuffer` — prototype and benchmark vs. AoS baseline from v0.1.0
- [ ] `[RESEARCH]` mmap-backed `ReplayBuffer` — prototype and benchmark vs. heap
- [ ] `[DECISION]` Adopt winning memory layout from research above
- [ ] `[RESEARCH]` Allocator comparison: system allocator vs. jemalloc vs. mimalloc on target edge hardware — musl libc's allocator and glibc's ptmalloc2 are known to perform poorly under the continuous small alloc/dealloc patterns of an RL training loop; jemalloc adds ~300KB binary overhead and may not be available on all targets (e.g., RISC-V with musl); needs measurement before any recommendation
- [ ] `WeightedSampler`
- [ ] `RandomSampler` with replacement
- [ ] Benchmark suite: throughput (samples/sec), memory usage, latency distribution
- [ ] Publish benchmark results in `benches/` with full reproducible methodology (hardware, OS, Rust version, seeds)

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

- [ ] `[DECISION]` File format decision: raw binary vs. Arrow IPC vs. custom — evaluate extensibility to gradient checkpoint representation for FDRL forward compatibility (see ARCHITECTURE.md `## FDRL Design Considerations`)
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
- [ ] MSRV (Minimum Supported Rust Version) declared and tested in CI
- [ ] `Transition: Send + Sync` bound verified — FDRL forward compatibility confirmed

---

## v1.x — torchforge-federated Foundation *(added 2026-03-26)*

**This section is not an implementation target for this crate.** It documents the interface commitments torchforge-data makes at v1.0.0 that torchforge-federated (the FDRL crate) will depend on.

torchforge-federated will implement:
- Gradient aggregation (FedAvg baseline)
- Communication protocol across edge device fleet
- Differential privacy (gradient noise — open)
- Heterogeneous device support

torchforge-data's role at v1.x:
- Provide the per-device `ReplayBuffer` that each federated agent trains against
- The buffer interface must be stable enough that torchforge-federated can depend on it without a breaking change
- The file format chosen at v0.4.0 must be evaluable for gradient checkpoint use — not a requirement, but a consideration

**No torchforge-federated code lives in this crate. The boundary is the `ReplayBuffer` API.**

See project knowledge section 12 (FDRL — Edge Vertical) for full context.

---

## Ongoing

- [ ] Keep dependencies on latest stable versions
- [ ] `cargo audit` clean at all times
- [ ] `cargo clippy -- -D warnings` clean at all times
- [ ] All public items documented (`#![deny(missing_docs)]` enforced from v0.1.0)
- [ ] CHANGELOG.md maintained per [Keep a Changelog](https://keepachangelog.com/)
