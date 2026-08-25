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

### The agreement invariant

For every stdlib operation and every input, one of two things holds:

1. the Rust value and the Z3 value are **equal**, or
2. the Rust side **panics**, because Z3 has no value to agree with — the SMT-LIB
   term is unconstrained (`(div 1 0)` is satisfiable equal to 5 *and* to 77), or
   it leaves the sort (`(^ 2.0 0.5)` is an algebraic number), or Rust cannot
   represent what Z3 admits (a lone surrogate is not a `char`).

There is no third case: no operation returns a value that disagrees with Z3.
Anything else would be unsound — Z3 would synthesise a witness for a path that
concrete execution does not take, and the generated test would not test what it
claims. A panic is not "safe" either (see the rule below); it is a refusal, and
every one of them is a proof obligation on the interpreter author, so the stdlib
keeps them to the minimum the semantics force.

**This is not mechanically checked in this tree.** A differential suite —
running every operation in Rust, posing the term the backend emits to Z3, and
asking whether the two can differ — would establish it, and building one is the
obvious next step. Until then the correspondence is a design obligation
discharged by construction and review, not a measured result. See
[Stdlib design](stdlib-design.md).

**Rule for interpreter authors:** a stdlib function that can panic must be
guarded by your interpreter. A panic is not a safe fallback: if the model reaches
that path, replay crashes instead of returning, and any witness Z3 found for it
is worthless.

### Integer — functions that panic

Only where Z3 has no value to agree with. `div`/`mod` by zero is *uninterpreted*
in SMT-LIB — `(= (div 1 0) 5)` and `(= (div 1 0) 77)` are both satisfiable — so
there is nothing to return.

- `div(rhs)`, `div_trunc(rhs)`, `modulo(rhs)`, `rem(rhs)` — panic when `rhs == 0`
- `divides(rhs)` — panics when `self == 0` (it is `(= (mod rhs self) 0)`)
- `pow(exp)` — panics on `0^0`, which Z3 leaves unconstrained, and on an exponent
  beyond `u32`, which Z3 handles but `BigInt::pow` cannot take (`2^(2^32)` is some
  1.3 billion digits). A **negative** exponent no longer panics: `^` is Real-sorted
  in Z3 even for `Int` arguments, so the backend emits `(to_int (^ a b))`, and the
  stdlib returns the same floor — `2^-1` is `0`, `(-2)^-1` is `-1`, `1^-5` is `1`,
  and `0^-1` is `0` because Z3 reads `(^ 0 e)` as `0.0` rather than a division by
  zero. The `to_int` also keeps the term usable where an `Int` is required:
  a bare `(^ 2 3)` prints `8.0` and `str.at` rejects it.

Everything else is total and agrees with Z3, including the cases that used to
panic:

- `to_i32()` / `to_i64()` / `to_u32()` / `to_u64()` — `(_ int2bv N)` is total and
  takes the value modulo `2^N`, so these **wrap** rather than panic
- `to_f32()` / `to_f64()` — out-of-range values round to `±oo`, matching
  `((_ to_fp ..) RNE (to_real x))`
- `from_hex_str(s)` / `from_oct_str(s)` / `from_bin_str(s)` — Z3 folds the string
  left and maps every character that is not a digit of the radix to zero. There is
  no sign, no prefix and no failure: `from_hex_str("-ff")` is `255`, and
  `from_hex_str("")` is `0`. Validate the string yourself if your language cares.

### String — code points

