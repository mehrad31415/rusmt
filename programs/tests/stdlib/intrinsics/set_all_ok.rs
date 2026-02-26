use rusmart_smt_remark_derive::smt_fn;
use rusmart_smt_stdlib::{Boolean, Integer, Set};

#[smt_fn]
pub fn set_all(a: Set<Integer>, b: Set<Integer>, x: Integer, y: Integer) -> Boolean {
    let _new: Set<Integer> = Set::<Integer>::new();
    let _len: Integer = Set::length(a);
    let _insert: Set<Integer> = Set::insert(a, x);
    let _remove: Set<Integer> = Set::remove(b, y);
    let _contains: Boolean = Set::contains(Set::insert(Set::<Integer>::new(), x), x);
    let _is_empty: Boolean = Set::is_empty(Set::<Integer>::new());

    let _intersection: Set<Integer> = Set::intersection(
        Set::insert(Set::<Integer>::new(), x),
        Set::insert(Set::<Integer>::new(), y),
    );
    let _union: Set<Integer> = Set::union(
        Set::insert(Set::<Integer>::new(), x),
        Set::insert(Set::<Integer>::new(), y),
    );
    let _difference: Set<Integer> = Set::difference(
        Set::insert(Set::<Integer>::new(), x),
        Set::insert(Set::<Integer>::new(), y),
    );
    let _symdiff: Set<Integer> = Set::symmetric_difference(
        Set::insert(Set::<Integer>::new(), x),
        Set::insert(Set::<Integer>::new(), y),
    );

    let _subset: Boolean =
        Set::is_subset(Set::<Integer>::new(), Set::insert(Set::<Integer>::new(), x));
    let _proper_subset: Boolean =
        Set::is_proper_subset(Set::<Integer>::new(), Set::insert(Set::<Integer>::new(), x));
    let _disjoint: Boolean = Set::is_disjoint(
        Set::insert(Set::<Integer>::new(), x),
        Set::insert(Set::<Integer>::new(), y),
    );
    let _has_size: Boolean = Set::has_size(Set::insert(Set::<Integer>::new(), x), Integer::from(1));

    Boolean::from(true)
}
