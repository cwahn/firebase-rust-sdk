# Using the Dependency Analysis Results for Porting

## Overview

The dependency analysis now includes **file paths and line numbers** for all APIs, making it easy to use as a TODO list for porting to Rust.

## Files with Location Information

### 1. API Reports (399 files)
Location: `analysis_output/api_reports/`

Each API has a JSON report with:
- **`location`**: File path and line number where the API is defined
- **`metadata`**: API details (class, method, static/virtual flags)
- **`direct_dependencies`**: Immediate requirements
- **`transitive_dependencies`**: Complete dependency closure
- **`implementation_order`**: Suggested implementation sequence

### 2. CSV Results (7 files)
Location: `codeql_results/`

Raw CodeQL query results with location columns:
- `public_api_methods.csv` - All public APIs with file_path, line_number
- `class_hierarchy.csv` - Inheritance relationships with locations
- `method_dependencies.csv` - Function calls with file_path, line_number
- `type_dependencies.csv` - Type usage with locations
- `return_type_dependencies.csv` - Return types with locations
- `field_dependencies.csv` - Field types with locations
- `include_dependencies.csv` - Header includes with line numbers

## Example Usage

### Finding an API Implementation

Want to implement `Auth::SignInWithCredential`?

```bash
cat analysis_output/api_reports/firebase_auth_Auth_SignInWithCredential.json
```

Output shows:
```json
{
  "api": "firebase::auth::Auth::SignInWithCredential",
  "location": {
    "file_path": "auth/src/desktop/auth_desktop.cc",
    "line_number": "356"
  },
  "direct_dependencies": [
    "Credential",
    "Future<User>",
    ...
  ]
}
```

Now you know:
1. **Where to look**: `firebase-cpp-sdk/auth/src/desktop/auth_desktop.cc:356`
2. **What to implement first**: `Credential`, `Future<User>`, etc.
3. **Implementation order**: Follow the `implementation_order` array

### Opening in Editor

```bash
# VSCode
code -g firebase-cpp-sdk/auth/src/desktop/auth_desktop.cc:356

# vim
vim +356 firebase-cpp-sdk/auth/src/desktop/auth_desktop.cc

# Or use the CSV for batch operations
awk -F',' 'NR>1 {print $6 ":" $7}' codeql_results/public_api_methods.csv | head
```

### Creating a TODO List

Extract all Auth APIs with locations:
```bash
grep -h '"firebase::auth::' analysis_output/api_reports/*.json | \
  jq -r '.api + " @ " + .location.file_path + ":" + .location.line_number' | \
  sort
```

Output:
```
firebase::auth::Auth::AddAuthStateListener @ auth/src/desktop/auth_desktop.cc:123
firebase::auth::Auth::CreateUserWithEmailAndPassword @ auth/src/desktop/auth_desktop.cc:245
firebase::auth::Auth::SignInWithCredential @ auth/src/desktop/auth_desktop.cc:356
...
```

### Finding Implementations of a Specific Type

Want to see all methods that return `Future<User>`?
```bash
grep -l '"Future<User>"' analysis_output/api_reports/*.json | \
  xargs -I {} jq -r '.api + " @ " + .location.file_path + ":" + .location.line_number' {}
```

### Grouping by File

See all APIs in a specific file:
```bash
jq -r 'select(.location.file_path | contains("auth.h")) | 
  .api + " @ line " + .location.line_number' \
  analysis_output/api_reports/*.json
```

## CSV Column Reference

### public_api_methods.csv
- `class_name`: Fully qualified class name
- `method_name`: Method name only
- `method_qualified`: Fully qualified method name
- `return_type`: Return type string
- `param_count`: Number of parameters
- **`file_path`**: Relative path from firebase-cpp-sdk/
- **`line_number`**: Line where method is defined

### method_dependencies.csv
- `caller_class`: Class containing the calling method
- `caller_method`: Method making the call
- `caller_qualified`: Fully qualified caller
- `callee_function`: Function being called
- **`file_path`**: Where the call occurs
- **`line_number`**: Line of the function call

### type_dependencies.csv
- `class_name`: Class containing the method
- `method_name`: Method name
- `method_qualified`: Fully qualified method
- `param_type`: Type of parameter
- **`file_path`**: Method definition location
- **`line_number`**: Method definition line

