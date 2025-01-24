### Derive

We will start analyzing the _rusmart-smt-derive_ package differently. This package has one root library crate. The content of the _Cargo.toml_ file is shown below:

```toml
[package]
name = "rusmart-smt-derive"
description = "SMT model derivation from Rust code"
version = "0.1.0"
edition = "2021"
authors = ["Meng Xu <meng.xu.cs@uwaterloo.ca>"]
license = "GPL-3.0"
 
[dependencies]
anyhow = { workspace = true }
command-group = { workspace = true }
itertools = { workspace = true }
lazy_static = { workspace = true }
log = { workspace = true }
tempfile = { workspace = true }
petgraph = { workspace = true }
proc-macro2 = { workspace = true }
quote = { workspace = true }
syn = { workspace = true }
walkdir = { workspace = true }
rusmart-cli = { workspace = true }
rusmart-utils = { workspace = true }
```
The _package_ descriptions and dependencies can be found in the _Cargo.toml_ file. The _lib.rs_ file contains the module tree system of the crate as following:

```rust
mod analysis;
mod backend;
mod ir;
mod parser;

...omitted...
```

The _lib.rs_ contains two important functions: _model_ and _derive_. The _model_ function receives a path to a file and constructs a vector of __Intermediate Representation (IR)__ objects. The _derive_ function receives a path to the input file (the one given to the _model_ function) and a path to the output file. The _derive_ function internally calls the _model_ function to get the IR objects and then calls the _backend_ module to generate the SMT model and solve it. As seen in the _lib.rs_ there are four modules in the crate. The content of each module is under the respective subdirectory. The _mod.rs_ file in each subdirectory contains the module tree system of the subdirectory. For example, the _mod.rs_ file in the _parser_ subdirectory is shown below:

```rust
pub mod ctxt;
mod err;
mod test;

mod attr;
pub mod name;

pub mod generics;
pub mod infer;
pub mod ty;

mod apply;
pub mod func;

pub mod path;

mod adt;
mod dsl;
pub mod expr;
pub mod intrinsics;
```

The _parser_ module contains 13 submodules. Instead of analyzing all the modules, we will provide a high level description of what the parser module is doing. The main logic and flow can be found in the _ctxt_ module. The path to the file containing the _Rusmart_ code is taken as input. The Context::new (input) function is called to create a new context object. The way this is done is by storing annotated functions, struct, and enums of the rust source code. To emphasize, only the structs and enums that are annotated with an _#\[smt\_type\]_ attribute are stored in the context object. The functions that are annotated with _#\[smt\_impl\]_, _#\[smt\_axiom\]_, and _#\[smt\_spec\]_ are also stored. Also note that code stored in local modules are stored in the context object. Some sanity checks are performed on the stored data. The context object is then returned.