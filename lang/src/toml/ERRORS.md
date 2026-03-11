# TOML Parser Error Index

Each `Error::fresh()` marks a distinct _path condition_ that the SMT solver can target.
Format: `N) function - path condition`

---

## mod.rs (3 errors)

1) parse_comment_rest - Comment body contains a character that is neither tab (%x09), printable ASCII (%x20-7E), nor valid non-ASCII Unicode; i.e. a raw control character like NUL or DEL.

2) parse_toml_loop - Merging two consecutive top-level expressions failed: the same key is defined twice with incompatible value types (e.g., `key = 1` then `key = "s"`).

3) parse_toml_loop - After a complete expression a non-newline character appears where a newline separator between expressions is required (e.g., trailing junk on a line).

---

## key_value.rs (8 errors)

4) parse_key_value - A key and `=` were parsed but no value type matched (all parsers returned NoMatch).

5) parse_key_value - A key and `=` were parsed but end of input reached before any value could be parsed.

6) parse_keyval_sep - A character other than `=` appears where the key-value separator is expected (e.g., `key : value` uses `:` instead of `=`).

7) parse_keyval_sep - End of input reached where `=` separator is expected (bare key with no value).

8) parse_dotted_key_loop - A `.` dot separator was found but no key segment follows it (trailing dot, e.g., `a.b. = 1`).

9) parse_unquoted_key - First character of an unquoted key is not alphanumeric, `-`, or `_` (e.g., `@key = 1`).

10) parse_quoted_key - Multi-line basic string delimiter `"""` found in key position; you cannot use multi-line basic strings (`"""..."""`) as quoted keys.

11) parse_quoted_key - Multi-line literal string delimiter `'''` found in key position; you cannot use multi-line literal strings (`'''...'''`) as quoted keys.

---

## boolean.rs (4 errors)

12) parse_boolean - Boolean literal uses title-case `True`; TOML is case-sensitive, only `true` is valid.

13) parse_boolean - Boolean literal uses all-uppercase `TRUE`; only `true` is valid.

14) parse_boolean - Boolean literal uses title-case `False`; only `false` is valid.

15) parse_boolean - Boolean literal uses all-uppercase `FALSE`; only `false` is valid.

---

## array.rs (8 errors)

16) parse_array - End of input immediately after `[` array open; no element or `]` follows.

17) parse_array - End of input after leading whitespace/comments following `[`; closing `]` never found.

18) parse_array - The key for this inline array was already defined as an `[[array-of-tables]]` header; a static inline array and a dynamic array-of-tables cannot share the same key.

19) parse_array_values - No value type matched where an array element was expected (empty slot with no value).

20) parse_array_values - End of input after an array value; expected `]` or `,` but found nothing.

21) parse_array_values - End of input after `,` separator inside array; expected next element or `]` but found nothing.

22) parse_array_values - Unexpected character after an array value; expected `]` or `,` (e.g., `[1 2]` missing comma).

23) parse_ws_comment_newline - A `#` comment was found but it is not followed by a newline (comment must end before end-of-line).

---

## table.rs (16 errors)

24) parse_std_table - End of input immediately after `[`; no table name or `]` follows.

25) parse_std_table - Empty standard table name `[]` (table header has no key).

26) parse_std_table - Unexpected character found where `]` is expected to close the table header (e.g., `[table`extra`]`).

27) parse_std_table - End of input reached where `]` is expected to close the table header.

28) parse_array_table - End of input immediately after `[[`; no array-table name or `]]` follows.

29) parse_array_table - Empty array-table name `[[]]` (array-table header has no key).

30) parse_array_table - Unexpected character found where `]]` is expected to close the array-table header.

31) parse_array_table - End of input reached where `]]` is expected to close the array-table header.

32) parse_inline_table - The key path for this inline table was already defined as an inline table; inline tables are immutable after definition.

33) parse_inline_table - The key path for this inline table was already closed/implicitly defined by a prior dotted key; cannot reopen as inline table.

34) parse_inline_table - The key path for this inline table was already explicitly defined via a `[table]` header; cannot redefine as inline table.

35) parse_inline_table - End of input immediately after `{`; inline table body never terminated.

36) parse_inline_table_keyvals - End of input inside inline table body while expecting `}` close brace or `,` separator.

37) parse_inline_table_keyvals - Conflicting (duplicate) keys when merging inline table entries (e.g., `{a = 1, a = 2}`).

