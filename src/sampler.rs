//! Sampler trait and implementations
//!
//! This module defines the `Sampler` trait and various sampling strategies
//! for creating batches from datasets.

/// Trait for sampling strategies
///
/// Samplers are responsible for determining which items from a dataset
/// should be included in each batch and in what order.
pub trait Sampler {
    /// The type of iterator produced by this sampler
    type Iter<'a>: Iterator<Item = usize>
    where
        Self: 'a;

    /// Creates an iterator over sample indices
    ///
    /// # Arguments
    ///
    /// * `len` - The total number of items in the dataset
    ///
    /// # Returns
    ///
    /// An iterator that yields indices to be sampled
    fn iter(&self, len: usize) -> Self::Iter<'_>;
}

/// Uniform random sampler
///
/// Samples items uniformly at random without replacement for each epoch.
///
/// **Note**: This implementation uses a simple deterministic Linear Congruential Generator (LCG)
/// for reproducibility. It is NOT suitable for cryptographic purposes or applications requiring
/// high-quality randomness. For production use with proper randomness, consider using the `rand` crate.
#[derive(Debug, Clone)]
pub struct UniformSampler {
    /// Random number generator seed
    seed: u64,
}

impl UniformSampler {
    /// Creates a new uniform sampler
    ///
    /// # Arguments
    ///
    /// * `seed` - Seed for the random number generator
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }
}

impl Sampler for UniformSampler {
    type Iter<'a> = UniformSamplerIter;

    fn iter(&self, len: usize) -> Self::Iter<'_> {
        UniformSamplerIter::new(len, self.seed)
    }
}

/// Iterator for uniform sampling
pub struct UniformSamplerIter {
    /// Indices to sample
    indices: Vec<usize>,
    /// Current position
    position: usize,
}

impl UniformSamplerIter {
    /// Creates a new uniform sampler iterator
    fn new(len: usize, seed: u64) -> Self {
        let mut indices: Vec<usize> = (0..len).collect();

        // Use a simple deterministic shuffle for now
        // In a real implementation, we'd use a proper PRNG
        let mut rng_state = seed;
        for i in (1..len).rev() {
            rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
            let j = rng_state as usize % (i + 1);
            indices.swap(i, j);
        }

        Self {
            indices,
            position: 0,
        }
    }
}

impl Iterator for UniformSamplerIter {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position < self.indices.len() {
            let index = self.indices[self.position];
            self.position += 1;
            Some(index)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.indices.len().saturating_sub(self.position);
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for UniformSamplerIter {}

/// Sequential sampler
///
/// Samples items in sequential order from 0 to len-1.
/// This is the simplest sampling strategy, useful for deterministic
/// data processing and debugging.
#[derive(Debug, Clone, Copy)]
pub struct SequentialSampler;

impl SequentialSampler {
    /// Creates a new sequential sampler
    pub fn new() -> Self {
        Self
    }
}

impl Default for SequentialSampler {
    fn default() -> Self {
        Self::new()
    }
}

impl Sampler for SequentialSampler {
    type Iter<'a> = SequentialSamplerIter;

    fn iter(&self, len: usize) -> Self::Iter<'_> {
        SequentialSamplerIter::new(len)
    }
}

/// Iterator for sequential sampling
pub struct SequentialSamplerIter {
    /// Current index
    current: usize,
    /// Total length
    len: usize,
}

impl SequentialSamplerIter {
    /// Creates a new sequential sampler iterator
    fn new(len: usize) -> Self {
        Self { current: 0, len }
    }
}

impl Iterator for SequentialSamplerIter {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.len {
            let index = self.current;
            self.current += 1;
            Some(index)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len.saturating_sub(self.current);
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for SequentialSamplerIter {}
