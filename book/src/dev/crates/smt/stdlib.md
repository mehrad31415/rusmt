### Rusmart Standard Library (stdlib)

Rusmart standard library (_stdlib_) contains language constructs that cannot be expressed readily in Rust as they have special semantics in SMT. The _rusmart-smt-stdlib_ package consists of one _library crate_. The root of the library crate, _src/lib.rs_, contains the following content:

```rust
/// SMT-related data types
mod dt;
/// SMT-related expressions
mod exp;

/// Re-export SMT-related data types
/// This allows users to call `rusmart_stdlib::Boolean` instead of `rusmart_stdlib::dt::Boolean`
pub use dt::*;
/// Re-export SMT-related expressions
/// This allows users to call `rusmart_stdlib::forall` instead of `rusmart_stdlib::exp::forall`
pub use exp::*;
```

The crate contains two modules: `dt` and `exp`. The `dt` module contains data types part of the type system in Rusmart, while the `exp` module contains expressions. Both modules are re-exported in the root of the crate to allow users to use data types and expressions directly.

#### Trait

- `SMT`: marks that a Rust type is also an SMT type.
  In order to bridge the semantic gap between a Rust type and an SMT type,
  the `SMT` trait encodes several restrictions:

    - Rust types implementing the `SMT` trait
      must also implement other traits (these are the supertraits of `SMT`):
        - `Copy`: as SMT values are processed by value and never by reference.
        - `Default`: to allow quantified expressions
          (including `choose` operators)
          to type-check in Rust type system.
          The implementation can be arbitrary (including `panic!`)
          as it won't be executed concretely.
        - `Hash`: to allow SMT values to be used as keys in a `HashSet` or `HashMap`.
        - `Send` and `Sync`: to allow SMT values to be sent across threads. The `Send`  indicates that ownership of values of the type implementing Send can be transferred between threads. The `Sync` marker trait indicates that it is safe for the type implementing Sync to be referenced from multiple threads. 
    - Rust types implementing the `SMT` trait
      will also need to implement the following functions
      that are generally supported on SMT values:
        - `_cmp`: comparison test
        - `eq`: equality test
        - `ne`: non-equality test
        - The `ne` and `eq` methods have default implementations that use the `_cmp` method. The `_cmp` method is used to compare two values of the type implementing the `SMT` trait. The `_cmp` method returns an `Ordering` value. The `Ordering` enum is defined in the standard library and has the following variants: `Less`, `Equal`, and `Greater`. The `eq` method returns true if the `_cmp` method returns `Equal`. The `ne` method returns true if the `_cmp` method returns `Less` or `Greater`. The `_cmp` method is required to be defined by the type implementing the `SMT` trait.

#### Data types

These data types are part of the [type system](../../../user/typing.md) in Rusmart:

- `Boolean`: A wrapper around the Rust boolean type. The definition of the `Boolean` type is as follows:

```rust
pub struct Boolean {
    inner: bool,
}
```
Note that this approach of wrapping inside a struct with an `inner` field is a common way in the libraries of Rust itself and this approach is used in the Rusmart standard library as well.

- `Integer`: A wrapper around the Rust signed and unsigned integer types from 8 to 128 bits.
- `Rational`: A wrapper around the Rust signed and unsigned integer types from 8 to 128 bits along with f32 and f64.
- `Text`: A wrapper around the Rust `String` type.
- `Cloak<T>`: A wrapper over `T` to allow recursive data types to be defined (similar to `Box<T>` in Rust). A `Cloak<T>` will be uncloaked to `T` after the parsing stage of Rusmart.
- `Seq<T>`: SMT sequence of type `T` similar to Rust `Vec<T>`.
- `Set<T>`: SMT set of type `T` similar to Rust `BTreeSet<T>`.
- `Map<K, V>`: SMT array of key type `K` and value type `V`, similar to Rust `BTreeMap<K, V>`.
- `Error`: A special marker to indicate error states. The error state is created by calling the `Error::fresh()` function. Every time the `fresh()` method is called, a new error state is created with a unique inner value. The inner values are incremented by one each time a new error state is created.

## Expressions

- `forall |v1: T1, v2: T2, ..., vn: Tn| <predicate>(v1, v2, ..., vn)`
    - **SMT only**: universally quantified over potentially unbounded types

- `forall |v1 in c1, v2 in c2, ..., vn in cn| <predicate>(v1, v2, ..., vn)`
    - **SMT and Rust**: universally quantified over bounded collections
    - In Rust, the `<predicate>` are checked in a loop
      iterating over all possible combination of variables `v1, v2, ..., vn`.

Basically, the _forall_ macro has two forms:

    - `forall! (|v1: T1, v2: T2, ..., vn: Tn| <predicate>)`
    - `forall! (v1 in c1, v2 in c2, ..., vn in cn => <predicate>)`

The first form, replaces the variables with the default values of the types. For Integer, the default value is 0, for Boolean, the default value is false, and so on. It then checks the predicate for the default values of the variables. If the predicate is true for the default values, then the forall macro is true. In the second form, the _c1_, _c2_, ..., _cn_ are collections that have an _iterator_ method. A cartesian product of the collections is taken and the predicate is checked for each combination of the variables. If the predicate is true for **all** the combinations, then the forall macro is true.

