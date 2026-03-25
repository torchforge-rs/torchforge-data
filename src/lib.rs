//! # torchforge-data
//!
//! Zero-copy, streaming DataLoader for edge-native ML pipelines in Rust.
//!
//! Designed to replace Python's `torch.utils.data.DataLoader` with a
//! memory-efficient, streaming-first alternative that runs on constrained
//! edge hardware without GC pressure or multi-process overhead.
//!
//! ## Status
//!
//! Active development. API is not yet stable.
