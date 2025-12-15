# CodeQL Dependency Analysis Plan

## Objective
Programmatically analyze the Firebase C++ SDK to extract a complete dependency graph for all Auth and Firestore APIs, then compute the union of dependencies to determine implementation order.

## Plan Overview

### Phase 1: CodeQL Setup
1. Install CodeQL CLI
2. Create CodeQL database from C++ SDK
3. Write custom CodeQL queries for dependency extraction

### Phase 2: Dependency Extraction
1. Extract class hierarchies
2. Extract method dependencies (calls, parameter types, return types)
3. Extract type dependencies (fields, template parameters)
4. Extract include dependencies

### Phase 3: Graph Processing
1. Merge all dependency data into unified graph
2. Compute transitive closure of dependencies
3. Perform topological sort to find implementation order
4. Generate implementation plan (leaf → root)

### Phase 4: Output Generation
1. Generate JSON dependency graph
2. Generate ordered implementation list
3. Generate per-API dependency reports

---

## Detailed Implementation

### Phase 1: CodeQL Setup

#### Step 1.1: Install CodeQL CLI

```bash
# Download CodeQL CLI
cd ~/
wget https://github.com/github/codeql-cli-binaries/releases/latest/download/codeql-osx64.zip
unzip codeql-osx64.zip
export PATH="$HOME/codeql:$PATH"

# Verify installation
codeql --version
```

#### Step 1.2: Create CodeQL Database

```bash
cd /Users/chanwooahn/Documents/dev/rust/firebase-rust-sdk/firebase-cpp-sdk

# Create compile_commands.json for the codebase
# Option A: If using CMake
mkdir -p build
cd build
cmake -DCMAKE_EXPORT_COMPILE_COMMANDS=ON ..
cd ..

# Option B: Create database directly (CodeQL will infer build)
codeql database create \
  ../codeql-db \
  --language=cpp \
  --source-root=. \
  --command="echo 'Using autodetection'"

# This creates a database at: /Users/chanwooahn/Documents/dev/rust/firebase-rust-sdk/codeql-db
```

#### Step 1.3: Verify Database

```bash
codeql database info ../codeql-db
```

---

### Phase 2: Custom CodeQL Queries

We need to create queries to extract:

#### Query 1: Class Hierarchy (`class_hierarchy.ql`)

```ql
/**
 * @name Class Hierarchy
 * @description Extract all class inheritance relationships
 * @kind table
 */

import cpp

from Class c, Class base
where c.getABaseClass() = base
select c.getQualifiedName() as derived_class,
       base.getQualifiedName() as base_class
```

#### Query 2: Method Dependencies (`method_dependencies.ql`)

```ql
/**
 * @name Method Dependencies
 * @description Extract all method call dependencies
 * @kind table
 */

import cpp

from MemberFunction caller, Function callee, FunctionCall call
where 
  call.getEnclosingFunction() = caller and
  call.getTarget() = callee and
  (
    caller.getDeclaringType().getQualifiedName().matches("firebase::auth::%") or
    caller.getDeclaringType().getQualifiedName().matches("firebase::firestore::%")
  )
select 
  caller.getDeclaringType().getQualifiedName() as caller_class,
  caller.getName() as caller_method,
  callee.getQualifiedName() as callee_function,
  call.getLocation().toString() as location
```

#### Query 3: Type Dependencies (`type_dependencies.ql`)

