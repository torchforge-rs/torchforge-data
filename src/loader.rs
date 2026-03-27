//! DataLoader and configuration
//!
//! This module provides the main `DataLoader` type that combines datasets
//! and samplers to create efficient data loading pipelines.

use crate::dataset::Dataset;
use crate::error::Result;
use crate::sampler::{Sampler, UniformSampler};

/// Configuration for data loading
///
/// This struct defines how data should be loaded and batched.
#[derive(Debug, Clone)]
pub struct LoaderConfig {
    /// Batch size
    pub batch_size: usize,
    /// Whether to shuffle the data
    pub shuffle: bool,
    /// Number of items to prefetch
    pub prefetch: usize,
    /// Random seed for reproducible shuffling
    pub seed: u64,
}

impl Default for LoaderConfig {
    fn default() -> Self {
        Self {
            batch_size: 32,
            shuffle: true,
            prefetch: 2,
            seed: 42,
        }
    }
}

impl LoaderConfig {
    /// Creates a new loader configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the batch size
    pub fn batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    /// Sets whether to shuffle the data
    pub fn shuffle(mut self, shuffle: bool) -> Self {
        self.shuffle = shuffle;
        self
    }

    /// Sets the number of items to prefetch
    pub fn prefetch(mut self, prefetch: usize) -> Self {
        self.prefetch = prefetch;
        self
    }

    /// Sets the random seed
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }
}

/// Main data loader type
///
/// Combines a dataset and sampler to provide efficient batched data loading.
pub struct DataLoader<D: Dataset, S: Sampler = UniformSampler> {
    /// The dataset to load from
    dataset: D,
    /// The sampler to use
    sampler: S,
    /// Configuration
    config: LoaderConfig,
}

impl<D: Dataset> DataLoader<D, UniformSampler> {
    /// Creates a new data loader with the default uniform sampler
    ///
    /// # Arguments
    ///
    /// * `dataset` - The dataset to load from
    /// * `config` - Configuration for loading
    pub fn new(dataset: D, config: LoaderConfig) -> Result<Self> {
        let sampler = if config.shuffle {
            UniformSampler::new(config.seed)
        } else {
            // For non-shuffled case, we could use a sequential sampler
            // For now, use uniform sampler with fixed seed for deterministic order
            UniformSampler::new(0)
        };

        Ok(Self {
            dataset,
            sampler,
            config,
        })
    }
}

impl<D: Dataset, S: Sampler> DataLoader<D, S> {
    /// Creates a new data loader with a custom sampler
    ///
    /// # Arguments
    ///
    /// * `dataset` - The dataset to load from
    /// * `sampler` - The sampler to use
    /// * `config` - Configuration for loading
    pub fn with_sampler(dataset: D, sampler: S, config: LoaderConfig) -> Self {
        Self {
            dataset,
            sampler,
            config,
        }
    }

    /// Returns an iterator over batches
    pub fn iter(&self) -> DataLoaderIter<'_, D, S> {
        DataLoaderIter {
            loader: self,
            sampler_iter: self.sampler.iter(self.dataset.len().unwrap_or(0)),
            current_batch: Vec::with_capacity(self.config.batch_size),
        }
    }
}

/// Iterator over data batches
pub struct DataLoaderIter<'a, D: Dataset, S: Sampler> {
    /// Reference to the data loader
    loader: &'a DataLoader<D, S>,
    /// Iterator over sample indices
    sampler_iter: S::Iter<'a>,
    /// Current batch being built
    current_batch: Vec<usize>,
}

impl<'a, D: Dataset, S: Sampler> Iterator for DataLoaderIter<'a, D, S> {
    type Item = Result<Vec<D::Item<'a>>>;

    fn next(&mut self) -> Option<Self::Item> {
        // Collect indices for the next batch
        self.current_batch.clear();

        while self.current_batch.len() < self.loader.config.batch_size {
            match self.sampler_iter.next() {
                Some(index) => self.current_batch.push(index),
                None => break,
            }
        }

        // If we have no items, return None
        if self.current_batch.is_empty() {
            return None;
        }

        // Collect the actual data items
        let mut batch = Vec::with_capacity(self.current_batch.len());
        for &index in &self.current_batch {
            match self.loader.dataset.get(index) {
                Ok(item) => batch.push(item),
                Err(e) => return Some(Err(e)),
            }
        }

        Some(Ok(batch))
    }
}
