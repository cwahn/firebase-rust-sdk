#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SDK_DIR="$SCRIPT_DIR/firebase-cpp-sdk"
DB_DIR="$SCRIPT_DIR/codeql-db"

echo "==============================================================================="
echo "Creating CodeQL Database - Direct Extraction Method"
echo "==============================================================================="
echo ""

rm -rf "$DB_DIR"

# Initialize the database
echo "[1/3] Initializing CodeQL database..."
codeql database init --language=cpp --source-root="$SDK_DIR" "$DB_DIR"

# Run the extractor with tracing on the header files directly
echo "[2/3] Extracting source code (this may take 10-20 minutes)..."
cd "$SDK_DIR"

# Use codeql database trace-command to trace file access while listing all files
codeql database trace-command "$DB_DIR" \
    find auth/src/include firestore/src/include -name "*.h" -o -name "*.hpp" -exec cat {} \; > /dev/null

echo "[3/3] Finalizing database..."
codeql database finalize "$DB_DIR"

echo ""
echo "Database created successfully at: $DB_DIR"
echo ""

# Show database info
codeql database info "$DB_DIR" 2>/dev/null || echo "Database created (info command not available)"
