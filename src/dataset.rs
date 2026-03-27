//! Dataset trait and implementations
//!
//! This module defines the core `Dataset` trait and various implementations
//! for different data sources and storage mechanisms.

use crate::error::{DataError, Result};

/// Core dataset trait with zero-copy support
///
/// This trait defines the interface for all dataset implementations.
/// The use of Generic Associated Types (GATs) allows for zero-copy
/// operations where possible.
pub trait Dataset {
    /// The type of data items returned by the dataset
    type Item<'a>
    where
        Self: 'a;

    /// Returns the number of items in the dataset
    fn len(&self) -> Result<usize>;

    /// Returns true if the dataset is empty
    fn is_empty(&self) -> Result<bool> {
        self.len().map(|l| l == 0)
    }

    /// Returns the item at the given index
    fn get(&self, index: usize) -> Result<Self::Item<'_>>;

    /// Returns an iterator over the dataset items
    fn iter(&self) -> DatasetIter<'_, Self>
    where
        Self: Sized,
    {
        DatasetIter { dataset: self, index: 0, len: self.len().unwrap_or(0) }
    }
}

/// Iterator over dataset items
pub struct DatasetIter<'a, D: Dataset> {
    dataset: &'a D,
    index: usize,
    len: usize,
}

impl<'a, D: Dataset> Iterator for DatasetIter<'a, D> {
    type Item = Result<D::Item<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.len {
            let result = self.dataset.get(self.index);
            self.index += 1;
            Some(result)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len.saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}

impl<'a, D: Dataset> ExactSizeIterator for DatasetIter<'a, D> {}

/// Memory-mapped dataset implementation
///
/// This implementation provides zero-copy access to data stored
/// in files using memory mapping.
pub struct MmapDataset {
    /// Memory-mapped data
    data: memmap2::Mmap,
    /// Item size in bytes
    item_size: usize,
    /// Number of items
    len: usize,
}

impl MmapDataset {
    /// Creates a new memory-mapped dataset from a file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the data file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened or memory mapped
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        let mmap = unsafe { memmap2::Mmap::map(&file)? };

        // For now, assume fixed item size - this will be made configurable
        let item_size = 4; // Placeholder

        // Validate item_size to prevent division by zero
        if item_size == 0 {
            return Err(DataError::Config("item_size must be greater than 0".to_string()));
        }

        let len = mmap.len() / item_size;

        Ok(Self { data: mmap, item_size, len })
    }
}

impl Dataset for MmapDataset {
    type Item<'a>
        = &'a [u8]
    where
        Self: 'a;

    fn len(&self) -> Result<usize> {
        Ok(self.len)
    }

    fn get(&self, index: usize) -> Result<Self::Item<'_>> {
        if index >= self.len {
            return Err(DataError::Format(format!(
                "Index {} out of bounds for dataset of length {}",
                index, self.len
            )));
        }

        // Use checked arithmetic to prevent overflow
        let start = index.checked_mul(self.item_size).ok_or_else(|| {
            DataError::Format(
                "Index overflow - multiplication would exceed usize bounds".to_string(),
            )
        })?;
        let end = start.checked_add(self.item_size).ok_or_else(|| {
            DataError::Format("Index overflow - addition would exceed usize bounds".to_string())
        })?;

        if end > self.data.len() {
            return Err(DataError::Format(
                "Invalid item size would exceed data bounds".to_string(),
            ));
        }

        Ok(&self.data[start..end])
    }
}
