### Rusmart Standard Library (stdlib)

---

## Stdlib Semantics and the Soundness Invariant

### What the stdlib represents

Every stdlib function has **two sides** that must agree:

1. **The Rust implementation** — runs concretely when you execute your interpreter.
2. **The Z3 translation** — the SMT-LIB formula emitted when transpiling to Z3.

The core soundness requirement is:

> **For every stdlib function `f` and every concrete input `x`:**
> ```
> rust_impl(f, x) == z3_eval(z3_formula(f), x)
> ```
> That is, evaluating `f` concretely in Rust must yield the same result as evaluating Z3's formula for `f` on the same concrete value.

This matters because the workflow is:
1. You write an interpreter for a target language using the DSL.
2. The DSL is transpiled to Z3.
3. Z3 finds **models** — concrete inputs that trigger specific path conditions.
4. Those models are fed back into the **Rust interpreter** to generate conformance test cases.

If the Rust side and the Z3 side disagree, a model that Z3 finds "satisfies condition C" may not actually satisfy C when run concretely — producing **unsound test cases**.

### What the stdlib does NOT represent

The stdlib represents **Z3's mathematical theories** — not Rust's native semantics, and not the semantics of any specific target language.

- **Not Rust semantics.** Rust's `i32` overflows; `Integer` (backed by `BigInt`) does not. Rust's integer division truncates; `Integer::div` is Euclidean. These deliberate divergences make the stdlib match Z3, not Rust.
- **Not target language semantics.** WebAssembly, Python, C, and JavaScript all have different overflow, rounding, and division rules. The stdlib cannot simultaneously model all of them. This is the **interpreter author's responsibility**.

Think of it as three layers:

```
Layer 3: Conformance test case (a Z3 model — concrete inputs)
               ↑  produced by
Layer 2: Your interpreter (encodes YOUR target language's semantics)
               ↑  uses as building blocks
Layer 1: Stdlib (Z3-correct primitives — the vocabulary)
```

### Adapting stdlib operations to your target language

When a target language operation matches a stdlib primitive exactly, use it directly:

```rust
// Python's // uses floor division — Integer::div is Euclidean = floor for ints
fn python_floordiv(a: Integer, b: Integer) -> Integer {
    a.div(b)
}

// C/Rust/Java's / uses truncation division — use div_trunc
fn c_divide(a: Integer, b: Integer) -> Integer {
    a.div_trunc(b)
}
```

When a target language semantics only diverges for **specific inputs** (e.g., error cases, overflow, NaN), use **guard branches** to handle the diverging cases and fall through to the stdlib for the rest:

```rust
fn wasm_i32_div_s(a: I32, b: I32) -> Result<I32, Error> {
    // WebAssembly traps on division by zero and on i32::MIN / -1
    if b.eq(I32::from(0)) {
        return Err(trap());
    }
    if a.eq(I32::from(i32::MIN)) && b.eq(I32::from(-1)) {
        return Err(trap());  // would overflow
    }
    Ok(a.bv_div(b))  // stdlib is correct for all other inputs
}
```

When no single stdlib primitive matches, compose multiple primitives:

```rust
// Saturating 32-bit addition (clamps at ±MAX instead of wrapping)
fn lang_sat_add(a: I32, b: I32) -> I32 {
    let r = a.to_int().add(b.to_int());  // lift to unbounded Integer
    if r.gt(Integer::from(i32::MAX)) {
        I32::from(i32::MAX)
    } else if r.lt(Integer::from(i32::MIN)) {
        I32::from(i32::MIN)
    } else {
        r.to_i32()
    }
}
```

### The if-then-else pattern — summary

> **If stdlib function `f` matches your target language for most inputs but diverges for some specific cases, write guard branches for the diverging cases first, then use `f` in the final else branch.**

The guards encode the diverging cases as Z3-checkable conditions. Z3 will find models for each branch independently. For each model, the concrete Rust execution follows the same branch as Z3 predicted — so soundness is preserved in every branch.

### Input validation contract

Some stdlib functions have preconditions on their inputs. For example, `Integer::from_hex_str`
expects a string of pure hex digits — no `0x` prefix, no underscores. On invalid input:

- **Rust** panics (`unwrap()` on a failed parse).
- **Z3** silently returns a wrong value (e.g., 0 for an unrecognized character).

These behaviors diverge, but this does NOT break soundness because:

> **The interpreter must validate inputs before calling stdlib functions.**

The TOML parser, for example, checks each character with `is_hex_digit` and strips
prefixes/underscores before the string ever reaches `from_hex_str`. Invalid inputs are
caught by the parser and produce `Error::fresh()` — they never reach the stdlib.

