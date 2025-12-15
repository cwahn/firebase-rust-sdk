#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SDK_DIR="$SCRIPT_DIR/firebase-cpp-sdk"
DB_DIR="$SCRIPT_DIR/codeql-db"
QUERIES_DIR="$SCRIPT_DIR/codeql_queries"
RESULTS_DIR="$SCRIPT_DIR/codeql_results"

echo "==============================================================================="
echo "Firebase C++ SDK Dependency Analysis"
echo "==============================================================================="
echo ""

# Check if CodeQL is installed
if ! command -v codeql &> /dev/null; then
    echo "ERROR: CodeQL CLI not found!"
    echo ""
    echo "Please install CodeQL:"
    echo "  1. Download from: https://github.com/github/codeql-cli-binaries/releases"
    echo "  2. Extract and add to PATH"
    echo "  3. Run: codeql --version"
    echo ""
    exit 1
fi

echo "CodeQL version: $(codeql --version | head -n1)"
echo ""

# Check if SDK exists
if [ ! -d "$SDK_DIR" ]; then
    echo "ERROR: Firebase C++ SDK not found at: $SDK_DIR"
    echo "Please clone it first: git clone https://github.com/firebase/firebase-cpp-sdk.git"
    exit 1
fi

echo "SDK location: $SDK_DIR"
echo ""

# Step 1: Create CodeQL database
echo "==============================================================================="
echo "[1/4] Creating CodeQL Database"
echo "==============================================================================="
echo ""

if [ -d "$DB_DIR" ]; then
    echo "Database already exists at: $DB_DIR"
    read -p "Delete and recreate? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo "Removing existing database..."
        rm -rf "$DB_DIR"
    else
        echo "Using existing database."
    fi
fi

if [ ! -d "$DB_DIR" ]; then
    echo "Creating CodeQL database (this may take 10-30 minutes)..."
    echo "Database will be created at: $DB_DIR"
    echo ""
    
    # Step 1: Generate compile_commands.json with CMake
    BUILD_DIR="$SDK_DIR/build_codeql"
    echo "Generating compile_commands.json with CMake..."
    mkdir -p "$BUILD_DIR"
    cd "$BUILD_DIR"
    
    cmake -DCMAKE_EXPORT_COMPILE_COMMANDS=ON \
          -DFIREBASE_CPP_BUILD_TESTS=OFF \
          -DFIREBASE_CPP_BUILD_SAMPLES=OFF \
          .. > /dev/null 2>&1
    
    if [ ! -f "compile_commands.json" ]; then
        echo "WARNING: Could not generate compile_commands.json"
        echo "Trying alternative approach..."
        cd "$SDK_DIR"
    else
        echo "compile_commands.json generated successfully"
        cp compile_commands.json "$SDK_DIR/"
        cd "$SDK_DIR"
    fi
    
    # Step 2: Create CodeQL database using compile_commands.json
    if [ -f "$SDK_DIR/compile_commands.json" ]; then
        echo "Creating database using compile_commands.json..."
        codeql database create "$DB_DIR" \
            --language=cpp \
            --source-root="$SDK_DIR" \
            --command="python3 -c 'import json; [print(c[\"file\"]) for c in json.load(open(\"compile_commands.json\"))]'" \
            --overwrite \
            2>&1 | tee "$SCRIPT_DIR/codeql_create.log"
    else
        echo "Using header-only analysis mode..."
        # For header-only analysis, we need to trace actual file access
        codeql database create "$DB_DIR" \
            --language=cpp \
            --source-root="$SDK_DIR" \
            --begin-tracing \
            --no-run-unnecessary-builds \
            2>&1 | tee "$SCRIPT_DIR/codeql_create.log" || true
    fi
    
    echo ""
    echo "Database created!"
    echo ""
else
    echo "Using existing database."
    echo ""
fi

# Verify database
echo "Verifying database..."
codeql database info "$DB_DIR" || echo "Database verification skipped"
echo ""

# Step 2: Run CodeQL queries
echo "==============================================================================="
echo "[2/4] Running CodeQL Queries"
echo "==============================================================================="
echo ""

mkdir -p "$RESULTS_DIR"

query_count=0
for query in "$QUERIES_DIR"/*.ql; do
    if [ ! -f "$query" ]; then
        echo "No queries found in $QUERIES_DIR"
        exit 1
    fi
    
    query_name=$(basename "$query" .ql)
    echo "Running query: $query_name"
    echo "  Query file: $query"
    
    # Run query and save BQRS
    codeql query run \
        --database="$DB_DIR" \
        --output="$RESULTS_DIR/${query_name}.bqrs" \
        "$query"
    
    # Convert BQRS to CSV
    codeql bqrs decode \
        --format=csv \
        --output="$RESULTS_DIR/${query_name}.csv" \
        "$RESULTS_DIR/${query_name}.bqrs"
    
    # Show row count
    row_count=$(tail -n +2 "$RESULTS_DIR/${query_name}.csv" | wc -l | tr -d ' ')
    echo "  Result: $row_count rows"
    echo ""
    
    query_count=$((query_count + 1))
done

echo "Completed $query_count queries"
echo "Results saved to: $RESULTS_DIR"
echo ""

# Step 3: Analyze dependencies
echo "==============================================================================="
echo "[3/4] Analyzing Dependencies"
echo "==============================================================================="
echo ""

if ! command -v python3 &> /dev/null; then
    echo "ERROR: python3 not found!"
    exit 1
fi

python3 "$SCRIPT_DIR/analyze_dependencies.py"

# Step 4: Display summary
echo ""
echo "==============================================================================="
echo "[4/4] Analysis Summary"
echo "==============================================================================="
echo ""

if [ -f "$SCRIPT_DIR/analysis_output/SUMMARY.txt" ]; then
    cat "$SCRIPT_DIR/analysis_output/SUMMARY.txt"
else
    echo "Summary file not found"
fi

echo ""
echo "==============================================================================="
echo "Analysis Complete!"
echo "==============================================================================="
echo ""
echo "Output files:"
echo "  - analysis_output/implementation_plan.json"
echo "  - analysis_output/api_reports/*.json"
echo "  - analysis_output/SUMMARY.txt"
echo ""
echo "Next steps:"
echo "  1. Review SUMMARY.txt for overview"
echo "  2. Check implementation_plan.json for full implementation order"
echo "  3. Check api_reports/ for per-API dependency details"
echo "  4. Start implementing from leaf nodes!"
echo ""
