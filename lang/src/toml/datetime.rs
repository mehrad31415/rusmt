//! Parse Date and Time (as defined in RFC 3339)

use crate::toml::float::parse_float;
use crate::toml::{Optional, ParseResult, State, advance, ast::DateTime, current_char, peek};
use rusmart_smt_remark_derive::smt_fn;
use rusmart_smt_stdlib::{Boolean, Error, Integer, String, smt::SMT};

/// date-time = offset-date-time / local-date-time / local-date / local-time
/// offset-date-time = full-date time-delim partial-time ("Z" / time-numoffset)
/// local-date-time = full-date time-delim partial-time
/// local-date = full-date
/// local-time = partial-time
#[smt_fn]
pub(crate) fn parse_datetime(state: State) -> ParseResult<DateTime> {
    match parse_full_date(state) {
        ParseResult::NoMatch => {
            // try local-time
            match partial_time(state) {
                ParseResult::NoMatch => return ParseResult::NoMatch,
                ParseResult::Err(e) => return ParseResult::Err(e),
                ParseResult::Ok(local_time_str, state_after_local_time) => {
                    ParseResult::Ok(DateTime::LocalTime(local_time_str), state_after_local_time)
                }
            }
        }
        ParseResult::Err(e) => return ParseResult::Err(e),
        ParseResult::Ok(full_date_str, state_after_full_date) => {
            match parse_time_delim(state_after_full_date) {
                ParseResult::NoMatch => {
                    // try local-date
                    ParseResult::Ok(DateTime::LocalDate(full_date_str), state_after_full_date)
                }
                ParseResult::Err(e) => return ParseResult::Err(e),
                ParseResult::Ok(_time_delim, state_after_time_delim) => {
                    match partial_time(state_after_time_delim) {
                        ParseResult::NoMatch => {
                            // println!("expect partial-time after full-date and time-delim");
                            ParseResult::Err(Error::fresh()) // expect partial-time
                        }
                        ParseResult::Err(e) => return ParseResult::Err(e),
                        ParseResult::Ok(partial_time_str, state_after_partial_time) => {
                            match current_char(state_after_partial_time) {
                                Optional::None => {
                                    // try local-date-time
                                    ParseResult::Ok(
                                        DateTime::LocalDateTime(
                                            full_date_str
                                                .concat(_time_delim)
                                                .concat(partial_time_str),
                                        ),
                                        state_after_partial_time,
                                    )
                                }
                                Optional::Some(c) => {
                                    if *c.eq(String::from("Z")) {
                                        let after_z = advance(state_after_partial_time);
                                        let datetime_str = full_date_str
                                            .concat(_time_delim)
                                            .concat(partial_time_str)
                                            .concat(String::from("Z"));
                                        ParseResult::Ok(
                                            DateTime::OffsetDateTime(datetime_str),
                                            after_z,
                                        )
                                    } else {
                                        match parse_time_numoffset(state_after_partial_time) {
                                            ParseResult::NoMatch => {
                                                // try local-date-time
                                                ParseResult::Ok(
                                                    DateTime::LocalDateTime(
                                                        full_date_str
                                                            .concat(_time_delim)
                                                            .concat(partial_time_str),
                                                    ),
                                                    state_after_partial_time,
                                                )
                                            }
                                            ParseResult::Err(e) => return ParseResult::Err(e),
                                            ParseResult::Ok(
                                                time_numoffset_str,
                                                state_after_numoffset,
                                            ) => {
                                                let datetime_str = full_date_str
                                                    .concat(_time_delim)
                                                    .concat(partial_time_str)
                                                    .concat(time_numoffset_str);
                                                ParseResult::Ok(
                                                    DateTime::OffsetDateTime(datetime_str),
                                                    state_after_numoffset,
                                                )
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// time-hour = 2DIGIT  ; 00-23
#[smt_fn]
fn parse_time_hour(input: State) -> ParseResult<String> {
    match current_char(input) {
        Optional::None => ParseResult::NoMatch, // cannot happen
        Optional::Some(c1) => {
            if *c1.is_digit().not() {
                // println!("first char not digit");
                return ParseResult::Err(Error::fresh()); // first char not digit
            } else {
                let after_c1 = advance(input);
                match current_char(after_c1) {
                    Optional::None => ParseResult::NoMatch, // cannot happen
                    Optional::Some(c2) => {
                        if *c2.is_digit().not() {
                            // println!("second char not digit");
                            return ParseResult::Err(Error::fresh()); // second char not digit
                        } else {
                            let hour_str = c1.concat(c2);
                            if *hour_str
                                .lt(String::from("00"))
                                .or(hour_str.gt(String::from("23")))
                            {
                                // println!("invalid hour range");
                                return ParseResult::Err(Error::fresh()); // invalid hour range
                            } else {
                                let after_c2 = advance(after_c1);
                                ParseResult::Ok(hour_str, after_c2)
                            }
                        }
                    }
                }
            }
        }
    }
}

/// time-minute = 2DIGIT  ; 00-59
#[smt_fn]
fn parse_time_minute(input: State) -> ParseResult<String> {
    match current_char(input) {
        Optional::None => ParseResult::NoMatch, // cannot happen
        Optional::Some(c1) => {
            if *c1.is_digit().not() {
                // println!("first char in minute not digit");
                return ParseResult::Err(Error::fresh()); // first char not digit
            } else {
                let after_c1 = advance(input);
                match current_char(after_c1) {
                    Optional::None => ParseResult::NoMatch, // cannot happen
                    Optional::Some(c2) => {
                        if *c2.is_digit().not() {
                            // println!("second char in minute not digit");
                            return ParseResult::Err(Error::fresh()); // second char not digit
                        } else {
                            let minute_str = c1.concat(c2);
                            if *minute_str
                                .lt(String::from("00"))
                                .or(minute_str.gt(String::from("59")))
                            {
                                // println!("invalid minute range");
                                return ParseResult::Err(Error::fresh()); // invalid minute range
                            } else {
                                let after_c2 = advance(after_c1);
                                ParseResult::Ok(minute_str, after_c2)
                            }
                        }
                    }
                }
            }
        }
    }
}

/// time-second = 2DIGIT  ; 00-58, 00-59, 00-60 based on leap second rules
#[smt_fn]
fn parse_time_second(input: State) -> ParseResult<String> {
    match current_char(input) {
        Optional::None => ParseResult::NoMatch, // cannot happen
        Optional::Some(c1) => {
            if *c1.is_digit().not() {
                // println!("first char in second not digit");
                return ParseResult::Err(Error::fresh()); // first char not digit
            } else {
                let after_c1 = advance(input);
                match current_char(after_c1) {
                    Optional::None => ParseResult::NoMatch, // cannot happen
                    Optional::Some(c2) => {
                        if *c2.is_digit().not() {
                            // println!("second char in second not digit");
                            return ParseResult::Err(Error::fresh()); // second char not digit
                        } else {
                            let second_str = c1.concat(c2);
                            if *second_str
                                .lt(String::from("00"))
                                .or(second_str.gt(String::from("60")))
                            {
                                // println!("invalid second range");
                                return ParseResult::Err(Error::fresh()); // invalid second range
                            } else {
                                let after_c2 = advance(after_c1);
                                ParseResult::Ok(second_str, after_c2)
                            }
                        }
                    }
                }
            }
        }
    }
}

/// time-secfrac   = "." 1*DIGIT
#[smt_fn]
fn parse_time_secfrac(input: State) -> ParseResult<String> {
    match current_char(input) {
        Optional::None => ParseResult::NoMatch,
        Optional::Some(c) => {
            if *c.eq(String::from(".")) {
                let after_dot = advance(input);
                match current_char(after_dot) {
                    Optional::None => {
                        // println!("must have at least one digit after '.' found nothing");
                        ParseResult::Err(Error::fresh())
                    } // must have at least one digit after '.'
                    Optional::Some(c1) => {
                        if *c1.is_digit().not() {
                            // println!("must have at least one digit after '.' found something else");
                            return ParseResult::Err(Error::fresh()); // must have at least one digit after '.'
                        } else {
                            parse_digit_sequence_rest(advance(input), c.concat(c1))
                        }
                    }
                }
            } else {
                ParseResult::NoMatch
            }
        }
    }
}

/// *DIGIT
#[smt_fn]
fn parse_digit_sequence_rest(input: State, acc: String) -> ParseResult<String> {
    match current_char(input) {
        Optional::None => ParseResult::Ok(acc, input),
        Optional::Some(c) => {
            if *c.is_digit().not() {
                // finish parsing
                return ParseResult::Ok(acc, input);
            } else {
                let new_acc = acc.concat(c);
                parse_digit_sequence_rest(advance(input), new_acc)
            }
        }
    }
}

/// partial-time = time-hour ":" time-minute ":" time-second [ time-secfrac ]
#[smt_fn]
fn partial_time(input: State) -> ParseResult<String> {
    // peek to see if this is partial time or an float
    let first_colon = peek(input, 2.into());
    let second_colon = peek(input, 5.into());
    match first_colon {
        Optional::None => ParseResult::NoMatch,
        Optional::Some(c1) => {
            if *c1.eq(String::from(":")) {
                match second_colon {
                    Optional::None => {
                        // println!("second colon not found non existent when the first colon is found");
                        ParseResult::Err(Error::fresh())
                    } // invalid partial-time
                    Optional::Some(c2) => {
                        if *c2.eq(String::from(":")).not() {
                            // it doesnt have second only minutes and time and seconds is assumed to be 00
                            match parse_time_hour(input) {
                                ParseResult::NoMatch => return ParseResult::NoMatch, // cannot happen
                                ParseResult::Err(e) => return ParseResult::Err(e),
                                ParseResult::Ok(time_hour, state_after_time_hour) => {
                                    let after_colon = advance(state_after_time_hour);
                                    match parse_time_minute(after_colon) {
                                        ParseResult::NoMatch => return ParseResult::NoMatch, // cannot happen
                                        ParseResult::Err(e) => return ParseResult::Err(e),
                                        ParseResult::Ok(time_minute, state_after_time_minute) => {
                                            // it can still have time-secfrac
                                            match parse_time_secfrac(state_after_time_minute) {
                                                ParseResult::Ok(
                                                    time_secfrac,
                                                    state_after_secfrac,
                                                ) => {
                                                    let time_str = time_hour
                                                        .concat(String::from(":"))
                                                        .concat(time_minute)
                                                        .concat(String::from(":"))
                                                        .concat(String::from("00"))
                                                        .concat(time_secfrac);
                                                    ParseResult::Ok(time_str, state_after_secfrac)
                                                }
                                                ParseResult::Err(e) => return ParseResult::Err(e),
                                                ParseResult::NoMatch => {
                                                    let time_str = time_hour
                                                        .concat(String::from(":"))
                                                        .concat(time_minute)
                                                        .concat(String::from(":"))
                                                        .concat(String::from("00"));
                                                    ParseResult::Ok(
                                                        time_str,
                                                        state_after_time_minute,
                                                    )
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            // it's partial time
                            match parse_time_hour(input) {
                                ParseResult::NoMatch => return ParseResult::NoMatch, // cannot happen
                                ParseResult::Err(e) => return ParseResult::Err(e),
                                ParseResult::Ok(time_hour, state_after_time_hour) => {
                                    let after_colon = advance(state_after_time_hour);
                                    match parse_time_minute(after_colon) {
                                        ParseResult::NoMatch => return ParseResult::NoMatch, // cannot happen
                                        ParseResult::Err(e) => return ParseResult::Err(e),
                                        ParseResult::Ok(time_minute, state_after_time_minute) => {
                                            let after_colon2 = advance(state_after_time_minute);
                                            match parse_time_second(after_colon2) {
                                                ParseResult::NoMatch => {
                                                    // println!("expect time-second after time-hour and time-minute");
                                                    ParseResult::Err(Error::fresh())
                                                } // can happen because we only peeked until minutes
                                                ParseResult::Err(e) => return ParseResult::Err(e),
                                                ParseResult::Ok(
                                                    time_second,
                                                    state_after_time_second,
                                                ) => {
                                                    // Check for optional time-secfrac
                                                    match parse_time_secfrac(
                                                        state_after_time_second,
                                                    ) {
                                                        ParseResult::Ok(
                                                            time_secfrac,
                                                            state_after_secfrac,
                                                        ) => {
                                                            let time_str = time_hour
                                                                .concat(String::from(":"))
                                                                .concat(time_minute)
                                                                .concat(String::from(":"))
                                                                .concat(time_second)
                                                                .concat(time_secfrac);
                                                            ParseResult::Ok(
                                                                time_str,
                                                                state_after_secfrac,
                                                            )
                                                        }
                                                        ParseResult::Err(e) => {
                                                            return ParseResult::Err(e);
                                                        }
                                                        ParseResult::NoMatch => {
                                                            let time_str = time_hour
                                                                .concat(String::from(":"))
                                                                .concat(time_minute)
                                                                .concat(String::from(":"))
                                                                .concat(time_second);
                                                            ParseResult::Ok(
                                                                time_str,
                                                                state_after_time_second,
                                                            )
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                match second_colon {
                    Optional::None => ParseResult::NoMatch,
                    Optional::Some(c2) => {
                        if *c2.eq(String::from(":")) {
                            // println!("first char not colon found when the second colon is found");
                            return ParseResult::Err(Error::fresh()); // invalid partial-time
                        } else {
                            return ParseResult::NoMatch;
                        }
                    }
                }
            }
        }
    }
}

/// full-date = date-fullyear "-" date-month "-" date-mday
#[smt_fn]
fn parse_full_date(input: State) -> ParseResult<String> {
    // peek
    let first_dash = peek(input, 4.into());
    let second_dash = peek(input, 7.into());
    match first_dash {
        Optional::None => ParseResult::NoMatch,
        Optional::Some(c1) => {
            if *c1.eq(String::from("-")) {
                match second_dash {
                    Optional::None => {
                        // try parsing it as an float
                        match parse_float(input) {
                            ParseResult::Ok(_, _) => {
                                ParseResult::NoMatch // it's an float, not a full-date
                            }
                            ParseResult::Err(e) => return ParseResult::Err(e),
                            ParseResult::NoMatch => {
                                // println!("second dash not found non existent when the first dash is found");
                                ParseResult::Err(Error::fresh())
                            }
                        }
                    } // invalid full-date
                    Optional::Some(c2) => {
                        if *c2.eq(String::from("-")) {
                            // it's full-date
                            match parse_date_fullyear(input) {
                                ParseResult::NoMatch => return ParseResult::NoMatch, // cannot happen
                                ParseResult::Err(e) => return ParseResult::Err(e),
                                ParseResult::Ok(year_str, state_after_year) => {
                                    let after_dash1 = advance(state_after_year);
                                    match parse_date_month(after_dash1) {
                                        ParseResult::NoMatch => return ParseResult::NoMatch, // cannot happen
                                        ParseResult::Err(e) => return ParseResult::Err(e),
                                        ParseResult::Ok(month_str, state_after_month) => {
                                            let after_dash2 = advance(state_after_month);
                                            match parse_date_mday(after_dash2) {
                                                ParseResult::NoMatch => {
                                                    // println!("expect date-mday after date-month and year");
                                                    ParseResult::Err(Error::fresh())
                                                } // can happen because we only peeked until month
                                                ParseResult::Err(e) => return ParseResult::Err(e),
                                                ParseResult::Ok(day_str, state_after_day) => {
                                                    // Validate day against month and year
                                                    let is_valid =
                                                        is_valid_day(month_str, day_str, year_str);
                                                    if *is_valid {
                                                        // validate month
                                                        if *month_str
                                                            .lt(String::from("01"))
                                                            .or(month_str.gt(String::from("12")))
                                                        {
                                                            // println!("invalid month range");
                                                            return ParseResult::Err(Error::fresh()); // invalid month
                                                        } else {
                                                            let date_str = year_str
                                                                .concat(String::from("-"))
                                                                .concat(month_str)
                                                                .concat(String::from("-"))
                                                                .concat(day_str);
                                                            ParseResult::Ok(
                                                                date_str,
                                                                state_after_day,
                                                            )
                                                        }
                                                    } else {
                                                        // println!("invalid day for month/year");
                                                        ParseResult::Err(Error::fresh()) // invalid day for month/year
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            match parse_float(input) {
                                ParseResult::Ok(_, _) => {
                                    ParseResult::NoMatch // it's an float, not a full-date
                                }
                                ParseResult::Err(e) => return ParseResult::Err(e),
                                ParseResult::NoMatch => {
                                    // println!("another char found instead of second dash when the first dash is found");
                                    return ParseResult::Err(Error::fresh()); // invalid full-date
                                }
                            }
                        }
                    }
                }
            } else {
                match second_dash {
                    Optional::None => ParseResult::NoMatch,
                    Optional::Some(c2) => {
                        if *c2.eq(String::from("-")) {
                            match parse_float(input) {
                                ParseResult::Ok(_, _) => {
                                    ParseResult::NoMatch // it's a float, not a full-date
                                }
                                ParseResult::Err(e) => return ParseResult::Err(e),
                                ParseResult::NoMatch => {
                                    // println!("another char found instead of first dash when the second dash is found");
                                    return ParseResult::Err(Error::fresh()); // invalid full-date
                                }
                            }
                        } else {
                            return ParseResult::NoMatch;
                        }
                    }
                }
            }
        }
    }
}

/// date-fullyear  = 4DIGIT
#[smt_fn]
fn parse_date_fullyear(input: State) -> ParseResult<String> {
    match current_char(input) {
        Optional::None => ParseResult::NoMatch,
        Optional::Some(c1) => {
            if *c1.is_digit().not() {
                // println!("first char not digit");
                return ParseResult::Err(Error::fresh()); // first char not digit
            } else {
                let after_c1 = advance(input);
                match current_char(after_c1) {
                    Optional::None => ParseResult::NoMatch,
                    Optional::Some(c2) => {
                        if *c2.is_digit().not() {
                            // println!("second char not digit");
                            return ParseResult::Err(Error::fresh()); // second char not digit
                        } else {
                            let after_c2 = advance(after_c1);
                            match current_char(after_c2) {
                                Optional::None => ParseResult::NoMatch,
                                Optional::Some(c3) => {
                                    if *c3.is_digit().not() {
                                        // println!("third char not digit");
                                        return ParseResult::Err(Error::fresh()); // third char not digit
                                    } else {
                                        let after_c3 = advance(after_c2);
                                        match current_char(after_c3) {
                                            Optional::None => ParseResult::NoMatch,
                                            Optional::Some(c4) => {
                                                if *c4.is_digit().not() {
                                                    // println!("fourth char not digit");
                                                    return ParseResult::Err(Error::fresh()); // fourth char not digit
                                                } else {
                                                    let year_str =
                                                        c1.concat(c2).concat(c3).concat(c4);
                                                    let after_c4 = advance(after_c3);
                                                    ParseResult::Ok(year_str, after_c4)
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// date-month = 2DIGIT  ; 01-12
#[smt_fn]
fn parse_date_month(input: State) -> ParseResult<String> {
    match current_char(input) {
        Optional::None => ParseResult::NoMatch,
        Optional::Some(c1) => {
            if *c1.is_digit().not() {
                // println!("first char not digit in month");
                return ParseResult::Err(Error::fresh()); // first char not digit
            } else {
                let after_c1 = advance(input);
                match current_char(after_c1) {
                    Optional::None => ParseResult::NoMatch,
                    Optional::Some(c2) => {
                        if *c2.is_digit().not() {
                            // println!("second char not digit in month");
                            return ParseResult::Err(Error::fresh()); // second char not digit
                        } else {
                            let month_str = c1.concat(c2);
                            let after_c2 = advance(after_c1);
                            ParseResult::Ok(month_str, after_c2)
                        }
                    }
                }
            }
        }
    }
}

/// date-mday = 2DIGIT  ; 01-28, 01-29, 01-30, 01-31 based on month/year
#[smt_fn]
fn parse_date_mday(input: State) -> ParseResult<String> {
    match current_char(input) {
        Optional::None => ParseResult::NoMatch,
        Optional::Some(c1) => {
            if *c1.is_digit().not() {
                // println!("first char not digit in day");
                return ParseResult::Err(Error::fresh()); // first char not digit
            } else {
                let after_c1 = advance(input);
                match current_char(after_c1) {
                    Optional::None => ParseResult::NoMatch,
                    Optional::Some(c2) => {
                        if *c2.is_digit().not() {
                            // println!("second char not digit in day");
                            return ParseResult::Err(Error::fresh()); // second char not digit
                        } else {
                            let mday_str = c1.concat(c2);
                            let after_c2 = advance(after_c1);
                            ParseResult::Ok(mday_str, after_c2)
                        }
                    }
                }
            }
        }
    }
}

/// validate day against month and year (for leap years)
#[smt_fn]
fn is_valid_day(month: String, day: String, year: String) -> Boolean {
    let day_int = day.to_int();
    let year_int = year.to_int();
    if *month.eq(String::from("02")) {
        // February
        let is_leap_year = year_int
            .modulo(Integer::from(4))
            .eq(Integer::from(0))
            .and(year_int.modulo(Integer::from(100)).ne(Integer::from(0)))
            .or(year_int.modulo(Integer::from(400)).eq(Integer::from(0)));
        if *is_leap_year {
            // Leap year: 29 days
            return day_int
                .ge(Integer::from(1))
                .and(day_int.le(Integer::from(29)));
        } else {
            // Non-leap year: 28 days
            return day_int
                .ge(Integer::from(1))
                .and(day_int.le(Integer::from(28)));
        }
    } else {
        if *month
            .eq(String::from("04"))
            .or(month.eq(String::from("06")))
            .or(month.eq(String::from("09")))
            .or(month.eq(String::from("11")))
        {
            // April, June, September, November: 30 days
            return day_int
                .ge(Integer::from(1))
                .and(day_int.le(Integer::from(30)));
        } else {
            // All other months: 31 days
            return day_int
                .ge(Integer::from(1))
                .and(day_int.le(Integer::from(31)));
        }
    }
}

/// time-delim = "T" / %x20 ; T, t, or space
#[smt_fn]
fn parse_time_delim(input: State) -> ParseResult<String> {
    match current_char(input) {
        Optional::None => ParseResult::NoMatch,
        Optional::Some(c) => {
            if *c
                .eq(String::from("T"))
                .or(c.eq(String::from("t")))
                .or(c.eq(String::from(" ")))
            {
                let after_c = advance(input);
                ParseResult::Ok(c, after_c)
            } else {
                ParseResult::NoMatch // try to parse local-date
            }
        }
    }
}

/// time-numoffset = ( "+" / "-" ) time-hour ":" time-minute
#[smt_fn]
fn parse_time_numoffset(input: State) -> ParseResult<String> {
    match current_char(input) {
        Optional::None => ParseResult::NoMatch,
        Optional::Some(c) => {
            if *c.eq(String::from("+")).or(c.eq(String::from("-"))) {
                let sign = c;
                let after_sign = advance(input);
                match parse_time_hour(after_sign) {
                    ParseResult::Ok(hour_str, after_hour) => match current_char(after_hour) {
                        Optional::None => {
                            // println!("expect colon after time-hour in time-numoffset");
                            ParseResult::Err(Error::fresh())
                        } // expect colon
                        Optional::Some(colon) => {
                            if *colon.eq(String::from(":")) {
                                let after_colon = advance(after_hour);
                                match parse_time_minute(after_colon) {
                                    ParseResult::Ok(minute_str, after_minute) => {
                                        let offset_str =
                                            sign.concat(hour_str).concat(colon).concat(minute_str);
                                        ParseResult::Ok(offset_str, after_minute)
                                    }
                                    ParseResult::Err(e) => return ParseResult::Err(e),
                                    ParseResult::NoMatch => {
                                        // println!("expect time-minute after time-hour and colon; found nothing");
                                        ParseResult::Err(Error::fresh())
                                    } // expect time-minute
                                }
                            } else {
                                // println!("expect colon after time-hour in time-numoffset");
                                ParseResult::Err(Error::fresh()) // expect colon
                            }
                        }
                    },
                    ParseResult::Err(e) => return ParseResult::Err(e),
                    ParseResult::NoMatch => {
                        // println!("expect time-hour after sign in time-numoffset; found nothing");
                        ParseResult::Err(Error::fresh())
                    } // expect time-hour
                }
            } else {
                ParseResult::NoMatch
            }
        }
    }
}
