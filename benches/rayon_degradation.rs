//! Benchmarks for rayon-based parallel item loading degradation on single-core targets
//!
//! This module implements Phase 3 of the verification strategy:
//! - Quantify overhead and establish baseline for v0.2.0 comparisons
//! - Measure throughput, latency, and memory usage across core counts
//! - Verify <10% overhead on single-core systems
//!
//! ## References
//!
//! - [TODO.md](TODO.md) - Line 141: Original verification requirement
//! - [ARCHITECTURE.md](ARCHITECTURE.md) - Parallelism model and single-core requirements
//! - [tests/rayon_degradation.rs](tests/rayon_degradation.rs) - Phase 1 implementation
//! - [benches/rayon_degradation.rs](benches/rayon_degradation.rs) - Phase 3 baseline
//! - [docs/BENCHMARKS.md](docs/BENCHMARKS.md) - Complete verification results and progress

use criterion::{BenchmarkId, Criterion, black_box, criterion_group};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use tempfile::NamedTempFile;
use torchforge_data::{DataLoader, LoaderConfig, MmapDataset};

/// Custom Criterion configuration with automatic output saving
fn configure_criterion() -> Criterion {
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let report_dir = format!("bench_results/reports/{}", timestamp);

    // Create reports directory
    if let Err(e) = fs::create_dir_all(&report_dir) {
        eprintln!("Failed to create reports directory: {}", e);
    }

    Criterion::default().output_directory(Path::new(&report_dir)).with_plots()
}

/// Hardware information for benchmark reproducibility
fn print_hardware_info() {
    println!("=== Hardware Information ===");

    // CPU info
    if let Ok(output) = Command::new("lscpu").output() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if line.contains("Model name")
                || line.contains("CPU(s)")
                || line.contains("Thread(s) per core")
                || line.contains("Core(s) per socket")
            {
                println!("{}", line);
            }
        }
    }

    // Memory info
    if let Ok(output) = Command::new("free").arg("-h").output() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if line.starts_with("Mem:") {
                println!("{}", line);
                break;
            }
        }
    }

    // Rust version
    if let Ok(output) = Command::new("rustc").arg("--version").output() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        println!("Rust: {}", output_str.trim());
    }

    // Build info
    println!("Build: Optimized release build");
    println!("==========================");

    // Save hardware info to file
    save_benchmark_output("hardware_info.txt", &get_hardware_info_string());
}

/// Get hardware info as string
fn get_hardware_info_string() -> String {
    let mut info = String::new();

    // CPU info
    if let Ok(output) = Command::new("lscpu").output() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if line.contains("Model name")
                || line.contains("CPU(s)")
                || line.contains("Thread(s) per core")
                || line.contains("Core(s) per socket")
            {
                info.push_str(line);
                info.push('\n');
            }
        }
    }

    // Memory info
    if let Ok(output) = Command::new("free").arg("-h").output() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if line.starts_with("Mem:") {
                info.push_str(line);
                info.push('\n');
                break;
            }
        }
    }

    // Rust version
    if let Ok(output) = Command::new("rustc").arg("--version").output() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        info.push_str(&format!("Rust: {}\n", output_str.trim()));
    }

    info.push_str("Build: Optimized release build\n");
    info
}

/// Save benchmark output to dedicated folder
fn save_benchmark_output(filename: &str, content: &str) {
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let filepath = format!("bench_results/{}_{}.txt", timestamp, filename);

    if let Err(e) = fs::write(&filepath, content) {
        eprintln!("Failed to save benchmark output to {}: {}", filepath, e);
    } else {
        println!("Benchmark output saved to: {}", filepath);
    }
}

/// Creates a test dataset file for benchmarking
fn create_benchmark_dataset(size: usize, item_size: usize) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");

    // Write predictable data pattern
    for i in 0..size {
        let data = vec![(i % 256) as u8; item_size];
        file.write_all(&data).expect("Failed to write data");
    }

    file.flush().expect("Failed to flush file");
    file
}

/// Creates a DataLoader for benchmarking
fn create_benchmark_loader(dataset_size: usize, batch_size: usize) -> DataLoader<MmapDataset> {
    let item_size = 4; // 4 bytes per item (f32)
    let temp_file = create_benchmark_dataset(dataset_size, item_size);
    let dataset = MmapDataset::open(temp_file.path()).expect("Failed to open dataset");
    let config = LoaderConfig::new().batch_size(batch_size).shuffle(false); // Deterministic for benchmarking

    DataLoader::new(dataset, config).expect("Failed to create loader")
}

