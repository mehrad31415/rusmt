#!/bin/bash

> all_outputs.txt
for i in {1..50}; do
  echo "Running prg$i..."
  
  output=$(timeout 5s z3 -smt2 "../../studio/native/rego/prg$i/z3_chc_0/main.smt2" 2>&1 | head -n 1)

  if [ $? -eq 0 ]; then
    echo "prg$i: $output" >> all_outputs.txt
  else
    echo "prg$i: no output" >> all_outputs.txt
  fi
done

