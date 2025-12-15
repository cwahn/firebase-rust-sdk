#!/usr/bin/env python3
"""
Dependency Graph Analyzer for Firebase C++ SDK
Processes CodeQL query results and generates implementation order
"""

import json
import csv
from pathlib import Path
from collections import defaultdict, deque
from typing import Dict, Set, List, Tuple, Optional
import sys

class DependencyGraph:
    """Directed graph for tracking dependencies"""
    
    def __init__(self):
        self.nodes: Set[str] = set()
        self.edges: Dict[str, Set[str]] = defaultdict(set)
        self.reverse_edges: Dict[str, Set[str]] = defaultdict(set)
        self.node_metadata: Dict[str, Dict] = {}
        
    def add_edge(self, from_node: str, to_node: str, metadata: Optional[Dict] = None):
        """Add directed edge: from_node depends on to_node"""
        if not from_node or not to_node or from_node == to_node:
            return
            
        self.nodes.add(from_node)
        self.nodes.add(to_node)
        self.edges[from_node].add(to_node)
        self.reverse_edges[to_node].add(from_node)
        
        if metadata:
            if from_node not in self.node_metadata:
                self.node_metadata[from_node] = {}
            self.node_metadata[from_node].update(metadata)
    
    def add_node(self, node: str, metadata: Optional[Dict] = None):
        """Add a node without edges"""
        self.nodes.add(node)
        if metadata:
            if node not in self.node_metadata:
                self.node_metadata[node] = {}
            self.node_metadata[node].update(metadata)
    
    def get_leaves(self) -> Set[str]:
        """Get nodes with no dependencies (leaves)"""
        return {node for node in self.nodes if not self.edges[node]}
    
    def get_roots(self) -> Set[str]:
        """Get nodes nothing depends on"""
        return {node for node in self.nodes if not self.reverse_edges[node]}
    
    def topological_sort(self) -> List[str]:
        """Return topological sort (implementation order) using Kahn's algorithm"""
        in_degree = {node: len(self.edges[node]) for node in self.nodes}
        queue = deque([node for node in self.nodes if in_degree[node] == 0])
        result = []
        
        while queue:
            node = queue.popleft()
            result.append(node)
            
            for dependent in self.reverse_edges[node]:
                in_degree[dependent] -= 1
                if in_degree[dependent] == 0:
                    queue.append(dependent)
        
        if len(result) != len(self.nodes):
            remaining = self.nodes - set(result)
            print(f"WARNING: Cycle detected! {len(remaining)} nodes in cycles", 
                  file=sys.stderr)
            # Add remaining nodes to end
            result.extend(sorted(remaining))
        
        return result
    
    def get_transitive_dependencies(self, node: str) -> Set[str]:
        """Get all transitive dependencies of a node (BFS)"""
        if node not in self.nodes:
            return set()
            
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
        
        visited.discard(node)
        return visited
    
    def get_implementation_layers(self) -> List[List[str]]:
        """Group nodes into implementation layers"""
        layers = []
        remaining = set(self.nodes)
        
        while remaining:
            # Find all nodes whose dependencies are already implemented
            current_layer = {
                node for node in remaining 
                if all(dep not in remaining for dep in self.edges.get(node, []))
            }
            
            if not current_layer:
                # Remaining nodes have cycles - take nodes with minimum dependencies
                min_deps = min(
                    len(self.edges.get(node, [])) 
                    for node in remaining
                )
                current_layer = {
                    node for node in remaining
                    if len(self.edges.get(node, [])) == min_deps
                }
            
            layers.append(sorted(current_layer))
            remaining -= current_layer
        
        return layers
    
    def get_statistics(self) -> Dict:
        """Calculate graph statistics"""
        if not self.nodes:
            return {
                "num_nodes": 0,
                "num_edges": 0,
                "num_leaves": 0,
                "num_roots": 0
            }
        
        return {
            "num_nodes": len(self.nodes),
            "num_edges": sum(len(deps) for deps in self.edges.values()),
            "num_leaves": len(self.get_leaves()),
            "num_roots": len(self.get_roots()),
            "max_dependencies": max(
                len(self.edges.get(node, [])) 
                for node in self.nodes
            ),
            "max_dependents": max(
                len(self.reverse_edges.get(node, [])) 
                for node in self.nodes
            ),
            "avg_dependencies": sum(
                len(self.edges.get(node, [])) 
                for node in self.nodes
            ) / len(self.nodes)
        }


def load_csv_results(filepath: Path) -> List[Dict[str, str]]:
    """Load CodeQL CSV results"""
    if not filepath.exists():
        print(f"Warning: {filepath} not found", file=sys.stderr)
        return []
    
    results = []
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            reader = csv.DictReader(f)
            for row in reader:
                results.append(row)
        print(f"  Loaded {len(results)} rows from {filepath.name}")
    except Exception as e:
        print(f"Error loading {filepath}: {e}", file=sys.stderr)
    
    return results


