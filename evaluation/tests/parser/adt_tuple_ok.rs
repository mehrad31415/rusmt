use rusmart_smt_remark_derive::smt_type;
use rusmart_smt_stdlib::{Boolean, Integer, smt::SMT};

#[smt_type]
struct S<T1: SMT, T2: SMT>(Boolean, Integer, T1, T2, (T1, T2));
