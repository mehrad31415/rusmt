#!/bin/bash
set -euo pipefail

cargo tarpaulin \
    --engine llvm \
    --exclude-files 'lang/**' \
    --out Lcov \
    --out html \
    --output-dir target/tarpaulin

open target/tarpaulin/tarpaulin-report.html
