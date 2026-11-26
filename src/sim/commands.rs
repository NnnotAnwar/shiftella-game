//! `World::apply(Command)` and `World::undo()`.

use super::grid::{self, Floor, Placed};
use super::types::*;
use super::world::*;
use crate::content::handson::Duty;
use crate::content::{BuildCategory, Content, FurnitureDef, Job};

const UNDO_DEPTH: usize = 50;

pub(crate) fn furniture_unlocked(w: &World, f: &FurnitureDef) -> bool {
    if w.story.unlocked_furniture.contains(&f.id) {
        return true;
    }
    let flag_ok = f
        .unlock_flag
        .as_ref()
        .is_none_or(|fl| w.story.flags.contains(fl));
    let night_ok = match w.mode {
        Mode::Story => f.unlock_night <= w.night,
        Mode::Sandbox => f.unlock_night <= super::guests::sandbox_tier(w) + 2,
    };
    flag_ok && night_ok && f.category != BuildCategory::Structure
}

fn def<'a>(content: &'a Content, id: &str) -> Result<&'a FurnitureDef, CommandError> {
    content
        .furniture_def(id)
        .ok_or_else(|| CommandError::UnknownId(id.to_string()))
}

fn occupy(w: &mut World, d: &FurnitureDef, p: &Placed, on: bool) {
    let rug = grid::is_rug_layer(d);
    for (x, y) in grid::footprint(d, p.x, p.y, p.rot) {
        if let Some(i) = w.grid.idx(x, y) {
            let layer = if rug {
                &mut w.grid.rugs[i]
            } else {
                &mut w.grid.objects[i]
            };
            *layer = if on { Some(p.uid) } else { None };
        }
    }
}

/// Every non-rug piece must stay usable (an access tile reachable from the entrance).
fn connectivity_ok(w: &World, content: &Content) -> bool {
    let reach = grid::reachable(&w.grid, w.grid.entrance());
    for p in &w.furniture {
        let Some(d) = content.furniture_def(&p.def) else {
            continue;
        };
        if grid::is_rug_layer(d) || d.has_tag("decor") && !d.has_tag("light") {
            // pure decor may sit in a corner
            if !d.has_tag("seat") {
                continue;
            }
        }
        let fp = grid::footprint(d, p.x, p.y, p.rot);
        let ok = fp.iter().any(|&(x, y)| {
            (d.walkable && w.grid.idx(x, y).is_some_and(|i| reach[i]))
                || [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| {
                    w.grid
                        .idx(x + dx, y + dy)
                        .is_some_and(|i| reach[i] && !fp.contains(&(x + dx, y + dy)))
                })
        });
        if !ok {
            return false;
        }
    }
    true
}

fn place_piece(w: &mut World, content: &Content, piece: Placed) -> Result<(), CommandError> {
    let d = def(content, &piece.def)?;
    grid::can_place(&w.grid, d, piece.x, piece.y, piece.rot, None)?;
    occupy(w, d, &piece, true);
    w.furniture.push(piece.clone());
    w.grid.rebuild(&w.furniture, content);
    if !connectivity_ok(w, content) {
        w.furniture.pop();
        occupy(w, d, &piece, false);
        w.grid.rebuild(&w.furniture, content);
        return Err(CommandError::BlocksPath);
    }
    Ok(())
}

/// Places a free piece (migrations: grandma's jukebox in an old save). False if it won't fit.
pub(crate) fn place_free(
    w: &mut World,
    content: &Content,
    def: &str,
    x: i32,
    y: i32,
    rot: u8,
) -> bool {
    let Some(d) = content.furniture_def(def) else {
        return false;
    };
    let uid = w.alloc_id();
    let piece = Placed {
        uid,
        def: def.to_string(),
        x,
        y,
        rot,
        on: false,
        dirt: 0.0,
        dishes: 0,
        wear: 0.0,
        seat_by: vec![None; d.seats as usize],
        user: None,
        claim: None,
        paid: 0,
    };
    place_piece(w, content, piece).is_ok()
}

fn in_use(_w: &World, p: &Placed) -> bool {
    p.user.is_some() || p.claim.is_some() || p.seat_by.iter().any(|s| s.is_some())
}