38) parse_inline_table_sep - A character other than `,` appears where the inline-table separator is expected.

39) parse_inline_table_sep - End of input reached where `,` inline-table separator is expected.

---

## expr.rs (11 errors, all in parse_expression)

40) parse_expression - A dotted key redefines a path already established via a `[table]` header (e.g., `[a]\nb.c = 1` then `a.b.c = 2`).

41) parse_expression - A dotted key redefines a path already defined as an inline table (inline tables are immutable).

42) parse_expression - A dotted key redefines a path already established as an `[[array-table]]` header.

43) parse_expression - Duplicate `[table]` header: the same table name is defined more than once.

44) parse_expression - A `[table]` header redefines a path that was already implicitly/closed by a prior dotted-key assignment.

45) parse_expression - A `[table]` header redefines a path previously defined as an `[[array-table]]` header.

46) parse_expression - A `[table]` header redefines a path already defined as an inline table (inline tables are immutable).

47) parse_expression - An `[[array-table]]` header redefines a path previously defined as a `[table]` header.

48) parse_expression - An `[[array-table]]` header redefines a path already closed/implicitly defined by a dotted key.

49) parse_expression - An `[[array-table]]` header redefines a path previously defined as an inline table.

50) parse_expression - An `[[array-table]]` header redefines a path previously defined as an inline array.

---

## integer.rs (57 errors)

### parse_integer

51) parse_integer - Sign (`+` or `-`) followed by binary prefix `0b`; signed binary literals are not valid in TOML.

52) parse_integer - Sign (`+` or `-`) followed by octal prefix `0o`; signed octal literals are not valid in TOML.

53) parse_integer - Sign (`+` or `-`) followed by hex prefix `0x`; signed hex literals are not valid in TOML.

54) parse_integer - Uppercase binary prefix `0B`; only lowercase `0b` is valid.

55) parse_integer - Uppercase octal prefix `0O`; only lowercase `0o` is valid.

56) parse_integer - Uppercase hex prefix `0X`; only lowercase `0x` is valid.

### parse_dec_int

57) parse_dec_int - Unsigned decimal integer exceeds i64 maximum value (overflow).

58) parse_dec_int - Decimal integer with explicit `+` sign exceeds i64 maximum value (overflow).

59) parse_dec_int - `+` sign followed by end of input; no digits to form integer.

60) parse_dec_int - `+` sign followed by a non-digit character; no valid integer digits follow.

61) parse_dec_int - Negative decimal integer magnitude exceeds i64 minimum value (underflow).

62) parse_dec_int - `-` sign followed by end of input; no digits to form integer.

63) parse_dec_int - `-` sign followed by a non-digit character; no valid integer digits follow.

### parse_unsigned_dec_int

64) parse_unsigned_dec_int - Leading zero followed by another digit (e.g., `01`); leading zeros are forbidden in TOML decimal integers.

65) parse_unsigned_dec_int - Leading zero followed by an underscore (e.g., `0_1`); leading zeros are forbidden.

66) parse_unsigned_dec_int - Leading zero followed by an alpha character (e.g., `0a`); invalid sequence.

### parse_unsigned_dec_rest_int

67) parse_unsigned_dec_rest_int - Underscore at end of integer (e.g., `1_`); underscore must be surrounded by digits.

68) parse_unsigned_dec_rest_int - Double underscore in decimal integer (e.g., `1__2`); consecutive underscores are forbidden.

69) parse_unsigned_dec_rest_int - Non-decimal hex digit (a-f/A-F) immediately after underscore in decimal integer.

70) parse_unsigned_dec_rest_int - Other invalid character immediately after underscore in decimal integer.

71) parse_unsigned_dec_rest_int - Non-decimal hex digit (a-f/A-F) in the body of a decimal integer (e.g., `1a2`).

### parse_hex_int

72) parse_hex_int - Hexadecimal integer value exceeds i64 maximum (overflow).

73) parse_hex_int - Underscore immediately after `0x` prefix; first character must be a hex digit.

74) parse_hex_int - Invalid (non-hex) character immediately after `0x` prefix.

75) parse_hex_int - End of input immediately after `0x` prefix; at least one hex digit required.

### parse_hex_rest

76) parse_hex_rest - Double underscore in hexadecimal integer (e.g., `0x1__2`).

77) parse_hex_rest - Invalid character after underscore in hexadecimal integer.