```ql
/**
 * @name Type Dependencies
 * @description Extract type usage in methods (parameters, returns, fields)
 * @kind table
 */

import cpp

// Method parameter types
from MemberFunction m, Parameter p, Type t
where 
  p.getFunction() = m and
  t = p.getType().getUnspecifiedType() and
  (
    m.getDeclaringType().getQualifiedName().matches("firebase::auth::%") or
    m.getDeclaringType().getQualifiedName().matches("firebase::firestore::%")
  )
select 
  m.getDeclaringType().getQualifiedName() as class_name,
  m.getName() as method_name,
  "parameter" as dependency_kind,
  t.getQualifiedName() as type_name

// Method return types
from MemberFunction m, Type t
where 
  t = m.getType().getUnspecifiedType() and
  (
    m.getDeclaringType().getQualifiedName().matches("firebase::auth::%") or
    m.getDeclaringType().getQualifiedName().matches("firebase::firestore::%")
  )
select 
  m.getDeclaringType().getQualifiedName() as class_name,
  m.getName() as method_name,
  "return" as dependency_kind,
  t.getQualifiedName() as type_name

// Field types
from Class c, Field f, Type t
where 
  f.getDeclaringType() = c and
  t = f.getType().getUnspecifiedType() and
  (
    c.getQualifiedName().matches("firebase::auth::%") or
    c.getQualifiedName().matches("firebase::firestore::%")
  )
select 
  c.getQualifiedName() as class_name,
  f.getName() as field_name,
  "field" as dependency_kind,
  t.getQualifiedName() as type_name
```

#### Query 4: Public API Methods (`public_api_methods.ql`)

```ql
/**
 * @name Public API Methods
 * @description Extract all public methods from Auth and Firestore classes
 * @kind table
 */

import cpp

from Class c, MemberFunction m
where 
  m.getDeclaringType() = c and
  m.isPublic() and
  (
    c.getQualifiedName().matches("firebase::auth::%") or
    c.getQualifiedName().matches("firebase::firestore::%")
  ) and
  c.getFile().getAbsolutePath().matches("%/include/%")
select 
  c.getQualifiedName() as class_name,
  m.getName() as method_name,
  m.getSignature() as signature,
  m.getType().toString() as return_type,
  m.isStatic() as is_static,
  m.isVirtual() as is_virtual
```

#### Query 5: Include Dependencies (`include_dependencies.ql`)

```ql
/**
 * @name Include Dependencies
 * @description Extract header file dependencies
 * @kind table
 */

import cpp

from Include inc
where 
  inc.getFile().getAbsolutePath().matches("%firebase%") and
  inc.getFile().getAbsolutePath().matches("%/include/%")
select 
  inc.getFile().getAbsolutePath() as including_file,
  inc.getIncludedFile().getAbsolutePath() as included_file
```

---

### Phase 3: Graph Processing Script

Create a Python script to merge and analyze dependencies:

#### `analyze_dependencies.py`

