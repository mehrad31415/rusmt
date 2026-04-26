# Rego Subset — Error Index

Each `Error::fresh()` marks a distinct _path condition_ that the SMT solver
can target. Format: `N) function - path condition`.

The index covers parser-level synthesis targets (in the `parse_*` functions)
and evaluator-level synthesis targets (in `eval_*` and `apply_arith` — these
encode genuine semantic edge cases per the spec).

---

## literal.rs (11 errors)

### parse_boolean

1) parse_boolean - Boolean literal uses title-case `True`; Rego is case-sensitive, only `true` is valid.
2) parse_boolean - Boolean literal uses all-uppercase `TRUE`; only `true` is valid.
3) parse_boolean - Boolean literal uses title-case `False`; only `false` is valid.
4) parse_boolean - Boolean literal uses all-uppercase `FALSE`; only `false` is valid.

### parse_number

5) parse_number - Sign `-` followed by a non-digit / end-of-input where digits were expected.
6) parse_unsigned_number - Decimal point `.` followed by end-of-input (no fractional digits).
7) parse_unsigned_number - Decimal point `.` followed by a non-digit character.

### parse_string / parse_string_body

8) parse_string_body - End of input before the closing `"`.
9) parse_string_body - Literal newline character inside a single-line string (newlines in strings are not allowed).
10) parse_string_body - Trailing backslash with no escape character following.
11) parse_string_body - Unrecognized escape sequence (character after `\` is not one of `"`, `\`, `n`, `t`, `r`).

---

## term.rs (28 errors)

### parse_additive_tail / parse_multiplicative_tail

12) parse_additive_tail - `+` operator with no right-hand operand.
13) parse_additive_tail - `-` operator with no right-hand operand.
14) parse_multiplicative_tail - `*` operator with no right-hand operand.
15) parse_multiplicative_tail - `/` operator with no right-hand operand.

### parse_paren_term

16) parse_paren_term - Missing `)` after parenthesized term (junk character found).
17) parse_paren_term - End of input where `)` is expected.

### parse_array_lit / parse_array_elems

18) parse_array_lit - End of input immediately after `[` array open; no `]` ever found.
19) parse_array_elems - Expected an array element but no term matched.
20) parse_array_elems - End of input mid-array (expected `,` or `]`).
21) parse_array_elems - Input ended after `,` separator (no element or `]` follows).
22) parse_array_elems - `|` found where `,` or `]` expected — array comprehensions are out of scope.
23) parse_array_elems - Other junk character between array elements.

### parse_object_lit / parse_object_kvps

24) parse_object_lit - End of input immediately after `{`; object body never terminated.
25) parse_object_kvps - Expected an object key but none matched.
26) parse_object_kvps - End of input where `:` separator is expected after a key.
27) parse_object_kvps - `:` not followed by a value term.
28) parse_object_kvps - Duplicate key in object literal.
29) parse_object_kvps - End of input mid-object (no `}` ever found).
30) parse_object_kvps - Input ended after `,` (no kvp or `}` follows).
31) parse_object_kvps - `|` found where `,` or `}` expected — object comprehensions are out of scope.
32) parse_object_kvps - Other junk character between object key/value pairs.
33) parse_object_kvps - Object key not followed by `:`.

### parse_set_lit / parse_set_elems

34) parse_set_lit - End of input immediately after `set(`; closing `)` never found.
35) parse_set_elems - Expected a set element but no term matched.
36) parse_set_elems - End of input mid-set (no `)` ever found).
37) parse_set_elems - Input ended after `,` separator (no element or `)` follows).
38) parse_set_elems - Other junk character between set elements.

### parse_ref_segments

39) parse_ref_segments - Trailing `.` in a dotted reference (no segment after the dot).

---

## expr.rs (10 errors)

### parse_expr

40) parse_expr - `not` keyword without a following expression.

### parse_assignment

41) parse_assignment - `:=` operator not followed by a term.

### parse_compare_or_term

42) parse_compare_or_term - Comparison operator with no right-hand-side term.

### parse_body

43) parse_body - End of input or no expression where the body is required to contain at least one expression.
44) parse_body - Empty body `{ }` is forbidden — a rule body must contain at least one expression.

### parse_body_loop

45) parse_body_loop - Expected an expression in the body but none matched.
46) parse_body_loop - End of input mid-body (no `}` ever found).
47) parse_body_loop - End of input after `;` separator.
48) parse_body_loop - End of input after newline separator.
49) parse_body_loop - Missing separator (`;` or newline) between expressions in a body.

---

## rule.rs (50 errors)

### parse_default_rule (parser)

50) parse_default_rule - `default` keyword without a rule name (identifier).
51) parse_default_rule - `default <name> =` with no value term following.
52) parse_default_rule - `default <name>` without `=` separator.
53) parse_default_rule - End of input after `default <name>`.

### parse_value_rule (parser)

54) parse_value_rule - Rule head with no body and no `=` (incomplete rule).
55) parse_value_rule - `<name> =` not followed by a term.
56) parse_value_rule - Rule head not followed by `=`, `{`, or `[`.