Since the parser validates identically in both Rust and Z3, the invalid-input path is
unreachable in both worlds. The divergence exists on a path that no execution can take.

**Rule for interpreter authors:** if a stdlib function can panic on certain inputs, your
interpreter must guard against those inputs explicitly. Do not rely on the stdlib to handle
invalid inputs gracefully — it is not designed to. The stdlib assumes its inputs satisfy
the documented preconditions. If you violate them, Rust panics and Z3 gives garbage, and
any models Z3 finds on that path are unsound.

### Integer — functions that panic

The following `Integer` methods panic on certain inputs. When writing an interpreter,
we guard against these inputs before calling the method (e.g., checking divisor != 0                                                                                           
before calling `div`). On the unguarded path, we use the method as-is. On the guarded path, 
the panic is unreachable in both Rust and Z3, and both must produce identical results — soundness holds. 
Also we must replicate the behavior of the target language:    
                                                                                                   
- If the target language treats it as an error (e.g., division by zero is a runtime                                                                                            
error in most languages), we produce `Error::fresh()` in the guard branch.                                                                                                   
- If the target language defines specific behavior for that input (e.g., saturating,                                                                                           
wrapping, returning a default), we implement that behavior using other stdlib
operations in the guard branch.

In either case, the guard branch handles the diverging input before the stdlib method                                                                                          
is reached, so the panic never fires. We have not yet encountered a case where the                                                                                             
stdlib lacks the operations needed to replicate a target language's behavior in the                                                                                            
guard branch, but this may arise as more languages are implemented.

If the interpreter does NOT guard, Rust panics (crashes) while Z3 silently
returns a wrong value. Any models Z3 finds on that path are unsound because
the Rust side never reaches the return — it crashes instead.

- `div(rhs)` — panics when `rhs == 0`
- `div_trunc(rhs)` — panics when `rhs == 0`
- `modulo(rhs)` — panics when `rhs == 0`
- `rem(rhs)` — panics when `rhs == 0`
- `pow(exp)` — panics when `exp < 0` or `exp > u32::MAX`
- `divides(rhs)` — panics when `self == 0`
- `to_i32()` — panics when value is outside `[-2147483648, 2147483647]`
- `to_i64()` — panics when value is outside `[-9223372036854775808, 9223372036854775807]`
- `to_u32()` — panics when value is outside `[0, 4294967295]`
- `to_u64()` — panics when value is outside `[0, 18446744073709551615]`
- `to_f32()` — panics when value is too large for f32
- `to_f64()` — panics when value is too large for f64
- `from_hex_str(s)` — panics when `s` contains non-hex characters
- `from_oct_str(s)` — panics when `s` contains non-octal characters
- `from_bin_str(s)` — panics when `s` contains characters other than `0` or `1`

### String — functions that panic

- `at(index)` — panics when index is out of bounds (negative or >= length)
- `index_of(substr, offset)` — panics when offset is negative or bigger than the length of the string, or when substr is not found
- `index_of_default(substr)` — panics when substr is not found or the offset is bigger than the length of the string
- `substr(offset, length)` — panics when offset or length are negative or when offset is beyond the string length
- `to_int()` — panics when the string is not a valid integer. Divergence: Rust parses any valid integer (including negative), Z3's `str.to_int` returns -1 for non-digit strings & negative numbers. Guard: ensure the string contains only digits before calling.
- `from_int(i)` — does not panic but diverges: Rust's `to_string()` gives gives the expected result for negative integers, Z3's `str.from_int` gives `""` for negative numbers. Guard: check for negative integers before calling if the target language needs specific behavior.
- `replace_all(s, "", dst)` — does not panic but diverges: Rust inserts `dst` at every position (e.g., `"Hello"` → `"XHXeXlXlXoX"`), Z3 returns the original string unchanged. Guard: do not call `replace_all` with an empty source string.
- `from_code(code)` — panics when code is negative, greater than u32 max, or not a valid Unicode scalar value (not all u32 values are valid characters). Z3's `str.from_code` returns `""` for invalid values instead of panicking.
- `to_code()` — panics on empty string or string with more than one character but z3 returns -1.

### Seq — functions that panic

