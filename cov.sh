#!/bin/bash

cargo tarpaulin \
    --engine llvm \
    --exclude "tests" \
    --exclude "lang" \
    --workspace \
    --out Lcov \
    --out html \
    --output-dir target/tarpaulin \

open target/tarpaulin/tarpaulin-report.html