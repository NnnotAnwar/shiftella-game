//! Translations CZ / EN / UA / RU (codes `cs` / `en` / `uk` / `ru`).
//!
//! * Runtime files: `assets/i18n/{cs,en,uk,ru}.ron` — one flat map `{ "key": "text" }` per language.
//! * Those files are generated from the side-by-side sources `assets/i18n/src/*.tr`
//!   (`cargo run --example content_i18n_build`), so every key always exists in all four languages.
//! * Lookup falls back `lang → en → the key itself`, so a missing string is visible but never crashes.
//! * Placeholders are `{name}`; plurals are keys with `.one` / `.few` / `.many` suffixes.
//!
//! See `docs/CONTENT.md` for the key naming scheme.

pub mod source;

use std::collections::{BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

/// A game language. The order of [`Lang::ALL`] is the order of the language picker.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
pub enum Lang {
    /// Also reads the retired `Sk` (Slovak was dropped 2026-11-26): a saved Slovak choice becomes Czech.
    #[serde(alias = "Sk")]
    Cs,
    #[default]
    En,
    Uk,
    Ru,
}

impl Lang {
    pub const ALL: [Lang; 4] = [Lang::Cs, Lang::En, Lang::Uk, Lang::Ru];

    /// File / ISO code: `cs`, `en`, `uk`, `ru`.
    pub fn code(self) -> &'static str {
        match self {
            Lang::Cs => "cs",
            Lang::En => "en",
            Lang::Uk => "uk",
            Lang::Ru => "ru",
        }
    }

    /// Label used with the owner and on the picker chips: CZ / EN / UA / RU.
    pub fn short_label(self) -> &'static str {
        match self {
            Lang::Cs => "CZ",
            Lang::En => "EN",
            Lang::Uk => "UA",
            Lang::Ru => "RU",
        }
    }

    /// The language's own name (shown in the picker, never translated).
    pub fn native_name(self) -> &'static str {
        match self {
            Lang::Cs => "Čeština",
            Lang::En => "English",
            Lang::Uk => "Українська",
            Lang::Ru => "Русский",
        }
    }

    /// Accepts `cs`/`cz`, `en`, `uk`/`ua`, `ru` (any case). The retired `sk` maps to Czech.
    pub fn from_code(code: &str) -> Option<Lang> {
        match code.trim().to_ascii_lowercase().as_str() {
            "cs" | "cz" | "sk" => Some(Lang::Cs),
            "en" => Some(Lang::En),
            "uk" | "ua" => Some(Lang::Uk),
            "ru" => Some(Lang::Ru),
            _ => None,
        }
    }

    pub fn index(self) -> usize {
        match self {
            Lang::Cs => 0,
            Lang::En => 1,
            Lang::Uk => 2,
            Lang::Ru => 3,
        }
    }

    /// Grouping separator for thousands (a no-break space for cs/uk/ru, comma for en).
    fn group_separator(self) -> &'static str {
        match self {
            Lang::En => ",",
            _ => "\u{a0}",
        }
    }

    fn decimal_separator(self) -> char {
        match self {
            Lang::En => '.',
            _ => ',',
        }
    }
}

/// Plural category. Keys use the suffixes `.one`, `.few`, `.many`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluralForm {
    One,
    Few,
    Many,
}

impl PluralForm {
    pub fn suffix(self) -> &'static str {
        match self {
            PluralForm::One => "one",
            PluralForm::Few => "few",
            PluralForm::Many => "many",
        }
    }
}

/// Plural rules for integers.
/// * en: 1 → one, otherwise many (the `.few` text in en files equals `.many`).
/// * cs: 1 → one, 2–4 → few, otherwise (0, 5+) many.
/// * uk / ru: ends in 1 but not 11 → one; ends in 2–4 but not 12–14 → few; otherwise many.
pub fn plural_form(lang: Lang, n: i64) -> PluralForm {
    let n = n.unsigned_abs();
    match lang {
        Lang::En => {
            if n == 1 {
                PluralForm::One
            } else {
                PluralForm::Many
            }
        }
        Lang::Cs => match n {
            1 => PluralForm::One,
            2..=4 => PluralForm::Few,
            _ => PluralForm::Many,
        },
        Lang::Uk | Lang::Ru => {
            let (d10, d100) = (n % 10, n % 100);
            if d10 == 1 && d100 != 11 {
                PluralForm::One
            } else if (2..=4).contains(&d10) && !(12..=14).contains(&d100) {
                PluralForm::Few
            } else {
                PluralForm::Many
            }
        }
    }
}

