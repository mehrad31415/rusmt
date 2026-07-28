//! Runner: the Rust `toml` crate. Prints `OK` or `ERR <class>`.
fn main() {
    let path = std::env::args().nth(1).expect("usage: runner <file>");
    let src = std::fs::read_to_string(&path).expect("readable input");
    match src.parse::<toml::Table>() {
        Ok(_) => println!("OK"),
        Err(e) => {
            let m = e.message().replace('\n', " ");
            println!("ERR {}", &m[..m.len().min(60)]);
        }
    }
}