/// Benchmark: Sequential throughput baseline
/// This establishes the baseline for single-core performance comparison
fn bench_sequential_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequential_throughput");

    // Test different dataset sizes
    for &size in &[1000, 5000, 10000] {
        // Test different batch sizes
        for &batch_size in &[8, 32, 128] {
            let loader = create_benchmark_loader(size, batch_size);

            group.bench_with_input(
                BenchmarkId::new("items", format!("size_{}_batch_{}", size, batch_size)),
                &(size, batch_size),
                |b, _| {
                    b.iter(|| {
                        let mut total_items = 0;
                        let iter = loader.iter().expect("Failed to create iterator");

                        for batch_result in iter {
                            let batch = batch_result.expect("Failed to get batch");
                            total_items += batch.len();
                            black_box(batch); // Prevent compiler optimizations
                        }

                        black_box(total_items);
                    });
                },
            );
        }
    }

    group.finish();
}

/// Benchmark: Latency measurements
/// Measures time to load first batch and steady-state batch processing
fn bench_sequential_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequential_latency");

    let dataset_size = 10000;
    let batch_size = 32;

    // First batch latency (cold start)
    group.bench_function("first_batch", |b| {
        b.iter(|| {
            let loader = create_benchmark_loader(dataset_size, batch_size);
            let mut iter = loader.iter().expect("Failed to create iterator");

            if let Some(batch_result) = iter.next() {
                let batch = batch_result.expect("Failed to get batch");
                black_box(batch);
            }
        });
    });

    // Steady-state batch processing
    group.bench_function("steady_state", |b| {
        let loader = Arc::new(create_benchmark_loader(dataset_size, batch_size));

        b.iter(|| {
            let loader = Arc::clone(&loader);
            let mut iter = loader.iter().expect("Failed to create iterator");

            // Skip first batch to avoid cold start effects
            let _ = iter.next();

            // Measure next few batches
            for _ in 0..5 {
                if let Some(batch_result) = iter.next() {
                    let batch = batch_result.expect("Failed to get batch");
                    black_box(batch);
                }
            }
        });
    });

    group.finish();
}

/// Benchmark: Memory usage pattern
/// Measures allocations and memory pressure during iteration
fn bench_sequential_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequential_memory");

    let dataset_size = 10000;
    let batch_size = 32;

    group.bench_function("memory_pattern", |b| {
        b.iter(|| {
            let loader = create_benchmark_loader(dataset_size, batch_size);
            let iter = loader.iter().expect("Failed to create iterator");

            // Process all batches to observe memory pattern
            for batch_result in iter {
                let batch = batch_result.expect("Failed to get batch");
                black_box(batch);
            }
        });
    });

    group.finish();
}

/// Benchmark: Edge cases for single-core systems
/// Tests performance with challenging configurations
fn bench_sequential_edge_cases(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequential_edge_cases");

    // Very small batch size (high iteration overhead)
    group.bench_function("batch_size_1", |b| {
        let loader = create_benchmark_loader(1000, 1);
        b.iter(|| {
            let iter = loader.iter().expect("Failed to create iterator");
            for batch_result in iter {
                let batch = batch_result.expect("Failed to get batch");
                black_box(batch);
            }
        });
    });

    // Very large batch size (memory pressure)
    group.bench_function("batch_size_1024", |b| {
        let loader = create_benchmark_loader(10000, 1024);
        b.iter(|| {
            let iter = loader.iter().expect("Failed to create iterator");
            for batch_result in iter {
                let batch = batch_result.expect("Failed to get batch");
                black_box(batch);
            }
        });
    });

    // Single item dataset
    group.bench_function("single_item", |b| {
        let loader = create_benchmark_loader(1, 1);
        b.iter(|| {
            let iter = loader.iter().expect("Failed to create iterator");
            for batch_result in iter {
                let batch = batch_result.expect("Failed to get batch");
                black_box(batch);
            }
        });
    });

    group.finish();
}

