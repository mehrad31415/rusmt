use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{exists, forall, Boolean, Cloak, Error, Integer, Map, Set, Text, smt::SMT};

#[smt_impl]
fn len_elem() -> (Integer, Integer) {
    let m: Map<Integer, Set<Integer>> = Map::new();
    let s: Set<Integer> = Set::new();
    let i = Integer::from(0);

    let new_m: Map<Integer, Set<Integer>> = m.put_unchecked(i, s);
    let r1 = new_m.get_unchecked(i).length();
    let new_s: Set<Integer> = s.insert(i);
    let new_m2: Map<Integer, Set<Integer>> = m.put_unchecked(i, new_s);
    let r2 = new_m2.get_unchecked(i).length();

    (r1, r2)
}

#[smt_spec(impls = len_elem)]
fn len_elem_spec() -> (Integer, Integer) {
    unimplemented!()
}

#[smt_axiom]
fn ax() -> Boolean {
    // len_elem_spec()
    //     .0
    //     .eq(Integer::from(0))
    //     .and(len_elem_spec().1.eq(Integer::from(1)))
    len_elem_spec().eq((Integer::from(0), Integer::from(1)))
}
