//! Functions for parsing TOML tables.
//! table = std-table / array-table

use crate::toml::{
    Optional, ParseResult, ParserContext, State, advance,
    ast::Value,
    current_char,
    expr::make_nested_table,
    key_value::{parse_key, parse_key_value},
    parse_newline, parse_ws,
};
use rusmart_smt_remark_derive::smt_fn;
use rusmart_smt_stdlib::{Array, Cloak, Error, Integer, Seq, String, choose, forall, smt::SMT};

/// `std-table = std-table-open key std-table-close`
#[smt_fn]
pub(crate) fn parse_std_table(state: State) -> ParseResult<Seq<String>> {
    match parse_std_table_open(state) {
        ParseResult::Ok(_open_bracket, state_after_open) => {
            match current_char(state_after_open) {
                Optional::None => {
                    // println!("cannot have [");
                    ParseResult::Err(Error::fresh())
                } // cannot have [
                Optional::Some(x) => {
                    if *x.eq(String::from("]")) {
                        // empty table name
                        {
                            // println!("cannot have empty table name");
                            return ParseResult::Err(Error::fresh());
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
                                            return ParseResult::Err(Error::fresh());
                                        }
                                        Optional::None => {
                                            // reached end of input where ] expected
                                            // println!("reached end of input where ] expected");
                                            return ParseResult::Err(Error::fresh());
                                        }
                                    },
                                    ParseResult::Err(e) => ParseResult::Err(e), // cannot happen
                                }
                            }
                            ParseResult::NoMatch => ParseResult::NoMatch, // cannot happen
                            ParseResult::Err(e) => ParseResult::Err(e),
                        }
                    }
                }
            }
        }
        ParseResult::NoMatch => ParseResult::NoMatch,
        ParseResult::Err(e) => ParseResult::Err(e), // cannot happen
    }
}

/// `std-table-open  = %x5B ws     ; [ Left square bracket`
#[smt_fn]
fn parse_std_table_open(state: State) -> ParseResult<String> {
    match current_char(state) {
        Optional::Some(ch) => {
            if *ch.eq(String::from("[")) {
                let next_state = advance(state);
                let state_after_ws = parse_ws(next_state);
                ParseResult::Ok(ch, state_after_ws)
            } else {
                ParseResult::NoMatch
            }
        }
        Optional::None => ParseResult::NoMatch,
    }
}

/// `std-table-close = ws %x5D     ; ] Right square bracket`
#[smt_fn]
fn parse_std_table_close(state: State) -> ParseResult<String> {
    let state_after_ws = parse_ws(state);
    match current_char(state_after_ws) {
        Optional::Some(ch) => {
            if *ch.eq(String::from("]")) {
                let next_state = advance(state_after_ws);
                ParseResult::Ok(ch, next_state)
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
                    ParseResult::Err(Error::fresh())
                } // cannot have [[
                Optional::Some(x) => {
                    if *x.eq(String::from("]")) {
                        // empty table name
                        {
                            // println!("cannot have empty table name");
                            return ParseResult::Err(Error::fresh());
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
                                            return ParseResult::Err(Error::fresh());
                                        }
                                        Optional::None => {
                                            // reached end of input where ]] expected
                                            // println!("reached end of input where ]] expected");
                                            return ParseResult::Err(Error::fresh());
                                        }
                                    },
                                    ParseResult::Err(e) => ParseResult::Err(e), // cannot happen
                                }
                            }
                            ParseResult::NoMatch => ParseResult::NoMatch, // cannot happen
                            ParseResult::Err(e) => ParseResult::Err(e),
                        }
                    }
                }
            }
        }
        ParseResult::NoMatch => ParseResult::NoMatch,
        ParseResult::Err(e) => ParseResult::Err(e), // cannot happen
    }
}

