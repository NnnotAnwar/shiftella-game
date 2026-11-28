//! `Cond` and `Fx` — the hands-on extension of the scripting language.
//!
//! The core [`Condition`] / [`Effect`] enums are matched exhaustively by the sim, so the hands-on
//! update adds its new conditions and effects in two *extension* enums, [`XCondition`] and
//! [`XEffect`], and two union types that the new data files use everywhere:
//!
//! * [`Cond`] = a core [`Condition`] **or** an [`XCondition`];
//! * [`Fx`] = a core [`Effect`] **or** an [`XEffect`].
//!
//! In RON they read exactly like one enum — `[Raining, SeasonPhase("frost"), Any([Weather(Snow),
//! QuietHour])]`: the variant name decides which enum parses it. The combinators (`All`, `Any`,
//! `Not` for conditions; `Maybe`, `Later`, `If` for effects) live in the extension enums, so they
//! can mix core and new items freely.
//!
//! Semantics of every variant: `docs/CONTENT.md` §10.

use std::fmt;

use serde::de::{
    self, DeserializeSeed, Deserializer, EnumAccess, IntoDeserializer, VariantAccess, Visitor,
};
use serde::{Deserialize, Serialize, Serializer};

use super::journal::HighlightKind;
use super::recipes::Taste;
use super::records::HallMood;
use crate::content::defs::Job;
use crate::content::logic::{Condition, Effect, Ref};
use crate::content::world::ChecklistKind;

// ---------------------------------------------------------------------------------------------
// Conditions
// ---------------------------------------------------------------------------------------------

/// A core condition or a hands-on one. A `Vec<Cond>` means **all** must hold (empty = true).
#[derive(Debug, Clone, PartialEq)]
pub enum Cond {
    Core(Condition),
    X(XCondition),
}

/// Hands-on conditions (plus combinators over [`Cond`]).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum XCondition {
    All(Vec<Cond>),
    Any(Vec<Cond>),
    Not(Box<Cond>),
    /// Karolina knows this recipe.
    RecipeKnown(String),
    /// Karolina has served this regular this recipe at least once (any night).
    ServedRecipe {
        regular: String,
        recipe: String,
    },
    /// The regular's last `visits` visits each got a Karolina-made item with match ≥ `min`.
    WishMatchStreak {
        regular: String,
        min: f32,
        visits: u32,
    },
    /// The hall mood right now (from the playing record or an effect).
    HallMood(HallMood),
    /// Inside the quiet hour (`handson.ron` → `quiet_hour`).
    QuietHour,
    /// The power is out right now.
    Blackout,
    /// Some on-shift helper has energy below the value.
    StaffLowEnergy(f32),
    /// This moment has been resolved at least once (any night, any option).
    MomentResolvedEver(String),
    IngredientUnlocked(String),
    RecordUnlocked(String),
    /// The current season phase (`seasons.ron`).
    SeasonPhase(String),
    /// Hall coziness (0–100) ≥ n.
    CozinessAtLeast(f32),
    /// A story person of this archetype is employed (on shift or not).
    HiredArchetype(String),
    /// The person of this archetype has worked ≥ n shifts.
    ShiftsWorked(String, u32),
    /// Two archetypes have shared ≥ `shifts` shifts (≥ 4 h together each).
    SharedShifts {
        a: String,
        b: String,
        shifts: u32,
    },
    /// Lifetime tips in the jar ≥ CZK.
    TipsLifetimeAtLeast(u32),
    /// Thank-you napkins collected ≥ n.
    NapkinsAtLeast(u32),
    /// The decor theme is owned (bought or unlocked for free).
    ThemeOwned(String),
}

/// Variant names of [`XCondition`] (keep in sync; a test checks every name parses).
pub const XCONDITION_NAMES: &[&str] = &[
    "All",
    "Any",
    "Not",
    "RecipeKnown",
    "ServedRecipe",
    "WishMatchStreak",
    "HallMood",
    "QuietHour",
    "Blackout",
    "StaffLowEnergy",
    "MomentResolvedEver",
    "IngredientUnlocked",
    "RecordUnlocked",
    "SeasonPhase",
    "CozinessAtLeast",
    "HiredArchetype",
    "ShiftsWorked",
    "SharedShifts",
    "TipsLifetimeAtLeast",
    "NapkinsAtLeast",
    "ThemeOwned",
];

