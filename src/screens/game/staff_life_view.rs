//! Helpers at work in the world view: the small progress ring over each working helper's
//! head (a task or an idle pastime, `World::helper_work`) and the pastime props in hand.
//! Karolina's queue chips (`hands_view`) stay the bigger, brighter ring.

use macroquad::prelude::*;

use super::GameScreen;
use super::hands::cup_of;
use super::world_view::{self as wv, sim};
use crate::content::Content;
use crate::render::{self, HelperIcon, PastimeIcon, QueueIcon};
use sim::{AgentRef, HelperDoing, Pastime, Phase};

/// The ring / prop picture of a pastime.
pub fn pastime_icon(p: Pastime) -> PastimeIcon {
    match p {
        Pastime::WipeCounter | Pastime::WipeTable => PastimeIcon::Cloth,
        Pastime::PolishCups => PastimeIcon::Cup,
        Pastime::StackSaucers => PastimeIcon::Saucers,
        Pastime::CheckPastry => PastimeIcon::Pastry,
        Pastime::RefillNapkins => PastimeIcon::Napkin,
        Pastime::StraightenChairs => PastimeIcon::Chair,
        Pastime::RefillSugar => PastimeIcon::Sugar,
        Pastime::Sweep => PastimeIcon::Broom,
        Pastime::SipWater => PastimeIcon::Water,
        Pastime::Chat => PastimeIcon::Chat,
        Pastime::Phone => PastimeIcon::Phone,
        Pastime::Window => PastimeIcon::Window,
        Pastime::PetCat => PastimeIcon::Paw,
    }
}

/// The ring icon of what a helper does (the same pictures as Karolina's chips for tasks).
pub fn helper_icon(content: &Content, d: &HelperDoing) -> HelperIcon {
    match d {
        HelperDoing::Pastime(p) => HelperIcon::Pastime(pastime_icon(*p)),
        HelperDoing::Task { id, menu } => {
            let cup = || {
                menu.as_deref()
                    .map(|m| cup_of(content, m))
                    .unwrap_or_default()
            };
            HelperIcon::Task(match *id {
                "take_order" => QueueIcon::TakeOrder,
                "prepare" => QueueIcon::Brew(cup()),
                "serve" => QueueIcon::Serve(cup()),
                "clear_table" => QueueIcon::Clear,
                "wash" => QueueIcon::Wash,
                "mop" | "bucket" => QueueIcon::Mop,
                "checklist" => QueueIcon::Light,
                _ => QueueIcon::Walk,
            })
        }
    }
}

/// The props of the helpers' pastimes (cup and cloth, broom, glass, phone…), drawn over the
/// people (call right after the scene, before the lighting-independent overlays).
pub fn push_props(
    world: &sim::World,
    people: &[sim::AgentView],
    q: &mut render::RenderQueue,
    t: f32,
) {
    if world.phase() != Phase::Night {
        return;
    }
    for v in people {
        let AgentRef::Staff(id) = v.agent else {
            continue;
        };
        let Some(l) = world.pastime(id) else {
            continue;
        };
        if v.pose == sim::Pose::Walk || v.pose == sim::Pose::Carry {
            continue;
        }
        let pose = wv::person_pose(v, t);
        let icon = pastime_icon(l.kind);
        let (feet, facing) = (pose.feet, pose.facing);
        let k = wv::look_of(&v.look).height;
        q.push_custom(feet.y + 0.05, move || {
            render::draw_pastime_prop(feet, facing, k, icon, t)
        });
    }
}

/// The helpers' work rings (after the lighting, like the emotes).
pub fn draw_world(g: &mut GameScreen, content: &Content) {
    if g.world.phase() != Phase::Night {
        return;
    }
    let t = g.time;
    for hw in g.world.helper_work() {
        let Some(v) = g
            .scene
            .people
            .iter()
            .find(|v| v.agent == AgentRef::Staff(hw.staff))
            .cloned()
        else {
            continue;
        };
        // a speech bubble has the spot over the head
        if v.emote.is_some() {
            continue;
        }
        let look = g.scene.look(&v.look);
        let anchor = render::emote_anchor(&look, &wv::person_pose(&v, t));
        render::draw_helper_ring(anchor, helper_icon(content, &hw.doing), hw.progress, t);
    }
}