### parse_partial_rule (parser)

57) parse_partial_rule - `<name>[` not followed by a term.
58) parse_partial_rule - `<name>[k] =` with no value term following.
59) parse_partial_rule - Partial-object rule missing `{ body }`.
60) parse_partial_rule - End of input before `{` (partial-object body never started).
61) parse_partial_rule - Junk after `<name>[term]` where `=` or `{` is expected.
62) parse_partial_rule - End of input after `<name>[term]`.
63) parse_partial_rule - Missing `]` to close `<name>[…`.
64) parse_partial_rule - End of input before `]` was found.

### parse_rule_body (parser)

65) parse_rule_body - Missing `}` to close the rule body.
66) parse_rule_body - End of input where `}` is expected.

### eval_expr (evaluator)

67) eval_expr - `:=` rebinds a name that is already bound in the same body.

### eval_term (evaluator)

68) eval_term - Reference to an unbound variable.

### eval_ref (evaluator)

69) eval_ref - Empty dotted reference (zero segments — should be unreachable but is a defined synthesis target).
70) eval_ref - Dotted reference whose root identifier is not in the binding environment.

### eval_ref_descend (evaluator)

71) eval_ref_descend - Missing object field along a dotted-reference descent.
72) eval_ref_descend - Cannot descend into a `null` (non-object).
73) eval_ref_descend - Cannot descend into a boolean.
74) eval_ref_descend - Cannot descend into a number.
75) eval_ref_descend - Cannot descend into a string.
76) eval_ref_descend - Cannot descend into an unevaluated `Var` (should not happen post-eval).
77) eval_ref_descend - Cannot descend into an unevaluated `Ref` (should not happen post-eval).
78) eval_ref_descend - Cannot descend into an array (Rego objects only support keyed descent).
79) eval_ref_descend - Cannot descend into a set.
80) eval_ref_descend - Cannot descend into an unevaluated arithmetic expression.

### apply_arith (evaluator)

81) apply_arith - Division by zero in arithmetic expression.
82) apply_arith - Type mismatch — RHS is `null` (not a number).
83) apply_arith - Type mismatch — RHS is a boolean.
84) apply_arith - Type mismatch — RHS is a string.
85) apply_arith - Type mismatch — RHS is an unevaluated `Var`.
86) apply_arith - Type mismatch — RHS is an unevaluated `Ref`.
87) apply_arith - Type mismatch — RHS is an array.
88) apply_arith - Type mismatch — RHS is an object.
89) apply_arith - Type mismatch — RHS is a set.
90) apply_arith - Type mismatch — RHS is an unevaluated arithmetic expression.
91) apply_arith - Type mismatch — LHS is `null` (not a number).
92) apply_arith - Type mismatch — LHS is a boolean.
93) apply_arith - Type mismatch — LHS is a string.
94) apply_arith - Type mismatch — LHS is an unevaluated `Var`.
95) apply_arith - Type mismatch — LHS is an unevaluated `Ref`.
96) apply_arith - Type mismatch — LHS is an array.
97) apply_arith - Type mismatch — LHS is an object.
98) apply_arith - Type mismatch — LHS is a set.
99) apply_arith - Type mismatch — LHS is an unevaluated arithmetic expression.

---

## module.rs (5 errors)

100) parse_module - Module without a `package` clause (file does not start with `package <path>`).
101) parse_module - `import` statement found after the package clause; multi-module / `import` is out of scope.
102) parse_package_clause - `package` keyword without an identifier (e.g. `package` alone, `package =`).
103) parse_package_path_tail - Trailing `.` in package path (e.g. `package a.`).
104) parse_rules_loop - Junk between rules — neither end-of-input nor a recognizable rule head.

---

## Summary

| File         | Errors |
|--------------|--------|
| rule.rs      |     50 |
| term.rs      |     28 |
| literal.rs   |     11 |
| expr.rs      |     10 |
| module.rs    |      5 |
| **Total**    | **104**|

---

## Note on `Error::merge()`

`Error::merge(e1, e2)` is defined but intentionally not used in this parser
or evaluator. The reasoning matches the TOML case study (`lang/src/toml/ERRORS.md`):

**Fail-fast is correct here.** Each `Error::fresh()` is a unique symbolic path
marker. The SMT solver synthesizes one concrete input per target — e.g.,
"find a Rego policy that reaches error #59 (partial-object missing body)".
These are independent synthesis goals.

Using `Error::merge(e1, e2)` would create a *combined* target asking Z3 to
find an input that simultaneously reaches *both* error paths. For a sequential
parser this is rarely satisfiable: the parser stops at the first error. For
the evaluator, only one rule body path is followed per concrete input.

**When merge would help.** If the language acquired richer error-recovery
machinery (e.g., a Rego linter that reports every error in a module rather
than aborting at the first), merge would let `parse_rules_loop` accumulate
errors across independent rule definitions. That requires the `ParseResult`
type to carry a partial-success-with-errors variant, which is a significant
architectural change — out of scope here.

**Conclusion.** The current fail-fast, per-`Error::fresh()` design is correct
and sufficient for individual path synthesis.