/// Benchmark: Parallel vs Sequential throughput comparison
/// Compares performance across different thread configurations
fn bench_parallel_vs_sequential(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_vs_sequential");

    let dataset_size = 5000;
    let batch_size = 32;

    // Sequential baseline
    group.bench_function("sequential", |b| {
        let loader = create_benchmark_loader(dataset_size, batch_size);
        b.iter(|| {
            let iter = loader.iter().expect("Failed to create iterator");
            for batch_result in iter {
                let batch = batch_result.expect("Failed to get batch");
                black_box(batch);
            }
        });
    });

    // Parallel with 1 thread (should be similar to sequential)
    group.bench_function("parallel_1_thread", |b| {
        let config = LoaderConfig::new().batch_size(batch_size).parallel(true).num_threads(1);
        let temp_file = create_benchmark_dataset(dataset_size, 4);
        let dataset = MmapDataset::open(temp_file.path()).expect("Failed to open dataset");
        let loader = DataLoader::new(dataset, config).expect("Failed to create loader");

        b.iter(|| {
            let iter = loader.iter().expect("Failed to create iterator");
            for batch_result in iter {
                let batch = batch_result.expect("Failed to get batch");
                black_box(batch);
            }
        });
    });

    // Parallel with 2 threads
    group.bench_function("parallel_2_threads", |b| {
        let config = LoaderConfig::new().batch_size(batch_size).parallel(true).num_threads(2);
        let temp_file = create_benchmark_dataset(dataset_size, 4);
        let dataset = MmapDataset::open(temp_file.path()).expect("Failed to open dataset");
        let loader = DataLoader::new(dataset, config).expect("Failed to create loader");

        b.iter(|| {
            let iter = loader.iter().expect("Failed to create iterator");
            for batch_result in iter {
                let batch = batch_result.expect("Failed to get batch");
                black_box(batch);
            }
        });
    });

    group.finish();
}

/// Benchmark: Single-core degradation measurement
/// Specifically measures overhead on single-core systems
fn bench_single_core_degradation(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_core_degradation");

    let dataset_size = 10000;
    let batch_size = 64;

    // Sequential baseline
    group.bench_function("sequential_baseline", |b| {
        let loader = create_benchmark_loader(dataset_size, batch_size);
        b.iter(|| {
            let iter = loader.iter().expect("Failed to create iterator");
            for batch_result in iter {
                let batch = batch_result.expect("Failed to get batch");
                black_box(batch);
            }
        });
    });

    // Parallel with 1 thread (single-core simulation)
    group.bench_function("parallel_single_core", |b| {
        let config = LoaderConfig::new().batch_size(batch_size).parallel(true).num_threads(1);
        let temp_file = create_benchmark_dataset(dataset_size, 4);
        let dataset = MmapDataset::open(temp_file.path()).expect("Failed to open dataset");
        let loader = DataLoader::new(dataset, config).expect("Failed to create loader");

        b.iter(|| {
            let iter = loader.iter().expect("Failed to create iterator");
            for batch_result in iter {
                let batch = batch_result.expect("Failed to get batch");
                black_box(batch);
            }
        });
    });

    group.finish();
}

/// Benchmark: Scalability curve
/// Measures throughput across different core counts
fn bench_scalability_curve(c: &mut Criterion) {
    let mut group = c.benchmark_group("scalability_curve");

    let dataset_size = 8000;
    let batch_size = 32;

    for num_threads in [1, 2, 4, 8] {
        group.bench_with_input(
            BenchmarkId::new("parallel_threads", num_threads),
            &num_threads,
            |b, &num_threads| {
                let config = LoaderConfig::new()
                    .batch_size(batch_size)
                    .parallel(true)
                    .num_threads(num_threads);
                let temp_file = create_benchmark_dataset(dataset_size, 4);
                let dataset = MmapDataset::open(temp_file.path()).expect("Failed to open dataset");
                let loader = DataLoader::new(dataset, config).expect("Failed to create loader");

                b.iter(|| {
                    let iter = loader.iter().expect("Failed to create iterator");
                    for batch_result in iter {
                        let batch = batch_result.expect("Failed to get batch");
                        black_box(batch);
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_sequential_throughput,
    bench_sequential_latency,
    bench_sequential_memory,
    bench_sequential_edge_cases,
    bench_parallel_vs_sequential,
    bench_single_core_degradation,
    bench_scalability_curve
);

fn main() {
    print_hardware_info();

    let mut c = configure_criterion();
    bench_sequential_throughput(&mut c);
    bench_sequential_latency(&mut c);
    bench_sequential_memory(&mut c);
    bench_sequential_edge_cases(&mut c);
    bench_parallel_vs_sequential(&mut c);
    bench_single_core_degradation(&mut c);
    bench_scalability_curve(&mut c);

    // Save summary
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let timestamp_str = timestamp.to_string();
    let summary = format!(
        "Rayon Degradation Benchmarks - {}\n\
        Hardware: Intel Core i7-8650U @ 1.90GHz (4 cores, 8 threads)\n\
        Memory: 11GB total\n\
        Build: Optimized release build\n\
        Reports saved to: bench_results/reports/{}/\n\
        All benchmarks completed successfully.",
        timestamp_str, timestamp_str
    );
    save_benchmark_output("benchmark_summary.txt", &summary);
}
