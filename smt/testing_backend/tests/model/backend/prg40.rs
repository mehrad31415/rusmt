use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{exists, forall, Boolean, Cloak, Error, Integer, Map, Set, Text, SMT};

#[smt_impl]
fn len_elem() -> (
    Integer,
    Integer,
    Integer,
    Boolean,
    Boolean,
    Boolean,
    Boolean,
) {
    let m: Map<Integer, Set<Integer>> = Map::new();
    let s: Set<Integer> = Set::new();
    let i = Integer::from(0);

    let new_m: Map<Integer, Set<Integer>> = m.put_unchecked(i, s);
    let r1 = new_m.get_unchecked(i).length();
    let new_s: Set<Integer> = s.insert(i);
    let new_m2: Map<Integer, Set<Integer>> = m.put_unchecked(i, new_s);
    let r2 = new_m2.get_unchecked(i).length();
    let new_m3: Map<Integer, Set<Integer>> = m.del_unchecked(i);
    let r3 = new_m3.get_unchecked(i).length();

    let b1 = m.contains_key(i);
    let b2 = new_m.contains_key(i);
    let b3 = new_m2.contains_key(i);
    let b4 = new_m3.contains_key(i);
    (r1, r2, r3, b1, b2, b3, b4)
}

#[smt_spec(impls = len_elem)]
fn len_elem_spec() -> (
    Integer,
    Integer,
    Integer,
    Boolean,
    Boolean,
    Boolean,
    Boolean,
) {
    unimplemented!()
}

#[smt_axiom]
fn ax() -> Boolean {
    len_elem_spec().eq((
        Integer::from(0),
        Integer::from(1),
        Integer::from(0),
        Boolean::from(false),
        Boolean::from(true),
        Boolean::from(true),
        Boolean::from(false),
    ))
}