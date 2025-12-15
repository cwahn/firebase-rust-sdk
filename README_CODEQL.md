# CodeQL Dependency Analysis - Complete Setup

## Overview

This setup uses CodeQL to programmatically analyze the Firebase C++ SDK and extract a complete dependency graph for all Auth and Firestore APIs. The analysis will identify the implementation order, starting from leaf dependencies.

## Project Structure

```
firebase-rust-sdk/
├── CODEQL_ANALYSIS_PLAN.md          # Detailed plan (this file)
├── AVAILABLE_APIS.md                 # List of all APIs to implement
├── QUICKSTART.md                     # Quick installation guide
├── README_CODEQL.md                  # This overview
│
├── run_analysis.sh                   # Main execution script ⭐
├── analyze_dependencies.py           # Graph processing script
│
├── codeql_queries/                   # CodeQL queries (7 files)
│   ├── class_hierarchy.ql            # Extract class inheritance
│   ├── method_dependencies.ql        # Extract function calls
│   ├── type_dependencies.ql          # Extract parameter types
│   ├── return_type_dependencies.ql   # Extract return types
│   ├── field_dependencies.ql         # Extract field types
│   ├── public_api_methods.ql         # Extract public APIs
│   └── include_dependencies.ql       # Extract header includes
│
├── firebase-cpp-sdk/                 # Cloned C++ SDK ✓
│
└── [Generated directories]
    ├── codeql-db/                    # CodeQL database
    ├── codeql_results/               # Query results (CSV)
    └── analysis_output/              # Final analysis
        ├── implementation_plan.json
        ├── SUMMARY.txt
        └── api_reports/
```

## Quick Start

### Prerequisites

- **CodeQL CLI** (not installed yet)
- **Python 3.7+** (already have)
- **Firebase C++ SDK** (already cloned ✓)

### Installation

```bash
# 1. Install CodeQL CLI (one-time setup)
cd ~/Downloads
curl -LO https://github.com/github/codeql-cli-binaries/releases/latest/download/codeql-osx64.zip
unzip codeql-osx64.zip
sudo mv codeql /usr/local/
export PATH="/usr/local/codeql:$PATH"

# Verify
codeql --version

# 2. Run analysis (from project root)
cd /Users/chanwooahn/Documents/dev/rust/firebase-rust-sdk
./run_analysis.sh
```

### That's it! 🎉

The script will:
1. Create CodeQL database (~15 mins)
2. Run 7 queries (~5 mins)
3. Analyze dependencies (~1 min)
4. Generate implementation plan

## What Gets Generated

### 1. implementation_plan.json
Complete dependency graph with implementation order:
- Total nodes and edges
- Leaf nodes (start here!)
- Complete implementation order
- Grouped by layers
- Statistics

### 2. api_reports/*.json
One file per public API with:
- Direct dependencies
- Transitive dependencies (full tree)
- Implementation order for that API
- Dependent APIs

### 3. SUMMARY.txt
Human-readable overview:
- Statistics
- Top leaf nodes
- Layer breakdown

## How It Works

### Step 1: CodeQL Database Creation
CodeQL analyzes the C++ source code and creates a queryable database of:
- All classes, methods, functions
- Type information
- Call graphs
- Include dependencies

### Step 2: Query Execution
7 custom queries extract:
1. **Class hierarchy** - Who inherits from whom
2. **Method calls** - Which methods call which
3. **Parameter types** - What types methods use
4. **Return types** - What types methods return
5. **Field types** - What types classes contain
6. **Public APIs** - All public methods to implement
7. **Include deps** - Header file dependencies

### Step 3: Graph Construction
Python script merges all query results into unified dependency graph:
- Nodes = Classes, methods, types
- Edges = Dependencies (calls, uses, inherits, returns)

### Step 4: Topological Sort
Kahn's algorithm computes implementation order:
1. Find leaf nodes (no dependencies)
2. Remove leaves, find new leaves
3. Repeat until complete
4. Result = bottom-up implementation order

## Example Output

### Leaf Nodes (Start Here)
```
1. std::string
2. bool
3. int64_t
4. double
5. firebase::Timestamp
6. firebase::Variant
...
```

### Implementation Layers
```
Layer 1 (15 components):
  - std::string
  - bool
  - int64_t
  
Layer 2 (23 components):
  - firebase::Timestamp
  - firebase::Variant
  - firebase::firestore::GeoPoint
  
Layer 3 (31 components):
  - firebase::firestore::FieldValue
  - firebase::auth::Credential
  ...
```

