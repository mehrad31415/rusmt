//! Functions for parsing TOML tables.
//! table = std-table / array-table

use crate::toml::{
    Optional, ParseResult, ParserContext, State, advance,
    array::parse_ws_comment_newline,
    ast::{
        Table, Value, table_contains_key, table_del, table_get, table_is_empty, table_key_min,
        table_new, table_store,
    },
    cp_to_str, current_char,
    expr::make_nested_table,
    key_value::{parse_key, parse_key_value},
    parse_ws,
};
use rusmt_smt_remark_derive::smt_fn;
use rusmt_smt_stdlib::{Cloak, Integer, Path, Seq, String, U32, smt::SMT};

/// `std-table = std-table-open key std-table-close`
#[smt_fn]
pub(crate) fn parse_std_table(state: State) -> ParseResult<Seq<String>> {
    match parse_std_table_open(state) {
        ParseResult::Ok(_open_bracket, state_after_open) => {
            match current_char(state_after_open) {
                Optional::None => {
                    // println!("cannot have [");
                    ParseResult::Err(Path::named(String::from("std_table_open_eof")))
                } // cannot have [
                Optional::Some(x) => {
                    if *x.eq(U32::from(0x5D)) {
                        // empty table name
                        {
                            // println!("cannot have empty table name");
                            return ParseResult::Err(Path::named(String::from(
                                "std_table_empty_name",
                            )));
                        }
                    } else {
                        // non-empty table name
                        match parse_key(state_after_open) {
                            ParseResult::Ok(key, state_after_key) => {
                                match parse_std_table_close(state_after_key) {
                                    ParseResult::Ok(_close_bracket, state_after_close) => {
                                        return ParseResult::Ok(key, state_after_close);
                                    }
                                    ParseResult::NoMatch => match current_char(state_after_key) {
                                        Optional::Some(_ch) => {
                                            // found another character where ] expected
                                            // println!("found another character where ] expected");
                                            return ParseResult::Err(Path::named(String::from(
                                                "std_table_missing_close_char",
                                            )));
                                        }
                                        Optional::None => {
                                            // reached end of input where ] expected
                                            // println!("reached end of input where ] expected");
                                            return ParseResult::Err(Path::named(String::from(
                                                "std_table_missing_close_eof",
                                            )));
                                        }
                                    },
                                    ParseResult::Err(e) => return ParseResult::Err(e), // cannot happen
                                }
                            }
                            ParseResult::NoMatch => return ParseResult::NoMatch, // cannot happen
                            ParseResult::Err(e) => return ParseResult::Err(e),
                        }
                    }
                }
            }
        }
        ParseResult::NoMatch => return ParseResult::NoMatch,
        ParseResult::Err(e) => return ParseResult::Err(e), // cannot happen
    }
}

/// `std-table-open  = %x5B ws     ; [ Left square bracket`
#[smt_fn]
fn parse_std_table_open(state: State) -> ParseResult<String> {
    match current_char(state) {
        Optional::Some(ch) => {
            if *ch.eq(U32::from(0x5B)) {
                let next_state = advance(state);
                let state_after_ws = parse_ws(next_state);
                return ParseResult::Ok(cp_to_str(ch), state_after_ws);
            } else {
                return ParseResult::NoMatch;
            }
        }
        Optional::None => return ParseResult::NoMatch,
    }
}

/// `std-table-close = ws %x5D     ; ] Right square bracket`
#[smt_fn]
fn parse_std_table_close(state: State) -> ParseResult<String> {
    let state_after_ws = parse_ws(state);
    match current_char(state_after_ws) {
        Optional::Some(ch) => {
            if *ch.eq(U32::from(0x5D)) {
                let next_state = advance(state_after_ws);
                ParseResult::Ok(cp_to_str(ch), next_state)
            } else {
                ParseResult::NoMatch
            }
        }
        Optional::None => ParseResult::NoMatch,
    }
}

