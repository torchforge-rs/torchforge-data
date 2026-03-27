//! DataLoader and configuration
//!
//! This module provides the main `DataLoader` type that combines datasets
//! and samplers to create efficient data loading pipelines.

use crate::dataset::Dataset;
use crate::error::Result;
use crate::sampler::{Sampler, UniformSampler};

/// Trait for collating individual items into batches
///
/// This trait allows different types to define how they should be combined
/// into batches for efficient processing.
pub trait Collate<T> {
    /// The output type after collation
    type Output;

    /// Collates a collection of items into a single batch
    ///
    /// # Arguments
    ///
    /// * `items` - Collection of items to collate
    ///
    /// # Errors
    ///
    /// Returns an error if collation fails
    fn collate(items: Vec<T>) -> Result<Self::Output>;
}

/// Default collation for slices - returns them as-is
impl<'a, T: ?Sized> Collate<&'a T> for Vec<&'a T> {
    type Output = Vec<&'a T>;

    fn collate(items: Vec<&'a T>) -> Result<Self::Output> {
        Ok(items)
    }
}

/// Collation for f32 slices into Vec<f32>
impl<'a> Collate<&'a [u8]> for Vec<f32> {
    type Output = Vec<f32>;

    fn collate(items: Vec<&'a [u8]>) -> Result<Self::Output> {
        let total_len: usize = items.iter().map(|item| item.len() / 4).sum();
        let mut result = Vec::with_capacity(total_len);

        for item in items {
            if item.len() % 4 != 0 {
                return Err(crate::error::DataError::Format(
                    "f32 collation requires slice length to be multiple of 4".to_string(),
                ));
            }

            // Convert bytes to f32
            let chunks = item.chunks_exact(4);
            for chunk in chunks {
                let bytes: [u8; 4] = chunk.try_into().map_err(|_| {
                    crate::error::DataError::Format("Failed to convert slice to f32".to_string())
                })?;
                let value = f32::from_le_bytes(bytes);
                result.push(value);
            }
        }

        Ok(result)
    }
}

/// Collation for i64 slices into Vec<i64>
impl<'a> Collate<&'a [u8]> for Vec<i64> {
    type Output = Vec<i64>;

    fn collate(items: Vec<&'a [u8]>) -> Result<Self::Output> {
        let total_len: usize = items.iter().map(|item| item.len() / 8).sum();
        let mut result = Vec::with_capacity(total_len);

        for item in items {
            if item.len() % 8 != 0 {
                return Err(crate::error::DataError::Format(
                    "i64 collation requires slice length to be multiple of 8".to_string(),
                ));
            }

            // Convert bytes to i64
            let chunks = item.chunks_exact(8);
            for chunk in chunks {
                let bytes: [u8; 8] = chunk.try_into().map_err(|_| {
                    crate::error::DataError::Format("Failed to convert slice to i64".to_string())
                })?;
                let value = i64::from_le_bytes(bytes);
                result.push(value);
            }
        }

        Ok(result)
    }
}

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
    ///
    /// # Panics
    ///
    /// Panics if batch_size is 0
    pub fn batch_size(mut self, batch_size: usize) -> Self {
        assert!(batch_size > 0, "batch_size must be greater than 0");
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
    ///
    /// # Errors
    ///
    /// Returns an error if the dataset length cannot be determined
    pub fn iter(&self) -> Result<DataLoaderIter<'_, D, S>> {
        let len = self.dataset.len()?;
        Ok(DataLoaderIter {
            loader: self,
            sampler_iter: self.sampler.iter(len),
            current_batch: Vec::with_capacity(self.config.batch_size),
        })
    }

    /// Returns an iterator over collated batches
    ///
    /// This iterator collates individual items into batched arrays. For now,
    /// this is implemented for common primitive types. Future versions will
    /// support more flexible collation.
    ///
    /// # Errors
    ///
    /// Returns an error if the dataset length cannot be determined or collation fails
    pub fn iter_collated_f32(&self) -> Result<impl Iterator<Item = Result<Vec<f32>>> + '_>
    where
        for<'a> D::Item<'a>: AsRef<[u8]>,
    {
        let len = self.dataset.len()?;
        Ok(CollatedF32Iter {
            loader: self,
            sampler_iter: self.sampler.iter(len),
            current_batch: Vec::with_capacity(self.config.batch_size),
        })
    }

    /// Returns an iterator over collated batches for i64 data
    ///
    /// Similar to `iter_collated_f32` but for i64 data.
    ///
    /// # Errors
    ///
    /// Returns an error if the dataset length cannot be determined or collation fails
    pub fn iter_collated_i64(&self) -> Result<impl Iterator<Item = Result<Vec<i64>>> + '_>
    where
        for<'a> D::Item<'a>: AsRef<[u8]>,
    {
        let len = self.dataset.len()?;
        Ok(CollatedI64Iter {
            loader: self,
            sampler_iter: self.sampler.iter(len),
            current_batch: Vec::with_capacity(self.config.batch_size),
        })
    }
}