## Workflow for Porting

### Step 1: Start with Leaf Nodes
```bash
# Get leaf nodes with locations (from Layer 1)
jq -r '.layers[0][] as $node | 
  .api_locations[$node] // empty | 
  $node + " @ " + .file_path + ":" + .line_number' \
  analysis_output/implementation_plan.json | head -20
```

### Step 2: Check Dependencies
For each API you want to implement:
```bash
# Example: Check what Auth::SignInWithCredential needs
jq -r '.direct_dependencies[]' \
  analysis_output/api_reports/firebase_auth_Auth_SignInWithCredential.json
```

### Step 3: Find Implementation
```bash
# Open the C++ implementation
code -g firebase-cpp-sdk/$(jq -r '.location.file_path' \
  analysis_output/api_reports/firebase_auth_Auth_SignInWithCredential.json):$(jq -r '.location.line_number' \
  analysis_output/api_reports/firebase_auth_Auth_SignInWithCredential.json)
```

### Step 4: Port to Rust
1. Read the C++ implementation at the specified location
2. Understand the logic and dependencies
3. Implement the Rust equivalent
4. Mark as complete and move to next API

## Tips

### Batch Processing
Create a script to process all APIs:
```bash
#!/bin/bash
for api_file in analysis_output/api_reports/firebase_auth_*.json; do
    api=$(jq -r '.api' "$api_file")
    location=$(jq -r '.location.file_path + ":" + .location.line_number' "$api_file")
    deps=$(jq -r '.direct_dependencies | length' "$api_file")
    echo "TODO: $api @ $location (needs $deps deps)"
done
```

### Progress Tracking
Track which APIs you've implemented:
```bash
# Create a tracking file
jq -r '.api + " | TODO"' analysis_output/api_reports/firebase_auth_*.json > auth_progress.txt

# Mark as done
sed -i '' 's/firebase::auth::Auth::GetAuth | TODO/firebase::auth::Auth::GetAuth | DONE/' auth_progress.txt

# See progress
grep -c "DONE" auth_progress.txt
grep -c "TODO" auth_progress.txt
```

### Filter by Complexity
Find simple APIs (few dependencies):
```bash
for f in analysis_output/api_reports/firebase_auth_*.json; do
    api=$(jq -r '.api' "$f")
    deps=$(jq -r '.direct_dependencies | length' "$f")
    loc=$(jq -r '.location.file_path + ":" + .location.line_number' "$f")
    echo "$deps $api @ $loc"
done | sort -n | head -20
```

## Integration with IDEs

### VSCode Tasks
Add to `.vscode/tasks.json`:
```json
{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "Open C++ Implementation",
      "type": "shell",
      "command": "code -g firebase-cpp-sdk/$(jq -r '.location.file_path' ${input:apiReport}):$(jq -r '.location.line_number' ${input:apiReport})",
      "problemMatcher": []
    }
  ],
  "inputs": [
    {
      "id": "apiReport",
      "type": "pickString",
      "description": "Select API",
      "options": []
    }
  ]
}
```

### GitHub Issues
Generate GitHub issues from API list:
```bash
for api_file in analysis_output/api_reports/firebase_auth_*.json; do
    api=$(jq -r '.api' "$api_file")
    location=$(jq -r '.location.file_path + ":" + .location.line_number' "$api_file")
    deps=$(jq -r '.direct_dependencies | join(", ")' "$api_file")
    
    echo "## Port $api"
    echo ""
    echo "**Location:** \`$location\`"
    echo ""
    echo "**Dependencies:**"
    echo "$deps" | tr ',' '\n' | sed 's/^/- /'
    echo ""
    echo "---"
    echo ""
done > auth_issues.md
```

## Summary

With file locations now included in all analysis results:
- ✅ **Every API** has a file path and line number
- ✅ **Easy navigation** to C++ implementations
- ✅ **Clear TODO lists** can be generated
- ✅ **Progress tracking** is straightforward
- ✅ **IDE integration** is possible

Start implementing from leaf nodes (Layer 1) and work your way up through the 32 layers, using the location information to quickly find and understand each C++ implementation!