- `at(index)` — panics when index is out of bounds (negative or >= length). Z3's `seq.nth` returns an value `-1`.
- `at_seq(index)` — panics when index is out of bounds (negative or >= length). Z3's `seq.extract` returns an empty sequence.
- `extract(offset, length)` — panics when offset or length are negative, or when offset is beyond the sequence length. Z3's `seq.extract` returns an empty sequence for invalid inputs.
- `index_of(sub, offset)` — panics when offset is negative, when the subsequence is longer than the sequence, when offset is beyond the valid search range, or when the subsequence is not found. Z3's `seq.indexof` returns -1 when not found.
- `index_of_default(sub)` — same as `index_of` with offset 0.

### Real — functions that panic

- `div(rhs)` — panics when `rhs == 0`
- `pow(exp)` — panics when `exp` is too large for i32 (outside `[-2147483648, 2147483647]`)
- `to_f32()` — panics when value is too large for f32 but is finite in f64 (roughly `1.8 * 10^308 > |value| > 3.4 * 10^38`)

### Bitvector — functions that panic

- `bv_div(rhs)` — panics when `rhs == 0`. Z3's `bvsdiv`/`bvudiv` returns `0xFFFFFFFF` (all-ones) for division by zero. Note: signed `MIN / -1` does NOT panic — `wrapping_div` wraps to `MIN`, matching Z3's `bvsdiv` behavior.
- `bv_rem(rhs)` — panics when `rhs == 0`. Z3's `bvsrem`/`bvurem` returns the dividend unchanged for remainder by zero.
- `bv_mod(rhs)` — panics when `rhs == 0`. Z3's `bvsmod`/`bvurem` returns the dividend unchanged for modulo by zero.

### Float — functions that panic

- `to_integer()` — panics on NaN or Infinity. Z3's `(to_int (fp.to_real x))` returns an unspecified value.
- `to_real()` — panics on NaN or Infinity. Z3's `(fp.to_real x)` returns an unspecified value.
- `to_i32()` — panics on NaN, Infinity, or value outside `[-2147483648, 2147483647]`. Z3's `(fp.to_sbv 32)` returns an unspecified bitvector.
- `to_i64()` — panics on NaN, Infinity, or value outside `[-9223372036854775808, 9223372036854775807]`. Z3's `(fp.to_sbv 64)` returns an unspecified bitvector.
- `to_u32()` — panics on NaN, Infinity, negative values, or value > 4294967295. Z3's `(fp.to_ubv 32)` returns an unspecified bitvector.
- `to_u64()` — panics on NaN, Infinity, negative values, or value > 18446744073709551615. Z3's `(fp.to_ubv 64)` returns an unspecified bitvector.
- `from_hex_str(s)` — panics on invalid hex float string.

### Float — NaN behavior notes

- `is_negative()` / `is_positive()` — guarded with `!is_nan()` to match Z3. Rust's `is_sign_negative` returns true for `-NaN`, Z3's `fp.isNegative` returns false for all NaN. Without the guard, soundness breaks.
- `rem(rhs)` — uses `libm::remainderf`/`libm::remainder` (IEEE 754 remainder), NOT Rust's `%` which is fmod. Z3's `fp.rem` matches IEEE 754.
- `nearest()` — custom implementation for ties-to-even. Rust's `f64::round()` uses ties-away-from-zero, Z3's RNE uses ties-to-even.

### Theoretical limitations (unguarded edge cases)

The following cases are not explicitly guarded in the TOML interpreter because they
require pathological inputs that are impractical in real-world usage. They are documented
here for completeness.

- **`Real.pow(exp)` with digit-count exponent**: In the TOML float parser (`float.rs`),
  expressions like `Real::from(10).pow(number_of_digits(val).neg().to_real())` use the
  digit count of a parsed number as the exponent. `Real.pow` internally calls
  `exp.to_integer().to_i32().unwrap()`, which panics if the exponent exceeds i32 range.
  The digit count is not explicitly bounded — a TOML file containing a number with more
  than 2,147,483,647 digits (~2 GB of digits alone) would trigger this panic. In practice,
  the parser would exhaust memory long before reaching this limit. No guard is added
  because the input size makes this an unreachable test case in any realistic scenario.

- **`Integer.pow(exp)` with digit-count exponent**: Similarly, `Integer::from(10).pow(number_of_digits(val))`
  in the float parser uses the digit count as an exponent for `Integer.pow`, which requires
  the exponent to fit in `u32`. A number with more than 4,294,967,295 digits (~4 GB) would
  be needed to trigger this. Same practical impossibility applies.

### String encoding: ASCII-only soundness

