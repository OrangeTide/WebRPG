#!/usr/bin/env bash
# Check that both SSR and hydrate targets compile without warnings or errors.
set -euo pipefail

echo "=== Checking SSR target ==="
cargo check --features ssr 2>&1 | tee /tmp/ssr-check.log
if grep -qi "warning\|error" /tmp/ssr-check.log; then
    echo "FAIL: SSR target has warnings or errors"
    exit 1
fi
echo "PASS: SSR target clean"

echo ""
echo "=== Checking hydrate target ==="
cargo check --features hydrate --target wasm32-unknown-unknown 2>&1 | tee /tmp/hydrate-check.log
if grep -qi "warning\|error" /tmp/hydrate-check.log; then
    echo "FAIL: hydrate target has warnings or errors"
    exit 1
fi
echo "PASS: hydrate target clean"

echo ""
echo "=== Running cargo test ==="
cargo test 2>&1
echo "PASS: all tests passed"
