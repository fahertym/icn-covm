#!/bin/bash

cd "$(dirname "$0")/.." || exit 1

if [ $# -eq 0 ]; then
  echo "Usage: $0 <demo_file.dsl> [more_files.dsl...]"
  exit 1
fi

for file in "$@"; do
  echo "=== Running $file ==="
  cargo run -- -p "$file" --stdlib || echo "❌ Failed: $file"
  echo ""
done
