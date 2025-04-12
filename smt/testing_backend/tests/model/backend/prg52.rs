// testing Boolean(V)
use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{
    choose, exists, forall, Boolean, Cloak, Error, Integer, Map, Rational, Seq, Set, Text, SMT,
};

#[smt_type]
enum V {
    Boolean(Boolean),
}

#[smt_impl]
fn seq_min(v: V) -> Integer {
    match v {
        V::Boolean(b) => {
            if *b {
                Integer::from(1)
            } else {
                Integer::from(0)
            }
        }
    }
}

#[smt_spec(impls = seq_min)]
fn seq_min_spec(v: V) -> Integer {
    unimplemented!()
}

#[smt_axiom]
fn seq_min_axiom(b: Boolean) -> Boolean {
    if *b {
        seq_min_spec(V::Boolean(b)).eq(Integer::from(1))
    } else {
        seq_min_spec(V::Boolean(b)).eq(Integer::from(0))
    }
}