// ---------------------------------------------------------------------------------------------
// Effects
// ---------------------------------------------------------------------------------------------

/// A core effect or a hands-on one.
#[derive(Debug, Clone, PartialEq)]
pub enum Fx {
    Core(Effect),
    X(XEffect),
}

/// Where the camera eases to (`XEffect::CameraHint`; the render/integration decides how).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum CameraTarget {
    /// The anchor of the moment / checklist item that caused it.
    Anchor,
    Window,
    Door,
    Counter,
    Jukebox,
    Street,
}

/// Hands-on effects. "Target" effects act on the guest / helper a moment is anchored to
/// (`MomentAnchor::Agent`) or on the subject of a handover note / announcement.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum XEffect {
    // --- control flow (over Fx) -------------------------------------------------------------
    /// With probability `chance` apply `effects`.
    Maybe {
        chance: f32,
        effects: Vec<Fx>,
    },
    /// After `minutes` game minutes (dropped if the night ends first).
    Later {
        minutes: u32,
        effects: Vec<Fx>,
    },
    /// Evaluated when run.
    If {
        conditions: Vec<Cond>,
        then: Vec<Fx>,
        #[serde(default)]
        otherwise: Vec<Fx>,
    },
    /// Applied when the next night starts (22:00), e.g. handover notes and parcels.
    NextNight(Vec<Fx>),

    // --- knowledge & unlocks ------------------------------------------------------------------
    /// Karolina learns a recipe (`RecipeLearned` event, card toast).
    LearnRecipe(String),
    UnlockIngredient(String),
    /// Reveals one more ingredient on an undiscovered recipe's silhouette (a margin note).
    RevealIngredient(String),
    UnlockRecord(String),
    /// The theme becomes owned for free.
    UnlockTheme(String),
    UnlockLatte(String),

    // --- music & mood ---------------------------------------------------------------------------
    /// Sets the hall mood for `minutes` (`None` = until the record changes / night end).
    HallMood {
        mood: HallMood,
        minutes: Option<u32>,
    },
    /// Karolina puts on the record the moment's regular asked for.
    PlayRequestedRecord,
    /// Plays a music track through the radio (not the jukebox) for a while.
    RadioTrack {
        track: String,
        volume: f32,
        minutes: u32,
    },
    /// A busker plays outside for N minutes (ambient flavour).
    StreetMusic(u32),

    // --- moments & presentation ---------------------------------------------------------------
    SpawnMoment(String),
    CameraHint(CameraTarget),
    /// Logs the train passing now in the trainspotting logbook (LATER feature; may no-op).
    LogTrain,
    /// A journal highlight candidate (score / caption from `journal.ron`).
    Highlight(HighlightKind),
    /// A thank-you napkin next to the tip (`None` = a random eligible one).
    ThankYouNote(Option<String>),

    // --- the target guest / helper --------------------------------------------------------------
    TargetTrust(f32),
    TargetSatisfaction(f32),
    /// The target guest stays ± minutes.
    TargetStay(i32),
    /// The target guest leaves now, content.
    TargetLeave,
    TargetStaffMood(f32),
    TargetStaffEnergy(f32),
    /// The target helper takes a break now.
    TargetStaffBreak,
    /// Relationship of the target helper with Karolina ±.
    TargetRelationship(f32),
    /// Mood ± for the employed story person of this archetype.
    ArchetypeMood {
        staff: String,
        add: f32,
    },

    // --- the hall ---------------------------------------------------------------------------------
    /// Satisfaction ± for guests within `tiles` of the anchor.
    GuestSatisfactionNear {
        add: f32,
        tiles: u32,
    },
    /// Wishes with these tags are picked × `mul` more often.
    WishBoost {
        tags: Vec<Taste>,
        mul: f32,
        minutes: Option<u32>,
    },
    /// Coziness ± (0–100 scale) for `minutes` (`None` = rest of the night).
    Coziness {
        add: f32,
        minutes: Option<u32>,
    },
    /// Guest patience × `mul`.
    Patience {
        mul: f32,
        minutes: Option<u32>,
    },
    /// Candles on every table while the power is out (they count as light).
    LightCandles,
    /// A bucket on the anchor tile (blocks it, plinks).
    PlaceBucket,
    /// The bucket goes away; the tile is free again.
    EmptyBucket,
    /// The anchor window opens for N minutes (visual; closes itself).
    WindowOpen(u32),
    /// A tip into the jar, random CZK in the inclusive range.
    Tip {
        min: u32,
        max: u32,
    },

    // --- rituals, handover, stock -----------------------------------------------------------------
    /// Quick-brew quality ± for `minutes` (`None` = rest of the night).
    QuickQuality {
        add: f32,
        minutes: Option<u32>,
    },
    /// The first `count` items of `job` take × `mul` longer ("the machine is cold").
    SlowFirstItems {
        job: Job,
        count: u32,
        mul: f32,
    },
    /// Chance that one item runs out later tonight (off the menu for `minutes`).
    StockoutRisk {
        chance: f32,
        minutes: u32,
    },
    /// The opening stock check is done: auto-restock prioritises tonight's low list.
    StockChecked,
    /// The subject item (handover) is restocked first.
    RestockFirst,
    /// Wear ± (0–1) of furniture with the tag.
    Wear {
        tag: String,
        add: f32,
    },
    /// Cash is counted: digest insights complete, tips booked tonight.
    CashUp,
    /// Items of that checklist take × `mul` of their time.
    ChecklistSpeed {
        list: ChecklistKind,
        mul: f32,
    },
    /// Helpers wait at their stations `minutes` before the subject train; the rail pre-opens.
    HelpersReady {
        minutes: u32,
    },
    /// The subject regular's order card shows a "remember" mark; `trust` extra when served.
    RememberRegular {
        trust: f32,
    },
    /// The subject table is fixed: wobble gone, beauty ±.
    FixTable {
        beauty: f32,
    },
    /// The subject menu item is in the spotlight tonight: helpers prep × `prep_mul`.
    ItemSpotlight {
        prep_mul: f32,
    },
    /// Open swap requests are more likely to be picked up by colleagues tonight.
    SwapBoard,
    /// The stock counted by hand: the bot says what is on the shelves now and warns of low
    /// stock earlier tonight (`app.bot.stock_check`).
    StockWatch,
    /// Wishes for the tastes of tonight's specials (each special's first tag) are picked
    /// × `mul` more often.
    SpecialsBoost {
        mul: f32,
        minutes: Option<u32>,
    },
}