fn remove_piece(w: &mut World, content: &Content, uid: u32) -> Result<Placed, CommandError> {
    let i = w
        .furniture
        .iter()
        .position(|p| p.uid == uid)
        .ok_or_else(|| CommandError::UnknownId(uid.to_string()))?;
    if in_use(w, &w.furniture[i]) {
        return Err(CommandError::InUse);
    }
    let p = w.furniture.remove(i);
    if let Some(d) = content.furniture_def(&p.def) {
        occupy(w, d, &p, false);
    }
    // rugs share tiles: re-mark remaining pieces
    for q in w.furniture.clone() {
        if let Some(d) = content.furniture_def(&q.def) {
            occupy(w, d, &q, true);
        }
    }
    w.grid.rebuild(&w.furniture, content);
    Ok(p)
}

fn push_undo(w: &mut World, op: UndoOp) {
    w.undo.push(op);
    if w.undo.len() > UNDO_DEPTH {
        w.undo.remove(0);
    }
}

fn staff_idx(w: &World, id: u32) -> Result<usize, CommandError> {
    w.staff
        .iter()
        .position(|s| s.id == id)
        .ok_or_else(|| CommandError::UnknownId(id.to_string()))
}

impl World {
    /// Applies a player command. Reversible ones go on the undo stack.
    pub fn apply(&mut self, content: &Content, cmd: Command) -> Result<(), CommandError> {
        if self.closed && !matches!(cmd, Command::CutsceneFinished { .. }) {
            return Err(CommandError::GameOver);
        }
        if self.story.playing.is_some() && !matches!(cmd, Command::CutsceneFinished { .. }) {
            return Err(CommandError::CutscenePlaying);
        }
        self.ensure_hands(content);
        self.grid.ensure_masks(&self.furniture, content);
        match cmd {
            Command::PlaceFurniture { def: id, x, y, rot } => {
                let d = def(content, &id)?;
                if d.unique && self.furniture.iter().any(|p| p.def == id) {
                    return Err(CommandError::AlreadyPlaced);
                }
                if !furniture_unlocked(self, d) {
                    return Err(CommandError::NotUnlocked);
                }
                let price = d.price as i64;
                if d.category == BuildCategory::Floors {
                    return self.apply(content, Command::SetFloor { x, y, floor: id });
                }
                if self.econ.money < price {
                    return Err(CommandError::NotEnoughMoney {
                        need: price,
                        have: self.econ.money,
                    });
                }
                let uid = self.alloc_id();
                let piece = Placed {
                    uid,
                    def: id.clone(),
                    x,
                    y,
                    rot: rot % 4,
                    on: d.has_tag("light") && !d.has_tag("music"),
                    dirt: 0.0,
                    dishes: 0,
                    wear: 0.0,
                    seat_by: vec![None; d.seats as usize],
                    user: None,
                    claim: None,
                    paid: d.price,
                };
                place_piece(self, content, piece)?;
                self.econ.money -= price;
                if self.phase == Phase::Night {
                    self.tonight.other_expenses += price;
                }
                push_undo(self, UndoOp::Unplace { uid, refund: price });
                self.emit(SimEvent::FurniturePlaced { uid });
                Ok(())
            }
            Command::MoveFurniture { uid, x, y, rot } => {
                let old = self
                    .furniture_by_uid(uid)
                    .cloned()
                    .ok_or_else(|| CommandError::UnknownId(uid.to_string()))?;
                self.move_piece(content, uid, x, y, rot % 4)?;
                push_undo(
                    self,
                    UndoOp::MoveBack {
                        uid,
                        x: old.x,
                        y: old.y,
                        rot: old.rot,
                    },
                );
                Ok(())
            }
            Command::RotateFurniture { uid } => {
                let old = self
                    .furniture_by_uid(uid)
                    .cloned()
                    .ok_or_else(|| CommandError::UnknownId(uid.to_string()))?;
                self.move_piece(content, uid, old.x, old.y, (old.rot + 1) % 4)?;
                push_undo(
                    self,
                    UndoOp::MoveBack {
                        uid,
                        x: old.x,
                        y: old.y,
                        rot: old.rot,
                    },
                );
                Ok(())
            }
            Command::RemoveFurniture { uid } => {
                let p = remove_piece(self, content, uid)?;
                let refund = (p.paid / 2) as i64;
                self.econ.money += refund;
                push_undo(
                    self,
                    UndoOp::Replace {
                        piece: p,
                        charge: refund,
                    },
                );
                self.emit(SimEvent::FurnitureRemoved { uid });
                Ok(())
            }
            Command::SetFloor { x, y, floor } => {
                let f =
                    Floor::from_id(&floor).ok_or_else(|| CommandError::UnknownId(floor.clone()))?;
                let price = content
                    .furniture_def(&floor)
                    .map(|d| d.price as i64)
                    .unwrap_or(0);
                let i = self.grid.idx(x, y).ok_or(CommandError::OutOfBounds)?;
                if !self.grid.is_indoor_floor(x, y) {
                    return Err(CommandError::OutOfBounds);
                }
                let old = self.grid.tiles[i].floor;
                if old == f {
                    return Ok(());
                }
                if self.econ.money < price {
                    return Err(CommandError::NotEnoughMoney {
                        need: price,
                        have: self.econ.money,
                    });
                }
                self.econ.money -= price;
                self.grid.tiles[i].floor = f;
                self.grid.rebuild(&self.furniture, content);
                push_undo(
                    self,
                    UndoOp::Floor {
                        x,
                        y,
                        floor: old,
                        refund: price,
                    },
                );
                Ok(())
            }
            Command::SetFurnitureOn { uid, on } => {
                let p = self
                    .furniture
                    .iter_mut()
                    .find(|p| p.uid == uid)
                    .ok_or_else(|| CommandError::UnknownId(uid.to_string()))?;
                let old = p.on;
                p.on = on;
                let is_music = content
                    .furniture_def(&p.def)
                    .is_some_and(|d| d.has_tag("music"));
                if is_music {
                    super::staff::set_music(self, content, on);
                }
                push_undo(self, UndoOp::FurnitureOn { uid, old });
                Ok(())
            }
            Command::SetMusic { on } => {
                let has = self.furniture.iter().any(|p| {
                    content
                        .furniture_def(&p.def)
                        .is_some_and(|d| d.has_tag("music"))
                });
                if !has {
                    return Err(CommandError::Invalid("no radio or jukebox".into()));
                }
                super::staff::set_music(self, content, on);
                Ok(())
            }
            Command::Hire { candidate } => {
                let i = self
                    .candidates
                    .iter()
                    .position(|c| c.id == candidate)
                    .ok_or_else(|| CommandError::UnknownId(candidate.to_string()))?;
                let mut s = self.candidates.remove(i).staff;
                s.hired_night = self.night;
                s.personal_number = 100 + self.staff.len() as u32;
                s.pos = grid::tile_center(self.grid.entrance());
                let id = s.id;
                self.push_staff(s);
                super::app::crew_changed(self, content);
                if self.phase == Phase::Night {
                    // starts tomorrow
                    if let Some(st) = self.staff.iter_mut().find(|s| s.id == id) {
                        st.status = StaffStatus::OffDuty;
                    }
                }
                self.emit(SimEvent::StaffHired { staff: id });
                // the display name (proper noun): the log must still read well after they leave
                let name = self
                    .staff
                    .last()
                    .map(|s| s.name.clone())
                    .unwrap_or_default();
                self.log_key(
                    "ui.crew.hired",
                    vec![("staff".into(), name), ("id".into(), id.to_string())],
                );
                Ok(())
            }
            Command::Fire { staff } => {
                let i = staff_idx(self, staff)?;
                if self.staff[i].owner {
                    return Err(CommandError::Invalid("the owner can't be fired".into()));
                }
                super::staff::abandon(self, i);
                self.staff.remove(i);
                self.app.draft.remove(&staff);
                self.app.published.remove(&staff);
                super::app::crew_changed(self, content);
                self.emit(SimEvent::StaffFired { staff });
                Ok(())
            }
            // Legacy shim for the old priorities grid: 1–4 = the job's duty on, 0 = off
            // (cleaning → dishes; off also drops the floor duty).
            Command::SetPriority {
                staff,
                job,
                priority,
            } => {
                staff_idx(self, staff)?;
                if priority > 4 {
                    return Err(CommandError::Invalid("priority 0–4".into()));
                }
                let duty = match job {
                    Job::Cleaning => Duty::Dishes,
                    j => Duty::for_job(j),
                };
                let on = priority > 0;
                if !on && job == Job::Cleaning {
                    let was = super::duties::set_duty(self, content, staff, Duty::Floor, false)?;
                    if was {
                        push_undo(
                            self,
                            UndoOp::Duty {
                                staff,
                                duty: Duty::Floor,
                                was_on: true,
                            },
                        );
                    }
                }
                let was = super::duties::set_duty(self, content, staff, duty, on)?;
                if was != on {
                    push_undo(
                        self,
                        UndoOp::Duty {
                            staff,
                            duty,
                            was_on: was,
                        },
                    );
                }
                Ok(())
            }
            Command::SetDuty { staff, duty, on } => {
                let was = super::duties::set_duty(self, content, staff, duty, on)?;
                if was != on {
                    push_undo(
                        self,
                        UndoOp::Duty {
                            staff,
                            duty,
                            was_on: was,
                        },
                    );
                }
                Ok(())
            }
            Command::SetShift {
                staff,
                weekday,
                shift,
            } => {
                staff_idx(self, staff)?;
                let wd = weekday as usize % 7;
                let was_dirty = self.app.draft_dirty;
                let row = self.app.draft.entry(staff).or_insert([Shift::Full; 7]);
                let old = row[wd];
                row[wd] = shift;
                self.app.draft_dirty = true;
                self.app.manual_rozpis = true;
                push_undo(
                    self,
                    UndoOp::Shift {
                        staff,
                        weekday: wd as u8,
                        old,
                        was_dirty,
                    },
                );
                super::rozpis::score(self, content);
                Ok(())
            }
            Command::AutoSchedule | Command::AutoFillRozpis => {
                let old = self.app.draft.clone();
                let was_dirty = self.app.draft_dirty;
                let was_manual = self.app.manual_rozpis;
                let base = super::app::base_rozpis(self, content);
                let (rows, report) = super::rozpis::autofill(self, content, base);
                self.app.draft = rows;
                self.sys.rozpis.last_report = report;
                super::rozpis::score(self, content);
                self.app.manual_rozpis = false;
                self.app.draft_dirty = self.app.draft != self.app.published;
                push_undo(
                    self,
                    UndoOp::Draft {
                        old,
                        was_dirty,
                        was_manual,
                    },
                );
                Ok(())
            }
            Command::PublishSchedule => {
                self.app.published = self.app.draft.clone();
                let changed = self.app.draft_dirty;
                self.app.draft_dirty = false;
                if changed {
                    for s in self.staff.iter_mut() {
                        s.mood = (s.mood + 2.0).min(100.0);
                    }
                }
                self.emit(SimEvent::SchedulePublished);
                let ctx = EffectCtx::default();
                super::app::bot(self, content, "schedule_published", &ctx);
                super::rozpis::on_published(self, content);
                Ok(())
            }
            Command::AnswerSwap { id, approve } => {
                super::app::answer_swap(self, id, approve)?;
                super::app::answered(self, content, id, approve);
                Ok(())
            }
            Command::AnswerTimeOff { id, approve } => {
                super::app::answer_time_off(self, id, approve)?;
                super::app::crew_changed(self, content);
                super::app::answered(self, content, id, approve);
                Ok(())
            }
            Command::RequestTimeOff {
                staff,
                kind,
                weekday,
            } => {
                staff_idx(self, staff)?;
                let rid = self.alloc_id();
                let ahead = (weekday as u32 + 7 - self.weekday as u32) % 7;
                let night = self.night + if ahead == 0 { 7 } else { ahead };
                self.app.time_off.push(TimeOffRequest {
                    id: rid,
                    staff,
                    kind,
                    night,
                    hours: 8.0,
                    status: RequestStatus::Pending,
                });
                Ok(())
            }
            Command::AnswerEvent { event, choice } => {
                super::narrator::answer(self, content, &event, &choice)
            }
            Command::BotAction { msg, action } => {
                super::app::bot_action(self, content, msg, &action)
            }
            Command::CutsceneFinished { id, choices } => {
                super::story::finish_cutscene(self, content, &id, &choices)
            }
            Command::Talk { guest } => super::story::talk(self, content, guest),
            Command::BuySupplies { amount } => super::economy::buy_supplies(self, amount),
            Command::SetAutoRestock { target } => {
                self.econ.auto_restock = target;
                Ok(())
            }
            Command::SetPrice { item, price } => {
                let m = content
                    .menu_item(&item)
                    .ok_or_else(|| CommandError::UnknownId(item.clone()))?;
                if price == 0 || price > m.price * 3 {
                    return Err(CommandError::Invalid("price out of range".into()));
                }
                let old = self.econ.prices.insert(item.clone(), price);
                push_undo(self, UndoOp::Price { item, old });
                Ok(())
            }
            Command::SetMenuItem { item, on } => {
                content
                    .menu_item(&item)
                    .ok_or_else(|| CommandError::UnknownId(item.clone()))?;
                let was_off = self.econ.menu_off.contains(&item);
                if on {
                    self.econ.menu_off.remove(&item);
                } else {
                    self.econ.menu_off.insert(item.clone());
                }
                push_undo(self, UndoOp::MenuItem { item, was_off });
                Ok(())
            }
            Command::RenameCafe { name } => {
                let name = name.trim().to_string();
                if name.is_empty() || name.chars().count() > 40 {
                    return Err(CommandError::Invalid("name length".into()));
                }
                self.cafe_name = name;
                Ok(())
            }
            // At night a checklist item is Karolina's act: she walks to it and ticks it.
            Command::TickChecklist { id } => {
                if !self.app.checklist.iter().any(|c| c.id == id) {
                    if content.app.checklists.iter().any(|c| c.id == id) {
                        return Err(CommandError::WrongPhase);
                    }
                    return Err(CommandError::UnknownId(id));
                }
                super::player::queue(
                    self,
                    content,
                    super::hands::PlayerAct::Checklist { id },
                    false,
                )
            }
            Command::PostAnnouncement { id, args } => {
                let a = content
                    .app
                    .announcements
                    .iter()
                    .find(|a| a.id == id)
                    .ok_or_else(|| CommandError::UnknownId(id.clone()))?;
                let args_copy = args.clone();
                self.app.announcements.push(PostedAnnouncement {
                    id: id.clone(),
                    author: a.author.clone(),
                    night: self.night,
                    args,
                });
                for s in self.staff.iter_mut() {
                    s.mood = (s.mood + 1.0).min(100.0);
                }
                // its real effect: the person / item named in it
                let fx = a.effects.clone();
                if !fx.is_empty() {
                    let mut t = super::systems::XTarget::default();
                    for (k, v) in &args_copy {
                        match k.as_str() {
                            "name" => {
                                t.staff = self
                                    .staff
                                    .iter()
                                    .find(|s| &s.name == v || s.id.to_string() == *v)
                                    .map(|s| s.id)
                            }
                            "item" => t.item = Some(v.clone()),
                            _ => {}
                        }
                    }
                    super::xlogic::run(self, content, &fx, &t);
                }
                let ctx = EffectCtx::default();
                super::app::bot_with(
                    self,
                    content,
                    "announcement_posted",
                    &ctx,
                    vec![("title".into(), format!("app.announcement.{id}.title"))],
                );
                Ok(())
            }
            Command::WriteHandover { text } => {
                self.app.handover_text = Some(text);
                Ok(())
            }
            Command::SendOnBreak { staff } => {
                if self.phase != Phase::Night {
                    return Err(CommandError::WrongPhase);
                }
                let i = staff_idx(self, staff)?;
                let s = &self.staff[i];
                if s.owner {
                    return Err(CommandError::Invalid(
                        "Karolina takes her breaks herself".into(),
                    ));
                }
                if !s.on_duty() {
                    return Err(CommandError::Invalid("not on the floor".into()));
                }
                if matches!(s.task, Some(Task::Break { .. })) {
                    return Err(CommandError::Invalid("already on a break".into()));
                }
                if self.minute() >= NIGHT_MINUTES - 5 {
                    return Err(CommandError::Invalid("the night is almost over".into()));
                }
                let task = super::staff::break_task(self, content, i, super::staff::BREAK_MINUTES);
                super::staff::start_break(self, i, task);
                Ok(())
            }
            Command::StartNight => {
                if self.phase != Phase::Day {
                    return Err(CommandError::WrongPhase);
                }
                self.start_night(content);
                Ok(())
            }
            Command::PlayerTakeOrder { guest } => super::player::take_order(self, content, guest),
            Command::PlayerBrew {
                guest,
                recipe,
                quality,
                mode,
            } => super::player::brew(self, content, guest, &recipe, quality, mode),
            Command::PlayerUsual { guest, quality } => {
                super::player::usual(self, content, guest, quality)
            }
            Command::PlayerHandOff { guest } => super::player::hand_off(self, content, guest),
            Command::PlayerCancel => super::player::cancel(self, content),
            Command::PlayerQueue { act, front } => super::player::queue(self, content, act, front),
            Command::PlayerUnqueue { index } => super::player::unqueue(self, content, index),
            Command::PlayerWashProgress { sink, seconds } => {
                super::player::wash_progress(self, sink, seconds)
            }
            Command::PlayerGreet { guest } => super::player::greet(self, content, guest),
            Command::PlayRecord { id, repeat } => super::player::queue(
                self,
                content,
                super::hands::PlayerAct::PlayRecord { record: id, repeat },
                false,
            ),
            Command::RecordFinished { id } => super::music::record_finished(self, content, &id),
            Command::SetAssistWishes { on } => {
                self.hands.assist_wishes = on;
                Ok(())
            }
            Command::ResolveMoment { id, option } => {
                super::moments::resolve(self, content, &id, &option)
            }
            Command::TryExperiment { ingredients } => {
                self.apply(content, Command::Experiment { ingredients })
            }
            Command::PlayerBrewStart {
                guest,
                recipe,
                mode,
            } => super::player::brew_start(self, content, guest, &recipe, mode),
            Command::PlayerBrewQuality { guest, quality } => {
                super::player::brew_quality_in(self, guest, quality)
            }
            Command::TutorialShown { id } => {
                super::tool::tutorial_show(self, content, &id, true);
                Ok(())
            }
            Command::SetTutorial { on } => {
                self.sys.tutorial_off = !on;
                Ok(())
            }
            Command::Experiment { ingredients } => {
                super::knowledge::validate(self, content, &ingredients)?;
                if self.phase == Phase::Day {
                    super::knowledge::run(self, content, &ingredients);
                    Ok(())
                } else {
                    let tile = super::staff::checklist_tile(self, content, Job::Register);
                    super::player::queue_walk(
                        self,
                        content,
                        tile,
                        super::systems::WalkKind::Experiment { ingredients },
                    )
                }
            }
            Command::PickTipGoal { choice } => {
                super::moments::pick_tip_goal(self, content, &choice)
            }
            Command::SetSpecials { recipes } => {
                if recipes.len() > 2 {
                    return Err(CommandError::Invalid("two specials at most".into()));
                }
                for r in &recipes {
                    if !self.hands.knowledge.recipes.contains_key(r) {
                        return Err(CommandError::NotUnlocked);
                    }
                }
                self.sys.extras.specials = recipes;
                if self.phase == Phase::Night
                    && self
                        .app
                        .checklist
                        .iter()
                        .any(|c| c.id == "menu_board" && !c.done)
                {
                    super::app::tick_checklist(self, content, "menu_board", None);
                }
                Ok(())
            }
            Command::BuyTheme { id } => super::atmos::buy_theme(self, content, &id),
            Command::SetTheme { id } => {
                let old = super::atmos::set_theme(self, content, &id)?;
                if old != id {
                    push_undo(self, UndoOp::Theme { old });
                }
                super::atmos::recompute(self, content);
                Ok(())
            }
            Command::PickPolaroid { index } => super::journal::keep(self, content, index),
            Command::AnswerEarlyLeave { id, approve } => {
                super::rozpis::answer_early_leave(self, content, id, approve)
            }
            Command::PickHandover { picks } => super::tool::pick_handover(self, content, picks),
            Command::SetTipSplit { split } => {
                self.sys.tool.tip_split = Some(split);
                Ok(())
            }
            Command::PayBonus { staff } => super::tool::pay_bonus(self, staff),
            Command::GrantToil { staff, weekday } => {
                super::tool::grant_toil(self, content, staff, weekday)
            }
            Command::SendPayroll { week } => super::tool::send_payroll(self, content, week),
        }
    }

