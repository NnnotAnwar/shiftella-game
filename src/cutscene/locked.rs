//! Why a story choice option can't be picked right now: the one-line reason shown under a
//! greyed-out option (`ChoiceOption::locked_hint`, else `<label_key>.locked`, else one derived
//! from the failing condition, else `story_ui.locked.generic`).

use crate::content::{ChoiceOption, Condition};
use crate::settings::Locale;

/// A reason, still as keys: translated when drawn (`{what}` = the names, translated and joined).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockReason {
    pub key: String,
    /// Text keys of the names filling `{what}` ("furniture.armchair.name").
    pub what: Vec<String>,
    /// Joins the names (", " for things needed together, " / " for alternatives).
    pub sep: &'static str,
}

impl LockReason {
    pub fn plain(key: impl Into<String>) -> LockReason {
        LockReason {
            key: key.into(),
            what: Vec::new(),
            sep: ", ",
        }
    }

    fn names(key: &str, what: Vec<String>, sep: &'static str) -> LockReason {
        LockReason {
            key: key.to_string(),
            what,
            sep,
        }
    }

    pub fn text(&self, loc: &Locale) -> String {
        if self.what.is_empty() {
            return loc.tr_owned(&self.key);
        }
        let names: Vec<String> = self.what.iter().map(|k| loc.tr_owned(k)).collect();
        loc.trf(&self.key, &[("what", &names.join(self.sep))])
    }
}

/// Generic reason keys (all four languages in `story_ui.tr`).
pub const GENERIC: &str = "story_ui.locked.generic";
pub const FURNITURE: &str = "story_ui.locked.furniture";
pub const FURNITURE_ANY: &str = "story_ui.locked.furniture_any";
pub const TRAIT: &str = "story_ui.locked.trait";
pub const STAFF: &str = "story_ui.locked.staff";
pub const MONEY: &str = "story_ui.locked.money";
pub const TRUST: &str = "story_ui.locked.trust";
pub const ITEM: &str = "story_ui.locked.item";
pub const MENU: &str = "story_ui.locked.menu";

/// Every key [`reason`] may produce (validated in all languages).
pub const KEYS: [&str; 9] = [
    GENERIC,
    FURNITURE,
    FURNITURE_ANY,
    TRAIT,
    STAFF,
    MONEY,
    TRUST,
    ITEM,
    MENU,
];

/// The reason for a locked `opt`. `failing` are its top-level conditions that don't hold;
/// `has` says whether a text key exists.
pub fn reason(
    opt: &ChoiceOption,
    failing: &[&Condition],
    has: impl Fn(&str) -> bool,
) -> LockReason {
    if let Some(k) = &opt.locked_hint {
        return LockReason::plain(k.clone());
    }
    let conv = opt.conventional_hint_key();
    if has(&conv) {
        return LockReason::plain(conv);
    }
    derive(failing)
}

/// A short generic reason from the failing conditions (the first kind that says something).
pub fn derive(failing: &[&Condition]) -> LockReason {
    let mut flat: Vec<&Condition> = Vec::new();
    for c in failing {
        match c {
            Condition::All(v) => flat.extend(v.iter()),
            c => flat.push(c),
        }
    }
    // furniture needed together: every missing piece in one line
    let furniture: Vec<String> = flat
        .iter()
        .filter_map(|c| match c {
            Condition::HasFurniture(id) => Some(format!("furniture.{id}.name")),
            _ => None,
        })
        .collect();
    if !furniture.is_empty() {
        return LockReason::names(FURNITURE, furniture, ", ");
    }
    for c in &flat {
        if let Some(r) = one(c) {
            return r;
        }
    }
    LockReason::plain(GENERIC)
}

fn one(c: &Condition) -> Option<LockReason> {
    Some(match c {
        Condition::HasFurnitureTag(_) | Condition::FurnitureTagCount(..) => {
            LockReason::plain(FURNITURE_ANY)
        }
        Condition::StaffWithTrait(t) => LockReason::names(TRAIT, vec![trait_key(t)], " / "),
        // "a kind or a chatty colleague"
        Condition::Any(v)
            if !v.is_empty() && v.iter().all(|c| matches!(c, Condition::StaffWithTrait(_))) =>
        {
            let what = v
                .iter()
                .filter_map(|c| match c {
                    Condition::StaffWithTrait(t) => Some(trait_key(t)),
                    _ => None,
                })
                .collect();
            LockReason::names(TRAIT, what, " / ")
        }
        Condition::StaffArchetype(a) => {
            LockReason::names(STAFF, vec![format!("staff.{a}.role")], " / ")
        }
        Condition::MoneyAtLeast(_) => LockReason::plain(MONEY),
        Condition::TrustAtLeast(r, _) => {
            LockReason::names(TRUST, vec![format!("regular.{r}.name")], ", ")
        }
        Condition::HasItem(i) => LockReason::names(ITEM, vec![format!("item.{i}.name")], ", "),
        Condition::MenuHas(m) => LockReason::names(MENU, vec![format!("menu.{m}.name")], ", "),
        _ => return None,
    })
}

fn trait_key(t: &str) -> String {
    format!("trait.{t}.name")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opt(conds: Vec<Condition>) -> ChoiceOption {
        ChoiceOption {
            label_key: "scene.x.01".into(),
            effects: vec![],
            set_flag: None,
            conditions: conds,
            locked_hint: None,
            hidden_when_locked: false,
        }
    }

    #[test]
    fn explicit_hint_then_convention_then_derived() {
        let c = Condition::HasFurniture("armchair".into());
        let mut o = opt(vec![c.clone()]);
        o.locked_hint = Some("my.hint".into());
        assert_eq!(reason(&o, &[&c], |_| true).key, "my.hint");
        o.locked_hint = None;
        assert_eq!(
            reason(&o, &[&c], |k| k == "scene.x.01.locked").key,
            "scene.x.01.locked"
        );
        let r = reason(&o, &[&c], |_| false);
        assert_eq!(r.key, FURNITURE);
        assert_eq!(r.what, vec!["furniture.armchair.name".to_string()]);
    }

    #[test]
    fn derives_from_the_condition_kind() {
        let lamp = Condition::HasFurniture("floor_lamp".into());
        let chair = Condition::HasFurniture("armchair".into());
        let r = derive(&[&chair, &lamp]);
        assert_eq!(r.what.len(), 2, "both missing pieces in one line");
        let traits = Condition::Any(vec![
            Condition::StaffWithTrait("kind".into()),
            Condition::StaffWithTrait("chatterbox".into()),
        ]);
        let r = derive(&[&traits]);
        assert_eq!(r.key, TRAIT);
        assert_eq!(r.sep, " / ");
        assert_eq!(derive(&[&Condition::MoneyAtLeast(100)]).key, MONEY);
        assert_eq!(derive(&[&Condition::Flag("x".into())]).key, GENERIC);
    }
}
