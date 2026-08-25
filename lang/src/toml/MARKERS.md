# TOML Marker Index

## `lang/src/toml/array.rs` (8)

- `array_open_eof`
- `array_open_after_ws_eof`
- `array_of_tables_inline_array`
- `array_values_expected_value`
- `array_value_eof_after_value`
- `array_sep_eof_after_comma`
- `array_value_invalid_separator`
- `array_comment_missing_newline`

## `lang/src/toml/boolean.rs` (4)

- `boolean_invalid_capital_true`
- `boolean_invalid_allcaps_true`
- `boolean_invalid_capital_false`
- `boolean_invalid_allcaps_false`

## `lang/src/toml/datetime.rs` (32)

- `datetime_expect_partial_time_after_delim`
- `time_hour_first_char_not_digit`
- `time_hour_second_char_not_digit`
- `time_hour_out_of_range`
- `time_minute_first_char_not_digit`
- `time_minute_second_char_not_digit`
- `time_minute_out_of_range`
- `time_second_first_char_not_digit`
- `time_second_second_char_not_digit`
- `time_second_out_of_range`
- `time_secfrac_no_digit_after_dot_eof`
- `time_secfrac_no_digit_after_dot_nondigit`
- `partial_time_expect_second_after_minute`
- `partial_time_second_colon_without_first`
- `full_date_first_dash_without_second`
- `full_date_expect_mday_after_month`
- `date_invalid_month`
- `date_invalid_day`
- `full_date_second_dash_wrong_char`
- `full_date_first_dash_wrong_char`
- `date_year_first_char_not_digit`
- `date_year_second_char_not_digit`
- `date_year_third_char_not_digit`
- `date_year_fourth_char_not_digit`
- `date_month_first_char_not_digit`
- `date_month_second_char_not_digit`
- `date_mday_first_char_not_digit`
- `date_mday_second_char_not_digit`
- `numoffset_expect_colon_eof`
- `numoffset_expect_minute_after_colon`
- `numoffset_expect_colon_wrong_char`
- `numoffset_expect_hour_after_sign`

## `lang/src/toml/expr.rs` (11)

- `dotted_key_redefines_std_table`
- `dotted_key_redefines_inline_table`
- `dotted_key_redefines_array_table`
- `std_table_duplicate`
- `std_table_redefines_implicit_table`
- `std_table_redefines_array_table`
- `std_table_redefines_inline_table`
- `array_table_redefines_std_table`
- `array_table_redefines_closed_table`
- `array_table_redefines_inline_table`
- `array_table_redefines_inline_array`

## `lang/src/toml/float.rs` (25)

- `float_frac_combined_overflow`
- `float_frac_exp_overflow_i32`
- `float_frac_exp_underflow_i32`
- `float_frac_pow10_overflow`
- `float_frac_final_overflow`
- `float_frac_noexp_combined_overflow`
- `float_frac_eof_combined_overflow`
- `float_exp_only_exp_overflow_i32`
- `float_exp_only_exp_underflow_i32`
- `float_exp_only_pow10_overflow`
- `float_exp_only_final_overflow`
- `float_no_digit_after_dot_eof`
- `float_no_digit_after_dot`
- `float_underscore_at_end`
- `float_multiple_underscores`
- `float_hex_char_after_underscore`
- `float_invalid_char_after_underscore`
- `float_duplicate_exponent`
- `float_hex_char_in_part`
- `float_invalid_nan_casing_titlecase`
- `float_invalid_inf_casing_titlecase`
- `float_invalid_nan_casing_allcaps`
- `float_invalid_inf_casing_allcaps`
- `float_invalid_nan_casing_camelcase`
- `float_exp_no_digit_after_e`

## `lang/src/toml/integer.rs` (57)