/// Integer with language-specific grouping: `12 480` (cs/uk/ru, no-break space) or `12,480` (en).
/// Negative numbers use the real minus sign `−`.
pub fn format_int(lang: Lang, n: i64) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    if n < 0 {
        out.push('\u{2212}');
    }
    let len = digits.len();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            out.push_str(lang.group_separator());
        }
        out.push(ch);
    }
    out
}

/// Money in Czech crowns: `1 250 Kč` (cs/uk/ru) and `1,250 CZK` (en).
pub fn format_money(lang: Lang, czk: i64) -> String {
    match lang {
        Lang::En => format!("{}\u{a0}CZK", format_int(lang, czk)),
        _ => format!("{}\u{a0}Kč", format_int(lang, czk)),
    }
}

/// Signed money for deltas: `+1 860 Kč`, `−150 Kč`.
pub fn format_money_delta(lang: Lang, czk: i64) -> String {
    if czk > 0 {
        format!("+{}", format_money(lang, czk))
    } else {
        format_money(lang, czk)
    }
}

/// Decimal number with the language's separator: `4,2` / `4.2`.
pub fn format_decimal(lang: Lang, value: f32, decimals: usize) -> String {
    let s = format!("{:.*}", decimals, value.abs());
    let s = s.replace('.', &lang.decimal_separator().to_string());
    if value < 0.0 && s.chars().any(|c| c.is_ascii_digit() && c != '0') {
        format!("\u{2212}{s}")
    } else {
        s
    }
}

/// 24-hour clock for every language (the café lives by the station timetable): `03:33`.
pub fn format_clock(hour: u32, minute: u32) -> String {
    format!("{:02}:{:02}", hour % 24, minute % 60)
}

