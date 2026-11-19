#!/bin/sh
# Prints every Rust source file with the first line of its `//!` header: a map of the whole project in about 3,000 tokens.
# Usage: scripts/code-map.sh
cd "$(dirname "$0")/.." || exit 1
find apps crates -name '*.rs' -path '*/src/*' | sort | while read -r file; do
  summary=$(sed -n 's|^//! *||p' "$file" | head -1)
  printf '%s (%s lines): %s\n' "$file" "$(wc -l < "$file" | tr -d ' ')" "${summary:-NO HEADER}"
done