/// `array-table = array-table-open key array-table-close`
#[smt_fn]
pub(crate) fn parse_array_table(state: State) -> ParseResult<Seq<String>> {
    match parse_array_table_open(state) {
        ParseResult::Ok(_open_brackets, state_after_open) => {
            match current_char(state_after_open) {
                Optional::None => {
                    // println!("cannot have [[");
                    ParseResult::Err(Path::named(String::from("array_table_open_eof")))
                } // cannot have [[
                Optional::Some(x) => {
                    if *x.eq(U32::from(0x5D)) {
                        // empty table name
                        {
                            // println!("cannot have empty table name");
                            return ParseResult::Err(Path::named(String::from(
                                "array_table_empty_name",
                            )));
                        }
                    } else {
                        // non-empty table name
                        match parse_key(state_after_open) {
                            ParseResult::Ok(key, state_after_key) => {
                                match parse_array_table_close(state_after_key) {
                                    ParseResult::Ok(_close_brackets, state_after_close) => {
                                        return ParseResult::Ok(key, state_after_close);
                                    }
                                    ParseResult::NoMatch => match current_char(state_after_key) {
                                        Optional::Some(_ch) => {
                                            // found another character where ]] expected
                                            // println!("found another character where ]] expected");
                                            return ParseResult::Err(Path::named(String::from(
                                                "array_table_missing_close_char",
                                            )));
                                        }
                                        Optional::None => {
                                            // reached end of input where ]] expected
                                            // println!("reached end of input where ]] expected");
                                            return ParseResult::Err(Path::named(String::from(
                                                "array_table_missing_close_eof",
                                            )));
                                        }
                                    },
                                    ParseResult::Err(e) => return ParseResult::Err(e), // cannot happen
                                }
                            }
                            ParseResult::NoMatch => return ParseResult::NoMatch, // cannot happen
                            ParseResult::Err(e) => return ParseResult::Err(e),
                        }
                    }
                }
            }
        }
        ParseResult::NoMatch => return ParseResult::NoMatch,
        ParseResult::Err(e) => return ParseResult::Err(e), // cannot happen
    }
}

/// `array-table-open  = %x5B.5B ws  ; [[ Double left square bracket`
#[smt_fn]
fn parse_array_table_open(state: State) -> ParseResult<String> {
    match current_char(state) {
        Optional::Some(ch1) => {
            if *ch1.eq(U32::from(0x5B)) {
                let next_state = advance(state);
                match current_char(next_state) {
                    Optional::Some(ch2) => {
                        if *ch2.eq(U32::from(0x5B)) {
                            let next_next_state = advance(next_state);
                            let state_after_ws = parse_ws(next_next_state);
                            ParseResult::Ok(cp_to_str(ch1).concat(cp_to_str(ch2)), state_after_ws)
                        } else {
                            ParseResult::NoMatch
                        }
                    }
                    Optional::None => ParseResult::NoMatch,
                }
            } else {
                ParseResult::NoMatch
            }
        }
        Optional::None => ParseResult::NoMatch, // never happens
    }
}

/// `array-table-close = ws %x5D.5D  ; ]] Double right square bracket`
#[smt_fn]
fn parse_array_table_close(state: State) -> ParseResult<String> {
    let state_after_ws = parse_ws(state);
    match current_char(state_after_ws) {
        Optional::Some(ch1) => {
            if *ch1.eq(U32::from(0x5D)) {
                let next_state = advance(state_after_ws);
                match current_char(next_state) {
                    Optional::Some(ch2) => {
                        if *ch2.eq(U32::from(0x5D)) {
                            let next_next_state = advance(next_state);
                            let res = cp_to_str(ch1).concat(cp_to_str(ch2));
                            ParseResult::Ok(res, next_next_state)
                        } else {
                            ParseResult::NoMatch
                        }
                    }
                    Optional::None => ParseResult::NoMatch,
                }
            } else {
                ParseResult::NoMatch
            }
        }
        Optional::None => ParseResult::NoMatch,
    }
}