def normalize_type(type_str: str) -> str:
    """Normalize type names for consistency"""
    if not type_str:
        return ""
    
    # Remove const, &, *, and whitespace
    normalized = type_str.strip()
    normalized = normalized.replace("const ", "")
    normalized = normalized.replace("&", "")
    normalized = normalized.replace("*", "")
    normalized = normalized.strip()
    
    return normalized


def build_graph_from_queries(results_dir: Path) -> DependencyGraph:
    """Build unified dependency graph from all query results"""
    graph = DependencyGraph()
    
    print("\n=== Building Dependency Graph ===")
    
    # 1. Process public API methods first (these are our targets)
    public_api_file = results_dir / "public_api_methods.csv"
    if public_api_file.exists():
        print(f"\n[1/6] Processing {public_api_file.name}...")
        for row in load_csv_results(public_api_file):
            api_name = row['method_qualified']
            graph.add_node(api_name, {
                'type': 'public_api',
                'class': row['class_name'],
                'method': row['method_name'],
                'is_static': row.get('is_static', 'false') == 'true',
                'is_virtual': row.get('is_virtual', 'false') == 'true'
            })
    
    # 2. Process class hierarchy
    hierarchy_file = results_dir / "class_hierarchy.csv"
    if hierarchy_file.exists():
        print(f"\n[2/6] Processing {hierarchy_file.name}...")
        for row in load_csv_results(hierarchy_file):
            derived = row['derived_class']
            base = row['base_class']
            graph.add_edge(derived, base, {'relationship': 'inherits'})
    
    # 3. Process method dependencies (function calls)
    method_deps_file = results_dir / "method_dependencies.csv"
    if method_deps_file.exists():
        print(f"\n[3/6] Processing {method_deps_file.name}...")
        for row in load_csv_results(method_deps_file):
            caller = row['caller_qualified']
            callee = row['callee_function']
            graph.add_edge(caller, callee, {'relationship': 'calls'})
    
    # 4. Process parameter type dependencies
    type_deps_file = results_dir / "type_dependencies.csv"
    if type_deps_file.exists():
        print(f"\n[4/6] Processing {type_deps_file.name}...")
        for row in load_csv_results(type_deps_file):
            method = row['method_qualified']
            type_name = normalize_type(row.get('type_qualified', ''))
            if type_name and type_name != "void":
                graph.add_edge(method, type_name, {'relationship': 'uses_type'})
    
    # 5. Process return type dependencies
    return_type_file = results_dir / "return_type_dependencies.csv"
    if return_type_file.exists():
        print(f"\n[5/6] Processing {return_type_file.name}...")
        for row in load_csv_results(return_type_file):
            method = row['method_qualified']
            type_name = normalize_type(row.get('type_qualified', ''))
            if type_name and type_name != "void":
                graph.add_edge(method, type_name, {'relationship': 'returns'})
    
    # 6. Process field dependencies
    field_deps_file = results_dir / "field_dependencies.csv"
    if field_deps_file.exists():
        print(f"\n[6/6] Processing {field_deps_file.name}...")
        for row in load_csv_results(field_deps_file):
            class_name = row['class_name']
            type_name = normalize_type(row.get('type_qualified', ''))
            if type_name and type_name != "void":
                graph.add_edge(class_name, type_name, {'relationship': 'has_field'})
    
    return graph


def generate_implementation_plan(graph: DependencyGraph, 
                                 output_file: Path) -> Dict:
    """Generate implementation order and statistics"""
    print("\n=== Generating Implementation Plan ===")
    
    order = graph.topological_sort()
    leaves = graph.get_leaves()
    layers = graph.get_implementation_layers()
    stats = graph.get_statistics()
    
    plan = {
        "total_nodes": len(graph.nodes),
        "total_edges": sum(len(deps) for deps in graph.edges.values()),
        "leaf_nodes": sorted(leaves),
        "implementation_order": order,
        "implementation_layers": layers,
        "statistics": {
            **stats,
            "num_layers": len(layers),
            "avg_layer_size": len(graph.nodes) / len(layers) if layers else 0
        }
    }
    
    # Save to JSON
    with open(output_file, 'w') as f:
        json.dump(plan, f, indent=2)
    
    print(f"Implementation plan saved to: {output_file}")
    
    return plan