78) parse_hex_rest - Underscore at end of hexadecimal integer (e.g., `0x1_`).

### parse_oct_int

79) parse_oct_int - Octal integer value exceeds i64 maximum (overflow).

80) parse_oct_int - Underscore immediately after `0o` prefix; first character must be an octal digit.

81) parse_oct_int - Decimal digit 8 or 9 immediately after `0o` prefix; only digits 0-7 are valid octal.

82) parse_oct_int - Hex digit (a-f/A-F) immediately after `0o` prefix; not a valid octal digit.

83) parse_oct_int - Other invalid character immediately after `0o` prefix.

84) parse_oct_int - End of input immediately after `0o` prefix; at least one octal digit required.

### parse_oct_rest

85) parse_oct_rest - Double underscore in octal integer (e.g., `0o1__2`).

86) parse_oct_rest - Decimal digit 8 or 9 after underscore in octal integer.

87) parse_oct_rest - Hex digit after underscore in octal integer.

88) parse_oct_rest - Other invalid character after underscore in octal integer.

89) parse_oct_rest - Underscore at end of octal integer (e.g., `0o1_`).

90) parse_oct_rest - Decimal digit 8 or 9 in the body of an octal integer (e.g., `0o18`).

91) parse_oct_rest - Hex digit in the body of an octal integer (e.g., `0o1a`).

### parse_bin_int

92) parse_bin_int - Binary integer value exceeds i64 maximum (overflow).

93) parse_bin_int - Underscore immediately after `0b` prefix; first character must be a binary digit.

94) parse_bin_int - Octal digit (2-7) immediately after `0b` prefix; only 0 and 1 are valid binary digits.

95) parse_bin_int - Decimal digit (2-9) immediately after `0b` prefix; only 0 and 1 are valid.

96) parse_bin_int - Hex digit (a-f/A-F) immediately after `0b` prefix.

97) parse_bin_int - Other invalid character immediately after `0b` prefix.

98) parse_bin_int - End of input immediately after `0b` prefix; at least one binary digit required.

### parse_bin_rest

99) parse_bin_rest - Double underscore in binary integer (e.g., `0b1__0`).

100) parse_bin_rest - Octal digit (2-7) after underscore in binary integer.

101) parse_bin_rest - Decimal digit (2-9) after underscore in binary integer.

102) parse_bin_rest - Hex digit after underscore in binary integer.

103) parse_bin_rest - Other invalid character after underscore in binary integer.

104) parse_bin_rest - Underscore at end of binary integer (e.g., `0b1_`).

105) parse_bin_rest - Octal digit (2-7) in the body of a binary integer (e.g., `0b12`).

106) parse_bin_rest - Decimal digit (2-9) in the body of a binary integer.

107) parse_bin_rest - Hex digit in the body of a binary integer (e.g., `0b1a`).

---

## float.rs (26 errors)

### parse_float (overflow/range checks)

108) parse_float - Combined mantissa (integer + fractional parts) with exponent overflows to IEEE 754 infinity.

109) parse_float - Exponent value (adjusted for fractional digits) exceeds i32 maximum; cannot represent as IEEE 754 exponent.

110) parse_float - Exponent value falls below i32 minimum (extreme negative exponent).

111) parse_float - `10^exp` itself overflows to infinity (exponent in i32 range but power is too large).

112) parse_float - Final float result `mantissa * 10^exp` overflows to IEEE 754 infinity.

113) parse_float - Combined integer and fractional parts overflow to infinity (no exponent, frac + decimal).

114) parse_float - Combined integer and fractional parts overflow to infinity (no exponent, end of input after frac).

115) parse_float - Integer part alone overflows to infinity when used with exponent (no fractional part).

116) parse_float - Exponent value exceeds i32 maximum (integer-only float with exponent, no fractional part).

117) parse_float - Exponent value falls below i32 minimum (integer-only float with exponent).

118) parse_float - `10^exp` overflows to infinity (integer-only float with exponent).

119) parse_float - Final float result overflows to infinity (integer-only float with exponent).

### parse_unsigned_dec_rest (fractional part)

120) parse_unsigned_dec_rest - End of input immediately after decimal point `.`; at least one digit required after `.` in a float.

121) parse_unsigned_dec_rest - Non-digit character immediately after decimal point `.`; at least one digit required.

### parse_float_rest

