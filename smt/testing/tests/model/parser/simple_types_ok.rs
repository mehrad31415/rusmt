use rusmart_smt_remark_derive::smt_type;
use rusmart_smt_stdlib::{Boolean, Seq, SMT};

#[smt_type]
struct SimpleBool(Boolean);

#[smt_type]
struct SimpleVec<T: SMT>(Seq<T>);
