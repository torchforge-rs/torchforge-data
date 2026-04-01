//! Tests for rayon-based parallel item loading degradation on single-core targets
//!
//! This module implements the verification strategy for ensuring that rayon-based
//! parallel loading degrades gracefully on single-core systems.
//!
//! Phase 1: Unit & Property Tests (Pre-implementation)
//! - Define correctness invariants before rayon code is written
//! - Ensure parallel loading produces identical results to sequential loading
//!
//! See [docs/BENCHMARKS.md](docs/BENCHMARKS.md) for complete verification progress.

use proptest::prelude::*;
use std::io::Write;
use tempfile::NamedTempFile;
use torchforge_data::{DataLoader, LoaderConfig, MmapDataset};

/// Creates a temporary dataset file with known data for testing
fn create_test_dataset(size: usize, item_size: usize) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");

    // Write predictable data pattern: item index repeated item_size times
    for i in 0..size {
        let data = vec![i as u8; item_size];
        file.write_all(&data).expect("Failed to write data");
    }

    // Flush to ensure all data is written
    file.flush().expect("Failed to flush file");

    file
}

/// Creates a DataLoader with sequential sampling for baseline comparison
fn create_sequential_loader(
    dataset_size: usize,
    batch_size: usize,
) -> (DataLoader<MmapDataset>, NamedTempFile) {
    let item_size = 4; // 4 bytes per item (f32)
    let temp_file = create_test_dataset(dataset_size, item_size);
    let dataset = MmapDataset::open(temp_file.path()).expect("Failed to open dataset");
    let config = LoaderConfig::new().batch_size(batch_size).shuffle(false); // No shuffling for deterministic comparison

    let loader = DataLoader::new(dataset, config).expect("Failed to create loader");
    (loader, temp_file)
}

/// Phase 1.1: Determinism test
/// Verify that sequential loading produces consistent, predictable results
#[test]
fn test_sequential_determinism() {
    let dataset_size = 100;
    let batch_size = 8;

    let (loader1, _file1) = create_sequential_loader(dataset_size, batch_size);
    let (loader2, _file2) = create_sequential_loader(dataset_size, batch_size);

    let mut batches1 = Vec::new();
    let mut batches2 = Vec::new();

    // Collect all batches from first loader
    for batch_result in loader1.iter().expect("Failed to create iterator") {
        let batch = batch_result.expect("Failed to get batch");
        batches1.push(batch);
    }

    // Collect all batches from second loader
    for batch_result in loader2.iter().expect("Failed to create iterator") {
        let batch = batch_result.expect("Failed to get batch");
        batches2.push(batch);
    }

    // Verify identical results
    assert_eq!(batches1.len(), batches2.len(), "Number of batches should match");

    for (i, (batch1, batch2)) in batches1.iter().zip(batches2.iter()).enumerate() {
        assert_eq!(batch1.len(), batch2.len(), "Batch {} size should match", i);

        for (j, (item1, item2)) in batch1.iter().zip(batch2.iter()).enumerate() {
            assert_eq!(item1, item2, "Item {} in batch {} should match", j, i);
        }
    }
}

/// Phase 1.2: Property-based test for batch correctness
/// Invariant: All batches should have correct size except possibly the last
#[test]
fn test_batch_size_invariant() {
    let dataset_size = 50;
    let batch_size = 8;

    let (loader, _file) = create_sequential_loader(dataset_size, batch_size);
    let iter = loader.iter().expect("Failed to create iterator");

    let mut total_items = 0;
    let mut batch_count = 0;

    for batch_result in iter {
        let batch = batch_result.expect("Failed to get batch");
        batch_count += 1;
        total_items += batch.len();

        // All batches except possibly the last should be full size
        if total_items < dataset_size {
            assert_eq!(
                batch.len(),
                batch_size,
                "Batch {} should be full size ({} items)",
                batch_count,
                batch_size
            );
        }
    }

    // Total items should match dataset size
    assert_eq!(total_items, dataset_size, "Total items should match dataset size");

    // Expected number of batches
    let expected_batches = dataset_size.div_ceil(batch_size);
    assert_eq!(batch_count, expected_batches, "Number of batches should be correct");
}