/// `array-table-open  = %x5B.5B ws  ; [[ Double left square bracket`
#[smt_fn]
fn parse_array_table_open(state: State) -> ParseResult<String> {
    match current_char(state) {
        Optional::Some(ch1) => {
            if *ch1.eq(String::from("[")) {
                let next_state = advance(state);
                match current_char(next_state) {
                    Optional::Some(ch2) => {
                        if *ch2.eq(String::from("[")) {
                            let next_next_state = advance(next_state);
                            let state_after_ws = parse_ws(next_next_state);
                            ParseResult::Ok(ch1.concat(ch2), state_after_ws)
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
            if *ch1.eq(String::from("]")) {
                let next_state = advance(state_after_ws);
                match current_char(next_state) {
                    Optional::Some(ch2) => {
                        if *ch2.eq(String::from("]")) {
                            let next_next_state = advance(next_state);
                            let res = ch1.concat(ch2);
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
pub(crate) fn parse_inline_table(
    key: Seq<String>,
    state: State,
) -> ParseResult<Array<String, Value>> {
    // change the inline table in the parser context
    let State {
        stream: _,
        cursor: _,
        context,
    } = state;
    let ParserContext {
        current_table_path,
        explicit_tables,
        closed_tables,
        inline_tables,
        inline_arrays: _,
        array_of_tables: _,
    } = context;
    let new_key = current_table_path.concat(key);
    if *inline_tables.contains(new_key) {
        // println!("inline table already defined");
        return ParseResult::Err(Error::fresh());
    } else {
        if *closed_tables.contains(new_key) {
            // println!("inline table already closed");
            return ParseResult::Err(Error::fresh());
        } else {
            if *explicit_tables.contains(new_key) {
                // println!("inline table already defined as explicit table");
                return ParseResult::Err(Error::fresh());
            } else {
                match parse_inline_table_open(state) {
                    ParseResult::Ok(_open_brace, state_after_open) => {
                        match current_char(state_after_open) {
                            Optional::None => {
                                // println!("cannot have {{");
                                ParseResult::Err(Error::fresh())
                            } // cannot have {
                            Optional::Some(_x) => {
                                match parse_inline_table_close(state_after_open) {
                                    ParseResult::Ok(_close_brace, state_after_close) => {
                                        // change the inline table in the parser context
                                        let State {
                                            stream,
                                            cursor,
                                            context,
                                        } = state_after_close;
                                        let ParserContext {
                                            current_table_path,
                                            explicit_tables,
                                            closed_tables,
                                            inline_tables,
                                            inline_arrays,
                                            array_of_tables,
                                        } = context;
                                        let new_context = ParserContext {
                                            current_table_path,
                                            explicit_tables,
                                            closed_tables,
                                            inline_tables: inline_tables.append(new_key),
                                            inline_arrays,
                                            array_of_tables,
                                        };
                                        let new_state_after_close = State {
                                            stream,
                                            cursor,
                                            context: new_context,
                                        };
                                        ParseResult::Ok(Array::new(), new_state_after_close)
                                    }
                                    ParseResult::NoMatch => {
                                        // non-empty inline table
                                        parse_inline_table_keyvals(new_key, state_after_open)
                                    }
                                    ParseResult::Err(e) => ParseResult::Err(e), // cannot happen
                                }
                            }
                        }
                    }
                    ParseResult::NoMatch => ParseResult::NoMatch,
                    ParseResult::Err(e) => ParseResult::Err(e), // cannot happen
                }
            }
        }
    }
}

/// `inline-table-keyvals = keyval [ inline-table-sep inline-table-keyvals ]`
#[smt_fn]
fn parse_inline_table_keyvals(
    new_key: Seq<String>,
    state: State,
) -> ParseResult<Array<String, Value>> {
    match parse_newline(state) {
        ParseResult::Ok(_newline, _state_after_newline) => {
            // Inline tables are intended to appear on a single line...
            // println!("newlines not permitted in inline table");
            ParseResult::Err(Error::fresh())
        }
        ParseResult::NoMatch => {
            match parse_key_value(state) {
                ParseResult::Ok((key, val), state_after_kv) => {
                    match current_char(state_after_kv) {
                        Optional::None => {
                            // println!("cannot have end of input here expecting }} or ,");
                            ParseResult::Err(Error::fresh()) // cannot have end of input here expecting } or ,
                        }
                        Optional::Some(_ch) => {
                            match parse_inline_table_close(state_after_kv) {
                                ParseResult::Ok(_close_brace, state_after_close) => {
                                    let nested_table = make_nested_table(key, val);
                                    let State {
                                        stream,
                                        cursor,
                                        context,
                                    } = state_after_close;
                                    let ParserContext {
                                        current_table_path,
                                        explicit_tables,
                                        closed_tables,
                                        inline_tables,
                                        inline_arrays,
                                        array_of_tables,
                                    } = context;
                                    let new_context = ParserContext {
                                        current_table_path,
                                        explicit_tables,
                                        closed_tables,
                                        inline_tables: inline_tables.append(new_key),
                                        inline_arrays,
                                        array_of_tables,
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
                                                ParseResult::Ok(
                                                    _close_brace,
                                                    _state_after_close,
                                                ) => {
                                                    // A terminating comma (also called trailing comma) is not permitted after the last key/value pair in an inline table.
                                                    // println!(
                                                    //     "trailing comma not permitted in inline table"
                                                    // );
                                                    ParseResult::Err(Error::fresh())
                                                }
                                                ParseResult::NoMatch => {
                                                    // Parse the next key-value pair(s) recursively
                                                    match parse_inline_table_keyvals(
                                                        key,
                                                        state_after_sep,
                                                    ) {
                                                        ParseResult::Ok(rest_kvs, final_state) => {
                                                            let nested_table =
                                                                make_nested_table(key, val);
                                                            let combined = recursive_merge_tables(
                                                                nested_table,
                                                                rest_kvs,
                                                            );
                                                            match combined {
                                                                Optional::Some(merged_table) => {
                                                                    let State {
                                                                        stream,
                                                                        cursor,
                                                                        context,
                                                                    } = final_state;
                                                                    let ParserContext {
                                                                        current_table_path,
                                                                        explicit_tables,
                                                                        closed_tables,
                                                                        inline_tables,
                                                                        inline_arrays,
                                                                        array_of_tables,
                                                                    } = context;
                                                                    let new_context =
                                                                        ParserContext {
                                                                            current_table_path,
                                                                            explicit_tables,
                                                                            closed_tables,
                                                                            inline_tables:
                                                                                inline_tables
                                                                                    .append(new_key),
                                                                            inline_arrays,
                                                                            array_of_tables,
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
                                                                    ParseResult::Err(Error::fresh())
                                                                }
                                                            }
                                                        }
                                                        ParseResult::NoMatch => {
                                                            ParseResult::NoMatch
                                                        } // cannot happen
                                                        ParseResult::Err(e) => ParseResult::Err(e),
                                                    }
                                                }
                                                ParseResult::Err(e) => ParseResult::Err(e), // cannot happen
                                            }
                                        }
                                        ParseResult::NoMatch => ParseResult::NoMatch, // cannot happen
                                        ParseResult::Err(e) => ParseResult::Err(e),
                                    }
                                }
                                ParseResult::Err(e) => ParseResult::Err(e), // cannot happen
                            }
                        }
                    }
                }
                ParseResult::NoMatch => ParseResult::NoMatch, // will not happen
                ParseResult::Err(e) => ParseResult::Err(e),
            }
        }
        ParseResult::Err(e) => ParseResult::Err(e), // cannot happen
    }
}

/// `inline-table-open  = %x7B ws     ; {`
#[smt_fn]
fn parse_inline_table_open(state: State) -> ParseResult<String> {
    match current_char(state) {
        Optional::Some(ch) => {
            if *ch.eq(String::from("{")) {
                let next_state = advance(state);
                let state_after_ws = parse_ws(next_state);
                ParseResult::Ok(ch, state_after_ws)
            } else {
                ParseResult::NoMatch
            }
        }
        Optional::None => ParseResult::NoMatch,
    }
}

/// `inline-table-close = ws %x7D     ; }`
#[smt_fn]
fn parse_inline_table_close(state: State) -> ParseResult<String> {
    let state_after_ws = parse_ws(state);
    match current_char(state_after_ws) {
        Optional::Some(ch) => {
            if *ch.eq(String::from("}")) {
                let next_state = advance(state_after_ws);
                ParseResult::Ok(ch, next_state)
            } else {
                ParseResult::NoMatch
            }
        }
        Optional::None => ParseResult::NoMatch,
    }
}

/// `inline-table-sep   = ws %x2C ws  ; , Comma`
#[smt_fn]
fn parse_inline_table_sep(state: State) -> ParseResult<String> {
    let state_after_ws1 = parse_ws(state);
    match current_char(state_after_ws1) {
        Optional::Some(ch) => {
            if *ch.eq(String::from(",")) {
                let next_state = advance(state_after_ws1);
                let state_after_ws2 = parse_ws(next_state);
                ParseResult::Ok(ch, state_after_ws2)
            } else {
                // Another character present where comma expected
                // println!("found another character where , expected");
                ParseResult::Err(Error::fresh())
            }
        }
        // No character present where comma expected
        Optional::None => {
            // println!("reached end of input where , expected");
            ParseResult::Err(Error::fresh())
        }
    }
}

/// Chooses the minimum key from a TOML table (Array<String, Value>).
#[smt_fn]
pub(crate) fn array_key_min(array: Array<String, Value>) -> String {
    choose!(s in array => forall!(e in array => s.eq(e).or(s.lt(e))))
}

/// Recursively merges two TOML tables.
#[smt_fn]
pub(crate) fn recursive_merge_tables(
    acc_table: Array<String, Value>,
    new_table: Array<String, Value>,
) -> Optional<Array<String, Value>> {
    if *new_table.is_empty() {
        return Optional::Some(acc_table);
    }

    let key_to_merge = array_key_min(new_table);
    let new_val = new_table.select(key_to_merge);
    let rest_of_new_table = new_table.del(key_to_merge);
    let key_exists = acc_table.contains_key(key_to_merge);

    if *key_exists {
        let acc_val = acc_table.select(key_to_merge);

        match acc_val {
            Value::Table(_tab) => match new_val {
                Value::Table(_next_table) => {
                    let sub_merge_result =
                        recursive_merge_tables(_tab.reveal(), _next_table.reveal());

                    // Check if the sub-merge succeeded
                    match sub_merge_result {
                        Optional::Some(merged_sub_array) => {
                            let next_acc = acc_table
                                .store(key_to_merge, Value::Table(Cloak::shield(merged_sub_array)));
                            return recursive_merge_tables(next_acc, rest_of_new_table);
                        }
                        Optional::None => return Optional::None,
                    }
                }
                _other => {
                    return Optional::None;
                }
            },
            Value::Array(acc_cloak) => match new_val {
                Value::Array(new_cloak) => {
                    let acc_seq = acc_cloak.reveal();
                    let new_seq = new_cloak.reveal();

                    // Concatenate the new items to the existing array
                    let final_seq = acc_seq.concat(new_seq);

                    let next_acc =
                        acc_table.store(key_to_merge, Value::Array(Cloak::shield(final_seq)));
                    return recursive_merge_tables(next_acc, rest_of_new_table);
                }

                Value::Table(new_cloak) => {
                    let acc_seq = acc_cloak.reveal();
                    if *acc_seq.is_empty() {
                        return Optional::None;
                    }
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
                                    let prefix = acc_seq.extract(Integer::from(0), last_idx);

                                    let new_last_val =
                                        Value::Table(Cloak::shield(merged_last_table));
                                    let new_tail = Seq::new().append(new_last_val);
                                    let new_seq = prefix.concat(new_tail);
                                    let next_acc = acc_table
                                        .store(key_to_merge, Value::Array(Cloak::shield(new_seq)));
                                    return recursive_merge_tables(next_acc, rest_of_new_table);
                                }
                                Optional::None => return Optional::None,
                            }
                        }
                        _ => return Optional::None,
                    }
                }
                _other => return Optional::None,
            },
            _other => {
                return Optional::None;
            }
        };
    } else {
        let next_acc = acc_table.store(key_to_merge, new_val);
        recursive_merge_tables(next_acc, rest_of_new_table)
    }
}
