//! Delegation (design/handson.md §1.3): helpers only do the duties the player gave them —
//! six duties, capacity 1–3 per helper, a default duty per archetype on the hire card.
//! Karolina (the owner) has no duties: the sim never assigns her anything.

use super::types::*;
use super::world::*;
use crate::content::handson::Duty;
use crate::content::{Content, Job};

pub(crate) fn duty_index(d: Duty) -> usize {
    Duty::ALL.iter().position(|x| *x == d).unwrap_or(0)
}

pub(crate) fn has_duty(s: &Staff, d: Duty) -> bool {
    s.duties.contains(&d)
}

/// Capacity = `base + [nights_worked ≥ after_nights] + [a held duty's skill ≥ skill_bonus_at
/// or trait]`, at most `max`. `adding` counts as held for the skill check.
pub(crate) fn capacity(content: &Content, s: &Staff, night: u32, adding: Option<Duty>) -> u32 {
    let c = &content.handson.tuning.capacity;
    let mut cap = c.base;
    if s.nights_worked >= c.after_nights {
        cap += 1;
    }
    let skilled = s
        .duties
        .iter()
        .copied()
        .chain(adding)
        .any(|d| s.skill(d.job(), night) >= c.skill_bonus_at);
    if skilled || s.has_trait(&c.trait_bonus) {
        cap += 1;
    }
    cap.min(c.max).max(1)
}

/// `priorities` mirrors the duties (on → 1, off → 0) for old UI and save compatibility.
pub(crate) fn sync_priorities(s: &mut Staff) {
    let mut p = [0u8; 5];
    for d in &s.duties {
        p[d.job().index()] = 1;
    }
    s.priorities = p;
}

/// The duty the hire card proposes (`StaffArchetype.default_duty`, else the role's duty).
pub(crate) fn default_duties(content: &Content, archetype: &str) -> Vec<Duty> {
    let Some(a) = content.staff_archetype(archetype) else {
        return Vec::new();
    };
    let d = a.default_duty.unwrap_or(match a.role {
        Job::Cleaning => Duty::Dishes,
        j => Duty::for_job(j),
    });
    // a second duty that always has something to do, so a new helper never just stands
    // there (capacity starts at 2): the dishes, or the floor for whoever already washes up
    let second = if d == Duty::Dishes { Duty::Floor } else { Duty::Dishes };
    vec![d, second]
}

/// Save v1 → v2: every priority-1 job becomes a duty (cleaning → dishes).
pub(crate) fn duties_from_priorities(p: &[u8; 5]) -> Vec<Duty> {
    let mut v = Vec::new();
    for job in Job::ALL {
        if p[job.index()] == 1 {
            v.push(match job {
                Job::Cleaning => Duty::Dishes,
                j => Duty::for_job(j),
            });
        }
    }
    v
}

/// Some helper on the floor holds this duty right now.
pub(crate) fn duty_on_floor(w: &World, d: Duty) -> bool {
    w.staff
        .iter()
        .any(|s| !s.owner && s.on_duty() && has_duty(s, d))
}

/// Some helper holds it tonight (on the floor, or still to arrive).
pub(crate) fn duty_tonight(w: &World, d: Duty) -> bool {
    w.staff.iter().any(|s| {
        !s.owner
            && matches!(s.status, StaffStatus::OnDuty | StaffStatus::Expected { .. })
            && has_duty(s, d)
    })
}

/// The duty a helper's task belongs to (digest credits).
pub(crate) fn duty_of_task(task: &Task, content: &Content, w: &World) -> Option<Duty> {
    Some(match task {
        Task::TakeOrder { .. } => Duty::Register,
        Task::Prepare { item, .. } => {
            let job = w
                .items
                .iter()
                .find(|i| i.id == *item)
                .and_then(|i| content.menu_item(&i.menu))
                .map(|m| m.job)?;
            Duty::for_job(job)
        }
        Task::Serve { .. } => Duty::Serving,
        Task::ClearTable { .. } | Task::Wash { .. } => Duty::Dishes,
        Task::Mop { .. } | Task::Bucket { .. } => Duty::Floor,
        _ => return None,
    })
}

/// `Command::SetDuty`.
pub(crate) fn set_duty(
    w: &mut World,
    content: &Content,
    staff: u32,
    duty: Duty,
    on: bool,
) -> Result<bool, CommandError> {
    let i = w
        .staff
        .iter()
        .position(|s| s.id == staff)
        .ok_or_else(|| CommandError::UnknownId(staff.to_string()))?;
    if w.staff[i].owner {
        return Err(CommandError::Invalid(
            "Karolina has no duties — she does what you ask".into(),
        ));
    }
    let was = has_duty(&w.staff[i], duty);
    if was == on {
        return Ok(was);
    }
    if on {
        let cap = capacity(content, &w.staff[i], w.night, Some(duty));
        if w.staff[i].duties.len() as u32 >= cap {
            return Err(CommandError::Capacity);
        }
        w.staff[i].duties.push(duty);
        w.staff[i].duties.sort();
    } else {
        w.staff[i].duties.retain(|d| *d != duty);
        // drop a task of that duty right away
        if let Some(t) = w.staff[i].task.clone()
            && duty_of_task(&t, content, w) == Some(duty)
        {
            super::staff::abandon(w, i);
        }
    }
    sync_priorities(&mut w.staff[i]);
    Ok(was)
}

/// Digest credits: each helper's busiest duty tonight.
pub(crate) fn helper_credits(w: &World) -> Vec<(String, String, u32)> {
    let mut v = Vec::new();
    for s in w.staff.iter().filter(|s| !s.owner) {
        if let Some((k, n)) = s
            .duty_done
            .iter()
            .enumerate()
            .filter(|(_, n)| **n > 0)
            .max_by_key(|(k, n)| (**n, std::cmp::Reverse(*k)))
        {
            v.push((s.name.clone(), Duty::ALL[k].id().to_string(), *n));
        }
    }
    v
}
