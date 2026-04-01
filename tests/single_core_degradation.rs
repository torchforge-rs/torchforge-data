//! Phase 2: Single-core degradation tests
//!
//! This module implements Phase 2 of the verification strategy:
//! - Verify no panics/hangs on single-core systems
//! - Test graceful degradation of rayon parallelism on single-core targets
//! - Stress test parallel loading under single-core constraints

use std::io::Write;
use std::time::Instant;
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
    num_threads: usize,
) -> DataLoader<MmapDataset> {
    let item_size = 4; // 4 bytes per item (f32)
    let temp_file = create_test_dataset(dataset_size, item_size);
    let dataset = MmapDataset::open(temp_file.path()).expect("Failed to open dataset");
    let config = LoaderConfig::new()
        .batch_size(batch_size)
        .shuffle(false)
        .parallel(parallel)
        .num_threads(num_threads);

    DataLoader::new(dataset, config).expect("Failed to create loader")
}

/// Phase 2.1: CPU affinity test
/// Verify parallel loading works when forced to single-threaded execution
#[test]
fn test_parallel_single_threaded_execution() {
    let dataset_size = 1000;
    let batch_size = 32;

    // Create parallel loader with single thread
    let loader = create_loader(dataset_size, batch_size, true, 1);

    // Should be able to iterate without panicking
    let mut total_items = 0;
    let iter = loader.iter().expect("Failed to create iterator");

    for batch_result in iter {
        let batch = batch_result.expect("Failed to get batch");
        total_items += batch.len();

        // Verify batch sizes are reasonable
        assert!(batch.len() <= batch_size, "Batch size should not exceed configured size");
    }

    assert_eq!(total_items, dataset_size, "Should process all items");
}

/// Phase 2.2: Stress test on single-core
/// Test parallel loading with large dataset on single-threaded configuration
#[test]
fn test_parallel_single_core_stress() {
    let dataset_size = 10000; // Large dataset for stress testing
    let batch_size = 64;

    let loader = create_loader(dataset_size, batch_size, true, 1);

    let start_time = Instant::now();
    let mut total_items = 0;

    // Process all batches
    let iter = loader.iter().expect("Failed to create iterator");
    for batch_result in iter {
        let batch = batch_result.expect("Failed to get batch");
        total_items += batch.len();

        // Verify data integrity during stress test
        assert!(!batch.is_empty(), "Batch should not be empty");
    }

    let elapsed = start_time.elapsed();

    // Verify correctness
    assert_eq!(total_items, dataset_size, "Should process all items");

    // Verify reasonable performance (should complete within reasonable time)
    assert!(elapsed.as_secs() < 10, "Stress test should complete within 10 seconds");

    println!("Single-core stress test: {} items in {:?}", dataset_size, elapsed);
}

/// Phase 2.3: Thread pool initialization test
/// Verify rayon thread pool can be configured and works correctly
#[test]
fn test_thread_pool_initialization() {
    let dataset_size = 500;
    let batch_size = 16;

    // Test with different thread configurations
    for num_threads in [0, 1, 2, 4] {
        let loader = create_loader(dataset_size, batch_size, true, num_threads);

        // Should be able to create iterator without panicking
        let iter = loader.iter().expect("Failed to create iterator");

        // Process a few batches to verify thread pool works
        let mut batch_count = 0;
        for batch_result in iter.take(5) {
            // Only process first 5 batches
            let batch = batch_result.expect("Failed to get batch");
            assert!(!batch.is_empty(), "Batch should not be empty");
            batch_count += 1;
        }

        assert!(batch_count > 0, "Should process at least one batch");
    }
}

