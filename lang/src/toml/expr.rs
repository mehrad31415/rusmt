//! parsing Expressions
use crate::toml::{
    Optional, ParseResult, ParserContext, State,
    ast::Value,
    current_char,
    key_value::parse_key_value,
    parse_comment, parse_newline, parse_ws,
    table::{parse_array_table, parse_std_table},
};
use rusmart_smt_remark_derive::smt_fn;
use rusmart_smt_stdlib::{
    Array, Boolean, Cloak, Error, Integer, Seq, String,
    smt::SMT,
};

/// `expression = ws [ comment ] / ws keyval ws [ comment ] / ws table ws [ comment ]`
#[smt_fn]
pub(crate) fn parse_expression(state: State) -> ParseResult<Array<String, Value>> {
    let state_after_ws = parse_ws(state);
    // now we are at the first non-whitespace character - take a peek
    let next_char = current_char(state_after_ws);
    match next_char {
        Optional::None => {
            // covers ws and reaches eof
            return ParseResult::Ok(Array::new(), state_after_ws);
        }
        Optional::Some(_x) => {
            match parse_newline(state_after_ws) {
                ParseResult::Err(e) => ParseResult::Err(e),
                // its just whitespace until newline and then eof
                ParseResult::NoMatch => {
                    match parse_comment(state_after_ws) {
                        ParseResult::Err(e) => ParseResult::Err(e),
                        // its not empty and it is not a comment either
                        ParseResult::NoMatch => {
                            // parse table array
                            match parse_array_table(state_after_ws) {
                                ParseResult::Err(e) => ParseResult::Err(e),
                                ParseResult::NoMatch => {
                                    match parse_std_table(state_after_ws) {
                                        ParseResult::Err(e) => ParseResult::Err(e),
                                        ParseResult::NoMatch => {
                                            // parse key-value
                                            match parse_key_value(state_after_ws) {
                                                ParseResult::Err(e) => ParseResult::Err(e),
                                                ParseResult::NoMatch => {
                                                    // We tried comment, array_table, std_table, and key_value.
                                                    // None matched.
                                                    ParseResult::NoMatch // cannot happen
                                                }
                                                ParseResult::Ok((key, val), state_after_kv) => {
                                                    // destructuring the state and context
                                                    let state_after_kv_temp = state_after_kv;
                                                    let stream = state_after_kv_temp.stream;
                                                    let cursor = state_after_kv_temp.cursor;
                                                    let context = state_after_kv_temp.context;
                                                    let context_temp = context;
                                                    let current_table_path = context_temp.current_table_path;
                                                    let explicit_tables = context_temp.explicit_tables;
                                                    let closed_tables = context_temp.closed_tables;
                                                    let inline_tables = context_temp.inline_tables;
                                                    let inline_arrays = context_temp.inline_arrays;
                                                    let array_of_tables = context_temp.array_of_tables;
                                                    let implicit_tables = make_tables_from_key(
                                                        current_table_path,
                                                        key,
                                                        Seq::new(),
                                                    );
                                                    // if there are any elements in implicit_tables that are already in explicit_table_names, it's an error
                                                    if *has_intersection(
                                                        explicit_tables,
                                                        implicit_tables,
                                                    ) {
                                                        // using dotted keys to redefine tables already defined in [table] form is not allowed.
                                                        // println!(
                                                        //     "Table redefinition error using dotted keys"
                                                        // );
                                                        return ParseResult::Err(Error::fresh());
                                                    } else {
                                                        if *has_intersection(
                                                            inline_tables,
                                                            implicit_tables,
                                                        ) {
                                                            // using dotted keys to redefine tables already defined as inline tables is not allowed.
                                                            // println!(
                                                            //     "Cannot redefine an inline table using dotted keys"
                                                            // );
                                                            return ParseResult::Err(Error::fresh());
                                                        } else {
                                                            if *has_intersection(
                                                                array_of_tables,
                                                                implicit_tables,
                                                            ) {
                                                                // cannot define a array table as a table using dotted keys
                                                                // println!(
                                                                //     "Cannot redefine an array table using dotted keys"
                                                                // );
                                                                return ParseResult::Err(
                                                                    Error::fresh(),
                                                                );
                                                            } else {
                                                                let new_implicit_table_names =
                                                                    closed_tables
                                                                        .concat(implicit_tables);
                                                                let new_context = ParserContext {
                                                                    current_table_path:
                                                                        current_table_path,
                                                                    explicit_tables:
                                                                        explicit_tables,
                                                                    closed_tables:
                                                                        new_implicit_table_names,
                                                                    inline_tables: inline_tables,
                                                                    inline_arrays: inline_arrays,
                                                                    array_of_tables:
                                                                        array_of_tables,
                                                                };
                                                                let new_state_after_kv = State {
                                                                    stream: stream,
                                                                    cursor: cursor,
                                                                    context: new_context,
                                                                };
                                                                // Construct a key-value pair representation.
                                                                let nested_table =
                                                                    make_nested_table(key, val);
                                                                let patch = if *current_table_path
                                                                    .is_empty()
                                                                {
                                                                    nested_table
                                                                } else {
                                                                    make_nested_table(
                                                                        current_table_path,
                                                                        Value::Table(
                                                                            Cloak::shield(
                                                                                nested_table,
                                                                            ),
                                                                        ),
                                                                    )
                                                                };
                                                                let state_after_ws2 =
                                                                    parse_ws(new_state_after_kv);
                                                                match parse_comment(state_after_ws2)
                                                                {
                                                                    ParseResult::Err(e) => {
                                                                        ParseResult::Err(e)
                                                                    }
                                                                    ParseResult::NoMatch => {
                                                                        return ParseResult::Ok(
                                                                            patch,
                                                                            state_after_ws2,
                                                                        );
                                                                    }
                                                                    ParseResult::Ok(
                                                                        _comment,
                                                                        state_after_comment,
                                                                    ) => {
                                                                        return ParseResult::Ok(
                                                                            patch,
                                                                            state_after_comment,
                                                                        );
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        // its a standard table
                                        ParseResult::Ok(_table_name, _state_after_table) => {
                                            // destructuring the state and context
                                            let _state_after_table_temp = _state_after_table;
                                            let stream = _state_after_table_temp.stream;
                                            let cursor = _state_after_table_temp.cursor;
                                            let context = _state_after_table_temp.context;
                                            let context_temp = context;
                                            let _current_table_path = context_temp.current_table_path;
                                            let explicit_tables = context_temp.explicit_tables;
                                            let closed_tables = context_temp.closed_tables;
                                            let inline_tables = context_temp.inline_tables;
                                            let inline_arrays = context_temp.inline_arrays;
                                            let array_of_tables = context_temp.array_of_tables;
                                            if *explicit_tables.contains(_table_name) {
                                                // table redefinition error
                                                // println!("Duplicate table definition error");
                                                return ParseResult::Err(Error::fresh());
                                            } else {
                                                // its a std table
                                                if *closed_tables.contains(_table_name) {
                                                    // cannot define a table that was already implicitly defined
                                                    // println!("Cannot redefine an implicitly defined table");
                                                    return ParseResult::Err(Error::fresh());
                                                } else {
                                                    if *array_of_tables.contains(_table_name) {
                                                        // cannot define a standard table that was already defined as an array table
                                                        // println!("Cannot redefine an array table as a standard table");
                                                        return ParseResult::Err(Error::fresh());
                                                    } else {
                                                        if *has_intersection(
                                                            inline_tables,
                                                            make_tables_from_table_names(
                                                                _table_name,
                                                                Seq::new(),
                                                            ),
                                                        ) {
                                                            // cannot define a table that was already defined as an inline table
                                                            // println!("Cannot redefine an inline table as a standard table");
                                                            return ParseResult::Err(Error::fresh());
                                                        } else {
                                                            let empty_smt_table = Value::Table(
                                                                Cloak::shield(Array::new()),
                                                            );
                                                            let patch = make_nested_table(
                                                                _table_name,
                                                                empty_smt_table,
                                                            );
                                                            let new_explicit_tables =
                                                                explicit_tables.append(_table_name);
                                                            let new_context = ParserContext {
                                                                current_table_path: _table_name,
                                                                explicit_tables:
                                                                    new_explicit_tables,
                                                                closed_tables: closed_tables,
                                                                inline_tables: inline_tables,
                                                                inline_arrays: inline_arrays,
                                                                array_of_tables: array_of_tables,
                                                            };
                                                            let _state_after_table = State {
                                                                stream: stream,
                                                                cursor: cursor,
                                                                context: new_context,
                                                            };
                                                            let state_after_ws2 =
                                                                parse_ws(_state_after_table);
                                                            match parse_comment(state_after_ws2) {
                                                                ParseResult::Err(e) => {
                                                                    ParseResult::Err(e)
                                                                } // cannot happen
                                                                ParseResult::NoMatch => {
                                                                    // return empty table
                                                                    return ParseResult::Ok(
                                                                        patch,
                                                                        state_after_ws2,
                                                                    );
                                                                }
                                                                ParseResult::Ok(
                                                                    _comment,
                                                                    state_after_comment,
                                                                ) => {
                                                                    return ParseResult::Ok(
                                                                        patch,
                                                                        state_after_comment,
                                                                    );
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                // its a table array
                                ParseResult::Ok(new_array_table_name, state_after_table) => {
                                    // destructuring the state and context
                                    let state_after_table_temp = state_after_table;
                                    let stream = state_after_table_temp.stream;
                                    let cursor = state_after_table_temp.cursor;
                                    let context = state_after_table_temp.context;
                                    let context_temp = context;
                                    let explicit_tables = context_temp.explicit_tables;
                                    let closed_tables = context_temp.closed_tables;
                                    let inline_tables = context_temp.inline_tables;
                                    let inline_arrays = context_temp.inline_arrays;
                                    let array_of_tables = context_temp.array_of_tables;
                                    if *explicit_tables.contains(new_array_table_name) {
                                        // table redefinition error
                                        // println!("This array table was already defined as a standard table");
                                        return ParseResult::Err(Error::fresh());
                                    } else {
                                        // its a new array table
                                        if *closed_tables.contains(new_array_table_name) {
                                            // cannot define a table that was already closed
                                            // println!("Cannot redefine a closed table");
                                            return ParseResult::Err(Error::fresh());
                                        } else {
                                            if *has_intersection(
                                                inline_tables,
                                                make_tables_from_table_names(
                                                    new_array_table_name,
                                                    Seq::new(),
                                                ),
                                            ) {
                                                // cannot define a table that was already defined as an inline table
                                                // println!("Cannot redefine an inline table as an array table");
                                                return ParseResult::Err(Error::fresh());
                                            } else {
                                                if *inline_arrays.contains(new_array_table_name) {
                                                    // cannot define an array table that was already defined as an inline array
                                                    // println!("Cannot redefine an inline array as an array table");
                                                    return ParseResult::Err(Error::fresh());
                                                } else {
                                                    let new_array_tables = array_of_tables
                                                        .append(new_array_table_name);
                                                    let new_context = ParserContext {
                                                        current_table_path: new_array_table_name,
                                                        explicit_tables: explicit_tables,
                                                        closed_tables: closed_tables,
                                                        inline_tables: inline_tables,
                                                        inline_arrays: inline_arrays,
                                                        array_of_tables: new_array_tables,
                                                    };
                                                    let _state_after_table = State {
                                                        stream: stream,
                                                        cursor: cursor,
                                                        context: new_context,
                                                    };
                                                    let arr = Value::Array(Cloak::shield(
                                                        Seq::new().append(Value::Table(
                                                            Cloak::shield(Array::new()),
                                                        )),
                                                    ));
                                                    let patch = make_nested_table(
                                                        new_array_table_name,
                                                        arr,
                                                    );
                                                    let state_after_ws2 =
                                                        parse_ws(_state_after_table);
                                                    match parse_comment(state_after_ws2) {
                                                        ParseResult::Err(e) => ParseResult::Err(e), // cannot happen
                                                        ParseResult::NoMatch => {
                                                            // return empty array
                                                            return ParseResult::Ok(
                                                                patch,
                                                                state_after_ws2,
                                                            );
                                                        }
                                                        ParseResult::Ok(
                                                            _comment,
                                                            state_after_comment,
                                                        ) => {
                                                            return ParseResult::Ok(
                                                                patch,
                                                                state_after_comment,
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // its a comment
                        ParseResult::Ok(_comment, state_after_comment) => {
                            return ParseResult::Ok(Array::new(), state_after_comment);
                        }
                    }
                }
                ParseResult::Ok(_newline, _state_after_newline) => {
                    return ParseResult::Ok(Array::new(), state_after_ws);
                }
            }
        }
    }
}

/// Helper function to create nested tables from dotted keys.
#[smt_fn]
pub(crate) fn make_nested_table(keys: Seq<String>, value: Value) -> Array<String, Value> {
    if *keys.length().eq(0.into()) {
        return Array::<String, Value>::new(); // empty table name which is the root
    } else {
        if *keys.length().eq(1.into()) {
            let key = keys.at(Integer::from(0));
            let arr = Array::<String, Value>::new();
            let arr_filled = arr.store(key, value);
            return arr_filled;
        } else {
            let key = keys.at(Integer::from(0));
            let rest_of_keys = keys.extract(Integer::from(1), keys.length().sub(Integer::from(1)));
            let arr = Array::<String, Value>::new();
            let filled_arr = arr.store(
                key,
                Value::Table(Cloak::shield(make_nested_table(rest_of_keys, value))),
            );
            filled_arr
        }
    }
}

/// delete last element from a Seq<String>
#[smt_fn]
pub(crate) fn droplast(seq: Seq<String>) -> Seq<String> {
    let len = seq.length();
    if *len.eq(0.into()) {
        seq
    } else {
        seq.extract(Integer::from(0), len.sub(Integer::from(1)))
    }
}

/// for a Seq<String> keep on deleting last element and concatenating with "." until empty
#[smt_fn]
pub(crate) fn make_tables_from_key(
    table_keyname: Seq<String>,
    key_parts: Seq<String>,
    acc: Seq<Seq<String>>,
) -> Seq<Seq<String>> {
    // its just a simple key without dots
    if *key_parts.length().eq(1.into()) {
        return acc;
    } else {
        make_tables_from_key_inner(table_keyname, droplast(key_parts), acc)
    }
    
}

/// inner recursive function for make_tables_from_key
#[smt_fn]
pub(crate) fn make_tables_from_key_inner(
    table_keyname: Seq<String>,
    key_parts: Seq<String>,
    acc: Seq<Seq<String>>,
) -> Seq<Seq<String>> {
    if *key_parts.length().eq(0.into()) {
        acc
    } else {
        make_tables_from_key_inner(
            table_keyname,
            droplast(key_parts),
            acc.append(table_keyname.concat(key_parts)),
        )
    }
}

/// check for intersection between two Seq<Seq<String>>
#[smt_fn]
pub(crate) fn has_intersection(seq1: Seq<Seq<String>>, seq2: Seq<Seq<String>>) -> Boolean {
    if *seq2.length().eq(0.into()) {
        return Boolean::from(false);
    } else {
        if *seq2.length().eq(1.into()) {
            let first = seq2.at(Integer::from(0));
            if *seq1.contains(first) {
                return Boolean::from(true);
            } else {
                return Boolean::from(false);
            }
        } else {
            let first = seq2.at(Integer::from(0));
            let rest = seq2.extract(Integer::from(1), seq2.length().sub(Integer::from(1)));
            if *seq1.contains(first) {
                return Boolean::from(true);
            } else {
                has_intersection(seq1, rest)
            }
        }
    }
}

/// make tables from std table and array table names
#[smt_fn]
pub(crate) fn make_tables_from_table_names(
    name: Seq<String>,
    acc: Seq<Seq<String>>,
) -> Seq<Seq<String>> {
    if *name.length().eq(0.into()) {
        acc
    } else {
        make_tables_from_table_names(droplast(name), acc.append(name))
    }
}
