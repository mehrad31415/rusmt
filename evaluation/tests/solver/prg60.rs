use std::vec;

use rusmart_smt_remark_derive::{smt_axiom, smt_impl, smt_spec, smt_type};
use rusmart_smt_stdlib::{
    Boolean, Cloak, Error, Integer, Map, Rational, Seq, Set, Text, exists, forall, smt::SMT,
};

#[smt_impl]
fn x_impl() -> Boolean {
    let x = Boolean::from(true);
    let x_not = x.not();
    let x_and = x.and(x_not);
    let x_or = x.or(x_not);
    let x_xor = x.xor(x_not);
    let x_implies = x.implies(x_not);
    let x_iff = x.iff(x_not);
    // print all
    // println!(
    //     "x, x_not, x_and, x_or, x_xor, x_implies, x_iff: {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}",
    //     x, x_not, x_and, x_or, x_xor, x_implies, x_iff
    // );

    let y = Integer::from(2);
    let y_rational = y.to_rational();
    let y_pow = y.pow(Integer::from(2));
    let y_abs = y.abs();
    let y_add = y.add(Integer::from(10));
    let y_sub = y.sub(Integer::from(10));
    let y_mul = y.mul(Integer::from(2));
    let y_div = y.div(Integer::from(2));
    let y_rem = y.rem(Integer::from(3));
    let y_lt = y.lt(Integer::from(50));
    let y_le = y.le(Integer::from(50));
    let y_ge = y.ge(Integer::from(30));
    let y_gt = y.gt(Integer::from(30));
    // // print all
    // println!("y, y_rational, y_pow, y_abs, y_add, y_sub, y_mul, y_div, y_rem, y_lt, y_le, y_ge, y_gt: {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}",
    //          y, y_rational, y_pow, y_abs, y_add, y_sub, y_mul, y_div, y_rem, y_lt, y_le, y_ge, y_gt);

    let z = Rational::from(3.5);
    let z_pow = z.pow(Rational::from(2.3));
    let z_abs = z.abs();
    let z_add = z.add(Rational::from(2));
    let z_sub = z.sub(Rational::from(1));
    let z_mul = z.mul(Rational::from(2));
    let z_div = z.div(Rational::from(2));
    let z_lt = z.lt(Rational::from(5));
    let z_le = z.le(Rational::from(5));
    let z_ge = z.ge(Rational::from(1));
    let z_gt = z.gt(Rational::from(1));
    // // print all
    // println!("z, z_pow, z_abs, z_add, z_sub, z_mul, z_div, z_lt, z_le, z_ge, z_gt: {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}",
    //          z, z_pow, z_abs, z_add, z_sub, z_mul, z_div, z_lt, z_le, z_ge, z_gt);

    let z_round = z.round();
    let z_floor = z.floor();
    let z_ceil = z.ceil();
    // // print all
    // println!(
    //     "z_round, z_floor, z_ceil: {:?}, {:?}, {:?}",
    //     z_round, z_floor, z_ceil
    // );

    let w = Text::from("Hello");
    let w_le = w.le(Text::from("World"));
    let w_ge = w.ge(Text::from("Aloha"));
    let w_lt = w.lt(Text::from("World"));
    let w_gt = w.gt(Text::from("Aloha"));
    let w_concat = w.concat(Text::from(" Rust!"));
    let w_length = w.length();
    let w_at_index = w.at_index(Integer::from(1));
    let w_contains = w.contains(Text::from("lo"));
    let w_starts_with = w.starts_with(Text::from("He"));
    let w_ends_with = w.ends_with(Text::from("Rust!"));
    // // print all
    // println!("w, w_le, w_ge, w_lt, w_gt, w_concat, w_length, w_at_index, w_contains, w_starts_with, w_ends_with: {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}",
    //          w, w_le, w_ge, w_lt, w_gt,
    //          w_concat, w_length, w_at_index, w_contains, w_starts_with,
    //          w_ends_with);

    let e = Error::fresh();
    let e2 = Error::fresh();
    let e_merge = e.merge(e2);
    // // print all
    // println!("e, e2, e_merge: {:?}, {:?}, {:?}", e, e2, e_merge);

    let cloak: Cloak<Integer> = Cloak::shield(Integer::from(1));
    let reveal_cloak = cloak.reveal();
    // // print all
    // println!("cloak, reveal_cloak: {:?}, {:?}", cloak, reveal_cloak);

    let sequence: Seq<Integer> = Seq::new();
    let sequence_push: Seq<Integer> = sequence.append(Integer::from(1));
    let sequence_length = sequence_push.length();
    let sequence_at_index = sequence_push.at_unchecked(Integer::from(0));
    let s_includes = sequence_push.includes(Integer::from(1));
    // let s_iterator = sequence_push.iterator();
    let s_isempty = sequence_push.is_empty();
    // // print all
    // println!("sequence, sequence_push, sequence_length, sequence_at_index, s_includes, s_iterator, s_isempty: {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}",
    //          sequence, sequence_push, sequence_length, sequence_at_index, s_includes, s_iterator, s_isempty);

    let set: Set<Integer> = Set::new();
    let set_push: Set<Integer> = set.insert(Integer::from(1));
    let set_length = set_push.length();
    let set_contains = set_push.contains(Integer::from(1));
    let s_remove: Set<Integer> = set_push.remove(Integer::from(1));
    let s_isempty2 = set_push.is_empty();
    let s_contains2 = set_push.contains(Integer::from(2));
    // let s_itera = set_push.iterator();
    let s_is_subset = set_push.is_subset(set);
    let s_union: Set<Integer> = set_push.union(set);
    let s_intersection: Set<Integer> = set_push.intersection(set);
    let s_difference: Set<Integer> = set_push.difference(set);
    // // print all
    // println!("set, set_push, set_length, set_contains, s_remove, s_isempty2, s_contains2, s_itera, s_is_subset, s_union, s_intersection, s_difference: {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}",
    //          set, set_push, set_length, set_contains, s_remove, s_isempty2, s_contains2, s_itera, s_is_subset, s_union, s_intersection, s_difference);

    let map: Map<Integer, Text> = Map::new();
    let map_put: Map<Integer, Text> = map.put_unchecked(Integer::from(1), Text::from("one"));
    let map_length = map_put.length();
    let map_put222: Map<Integer, Text> = map_put.put_unchecked(Integer::from(2), Text::from("two"));
    let map_get = map_put.get_unchecked(Integer::from(1));
    let map_del: Map<Integer, Text> = map_put.del_unchecked(Integer::from(1));
    let map_contains = map_put.contains_key(Integer::from(1));
    // let map_iterator = map_put.iterator();
    let map_is_empty = map_put.is_empty();
    // // print all
    // println!("map, map_put, map_length, map_get, map_del, map_contains, map_iterator, map_is_empty: {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}",
    //          map, map_put, map_length, map_get, map_del, map_contains, map_iterator, map_is_empty);

    x
}

#[smt_spec(impls = x_impl)]
fn x_spec() -> Boolean {
    unimplemented!()
}

#[smt_axiom]
fn ax() -> Boolean {
    x_spec().implies(true.into())
}