/// `inline-table = inline-table-open [ inline-table-keyvals ] inline-table-close`
#[smt_fn]
pub(crate) fn parse_inline_table(key: Seq<String>, state: State) -> ParseResult<Table> {
    // change the inline table in the parser context
    let state_temp = state;
    let context = state_temp.context;
    let context_temp1 = context;
    let current_table_path = context_temp1.current_table_path;
    let explicit_tables = context_temp1.explicit_tables;
    let closed_tables = context_temp1.closed_tables;
    let inline_tables = context_temp1.inline_tables;

    let new_key = current_table_path.concat(key);
    if *inline_tables.contains(new_key) {
        // println!("inline table already defined");
        return ParseResult::Err(Path::named(String::from("inline_table_redefined_inline")));
    } else {
        if *closed_tables.contains(new_key) {
            // println!("inline table already closed");
            return ParseResult::Err(Path::named(String::from("inline_table_redefined_closed")));
        } else {
            if *explicit_tables.contains(new_key) {
                // println!("inline table already defined as explicit table");
                return ParseResult::Err(Path::named(String::from(
                    "inline_table_redefined_explicit",
                )));
            } else {
                match parse_inline_table_open(state) {
                    ParseResult::Ok(_open_brace, state_after_open) => {
                        match current_char(state_after_open) {
                            Optional::None => {
                                // println!("cannot have {{");
                                ParseResult::Err(Path::named(String::from("inline_table_open_eof")))
                            } // cannot have {
                            Optional::Some(_x) => {
                                match parse_inline_table_close(state_after_open) {
                                    ParseResult::Ok(_close_brace, state_after_close) => {
                                        // change the inline table in the parser context
                                        let state_after_close_temp = state_after_close;
                                        let stream = state_after_close_temp.stream;
                                        let cursor = state_after_close_temp.cursor;
                                        let context_inner = state_after_close_temp.context;
                                        let context_temp = context_inner;
                                        let current_table_path2 = context_temp.current_table_path;
                                        let explicit_tables2 = context_temp.explicit_tables;
                                        let closed_tables2 = context_temp.closed_tables;
                                        let inline_tables2 = context_temp.inline_tables;
                                        let inline_arrays2 = context_temp.inline_arrays;
                                        let array_of_tables2 = context_temp.array_of_tables;
                                        let new_context = ParserContext {
                                            current_table_path: current_table_path2,
                                            explicit_tables: explicit_tables2,
                                            closed_tables: closed_tables2,
                                            inline_tables: inline_tables2.append(new_key),
                                            inline_arrays: inline_arrays2,
                                            array_of_tables: array_of_tables2,
                                        };
                                        let new_state_after_close = State {
                                            stream,
                                            cursor,
                                            context: new_context,
                                        };
                                        ParseResult::Ok(table_new(), new_state_after_close)
                                    }
                                    ParseResult::NoMatch => {
                                        // non-empty inline table
                                        parse_inline_table_keyvals(new_key, state_after_open)
                                    }
                                    ParseResult::Err(e) => return ParseResult::Err(e), // cannot happen
                                }
                            }
                        }
                    }
                    ParseResult::NoMatch => return ParseResult::NoMatch,
                    ParseResult::Err(e) => return ParseResult::Err(e), // cannot happen
                }
            }
        }
    }
}

