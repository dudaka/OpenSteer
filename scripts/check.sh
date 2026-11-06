#!/bin/sh
# Every local gate as its own step: a failure stops the chain (never pipe a gate into tail
# or grep; the pipeline's status is the last command's, and #64 merged red that way).
set -e
cd "$(dirname "$0")/.."
cargo fmt --all
cargo clippy --all-targets -- -D warnings >/dev/null 2>&1 || { echo "CLIPPY FAILED"; cargo clippy --all-targets -- -D warnings 2>&1 | grep -E "^(error|warning)" | head -5; exit 1; }
cargo test >/dev/null 2>&1 || { echo "TESTS FAILED"; exit 1; }
npm run typecheck >/dev/null 2>&1 || { echo "TYPECHECK FAILED"; exit 1; }
npm run build >/dev/null 2>&1 || { echo "BUILD FAILED"; exit 1; }
echo "ALL GATES GREEN"