/// Phase 1.3: Edge case tests
#[test]
fn test_empty_dataset() {
    let dataset_size = 0;
    let batch_size = 8;

    let (loader, _file) = create_sequential_loader(dataset_size, batch_size);
    let iter = loader.iter().expect("Failed to create iterator");

    let batch_count = iter.count();
    assert_eq!(batch_count, 0, "Empty dataset should produce no batches");
}

#[test]
fn test_single_item_dataset() {
    let dataset_size = 1;
    let batch_size = 8;

    let (loader, _file) = create_sequential_loader(dataset_size, batch_size);
    let mut iter = loader.iter().expect("Failed to create iterator");

    let batch_result = iter.next().expect("Should have one batch");
    let batch = batch_result.expect("Batch should be valid");

    assert_eq!(batch.len(), 1, "Single item dataset should produce one item in batch");

    // No more batches
    assert!(iter.next().is_none(), "Should have no more batches");
}

#[test]
fn test_batch_size_larger_than_dataset() {
    let dataset_size = 5;
    let batch_size = 10;

    let (loader, _file) = create_sequential_loader(dataset_size, batch_size);
    let mut iter = loader.iter().expect("Failed to create iterator");

    let batch_result = iter.next().expect("Should have one batch");
    let batch = batch_result.expect("Batch should be valid");

    assert_eq!(batch.len(), 5, "Batch should contain all dataset items");

    // No more batches
    assert!(iter.next().is_none(), "Should have no more batches");
}

#[test]
fn test_batch_size_one() {
    let dataset_size = 10;
    let batch_size = 1;

    let (loader, _file) = create_sequential_loader(dataset_size, batch_size);
    let iter = loader.iter().expect("Failed to create iterator");

    let batch_count = iter.count();
    assert_eq!(batch_count, 10, "Batch size 1 should produce one batch per item");
}

proptest! {
    #[test]
    fn test_sequential_property(batch_size in 1usize..=128, dataset_size in 1000usize..=10000) {
        // Skip invalid combinations
        if batch_size == 0 {
            return Ok(());
        }

        let (loader, _file) = create_sequential_loader(dataset_size, batch_size);
        let iter = loader.iter().expect("Failed to create iterator");

        let mut total_items = 0;
        let mut batch_count = 0;

        for batch_result in iter {
            let batch = batch_result.expect("Failed to get batch");
            batch_count += 1;
            total_items += batch.len();

            // Batch size should never exceed configured size
            prop_assert!(batch.len() <= batch_size,
                        "Batch size {} should not exceed configured {}", batch.len(), batch_size);
        }

        // Total items should match dataset size
        prop_assert_eq!(total_items, dataset_size,
                       "Total items {} should match dataset size {}", total_items, dataset_size);

        // Expected number of batches
        let expected_batches = dataset_size.div_ceil(batch_size);
        prop_assert_eq!(batch_count, expected_batches,
                       "Batch count {} should match expected {}", batch_count, expected_batches);
    }
}

/// Phase 1.5: Data integrity test
/// Verify that loaded data matches the expected pattern
#[test]
fn test_data_integrity() {
    let dataset_size = 8;
    let batch_size = 2;
    let item_size = 4; // 4 bytes per item

    let (loader, _temp_file) = create_sequential_loader(dataset_size, batch_size);

    let iter = loader.iter().expect("Failed to create iterator");

    let mut total_items = 0;

    for batch_result in iter {
        let batch = batch_result.expect("Failed to get batch");
        
        for item in batch {
            let data = item;
            assert_eq!(data.len(), item_size, "Item should have correct size");
            
            // Simple check: ensure all bytes in an item are the same
            if data.len() > 1 {
                let first_byte = data[0];
                for &byte in data.iter().skip(1) {
                    assert_eq!(byte, first_byte, "All bytes in an item should be the same");
                }
            }
            
            total_items += 1;
        }
    }

    assert_eq!(total_items, dataset_size, "Should have processed all items");
}
