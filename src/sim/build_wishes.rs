//! Requests from guests and staff (design/v0.2.md item 4): data-driven build wishes
//! (`build_wishes.ron`) — "Hana: a quiet corner", "Vera: a seat by the window", "a coat rack,
//! please". A request shows once its person is known and `appear` holds; the person says it
//! when they are around (a bubble) and it sits as a card in the Shiftella app (Requests).
//! Once `done` holds and the person is in the café, they react (bubble + toast) and the
//! reward runs once. The story arcs keep their own conditions — the requests only tell the
//! player about them, and nothing here is needed to finish the story.

use super::build_content::WishProgress;
use super::systems::XTarget;
use super::types::*;
use super::world::*;
use crate::content::Content;
use crate::content::build::{BuildWishDef, WishWho};

/// The "wanted" badge on a build card: an open request nobody has answered yet.
/// `who` = a regular id (`filip`), a helper's staff id as a number (`"7"`), or a guest
/// archetype id (`students`) — see [`World::build_requests`] for the resolved person.
#[derive(Debug, Clone, PartialEq)]
pub struct BuildWish {
    pub who: String,
    pub def: Option<String>,
    pub tag: Option<String>,
    pub reason_key: String,
}

/// Who a request belongs to, resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildWho {
    /// `regular.<id>.name`.
    Regular(String),
    /// A helper (`World::staff_by_id`).
    Staff(u32),
    /// `guest.<id>.name` (a group, e.g. the students).
    Guests(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildRequestStatus {
    /// Not built yet.
    Waiting,
    /// It stands; they will see it on their next visit / shift.
    Built,
    /// They saw it and said thank you (the night).
    Thanked(u32),
}

/// A request card for the Shiftella app.
#[derive(Debug, Clone, PartialEq)]
pub struct BuildRequest {
    pub id: String,
    pub who: BuildWho,
    pub def: Option<String>,
    pub tag: Option<String>,
    /// `build_wish.<id>.reason` / `.ask` / `.thanks`.
    pub reason_key: String,
    pub ask_key: String,
    pub thanks_key: String,
    pub status: BuildRequestStatus,
    pub opened: u32,
    /// Mirrors a story arc (the card says it matters to them).
    pub arc: bool,
}

/// The helper a `Staff` request belongs to: the first employed one of that archetype (or the
/// first helper hired).
fn staff_for(w: &World, archetype: Option<&str>) -> Option<u32> {
    w.staff
        .iter()
        .filter(|s| !s.owner && archetype.is_none_or(|a| s.archetype == a))
        .map(|s| s.id)
        .min()
}

fn regular_known(w: &World, r: &str) -> bool {
    w.story
        .regulars
        .get(r)
        .is_some_and(|st| !st.visits.is_empty() || st.party.is_some())
}

/// The person is known (the request may show).
fn known(w: &World, def: &BuildWishDef, p: Option<&WishProgress>) -> Option<BuildWho> {
    match &def.who {
        WishWho::Regular(r) => regular_known(w, r).then(|| BuildWho::Regular(r.clone())),
        WishWho::Staff(a) => p
            .and_then(|p| p.staff)
            .filter(|id| w.staff.iter().any(|s| s.id == *id))
            .or_else(|| staff_for(w, a.as_deref()))
            .map(BuildWho::Staff),
        WishWho::Guests(g) => Some(BuildWho::Guests(g.clone())),
    }
}

/// Seated-ish parties only (arriving / leaving people don't stop to talk).
fn settled(p: &Party) -> bool {
    matches!(
        p.state,
        PartyState::Queue | PartyState::WaitOrder | PartyState::WaitFood | PartyState::Eating
    )
}

/// The person in the café right now: who speaks, and the effect target.
fn present(w: &World, who: &BuildWho) -> Option<(AgentRef, XTarget)> {
    match who {
        BuildWho::Regular(r) => {
            let pid = w.story.regulars.get(r)?.party?;
            let p = w.parties.iter().find(|p| p.id == pid && settled(p))?;
            let leader = *p.members.first()?;
            Some((
                AgentRef::Guest(leader),
                XTarget {
                    party: Some(p.id),
                    regular: Some(r.clone()),
                    ..Default::default()
                },
            ))
        }
        BuildWho::Staff(id) => {
            let s = w.staff.iter().find(|s| s.id == *id && s.on_duty())?;
            Some((
                AgentRef::Staff(s.id),
                XTarget {
                    staff: Some(s.id),
                    ..Default::default()
                },
            ))
        }
        BuildWho::Guests(a) => {
            let p = w
                .parties
                .iter()
                .find(|p| &p.archetype == a && p.regular.is_none() && settled(p))?;
            let leader = *p.members.first()?;
            Some((
                AgentRef::Guest(leader),
                XTarget {
                    party: Some(p.id),
                    ..Default::default()
                },
            ))
        }
    }
}

fn speaker(who: &BuildWho) -> String {
    match who {
        BuildWho::Regular(r) => r.clone(),
        BuildWho::Staff(id) => format!("staff:{id}"),
        BuildWho::Guests(g) => g.clone(),
    }
}

fn who_id(who: &BuildWho) -> String {
    match who {
        BuildWho::Regular(r) | BuildWho::Guests(r) => r.clone(),
        BuildWho::Staff(id) => id.to_string(),
    }
}

/// Opens requests whose person is known and whose `appear` holds (day start, night start,
/// after a furniture change, each minute).
pub(crate) fn refresh(w: &mut World, content: &Content) {
    for def in &content.build.wishes.wishes {
        if w.build.wishes.contains_key(&def.id) {
            continue;
        }
        let Some(who) = known(w, def, None) else {
            continue;
        };
        if !super::xlogic::holds(w, content, &def.appear) {
            continue;
        }
        // already standing when it would show: nothing to ask for
        if super::xlogic::holds(w, content, &def.done) {
            continue;
        }
        let staff = match who {
            BuildWho::Staff(id) => Some(id),
            _ => None,
        };
        w.build.wishes.insert(
            def.id.clone(),
            WishProgress {
                opened: w.night,
                staff,
                ..Default::default()
            },
        );
        w.emit(SimEvent::BuildWishOpened {
            id: def.id.clone(),
            who: who_id(&who),
        });
    }
}

/// Each night minute: one person says their request (once), or reacts to it standing.
pub(crate) fn on_minute(w: &mut World, content: &Content) {
    if w.minute().is_multiple_of(5) {
        refresh(w, content);
    }
    for def in &content.build.wishes.wishes {
        let Some(p) = w.build.wishes.get(&def.id).cloned() else {
            continue;
        };
        if p.thanked.is_some() {
            continue;
        }
        let Some(who) = known(w, def, Some(&p)) else {
            continue;
        };
        let Some((agent, target)) = present(w, &who) else {
            continue;
        };
        let done = super::xlogic::holds(w, content, &def.done);
        if done {
            let night = w.night;
            if let Some(st) = w.build.wishes.get_mut(&def.id) {
                st.thanked = Some(night);
            }
            w.emit(SimEvent::Speech {
                speaker: speaker(&who),
                agent: Some(agent),
                key: def.thanks_key(),
            });
            w.emit(SimEvent::BuildWishFulfilled {
                id: def.id.clone(),
                who: who_id(&who),
            });
            let reward = def.reward.clone();
            super::xlogic::run(w, content, &reward, &target);
            return;
        }
        if !p.asked {
            if let Some(st) = w.build.wishes.get_mut(&def.id) {
                st.asked = true;
            }
            w.emit(SimEvent::Speech {
                speaker: speaker(&who),
                agent: Some(agent),
                key: def.ask_key(),
            });
            return;
        }
    }
}

impl World {
    /// Open requests nothing answers yet — the build cards' "wanted" badge
    /// (`who` + `def` / `tag` + `build_wish.<id>.reason`).
    pub fn build_wishes(&self, content: &Content) -> Vec<BuildWish> {
        self.build_requests(content)
            .into_iter()
            .filter(|r| r.status == BuildRequestStatus::Waiting)
            .map(|r| BuildWish {
                who: who_id(&r.who),
                def: r.def,
                tag: r.tag,
                reason_key: r.reason_key,
            })
            .collect()
    }

    /// Every request shown so far, for the app's Requests cards: waiting first, then built
    /// (they'll see it), then thanked (newest first).
    pub fn build_requests(&self, content: &Content) -> Vec<BuildRequest> {
        let mut out: Vec<BuildRequest> = Vec::new();
        for def in &content.build.wishes.wishes {
            let Some(p) = self.build.wishes.get(&def.id) else {
                continue;
            };
            let Some(who) = known(self, def, Some(p)) else {
                continue;
            };
            let status = match p.thanked {
                Some(n) => BuildRequestStatus::Thanked(n),
                None if super::xlogic::holds(self, content, &def.done) => BuildRequestStatus::Built,
                None => BuildRequestStatus::Waiting,
            };
            out.push(BuildRequest {
                id: def.id.clone(),
                who,
                def: def.def.clone(),
                tag: def.tag.clone(),
                reason_key: def.reason_key(),
                ask_key: def.ask_key(),
                thanks_key: def.thanks_key(),
                status,
                opened: p.opened,
                arc: def.arc,
            });
        }
        let rank = |s: &BuildRequestStatus| match s {
            BuildRequestStatus::Waiting => (0, 0),
            BuildRequestStatus::Built => (1, 0),
            BuildRequestStatus::Thanked(n) => (2, u32::MAX - n),
        };
        out.sort_by_key(|r| (rank(&r.status), u32::MAX - r.opened));
        out
    }

    /// Requests waiting for a piece (the app's tab badge).
    pub fn build_requests_waiting(&self, content: &Content) -> u32 {
        self.build_wishes(content).len() as u32
    }
}