```python
#!/usr/bin/env python3
"""
Dependency Graph Analyzer for Firebase C++ SDK
Processes CodeQL query results and generates implementation order
"""

import json
import csv
from pathlib import Path
from collections import defaultdict, deque
from typing import Dict, Set, List, Tuple
import sys

class DependencyGraph:
    def __init__(self):
        self.nodes: Set[str] = set()
        self.edges: Dict[str, Set[str]] = defaultdict(set)
        self.reverse_edges: Dict[str, Set[str]] = defaultdict(set)
        
    def add_edge(self, from_node: str, to_node: str):
        """Add directed edge: from_node depends on to_node"""
        self.nodes.add(from_node)
        self.nodes.add(to_node)
        self.edges[from_node].add(to_node)
        self.reverse_edges[to_node].add(from_node)
    
    def get_leaves(self) -> Set[str]:
        """Get nodes with no dependencies (leaves)"""
        return {node for node in self.nodes if not self.edges[node]}
    
    def get_roots(self) -> Set[str]:
        """Get nodes nothing depends on"""
        return {node for node in self.nodes if not self.reverse_edges[node]}
    
    def topological_sort(self) -> List[str]:
        """Return topological sort (implementation order)"""
        # Kahn's algorithm
        in_degree = {node: len(self.edges[node]) for node in self.nodes}
        queue = deque([node for node in self.nodes if in_degree[node] == 0])
        result = []
        
        while queue:
            node = queue.popleft()
            result.append(node)
            
            # Reduce in-degree for dependents
            for dependent in self.reverse_edges[node]:
                in_degree[dependent] -= 1
                if in_degree[dependent] == 0:
                    queue.append(dependent)
        
        if len(result) != len(self.nodes):
            # Cycle detected
            remaining = self.nodes - set(result)
            print(f"WARNING: Cycle detected! Remaining nodes: {remaining}", 
                  file=sys.stderr)
        
        return result
    
    def get_transitive_dependencies(self, node: str) -> Set[str]:
        """Get all transitive dependencies of a node"""
        visited = set()
        queue = deque([node])
        
        while queue:
            current = queue.popleft()
            if current in visited:
                continue
            visited.add(current)
            
            for dependency in self.edges.get(current, []):
                if dependency not in visited:
                    queue.append(dependency)
        
        visited.discard(node)  # Don't include the node itself
        return visited

def load_csv_results(filepath: Path) -> List[Dict[str, str]]:
    """Load CodeQL CSV results"""
    results = []
    with open(filepath, 'r', encoding='utf-8') as f:
        reader = csv.DictReader(f)
        for row in reader:
            results.append(row)
    return results

def build_graph_from_queries(results_dir: Path) -> DependencyGraph:
    """Build unified dependency graph from all query results"""
    graph = DependencyGraph()
    
    # Process class hierarchy
    hierarchy_file = results_dir / "class_hierarchy.csv"
    if hierarchy_file.exists():
        print(f"Processing {hierarchy_file}...")
        for row in load_csv_results(hierarchy_file):
            derived = row['derived_class']
            base = row['base_class']
            graph.add_edge(derived, base)
    
    # Process method dependencies
    method_deps_file = results_dir / "method_dependencies.csv"
    if method_deps_file.exists():
        print(f"Processing {method_deps_file}...")
        for row in load_csv_results(method_deps_file):
            caller = f"{row['caller_class']}::{row['caller_method']}"
            callee = row['callee_function']
            graph.add_edge(caller, callee)
    
    # Process type dependencies
    type_deps_file = results_dir / "type_dependencies.csv"
    if type_deps_file.exists():
        print(f"Processing {type_deps_file}...")
        for row in load_csv_results(type_deps_file):
            method = f"{row['class_name']}::{row['method_name']}"
            type_name = row['type_name']
            if type_name and type_name != "void":
                graph.add_edge(method, type_name)
    
    return graph

def generate_implementation_plan(graph: DependencyGraph, 
                                 output_file: Path) -> Dict:
    """Generate implementation order and statistics"""
    order = graph.topological_sort()
    leaves = graph.get_leaves()
    
    # Group by implementation layers
    layers = []
    remaining = set(order)
    
    while remaining:
        current_layer = {
            node for node in remaining 
            if all(dep not in remaining or dep == node 
                   for dep in graph.edges.get(node, []))
        }
        if not current_layer:
            # Remaining nodes have cycles
            current_layer = remaining
            remaining = set()
        else:
            remaining -= current_layer
        layers.append(sorted(current_layer))
    
    plan = {
        "total_nodes": len(graph.nodes),
        "total_edges": sum(len(deps) for deps in graph.edges.values()),
        "leaf_nodes": sorted(leaves),
        "implementation_order": order,
        "implementation_layers": layers,
        "statistics": {
            "num_layers": len(layers),
            "avg_layer_size": len(graph.nodes) / len(layers) if layers else 0,
            "max_dependencies": max(
                (len(graph.get_transitive_dependencies(node)) 
                 for node in graph.nodes), 
                default=0
            )
        }
    }
    
    # Save to JSON
    with open(output_file, 'w') as f:
        json.dump(plan, f, indent=2)
    
    return plan

def generate_per_api_reports(graph: DependencyGraph, 
                              apis: List[str], 
                              output_dir: Path):
    """Generate individual dependency reports for each API"""
    output_dir.mkdir(parents=True, exist_ok=True)
    
    for api in apis:
        if api not in graph.nodes:
            continue
        
        deps = graph.get_transitive_dependencies(api)
        
        report = {
            "api": api,
            "direct_dependencies": sorted(graph.edges.get(api, [])),
            "transitive_dependencies": sorted(deps),
            "dependency_count": len(deps),
            "dependents": sorted(graph.reverse_edges.get(api, []))
        }
        
        output_file = output_dir / f"{api.replace('::', '_').replace('/', '_')}.json"
        with open(output_file, 'w') as f:
            json.dump(report, f, indent=2)

def main():
    """Main analysis pipeline"""
    base_dir = Path(__file__).parent
    results_dir = base_dir / "codeql_results"
    output_dir = base_dir / "analysis_output"
    output_dir.mkdir(parents=True, exist_ok=True)
    
    print("Building dependency graph...")
    graph = build_graph_from_queries(results_dir)
    
    print(f"Graph built: {len(graph.nodes)} nodes, "
          f"{sum(len(deps) for deps in graph.edges.values())} edges")
    
    print("Generating implementation plan...")
    plan = generate_implementation_plan(
        graph, 
        output_dir / "implementation_plan.json"
    )
    
    print(f"Implementation plan generated:")
    print(f"  - Total components: {plan['total_nodes']}")
    print(f"  - Implementation layers: {plan['statistics']['num_layers']}")
    print(f"  - Leaf nodes (start here): {len(plan['leaf_nodes'])}")
    
    # Generate per-API reports for key APIs
    print("Generating per-API reports...")
    public_api_file = results_dir / "public_api_methods.csv"
    if public_api_file.exists():
        apis = []
        for row in load_csv_results(public_api_file):
            api = f"{row['class_name']}::{row['method_name']}"
            apis.append(api)
        
        generate_per_api_reports(
            graph, 
            apis, 
            output_dir / "api_reports"
        )
        print(f"Generated reports for {len(apis)} APIs")
    
    print(f"\nOutput saved to: {output_dir}")
    print(f"  - implementation_plan.json: Full implementation order")
    print(f"  - api_reports/: Per-API dependency details")

if __name__ == "__main__":
    main()
```

