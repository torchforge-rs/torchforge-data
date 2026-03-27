//! Fuzz target for MmapDataset file parsing
//!
//! This fuzz target tests the robustness of MmapDataset when handling
//! various malformed, truncated, and corrupt input files.

#![no_main]

use libfuzzer_sys::fuzz_target;
use std::io::Write;
use tempfile::NamedTempFile;
use torchforge_data::Dataset;

fuzz_target!(|data: &[u8]| {
    // Create a temporary file with the fuzz data
    let mut temp_file = match NamedTempFile::new() {
        Ok(file) => file,
        Err(_) => return, // Skip if we can't create temp file
    };

    // Write fuzz data to the temporary file
    if let Err(_) = temp_file.write_all(data) {
        return; // Skip if write fails
    }

    // Attempt to create MmapDataset from the fuzz data
    let result = torchforge_data::MmapDataset::open(temp_file.path());

    match result {
        Ok(dataset) => {
            // If successful, test basic operations
            let len_result = dataset.len();
            if let Ok(len) = len_result {
                if len > 0 {
                    // Test accessing a few items (but not too many to avoid timeouts)
                    let test_indices = [0, len / 4, len / 2, len - 1];
                    for &index in &test_indices {
                        if index < len {
                            let _ = dataset.get(index);
                        }
                    }
                }
            }
        }
        Err(_) => {
            // Expected for malformed data - this is fine
        }
    }
});
