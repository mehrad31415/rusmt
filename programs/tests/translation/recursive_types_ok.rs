// Test 2: Recursive and mutually recursive types
// Tests: Self-referencing, mutual recursion, generic parameters with recursion

use rusmart_smt_remark_derive::{smt_fn, smt_type};
use rusmart_smt_stdlib::{Boolean, Cloak, Integer, smt::SMT};

// Simple recursive type - Linked List
#[smt_type]
pub enum Listx<T: SMT> {
    Nil,
    Cons(T, Cloak<Listx<T>>),
}

// Mutually recursive types - Tree and Forest
#[smt_type]
pub struct Tree<T: SMT> {
    value: T,
    kids: Forest<T>,
}

#[smt_type]
pub enum Forest<T: SMT> {
    Empty,
    Trees(Cloak<Tree<T>>, Cloak<Forest<T>>),
}

// Binary tree
#[smt_type]
pub enum BinTree<T: SMT> {
    Leaf,
    Node {
        value: T,
        left: Cloak<BinTree<T>>,
        right: Cloak<BinTree<T>>,
    },
}
