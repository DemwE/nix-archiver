# Enhanced Logging Examples

## Overview

The nix-archiver now includes comprehensive logging with detailed progress tracking, performance metrics, and statistics.

## Log Levels

### INFO (Default)
Standard operational information with progress tracking and final statistics.

```bash
RUST_LOG=info ./target/release/nix-archiver index --repo ./nixpkgs --from HEAD --max-commits 1000
```

### DEBUG
Detailed debugging information including database performance, thread utilization, and timing details.

```bash
RUST_LOG=debug ./target/release/nix-archiver index --repo ./nixpkgs --from HEAD --max-commits 1000
```

### WARN/ERROR
Only warnings and errors (useful for production).

```bash
RUST_LOG=warn ./target/release/nix-archiver index --repo ./nixpkgs --from HEAD
```

## Example Output

### Startup Logs (INFO)

```
[INFO] Starting indexing of repository at "./nixpkgs"
[INFO] Using 24 threads for parallel processing
[INFO] Max commits: 1000
[INFO] From commit: a761838c2f3a (2026-02-23 22:24:15)
[INFO] ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

### Progress Logs (INFO)

Every batch of 500 commits:

```
[INFO] ⚡ Batch #1 | Commits: 500/1,000 (50%) | Packages: 1,234 inserted (2,456 found) | Speed: 95.3 commits/s | ETA: 5.2s
[INFO] ⚡ Batch #2 | Commits: 1,000/1,000 (100%) | Packages: 2,567 inserted (4,890 found) | Speed: 98.1 commits/s | ETA: 0.0s
```

### Final Statistics (INFO)

```
[INFO] ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
[INFO] ✅ Indexing completed!
[INFO] 📊 Final Statistics:
[INFO]    • Total time:        10.5s
[INFO]    • Commits processed: 1,000 (1,000 new, 0 skipped)
[INFO]    • Packages found:    4,890
[INFO]    • Packages inserted: 2,567 (2,323 duplicates filtered)
[INFO]    • Average speed:     95.2 commits/s, 244.5 packages/s
[INFO]    • Errors:            0
```

### Debug Logs (DEBUG)

Additional information when `RUST_LOG=debug`:

```
[DEBUG] Database flushed after 5 batches (0.32s flush time)
[DEBUG] Database flushed after 10 batches (0.28s flush time)
```

### Warning Logs (WARN)

```
[WARN] Skipping invalid version in nodejs: "v${lib.head..." (contains Nix code)
[WARN] Failed to insert package nodejs:14.17.0: database error
```

### Error Logs (ERROR)

```
[ERROR] Failed to process commit abc123def: permission denied
[ERROR]   └─ Context: file path pkgs/development/tools/cmake/default.nix
```

## Metrics Explanation

### Progress Information

- **Commits**: `X/Y (Z%)` - Current commits processed out of total, with percentage
- **Packages inserted**: Number of packages successfully written to database
- **Packages found**: Total packages discovered (before deduplication)
- **Speed**: commits/s - Current processing rate
- **ETA**: Estimated time to completion (calculated from current speed)

### Final Statistics

- **Total time**: Complete duration of indexing operation
- **Commits processed**: Total commits analyzed (new + skipped)
- **Packages found**: All packages discovered in .nix files
- **Packages inserted**: Unique packages written (after deduplication by timestamp)
- **Duplicates filtered**: `found - inserted` = packages skipped due to older timestamp
- **Average speed**: Overall throughput (commits/s and packages/s)
- **Errors**: Count and percentage of failed operations

## Number Formatting

Large numbers are formatted with thousand separators for readability:
- `1234` → `1,234`
- `1234567` → `1,234,567`

## Time Formatting

Durations are formatted in human-readable format:
- Under 1 minute: `10.5s`
- 1-60 minutes: `5m 23s`
- Over 1 hour: `2h 15m 30s`

## Usage Examples

### Standard indexing with progress

```bash
./target/release/nix-archiver index --repo ./nixpkgs --from HEAD --max-commits 5000
```

### High-performance mode (more threads)

```bash
./target/release/nix-archiver index --repo ./nixpkgs --from HEAD -j 36
```

### Debug mode for troubleshooting

```bash
RUST_LOG=debug ./target/release/nix-archiver index --repo ./nixpkgs --from HEAD --max-commits 100
```

### Quiet mode (errors only)

```bash
RUST_LOG=error ./target/release/nix-archiver index --repo ./nixpkgs --from HEAD
```

## Log Location

By default, logs are written to stderr. You can redirect them:

```bash
# Save logs to file
./target/release/nix-archiver index --repo ./nixpkgs --from HEAD 2> indexing.log

# Both stdout and stderr to file
./target/release/nix-archiver index --repo ./nixpkgs --from HEAD &> indexing.log

# Display progress and save logs
./target/release/nix-archiver index --repo ./nixpkgs --from HEAD 2>&1 | tee indexing.log
```

## Performance Tips

1. **Use INFO level** for production (default) - provides good balance of information vs performance
2. **Use DEBUG level** sparingly - generates more logs, slight performance impact
3. **Monitor progress** - ETA helps estimate completion time for large repositories
4. **Watch for errors** - non-zero error count indicates issues that need investigation
5. **Check duplication rate** - high duplicate filtering is normal (older commits have duplicate packages)