/// Replaces `{name}` placeholders. Unknown placeholders are left untouched.
pub fn fill(template: &str, args: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        match after.find('}') {
            Some(close) => {
                let name = &after[..close];
                match args.iter().find(|(k, _)| *k == name) {
                    Some((_, v)) => out.push_str(v),
                    None => {
                        out.push('{');
                        out.push_str(name);
                        out.push('}');
                    }
                }
                rest = &after[close + 1..];
            }
            None => {
                out.push_str(&rest[open..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// Placeholder names used in a template (`{name}` → `name`).
pub fn placeholders(template: &str) -> BTreeSet<&str> {
    let mut set = BTreeSet::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        match after.find('}') {
            Some(close) => {
                let name = &after[..close];
                if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    set.insert(name);
                }
                rest = &after[close + 1..];
            }
            None => break,
        }
    }
    set
}

/// All four translation tables.
#[derive(Debug, Clone, Default)]
pub struct I18n {
    tables: [HashMap<String, String>; 4],
    /// Runtime texts that win over the tables (the café's own drink's name: `set_override`).
    overrides: [HashMap<String, String>; 4],
    /// What the overrides were last built from (the caller's hash; 0 = none).
    override_stamp: u64,
}

impl I18n {
    /// Loads `dir/{cs,en,uk,ru}.ron`.
    pub fn load(dir: &str) -> Result<I18n, String> {
        let mut i18n = I18n::default();
        for lang in Lang::ALL {
            let path = format!("{dir}/{}.ron", lang.code());
            let text = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
            i18n.tables[lang.index()] = parse_table(&text).map_err(|e| format!("{path}: {e}"))?;
        }
        Ok(i18n)
    }

    /// Builds from in-memory tables (tests, the source compiler).
    pub fn from_tables(tables: [HashMap<String, String>; 4]) -> I18n {
        I18n {
            tables,
            ..Default::default()
        }
    }

    /// A runtime text for `key` in `lang` that wins over the tables (`None` removes it).
    /// Used for player-made names (the café's own drink); never for shipped text.
    pub fn set_override(&mut self, lang: Lang, key: &str, value: Option<String>) {
        let m = &mut self.overrides[lang.index()];
        match value {
            Some(v) => {
                m.insert(key.to_string(), v);
            }
            None => {
                m.remove(key);
            }
        }
    }

    /// Drops every runtime text.
    pub fn clear_overrides(&mut self) {
        for m in &mut self.overrides {
            m.clear();
        }
        self.override_stamp = 0;
    }

    /// What the overrides were built from (the caller's hash; 0 = none).
    pub fn override_stamp(&self) -> u64 {
        self.override_stamp
    }

    pub fn set_override_stamp(&mut self, stamp: u64) {
        self.override_stamp = stamp;
    }

    pub fn table(&self, lang: Lang) -> &HashMap<String, String> {
        &self.tables[lang.index()]
    }

    /// Exact lookup without fallback.
    pub fn get(&self, lang: Lang, key: &str) -> Option<&str> {
        self.overrides[lang.index()]
            .get(key)
            .or_else(|| self.tables[lang.index()].get(key))
            .map(String::as_str)
    }

    pub fn has(&self, lang: Lang, key: &str) -> bool {
        self.overrides[lang.index()].contains_key(key)
            || self.tables[lang.index()].contains_key(key)
    }

    /// True when the key exists in all four languages.
    pub fn has_all(&self, key: &str) -> bool {
        Lang::ALL.iter().all(|l| self.has(*l, key))
    }

    /// Translation with fallback `lang → en → key`.
    pub fn tr<'a>(&'a self, lang: Lang, key: &'a str) -> &'a str {
        self.get(lang, key)
            .or_else(|| self.get(Lang::En, key))
            .unwrap_or(key)
    }

    /// Translation with `{name}` placeholders filled from `args`.
    pub fn trf(&self, lang: Lang, key: &str, args: &[(&str, &str)]) -> String {
        fill(self.tr(lang, key), args)
    }

    /// Plural key `base` (`base.one` / `base.few` / `base.many`), with `{n}` set to the
    /// formatted number (unless `args` already has `n`).
    pub fn trn(&self, lang: Lang, base: &str, n: i64, args: &[(&str, &str)]) -> String {
        let key = format!("{base}.{}", plural_form(lang, n).suffix());
        let many = format!("{base}.many");
        let en_key = format!("{base}.{}", plural_form(Lang::En, n).suffix());
        let template = self
            .get(lang, &key)
            .or_else(|| self.get(lang, &many))
            .or_else(|| self.get(Lang::En, &en_key))
            .unwrap_or(base);
        let n_text = format_int(lang, n);
        let mut all: Vec<(&str, &str)> = args.to_vec();
        if !args.iter().any(|(k, _)| *k == "n") {
            all.push(("n", &n_text));
        }
        fill(template, &all)
    }

    /// Duration like `45 min` or `2 h 15 min` using `ui.unit.*` keys.
    pub fn format_duration(&self, lang: Lang, minutes: u32) -> String {
        let (h, m) = (minutes / 60, minutes % 60);
        let (hs, ms) = (h.to_string(), m.to_string());
        if h == 0 {
            self.trf(lang, "ui.unit.minutes_short", &[("m", &ms)])
        } else if m == 0 {
            self.trf(lang, "ui.unit.hours_short", &[("h", &hs)])
        } else {
            self.trf(
                lang,
                "ui.unit.hours_minutes_short",
                &[("h", &hs), ("m", &ms)],
            )
        }
    }

    pub fn keys(&self, lang: Lang) -> impl Iterator<Item = &str> {
        self.tables[lang.index()].keys().map(String::as_str)
    }

    pub fn len(&self, lang: Lang) -> usize {
        self.tables[lang.index()].len()
    }

    pub fn is_empty(&self) -> bool {
        self.tables.iter().all(HashMap::is_empty)
    }

    /// Problems with key parity and placeholders between languages (empty = all good).
    /// * every key must exist in all four languages;
    /// * every language must use the same set of `{placeholders}` as English.
    pub fn check(&self) -> Vec<String> {
        let mut problems = Vec::new();
        let mut all_keys: BTreeSet<&str> = BTreeSet::new();
        for lang in Lang::ALL {
            all_keys.extend(self.keys(lang));
        }
        for key in all_keys {
            for lang in Lang::ALL {
                if !self.has(lang, key) {
                    problems.push(format!("i18n: key `{key}` missing in {}", lang.code()));
                }
            }
            if let Some(en) = self.get(Lang::En, key) {
                let want = placeholders(en);
                for lang in [Lang::Cs, Lang::Uk, Lang::Ru] {
                    if let Some(text) = self.get(lang, key)
                        && placeholders(text) != want
                    {
                        problems.push(format!(
                            "i18n: key `{key}` in {} has placeholders {:?}, en has {:?}",
                            lang.code(),
                            placeholders(text),
                            want
                        ));
                    }
                }
            }
        }
        problems
    }
}

/// Parses one language file: a RON map of string → string.
pub fn parse_table(text: &str) -> Result<HashMap<String, String>, String> {
    ron::from_str::<HashMap<String, String>>(text).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/i18n");

    fn table(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn sample() -> I18n {
        let en = table(&[
            ("hello", "Hello, {name}!"),
            ("only.en", "English only"),
            ("g.one", "{n} guest"),
            ("g.few", "{n} guests"),
            ("g.many", "{n} guests"),
        ]);
        let cs = table(&[
            ("hello", "Ahoj, {name}!"),
            ("g.one", "{n} host"),
            ("g.few", "{n} hosté"),
            ("g.many", "{n} hostů"),
        ]);
        let uk = table(&[
            ("g.one", "{n} гість"),
            ("g.few", "{n} гості"),
            ("g.many", "{n} гостей"),
        ]);
        let ru = table(&[
            ("g.one", "{n} гость"),
            ("g.few", "{n} гостя"),
            ("g.many", "{n} гостей"),
        ]);
        I18n::from_tables([cs, en, uk, ru])
    }

    #[test]
    fn lang_codes_round_trip() {
        for lang in Lang::ALL {
            assert_eq!(Lang::from_code(lang.code()), Some(lang));
        }
        assert_eq!(Lang::from_code("UA"), Some(Lang::Uk));
        assert_eq!(Lang::from_code("cz"), Some(Lang::Cs));
        assert_eq!(Lang::from_code("RU"), Some(Lang::Ru));
        assert_eq!(
            Lang::from_code("sk"),
            Some(Lang::Cs),
            "retired Slovak reads as Czech"
        );
        assert_eq!(Lang::from_code("de"), None);
        assert_eq!(Lang::Ru.native_name(), "Русский");
        assert_eq!(Lang::Ru.short_label(), "RU");
    }

    #[test]
    fn fallback_goes_en_then_key() {
        let i = sample();
        assert_eq!(i.tr(Lang::Cs, "hello"), "Ahoj, {name}!");
        assert_eq!(i.tr(Lang::Ru, "hello"), "Hello, {name}!");
        assert_eq!(i.tr(Lang::Cs, "only.en"), "English only");
        assert_eq!(i.tr(Lang::Uk, "nope.key"), "nope.key");
    }

    #[test]
    fn placeholders_are_filled() {
        let i = sample();
        assert_eq!(
            i.trf(Lang::Cs, "hello", &[("name", "Mirek")]),
            "Ahoj, Mirek!"
        );
        assert_eq!(fill("{a} and {b}", &[("a", "1")]), "1 and {b}");
        assert_eq!(fill("broken {", &[]), "broken {");
        let p = placeholders("{a} {b_2} {} {not valid}");
        assert_eq!(p.into_iter().collect::<Vec<_>>(), vec!["a", "b_2"]);
    }

    #[test]
    fn plural_rules() {
        use PluralForm::*;
        let cs: Vec<_> = [0, 1, 2, 4, 5, 11, 21, 22]
            .iter()
            .map(|n| plural_form(Lang::Cs, *n))
            .collect();
        assert_eq!(cs, vec![Many, One, Few, Few, Many, Many, Many, Many]);
        let uk: Vec<_> = [0, 1, 2, 4, 5, 11, 12, 21, 22, 25, 101, 111, 112]
            .iter()
            .map(|n| plural_form(Lang::Uk, *n))
            .collect();
        assert_eq!(
            uk,
            vec![
                Many, One, Few, Few, Many, Many, Many, One, Few, Many, One, Many, Many
            ]
        );
        let ru: Vec<_> = [
            0, 1, 2, 3, 4, 5, 11, 12, 14, 21, 22, 25, 101, 111, 112, 1001,
        ]
        .iter()
        .map(|n| plural_form(Lang::Ru, *n))
        .collect();
        assert_eq!(
            ru,
            vec![
                Many, One, Few, Few, Few, Many, Many, Many, Many, One, Few, Many, One, Many, Many,
                One
            ]
        );
        assert_eq!(plural_form(Lang::Ru, -21), One);
        assert_eq!(plural_form(Lang::En, 1), One);
        assert_eq!(plural_form(Lang::En, 2), Many);
        let i = sample();
        assert_eq!(i.trn(Lang::Cs, "g", 3, &[]), "3 hosté");
        assert_eq!(i.trn(Lang::Cs, "g", 5, &[]), "5 hostů");
        assert_eq!(i.trn(Lang::Uk, "g", 21, &[]), "21 гість");
        assert_eq!(i.trn(Lang::En, "g", 1, &[]), "1 guest");
        assert_eq!(i.trn(Lang::Ru, "g", 2, &[]), "2 гостя");
        assert_eq!(i.trn(Lang::Ru, "g", 11, &[]), "11 гостей");
        assert_eq!(i.trn(Lang::Ru, "g", 21, &[]), "21 гость");
        assert_eq!(i.trn(Lang::Ru, "g", 1250, &[]), "1\u{a0}250 гостей");
    }

    #[test]
    fn money_and_numbers() {
        assert_eq!(format_money(Lang::Cs, 1250), "1\u{a0}250\u{a0}Kč");
        assert_eq!(format_money(Lang::En, 1250), "1,250\u{a0}CZK");
        assert_eq!(format_money(Lang::Uk, 90000), "90\u{a0}000\u{a0}Kč");
        assert_eq!(format_money(Lang::Ru, -150), "\u{2212}150\u{a0}Kč");
        assert_eq!(format_money(Lang::Ru, 1250), "1\u{a0}250\u{a0}Kč");
        assert_eq!(format_decimal(Lang::Ru, 4.25, 1), "4,2");
        assert_eq!(format_money_delta(Lang::Cs, 1860), "+1\u{a0}860\u{a0}Kč");
        assert_eq!(format_int(Lang::En, 1_234_567), "1,234,567");
        assert_eq!(format_int(Lang::Cs, 999), "999");
        assert_eq!(format_decimal(Lang::Cs, 4.25, 1), "4,2");
        assert_eq!(format_decimal(Lang::En, 4.26, 1), "4.3");
        assert_eq!(format_decimal(Lang::En, -0.01, 1), "0.0");
        assert_eq!(format_clock(3, 33), "03:33");
    }

    #[test]
    fn real_files_load_and_have_identical_key_sets() {
        let i = I18n::load(DIR).expect("i18n files load");
        let en: BTreeSet<&str> = i.keys(Lang::En).collect();
        assert!(
            en.len() > 500,
            "expected a full vocabulary, got {}",
            en.len()
        );
        for lang in Lang::ALL {
            let keys: BTreeSet<&str> = i.keys(lang).collect();
            assert_eq!(keys, en, "key set of {} differs from en", lang.code());
        }
        let problems = i.check();
        assert!(problems.is_empty(), "{}", problems.join("\n"));
        assert_eq!(i.format_duration(Lang::En, 135), "2 h 15 min");
        assert_eq!(
            Lang::ALL.map(Lang::code),
            ["cs", "en", "uk", "ru"],
            "the game ships CZ / EN / UA / RU"
        );
        assert_eq!(i.format_duration(Lang::Ru, 135), "2 ч 15 мин");
    }

    #[test]
    fn no_english_placeholders_left() {
        // A text shared by all four languages is fine for names, symbols and titles
        // ("Café Nocturne", "+{n}", "Last Tram Trio") but never for an English sentence.
        let i = I18n::load(DIR).expect("i18n files load");
        let mut left = Vec::new();
        for key in i.keys(Lang::En) {
            let en = i.tr(Lang::En, key);
            if [Lang::Cs, Lang::Uk, Lang::Ru]
                .iter()
                .any(|l| i.tr(*l, key) != en)
            {
                continue;
            }
            // handles (`@shiftella_bot`) and `{placeholders}` are not words
            let prose = en
                .split_whitespace()
                .filter(|t| !t.starts_with('@') && !t.contains('{'))
                .flat_map(|t| t.split(|c: char| !c.is_alphabetic()))
                .any(|w| w.len() >= 4 && w.chars().all(|c| c.is_ascii_lowercase()));
            if prose {
                left.push(format!("{key}: {en}"));
            }
        }
        left.sort();
        assert!(left.is_empty(), "untranslated:\n{}", left.join("\n"));
    }

    #[test]
    fn generated_files_match_sources() {
        let compiled = source::compile_dir(&format!("{DIR}/src")).expect("sources compile");
        let loaded = I18n::load(DIR).expect("i18n files load");
        for lang in Lang::ALL {
            let from_src: HashMap<String, String> =
                compiled[lang.index()].clone().into_iter().collect();
            assert!(
                &from_src == loaded.table(lang),
                "assets/i18n/{}.ron is stale: run `cargo run --example content_i18n_build`",
                lang.code()
            );
        }
    }
}
