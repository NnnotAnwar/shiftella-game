//! Build wishes — what a regular or a helper would like to see in the café
//! (design/v0.2.md §A.6 / §C.4).
//!
//! STUB owned by package A (`build-ux`): it returns the hard facts the story already has.
//! Package C (`build-content`) replaces the BODY of [`World::build_wishes`] with the
//! data-driven version (`assets/data/build_wishes.ron`); keep the signature.

use super::world::World;
use crate::content::Content;

/// One open wish: `who` wants a piece `def` (or any piece tagged `tag`).
#[derive(Debug, Clone, PartialEq)]
pub struct BuildWish {
    /// Regular or staff id (`filip`, `lori`, `hana`, …).
    pub who: String,
    pub def: Option<String>,
    pub tag: Option<String>,
    /// i18n key of the one-line reason ("a table with a socket for the laptop").
    pub reason_key: String,
}

impl World {
    /// Open build wishes right now (fulfilled ones are gone).
    pub fn build_wishes(&self, content: &Content) -> Vec<BuildWish> {
        let has_def = |id: &str| self.furniture.iter().any(|p| p.def == id);
        let has_tag = |tag: &str| {
            self.furniture.iter().any(|p| {
                content
                    .furniture_def(&p.def)
                    .is_some_and(|d| d.has_tag(tag))
            })
        };
        let met = |id: &str| self.story.regulars.contains_key(id);
        let mut out = Vec::new();
        // Filip asks for a table with a socket from night 7 (the bot builds it then too)
        if self.night >= 7 && met("filip") && !has_tag("power_socket") {
            out.push(BuildWish {
                who: "filip".into(),
                def: None,
                tag: Some("power_socket".into()),
                reason_key: "bux.wish.filip".into(),
            });
        }
        // Lori plays once she's invited — on a little stage
        if self.story.flags.contains("lori_invited") && !has_tag("stage") {
            out.push(BuildWish {
                who: "lori".into(),
                def: None,
                tag: Some("stage".into()),
                reason_key: "bux.wish.lori".into(),
            });
        }
        // Hana's quiet corner: an armchair and a floor lamp (her story choice checks both)
        let hana_done = ["hana_quiet_corner", "hana_stays", "hana_left"]
            .iter()
            .any(|f| self.story.flags.contains(*f));
        if met("hana") && !hana_done {
            for def in ["armchair", "floor_lamp"] {
                if !has_def(def) {
                    out.push(BuildWish {
                        who: "hana".into(),
                        def: Some(def.into()),
                        tag: None,
                        reason_key: "bux.wish.hana".into(),
                    });
                }
            }
        }
        out
    }
}