Both sides count Unicode code points: Rust via `chars()`, Z3 over its string
alphabet `U+0000..U+2FFFF`. The backend emits every string literal with each
non-ASCII character, `"` and `\` escaped as `\u{..}`, because Z3's lexer reads a
raw non-ASCII byte as one character and interprets `\u{..}`/`\uXXXX` inside
literals — unescaped, a literal would denote a different string. Together with
the `U+2FFFF` guard on `from_code`, no character outside Z3's alphabet can enter
a query.

### String — functions that panic

- `from_code(code)` — panics only on a surrogate (`0xD800..=0xDFFF`). Z3 admits
  one as a character; Rust's `char` cannot hold it, so there is no value to
  return. Outside `[0, 0x2FFFF]` it returns `""`, as `str.from_code` does.

Every other `String` operation is total and agrees with Z3. Note the conventions
this inherits from the SMT-LIB theory:

- `at(i)`, `substr(off, len)` — `""` when the index or range is out of bounds
- `index_of(sub, off)` — `-1` when `off` is outside `[0, len]` or the needle is
  not found; `off == len` is in range and finds the empty needle there
- `to_int()` — `-1` unless the string is a non-empty run of ASCII digits, so
  `"-5"` is `-1`, not `-5`
- `from_int(i)` — `""` for negative `i`
- `to_code()` — `-1` unless the string is exactly one character

### Seq — functions that panic

- `at(index)` — panics when the index is out of bounds. `seq.nth` is unconstrained
  off the end (satisfiable equal to any value), so there is nothing to return.

`at_seq`, `extract` and `index_of` are total and mirror the String theory exactly:
the empty sequence for an out-of-range extract, `-1` for a missing subsequence,
and `off == len` in range for the empty needle.

### Real — functions that panic

- `div(rhs)` — panics when `rhs == 0`; `(/ 1.0 0.0)` is unconstrained in Z3
- `pow(exp)` — see [Real.pow vs SMT-LIB `^`](#realpow-vs-smt-lib-) below

`to_f32()` / `to_f64()` are total: an out-of-range magnitude rounds to `±oo`,
matching Z3.

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

### Bitvector — no panics

Every bitvector operation is total and agrees with Z3 on all 3,838 tested cases.
The SMT-LIB conventions worth knowing, since they are not Rust's:

- `bv_div(0)` — `(bvudiv x 0)` is the all-ones vector; `(bvsdiv x 0)` is `-1` for
  non-negative `x` and `1` otherwise
- `bv_rem(0)`, `bv_mod(0)` — both return the dividend unchanged
- signed `MIN / -1` wraps to `MIN`, matching `bvsdiv`
- shifts of `>=` the bit width give `0` (`bvashr` sign-extends instead)

### Float — functions that panic

All of these are unconstrained in Z3 — each was confirmed satisfiable against two
different values — so there is nothing to agree with:

- `to_integer()`, `to_real()` — on NaN or infinity
- `to_i32()`, `to_i64()`, `to_u32()`, `to_u64()` — on NaN, infinity, or a value
  outside the target range (`fp.to_sbv` / `fp.to_ubv` are partial)
- `min(rhs)`, `max(rhs)` — only when the two arguments are zeros of **opposite
  sign**: SMT-LIB leaves `(fp.min +zero -zero)` unspecified, and Z3 does not
  reduce it. Every other `min`/`max` pair, NaN included, agrees.

### Float — NaN behavior notes

- `is_negative()` / `is_positive()` — guarded with `!is_nan()` to match Z3. Rust's
  `is_sign_negative` returns true for `-NaN`; `fp.isNegative` is false for all NaN.
- `rem(rhs)` — uses `libm::remainderf` / `libm::remainder` (IEEE 754 remainder),
  not Rust's `%`, which is fmod. Z3's `fp.rem` is the IEEE one.
- `nearest()` — `f32::round_ties_even` / `f64::round_ties_even`, which is IEEE
  roundTiesToEven and keeps the sign of a zero result: `RNE(-0.5)` is `-0.0`.
  Rust's `round()` is ties-away-from-zero and loses that sign.

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

### Array — no panics

- `select(key)` — returns `V::default()` when the key is absent, matching the term
  the backend writes into every empty slot (`array_null_value`). The two are kept
  in step deliberately: for an enum, `array_null_value` builds the **first variant
  in declaration order**, because that is the one `#[smt_type]` generates
  `Default::default()` from. Pick any other variant and a concrete `V::default()`
  would silently disagree with what Z3 reads.

  The value carries no meaning — membership lives in `rarr-pres`, not in the
  value slot — so read it only after `contains_key`. But reading it early is now
  a bug in your interpreter, not a divergence between the two sides.

### Theoretical limitations (unguarded edge cases)

The following cases are not explicitly guarded in the TOML interpreter because they require pathological inputs that are impractical in real-world usage. They are documented here for completeness.

- **`Real.pow(exp)` with digit-count exponent**: In the TOML float parser (`float.rs`), expressions like `Real::from(10).pow(number_of_digits(val).neg().to_real())` use the digit count of a parsed number as the exponent. `Real.pow` converts the exponent's numerator to `i32` (and, for a fractional exponent, its denominator to `u32`), which panics if either exceeds range. The digit count is not explicitly bounded — a TOML file containing a number with more than 2,147,483,647 digits (~2 GB of digits alone) would trigger this panic. In practice, the parser would exhaust memory long before reaching this limit. No guard is added because the input size makes this an undesirable test case in any realistic scenario. (These exponents are always integral, coming from `Integer::to_real()`, so the fractional path is never taken here.)

