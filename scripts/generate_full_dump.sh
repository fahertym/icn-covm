#!/bin/bash

# Ensure we're in the project root
cd "$(dirname "$0")/.."

# Generate the full project dump
find . -type f \( \
    -name "*.rs" -o \
    -name "*.toml" -o \
    -name "*.md" -o \
    -name "*.json" -o \
    -name "*.yml" -o \
    -name "*.yaml" \
\) \
-not -path "*/target/*" \
-not -path "*/.git/*" \
-not -path "./full_project_dump.txt" \
-exec echo "--- FILE: {} ---" \; \
-exec cat {} \; > full_project_dump.txt

echo "✅ Full project dump generated at full_project_dump.txt"