/// Variant names of [`XEffect`] (keep in sync; a test checks every name parses).
pub const XEFFECT_NAMES: &[&str] = &[
    "Maybe",
    "Later",
    "If",
    "NextNight",
    "LearnRecipe",
    "UnlockIngredient",
    "RevealIngredient",
    "UnlockRecord",
    "UnlockTheme",
    "UnlockLatte",
    "HallMood",
    "PlayRequestedRecord",
    "RadioTrack",
    "StreetMusic",
    "SpawnMoment",
    "CameraHint",
    "LogTrain",
    "Highlight",
    "ThankYouNote",
    "TargetTrust",
    "TargetSatisfaction",
    "TargetStay",
    "TargetLeave",
    "TargetStaffMood",
    "TargetStaffEnergy",
    "TargetStaffBreak",
    "TargetRelationship",
    "ArchetypeMood",
    "GuestSatisfactionNear",
    "WishBoost",
    "Coziness",
    "Patience",
    "LightCandles",
    "PlaceBucket",
    "EmptyBucket",
    "WindowOpen",
    "Tip",
    "QuickQuality",
    "SlowFirstItems",
    "StockoutRisk",
    "StockChecked",
    "RestockFirst",
    "Wear",
    "CashUp",
    "ChecklistSpeed",
    "HelpersReady",
    "RememberRegular",
    "FixTable",
    "ItemSpotlight",
    "SwapBoard",
    "StockWatch",
    "SpecialsBoost",
];

// ---------------------------------------------------------------------------------------------
// References (validation)
// ---------------------------------------------------------------------------------------------

