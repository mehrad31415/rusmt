use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{
    choose, exists, forall, Boolean, Cloak, Error, Integer, Map, Rational, Seq, Set, Text, smt::SMT,
};

#[smt_impl]
fn seq_min(s: Seq<Boolean>) -> Seq<Boolean> {
    let x = {
        let s2: Seq<Boolean> = s.append(Boolean::from(false));
        let s3: Seq<Boolean> = s2.append(Boolean::from(false));
        let s4: Seq<Boolean> = s3.append(Boolean::from(false));
        s4
    };
    // let s2: Seq<Integer> = Seq::new(); // uncommenting this will cause an error: conflicting variable names
    // here testing that the vars in the local are not outside
    let s2: Seq<Integer> = Seq::new();
    x
}

#[smt_spec(impls = seq_min)]
fn seq_min_spec(s: Seq<Boolean>) -> Seq<Boolean> {
    unimplemented!()
}

#[smt_axiom]
fn seq_min_axiom(s: Seq<Boolean>) -> Boolean {
    // seq_min_spec(s) here is a Seq<Integer> not a Seq<Boolean>
    // if we had seq_min_spec(s) as Seq<Integer> then i could refer to the index or element
    forall!(i in seq_min_spec(s) => seq_min_spec(s).at_unchecked(i).and(Boolean::from(true)).eq(Boolean::from(false)))
}
