#!/bin/bash
# Cleanup redundant documentation files

cd "$(dirname "$0")"

echo "Removing redundant documentation files..."

# Remove files
rm -f AVAILABLE_APIS.md \
      CODEQL_ANALYSIS_PLAN.md \
      IMPLEMENTATION_READY.md \
      QUICKSTART.md \
      README_CODEQL.md \
      READY_TO_PORT.md \
      USING_LOCATIONS.md

echo "Cleanup complete. Remaining docs:"
ls -1 *.md

echo ""
echo "Keeping:"
echo "  - README.md (project overview)"
echo "  - IMPLEMENTATION_MANUAL.md (THE guide)"
echo "  - QUICK_SUMMARY.md (one-page reference)"
echo "  - RUST_PORTING_ANALYSIS.md (detailed analysis)"
echo "  - SETUP_GUIDE.md (CodeQL setup if needed)"