### Per-API Report Example
```json
{
  "api": "firebase::auth::Auth::SignInAnonymously",
  "direct_dependencies": [
    "firebase::Future<firebase::auth::AuthResult>"
  ],
  "transitive_dependencies": [
    "firebase::Future",
    "firebase::auth::AuthResult",
    "firebase::auth::User",
    "firebase::UserInfoInterface",
    "std::string",
    "std::vector",
    ...
  ],
  "dependency_count": 12,
  "implementation_order": [
    "std::string",
    "firebase::UserInfoInterface",
    "firebase::auth::User",
    "firebase::auth::AuthResult",
    "firebase::Future",
    "firebase::auth::Auth::SignInAnonymously"
  ]
}
```

## Using the Results

### For Complete Implementation
Use `implementation_plan.json`:
```python
import json

plan = json.load(open('analysis_output/implementation_plan.json'))

# Start with leaves
for leaf in plan['leaf_nodes']:
    print(f"Implement: {leaf}")

# Or go layer by layer
for i, layer in enumerate(plan['implementation_layers']):
    print(f"Layer {i}: {len(layer)} components")
    for component in layer:
        print(f"  - {component}")
```

### For Individual APIs
Use per-API reports:
```python
# To implement Auth::SignInAnonymously, first implement its dependencies
report = json.load(open('analysis_output/api_reports/firebase_auth_Auth_SignInAnonymously.json'))

print("Implementation order for SignInAnonymously:")
for dep in report['implementation_order']:
    print(f"  {dep}")
```

## Advanced Usage

### Re-run Analysis Only
If you've already created the database:
```bash
# Skip database creation, just run queries
codeql query run --database=codeql-db codeql_queries/*.ql
python3 analyze_dependencies.py
```

### Add Custom Queries
Create new `.ql` file in `codeql_queries/`:
```ql
/**
 * @name My Custom Query
 * @description What it does
 * @kind table
 */
import cpp

from Class c
where c.getQualifiedName().matches("firebase::%")
select c.getQualifiedName()
```

Run with `./run_analysis.sh`

### Filter Results
Modify `analyze_dependencies.py` to filter:
```python
# Only include auth APIs
if 'auth' in api_name:
    graph.add_node(api_name)
```

## Performance

- **First run**: 20-40 minutes
  - Database: 15-30 mins
  - Queries: 5-10 mins
  - Analysis: 1 min

- **Subsequent runs**: 5-10 minutes
  - Reuse database
  - Only run queries + analysis

## Optimization Tips

1. **Parallel queries**: Run multiple queries at once
2. **Incremental**: Database can be updated incrementally
3. **Caching**: Results are cached in CSV files
4. **Filtering**: Focus on specific modules

## Troubleshooting

### Database Creation Fails
```bash
# Check logs
cat codeql_create.log

# Try with more verbose output
codeql database create codeql-db \
  --language=cpp \
  --source-root=firebase-cpp-sdk \
  --command="" \
  --verbose
```

### Query Returns No Results
```bash
# Test single query
codeql query run \
  --database=codeql-db \
  codeql_queries/public_api_methods.ql

# Check if database is valid
codeql database info codeql-db
```

### Out of Memory
```bash
# Reduce memory usage
codeql database create codeql-db \
  --language=cpp \
  --source-root=firebase-cpp-sdk \
  --command="" \
  --ram=4096  # Limit to 4GB
```

## Next Steps

After running analysis:

1. ✅ Review `SUMMARY.txt`
2. ✅ Check `implementation_plan.json`
3. ✅ Browse `api_reports/`
4. 🚀 Start implementing from leaves!

## Implementation Strategy

### Option 1: Bottom-Up (Recommended)
Start from leaf nodes and work up:
1. Implement primitive types (Timestamp, Variant)
2. Implement data types (FieldValue, Credential)
3. Implement helper classes (Future, Result)
4. Implement main APIs (Auth, Firestore)

### Option 2: Top-Down (By Feature)
Pick an API and implement all its dependencies:
1. Choose target API (e.g., SignInAnonymously)
2. Get its dependency report
3. Implement in order from report
4. Repeat for next API

### Option 3: Layer-by-Layer
Implement one complete layer at a time:
1. All layer 1 components
2. All layer 2 components
3. Continue...

## Resources

- [CodeQL Documentation](https://codeql.github.com/docs/)
- [CodeQL for C++](https://codeql.github.com/docs/codeql-language-guides/codeql-for-cpp/)
- [Firebase C++ SDK](https://github.com/firebase/firebase-cpp-sdk)

## Status

- [x] Setup complete
- [x] Queries written
- [x] Analysis script ready
- [ ] Run analysis
- [ ] Review results
- [ ] Start implementation
