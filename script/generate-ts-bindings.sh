#!/bin/bash
# Generate TypeScript bindings from Rust types using ts-rs
#
# Usage:
#   ./script/generate-ts-bindings.sh
#
# This script:
# 1. Cleans previous output
# 2. Runs the ts-rs export test with the ts-bindings feature
# 3. Outputs TypeScript files to packages/ccode/src/generated/

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

OUTPUT_DIR="$PROJECT_ROOT/packages/ccode/src/generated"

echo "Generating TypeScript bindings from Rust..."
echo "Output directory: $OUTPUT_DIR"

# Clean previous output (keep index.ts)
if [ -d "$OUTPUT_DIR" ]; then
    echo "Cleaning previous output..."
    find "$OUTPUT_DIR" -name "*.ts" ! -name "index.ts" -delete 2>/dev/null || true
    rm -rf "$OUTPUT_DIR/guardrails" "$OUTPUT_DIR/hitl" "$OUTPUT_DIR/events" 2>/dev/null || true
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Generate bindings
cd "$PROJECT_ROOT/services"

echo "Running ts-rs export test..."
TS_RS_EXPORT_DIR="$OUTPUT_DIR" cargo test \
    --package zero-common \
    --features ts-bindings,hitl-client \
    export_bindings \
    --release \
    -- --nocapture

echo ""
echo "Generated TypeScript bindings:"
find "$OUTPUT_DIR" -name "*.ts" | sort

echo ""
echo "Done! Bindings written to $OUTPUT_DIR"
