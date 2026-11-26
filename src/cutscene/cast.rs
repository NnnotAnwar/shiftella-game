//! Who is who in a cutscene: display names, looks, portrait expressions and chip colours.

use std::collections::HashMap;

use macroquad::prelude::*;

use crate::content::{self, Content, Look};
use crate::render::{Expression, HairStyle, PersonLook, TopStyle};
use crate::settings::Locale;

/// How a speaker id is named on screen.
#[derive(Debug, Clone, PartialEq)]
pub enum SpeakerName {
    /// Translated key (`cast.<id>.name`, `regular.<id>.name`, `guest.<id>.name`).
    Key(String),
    /// Proper noun, never translated (a staff archetype's `story_name`).
    Plain(String),
}

/// Speaker / actor lookup built from content once.
#[derive(Debug, Clone, Default)]
pub struct Cast {
    names: HashMap<String, SpeakerName>,
    looks: HashMap<String, Look>,
}

impl Cast {
    pub fn from_content(c: &Content) -> Cast {
        let mut cast = Cast::default();
        for g in &c.guests {
            cast.names
                .insert(g.id.clone(), SpeakerName::Key(g.name_key()));
        }
        for s in &c.staff {
            cast.names
                .insert(s.id.clone(), SpeakerName::Plain(s.story_name.clone()));
        }
        for r in &c.regulars {
            cast.names
                .insert(r.id.clone(), SpeakerName::Key(r.name_key()));
        }
        for m in &c.cutscenes.cast {
            cast.names.insert(
                m.id.clone(),
                SpeakerName::Key(format!("cast.{}.name", m.id)),
            );
            cast.looks.insert(m.id.clone(), m.look.clone());
        }
        cast
    }

    /// Display name for a speaker id.
    pub fn name(&self, loc: &Locale, id: &str) -> String {
        match self.names.get(id) {
            Some(SpeakerName::Key(k)) => loc.tr_owned(k),
            Some(SpeakerName::Plain(s)) => s.clone(),
            None => {
                let k = format!("cast.{id}.name");
                if loc.has(&k) {
                    loc.tr_owned(&k)
                } else {
                    id.to_string()
                }
            }
        }
    }

    /// The speaker has a face (the station announcer is only a voice).
    pub fn has_portrait(&self, id: &str) -> bool {
        id != "announcer"
    }

    /// Look for portraits and the world actor.
    pub fn look(&self, id: &str) -> PersonLook {
        match id {
            "grandma" => PersonLook::preset("grandma_lida"),
            "bot" => PersonLook::preset("shiftella_bot"),
            "player" => {
                // in cutscenes the heir wears the cast look (raincoat), not the apron
                let mut l = PersonLook::preset("player");
                if let Some(look) = self.looks.get(id) {
                    apply_look(&mut l, look);
                    l.apron = false;
                }
                // Karolina keeps her ponytail even if the cast data still says "short"
                if matches!(l.hair_style, HairStyle::Short | HairStyle::Buzz) {
                    l.hair_style = HairStyle::Ponytail;
                }
                l
            }
            _ => {
                let bare = id.strip_prefix("guest_").unwrap_or(id);
                if PersonLook::PRESETS
                    .iter()
                    .any(|p| p.strip_prefix("guest_").unwrap_or(p) == bare)
                {
                    return PersonLook::preset(id);
                }
                let mut l = PersonLook::preset(id);
                if let Some(look) = self.looks.get(id) {
                    apply_look(&mut l, look);
                }
                l
            }
        }
    }

    /// Accent of the speaker chip.
    pub fn color(&self, id: &str) -> Color {
        match id {
            "grandma" => Color::from_hex(0x7C3AED),
            "bot" | "player" => Color::from_hex(0x6366F1),
            "announcer" => Color::from_hex(0x3F8A8C),
            other => crate::ui::avatar_color(other),
        }
    }
}

/// Content expression → render expression (same names).
pub fn expression(e: content::Expression) -> Expression {
    match e {
        content::Expression::Neutral => Expression::Neutral,
        content::Expression::Smile => Expression::Smile,
        content::Expression::Sad => Expression::Sad,
        content::Expression::Surprised => Expression::Surprised,
        content::Expression::Thinking => Expression::Thinking,
        content::Expression::Laugh => Expression::Laugh,
        content::Expression::Tired => Expression::Tired,
    }
}

fn parse_hex(s: &str) -> Option<Color> {
    let h = s.trim().trim_start_matches('#');
    u32::from_str_radix(h, 16).ok().map(Color::from_hex)
}

/// Applies the content's look hints (hair, outfit colour/style, height) on top of a seeded look.
fn apply_look(l: &mut PersonLook, look: &Look) {
    if let Some(c) = look.hair_colors.first().and_then(|c| parse_hex(c)) {
        l.hair = c;
    }
    if let Some(c) = look.outfit_colors.first().and_then(|c| parse_hex(c)) {
        l.top = c;
    }
    if let Some(c) = look.skin.first().and_then(|c| parse_hex(c)) {
        l.skin = c;
    }
    if let Some(h) = look.hair.first() {
        l.hair_style = match h.as_str() {
            "buzz" => HairStyle::Buzz,
            "bob" => HairStyle::Bob,
            "long" | "braid" => HairStyle::Long,
            "bun" => HairStyle::Bun,
            "ponytail" => HairStyle::Ponytail,
            "curly" => HairStyle::Curly,
            "bald" => HairStyle::Bald,
            _ => HairStyle::Short,
        };
    }
    l.top_style = match look.outfit.as_str() {
        "coat" | "raincoat" | "old_coat" => TopStyle::Coat,
        "hoodie" => TopStyle::Hoodie,
        "cardigan" => TopStyle::Cardigan,
        "suit" | "jacket" | "uniform" | "hi_vis" => TopStyle::Jacket,
        "scrubs" => TopStyle::Scrubs,
        _ => l.top_style,
    };
    if look.height > 0.0 {
        l.height = (look.height * 1.0).clamp(0.85, 1.1);
    }
}