/// Iterator over data batches
///
/// **Note**: If an error occurs while accessing dataset items during batch construction,
/// the entire batch is discarded and the error is returned immediately. Partial batches
/// are not preserved in error conditions.
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

/// Iterator over collated f32 batches
///
/// This iterator collates byte slices into f32 vectors.
/// **Note**: If an error occurs during batch construction or collation, the entire batch
/// is discarded and the error is returned immediately.
pub struct CollatedF32Iter<'a, D: Dataset, S: Sampler> {
    /// Reference to the data loader
    loader: &'a DataLoader<D, S>,
    /// Iterator over sample indices
    sampler_iter: S::Iter<'a>,
    /// Current batch being built
    current_batch: Vec<usize>,
}

impl<'a, D: Dataset, S: Sampler> Iterator for CollatedF32Iter<'a, D, S>
where
    D::Item<'a>: AsRef<[u8]>,
{
    type Item = Result<Vec<f32>>;

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

        // Collect the actual data items and collate into f32 array
        let total_len: usize = self
            .current_batch
            .iter()
            .map(|&index| {
                let item = self.loader.dataset.get(index);
                match item {
                    Ok(slice) => slice.as_ref().len() / 4,
                    Err(_) => 0,
                }
            })
            .sum();

        let mut result = Vec::with_capacity(total_len);

        for &index in &self.current_batch {
            match self.loader.dataset.get(index) {
                Ok(item) => {
                    let slice = item.as_ref();
                    if slice.len() % 4 != 0 {
                        return Some(Err(crate::error::DataError::Format(
                            "f32 collation requires slice length to be multiple of 4".to_string(),
                        )));
                    }

                    // Convert bytes to f32
                    let chunks = slice.chunks_exact(4);
                    for chunk in chunks {
                        let bytes: [u8; 4] = match chunk.try_into() {
                            Ok(bytes) => bytes,
                            Err(_) => {
                                return Some(Err(crate::error::DataError::Format(
                                    "Failed to convert slice to f32".to_string(),
                                )));
                            }
                        };
                        let value = f32::from_le_bytes(bytes);
                        result.push(value);
                    }
                }
                Err(e) => return Some(Err(e)),
            }
        }

        Some(Ok(result))
    }
}

/// Iterator over collated i64 batches
///
/// This iterator collates byte slices into i64 vectors.
/// **Note**: If an error occurs during batch construction or collation, the entire batch
/// is discarded and the error is returned immediately.
pub struct CollatedI64Iter<'a, D: Dataset, S: Sampler> {
    /// Reference to the data loader
    loader: &'a DataLoader<D, S>,
    /// Iterator over sample indices
    sampler_iter: S::Iter<'a>,
    /// Current batch being built
    current_batch: Vec<usize>,
}

impl<'a, D: Dataset, S: Sampler> Iterator for CollatedI64Iter<'a, D, S>
where
    D::Item<'a>: AsRef<[u8]>,
{
    type Item = Result<Vec<i64>>;

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

        // Collect the actual data items and collate into i64 array
        let total_len: usize = self
            .current_batch
            .iter()
            .map(|&index| {
                let item = self.loader.dataset.get(index);
                match item {
                    Ok(slice) => slice.as_ref().len() / 8,
                    Err(_) => 0,
                }
            })
            .sum();

        let mut result = Vec::with_capacity(total_len);

        for &index in &self.current_batch {
            match self.loader.dataset.get(index) {
                Ok(item) => {
                    let slice = item.as_ref();
                    if slice.len() % 8 != 0 {
                        return Some(Err(crate::error::DataError::Format(
                            "i64 collation requires slice length to be multiple of 8".to_string(),
                        )));
                    }

                    // Convert bytes to i64
                    let chunks = slice.chunks_exact(8);
                    for chunk in chunks {
                        let bytes: [u8; 8] = match chunk.try_into() {
                            Ok(bytes) => bytes,
                            Err(_) => {
                                return Some(Err(crate::error::DataError::Format(
                                    "Failed to convert slice to i64".to_string(),
                                )));
                            }
                        };
                        let value = i64::from_le_bytes(bytes);
                        result.push(value);
                    }
                }
                Err(e) => return Some(Err(e)),
            }
        }

        Some(Ok(result))
    }
}