- `integer_signed_bin_prefix`
- `integer_signed_oct_prefix`
- `integer_signed_hex_prefix`
- `integer_bin_prefix_uppercase`
- `integer_oct_prefix_uppercase`
- `integer_hex_prefix_uppercase`
- `integer_dec_overflow_unsigned`
- `integer_dec_overflow_plus_sign`
- `integer_dec_plus_sign_no_digits_eof`
- `integer_dec_plus_sign_no_digits_other`
- `integer_dec_underflow_minus_sign`
- `integer_dec_minus_sign_no_digits_eof`
- `integer_dec_minus_sign_no_digits_other`
- `integer_dec_leading_zero_digit`
- `integer_dec_leading_zero_underscore`
- `integer_dec_invalid_char_after_zero`
- `integer_dec_underscore_at_end`
- `integer_dec_double_underscore`
- `integer_dec_hex_char_after_underscore`
- `integer_dec_invalid_char_after_underscore`
- `integer_dec_hex_char_in_body`
- `integer_hex_overflow`
- `integer_hex_underscore_after_prefix`
- `integer_hex_invalid_char_after_prefix`
- `integer_hex_no_digits_after_prefix`
- `integer_hex_double_underscore`
- `integer_hex_invalid_char_after_underscore`
- `integer_hex_underscore_at_end`
- `integer_oct_overflow`
- `integer_oct_underscore_after_prefix`
- `integer_oct_dec_digit_after_prefix`
- `integer_oct_hex_digit_after_prefix`
- `integer_oct_invalid_char_after_prefix`
- `integer_oct_no_digits_after_prefix`
- `integer_oct_double_underscore`
- `integer_oct_dec_digit_after_underscore`
- `integer_oct_hex_digit_after_underscore`
- `integer_oct_invalid_char_after_underscore`
- `integer_oct_underscore_at_end`
- `integer_oct_dec_digit_in_body`
- `integer_oct_hex_digit_in_body`
- `integer_bin_overflow`
- `integer_bin_underscore_after_prefix`
- `integer_bin_oct_digit_after_prefix`
- `integer_bin_dec_digit_after_prefix`
- `integer_bin_hex_digit_after_prefix`
- `integer_bin_invalid_char_after_prefix`
- `integer_bin_no_digits_after_prefix`
- `integer_bin_double_underscore`
- `integer_bin_oct_digit_after_underscore`
- `integer_bin_dec_digit_after_underscore`
- `integer_bin_hex_digit_after_underscore`
- `integer_bin_invalid_char_after_underscore`
- `integer_bin_underscore_at_end`
- `integer_bin_oct_digit_in_body`
- `integer_bin_dec_digit_in_body`
- `integer_bin_hex_digit_in_body`

## `lang/src/toml/key_value.rs` (8)

- `key_value_missing_value_nomatch`
- `key_value_missing_value_eof`
- `key_value_missing_equals_char`
- `key_value_missing_equals_eof`
- `dotted_key_missing_segment`
- `bare_key_invalid_start`
- `quoted_key_multiline_basic`
- `quoted_key_multiline_literal`

## `lang/src/toml/mod.rs` (3)

- `comment_invalid_char`
- `toml_table_merge_type_mismatch`
- `toml_expected_newline_between_expressions`

## `lang/src/toml/string.rs` (19)

- `ml_basic_missing_close_no_newline`
- `ml_basic_missing_close_after_newline`
- `ml_basic_open_eof`
- `string_invalid_escape`
- `string_unicode_escape_invalid_hex`
- `string_unicode_escape_invalid_scalar`
- `ml_basic_escaped_newline_missing_newline`
- `ml_basic_invalid_escape`
- `ml_basic_quotes_without_content`
- `ml_literal_missing_close_no_newline`
- `ml_literal_missing_close_after_newline`
- `ml_literal_open_eof`
- `ml_literal_quotes_without_content`
- `basic_string_missing_close_eof`
- `basic_string_newline`
- `basic_string_invalid_char`
- `literal_string_missing_close_eof`
- `literal_string_newline`
- `literal_string_invalid_char`

## `lang/src/toml/table.rs` (16)

- `std_table_open_eof`
- `std_table_empty_name`
- `std_table_missing_close_char`
- `std_table_missing_close_eof`
- `array_table_open_eof`
- `array_table_empty_name`
- `array_table_missing_close_char`
- `array_table_missing_close_eof`
- `inline_table_redefined_inline`
- `inline_table_redefined_closed`
- `inline_table_redefined_explicit`
- `inline_table_open_eof`
- `inline_table_unterminated`
- `inline_table_conflicting_keys`
- `inline_table_sep_expected_comma`
- `inline_table_sep_eof`