---

### Phase 4: Execution Plan

#### Script: `run_analysis.sh`

```bash
#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SDK_DIR="$SCRIPT_DIR/firebase-cpp-sdk"
DB_DIR="$SCRIPT_DIR/codeql-db"
QUERIES_DIR="$SCRIPT_DIR/codeql_queries"
RESULTS_DIR="$SCRIPT_DIR/codeql_results"

echo "=== Firebase C++ SDK Dependency Analysis ==="
echo ""

# Step 1: Create CodeQL queries directory
echo "[1/5] Creating CodeQL queries..."
mkdir -p "$QUERIES_DIR"

# (Queries will be created as separate files)

# Step 2: Create CodeQL database
echo "[2/5] Creating CodeQL database..."
if [ ! -d "$DB_DIR" ]; then
    codeql database create "$DB_DIR" \
        --language=cpp \
        --source-root="$SDK_DIR" \
        --command="echo 'Using build autodetection'"
    echo "Database created at: $DB_DIR"
else
    echo "Database already exists at: $DB_DIR"
fi

# Step 3: Run CodeQL queries
echo "[3/5] Running CodeQL queries..."
mkdir -p "$RESULTS_DIR"

for query in "$QUERIES_DIR"/*.ql; do
    query_name=$(basename "$query" .ql)
    echo "  Running $query_name..."
    
    codeql query run \
        --database="$DB_DIR" \
        --output="$RESULTS_DIR/${query_name}.bqrs" \
        "$query"
    
    # Convert to CSV
    codeql bqrs decode \
        --format=csv \
        --output="$RESULTS_DIR/${query_name}.csv" \
        "$RESULTS_DIR/${query_name}.bqrs"
done

echo "Query results saved to: $RESULTS_DIR"

# Step 4: Run dependency analysis
echo "[4/5] Analyzing dependencies..."
python3 "$SCRIPT_DIR/analyze_dependencies.py"

# Step 5: Generate summary report
echo "[5/5] Generating summary..."
python3 - << 'EOF'
import json
from pathlib import Path

plan_file = Path("analysis_output/implementation_plan.json")
with open(plan_file) as f:
    plan = json.load(f)

print("\n" + "="*60)
print("DEPENDENCY ANALYSIS SUMMARY")
print("="*60)
print(f"Total Components: {plan['total_nodes']}")
print(f"Total Dependencies: {plan['total_edges']}")
print(f"Implementation Layers: {plan['statistics']['num_layers']}")
print(f"Average Layer Size: {plan['statistics']['avg_layer_size']:.1f}")
print(f"\nLeaf Nodes (Start Implementation Here): {len(plan['leaf_nodes'])}")
print("\nFirst 10 leaf nodes:")
for leaf in plan['leaf_nodes'][:10]:
    print(f"  - {leaf}")
print("\n" + "="*60)
EOF

echo ""
echo "Analysis complete! Results in: $SCRIPT_DIR/analysis_output"
```

