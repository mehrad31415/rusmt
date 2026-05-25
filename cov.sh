#!/bin/bash

cargo clean

cargo tarpaulin \
    --engine llvm \
    --packages "rusmt-smt-stdlib" \
    --packages "rusmt-smt-remark" \
    --packages "rusmt-smt-remark-derive" \
    --packages "rusmt-smt-derive" \
    --out Lcov \
    --out html \
    --output-dir target/tarpaulin

open target/tarpaulin/tarpaulin-report.html
