//! The duties board (H, P as an alias — design/handson.md §1.3): rows are the helpers,
//! columns the six duties; capacity dots; "Suggest" fills free slots by skill. Duties
//! nobody holds stay with Karolina. Replaces the old 5×4 priority grid.

use macroquad::prelude::*;

use super::world_view::{self as wv, sim};
use super::{Ctx, GameScreen, Notice};
use crate::content::Content;
use crate::content::handson::Duty as CDuty;
use crate::ui::handson::{
    DutiesAction, DutiesLabels, DutiesView, Duty, HelperRow, duties_board, duties_board_size,
};
use crate::ui::{Icon, ToastKind, Ui};
use sim::Command;

pub fn to_ui(d: CDuty) -> Duty {
    match d {
        CDuty::Dishes => Duty::Dishes,
        CDuty::Floor => Duty::Floor,
        CDuty::Register => Duty::Register,
        CDuty::Bar => Duty::Bar,
        CDuty::Kitchen => Duty::Kitchen,
        CDuty::Serving => Duty::Serving,
    }
}

pub fn to_sim(d: Duty) -> CDuty {
    match d {
        Duty::Dishes => CDuty::Dishes,
        Duty::Floor => CDuty::Floor,
        Duty::Register => CDuty::Register,
        Duty::Bar => CDuty::Bar,
        Duty::Kitchen => CDuty::Kitchen,
        Duty::Serving => CDuty::Serving,
    }
}

/// Gives (`on`) or takes back a helper's duty — the one path for the board and the character
/// card (`crew_ui`), so both always show the same. A full helper says so in a toast.
pub fn set_duty(g: &mut GameScreen, content: &Content, staff: u32, duty: CDuty, on: bool) -> bool {
    match g.world.apply(content, Command::SetDuty { staff, duty, on }) {
        Ok(()) => {
            g.hands.tut.bump("duty_set");
            true
        }
        Err(sim::CommandError::Capacity) => {
            g.notices.push(Notice::Key {
                kind: ToastKind::Info,
                icon: Icon::Crew,
                key: "ui.duties.full_short".into(),
                args: Vec::new(),
            });
            false
        }
        Err(_) => false,
    }
}

/// Helpers in board order (on shift first, then by name).
pub fn helpers(g: &GameScreen) -> Vec<sim::Staff> {
    let mut v: Vec<sim::Staff> = g
        .world
        .staff()
        .iter()
        .filter(|s| !s.owner)
        .cloned()
        .collect();
    v.sort_by(|a, b| b.on_duty().cmp(&a.on_duty()).then(a.name.cmp(&b.name)));
    v
}

/// The board's rows for these helpers (read straight from the world).
pub fn helper_rows(
    g: &GameScreen,
    content: &Content,
    loc: &crate::settings::Locale,
    crew: &[sim::Staff],
) -> Vec<HelperRow> {
    let night = g.world.night();
    crew.iter()
        .map(|s| {
            let cap = g.world.capacity(content, s.id).max(1);
            let likes = content
                .staff_archetype(&s.archetype)
                .and_then(|a| a.default_duty)
                .map(to_ui);
            let skills = CDuty::ALL.map(|d| s.skill(d.job(), night).round().clamp(0.0, 10.0) as u8);
            let next_slot = s.duties.len() as u32 >= cap && cap < 3;
            HelperRow {
                name: wv::staff_name(loc, s),
                role: wv::staff_role(loc, content, s),
                capacity: cap as u8,
                duties: s.duties.iter().map(|d| to_ui(*d)).collect(),
                on_shift: s.on_duty() || g.world.phase() == sim::Phase::Day,
                likes,
                skills,
                note: next_slot.then(|| loc.tr_owned("ui.duties.grows")),
            }
        })
        .collect()
}

pub fn board(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let content = cx.content;
    let loc = cx.loc;
    let crew = helpers(g);
    let rows = helper_rows(g, content, &loc, &crew);
    let view = DutiesView {
        helpers: rows,
        karolina: loc.tr_owned("ui.staff.you_row"),
    };
    let labels = DutiesLabels {
        title: loc.tr_owned("ui.duties.title"),
        subtitle: loc.tr_owned("ui.duties.player_only"),
        duty: CDuty::ALL.map(|d| loc.tr_owned(&d.name_key())),
        duty_desc: CDuty::ALL.map(|d| loc.tr_owned(&d.desc_key())),
        suggest: loc.tr_owned("ui.duties.suggest"),
        karolina_takes: loc.tr_owned("ui.duties.left_for_you"),
        all_covered: loc.tr_owned("ui.duties.all_covered"),
        off_shift: loc.tr_owned("ui.duties.off_shift"),
        full: loc.tr_owned("ui.duties.full_short"),
        capacity: loc.trf("ui.duties.capacity", &[("n", "{used}"), ("max", "{cap}")]),
        skill: loc.tr_owned("ui.duties.skill"),
        likes: loc.tr_owned("ui.duties.likes"),
    };
    let s = ui.screen();
    let size = duties_board_size(view.helpers.len());
    let h = size.y.min(s.h - 92.0 - 64.0);
    let w = size.x.min(s.w - 24.0);
    let r = Rect::new(((s.w - w) / 2.0).round(), 92.0, w, h);
    g.anchors.push(("panel", r));
    g.anchors.push(("duties.board", r));
    let action = duties_board(ui, "duties", r, &view, &labels, &mut g.hands.duties);
    if view.helpers.is_empty() {
        // nobody to delegate to yet: say when that changes
        let t = ui.theme.clone();
        let rows = Rect::new(r.x + 14.0, r.y + 45.0 + 10.0 + 30.0, r.w - 28.0, 50.0);
        ui.fonts.draw_in(
            loc.tr("ui.duties.none"),
            rows,
            &t.small().color(t.text_dim),
            crate::ui::Align::Center,
        );
    }
    let Some(action) = action else {
        return;
    };
    let set = |g: &mut GameScreen, staff: u32, duty: Duty, on: bool| -> bool {
        set_duty(g, content, staff, to_sim(duty), on)
    };
    match action {
        DutiesAction::Toggle { helper, duty } => {
            if let Some(s) = crew.get(helper) {
                let on = !s.duties.contains(&to_sim(duty));
                set(g, s.id, duty, on);
            }
        }
        DutiesAction::Move { duty, from, to } => {
            if let (Some(a), Some(b)) = (crew.get(from), crew.get(to))
                && set(g, b.id, duty, true)
            {
                set(g, a.id, duty, false);
            }
        }
        DutiesAction::Suggest => {
            for s in &crew {
                let cap = g.world.capacity(content, s.id);
                let mut held = g.world.staff_duties(s.id).len() as u32;
                while held < cap {
                    let Some(d) = g.world.suggest_duty(s.id) else {
                        break;
                    };
                    if !set(g, s.id, to_ui(d), true) {
                        break;
                    }
                    held += 1;
                }
            }
        }
        DutiesAction::Close => g.panel = None,
    }
}
