//! Compiles every file under `templates/` into the crate: `include_str!` makes Cargo rebuild when a
//! file changes, and the directory rerun catches files added or removed.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
    println!("cargo:rerun-if-changed=templates");

    let mut files = Vec::new();
    collect(&root, &mut files);
    files.sort();

    let mut out = String::from("pub(crate) static FILES: &[(&str, &str)] = &[\n");
    for path in &files {
        let rel = path.strip_prefix(&root).expect("under templates/");
        let key = rel.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/");
        let full = path.to_string_lossy().replace('\\', "/");
        writeln!(out, "    ({key:?}, include_str!({full:?})),").unwrap();
    }
    out.push_str("];\n");

    let dest = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("files.rs");
    std::fs::write(dest, out).unwrap();
}

/// Every file under `dir`, skipping hidden ones like `.DS_Store`
fn collect(dir: &Path, files: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir).unwrap().flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            println!("cargo:rerun-if-changed={}", path.display());
            collect(&path, files);
        } else if !entry.file_name().to_string_lossy().starts_with('.') {
            files.push(path);
        }
    }
}
