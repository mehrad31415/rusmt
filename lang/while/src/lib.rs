mod eval;

use rusmart_smt_derive::derive;
use rusmart_utils::config::WKS;
use std::{collections::BTreeMap, path::PathBuf};

/// run takes a path to an rusmart file and generates a BTreeMap of model to their corresponding result when z3 is run (sat/unsat/timeout/unknown)
pub fn run(path: PathBuf) -> BTreeMap<String, String> {
    // studio/native/rego from the root workspace = WKS.studio.join("while")
    match derive(path, WKS.studio.join("while")) {
        Ok(map) => {
            println!("ok");
            map
        }
        Err(e) => panic!("{}", e),
    }
}
