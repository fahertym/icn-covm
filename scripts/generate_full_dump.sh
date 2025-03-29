#!/bin/bash

# Ensure we're in the project root
cd "$(dirname "$0")/.." || exit 1

# Output path
OUTPUT="full_project_dump.txt"

# Header
echo "# nano-cvm full project dump" > "$OUTPUT"
echo "# Generated on $(date)" >> "$OUTPUT"
echo "" >> "$OUTPUT"

# File types to include
find . -type f \( \
    -name "*.rs" -o \
    -name "*.toml" -o \
    -name "*.md" -o \
    -name "*.json" -o \
    -name "*.yml" -o \
    -name "*.yaml" \
\) -not -path "*/target/*" -not -path "*/.git/*" | sort | while read -r file; do
    echo "--- FILE: $file ---" >> "$OUTPUT"
    cat "$file" >> "$OUTPUT"
    echo -e "\n" >> "$OUTPUT"
done

echo "✅ Full project dump written to $OUTPUT"
