# Quick Start Guide: CodeQL Dependency Analysis

## Prerequisites

1. **CodeQL CLI** - Required for analyzing C++ code
2. **Python 3** - For processing results
3. **Firebase C++ SDK** - Already cloned ✓

## Installation Steps

### 1. Install CodeQL CLI

```bash
# For macOS
cd ~/Downloads
curl -LO https://github.com/github/codeql-cli-binaries/releases/latest/download/codeql-osx64.zip
unzip codeql-osx64.zip
sudo mv codeql /usr/local/

# Add to PATH (add this to ~/.zshrc)
export PATH="/usr/local/codeql:$PATH"

# Verify installation
codeql --version
```

### 2. Run Analysis

```bash
cd /Users/chanwooahn/Documents/dev/rust/firebase-rust-sdk

# Run the complete analysis pipeline
./run_analysis.sh
```

This script will:
1. Create a CodeQL database from the C++ SDK (~10-30 minutes)
2. Run 7 custom queries to extract dependencies (~5 minutes)
3. Build and analyze the dependency graph (~1 minute)
4. Generate implementation plan and reports

## Expected Output

```
firebase-rust-sdk/
├── codeql-db/                           # CodeQL database (generated)
├── codeql_results/                      # Query results (generated)
│   ├── class_hierarchy.csv
│   ├── method_dependencies.csv
│   ├── type_dependencies.csv
│   ├── return_type_dependencies.csv
│   ├── field_dependencies.csv
│   ├── public_api_methods.csv
│   └── include_dependencies.csv
└── analysis_output/                     # Final analysis (generated)
    ├── implementation_plan.json         # Complete implementation order
    ├── SUMMARY.txt                      # Human-readable summary
    └── api_reports/                     # Per-API dependency details
        ├── firebase_auth_Auth_GetAuth.json
        ├── firebase_auth_Auth_SignInAnonymously.json
        └── ... (one file per public API method)
```

## Understanding the Results

### implementation_plan.json

Contains the complete dependency graph analysis:

```json
{
  "total_nodes": 450,
  "total_edges": 1823,
  "leaf_nodes": ["std::string", "int64_t", ...],
  "implementation_order": [...],
  "implementation_layers": [
    ["std::string", "bool", "int64_t"],
    ["firebase::Timestamp", "firebase::Variant"],
    ...
  ],
  "statistics": {
    "num_layers": 15,
    "avg_layer_size": 30.0,
    "max_dependencies": 45
  }
}
```

**Key fields:**
- `leaf_nodes`: Start implementation here (no dependencies)
- `implementation_order`: Complete list in dependency order
- `implementation_layers`: Grouped by dependency level

### Per-API Reports (api_reports/*.json)

Each public API method gets a detailed report:

```json
{
  "api": "firebase::auth::Auth::SignInAnonymously",
  "metadata": {
    "type": "public_api",
    "class": "firebase::auth::Auth",
    "is_static": false
  },
  "direct_dependencies": [
    "firebase::Future<firebase::auth::AuthResult>",
    "firebase::auth::AuthResult"
  ],
  "transitive_dependencies": [
    "firebase::Future",
    "firebase::auth::AuthResult",
    "firebase::auth::User",
    "std::string",
    ...
  ],
  "dependency_count": 12,
  "implementation_order": [...]
}
```

### SUMMARY.txt

Human-readable overview with:
- Total components and dependencies
- Number of implementation layers
- List of leaf nodes (starting points)
- Preview of implementation layers

## Next Steps After Analysis

1. **Review SUMMARY.txt** to understand scope
2. **Check leaf nodes** - these are your starting points
3. **Review per-API reports** for specific APIs you want to implement
4. **Follow implementation_order** when porting to Rust

## Manual Analysis (If Needed)

If the automated script fails, you can run steps manually:

```bash
# 1. Create database
codeql database create codeql-db \
  --language=cpp \
  --source-root=firebase-cpp-sdk \
  --command=""

# 2. Run a single query
codeql query run \
  --database=codeql-db \
  --output=results.bqrs \
  codeql_queries/public_api_methods.ql

# 3. Convert to CSV
codeql bqrs decode \
  --format=csv \
  --output=results.csv \
  results.bqrs

# 4. Run analysis
python3 analyze_dependencies.py
```

## Troubleshooting

### "CodeQL not found"
- Install CodeQL CLI (see step 1)
- Add to PATH

### "Database creation failed"
- Check if you have enough disk space (~5GB needed)
- Try with `--overwrite` flag

### "No results found"
- Check if SDK path is correct
- Verify queries are in `codeql_queries/`
- Check `codeql_create.log` for errors

### "Python dependencies missing"
- Script uses only standard library
- Requires Python 3.7+
- Run: `python3 --version`

## Time Estimates

- CodeQL database creation: 10-30 minutes (one-time)
- Running queries: 5-10 minutes
- Graph analysis: 1-2 minutes
- **Total first run: ~20-40 minutes**
- **Subsequent runs: ~5-10 minutes** (reuse database)

## Tips

- Database creation is the slowest part - only do once
- Queries can be re-run quickly if you modify them
- Add `--threads=4` to CodeQL commands to speed up
- Large result files (~10-100MB) are normal

## Getting Help

If you encounter issues:
1. Check `codeql_create.log` for database creation errors
2. Check individual CSV files in `codeql_results/`
3. Review CodeQL documentation: https://codeql.github.com/docs/
