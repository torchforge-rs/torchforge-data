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

#![deny(missing_docs)]

pub mod dataset;
pub mod error;
pub mod loader;
pub mod sampler;

// Re-export main types for convenience
pub use error::{DataError, Result};
pub use dataset::{Dataset, MmapDataset};
pub use sampler::Sampler;
pub use loader::{DataLoader, LoaderConfig};
