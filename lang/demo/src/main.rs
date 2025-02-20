use rusmart_smt_derive::derive;
use rusmart_utils::config::WKS;

fn main() {
    // println!("{}", env!("CARGO_MANIFEST_DIR"));
    // The input file is ../.....lang/demo
    // the output file is ..../studio/native/demo
    match derive(env!("CARGO_MANIFEST_DIR"), WKS.studio.join("demo")) {
        Ok(()) => (),
        Err(e) => panic!("{}", e),
    }
}