- **`Integer.pow(exp)` with digit-count exponent**: Similarly, `Integer::from(10).pow(number_of_digits(val))`in the float parser uses the digit count as an exponent for `Integer.pow`, which requires the exponent to fit in `u32`. A number with more than 4,294,967,295 digits (~4 GB) would be needed to trigger this. Same practical impossibility applies.

### String encoding

The stdlib's `String` operations count Unicode **code points** (Rust's `.chars()`).
So does Z3's string theory — its alphabet is code points, not UTF-8 bytes.

What used to break this was the *script*, not the theory: the backend wrote string
literals verbatim, and Z3's lexer reads a raw non-ASCII byte as one character, so
`"é"` arrived as a two-character string. Worse, SMT-LIB interprets `\u{..}` and
`\uXXXX` *inside* literals, so a backslash in the data could start an escape that
was never in the value — `"\u0041"` arrived as `"A"`.

`format_str_literal` (`backend/z3/intrinsics.rs`) now emits printable ASCII
verbatim and everything else — `"`, `\`, control characters, all non-ASCII — as a
`\u{..}` escape. Measured against Z3 4.15.4:

| Rust value | emitted | Z3 `str.len` | Rust `chars().count()` |
|---|---|---|---|
| `Hello` | `"Hello"` | 5 | 5 |
| `é` | `"\u{e9}"` | 1 | 1 |
| `😀` | `"\u{1f600}"` | 1 | 1 |
| `\u0041` | `"\u{5c}u0041"` | 6 | 6 |

Interpreter authors no longer need to restrict input to ASCII. The one remaining
limit is the alphabet ceiling below.

#### Z3's character range stops at `0x2FFFF`

Separately from the byte/code-point split above, Z3's character sort cannot
represent every Unicode scalar value. Measured on 4.15.4:

| code point | `str.to_code` | `str.len (str.from_code cp)` | Rust `String::from_code` |
|---|---|---|---|
| `0x1F600` | `128512` | `1` | 1 char |
| `0x2FFFF` | `196607` | `1` | 1 char |
| `0x30000` | `-1` | `0` | `""` |
| `0x10FFFF` | `-1` | `0` | `""` |

Where that ceiling falls in Unicode:

| range | plane | what lives there | Z3 | Rust |
|---|---|---|---|---|
| `U+0000`–`U+D7FF` | 0 (start) | ASCII, Latin, Greek, Cyrillic, CJK — most everything | yes | yes |
| `U+D800`–`U+DFFF` | 0 | surrogates — not characters at all | yes | **no** |
| `U+E000`–`U+FFFF` | 0 (rest) | private use, CJK compatibility | yes | yes |
| `U+10000`–`U+1FFFF` | 1 | emoji (😀 = `U+1F600`), ancient scripts | yes | yes |
| `U+20000`–`U+2FFFF` | 2 | rare CJK ideographs | yes | yes |
| `U+30000`–`U+10FFFF` | 3–16 | CJK extensions G/H, tag chars, private use | **no** | yes |

Above `0x2FFFF`, `str.from_code` yields the **empty** string, and
`String::from_code` now does the same, so the two agree. (It used to build the
character, which disagreed with Z3 immediately at `length()` — reachable, since
the TOML front-end feeds input code points straight into `String::from_code`.)
Surrogates are the one case Rust cannot mirror: Z3 admits one as a character and
`char` cannot hold it, so `from_code` panics there.

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
- `Path`: A **set-valued** marker indicating the path conditions reached during execution. A named path marker is created with `Path::named(name)`, where `name` is a stdlib `String` (written with the DSL idiom `Path::named(String::from("..."))`); its integer id is the stable hash `marker_id(name)`, identical in the transpiled SMT query and on concrete replay — which is what makes per-target replay checking sound. Markers are unioned with `Path::merge(a, b)` to accumulate multiple errors for *graceful*, non-short-circuiting error handling. Concretely a `Path` is a set of marker ids; the SMT search encodes it as a single representative id (`Int`) for decidability (see `book/src/dev/smt/derive.md`).

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