- `exists |v1: T1, v2: T2, ..., vn: Tn| <predicate>(v1, v2, ..., vn)`
    - **SMT only**: existentially quantified over potentially unbounded types

- `exists |v1 in c1, v2 in c2, ..., vn in cn| <predicate>(v1, v2, ..., vn)`
    - **SMT and Rust**: existentially quantified over bounded collections
    - In Rust, the `<predicate>` are checked in a loop
      iterating over all possible combination of variables `v1, v2, ..., vn`.

Basically, the _exists_ macro has two forms:

    - `exists! (|v1: T1, v2: T2, ..., vn: Tn| <predicate>)`
    - `exists! (v1 in c1, v2 in c2, ..., vn in cn => <predicate>)`

The first form, replaces the variables with the default values of the types. For Integer, the default value is 0, for Boolean, the default value is false, and so on. It then checks the predicate for the default values of the variables. If the predicate is true for the default values, then the exists macro is true. This is very similar to the behaviour of the first pattern of the forall macro. However, the use case of the exists macro is different. In the second form, the _c1_, _c2_, ..., _cn_ are collections that have an _iterator_ method. A cartesian product of the collections is taken and the predicate is checked for each combination of the variables. If the predicate is true for **any** of the combinations, then the exists macro is true.

- `choose |v1: T1, v2: T2, ..., vn: Tn| <predicate>(v1, v2, ..., vn)`
    - **SMT only**: choose operator over potentially unbounded types.
    - In SMT< Variables `v1, v2, ..., vn` will be defined in an axiomatized way.

- `choose |v1 in c1, v2 in c2, ..., vn in cn| <predicate>(v1, v2, ..., vn)`
    - **SMT and Rust**: choose operator over bounded collections
    - In Rust, one set of variables `v1, v2, ..., vn`
      that satisfies `<predicate>` will be returned
      by iterating over all possible combinations.
      If no such combination exists, exit with panic.
    - In SMT, variables `v1, v2, ..., vn` will be defined in an axiomatized way.

Basically, the _choose_ macro has two forms:
  
      - `choose! (|v1: T1, v2: T2, ..., vn: Tn| <predicate>)`
      - `choose! (v1 in c1, v2 in c2, ..., vn in cn => <predicate>)`

In the first form, if the default values of the variables satisfy the predicate, the default values are returned. In the second form, the _c1_, _c2_, ..., _cn_ are collections that have an _iterator_ method. A cartesian product of the collections is taken and the predicate is checked for the combinations of the variables in order. The first combination that satisfies the predicate is returned. If no such combination exists, the program panics.

The combination of these expression macros allows us to express complex logic for example getting the minimum value from a set of values as shown below:

```rust
set! { 1, 2, 3, 4, 5 }
choose! (x in set => forall! (y in set => x.lt(y).or(x.eq(y))))
```

#### Note on the `SMT` trait

As you can see for the _SMT_ trait, the supertraits are __'static, Copy, Default, Hash, Send, Sync__. The _'static_ lifetime is used to indicate that the data type is valid for the entire duration of the program. The _Copy_ trait is used to indicate that the data type can be copied by value as SMT types are not passed by reference. The _Default_ trait is used to allow quantified expressions to type-check in the Rust type system. The _Hash_ trait is used to allow SMT values to be used as keys in ordered collections like _HashSet_ or _HashMap_. The _Send_ and _Sync_ traits are used to allow SMT values to be sent across threads. The _Send_ trait indicates that ownership of values of the type implementing _Send_ can be transferred between threads. The _Sync_ marker trait indicates that it is safe for the type implementing _Sync_ to be referenced from multiple threads. As you can see, the _SMT_ trait does not implement the _Ord_ trait, even though SMT types should be comparable. This is because for the _Ord_ trait to be a supertrait, the traits _Eq_, _PartialEq_, and _PartialOrd_ should also be supertraits. This is okay, but the predefined functions of _ne_ and _eq_ of the PartialEq trait will both have the types _(&self, other: &Rhs) -> bool_, whereas for these functions we will want the types to be _(&self, other: &Rhs) -> Boolean_. Note that _Boolean_ is our self defined type that wraps the Rust boolean type. As the return types of all functions in _rusmart_ should be a type defined in _rusmart_, to be able to use the _eq_ for example, we need to wrap it inside Boolean::from. This clutters the code and makes it less readable. Therefore, we have decided to not implement the _Ord_ trait as a supertrait of the _SMT_ trait. For this reason, we have defined the method _cmp_ that returns an _Ordering_ enum and any type implementing the _SMT_ trait should define this method. The methods _ne_ and _eq_ are also defined in the _SMT_ trait and they use the _cmp_ method to compare two values of the type implementing the _SMT_ trait. This way SMT types can be compared and checked for equality and non-equality.

#### Testing

We have written unit tests for the _rusmart-smt-stdlib_ package to ensure that the data types and expressions are correctly implemented. The unit tests cover 100% of the code in the _rusmart-smt-stdlib_ crate.