The stdlib `String` operations (`length`, `at`, `substr`, `index_of`, etc.) count
Unicode code points (via Rust's `.chars()`). Z3's string theory counts UTF-8 bytes.
For ASCII (code points 0-127), one code point equals one byte so they agree. For
non-ASCII, they diverge:

| Input | Rust (code points) | Z3 (bytes) |
|-------|-------------------|------------|
| `"Hello"` | 5 | 5 |
| `"é"` (U+00E9) | 1 | 2 |
| `"😀"` (U+1F600) | 1 | 4 |

This affects all position-based operations (`at`, `substr`, `index_of`) since indices
refer to different units. The soundness invariant holds only for ASCII input.

Changing the Rust side to byte-based is not feasible: Z3's `str.at` can return
individual bytes of multi-byte UTF-8 sequences (e.g., `(str.at "😀" 0)` returns
`\xF0`), which cannot be stored in a Rust `String` (requires valid UTF-8).

For interpreter authors: restrict string inputs to ASCII for soundness.

---

Rusmart standard library (_stdlib_) contains language constructs that cannot be expressed readily in Rust as they have special semantics in SMT. The _rusmart-smt-stdlib_ package consists of one _library crate_. The crate contains two modules: `dt` and `exp`. The `dt` module contains data types part of the type system in Rusmart, while the `exp` module contains expressions. Both modules are re-exported in the root of the crate to allow users to use data types and expressions directly.

#### Trait

- `SMT`: marks that a Rust type is also an SMT type.
  In order to bridge the semantic gap between a Rust type and an SMT type,
  the `SMT` trait encodes several restrictions:

    - Rust types implementing the `SMT` trait
      must also implement other traits (these are the supertraits of `SMT`):
        - `Copy`: as SMT values are processed by value and never by reference.
        - `Default`: to allow quantified expressions
          (including `choose` operators)
          to type-check in Rust type system.
          The implementation can be arbitrary (including `panic!`)
          as it won't be executed concretely.
        - `Hash`: to allow SMT values to be used as keys in a `HashSet` or `HashMap`.
        - `Send` and `Sync`: to allow SMT values to be sent across threads. The `Send`  indicates that ownership of values of the type implementing Send can be transferred between threads. The `Sync` marker trait indicates that it is safe for the type implementing Sync to be referenced from multiple threads. 
    - Rust types implementing the `SMT` trait
      will also need to implement the following functions
      that are generally supported on SMT values:
        - `_cmp`: comparison test
        - `eq`: equality test
        - `ne`: non-equality test
        - The `ne` and `eq` methods have default implementations that use the `_cmp` method. The `_cmp` method is used to compare two values of the type implementing the `SMT` trait. The `_cmp` method returns an `Ordering` value. The `Ordering` enum is defined in the standard library and has the following variants: `Less`, `Equal`, and `Greater`. The `eq` method returns true if the `_cmp` method returns `Equal`. The `ne` method returns true if the `_cmp` method returns `Less` or `Greater`. The `_cmp` method is required to be defined by the type implementing the `SMT` trait.

#### Data types

These data types are part of the [type system](../../../user/typing.md) in Rusmart:

- `Boolean`: A wrapper around the Rust boolean type. The definition of the `Boolean` type is as follows:

```rust
pub struct Boolean {
    inner: bool,
}
```
Note that this approach of wrapping inside a struct with an `inner` field is a common way in the libraries of Rust itself and this approach is used in the Rusmart standard library as well.

- `Integer`: Unbounded integer (backed by `num_bigint::BigInt`).
- `Real`: Unbounded rational (backed by `num_rational::BigRational`).
- `I32`, `I64`, `U32`, `U64`: Bitvectors (SMT-LIB `(_ BitVec 32)` / `(_ BitVec 64)`), with signed/unsigned *interpretations* in the DSL API.
- `F32`, `F64`: Floating-point sorts (SMT-LIB `(_ FloatingPoint 8 24)` / `(_ FloatingPoint 11 53)`).
- `String`: A wrapper around Rust `String`, corresponding to SMT-LIB `String`.
- `Cloak<T>`: A wrapper over `T` to allow recursive data types to be defined (similar to `Box<T>` in Rust). A `Cloak<T>` will be uncloaked to `T` after the parsing stage of Rusmart.
- `Seq<T>`: SMT sequence of type `T` similar to Rust `Vec<T>`.
- `Set<T>`: SMT set of type `T` similar to Rust `BTreeSet<T>`.
- `Array<K, V>`: SMT array of key type `K` and value type `V`, similar to a persistent `BTreeMap<K, V>` in the interpreter.
- `Error`: A special marker to indicate error states. The error state is created by calling the `Error::fresh()` function. Every time the `fresh()` method is called, a new error state is created with a unique inner value. The inner values are incremented by one each time a new error state is created.

## Expressions

- `forall |v1 in c1, v2 in c2, ..., vn in cn| <predicate>(v1, v2, ..., vn)`
    - **SMT and Rust**: universally quantified over bounded collections
    - In Rust, the `<predicate>` are checked in a loop
      iterating over all possible combination of variables `v1, v2, ..., vn`.

Basically, the _forall_ macro has one form:
    - `forall! (v1 in c1, v2 in c2, ..., vn in cn => <predicate>)`

The _c1_, _c2_, ..., _cn_ are collections that have an _iterator_ method. A cartesian product of the collections is taken and the predicate is checked for each combination of the variables. If the predicate is true for **all** the combinations, then the forall macro is true.

- `exists |v1 in c1, v2 in c2, ..., vn in cn| <predicate>(v1, v2, ..., vn)`
    - **SMT and Rust**: existentially quantified over bounded collections
    - In Rust, the `<predicate>` are checked in a loop
      iterating over all possible combination of variables `v1, v2, ..., vn`.

Basically, the _exists_ macro has one form:
    - `exists! (v1 in c1, v2 in c2, ..., vn in cn => <predicate>)`

The _c1_, _c2_, ..., _cn_ are collections that have an _iterator_ method. A cartesian product of the collections is taken and the predicate is checked for each combination of the variables. If the predicate is true for **any** of the combinations, then the exists macro is true.

- `choose |v1 in c1, v2 in c2, ..., vn in cn| <predicate>(v1, v2, ..., vn)`
    - **SMT and Rust**: choose operator over bounded collections
    - In Rust, one set of variables `v1, v2, ..., vn`
      that satisfies `<predicate>` will be returned
      by iterating over all possible combinations.
      If no such combination exists, exit with panic.
    - In SMT, variables `v1, v2, ..., vn` will be defined in an axiomatized way.

Basically, the _choose_ macro has one form:
      - `choose! (v1 in c1, v2 in c2, ..., vn in cn => <predicate>)`

The _c1_, _c2_, ..., _cn_ are collections that have an _iterator_ method. A cartesian product of the collections is taken and the predicate is checked for the combinations of the variables in order. The first combination that satisfies the predicate is returned. If no such combination exists, the program panics.

The combination of these expression macros allows us to express complex logic for example getting the minimum value from a set of values as shown below:

```rust
set! { 1, 2, 3, 4, 5 }
choose! (x in set => forall! (y in set => x.lt(y).or(x.eq(y))))
```

#### Note on the `SMT` trait

As you can see for the _SMT_ trait, the supertraits are __'static, Copy, Default, Hash, Send, Sync__. The _'static_ lifetime is used to indicate that the data type is valid for the entire duration of the program. The _Copy_ trait is used to indicate that the data type can be copied by value as SMT types are not passed by reference. The _Default_ trait is used to allow quantified expressions to type-check in the Rust type system. This has been removed since, but the implementation of _Default_ has been kept. The _Hash_ trait is used to allow SMT values to be used as keys in ordered collections like _HashSet_ or _HashMap_. The _Send_ and _Sync_ traits are used to allow SMT values to be sent across threads. The _Send_ trait indicates that ownership of values of the type implementing _Send_ can be transferred between threads. The _Sync_ marker trait indicates that it is safe for the type implementing _Sync_ to be referenced from multiple threads. As you can see, the _SMT_ trait does not implement the _Ord_ trait, even though SMT types should be comparable. This is because for the _Ord_ trait to be a supertrait, the traits _Eq_, _PartialEq_, and _PartialOrd_ should also be supertraits. The predefined functions of _ne_ and _eq_ of the PartialEq trait will both have the types _(&self, other: &Rhs) -> bool_, whereas for these functions we will want the types to be _(self, other: Rhs) -> Boolean_. Note that _Boolean_ is our self defined type that wraps the Rust boolean type and that the return types of all functions in _rusmart_ should be a type defined in _rusmart_. We could have used the _eq_ for example, but needed to wrap it inside `Boolean::from`. This clutters the code and makes it less readable. Therefore, we have decided to not implement the _Ord_ trait as a supertrait of the _SMT_ trait. For this reason, we have defined the method _cmp_ that returns an _Ordering_ enum and any type implementing the _SMT_ trait should define this method. The methods _ne_ and _eq_ are also defined in the _SMT_ trait and they use the _cmp_ method to compare two values of the type implementing the _SMT_ trait. This way SMT types can be compared and checked for equality and non-equality.