/// A reference from a [`Cond`] / [`Fx`] to another definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XRef<'a> {
    Core(Ref<'a>),
    Recipe(&'a str),
    Ingredient(&'a str),
    Moment(&'a str),
    Record(&'a str),
    Theme(&'a str),
    SeasonPhase(&'a str),
    Latte(&'a str),
    MusicTrack(&'a str),
    Napkin(&'a str),
}

impl Cond {
    /// Calls `f` for every id this condition refers to.
    pub fn visit_refs<'a>(&'a self, f: &mut dyn FnMut(XRef<'a>)) {
        use XCondition::*;
        match self {
            Cond::Core(c) => c.visit_refs(&mut |r| f(XRef::Core(r))),
            Cond::X(x) => match x {
                All(list) | Any(list) => list.iter().for_each(|c| c.visit_refs(f)),
                Not(c) => c.visit_refs(f),
                RecipeKnown(r) => f(XRef::Recipe(r)),
                ServedRecipe { regular, recipe } => {
                    f(XRef::Core(Ref::Regular(regular)));
                    f(XRef::Recipe(recipe));
                }
                WishMatchStreak { regular, .. } => f(XRef::Core(Ref::Regular(regular))),
                MomentResolvedEver(m) => f(XRef::Moment(m)),
                IngredientUnlocked(i) => f(XRef::Ingredient(i)),
                RecordUnlocked(r) => f(XRef::Record(r)),
                SeasonPhase(p) => f(XRef::SeasonPhase(p)),
                HiredArchetype(s) | ShiftsWorked(s, _) => f(XRef::Core(Ref::Staff(s))),
                SharedShifts { a, b, .. } => {
                    f(XRef::Core(Ref::Staff(a)));
                    f(XRef::Core(Ref::Staff(b)));
                }
                ThemeOwned(t) => f(XRef::Theme(t)),
                HallMood(_)
                | QuietHour
                | Blackout
                | StaffLowEnergy(_)
                | CozinessAtLeast(_)
                | TipsLifetimeAtLeast(_)
                | NapkinsAtLeast(_) => {}
            },
        }
    }
}

impl Fx {
    /// Calls `f` for every id this effect refers to.
    pub fn visit_refs<'a>(&'a self, f: &mut dyn FnMut(XRef<'a>)) {
        use XEffect::*;
        match self {
            Fx::Core(e) => e.visit_refs(&mut |r| f(XRef::Core(r))),
            Fx::X(x) => match x {
                Maybe { effects, .. } | Later { effects, .. } | NextNight(effects) => {
                    effects.iter().for_each(|e| e.visit_refs(f))
                }
                If {
                    conditions,
                    then,
                    otherwise,
                } => {
                    conditions.iter().for_each(|c| c.visit_refs(f));
                    then.iter().chain(otherwise).for_each(|e| e.visit_refs(f));
                }
                LearnRecipe(r) | RevealIngredient(r) => f(XRef::Recipe(r)),
                UnlockIngredient(i) => f(XRef::Ingredient(i)),
                UnlockRecord(r) => f(XRef::Record(r)),
                UnlockTheme(t) => f(XRef::Theme(t)),
                UnlockLatte(l) => f(XRef::Latte(l)),
                RadioTrack { track, .. } => f(XRef::MusicTrack(track)),
                SpawnMoment(m) => f(XRef::Moment(m)),
                ThankYouNote(Some(n)) => f(XRef::Napkin(n)),
                ArchetypeMood { staff, .. } => f(XRef::Core(Ref::Staff(staff))),
                Wear { tag, .. } => f(XRef::Core(Ref::FurnitureTag(tag))),
                HallMood { .. }
                | PlayRequestedRecord
                | StreetMusic(_)
                | CameraHint(_)
                | LogTrain
                | Highlight(_)
                | ThankYouNote(None)
                | TargetTrust(_)
                | TargetSatisfaction(_)
                | TargetStay(_)
                | TargetLeave
                | TargetStaffMood(_)
                | TargetStaffEnergy(_)
                | TargetStaffBreak
                | TargetRelationship(_)
                | GuestSatisfactionNear { .. }
                | WishBoost { .. }
                | Coziness { .. }
                | Patience { .. }
                | LightCandles
                | PlaceBucket
                | EmptyBucket
                | WindowOpen(_)
                | Tip { .. }
                | QuickQuality { .. }
                | SlowFirstItems { .. }
                | StockoutRisk { .. }
                | StockChecked
                | RestockFirst
                | CashUp
                | ChecklistSpeed { .. }
                | HelpersReady { .. }
                | RememberRegular { .. }
                | FixTable { .. }
                | ItemSpotlight { .. }
                | SwapBoard
                | StockWatch
                | SpecialsBoost { .. } => {}
            },
        }
    }
}

/// Visits every reference of a list of conditions.
pub fn visit_conds<'a>(list: &'a [Cond], f: &mut dyn FnMut(XRef<'a>)) {
    list.iter().for_each(|c| c.visit_refs(f));
}

/// Visits every reference of a list of effects.
pub fn visit_fx<'a>(list: &'a [Fx], f: &mut dyn FnMut(XRef<'a>)) {
    list.iter().for_each(|e| e.visit_refs(f));
}

/// Calls `f` on every effect of the list, recursing into `Maybe` / `Later` / `If` / `NextNight`.
pub fn walk_fx<'a>(list: &'a [Fx], f: &mut dyn FnMut(&'a Fx)) {
    for e in list {
        f(e);
        if let Fx::X(x) = e {
            match x {
                XEffect::Maybe { effects, .. }
                | XEffect::Later { effects, .. }
                | XEffect::NextNight(effects) => walk_fx(effects, f),
                XEffect::If {
                    then, otherwise, ..
                } => {
                    walk_fx(then, f);
                    walk_fx(otherwise, f);
                }
                _ => {}
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Serde: dispatch on the variant name
// ---------------------------------------------------------------------------------------------

/// Reads an enum variant identifier as a string.
struct VariantName;

impl<'de> DeserializeSeed<'de> for VariantName {
    type Value = String;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<String, D::Error> {
        struct V;
        impl Visitor<'_> for V {
            type Value = String;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a variant name")
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<String, E> {
                Ok(v.to_owned())
            }
        }
        d.deserialize_identifier(V)
    }
}

/// Replays an already-read variant name plus the format's remaining variant payload, so the
/// derived `Deserialize` of either enum can take over.
struct Replay<A> {
    name: String,
    variant: A,
}

impl<'de, A: VariantAccess<'de>> Deserializer<'de> for Replay<A> {
    type Error = A::Error;

    fn deserialize_any<W: Visitor<'de>>(self, visitor: W) -> Result<W::Value, Self::Error> {
        visitor.visit_enum(self)
    }

    fn deserialize_enum<W: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: W,
    ) -> Result<W::Value, Self::Error> {
        visitor.visit_enum(self)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf
        option unit unit_struct newtype_struct seq tuple tuple_struct map struct identifier
        ignored_any
    }
}

impl<'de, A: VariantAccess<'de>> EnumAccess<'de> for Replay<A> {
    type Error = A::Error;
    type Variant = A;

    fn variant_seed<S: DeserializeSeed<'de>>(self, seed: S) -> Result<(S::Value, A), A::Error> {
        let name: de::value::StringDeserializer<A::Error> = self.name.into_deserializer();
        Ok((seed.deserialize(name)?, self.variant))
    }
}

macro_rules! union_serde {
    ($union:ident, $core:ty, $ext:ty, $names:expr, $label:literal) => {
        impl<'de> Deserialize<'de> for $union {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                struct V;
                impl<'de> Visitor<'de> for V {
                    type Value = $union;
                    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                        f.write_str($label)
                    }
                    fn visit_enum<A: EnumAccess<'de>>(self, data: A) -> Result<$union, A::Error> {
                        let (name, variant) = data.variant_seed(VariantName)?;
                        let replay = Replay { name, variant };
                        if $names.contains(&replay.name.as_str()) {
                            <$ext>::deserialize(replay).map($union::X)
                        } else {
                            <$core>::deserialize(replay).map($union::Core)
                        }
                    }
                }
                d.deserialize_enum(stringify!($union), $names, V)
            }
        }

        impl Serialize for $union {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                match self {
                    $union::Core(c) => c.serialize(s),
                    $union::X(x) => x.serialize(s),
                }
            }
        }
    };
}

union_serde!(Cond, Condition, XCondition, XCONDITION_NAMES, "a condition");
union_serde!(Fx, Effect, XEffect, XEFFECT_NAMES, "an effect");

impl From<Condition> for Cond {
    fn from(c: Condition) -> Cond {
        Cond::Core(c)
    }
}

impl From<XCondition> for Cond {
    fn from(c: XCondition) -> Cond {
        Cond::X(c)
    }
}

impl From<Effect> for Fx {
    fn from(e: Effect) -> Fx {
        Fx::Core(e)
    }
}

impl From<XEffect> for Fx {
    fn from(e: XEffect) -> Fx {
        Fx::X(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::defs::Weather;

    fn conds(text: &str) -> Vec<Cond> {
        ron::from_str(text).unwrap_or_else(|e| panic!("{text}: {e}"))
    }

    #[test]
    fn mixes_core_and_extension_conditions() {
        let c = conds(
            r#"[Raining, SeasonPhase("frost"), Any([Weather(Snow), QuietHour]),
                ServedFastStreak(regular: "mirek", minutes: 10, visits: 3)]"#,
        );
        assert_eq!(c[0], Cond::Core(Condition::Raining));
        assert_eq!(c[1], Cond::X(XCondition::SeasonPhase("frost".into())));
        assert_eq!(
            c[2],
            Cond::X(XCondition::Any(vec![
                Cond::Core(Condition::Weather(Weather::Snow)),
                Cond::X(XCondition::QuietHour)
            ]))
        );
        assert!(matches!(
            c[3],
            Cond::Core(Condition::ServedFastStreak { .. })
        ));
    }

    #[test]
    fn mixes_core_and_extension_effects() {
        let text = r#"#![enable(implicit_some)]
            [Money(-15), AddCounter("cat_affection", 2),
             If(conditions: [CounterAtLeast("cat_affection", 2)], then: [TargetTrust(1.0)]),
             Later(minutes: 60, effects: [SpawnMoment("m_leak_empty")]),
             HallMood(mood: Calm, minutes: 30)]"#;
        let fx: Vec<Fx> = ron::from_str(text).unwrap();
        assert_eq!(fx[0], Fx::Core(Effect::Money(-15)));
        assert!(matches!(&fx[2], Fx::X(XEffect::If { otherwise, .. }) if otherwise.is_empty()));
        assert!(matches!(
            &fx[4],
            Fx::X(XEffect::HallMood {
                minutes: Some(30),
                ..
            })
        ));
    }

    #[test]
    fn unknown_variants_are_errors() {
        assert!(ron::from_str::<Vec<Cond>>("[NoSuchThing]").is_err());
        assert!(ron::from_str::<Vec<Fx>>("[Money(\"x\")]").is_err());
    }

    #[test]
    fn round_trips_through_ron() {
        let text = r#"[Raining, Not(RecipeKnown("r_chai")), All([QuietHour, NightAtLeast(3)])]"#;
        let c = conds(text);
        let back: Vec<Cond> = ron::from_str(&ron::to_string(&c).unwrap()).unwrap();
        assert_eq!(c, back);
        let fx: Vec<Fx> =
            ron::from_str(r#"[NextNight([StaffMood(3.0), Coziness(add: 2.0, minutes: None)])]"#)
                .unwrap();
        let back: Vec<Fx> = ron::from_str(&ron::to_string(&fx).unwrap()).unwrap();
        assert_eq!(fx, back);
    }

    #[test]
    fn name_lists_cover_every_variant() {
        // Every listed name must be a real variant (a wrong name would fall through to the core
        // enum and fail there), and the lists must not shadow core names.
        let core_c = ["Raining", "Flag", "Weather", "NightAtLeast", "TrainNear"];
        let core_e = ["Money", "Toast", "StaffMood", "SetFlag", "Bot"];
        assert!(core_c.iter().all(|n| !XCONDITION_NAMES.contains(n)));
        assert!(core_e.iter().all(|n| !XEFFECT_NAMES.contains(n)));
        for name in XCONDITION_NAMES {
            let err = ron::from_str::<Cond>(name).err().map(|e| e.to_string());
            // A bare name is only valid for unit variants; others must fail with a *payload*
            // error, never with "unknown variant".
            if let Some(e) = err {
                assert!(!e.contains("variant"), "{name}: {e}");
            }
        }
        for name in XEFFECT_NAMES {
            if let Some(e) = ron::from_str::<Fx>(name).err().map(|e| e.to_string()) {
                assert!(!e.contains("variant"), "{name}: {e}");
            }
        }
    }
}
