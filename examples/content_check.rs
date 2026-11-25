//! Loads all content + translations, prints counts and the validation result.
//!
//! ```sh
//! cargo run --example content_check
//! ```

use shiftella_game::content::Content;
use shiftella_game::i18n::{I18n, Lang, format_money};

fn main() {
    let root = env!("CARGO_MANIFEST_DIR");
    let i18n = match I18n::load(&format!("{root}/assets/i18n")) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("i18n: {e}");
            std::process::exit(1);
        }
    };
    let content = match Content::load(&format!("{root}/assets/data")) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("content: {e}");
            std::process::exit(1);
        }
    };
    println!("content: {}", content.summary());
    for lang in Lang::ALL {
        println!(
            "i18n {} ({}): {} keys · debt {} · {}",
            lang.short_label(),
            lang.native_name(),
            i18n.len(lang),
            format_money(lang, content.story.debt),
            i18n.trn(lang, "ui.count.guests", 3, &[]),
        );
    }
    println!("required keys: {}", content.required_keys().len());
    match content.validate(&i18n) {
        Ok(()) => println!("validation: OK"),
        Err(problems) => {
            println!("validation: {} problems", problems.len());
            for p in &problems {
                println!("  - {p}");
            }
            std::process::exit(1);
        }
    }
}