    fn move_piece(
        &mut self,
        content: &Content,
        uid: u32,
        x: i32,
        y: i32,
        rot: u8,
    ) -> Result<(), CommandError> {
        let i = self
            .furniture
            .iter()
            .position(|p| p.uid == uid)
            .ok_or_else(|| CommandError::UnknownId(uid.to_string()))?;
        if in_use(self, &self.furniture[i]) {
            return Err(CommandError::InUse);
        }
        let d = def(content, &self.furniture[i].def)?;
        grid::can_place(&self.grid, d, x, y, rot, Some(uid))?;
        let old = self.furniture[i].clone();
        occupy(self, d, &old, false);
        let mut new = old.clone();
        new.x = x;
        new.y = y;
        new.rot = rot;
        occupy(self, d, &new, true);
        self.furniture[i] = new;
        self.grid.rebuild(&self.furniture, content);
        if !connectivity_ok(self, content) {
            let cur = self.furniture[i].clone();
            occupy(self, d, &cur, false);
            occupy(self, d, &old, true);
            self.furniture[i] = old;
            self.grid.rebuild(&self.furniture, content);
            return Err(CommandError::BlocksPath);
        }
        Ok(())
    }

    /// Reverts the last reversible command (furniture, floors, priorities, draft schedule,
    /// prices, menu, lamps).
    pub fn undo(&mut self, content: &Content) -> Result<(), CommandError> {
        let op = self.undo.pop().ok_or(CommandError::NothingToUndo)?;
        let r = match op.clone() {
            UndoOp::Unplace { uid, refund } => remove_piece(self, content, uid).map(|_| {
                self.econ.money += refund;
                if self.phase == Phase::Night {
                    self.tonight.other_expenses -= refund;
                }
                self.emit(SimEvent::FurnitureRemoved { uid });
            }),
            UndoOp::Replace { piece, charge } => {
                let uid = piece.uid;
                place_piece(self, content, piece).map(|_| {
                    self.econ.money -= charge;
                    self.emit(SimEvent::FurniturePlaced { uid });
                })
            }
            UndoOp::MoveBack { uid, x, y, rot } => self.move_piece(content, uid, x, y, rot),
            UndoOp::Floor {
                x,
                y,
                floor,
                refund,
            } => {
                if let Some(i) = self.grid.idx(x, y) {
                    self.grid.tiles[i].floor = floor;
                    self.econ.money += refund;
                    self.grid.rebuild(&self.furniture, content);
                }
                Ok(())
            }
            UndoOp::Priority { staff, job, old } => {
                let i = staff_idx(self, staff)?;
                self.staff[i].priorities[job as usize % 5] = old;
                Ok(())
            }
            UndoOp::Duty {
                staff,
                duty,
                was_on,
            } => super::duties::set_duty(self, content, staff, duty, was_on).map(|_| ()),
            UndoOp::Shift {
                staff,
                weekday,
                old,
                was_dirty,
            } => {
                if let Some(row) = self.app.draft.get_mut(&staff) {
                    row[weekday as usize % 7] = old;
                }
                self.app.draft_dirty = was_dirty;
                Ok(())
            }
            UndoOp::Draft {
                old,
                was_dirty,
                was_manual,
            } => {
                self.app.draft = old;
                self.app.draft_dirty = was_dirty;
                self.app.manual_rozpis = was_manual;
                Ok(())
            }
            UndoOp::Price { item, old } => {
                match old {
                    Some(p) => self.econ.prices.insert(item, p),
                    None => self.econ.prices.remove(&item),
                };
                Ok(())
            }
            UndoOp::MenuItem { item, was_off } => {
                if was_off {
                    self.econ.menu_off.insert(item);
                } else {
                    self.econ.menu_off.remove(&item);
                }
                Ok(())
            }
            UndoOp::FurnitureOn { uid, old } => {
                if let Some(p) = self.furniture.iter_mut().find(|p| p.uid == uid) {
                    p.on = old;
                }
                Ok(())
            }
            UndoOp::Theme { old } => super::atmos::set_theme(self, content, &old).map(|_| ()),
        };
        if r.is_ok() && matches!(op, UndoOp::Shift { .. } | UndoOp::Draft { .. }) {
            super::rozpis::score(self, content);
        }
        if r.is_err() {
            // couldn't revert (e.g. a guest sits on the chair now): keep it for later
            self.undo.push(op);
        }
        r
    }
}
