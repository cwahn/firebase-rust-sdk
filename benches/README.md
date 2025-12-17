# Firebase Rust SDK Benchmarks

Performance benchmarks for Auth and Firestore operations under various concurrency levels.

## Prerequisites

1. Firebase project with Auth and Firestore enabled
2. Test user credentials
3. Environment variables in `.env` file:
   ```
   FIREBASE_API_KEY=your-api-key
   FIREBASE_PROJECT_ID=your-project-id
   TEST_USER_EMAIL=test@example.com
   TEST_USER_PASSWORD=test-password
   ```

## Benchmark Categories

### 1. Authentication
- **sign_in**: Email/password authentication
- **token_refresh**: ID token refresh operations

### 2. CRUD Operations (Auth time excluded)
- **get**: Document retrieval
- **set**: Document creation/overwrite
- **update**: Document field updates
- **delete**: Document deletion

### 3. Query Operations (Auth time excluded)
- **simple_filter**: Queries with single field filter

### 4. Listen Operations (Auth time excluded)
- **document**: Real-time document listeners
- **query**: Real-time query listeners

## Concurrency Levels

Tests run with: **1, 2, 4, 8, 16, 32, 64, 128, 256** concurrent operations

*(Listener benchmarks limited to 1-32 for resource management)*

## Running Benchmarks

### All Benchmarks (Single Core)
```bash
RAYON_NUM_THREADS=1 cargo bench --bench firestore_bench
```

### All Benchmarks (Dual Core)
```bash
RAYON_NUM_THREADS=2 cargo bench --bench firestore_bench
```

### All Benchmarks (Quad Core)
```bash
RAYON_NUM_THREADS=4 cargo bench --bench firestore_bench
```

### Specific Category
```bash
# Auth benchmarks only
cargo bench --bench firestore_bench auth

# CRUD benchmarks only
cargo bench --bench firestore_bench crud

# Query benchmarks only
cargo bench --bench firestore_bench query

# Listen benchmarks only
cargo bench --bench firestore_bench listen
```

### Specific Operation
```bash
# GET operations at all concurrency levels
cargo bench --bench firestore_bench -- crud/get

# SET operations at 32 concurrency
cargo bench --bench firestore_bench -- crud/set/32

# Document listeners
cargo bench --bench firestore_bench -- listen/document
```

## Output

Criterion generates HTML reports in `target/criterion/`:

```
target/criterion/
├── auth/
│   ├── sign_in/
│   │   ├── 1/
│   │   ├── 2/
│   │   ├── 4/
│   │   └── ...
│   └── token_refresh/
├── crud/
│   ├── get/
│   ├── set/
│   ├── update/
│   └── delete/
├── query/
│   └── simple_filter/
└── listen/
    ├── document/
    └── query/
```

Open `target/criterion/report/index.html` in a browser for interactive results.

## Metrics

For each benchmark, Criterion provides:

- **Mean**: Average latency
- **Median**: 50th percentile latency
- **Std Dev**: Standard deviation
- **Min/Max**: Best and worst case latency
- **Throughput**: Operations per second
- **Percentiles**: P75, P90, P95, P99

## Example Output

```
auth/sign_in/1          time:   [850.23 ms 862.45 ms 875.67 ms]
                        thrpt:  [1.1414 elem/s 1.1597 elem/s 1.1759 elem/s]

auth/sign_in/32         time:   [2.1234 s 2.1567 s 2.1892 s]
                        thrpt:  [14.618 elem/s 14.841 elem/s 15.067 elem/s]

crud/get/1              time:   [45.234 ms 46.123 ms 47.012 ms]
                        thrpt:  [21.269 elem/s 21.683 elem/s 22.098 elem/s]

crud/get/256            time:   [1.2345 s 1.2567 s 1.2789 s]
                        thrpt:  [200.12 elem/s 203.67 elem/s 207.22 elem/s]
```

## Core Comparison

To compare performance across different core counts:

```bash
# Single core baseline
RAYON_NUM_THREADS=1 cargo bench --bench firestore_bench -- crud/get > results_1core.txt

# Dual core
RAYON_NUM_THREADS=2 cargo bench --bench firestore_bench -- crud/get > results_2core.txt

# Quad core
RAYON_NUM_THREADS=4 cargo bench --bench firestore_bench -- crud/get > results_4core.txt

# Compare results
diff results_1core.txt results_2core.txt
```

## Benchmark Design Notes

### Auth Time Exclusion

CRUD and listen benchmarks use a pre-authenticated Firestore instance (`FIRESTORE` static):
- Auth happens once during benchmark initialization
- Only operation latency is measured
- Reflects real-world scenarios where auth is cached

### Document Pre-creation

Some benchmarks pre-create documents to measure pure operation latency:
- **GET**: 256 documents pre-created
- **UPDATE**: 256 documents pre-created
- **QUERY**: 1000 documents pre-created with categories
- **LISTEN**: 256 documents pre-created

### Listener Benchmarks

Listener benchmarks measure time to establish connection and receive first snapshot:
- Limited to 32 max concurrency (resource constraints)
- Shorter measurement time (20s vs 30s)
- Fewer samples (20 vs 100)

## Troubleshooting

### "FIREBASE_API_KEY not set"
Create a `.env` file with your Firebase credentials.

### Out of memory
Reduce maximum concurrency level by modifying `CONCURRENCY_LEVELS` in benchmark code.

### Connection timeouts
Check Firebase quotas and network connectivity. Consider adding delays between high-concurrency tests.

### Inconsistent results
- Close other applications
- Run benchmarks multiple times
- Use `--sample-size` to increase sample count:
  ```bash
  cargo bench --bench firestore_bench -- --sample-size 200
  ```

## Performance Tips

Based on benchmark results, you can optimize:

1. **Connection pooling**: Reuse Firestore instances
2. **Batch operations**: Use WriteBatch for multiple writes
3. **Concurrency limits**: Find optimal concurrent request count
4. **Caching**: Cache frequently accessed documents
5. **Query optimization**: Add indexes for filtered fields

## CI Integration

For automated benchmarking in CI:

```bash
# Generate JSON output
cargo bench --bench firestore_bench -- --output-format json > benchmark_results.json

# Compare with baseline
cargo bench --bench firestore_bench -- --save-baseline main
# ... make changes ...
cargo bench --bench firestore_bench -- --baseline main
```

## Contributing

When adding new benchmarks:

1. Follow existing patterns (use `FIRESTORE` for pre-auth)
2. Pre-create test data when measuring operation latency
3. Use appropriate concurrency limits for resource-intensive operations
4. Document any special setup requirements
5. Add to appropriate criterion_group!