def generate_per_api_reports(graph: DependencyGraph, 
                              output_dir: Path):
    """Generate individual dependency reports for each API"""
    print("\n=== Generating Per-API Reports ===")
    
    output_dir.mkdir(parents=True, exist_ok=True)
    
    # Get only public API nodes
    public_apis = [
        node for node in graph.nodes
        if graph.node_metadata.get(node, {}).get('type') == 'public_api'
    ]
    
    print(f"Generating reports for {len(public_apis)} public APIs...")
    
    for api in public_apis:
        deps = graph.get_transitive_dependencies(api)
        
        report = {
            "api": api,
            "metadata": graph.node_metadata.get(api, {}),
            "direct_dependencies": sorted(graph.edges.get(api, [])),
            "transitive_dependencies": sorted(deps),
            "dependency_count": len(deps),
            "direct_dependents": sorted(graph.reverse_edges.get(api, [])),
            "implementation_order": []
        }
        
        # Calculate implementation order for this API's dependencies
        subgraph_nodes = deps | {api}
        subgraph_order = [n for n in graph.topological_sort() if n in subgraph_nodes]
        report["implementation_order"] = subgraph_order
        
        # Create safe filename
        safe_name = api.replace('::', '_').replace('/', '_').replace('<', '_').replace('>', '_')
        output_file = output_dir / f"{safe_name}.json"
        
        with open(output_file, 'w') as f:
            json.dump(report, f, indent=2)
    
    print(f"Generated {len(public_apis)} API reports in {output_dir}")


def generate_summary_report(plan: Dict, output_file: Path):
    """Generate human-readable summary"""
    with open(output_file, 'w') as f:
        f.write("=" * 80 + "\n")
        f.write("FIREBASE C++ SDK DEPENDENCY ANALYSIS SUMMARY\n")
        f.write("=" * 80 + "\n\n")
        
        f.write(f"Total Components: {plan['total_nodes']}\n")
        f.write(f"Total Dependencies: {plan['total_edges']}\n")
        f.write(f"Implementation Layers: {plan['statistics']['num_layers']}\n")
        f.write(f"Average Layer Size: {plan['statistics']['avg_layer_size']:.1f}\n")
        f.write(f"Leaf Nodes: {len(plan['leaf_nodes'])}\n")
        f.write(f"Max Dependencies: {plan['statistics']['max_dependencies']}\n")
        f.write(f"Max Dependents: {plan['statistics']['max_dependents']}\n")
        f.write(f"Avg Dependencies: {plan['statistics']['avg_dependencies']:.2f}\n\n")
        
        f.write("-" * 80 + "\n")
        f.write("LEAF NODES (Start Implementation Here)\n")
        f.write("-" * 80 + "\n")
        for i, leaf in enumerate(plan['leaf_nodes'][:50], 1):
            f.write(f"{i:3d}. {leaf}\n")
        
        if len(plan['leaf_nodes']) > 50:
            f.write(f"... and {len(plan['leaf_nodes']) - 50} more\n")
        
        f.write("\n" + "-" * 80 + "\n")
        f.write("IMPLEMENTATION LAYERS\n")
        f.write("-" * 80 + "\n")
        for i, layer in enumerate(plan['implementation_layers'][:10], 1):
            f.write(f"\nLayer {i} ({len(layer)} components):\n")
            for comp in layer[:10]:
                f.write(f"  - {comp}\n")
            if len(layer) > 10:
                f.write(f"  ... and {len(layer) - 10} more\n")
        
        if len(plan['implementation_layers']) > 10:
            f.write(f"\n... and {len(plan['implementation_layers']) - 10} more layers\n")
        
        f.write("\n" + "=" * 80 + "\n")
    
    print(f"Summary report saved to: {output_file}")


def main():
    """Main analysis pipeline"""
    base_dir = Path(__file__).parent
    results_dir = base_dir / "codeql_results"
    output_dir = base_dir / "analysis_output"
    output_dir.mkdir(parents=True, exist_ok=True)
    
    print("=" * 80)
    print("Firebase C++ SDK Dependency Analysis")
    print("=" * 80)
    
    # Build graph
    graph = build_graph_from_queries(results_dir)
    
    stats = graph.get_statistics()
    print(f"\nGraph Statistics:")
    print(f"  Nodes: {stats['num_nodes']}")
    print(f"  Edges: {stats['num_edges']}")
    print(f"  Leaf Nodes: {stats['num_leaves']}")
    print(f"  Root Nodes: {stats['num_roots']}")
    
    # Generate implementation plan
    plan = generate_implementation_plan(
        graph, 
        output_dir / "implementation_plan.json"
    )
    
    # Generate per-API reports
    generate_per_api_reports(
        graph, 
        output_dir / "api_reports"
    )
    
    # Generate summary
    generate_summary_report(
        plan,
        output_dir / "SUMMARY.txt"
    )
    
    print("\n" + "=" * 80)
    print("Analysis Complete!")
    print("=" * 80)
    print(f"Output directory: {output_dir}")
    print(f"  - implementation_plan.json: Full implementation order")
    print(f"  - api_reports/: Per-API dependency details")
    print(f"  - SUMMARY.txt: Human-readable summary")
    print("=" * 80)


if __name__ == "__main__":
    main()
