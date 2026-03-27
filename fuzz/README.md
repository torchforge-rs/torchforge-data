# Fuzzing torchforge-data

This directory contains fuzz targets for testing the robustness of torchforge-data components.

## Prerequisites

Install cargo-fuzz:
```bash
cargo install cargo-fuzz
```

## Running Fuzz Targets

### MmapDataset Fuzzing

The `mmap_fuzz` target tests the robustness of `MmapDataset` when handling various malformed, truncated, and corrupt input files.

```bash
# Run the fuzz target
cargo fuzz run mmap_fuzz

# Run with specific corpus directory
cargo fuzz run mmap_fuzz corpus/

# Run with limited time (e.g., 60 seconds)
cargo fuzz run mmap_fuzz -- -max_total_time=60

# Run with specific seed for reproducibility
cargo fuzz run mmap_fuzz -- -seed=12345
```

## What the Fuzz Target Tests

The `mmap_fuzz` target specifically tests:

1. **Malformed files**: Random binary data that doesn't represent valid data
2. **Truncated files**: Files that are cut off mid-item
3. **Corrupt indices**: Files with invalid item boundaries
4. **Empty files**: Zero-length input
5. **Very large files**: Files that might cause integer overflow
6. **Invalid memory mappings**: Files that can't be properly memory-mapped

## Fuzz Target Behavior

For each input, the fuzz target:

1. Creates a temporary file with the fuzz data
2. Attempts to create an `MmapDataset` from the file
3. If successful, tests basic operations:
   - Getting dataset length
   - Accessing items at various indices (0, 1/4, 1/2, last)
4. Gracefully handles errors (expected for malformed data)

## Corpus Management

The fuzz corpus is stored in `fuzz/corpus/mmap_fuzz/`. You can:

- Add interesting inputs to this directory
- Use existing corpus as starting points for fuzzing
- Share corpus with other developers

## Finding Crashes

If a crash is found, cargo-fuzz will:
1. Save the offending input to `fuzz/artifacts/`
2. Display the crash information
3. Allow you to reproduce the crash with the specific input

## Integration with CI

Consider adding fuzzing to your CI pipeline:
```bash
# Run fuzzing for a limited time in CI
cargo fuzz run mmap_fuzz -- -max_total_time=300
```
