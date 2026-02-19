use rusmart_smt_remark_derive::smt_fn;
use rusmart_smt_stdlib::{Boolean, Integer, Seq};

#[smt_fn]
pub fn seq_all(s: Seq<Integer>, t: Seq<Integer>, x: Integer, y: Integer) -> Boolean {
    let _new: Seq<Integer> = Seq::new();
    let _unit: Seq<Integer> = Seq::unit(x);
    let _len: Integer = Seq::length(s);
    let _append: Seq<Integer> = Seq::append(s, x);
    let _concat: Seq<Integer> = Seq::concat(t, Seq::unit(y));
    let _at: Integer = Seq::at(Seq::unit(x), Integer::from(0));
    let _at_seq: Seq<Integer> = Seq::at_seq(Seq::unit(x), Integer::from(0));
    let _extract: Seq<Integer> = Seq::extract(Seq::concat(Seq::unit(x), Seq::unit(y)), Integer::from(0), Integer::from(1));
    let _index_of: Integer = Seq::index_of(Seq::concat(Seq::unit(x), Seq::unit(y)), Seq::unit(y), Integer::from(0));
    let _index_of_default: Integer = Seq::index_of_default(Seq::concat(Seq::unit(x), Seq::unit(y)), Seq::unit(y));
    let _contains: Boolean = Seq::contains(Seq::unit(x), x);
    let _prefix: Boolean = Seq::prefix_of(Seq::unit(x), Seq::concat(Seq::unit(x), Seq::unit(y)));
    let _suffix: Boolean = Seq::suffix_of(Seq::unit(y), Seq::concat(Seq::unit(x), Seq::unit(y)));
    let _replace: Seq<Integer> = Seq::replace(Seq::concat(Seq::unit(x), Seq::unit(x)), x, y);
    let _is_empty: Boolean = Seq::is_empty(Seq::<Integer>::new());

    Boolean::from(true)
}

