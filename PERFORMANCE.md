# Performance Optimization Guide

## CPU Utilization Improvements

The nix-archiver now includes several optimizations for better CPU utilization during indexing:

### 1. Increased Batch Size
- **Default**: 500 commits per batch (increased from 100)
- **Impact**: Better work distribution across CPU cores
- **Trade-off**: Higher memory usage, but significantly better throughput

### 2. Reduced Database Flush Frequency
- **Default**: Flush every 5 batches (2500 commits)
- **Impact**: Reduces I/O overhead
- **Trade-off**: Slight risk of data loss on crash (reindexing is safe)

### 3. Thread Control
You can now control the number of worker threads:

```bash
# Use more threads than CPU cores (helps with I/O waiting)
./target/release/nix-archiver index --repo ./nixpkgs --from HEAD -j 16

# Use fewer threads for lower memory usage
./target/release/nix-archiver index --repo ./nixpkgs --from HEAD -j 4
```

### 4. Recommended Settings for Maximum Throughput

For systems with fast SSDs and good I/O:
```bash
# Use 1.5-2x the number of CPU cores
# Example for 8-core CPU:
./target/release/nix-archiver index --repo ./nixpkgs --from HEAD -j 12
```

For systems with slower storage:
```bash
# Use number of CPU cores or slightly more
# Example for 8-core CPU:
./target/release/nix-archiver index --repo ./nixpkgs --from HEAD -j 8
```

## Understanding CPU Utilization

If you see ~30% CPU usage per core, it typically means:
- **I/O bound**: Threads are waiting for disk reads (Git repository access)
- **Lock contention**: Minimal in current implementation (Sled handles concurrent writes well)

The optimizations above help by:
1. Processing more work in parallel (larger batches)
2. Reducing I/O operations (less frequent flushes)
3. Allowing more threads to compensate for I/O wait time

## Monitoring Performance

Enable debug logging to see detailed performance metrics:
```bash
RUST_LOG=debug ./target/release/nix-archiver index --repo ./nixpkgs --from HEAD
```

Look for:
- "Database flushed after N batches" - should appear less frequently
- "Progress: X commits processed" - should show larger numbers between flushes

## Future Improvements

Potential optimizations not yet implemented:
- Pre-loading commit metadata to reduce Git I/O
- Batch database writes within each thread
- Memory-mapped file access for repository data
- Async I/O for repository access
