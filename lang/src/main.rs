use rusmart_smt_derive::derive;
use rusmart_utils::config::WKS;
use std::path::PathBuf;
use syn::Result;
use walkdir::WalkDir;

fn main() -> Result<()> {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = base.join("src");
    for entry in WalkDir::new(src) {
        let entry = entry.unwrap();
        if entry.file_type().is_dir() {
            let mod_file = entry.path().join("mod.rs");
            if mod_file.exists() {
                let path_entry = entry.path().file_name().unwrap();
                derive(mod_file, WKS.studio.join(path_entry))?;
            }
        }
    }
    Ok(())
}
