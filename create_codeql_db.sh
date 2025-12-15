#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SDK_DIR="$SCRIPT_DIR/firebase-cpp-sdk"
DB_DIR="$SCRIPT_DIR/codeql-db"

echo "==============================================================================="
echo "Creating CodeQL Database - Simple Approach"
echo "==============================================================================="
echo ""

# Remove old database
rm -rf "$DB_DIR"

# Create a minimal CMake build to trace
cd "$SDK_DIR"
mkdir -p build_temp
cd build_temp

echo "Generating CMake configuration..."
cmake -DCMAKE_EXPORT_COMPILE_COMMANDS=ON \
      -DFIREBASE_CPP_BUILD_TESTS=OFF \
      ..  2>&1 | head -20

if [ ! -f "compile_commands.json" ]; then
    echo "ERROR: Could not generate compile_commands.json"
    exit 1
fi

echo ""
echo "Creating CodeQL database..."
cd "$SDK_DIR"

# Use the compile_commands.json to create database
codeql database create "$DB_DIR" \
    --language=cpp \
    --source-root=. \
    --command="cat build_temp/compile_commands.json > /dev/null" \
    --threads=0 \
    --overwrite

echo ""
echo "Database created at: $DB_DIR"
echo ""

# Cleanup
rm -rf build_temp

# Try to index the database
echo "Finalizing database..."
codeql database finalize "$DB_DIR"

echo ""
echo "Done!"
