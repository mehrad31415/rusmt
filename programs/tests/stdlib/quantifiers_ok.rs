use rusmart_smt_remark_derive::smt_fn;
use rusmart_smt_stdlib::{Array, Boolean, Integer, Seq, Set, smt::SMT};
use rusmart_smt_stdlib::{choose, exists, forall};

#[smt_fn]
pub fn quantifiers_bounded() -> Boolean {
    // Bounded (Rust-iterated) quantifiers use iterator() helpers.
    let s: Set<Integer> = Set::insert(
        Set::insert(Set::<Integer>::new(), Integer::from(0)),
        Integer::from(1),
    );
    let t: Seq<Integer> = Seq::unit(Integer::from(2)).append(Integer::from(3));
    let m: Array<Integer, Integer> = Array::store(
        Array::store(
            Array::<Integer, Integer>::new(),
            Integer::from(0),
            Integer::from(0),
        ),
        Integer::from(1),
        Integer::from(1),
    );

    let a = forall!(x in s, y in t => x.le(y));
    let e = exists!(k in m => k.eq(Integer::from(1)));
    let (v,): (Integer,) = (choose!(x in s => x.eq(Integer::from(1))),);

    a.and(e).and(v.eq(Integer::from(1)))
}
