#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SDK_DIR="$SCRIPT_DIR/firebase-cpp-sdk"
DB_DIR="$SCRIPT_DIR/codeql-db"

echo "==============================================================================="
echo "Creating CodeQL Database for C++ (Build Mode: None)"
echo "==============================================================================="
echo ""
echo "Using CodeQL's '--build-mode none' for C++ analysis without compilation"
echo "This analyzes header files and source code structure directly"
echo ""

# Remove old database
rm -rf "$DB_DIR"

cd "$SDK_DIR"

# Create database using --build-mode none (no compilation needed!)
echo "Creating CodeQL database (this may take 5-15 minutes)..."
codeql database create "$DB_DIR" \
    --language=cpp \
    --source-root=. \
    --build-mode=none \
    --overwrite \
    2>&1 | tee "$SCRIPT_DIR/codeql_create.log"

if [ $? -eq 0 ] && [ -d "$DB_DIR/db-cpp" ]; then
    echo ""
    echo "✓ Database created successfully!"
    echo "Location: $DB_DIR"
    echo ""
    
    # Show database info
    echo "Database information:"
    codeql database info "$DB_DIR"
else
    echo ""
    echo "✗ Database creation failed"
    echo "Check codeql_create.log for details"
    exit 1
fi
