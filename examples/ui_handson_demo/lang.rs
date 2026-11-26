//! Sample strings for the demo in the four game languages (en / cs / uk / ru).
//! The real game takes these from `assets/i18n`; the widgets never hardcode text.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    En,
    Cs,
    Uk,
    Ru,
}

impl Lang {
    pub fn from_env() -> Lang {
        match std::env::var("SHIFTELLA_LANG").as_deref() {
            Ok("cs") => Lang::Cs,
            Ok("uk") | Ok("ua") => Lang::Uk,
            Ok("ru") => Lang::Ru,
            _ => Lang::En,
        }
    }
    pub fn next(self) -> Lang {
        match self {
            Lang::En => Lang::Cs,
            Lang::Cs => Lang::Uk,
            Lang::Uk => Lang::Ru,
            Lang::Ru => Lang::En,
        }
    }
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "EN",
            Lang::Cs => "CZ",
            Lang::Uk => "UA",
            Lang::Ru => "RU",
        }
    }
}

/// Pick the string for `l` from `[en, cs, uk, ru]`.
pub fn s(l: Lang, v: [&str; 4]) -> String {
    let i = match l {
        Lang::En => 0,
        Lang::Cs => 1,
        Lang::Uk => 2,
        Lang::Ru => 3,
    };
    v[i].to_string()
}
