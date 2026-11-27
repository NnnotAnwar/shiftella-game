//! Words for what purchases do (`sim::purchases`): the atmosphere meter's "now / next tier"
//! lines, the build card's expected effect in numbers and the guest's seat in the hover line.

use super::world_view::sim;
use crate::content::{Content, FurnitureDef};
use crate::settings::Locale;
use sim::World;
use sim::purchases::{ApplianceRole, SeatFeel, TierDef};

fn pct(x: f32) -> String {
    format!("{}", (x * 100.0).round() as i32)
}

/// A tier's perks in a short comma list ("tips +8 %, stays +10 %…"); `None` = nothing extra.
pub fn perks_text(loc: &Locale, t: &TierDef) -> Option<String> {
    let mut v = Vec::new();
    if t.tips > 0.0 {
        v.push(loc.trf("purchases.perk.tips", &[("n", &pct(t.tips))]));
    }
    if t.stay > 0.0 {
        v.push(loc.trf("purchases.perk.stay", &[("n", &pct(t.stay))]));
    }
    if t.review > 0.0 {
        v.push(loc.trf("purchases.perk.review", &[("n", &pct(t.review))]));
    }
    if t.round > 0.0 {
        v.push(loc.trf("purchases.perk.round", &[("n", &pct(t.round))]));
    }
    if !t.rare.is_empty() {
        v.push(loc.tr_owned("purchases.perk.rare"));
    }
    if t.wishes {
        v.push(loc.tr_owned("purchases.perk.wishes"));
    }
    (!v.is_empty()).then(|| v.join(", "))
}

/// The next tier above `level` with its threshold: (level, threshold).
fn next_tier(content: &Content, level: u32) -> Option<(u32, f32)> {
    let levels = &content.handson.decor.coziness.levels;
    // level n (1-based) starts at levels[n - 2]; the next one at levels[n - 1]
    levels
        .get(level as usize - 1)
        .map(|at| (level + 1, *at))
        .filter(|_| level >= 1)
}

/// The meter popover's lines: what the tier gives now, then what the next one brings.
pub fn tier_lines(world: &World, content: &Content, loc: &Locale) -> Vec<String> {
    let level = world.coziness().level.clamp(1, 5);
    let now = perks_text(loc, &sim::purchases::tier_of(level))
        .unwrap_or_else(|| loc.tr_owned("purchases.tier.none"));
    let mut out = vec![loc.trf("purchases.tier.now", &[("perks", &now)])];
    match next_tier(content, level) {
        Some((next, at)) => {
            let perks = perks_text(loc, &sim::purchases::tier_of(next))
                .unwrap_or_else(|| loc.tr_owned("purchases.tier.none"));
            out.push(loc.trf(
                "purchases.tier.next",
                &[
                    ("level", &loc.tr_owned(&format!("atmo.level.{next}"))),
                    ("at", &format!("{}", at.round() as i32)),
                    ("perks", &perks),
                ],
            ));
        }
        None => out.push(loc.tr_owned("purchases.tier.top")),
    }
    out
}

/// The build card's expected effect, in numbers (empty = nothing to say).
pub fn card_lines(world: &World, content: &Content, loc: &Locale, d: &FurnitureDef) -> Vec<String> {
    let e = world.piece_effect(content, d);
    let mut out = Vec::new();
    if let Some((now, with, more)) = e.seats {
        out.push(loc.trf(
            "purchases.card.seats",
            &[
                ("now", &now.to_string()),
                ("with", &with.to_string()),
                ("n", &more.max(0).to_string()),
            ],
        ));
    }
    if let Some(p) = e.comfort {
        if p.stay >= 1.0 {
            out.push(loc.trf(
                "purchases.card.comfy",
                &[
                    ("stay", &pct(p.stay - 1.0)),
                    ("tips", &pct(p.tips - 1.0)),
                    ("round", &pct(p.round)),
                ],
            ));
        } else {
            out.push(loc.tr_owned("purchases.card.hard"));
        }
    }
    if let Some((role, acts)) = e.role {
        let line = match role {
            ApplianceRole::Parallel if acts => loc.tr_owned("purchases.card.parallel"),
            ApplianceRole::Parallel => loc.tr_owned("purchases.card.parallel_first"),
            ApplianceRole::Unlocks if !e.unlocks.is_empty() => {
                let items: Vec<String> = e
                    .unlocks
                    .iter()
                    .map(|m| loc.tr_owned(&format!("menu.{m}.name")))
                    .collect();
                loc.trf("purchases.card.unlocks", &[("items", &items.join(", "))])
            }
            ApplianceRole::Unlocks => String::new(),
        };
        if !line.is_empty() {
            out.push(line);
        }
    }
    // a fridge: the room for fresh stock now → with it
    if let Some((now, with)) = e.fridge {
        out.push(loc.trf(
            "purchases.card.fridge",
            &[
                ("now", &crate::i18n::format_money(loc.lang, now)),
                ("with", &crate::i18n::format_money(loc.lang, with)),
            ],
        ));
    }
    // light and decor: where the meter stands and what its next tier brings
    let decor = d.beauty > 0.0 || (d.light_radius > 0.0 && d.has_tag("light"));
    if decor && d.seats == 0 {
        let c = world.coziness();
        let level = c.level.clamp(1, 5);
        let next = match next_tier(content, level) {
            Some((n, at)) => loc.trf(
                "purchases.card.cozy_next",
                &[
                    ("level", &loc.tr_owned(&format!("atmo.level.{n}"))),
                    ("at", &format!("{}", at.round() as i32)),
                    (
                        "perks",
                        &perks_text(loc, &sim::purchases::tier_of(n))
                            .unwrap_or_else(|| loc.tr_owned("purchases.tier.none")),
                    ),
                ],
            ),
            None => loc.tr_owned("purchases.tier.top"),
        };
        out.push(loc.trf(
            "purchases.card.cozy",
            &[
                ("value", &format!("{}", c.value.round() as i32)),
                ("level", &loc.tr_owned(&format!("atmo.level.{level}"))),
                ("next", &next),
            ],
        ));
    }
    out
}

/// "comfy seat — staying a while" / "hard stool — won't stay long" for a seated guest.
pub fn seat_line(world: &World, content: &Content, loc: &Locale, guest: u32) -> Option<String> {
    world.seat_feel(content, guest).map(|(f, _)| match f {
        SeatFeel::Comfy => loc.tr_owned("purchases.guest.comfy"),
        SeatFeel::Hard => loc.tr_owned("purchases.guest.hard"),
    })
}
