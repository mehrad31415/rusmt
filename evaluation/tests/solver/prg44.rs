use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{exists, forall, Boolean, Integer, Seq, smt::SMT};

#[smt_impl]
fn _identity_(x1: Seq<Integer>) -> Seq<Integer> {
    let x2: Seq<Integer> = Seq::new(); // incomplete type error if explicit type not mentioned
    let a = x1.at_unchecked(Integer::from(0));
    let x3: Seq<Integer> = x2.append(a);
    x3
}

#[smt_spec(impls = _identity_)]
fn _identity_spec_(x1: Seq<Integer>) -> Seq<Integer> {
    unimplemented!()
}

#[smt_axiom]
fn _identity_axiom_(x: Seq<Integer>) -> Boolean {
    if *x.length().eq(Integer::from(1)) {
        _identity_spec_(x).eq(x)
    } else {
        Boolean::from(false)
    }
}
