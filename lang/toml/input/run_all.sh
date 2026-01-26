#!/bin/bash

BINARY_CMD="cargo run --quiet --toml" 
find . -type f -iname "*.toml" | while read -r TOML_FILE; do
    $BINARY_CMD "$TOML_FILE"
    if [ $? -ne 0 ]; then
        echo "ERROR: Parsing failed for $TOML_FILE"
        exit 1 
    fi
done

echo "--- Test run complete ---"