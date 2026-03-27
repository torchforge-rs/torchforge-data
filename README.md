# torchforge-data

> Zero-copy, streaming data pipeline for edge-native machine learning in Rust.

Part of the [torchforge-rs](https://github.com/torchforge-rs) ecosystem.

[![Crates.io](https://img.shields.io/crates/v/torchforge-data.svg)](https://crates.io/crates/torchforge-data)
[![Docs.rs](https://docs.rs/torchforge-data/badge.svg)](https://docs.rs/torchforge-data)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/torchforge-rs/torchforge-data/blob/main/LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange.svg)](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/)
[![CI](https://github.com/torchforge-rs/torchforge-data/actions/workflows/ci.yml/badge.svg)](https://github.com/torchforge-rs/torchforge-data/actions/workflows/ci.yml)

---

## The Destination

The long-term target of the torchforge-rs ecosystem is **Federated Deep Reinforcement Learning (FDRL) at the edge**: a fleet of constrained devices, each running a local DRL agent learning from its own physical environment, sharing only gradients — not raw data — with a coordinator. No cloud. No Python. No centralized data collection.

`torchforge-data` is the foundation layer for that target. The `ReplayBuffer` interface is designed with the federation boundary in mind from day one: what crosses the wire at v1.x are gradients, not experience tuples. That constraint shapes the data layer now.

This crate is v0.x infrastructure. FDRL is the v1.x target. The claim is not yet earned — the foundation has to be built first.

---

## Why

Python's `torch.utils.data.DataLoader` has well-documented structural problems on constrained hardware:

* Each worker process duplicates memory — consumption grows linearly with `num_workers`
* Worker processes are torn down and recreated each epoch unless `persistent_workers=True`
* GPU idle time of ~26% measured in profiling studies due to data pipeline stalls (arXiv:2211.04908)
* No path to deterministic latency — the GIL and Python runtime introduce unpredictable pauses

These are acceptable tradeoffs on a 512GB cloud GPU node. They are **not acceptable** on an edge device with 512MB of RAM running a real-time reinforcement learning policy loop.

`torchforge-data` is built for the edge-first case: constrained memory, deterministic latency, continuous operation, no Python runtime.

---

## Design Principles

1. **Zero-copy where provably achievable** — memory-mapped I/O as the default path; copies are explicit and justified
2. **No multiprocessing overhead** — parallelism via `rayon` (CPU) and `tokio` (async I/O, v0.5+), not forked processes
3. **Streaming-first** — datasets never need to fit in RAM
4. **RL-native** — replay buffers are first-class citizens, not afterthoughts
5. **Federation-aware by design** — the `ReplayBuffer` interface is designed with the FDRL boundary in mind: gradients are shared across devices, not raw experience tuples. No federation code ships here; the interface commitment is that none will be needed at v1.x.
6. **Honest about unknowns** — where optimal design is an open research question, we say so

---

## Status

**`v0.0.1` — Pre-alpha. No stable API. Active design phase.**

The repository structure, CI, governance documents, and OSS foundation are complete. Implementation of core abstractions begins at v0.1.0.

See [ARCHITECTURE.md](https://github.com/torchforge-rs/torchforge-data/blob/main/ARCHITECTURE.md) for current design decisions and open questions.
See [TODO.md](https://github.com/torchforge-rs/torchforge-data/blob/main/TODO.md) for the full implementation roadmap.

---

## Roadmap

| Version | Goal |
| --- | --- |
| **v0.1.0** | `Dataset`, `DataLoader`, `ReplayBuffer` (AoS baseline), `MmapDataset` — enough for a DQN training loop |
| **v0.2.0** | `rayon` parallelism, SoA vs AoS benchmark, allocator comparison, prefetch buffer |
| **v0.3.0** | `PrioritizedReplayBuffer`, N-step returns, episode boundary tracking |
| **v0.4.0** | File format decision, Python interop reader/writer |
| **v0.5.0** | Async `DataLoader` via `tokio` |
| **v1.0.0** | Stable API, published edge benchmark, real RL training loop end-to-end |

---

## Planned API

> **⚠️ Illustrative only — will change before v0.1.0 is released.**

```rust
use torchforge_data::{Dataset, DataLoader, LoaderConfig, ReplayBuffer};

// Streaming dataset from memory-mapped file
let dataset = MmapDataset::open("observations.bin")?;

let loader = DataLoader::new(dataset, LoaderConfig {
    batch_size: 32,
    shuffle: true,
    prefetch: 2,
    ..Default::default()
});

for batch in &loader {
    let batch = batch?;
    // batch is a zero-copy view into the memory-mapped region
}

// Replay buffer for online RL
let mut buffer: ReplayBuffer<Obs, Action, f32> = ReplayBuffer::new(50_000);
buffer.push(transition)?;
let batch = buffer.sample(32)?;
```

---

## Contributing

See [CONTRIBUTING.md](https://github.com/torchforge-rs/torchforge-data/blob/main/CONTRIBUTING.md) for the full guide — prerequisites, branching model, PR process, and what "ready to merge" means for this project.

The most valuable contributions right now are:

* Identifying incorrect assumptions in [ARCHITECTURE.md](https://github.com/torchforge-rs/torchforge-data/blob/main/ARCHITECTURE.md)
* Benchmarks on real edge hardware (Raspberry Pi, Jetson, RISC-V boards)
* Prior art we may have missed

**Open an issue before submitting a PR** — the design is not yet stable enough for unsolicited implementation PRs.

Please read our [Code of Conduct](https://github.com/torchforge-rs/torchforge-data/blob/main/CODE_OF_CONDUCT.md) before participating.
To report a security issue, see [SECURITY.md](https://github.com/torchforge-rs/torchforge-data/blob/main/SECURITY.md).

---

## License

Apache-2.0. See [LICENSE](https://github.com/torchforge-rs/torchforge-data/blob/main/LICENSE).

Part of the [torchforge-rs](https://github.com/torchforge-rs) ecosystem — also see [torchforge-viz](https://github.com/torchforge-rs/torchforge-viz) and [torchforge-bench](https://github.com/torchforge-rs/torchforge-bench).
