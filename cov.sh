#!/bin/bash

cargo clean

cargo tarpaulin \
    --engine llvm \
    --packages "rusmart-smt-stdlib" \
    --packages "rusmart-smt-remark" \
    --packages "rusmart-smt-remark-derive" \
    --packages "rusmart-smt-derive" \
    --out Lcov \
    --out html \
    --output-dir target/tarpaulin

open target/tarpaulin/tarpaulin-report.html
