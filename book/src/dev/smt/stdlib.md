### RuSmt Standard Library

---

## Stdlib Semantics

Every stdlib function has **two sides** that must agree:

1. **The Rust implementation** — runs concretely when you execute your interpreter.
2. **The Z3 translation** — the SMT-LIB formula emitted when transpiling to Z3.

The core soundness requirement is:

> **For every stdlib function `f` and every concrete input `x`:**
> ```
> rust_impl(f, x) == z3_eval(z3_formula(f), x)
> ```
> That is, evaluating `f` concretely in Rust must yield the same result as evaluating Z3's formula for `f` on the same concrete value (or the encoded z3-version of that concrete value).

This matters because the workflow is:
1. You write an interpreter for a target language using the DSL.
2. The DSL is transpiled to Z3.
3. Z3 finds **models** — concrete inputs that trigger specific path conditions.
4. Those models are fed back into the **Rust interpreter** to generate conformance test cases.

If the Rust side and the Z3 side disagree, a model that Z3 finds _satisfies condition C_ may not actually hit the path condition corresponding to the condition _C_ when run concretely — producing **unsound test cases**.

> Note that we do not claim completeness. In other words, even if an input that reaches condition C exists, Z3 may fail to find it: the encoded formula can fall outside what Z3 decides within its resource limits, so the solver returns unknown or times out.

### Adapting stdlib operations to your target language

The stdlib mostly represents **Z3's mathematical theories** — not Rust's native semantics, and not the semantics of any specific target language. WebAssembly, Python, C, and JavaScript all differ in their overflow, rounding, and division rules, and the stdlib cannot model all of them at once. Bridging the stdlib to a given target language is the **interpreter author's responsibility**. In some cases the stdlib's behavior also diverges from Z3's *native* operator — where Z3's default would be unintuitive for our
purposes. In every such case the backend emits a **custom Z3 encoding** rather than the native operator, so the Rust frontend and the Z3 backend always compute the same result. Before writing any interpreter using the stdlib types and functions, please familiarize yourself with the internal workings of each of the operations in the standard library. So far we have not encountered a case where the stdlib lacks an operation needed to replicate a target  language's behavior — though this may change as more 
languages are added. If you hit such a gap, please [email the maintainers](mailto:m3haghsh@uwaterloo.ca) or open a pull request adding the missing operation.

We in general have three patterns:

> 1. When a target language operation matches a stdlib primitive exactly, use it directly:

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

> 2. When a target language semantics only diverges for **specific inputs** (e.g., error cases, overflow, NaN), use **guard branches** to handle the diverging cases and fall through to the stdlib for the rest:

