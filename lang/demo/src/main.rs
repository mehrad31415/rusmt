use rusmart_smt_derive::derive;
use rusmart_utils::config::WKS;
use std::path::PathBuf;

fn main() {
    // The input file is lang/demo and the output file is studio/native/demo from the root workspace of the project.
    let path = env!("CARGO_MANIFEST_DIR");
    let path = PathBuf::from(path).join("src/temp"); // comment this line out at the end

    match derive(path, WKS.studio.join("demo")) {
        Ok(()) => {
            println!("ok");
        },
        Err(e) => panic!("{}", e),
    }
}
