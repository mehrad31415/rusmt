use rusmart_smt_derive::derive;
use rusmart_utils::config::WKS;

fn main() {
    // println!("{}", env!("CARGO_MANIFEST_DIR"));
    // The input file is ../.....lang/demo
    // the output file is ..../studio/native/demo
    // get current working directory
    let p = std::env::current_dir().unwrap();
    // join it with lang/demo/src/temp
    let path = p.join("lang/demo/src/temp");
    println!("{}", path.display());
    match derive(path, WKS.studio.join("demo")) {
        Ok(()) => {
            println!("ok");
        },
        Err(e) => panic!("{}", e),
    }
}
