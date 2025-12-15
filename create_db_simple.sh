#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SDK_DIR="$SCRIPT_DIR/firebase-cpp-sdk"
DB_DIR="$SCRIPT_DIR/codeql-db"

echo "==============================================================================="
echo "Creating CodeQL Database - Using Fake Build"
echo "==============================================================================="
echo ""

rm -rf "$DB_DIR"

cd "$SDK_DIR"

echo "Creating database with fake build command..."
codeql database create "$DB_DIR" \
    --language=cpp \
    --source-root="." \
    --command="./fake_build.sh" \
    --overwrite \
    2>&1 | tee "$SCRIPT_DIR/codeql_create.log"

result=$?

if [ $result -eq 0 ] || [ -d "$DB_DIR/db-cpp" ]; then
    echo ""
    echo "✓ Database created successfully!"
    echo "Location: $DB_DIR"
else
    echo ""
    echo "✗ Database creation failed"
    echo "Check codeql_create.log for details"
    exit 1
fi