/// `inline-table-keyvals = keyval [ inline-table-sep inline-table-keyvals ]`
/// v1.1.0: newlines (via ws-comment-newline in open/sep/close) and trailing commas are permitted.
#[smt_fn]
fn parse_inline_table_keyvals(new_key: Seq<String>, state: State) -> ParseResult<Table> {
    match parse_key_value(state) {
        ParseResult::Ok(kv, state_after_kv) => {
            let key = kv.key;
            let val = kv.val;
            match current_char(state_after_kv) {
                Optional::None => {
                    // println!("cannot have end of input here expecting }} or ,");
                    ParseResult::Err(Path::named(String::from("inline_table_unterminated"))) // cannot have end of input here expecting } or ,
                }
                Optional::Some(_ch) => {
                    match parse_inline_table_close(state_after_kv) {
                        ParseResult::Ok(_close_brace, state_after_close) => {
                            let nested_table = make_nested_table(key, val);
                            let state_after_close_temp = state_after_close;
                            let stream = state_after_close_temp.stream;
                            let cursor = state_after_close_temp.cursor;
                            let context = state_after_close_temp.context;
                            let context_tmp = context;
                            let current_table_path3 = context_tmp.current_table_path;
                            let explicit_tables3 = context_tmp.explicit_tables;
                            let closed_tables3 = context_tmp.closed_tables;
                            let inline_tables3 = context_tmp.inline_tables;
                            let inline_arrays3 = context_tmp.inline_arrays;
                            let array_of_tables3 = context_tmp.array_of_tables;
                            let new_context = ParserContext {
                                current_table_path: current_table_path3,
                                explicit_tables: explicit_tables3,
                                closed_tables: closed_tables3,
                                inline_tables: inline_tables3.append(new_key),
                                inline_arrays: inline_arrays3,
                                array_of_tables: array_of_tables3,
                            };
                            let new_state_after_close = State {
                                stream,
                                cursor,
                                context: new_context,
                            };
                            ParseResult::Ok(nested_table, new_state_after_close)
                        }
                        ParseResult::NoMatch => {
                            // Try to parse inline-table-sep
                            match parse_inline_table_sep(state_after_kv) {
                                ParseResult::Ok(_sep, state_after_sep) => {
                                    match parse_inline_table_close(state_after_sep) {
                                        ParseResult::Ok(_close_brace, state_after_close_tc) => {
                                            // v1.1.0: trailing comma is permitted
                                            let nested_table = make_nested_table(key, val);
                                            let state_tc_temp = state_after_close_tc;
                                            let stream_tc = state_tc_temp.stream;
                                            let cursor_tc = state_tc_temp.cursor;
                                            let context_tc = state_tc_temp.context;
                                            let context_tmp_tc = context_tc;
                                            let ctp_tc = context_tmp_tc.current_table_path;
                                            let et_tc = context_tmp_tc.explicit_tables;
                                            let ct_tc = context_tmp_tc.closed_tables;
                                            let it_tc = context_tmp_tc.inline_tables;
                                            let ia_tc = context_tmp_tc.inline_arrays;
                                            let aot_tc = context_tmp_tc.array_of_tables;
                                            let new_context_tc = ParserContext {
                                                current_table_path: ctp_tc,
                                                explicit_tables: et_tc,
                                                closed_tables: ct_tc,
                                                inline_tables: it_tc.append(new_key),
                                                inline_arrays: ia_tc,
                                                array_of_tables: aot_tc,
                                            };
                                            let new_state_tc = State {
                                                stream: stream_tc,
                                                cursor: cursor_tc,
                                                context: new_context_tc,
                                            };
                                            ParseResult::Ok(nested_table, new_state_tc)
                                        }
                                        ParseResult::NoMatch => {
                                            match parse_inline_table_keyvals(
                                                new_key,
                                                state_after_sep,
                                            ) {
                                                ParseResult::Ok(rest_kvs, final_state) => {
                                                    let nested_table = make_nested_table(key, val);
                                                    let combined = recursive_merge_tables(
                                                        nested_table,
                                                        rest_kvs,
                                                    );
                                                    match combined {
                                                        Optional::Some(merged_table) => {
                                                            let final_state_temp = final_state;
                                                            let stream = final_state_temp.stream;
                                                            let cursor = final_state_temp.cursor;
                                                            let context = final_state_temp.context;
                                                            let context_tmp2 = context;
                                                            let current_table_path4 =
                                                                context_tmp2.current_table_path;
                                                            let explicit_tables4 =
                                                                context_tmp2.explicit_tables;
                                                            let closed_tables4 =
                                                                context_tmp2.closed_tables;
                                                            let inline_tables4 =
                                                                context_tmp2.inline_tables;
                                                            let inline_arrays4 =
                                                                context_tmp2.inline_arrays;
                                                            let array_of_tables4 =
                                                                context_tmp2.array_of_tables;
                                                            let new_context = ParserContext {
                                                                current_table_path:
                                                                    current_table_path4,
                                                                explicit_tables: explicit_tables4,
                                                                closed_tables: closed_tables4,
                                                                inline_tables: inline_tables4
                                                                    .append(new_key),
                                                                inline_arrays: inline_arrays4,
                                                                array_of_tables: array_of_tables4,
                                                            };
                                                            let new_final_state = State {
                                                                stream,
                                                                cursor,
                                                                context: new_context,
                                                            };
                                                            ParseResult::Ok(
                                                                merged_table,
                                                                new_final_state,
                                                            )
                                                        }
                                                        // merging failed because of conflicting keys
                                                        Optional::None => {
                                                            // println!("conflicting keys in inline table");
                                                            ParseResult::Err(Path::named(
                                                                String::from(
                                                                    "inline_table_conflicting_keys",
                                                                ),
                                                            ))
                                                        }
                                                    }
                                                }
                                                ParseResult::NoMatch => ParseResult::NoMatch, // cannot happen
                                                ParseResult::Err(e) => {
                                                    return ParseResult::Err(e);
                                                }
                                            }
                                        }
                                        ParseResult::Err(e) => return ParseResult::Err(e), // cannot happen
                                    }
                                }
                                ParseResult::NoMatch => ParseResult::NoMatch, // cannot happen
                                ParseResult::Err(e) => return ParseResult::Err(e),
                            }
                        }
                        ParseResult::Err(e) => return ParseResult::Err(e), // cannot happen
                    }
                }
            }
        }
        ParseResult::NoMatch => return ParseResult::NoMatch, // will not happen
        ParseResult::Err(e) => return ParseResult::Err(e),
    }
}