```rust
fn wasm_i32_div_s(a: I32, b: I32) -> Result<I32, Path> {
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

> 3. When no single stdlib primitive matches, compose multiple primitives:

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

### Input validation contract

Some stdlib functions have preconditions on their inputs. For example, `Integer::from_hex_str`
expects a string of pure hex digits — no `0x` prefix, no underscores. On invalid input:

- **Rust** panics (`unwrap()` on a failed parse).
- **Z3** silently returns a wrong value (e.g., 0 for an unrecognized character).

These behaviors diverge, but this does NOT break soundness because we make the assumption that the author of the the interpreter must validate inputs before calling stdlib functions. The TOML parser, for example, checks each character with `is_hex_digit` and strips prefixes/underscores before the string ever reaches `from_hex_str`. Invalid inputs are caught by the parser and produce a named marker (`Path::named(String::from("..."))`) — they never reach the stdlib.

**Rule for interpreter authors:** if a stdlib function can panic on certain inputs, your
interpreter must guard against those inputs explicitly. Do not rely on the stdlib to handle
invalid inputs gracefully — it is not designed to. The stdlib assumes its inputs satisfy
the documented preconditions. If you violate them, Rust panics and Z3 gives garbage.

### Integer — functions that panic

The following `Integer` methods panic on certain inputs. When writing an interpreter, we guard against these inputs before calling the method (e.g., checking divisor != 0 before calling `div`). On the unguarded path, we use the method as-is. On the guarded path, the panic is unreachable and we must replicate the behavior of the target language:

- If the target language treats the operation as an error — division by zero, for instance, is a runtime error in most languages — place a named marker `Path::named(String::from("division_by_zero"))` in the guard branch. This marks the branch as a synthesis target: Z3 will search for an input that drives the program into that error case. The marker name is a stdlib `String` (written with the DSL string-literal idiom `String::from("...")`); its stable id (`marker_id(name)`) is what lets a synthesized input be replay-certified to reach *this specific* marker. If you don't want Z3 to synthesize such inputs (say the error case is uninteresting for testing), just handle the branch normally — return a sentinel or default value — and omit the marker. The branch still executes correctly under concrete Rust; it simply isn't used as a synthesis goal.

- If the target language defines specific behavior for that input (e.g., saturating, wrapping, returning a default), we implement that behavior using other stdlib operations in the guard branch.

In either case, the guard branch handles the diverging input before the stdlib method is reached, so the panic never fires. If the interpreter does NOT guard, Rust panics (crashes) while Z3 silently returns a wrong value. Any models Z3 finds on that path are unsound because the Rust side never reaches the return — it crashes instead.

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
- `index_of_default(substr)` — panics when substr is not found.
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
- `pow(exp)` — see [Real.pow vs SMT-LIB `^`](#realpow-vs-smt-lib-) below
- `to_f32()` — panics when value is too large for f32 but is finite in f64 (roughly `1.8 * 10^308 > |value| > 3.4 * 10^38`)

#### `Real.pow` vs SMT-LIB `^`

SMT-LIB `^` is **real** exponentiation, not integer power: Z3 evaluates a
fractional exponent rather than rejecting it. `Real.pow` therefore computes
fractional exponents too, exactly, as `x^(p/q) = (x^p)^(1/q)` on the reduced
rational.

`(a/b)^(1/q)` is rational exactly when `a` and `b` are each a perfect `q`-th
power, so representability is decided per side by `exact_nth_root`, which takes
the integer floor of the root and squares back to check.

Three cases have no `Real` to return, and all three **panic**:

- **Irrational result** — `(^ 2.0 2.5)` simplifies to
  `(root-obj (+ (^ x 2) (- 32)) 2)`, i.e. `sqrt(32)`. Z3 works over the
  algebraic numbers; `BigRational` is not closed under roots.
- **Negative base with an even root** — `(^ (- 2.0) 0.5)` has no real value;
  Z3 echoes the term back unsimplified and pinning it to a number returns
  `unknown`.
- **`0 ^ 0`** — Z3 leaves `(^ 0.0 0.0)` uninterpreted.

Every *other* member of the `0^x` family is `0` on both sides — positive or
negative, integer or fractional:

| expression | Z3 | `Real.pow` |
|---|---|---|
| `0 ^ 2`, `0 ^ 1`, `0 ^ 0.5`, `0 ^ (1/3)` | `0.0` | `0` |
| `0 ^ -0.5`, `0 ^ -1`, `0 ^ -2` | `0.0` | `0` |

For the negative exponents Z3 is *committed*, not merely permissive:
`(= (^ 0.0 (- 1.0)) 0.0)` is **sat** and `(= .. 1.0)` is **unsat**. That value
is Z3's division-by-zero convention rather than mathematics, but matching it
keeps the two semantics in step — and it is also why `Real.pow` short-circuits
the zero base before reaching `BigRational::pow`, which would otherwise build a
zero denominator and panic.

`p` outside `i32` or `q` outside `u32` also panics; see
[Theoretical limitations](#theoretical-limitations-unguarded-edge-cases).

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

### Float — NaN behavior notes

- `is_negative()` / `is_positive()` — guarded with `!is_nan()` to match Z3. Rust's `is_sign_negative` returns true for `-NaN`, Z3's `fp.isNegative` returns false for all NaN. Without the guard, soundness breaks.
- `rem(rhs)` — uses `libm::remainderf`/`libm::remainder` (IEEE 754 remainder), NOT Rust's `%` which is fmod. Z3's `fp.rem` matches IEEE 754.
- `nearest()` — custom implementation for ties-to-even. Rust's `f64::round()` uses ties-away-from-zero, Z3's RNE uses ties-to-even.

### Set — unsupported binary operations

Z3 does not support binary operations (such as `set.inter`, `set.union`, `set.setminus`) in SMT-LIB2 mode; these exist only in Z3's programmatic API (Python/C++). We have therefore omitted supporting operations like intersection, union, difference, and symmetric difference entirely. If your interpreter needs any of them, follow the [Adding a new intrinsic-backed method](../../user/methods.md#adding-a-new-intrinsic-backed-method-what-files-change) section.

  > Note that `union` / `intersection` / `difference` /
  `symmetric_difference`
  > *could* be expressed as 
  array operations
  > (`(_ map or)`, `(_ map and)`, and so on). The problem 
  is cardinality: Z3
  > cannot count how many elements are `true` in the 
  resulting array.
  > Axiomatizing the cardinality of a combined set 
  requires universal
  > quantifiers, and those quantifiers add enough overhead
   to make Z3 time out.
  > By contrast, `SetLen` is tracked *exactly* for 
  step-by-step construction
  > (`new` / `insert` / `remove`) by pairing the 
  membership array with an
  > integer counter (the `RuSmtSet` datatype) — no 
  quantifiers, no performance
  > cost.

### Array — functions that panic

- `select(key)` — panics when key does not exist. Z3 returns the null sentinel value. Use `contains_key` to guard first.

### Theoretical limitations (unguarded edge cases)

The following cases are not explicitly guarded in the TOML interpreter because they require pathological inputs that are impractical in real-world usage. They are documented here for completeness.

- **`Real.pow(exp)` with digit-count exponent**: In the TOML float parser (`float.rs`), expressions like `Real::from(10).pow(number_of_digits(val).neg().to_real())` use the digit count of a parsed number as the exponent. `Real.pow` converts the exponent's numerator to `i32` (and, for a fractional exponent, its denominator to `u32`), which panics if either exceeds range. The digit count is not explicitly bounded — a TOML file containing a number with more than 2,147,483,647 digits (~2 GB of digits alone) would trigger this panic. In practice, the parser would exhaust memory long before reaching this limit. No guard is added because the input size makes this an undesirable test case in any realistic scenario. (These exponents are always integral, coming from `Integer::to_real()`, so the fractional path is never taken here.)

- **`Integer.pow(exp)` with digit-count exponent**: Similarly, `Integer::from(10).pow(number_of_digits(val))`in the float parser uses the digit count as an exponent for `Integer.pow`, which requires the exponent to fit in `u32`. A number with more than 4,294,967,295 digits (~4 GB) would be needed to trigger this. Same practical impossibility applies.

### String encoding: ASCII-only soundness

The stdlib `String` operations (`length`, `at`, `substr`, `index_of`, etc.) count Unicode code points (via Rust's `.chars()`). Z3's string theory counts UTF-8 bytes. For ASCII (code points 0-127), one code point equals one byte so they agree. For non-ASCII, they diverge:

| Input | Rust (code points) | Z3 (bytes) |
|-------|-------------------|------------|
| `"Hello"` | 5 | 5 |
| `"é"` (U+00E9) | 1 | 2 |
| `"😀"` (U+1F600) | 1 | 4 |

This affects all position-based operations (`at`, `substr`, `index_of`) since indices refer to different units. The soundness invariant holds only for ASCII input. Changing the Rust side to byte-based is not desirable: Z3's `str.at` can return individual bytes of multi-byte UTF-8 sequences (e.g., `(str.at "😀" 0)` returns `\xF0`), which cannot be stored in a Rust `String` (requires valid UTF-8).

For interpreter authors: restrict string inputs to ASCII for soundness.

#### Z3's character range stops at `0x2FFFF`

Separately from the byte/code-point split above, Z3's character sort cannot
represent every Unicode scalar value. Measured on 4.15.4:

| code point | `str.to_code` | `str.len (str.from_code cp)` | Rust `String::from_code` |
|---|---|---|---|
| `0x1F600` | `128512` | `1` | 1 char |
| `0x2FFFF` | `196607` | `1` | 1 char |
| `0x30000` | `-1` | `0` | 1 char |
| `0x10FFFF` | `-1` | `0` | 1 char |

Above `0x2FFFF`, `str.from_code` yields the **empty** string while the stdlib's
`String::from_code` (`char::from_u32(..).unwrap()`) yields a one-character
string — so `length()` disagrees immediately. This is reachable: the TOML
front-end feeds input code points straight into `String::from_code`.

The transpiler therefore bounds `U32` / `Seq<U32>` inputs to
`[0x0, 0xD7FF] ∪ [0xE000, 0x2FFFF]` (`backend/z3/ctxt.rs::unicode_bound_for`).
Note the two exclusions differ in kind:

- Surrogates are not scalar values at all, so no concrete `char` produces them —
  excluding them costs nothing.
- The `0x2FFFF` ceiling **does** cost completeness: planes 3–16 are valid
  `char`s that will never be proposed as inputs. "No input found" for a target
  does not rule out one that needs a code point above `0x2FFFF`.

---

RuSmt standard library (_stdlib_) contains language constructs that cannot be expressed readily in Rust as they have special semantics in SMT. The _rusmt-smt-stdlib_ package consists of one _library crate_. The crate contains two modules: `dt` and `exp`. The `dt` module contains data types part of the type system in RuSmt, while the `exp` module contains expressions. Both modules are re-exported in the root of the crate to allow users to use data types and expressions directly.

### Trait
`SMT`: marks that a Rust type is also an SMT type. In order to bridge the semantic gap between a Rust type and an SMT type, the `SMT` trait encodes several restrictions.
-  Rust types implementing the `SMT` trait must implement other traits (these are the supertraits of `SMT`): the supertraits are __'static, Copy, Default, Hash, Send, Sync__. 
  - The _'static_ lifetime is used to indicate that the data type is valid for the entire duration of the program. 
  - The _Copy_ trait is used to indicate that the data type can be copied by value as SMT types are not passed by reference. 
  - The _Default_ trait is used to allow quantified expressions to type-check in the Rust type system. This has been removed since, but the implementation of _Default_ has been kept. 
  - The _Hash_ trait is used to allow SMT values to be used as keys in ordered collections like _HashSet_ or _HashMap_. 
  - The _Send_ trait indicates that ownership of values of the type implementing _Send_ can be transferred between threads. 
  - The _Sync_ marker trait indicates that it is safe for the type implementing _Sync_ to be referenced from multiple threads. 

> As you can see, the _SMT_ trait does not implement the _Ord_ trait, even though SMT types should be comparable. This is because for the _Ord_ trait to be a supertrait, the traits _Eq_, _PartialEq_, and _PartialOrd_ should also be supertraits. The predefined functions of _ne_ and _eq_ of the PartialEq trait will both have the types _(&self, other: &Rhs) -> bool_, whereas for these functions we will want the types to be _(self, other: Rhs) -> Boolean_. Note that _Boolean_ is our self defined type that wraps the Rust boolean type and that the return types of all functions in _rusmt_ should be a type defined in _rusmt_. We could have used the _eq_ for example, but needed to wrap it inside `Boolean::from`. This clutters the code and makes it less readable. Therefore, we have decided to not implement the _Ord_ trait as a supertrait of the _SMT_ trait. 

We have defined the method _cmp_ that returns an _Ordering_ enum and any type implementing the _SMT_ trait should define this method. The methods _ne_ and _eq_ are also defined in the _SMT_ trait and they use the _cmp_ method to compare two values of the type implementing the _SMT_ trait. This way SMT types can be compared and checked for equality and non-equality. The `ne` and `eq` methods have default implementations that use the `_cmp` method. The `eq` method returns true if the `_cmp` method returns `Equal`. The `ne` method returns true if the `_cmp` method returns `Less` or `Greater`. The `_cmp` method is required to be defined by the type implementing the `SMT` trait.

### Data types

These data types are part of the [type system](../../../user/typing.md) in RuSmt:

- `Boolean`: A wrapper around the Rust boolean type. The definition of the `Boolean` type is as follows:

```rust
pub struct Boolean {
    inner: bool,
}
```
Note that this approach of wrapping inside a struct with an `inner` field is a common way in the libraries of Rust itself and this approach is used in the RuSmt standard library as well.

- `Integer`: Unbounded integer (backed by `num_bigint::BigInt`).
- `Real`: Unbounded rational (backed by `num_rational::BigRational`).
- `I32`, `I64`, `U32`, `U64`: Bitvectors (SMT-LIB `(_ BitVec 32)` / `(_ BitVec 64)`), with signed/unsigned *interpretations* in the DSL API.
- `F32`, `F64`: Floating-point sorts (SMT-LIB `(_ FloatingPoint 8 24)` / `(_ FloatingPoint 11 53)`).
- `String`: A wrapper around Rust `String`, corresponding to SMT-LIB `String`.
- `Cloak<T>`: A **frontend-only** wrapper over `T` that lets users write self-referential ADTs without violating Rust's sized-type requirement (similar to `Box<T>` in Rust). The IR strips it: every `Cloak<T>` field becomes a plain `T` field, every `Cloak::shield(x)` lowers to `x`, every `.reveal()` lowers to identity. As a result the SMT-LIB output never mentions `Cloak`. The IR assumes every `Cloak<T>` appears alongside at least one non-recursive variant (i.e. the enum is well-founded).
- `Seq<T>`: SMT sequence of type `T` similar to Rust `Vec<T>`.
- `Set<T>`: SMT set of type `T` similar to Rust `BTreeSet<T>`.
- `Array<K, V>`: SMT array of key type `K` and value type `V`, similar to a persistent `BTreeMap<K, V>` in the interpreter.
- `Path`: A **set-valued** marker indicating the path conditions reached during execution. A named path marker is created with `Path::named(name)`, where `name` is a stdlib `String` (written with the DSL idiom `Path::named(String::from("..."))`); its integer id is the stable hash `marker_id(name)`, identical in the transpiled SMT query and on concrete replay — which is what makes per-target replay certification sound. Markers are unioned with `Path::merge(a, b)` to accumulate multiple errors for *graceful*, non-short-circuiting error handling. Concretely a `Path` is a set of marker ids; the SMT search encodes it as a single representative id (`Int`) for decidability (see `book/src/dev/smt/derive.md`).

### Expressions

There are three quantified expressions: `forall!` / `exists!` / `choose!`, each in a _bounded_ form.

> **Bounded** — quantify over the elements of one or more collections. The collections must implement an `iterator()` method (every `Seq` / `Set` / `Array` / `String` does). Combining macros lets us write things like “the minimum value of a set”: 
> ```rust
> set! { 1, 2, 3, 4, 5 }
> choose! (x in set => forall! (y in set => x.lt(y).or(x.eq(y))))
> ```

### Bounded form

- `forall!(v1 in c1, ..., vn in cn => <predicate>)`
    - **SMT and Rust**: universally quantified over the cartesian product of the collections.
    - In Rust the predicate is checked in a loop over `c_i.iterator()`; in SMT it is gated on membership in `c_i`.
- `exists!(v1 in c1, ..., vn in cn => <predicate>)`
    - **SMT and Rust**: existentially quantified over the cartesian product.
    - In Rust the predicate is checked in a loop and returns `true` on the first match.
- `choose!(v1 in c1, ..., vn in cn => <predicate>)`
    - **SMT and Rust**: Hilbert choice over the cartesian product.
    - **In Rust**, `choose` returns the *first* witness satisfying the predicate; if none exists, the program panics. Guard any use of
  `choose` with an existence check beforehand.
  - **In SMT**, the witness is axiomatized via a Skolem function plus the choose axiom, so Z3 returns an *arbitrary* value satisfying
  the predicate — not necessarily the first but this does not affect soundness: we assume only that the witness *satisfies the predicate*, never that a particular one is chosen. Any satisfying witness is equally valid.