122) parse_float_rest - Underscore at end of fractional part (e.g., `1.2_`); underscore must be surrounded by digits.

123) parse_float_rest - Double underscore in fractional part (e.g., `1.2__3`).

124) parse_float_rest - Non-decimal hex character after underscore in fractional part.

125) parse_float_rest - Other invalid character after underscore in fractional part.

126) parse_float_rest - Duplicate `e`/`E` exponent marker (e.g., `1.2e3e4`); only one exponent marker allowed.

127) parse_float_rest - Non-decimal hex digit in the body of the fractional part (e.g., `1.2a`).

### parse_special_float (case sensitivity)

128) parse_special_float - Special float uses `Nan` casing; only lowercase `nan` is valid.

129) parse_special_float - Special float uses `Inf` casing; only lowercase `inf` is valid.

130) parse_special_float - Special float uses `NAN` casing; only `nan` is valid.

131) parse_special_float - Special float uses `INF` casing; only `inf` is valid.

132) parse_special_float - Special float uses `NaN` casing; only `nan` is valid.

### parse_float_exp_part

133) parse_float_exp_part - End of input after `e`/`E` marker; at least one digit required for the exponent.

---

## datetime.rs (33 errors)

### parse_datetime

134) parse_datetime - Full-date followed by time delimiter (`T`, `t`, or space) but no valid partial-time follows.

### parse_time_hour

135) parse_time_hour - First character of time hour field is not a decimal digit.

136) parse_time_hour - Second character of time hour field is not a decimal digit.

137) parse_time_hour - Time hour value is outside the valid range 00-23.

### parse_time_minute

138) parse_time_minute - First character of time minute field is not a decimal digit.

139) parse_time_minute - Second character of time minute field is not a decimal digit.

140) parse_time_minute - Time minute value is outside the valid range 00-59.

### parse_time_second

141) parse_time_second - First character of time second field is not a decimal digit.

142) parse_time_second - Second character of time second field is not a decimal digit.

143) parse_time_second - Time second value is outside the valid range 00-60 (60 is permitted for leap seconds).

### parse_time_secfrac

144) parse_time_secfrac - `.` found but end of input reached before any fractional digits.

145) parse_time_secfrac - `.` found but first character after it is not a decimal digit.

### partial_time

146) partial_time - First colon found at position 2 but second colon (position 5) is missing (end of input); cannot determine whether this is HH:MM or HH:MM:SS format.

147) partial_time - Both colons present (positions 2 and 5) indicating HH:MM:SS format, but time-second parse fails after time-hour and time-minute were successfully parsed.

148) partial_time - Second colon present at position 5 but first colon absent at position 2 (malformed time pattern).

### parse_full_date

149) parse_full_date - First dash at position 4 found but second dash absent (end of input); input is neither a valid date nor a valid float.

150) parse_full_date - After parsing year and month, parsing date-mday fails (unexpected character or end of input).

151) parse_full_date - Date month value is outside the valid range 01-12.

152) parse_full_date - Date day value is invalid for the given month and year (e.g., Feb 30, or day 0).

153) parse_full_date - First dash at position 4 found but position 7 is not a second dash; input is neither a valid date nor a valid float.

154) parse_full_date - First dash not at position 4 but second dash present at position 7; input is neither a valid date nor a valid float.

### parse_date_fullyear (4 errors for 4 digit positions)

155) parse_date_fullyear - First character of year is not a digit.

156) parse_date_fullyear - Second character of year is not a digit.

157) parse_date_fullyear - Third character of year is not a digit.

158) parse_date_fullyear - Fourth character of year is not a digit.

### parse_date_month (2 errors)

159) parse_date_month - First character of month is not a digit.

160) parse_date_month - Second character of month is not a digit.

### parse_date_mday (2 errors)

161) parse_date_mday - First character of day is not a digit.

162) parse_date_mday - Second character of day is not a digit.

### parse_time_numoffset (4 errors)

163) parse_time_numoffset - End of input after parsing offset hour where `:` colon is expected between offset hour and minute.

164) parse_time_numoffset - Offset hour and `:` colon parsed successfully, but time-minute parse fails (NoMatch).

165) parse_time_numoffset - Character other than `:` found between offset hour and minute where colon is expected (e.g., `+05-00`).

166) parse_time_numoffset - `+` or `-` sign found but time-hour parse fails (NoMatch); no valid hour digits follow the sign.

---

## string.rs (20 errors)

