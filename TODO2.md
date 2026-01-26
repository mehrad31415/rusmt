Complete list of todos:

1 - Add bitvector size I8, U8, I16, U16 and so on (make the bitvector modular) also do it for floating points
2 - Z3_mk_set_complement in sets
3 - Maybe change the whole structure so that IR stores functions that cannot natively be converted to SMT instead of giving it to the backend
4 - Rust: Rounds ties away from zero (2.5 $\to$ 3.0).Wasm/Z3: Rounds ties to even (2.5 $\to$ 2.0). So what do we do? what if the behaviour of z3 is different than the language we are writing the interpreter for using the DSL and that is different than the behaviour of Rust itself?