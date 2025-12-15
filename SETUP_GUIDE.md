# Firebase Rust SDK - Setup Guide

This guide walks you through reproducing the CodeQL dependency analysis setup from scratch.

## Prerequisites

- macOS (these instructions are for macOS, adapt for other platforms)
- Homebrew
- Python 3
- Git

## Step 1: Install CodeQL CLI

```bash
brew install codeql
```

Verify installation:
```bash
codeql --version
# Should show: CodeQL command-line toolchain release 2.23.8 or later
```

## Step 2: Clone Required Repositories

### Clone Firebase C++ SDK
```bash
git clone https://github.com/firebase/firebase-cpp-sdk.git
```

### Clone CodeQL Standard Libraries
```bash
git clone https://github.com/github/codeql.git codeql-repo
```

## Step 3: Create CodeQL Database

The Firebase C++ SDK has complex build requirements, so we use `--build-mode=none` to analyze source files directly without compilation:

```bash
./create_db_final.sh
```

This will:
- Create a CodeQL database at `codeql-db/`
- Index ~2083 files (580 source files, 570 headers)
- Take approximately 3-4 minutes

## Step 4: Run CodeQL Queries

Execute all dependency extraction queries:

```bash
./run_analysis.sh
```

This will:
1. Run 7 CodeQL queries to extract different dependency types
2. Convert results from BQRS to CSV format
3. Generate ~10,000 dependency records

The queries extract:
- Class hierarchy (inheritance)
- Method call dependencies
- Type dependencies (parameters)
- Return type dependencies
- Field type dependencies
- Public API methods
- Include dependencies

## Step 5: Generate Dependency Graph

Process the CodeQL results to build a unified dependency graph:

```bash
python3 analyze_dependencies.py
```

This creates:
- `analysis_output/implementation_plan.json` - Full topologically sorted implementation order
- `analysis_output/api_reports/*.json` - Per-API dependency details (399 files)
- `analysis_output/SUMMARY.txt` - Human-readable summary

## Output Structure

```
firebase-rust-sdk/
├── firebase-cpp-sdk/          # Cloned C++ SDK (gitignored)
├── codeql-repo/               # CodeQL standard libraries (gitignored)
├── codeql-db/                 # CodeQL database (gitignored)
├── codeql_queries/            # 7 .ql query files
│   ├── class_hierarchy.ql
│   ├── method_dependencies.ql
│   ├── type_dependencies.ql
│   ├── return_type_dependencies.ql
│   ├── field_dependencies.ql
│   ├── public_api_methods.ql
│   ├── include_dependencies.ql
│   └── qlpack.yml
├── codeql_results/            # Query results (CSV format)
│   ├── class_hierarchy.csv
│   ├── method_dependencies.csv
│   ├── type_dependencies.csv
│   ├── return_type_dependencies.csv
│   ├── field_dependencies.csv
│   ├── public_api_methods.csv
│   └── include_dependencies.csv
├── analysis_output/           # Dependency graph analysis
│   ├── implementation_plan.json
│   ├── SUMMARY.txt
│   └── api_reports/
│       └── [399 JSON files]
├── analyze_dependencies.py    # Graph processing script
├── create_db_final.sh         # Database creation script
├── run_analysis.sh            # Query execution script
├── AVAILABLE_APIS.md          # Catalog of all public APIs
├── CODEQL_ANALYSIS_PLAN.md    # Technical analysis plan
└── README_CODEQL.md           # CodeQL overview
```

## Understanding the Results

### Implementation Plan
The `implementation_plan.json` contains:
- **layers**: 32 implementation layers in topological order
- **leaf_nodes**: 1115 components with no dependencies (start here)
- **cycles**: 13 nodes involved in circular dependencies

### API Reports
Each API has a JSON report showing:
- Direct dependencies (immediate requirements)
- Transitive dependencies (full closure)
- Metadata (class, method, static/virtual flags)

### Summary
The `SUMMARY.txt` provides:
- Total component count (3394 nodes)
- Total dependency count (7734 edges)
- List of leaf nodes (start implementation here)
- Layer-by-layer breakdown

## Next Steps

1. Review `analysis_output/SUMMARY.txt` to understand the dependency structure
2. Start implementing from leaf nodes (Layer 1)
3. Use per-API reports to understand what each API needs
4. Implement layer by layer following topological order

## Troubleshooting

### CodeQL database creation fails
- Ensure you have enough disk space (~500MB for database)
- Check that firebase-cpp-sdk was cloned successfully
- Try removing `codeql-db/` and running `create_db_final.sh` again

### Queries fail with "could not resolve module cpp"
- Ensure `codeql-repo` was cloned
- Check that `codeql_queries/qlpack.yml` exists with proper dependencies

### Python script fails
- Ensure all CSV files exist in `codeql_results/`
- Check that Python 3 is installed and accessible

## Time Estimates

- CodeQL database creation: ~3-4 minutes
- Query execution: ~2 minutes total
- Python analysis: ~1 minute
- **Total setup time: ~6-7 minutes**
