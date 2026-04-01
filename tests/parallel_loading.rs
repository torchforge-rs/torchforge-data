//! Tests for parallel loading functionality
//!
//! This module tests the rayon-based parallel loading implementation
//! and verifies it produces the same results as sequential loading.

use std::io::Write;
use tempfile::NamedTempFile;
use torchforge_data::{DataLoader, LoaderConfig, MmapDataset};

/// Creates a test dataset file
fn create_test_dataset(size: usize, item_size: usize) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");

    for i in 0..size {
        let data = vec![(i % 256) as u8; item_size];
        file.write_all(&data).expect("Failed to write data");
    }

    file.flush().expect("Failed to flush file");
    file
}

/// Creates a DataLoader with specified configuration
fn create_loader(
    dataset_size: usize,
    batch_size: usize,
    parallel: bool,
) -> DataLoader<MmapDataset> {
    let item_size = 4; // 4 bytes per item (f32)
    let temp_file = create_test_dataset(dataset_size, item_size);
    let dataset = MmapDataset::open(temp_file.path()).expect("Failed to open dataset");
    let config = LoaderConfig::new().batch_size(batch_size).shuffle(false).parallel(parallel);

    DataLoader::new(dataset, config).expect("Failed to create loader")
}

#[test]
fn test_parallel_vs_sequential_correctness() {
    let dataset_size = 100;
    let batch_size = 8;

    let sequential_loader = create_loader(dataset_size, batch_size, false);
    let parallel_loader = create_loader(dataset_size, batch_size, true);

    // Collect all batches from sequential loader
    let mut sequential_batches = Vec::new();
    for batch_result in sequential_loader.iter().expect("Failed to create iterator") {
        let batch = batch_result.expect("Failed to get batch");
        sequential_batches.push(batch);
    }

    // Collect all batches from parallel loader
    let mut parallel_batches = Vec::new();
    for batch_result in parallel_loader.iter().expect("Failed to create iterator") {
        let batch = batch_result.expect("Failed to get batch");
        parallel_batches.push(batch);
    }

    // Verify identical results
    assert_eq!(
        sequential_batches.len(),
        parallel_batches.len(),
        "Number of batches should match"
    );

    for (i, (seq_batch, par_batch)) in
        sequential_batches.iter().zip(parallel_batches.iter()).enumerate()
    {
        assert_eq!(seq_batch.len(), par_batch.len(), "Batch {} size should match", i);

        for (j, (seq_item, par_item)) in seq_batch.iter().zip(par_batch.iter()).enumerate() {
            assert_eq!(
                seq_item, par_item,
                "Item {} in batch {} should match",
                j, i
            );
        }
    }
}

#[test]
fn test_parallel_collated_f32_correctness() {
    let dataset_size = 50;
    let batch_size = 4;

    let sequential_loader = create_loader(dataset_size, batch_size, false);
    let parallel_loader = create_loader(dataset_size, batch_size, true);

    // Collect all batches from sequential loader
    let mut sequential_batches = Vec::new();
    for batch_result in sequential_loader.iter_collated_f32().expect("Failed to create iterator") {
        let batch = batch_result.expect("Failed to get batch");
        sequential_batches.push(batch);
    }

    // Collect all batches from parallel loader
    let mut parallel_batches = Vec::new();
    for batch_result in parallel_loader.iter_collated_f32().expect("Failed to create iterator") {
        let batch = batch_result.expect("Failed to get batch");
        parallel_batches.push(batch);
    }

    // Verify identical results
    assert_eq!(
        sequential_batches.len(),
        parallel_batches.len(),
        "Number of batches should match"
    );

    for (i, (seq_batch, par_batch)) in
        sequential_batches.iter().zip(parallel_batches.iter()).enumerate()
    {
        assert_eq!(seq_batch.len(), par_batch.len(), "Batch {} size should match", i);
        assert_eq!(seq_batch, par_batch, "Batch {} content should match", i);
    }
}

/// Explicitly tests that parallel loading preserves item order within batches
/// by comparing sequential and parallel loaders item-by-item in lockstep
#[test]
fn test_parallel_order_preservation() {
    let dataset_size = 100;
    let batch_size = 10;

    // Create loaders with same dataset
    let sequential_loader = create_loader(dataset_size, batch_size, false);
    let parallel_loader = create_loader(dataset_size, batch_size, true);

    let mut seq_iter = sequential_loader.iter().expect("Failed to create sequential iterator");
    let mut par_iter = parallel_loader.iter().expect("Failed to create parallel iterator");

    let mut batch_idx = 0;

    // Compare batches one at a time
    loop {
        match (seq_iter.next(), par_iter.next()) {
            (Some(seq_batch), Some(par_batch)) => {
                let seq_batch = seq_batch.expect("Sequential batch should be valid");
                let par_batch = par_batch.expect("Parallel batch should be valid");

                assert_eq!(
                    seq_batch.len(),
                    par_batch.len(),
                    "Batch {}: sizes should match",
                    batch_idx
                );

                // Verify each item matches in order
                for (item_idx, (seq_item, par_item)) in seq_batch.iter().zip(par_batch.iter()).enumerate() {
                    assert_eq!(
                        seq_item, par_item,
                        "Batch {}, Item {}: items should match (order preserved)",
                        batch_idx, item_idx
                    );
                }

                batch_idx += 1;
            }
            (None, None) => break, // Both exhausted - success
            (Some(_), None) => panic!("Sequential iterator has more batches than parallel"),
            (None, Some(_)) => panic!("Parallel iterator has more batches than sequential"),
        }
    }

    assert!(batch_idx > 0, "Should have processed at least one batch");
}

#[test]
fn test_parallel_configuration() {
    let dataset_size = 20;
    let batch_size = 4;

    let config = LoaderConfig::new().batch_size(batch_size).parallel(true).num_threads(2);

    assert_eq!(config.batch_size, batch_size);
    assert!(config.parallel);
    assert_eq!(config.num_threads, 2);

    // Test that parallel loader can be created
    let loader = create_loader(dataset_size, batch_size, true);
    let iter = loader.iter().expect("Failed to create iterator");

    // Should be able to iterate without panicking
    let batch_count = iter.count();
    assert_eq!(batch_count, 5); // 20 items / 4 batch_size = 5 batches
}
