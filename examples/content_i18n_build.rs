//! Regenerates `assets/i18n/{cs,en,sk,uk}.ron` from the side-by-side sources
//! `assets/i18n/src/*.tr`. Run after editing any `.tr` file:
//!
//! ```sh
//! cargo run --example content_i18n_build
//! ```

use shiftella_game::i18n::{Lang, source};

fn main() {
    let root = env!("CARGO_MANIFEST_DIR");
    let src = format!("{root}/assets/i18n/src");
    let tables = match source::compile_dir(&src) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };
    for lang in Lang::ALL {
        let path = format!("{root}/assets/i18n/{}.ron", lang.code());
        let text = source::to_ron(&tables[lang.index()], lang);
        if let Err(e) = std::fs::write(&path, text) {
            eprintln!("error: {path}: {e}");
            std::process::exit(1);
        }
        println!("{path}: {} keys", tables[lang.index()].len());
    }
}