---

## File Structure

```
firebase-rust-sdk/
├── CODEQL_ANALYSIS_PLAN.md          (this file)
├── AVAILABLE_APIS.md                (API list)
├── run_analysis.sh                   (main execution script)
├── analyze_dependencies.py           (graph processor)
├── codeql_queries/                   (CodeQL queries)
│   ├── class_hierarchy.ql
│   ├── method_dependencies.ql
│   ├── type_dependencies.ql
│   ├── public_api_methods.ql
│   └── include_dependencies.ql
├── codeql-db/                        (generated CodeQL database)
├── codeql_results/                   (query results - CSV)
│   ├── class_hierarchy.csv
│   ├── method_dependencies.csv
│   └── ...
├── analysis_output/                  (processed results)
│   ├── implementation_plan.json      (full implementation order)
│   └── api_reports/                  (per-API dependencies)
│       ├── firebase_auth_Auth_GetAuth.json
│       └── ...
└── firebase-cpp-sdk/                 (cloned C++ SDK)
```

---

## Expected Output

### `implementation_plan.json`
```json
{
  "total_nodes": 450,
  "total_edges": 1823,
  "leaf_nodes": ["std::string", "int64_t", "bool", ...],
  "implementation_order": ["std::string", "firebase::Timestamp", ...],
  "implementation_layers": [
    ["std::string", "int64_t", "bool"],
    ["firebase::Timestamp", "firebase::Variant"],
    ["firebase::firestore::GeoPoint"],
    ...
  ],
  "statistics": {
    "num_layers": 15,
    "avg_layer_size": 30.0,
    "max_dependencies": 45
  }
}
```

### Per-API Report (`api_reports/firebase_auth_Auth_SignIn.json`)
```json
{
  "api": "firebase::auth::Auth::SignIn",
  "direct_dependencies": [
    "firebase::auth::Credential",
    "firebase::Future<firebase::auth::User>"
  ],
  "transitive_dependencies": [
    "firebase::auth::Credential",
    "firebase::Future",
    "firebase::auth::User",
    "std::string",
    ...
  ],
  "dependency_count": 12,
  "dependents": [
    "MyApp::signInUser"
  ]
}
```

---

## Next Steps

1. **Review this plan** - Ensure approach makes sense
2. **Execute Phase 1** - Install CodeQL and create database
3. **Execute Phase 2** - Run queries and extract data
4. **Execute Phase 3** - Process graph and generate implementation order
5. **Begin Implementation** - Start coding from leaf nodes

## Notes

- CodeQL analysis may take 10-30 minutes for large codebase
- Results will be cached for future runs
- Graph may have cycles (e.g., mutual dependencies) - these will be flagged
- Some platform-specific code may not be fully captured
- Manual review of critical dependencies is recommended
