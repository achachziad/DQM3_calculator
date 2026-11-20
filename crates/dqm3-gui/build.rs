//! Build script: runs *before* compiling `dqm3-gui`.
//!
//! It lists the monster portraits in `data/Sprites/` and generates
//! `sprites.rs` (in Cargo's `OUT_DIR`): a table sorted by identifier,
//! `("slime", include_bytes!(".../slime-thumb.png"))`, so the 526 images
//! are embedded in the executable without writing 526 lines by hand.

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("set by Cargo"));
    let sprites_dir = manifest.join("../../data/Sprites");
    // Rebuild when portraits are added or removed (submodule update).
    println!("cargo:rerun-if-changed={}", sprites_dir.display());

    let mut entries: Vec<(String, PathBuf)> = fs::read_dir(&sprites_dir)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {} ({e}); run `git submodule update --init`",
                sprites_dir.display()
            )
        })
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let identifier = name.strip_suffix("-thumb.png")?.to_string();
            Some((identifier, entry.path()))
        })
        .collect();
    // Sorted, so that lookups can use a binary search.
    entries.sort();

    let mut code = String::from("pub static SPRITES: &[(&str, &[u8])] = &[\n");
    for (identifier, path) in &entries {
        // `{:?}` quotes and escapes the path (Windows backslashes included).
        code.push_str(&format!(
            "    ({identifier:?}, include_bytes!({:?})),\n",
            path.canonicalize().expect("file exists")
        ));
    }
    code.push_str("];\n");

    let out = PathBuf::from(env::var("OUT_DIR").expect("set by Cargo")).join("sprites.rs");
    fs::write(out, code).expect("write sprites.rs");
}