### parse_ml_basic_string

167) parse_ml_basic_string - Missing closing `"""` delimiter for multi-line basic string (body parsed, no newline after open).

168) parse_ml_basic_string - Missing closing `"""` delimiter for multi-line basic string (body parsed, newline was present after open).

169) parse_ml_basic_string - End of input immediately after opening `"""` delimiter.

### parse_escape_seq_char (escape sequence parsing for basic strings)

170) parse_escape_seq_char - Invalid escape sequence in basic string: character after `\` is not one of `"`, `\`, `n`, `r`, `t`, `b`, `e`, `f`, `u`, `U`, `x`.

171) parse_escape_seq_char - Invalid hex digits in `\xNN`, `\uXXXX`, or `\UXXXXXXXX` Unicode escape sequence (contains non-hex characters).

172) parse_escape_seq_char - Unicode scalar value from `\xNN`, `\uXXXX`, or `\UXXXXXXXX` is not a valid Unicode code point (e.g., surrogate range or out of range).

### parse_mlb_escaped_nl (multi-line basic string line-ending escape)

173) parse_mlb_escaped_nl - `\` line-ending escape found but the character immediately following (after optional whitespace) is not a newline.

### parse_mlb_quote_content (multi-line basic string quote sequences)

174) parse_mlb_quote_content - Quote sequence (1-2 `"` characters) found inside multi-line basic string, but no valid content follows and the next characters do not form the closing `"""` delimiter.

### parse_ml_literal_string

175) parse_ml_literal_string - Missing closing `'''` delimiter for multi-line literal string (no newline after open).

176) parse_ml_literal_string - Missing closing `'''` delimiter for multi-line literal string (newline was present after open).

177) parse_ml_literal_string - End of input immediately after opening `'''` delimiter.

### parse_ml_literal_quote_content (multi-line literal string quote sequences)

178) parse_ml_literal_quote_content - Quote sequence (1-2 `'` characters) found inside multi-line literal string, but no valid content follows and the next characters do not form the closing `'''` delimiter.

### parse_basic_string / parse_basic_string_content

179) parse_basic_string - End of input reached without a closing `"` quotation mark.

180) parse_basic_string - Character other than `"` found where closing quotation mark is expected.

181) parse_basic_string_content - Newline found inside a single-line basic string (newlines not permitted in basic strings).

182) parse_basic_string_content - Invalid character in basic string body (control character or other forbidden character).

### parse_literal_string / parse_literal_string_content

183) parse_literal_string - End of input reached without a closing `'` apostrophe.

184) parse_literal_string - Character other than `'` found where closing apostrophe is expected.

185) parse_literal_string_content - Newline found inside a single-line literal string (newlines not permitted in literal strings).

186) parse_literal_string_content - Invalid character in literal string body (control character or other forbidden character).

---

## Summary

| File           | Errors |
|----------------|--------|
| integer.rs     |     57 |
| datetime.rs    |     33 |
| float.rs       |     26 |
| string.rs      |     20 |
| table.rs       |     16 |
| expr.rs        |     11 |
| array.rs       |      8 |
| key_value.rs   |      8 |
| boolean.rs     |      4 |
| mod.rs         |      3 |
| **Total**      |**186** |

---

## Note on `Error::merge()`

`Error::merge(e1, e2)` is defined but intentionally not used in this parser.

**Why fail-fast is correct here:** Each `Error::fresh()` is a unique symbolic path marker. The SMT solver synthesizes one concrete input per target error — e.g., "find a TOML document that reaches error #37". These are independent synthesis goals.

Using `Error::merge(e1, e2)` would create a *combined* target asking Z3 to find a single input that simultaneously reaches *both* error paths. For a sequential parser this is rarely satisfiable: the parser stops at the first error, so a second error on the same document is not reachable in the same parse trace.

**When merge would help:** If the parser were restructured with error recovery (skip to the next newline on failure and continue), merge could accumulate errors across independent top-level expressions. The natural insertion point would be `parse_toml_loop` in `mod.rs`: after a failed `parse_expression`, skip the offending line, collect the error with `Error::merge`, and continue. This requires the `ParseResult` type to carry a partial-success-with-errors variant, which is a significant architectural change.

**Conclusion:** The current fail-fast, per-`Error::fresh()` design is correct and sufficient for individual path synthesis. Error collection across a document would require a dedicated error-recovery pass and a richer result type.
