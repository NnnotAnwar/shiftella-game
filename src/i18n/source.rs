//! Side-by-side translation sources `assets/i18n/src/*.tr` → the four runtime RON tables.
//!
//! Format (one entry = a key line followed by its texts):
//!
//! ```text
//! # comment
//! trait.night_owl.name
//! cs Sova
//! en Night Owl
//! uk Сова
//! ru Сова
//!
//! station.cheb.name
//! * Cheb
//! ```
//!
//! * A key line has no spaces and is not a language line.
//! * `cs ` / `en ` / `uk ` / `ru ` lines give the text for one language, `* ` for all four
//!   (proper nouns). Every key needs all four languages.
//! * Inside a text, `\n` is a line break. Leading / trailing spaces are trimmed.
//! * Keys must be unique across all files of the directory.

use std::collections::BTreeMap;

use super::Lang;

/// One table per language, indexed by [`Lang::index`].
pub type Tables = [BTreeMap<String, String>; 4];

fn lang_prefix(line: &str) -> Option<(Option<Lang>, &str)> {
    if let Some(rest) = line.strip_prefix("* ") {
        return Some((None, rest));
    }
    for lang in Lang::ALL {
        if let Some(rest) = line
            .strip_prefix(lang.code())
            .and_then(|r| r.strip_prefix(' '))
        {
            return Some((Some(lang), rest));
        }
    }
    None
}

fn unescape(text: &str) -> String {
    text.trim().replace("\\n", "\n")
}

/// Parses one `.tr` file into `tables`. `origin` is used in error messages.
pub fn parse_into(tables: &mut Tables, text: &str, origin: &str) -> Result<(), String> {
    let mut current: Option<(String, [Option<String>; 4], usize)> = None;

    fn finish(
        tables: &mut Tables,
        entry: Option<(String, [Option<String>; 4], usize)>,
        origin: &str,
    ) -> Result<(), String> {
        let Some((key, texts, line)) = entry else {
            return Ok(());
        };
        for lang in Lang::ALL {
            let Some(text) = &texts[lang.index()] else {
                return Err(format!(
                    "{origin}:{line}: key `{key}` has no `{}` text",
                    lang.code()
                ));
            };
            if tables[lang.index()]
                .insert(key.clone(), text.clone())
                .is_some()
            {
                return Err(format!("{origin}:{line}: duplicate key `{key}`"));
            }
        }
        Ok(())
    }

    for (n, raw) in text.lines().enumerate() {
        let line_no = n + 1;
        let line = raw.trim_end();
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let line = line.trim_start();
        if let Some((lang, value)) = lang_prefix(line) {
            let Some((key, texts, _)) = current.as_mut() else {
                return Err(format!("{origin}:{line_no}: text before any key"));
            };
            let value = unescape(value);
            if value.is_empty() {
                return Err(format!("{origin}:{line_no}: empty text for `{key}`"));
            }
            match lang {
                Some(lang) => {
                    if texts[lang.index()].replace(value).is_some() {
                        return Err(format!(
                            "{origin}:{line_no}: `{key}` has two `{}` texts",
                            lang.code()
                        ));
                    }
                }
                None => {
                    for slot in texts.iter_mut() {
                        *slot = Some(value.clone());
                    }
                }
            }
        } else {
            if line.contains(char::is_whitespace) {
                return Err(format!(
                    "{origin}:{line_no}: `{line}` is neither a key nor a `cs/en/uk/ru/*` text line"
                ));
            }
            finish(tables, current.take(), origin)?;
            current = Some((line.to_string(), Default::default(), line_no));
        }
    }
    finish(tables, current.take(), origin)
}

/// Compiles every `*.tr` file of `dir` (sorted by name).
pub fn compile_dir(dir: &str) -> Result<Tables, String> {
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("{dir}: {e}"))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "tr"))
        .collect();
    files.sort();
    if files.is_empty() {
        return Err(format!("{dir}: no .tr files"));
    }
    let mut tables: Tables = Default::default();
    for path in files {
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        parse_into(&mut tables, &text, &name)?;
    }
    Ok(tables)
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}

/// Renders one language table as the runtime RON file.
pub fn to_ron(table: &BTreeMap<String, String>, lang: Lang) -> String {
    let mut out = format!(
        "// {} — GENERATED from assets/i18n/src/*.tr by `cargo run --example content_i18n_build`.\n\
         // Do not edit by hand: edit the .tr sources (all four languages side by side) and regenerate.\n{{\n",
        lang.native_name()
    );
    for (key, value) in table {
        out.push_str(&format!(
            "    \"{}\": \"{}\",\n",
            escape(key),
            escape(value)
        ));
    }
    out.push_str("}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_entries_and_all_shortcut() {
        let mut t: Tables = Default::default();
        let src = "# c\nkey.a\ncs A\nen B\nuk Д\nru Б\n\nkey.b\n* Cheb\n";
        parse_into(&mut t, src, "t.tr").unwrap();
        assert_eq!(t[Lang::Uk.index()]["key.a"], "Д");
        assert_eq!(t[Lang::Ru.index()]["key.a"], "Б");
        assert_eq!(t[Lang::Ru.index()]["key.b"], "Cheb");
    }

    #[test]
    fn missing_language_is_an_error() {
        let mut t: Tables = Default::default();
        let err = parse_into(&mut t, "key.a\ncs A\nen B\nuk C\n", "t.tr").unwrap_err();
        assert!(err.contains("ru"), "{err}");
    }

    #[test]
    fn round_trips_through_ron() {
        let mut t: Tables = Default::default();
        parse_into(&mut t, "k.q\n* Say \"hi\"\\nback\\slash\n", "t.tr").unwrap();
        let ron_text = to_ron(&t[0], Lang::Cs);
        let back = super::super::parse_table(&ron_text).unwrap();
        assert_eq!(back["k.q"], "Say \"hi\"\nback\\slash");
    }
}