/// `inline-table-open  = %x7B ws-comment-newline     ; {`
#[smt_fn]
fn parse_inline_table_open(state: State) -> ParseResult<String> {
    match current_char(state) {
        Optional::Some(ch) => {
            if *ch.eq(U32::from(0x7B)) {
                let next_state = advance(state);
                match parse_ws_comment_newline(next_state) {
                    ParseResult::Ok(_ws, state_after_ws) => {
                        ParseResult::Ok(cp_to_str(ch), state_after_ws)
                    }
                    ParseResult::Err(e) => ParseResult::Err(e),
                    ParseResult::NoMatch => ParseResult::NoMatch, // cannot happen
                }
            } else {
                ParseResult::NoMatch
            }
        }
        Optional::None => ParseResult::NoMatch,
    }
}

/// `inline-table-close = ws-comment-newline %x7D     ; }`
#[smt_fn]
fn parse_inline_table_close(state: State) -> ParseResult<String> {
    match parse_ws_comment_newline(state) {
        ParseResult::Err(e) => ParseResult::Err(e),
        ParseResult::NoMatch => ParseResult::NoMatch, // cannot happen
        ParseResult::Ok(_ws, state_after_ws) => match current_char(state_after_ws) {
            Optional::Some(ch) => {
                if *ch.eq(U32::from(0x7D)) {
                    let next_state = advance(state_after_ws);
                    ParseResult::Ok(cp_to_str(ch), next_state)
                } else {
                    ParseResult::NoMatch
                }
            }
            Optional::None => ParseResult::NoMatch,
        },
    }
}

/// `inline-table-sep   = ws-comment-newline %x2C ws-comment-newline  ; , Comma`
#[smt_fn]
fn parse_inline_table_sep(state: State) -> ParseResult<String> {
    match parse_ws_comment_newline(state) {
        ParseResult::Err(e) => ParseResult::Err(e),
        ParseResult::NoMatch => ParseResult::NoMatch, // cannot happen
        ParseResult::Ok(_ws1, state_after_ws1) => match current_char(state_after_ws1) {
            Optional::Some(ch) => {
                if *ch.eq(U32::from(0x2C)) {
                    let next_state = advance(state_after_ws1);
                    match parse_ws_comment_newline(next_state) {
                        ParseResult::Ok(_ws2, state_after_ws2) => {
                            ParseResult::Ok(cp_to_str(ch), state_after_ws2)
                        }
                        ParseResult::Err(e) => ParseResult::Err(e),
                        ParseResult::NoMatch => ParseResult::NoMatch, // cannot happen
                    }
                } else {
                    // Another character present where comma expected
                    // println!("found another character where , expected");
                    ParseResult::Err(Path::named(String::from("inline_table_sep_expected_comma")))
                }
            }
            // No character present where comma expected
            Optional::None => {
                // println!("reached end of input where , expected");
                ParseResult::Err(Path::named(String::from("inline_table_sep_eof")))
            }
        },
    }
}

