pub mod eval1;
pub mod eval2;

use rusmart_smt_derive::derive;
use rusmart_utils::config::WKS;
use std::{collections::BTreeMap, path::PathBuf};

pub fn run(path: PathBuf) -> BTreeMap<String, String> {
    println!("path inside run: {:?}", path);
    // studio/native/demo from the root workspace = WKS.studio.join("demo")
    match derive(path, WKS.studio.join("demo")) {
        Ok(map) => {
            println!("ok");
            map
        }
        Err(e) => panic!("{}", e),
    }
}
