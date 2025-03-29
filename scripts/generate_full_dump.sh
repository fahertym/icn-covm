#!/bin/bash

cd "$(dirname "$0")/.." || exit 1
OUTPUT="full_project_dump.txt"

echo "# nano-cvm Project Dump" > "$OUTPUT"
echo "# Generated on $(date)" >> "$OUTPUT"
echo "# ----------------------" >> "$OUTPUT"
echo "" >> "$OUTPUT"

find . -type f \( \
  -name "*.rs" -o \
  -name "*.toml" -o \
  -name "*.md" -o \
  -name "*.dsl" -o \
  -name "*.json" -o \
  -name "*.yml" -o \
  -name "*.yaml" \
\) \
  -not -path "*/target/*" \
  -not -path "*/.git/*" \
  -not -name "$OUTPUT" \
  | sort | while read -r file; do
    echo "--- FILE: $file ---" >> "$OUTPUT"
    cat "$file" >> "$OUTPUT"
    echo "" >> "$OUTPUT"
done

echo "✅ Full project dump written to $OUTPUT"