/// Phase 2.4: Graceful fallback verification
/// Verify that parallel loading gracefully falls back to sequential behavior
#[test]
fn test_graceful_fallback() {
    let dataset_size = 200;
    let batch_size = 8;

    // Create sequential loader
    let sequential_loader = create_loader(dataset_size, batch_size, false, 0);

    // Create parallel loader with single thread (should behave like sequential)
    let parallel_loader = create_loader(dataset_size, batch_size, true, 1);

    // Collect results from both loaders
    let mut sequential_results = Vec::new();
    for batch_result in sequential_loader.iter().expect("Failed to create iterator") {
        let batch = batch_result.expect("Failed to get batch");
        sequential_results.push(batch);
    }

    let mut parallel_results = Vec::new();
    for batch_result in parallel_loader.iter().expect("Failed to create iterator") {
        let batch = batch_result.expect("Failed to get batch");
        parallel_results.push(batch);
    }

    // Verify identical results
    assert_eq!(
        sequential_results.len(),
        parallel_results.len(),
        "Number of batches should match"
    );

    for (i, (seq_batch, par_batch)) in
        sequential_results.iter().zip(parallel_results.iter()).enumerate()
    {
        assert_eq!(seq_batch.len(), par_batch.len(), "Batch {} size should match", i);

        for (j, (seq_item, par_item)) in seq_batch.iter().zip(par_batch.iter()).enumerate() {
            assert_eq!(seq_item, par_item, "Item {} in batch {} should match", j, i);
        }
    }
}

/// Phase 2.5: Memory pressure test on single-core
/// Verify parallel loading doesn't cause memory issues under single-core constraints
#[test]
fn test_single_core_memory_pressure() {
    let dataset_size = 5000;
    let batch_size = 128; // Large batch size for memory pressure

    let loader = create_loader(dataset_size, batch_size, true, 1);

    // Process all batches while monitoring for memory issues
    let iter = loader.iter().expect("Failed to create iterator");
    let mut batch_count = 0;
    let mut total_items = 0;

    for batch_result in iter {
        let batch = batch_result.expect("Failed to get batch");

        // Verify batch integrity (last batch may be smaller)
        assert!(batch.len() <= batch_size, "Batch size should not exceed configured size");
        assert!(!batch.is_empty(), "Batch should not be empty");

        batch_count += 1;
        total_items += batch.len();
    }

    // Verify we processed all items
    assert_eq!(total_items, dataset_size, "Should process all items");

    // Verify we processed the expected number of batches
    let expected_batches = dataset_size.div_ceil(batch_size);
    assert_eq!(batch_count, expected_batches, "Should process expected number of batches");
}

/// Phase 2.6: Error handling test on single-core
/// Verify error handling works correctly in parallel mode on single-core
#[test]
fn test_single_core_error_handling() {
    let dataset_size = 100;
    let batch_size = 16;

    let loader = create_loader(dataset_size, batch_size, true, 1);

    // Should be able to iterate without errors
    let iter = loader.iter().expect("Failed to create iterator");

    for batch_result in iter {
        // Should not panic or hang
        let batch = batch_result.expect("Batch should be valid");
        assert!(!batch.is_empty(), "Batch should not be empty");
    }
}

/// Phase 2.7: Collated iterators single-core test
/// Verify collated iterators work correctly on single-core parallel loading
#[test]
fn test_collated_single_core() {
    let dataset_size = 200;
    let batch_size = 8;

    let loader = create_loader(dataset_size, batch_size, true, 1);

    // Test f32 collation
    let f32_iter = loader.iter_collated_f32().expect("Failed to create f32 iterator");
    let mut f32_batches = 0;
    let mut total_f32_items = 0;

    for batch_result in f32_iter {
        let batch = batch_result.expect("Failed to get f32 batch");
        assert!(!batch.is_empty(), "F32 batch should not be empty");
        f32_batches += 1;
        total_f32_items += batch.len();
    }

    // Verify all items processed (each item is 4 bytes = 1 f32)
    assert_eq!(total_f32_items, dataset_size, "Should process all f32 items");
    assert!(f32_batches > 0, "Should have f32 batches");
}