/// Recursively merges two TOML tables.
#[smt_fn]
pub(crate) fn recursive_merge_tables(acc_table: Table, new_table: Table) -> Optional<Table> {
    if *table_is_empty(new_table) {
        return Optional::Some(acc_table);
    } else {
        if *table_is_empty(new_table) {
            return Optional::Some(acc_table); // early-return
        } else {
            let key_to_merge = table_key_min(new_table);
            let new_val = table_get(new_table, key_to_merge);
            let rest_of_new_table = table_del(new_table, key_to_merge);
            let key_exists = table_contains_key(acc_table, key_to_merge);

            if *key_exists {
                let acc_val = table_get(acc_table, key_to_merge);

                match acc_val {
                    Value::Table(_tab) => match new_val {
                        Value::Table(_next_table) => {
                            let sub_merge_result =
                                recursive_merge_tables(_tab.reveal(), _next_table.reveal());

                            // Check if the sub-merge succeeded
                            match sub_merge_result {
                                Optional::Some(merged_sub_array) => {
                                    let next_acc = table_store(
                                        acc_table,
                                        key_to_merge,
                                        Value::Table(Cloak::shield(merged_sub_array)),
                                    );
                                    return recursive_merge_tables(next_acc, rest_of_new_table);
                                }
                                Optional::None => return Optional::None,
                            }
                        }
                        _ => {
                            return Optional::None;
                        }
                    },
                    Value::Array(acc_cloak) => match new_val {
                        Value::Array(new_cloak) => {
                            let acc_seq = acc_cloak.reveal();
                            let new_seq = new_cloak.reveal();

                            // Concatenate the new items to the existing array
                            let final_seq = acc_seq.concat(new_seq);

                            let next_acc = table_store(
                                acc_table,
                                key_to_merge,
                                Value::Array(Cloak::shield(final_seq)),
                            );
                            return recursive_merge_tables(next_acc, rest_of_new_table);
                        }

                        Value::Table(new_cloak) => {
                            let acc_seq = acc_cloak.reveal();
                            if *acc_seq.is_empty() {
                                return Optional::None;
                            } else {
                                let last_idx = acc_seq.length().sub(Integer::from(1));
                                let last_val = acc_seq.at(last_idx);

                                match last_val {
                                    Value::Table(last_table_cloak) => {
                                        let sub_merge_result = recursive_merge_tables(
                                            last_table_cloak.reveal(),
                                            new_cloak.reveal(),
                                        );

                                        match sub_merge_result {
                                            Optional::Some(merged_last_table) => {
                                                let prefix: Seq<Value> =
                                                    acc_seq.extract(Integer::from(0), last_idx);

                                                let new_last_val =
                                                    Value::Table(Cloak::shield(merged_last_table));
                                                let new_tail: Seq<Value> =
                                                    Seq::new().append(new_last_val);
                                                let new_seq = prefix.concat(new_tail);
                                                let next_acc = table_store(
                                                    acc_table,
                                                    key_to_merge,
                                                    Value::Array(Cloak::shield(new_seq)),
                                                );
                                                return recursive_merge_tables(
                                                    next_acc,
                                                    rest_of_new_table,
                                                );
                                            }
                                            Optional::None => return Optional::None,
                                        }
                                    }
                                    _ => return Optional::None,
                                }
                            }
                        }
                        _ => return Optional::None,
                    },
                    _ => {
                        return Optional::None;
                    }
                }
            } else {
                let next_acc = table_store(acc_table, key_to_merge, new_val);
                recursive_merge_tables(next_acc, rest_of_new_table)
            }
        }
    }
}
