# TOML v1.1.0 — Integer values

Source: <https://toml.io/en/v1.1.0>, section "Integer", reproduced verbatim below.

## Scope of the semantics to write

A document is a sequence of newline-separated key/value lines. A key is a bare
key: one or more of `A-Z a-z 0-9 _ -`. It is followed by optional spaces, an
`=`, optional spaces, and a value. In this subset **every value is an integer**;
no other value type need be supported. Optional spaces may surround the `=`, and
a line may be the whole document (no trailing newline).

Report each way a document can fail to be a valid integer. Give every failure
its own `Path::named` marker, choosing the names yourself and making them as
fine-grained as the specification distinguishes: two documents that break
different rules should raise different markers. A document that is a valid
integer key/value line must produce no marker at all.

## Integer

Integers are whole numbers. Positive numbers may be prefixed with a plus sign.
Negative numbers are prefixed with a minus sign.

```toml
int1 = +99
int2 = 42
int3 = 0
int4 = -17
```

For large numbers, you may use underscores between digits to enhance
readability. Each underscore must be surrounded by at least one digit on each
side.

```toml
int5 = 1_000
int6 = 5_349_221
int7 = 53_49_221  # Indian number system grouping
int8 = 1_2_3_4_5  # VALID but discouraged
```

Leading zeros are not allowed. Integer values `-0` and `+0` are valid and
identical to an unprefixed zero.

Non-negative integer values may also be expressed in hexadecimal, octal, or
binary. In these formats, leading `+` is not allowed and leading zeros are
allowed (after the prefix). Hex values are case-insensitive. Underscores are
allowed between digits (but not between the prefix and the value).

```toml
# hexadecimal with prefix `0x`
hex1 = 0xDEADBEEF
hex2 = 0xdeadbeef
hex3 = 0xdead_beef

# octal with prefix `0o`
oct1 = 0o01234567
oct2 = 0o755 # useful for Unix file permissions

# binary with prefix `0b`
bin1 = 0b11010110
```

Implementations are free to support any integer size. It's recommended that at
least 64-bit signed integers (from −2^63 to 2^63−1) are accepted and handled
losslessly. If an integer cannot be represented losslessly, an error must be
thrown.

## Representation for this exercise

Represent the parsed value as a 64-bit signed integer (`I64`), and raise the
corresponding overflow/underflow marker when a literal cannot be represented in
that range. This fixes the choice the paragraph above leaves open, so that the
drafted semantics and the reference agree on where the boundary lies.
