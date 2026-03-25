# torchforge-data

> Zero-copy, streaming data pipeline for edge-native machine learning in Rust.

Part of the [torchforge-rs](https://github.com/torchforge-rs) ecosystem.

[![Crates.io](https://img.shields.io/crates/v/torchforge-data.svg)](https://crates.io/crates/torchforge-data)
[![Docs.rs](https://docs.rs/torchforge-data/badge.svg)](https://docs.rs/torchforge-data)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange.svg)](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/)

---

## Why

Python's `torch.utils.data.DataLoader` has well-documented structural problems on constrained hardware:

- Each worker process duplicates memory — consumption grows linearly with `num_workers`
- Worker processes are torn down and recreated each epoch unless `persistent_workers=True`
- GPU idle time of ~26% measured in profiling studies due to data pipeline stalls
- No path to deterministic latency — the GIL and Python runtime introduce unpredictable pauses

These are acceptable tradeoffs on a 512GB cloud GPU node. They are **not acceptable** on an edge device with 512MB of RAM running a real-time reinforcement learning policy loop.

`torchforge-data` is built for the edge-first case: constrained memory, deterministic latency, continuous operation, no Python runtime.

---

## Design Principles

1. **Zero-copy where provably achievable** — memory-mapped I/O as the default path; copies are explicit and justified
2. **No multiprocessing overhead** — parallelism via `rayon` (CPU) and `tokio` (async I/O), not forked processes
3. **Streaming-first** — datasets never need to fit in RAM
4. **RL-native** — replay buffers are first-class citizens, not afterthoughts
5. **Honest about unknowns** — where optimal design is an open research question, we say so

---

## Status

`v0.0.x` — **Pre-alpha. No stable API. Active design phase.**

See [ARCHITECTURE.md](ARCHITECTURE.md) for current design decisions and open questions.
See [TODO.md](TODO.md) for the implementation roadmap.

---

## Planned Usage

```rust
use torchforge_data::{Dataset, DataLoader, LoaderConfig};

// Streaming dataset from memory-mapped file
let dataset = MmapDataset::open("observations.bin")?;

let loader = DataLoader::new(dataset, LoaderConfig {
    batch_size: 32,
    shuffle: true,
    prefetch: 2,
    ..Default::default()
});

for batch in loader {
    let batch = batch?;
    // batch is a zero-copy view into the memory-mapped region
}
```

> **Note:** This API is illustrative. It will change. Do not depend on it.

---

## Contributing

This project is in active early design. The most valuable contributions right now are:

- Identifying incorrect assumptions in [ARCHITECTURE.md](ARCHITECTURE.md)
- Benchmarks on real edge hardware (Raspberry Pi, Jetson, RISC-V boards)
- Prior art we may have missed

Open an issue before submitting a PR — the design is not yet stable enough for unsolicited implementation PRs.

---

## License

Apache-2.0. See [LICENSE](LICENSE).
