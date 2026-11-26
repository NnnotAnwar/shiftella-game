//! Sim tests: grid & pathfinding, placement & undo, job assignment, guest lifecycle, economy,
//! storyteller pacing, story flow, the Shiftella app, saves, determinism and 3-night integration runs.

use super::grid::{self, TileKind};
use super::*;
use crate::content::handson::Duty;
use crate::content::{Content, Job};

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");

fn content() -> Content {
    Content::load(DATA).unwrap_or_else(|e| panic!("{e}"))
}

fn finish_cutscenes(w: &mut World, c: &Content) {
    while let Some(id) = w.playing_cutscene().map(str::to_string) {
        w.apply(
            c,
            Command::CutsceneFinished {
                id,
                choices: vec![],
            },
        )
        .unwrap();
    }
}

/// Runs the current night to 06:00 with nobody behind the counter (Karolina steps back).
fn run_night(w: &mut World, c: &Content) -> Vec<SimEvent> {
    w.run_night_headless(c)
}

/// Runs the current night with the player-bot (Karolina works; no hiring plan).
fn play_night(w: &mut World, c: &Content) -> Vec<SimEvent> {
    let mut bot = bot::PlayerBot::new(bot::BotConfig {
        hire_nights: Vec::new(),
        ..Default::default()
    });
    bot.run_night(w, c)
}

fn sandbox(c: &Content, seed: u64, difficulty: Difficulty) -> World {
    World::new_sandbox(
        c,
        SandboxSetup {
            seed,
            difficulty,
            ..Default::default()
        },
    )
}

fn place(def: &str, x: i32, y: i32) -> Command {
    Command::PlaceFurniture {
        def: def.into(),
        x,
        y,
        rot: 0,
    }
}

// ---------------------------------------------------------------------------------------------
// Grid & pathfinding
// ---------------------------------------------------------------------------------------------

#[test]
fn default_layout_is_well_formed() {
    for (i, row) in grid::DEFAULT_LAYOUT.lines().enumerate() {
        assert_eq!(row.chars().count(), 30, "row {i}");
    }
    let c = content();
    let w = World::new_story(&c, Difficulty::Cozy);
    let g = w.grid();
    assert_eq!((g.width, g.height), (30, 18));
    assert_eq!(g.tile(1, 15).unwrap().kind, TileKind::Door);
    assert_eq!(g.tile(5, 17).unwrap().kind, TileKind::Street);
    for tag in [
        "register",
        "station:coffee",
        "station:stove",
        "station:fridge",
        "sink",
        "table",
        "seat",
    ] {
        assert!(
            w.furniture()
                .iter()
                .any(|p| c.furniture_def(&p.def).is_some_and(|d| d.has_tag(tag))),
            "no {tag}"
        );
    }
    // cutscene spots inside the map are floor (walkable or furnished), never walls
    for (name, &(x, y)) in &c.cutscenes.spots {
        if y < 0 {
            continue;
        }
        assert!(g.in_bounds(x, y), "spot {name}");
        assert!(
            g.walkable(x, y) || g.object_at(x, y).is_some(),
            "spot {name} ({x},{y}) is a wall"
        );
    }
    assert!(!w.window_light_on(&c), "the window lamp starts off");
}

#[test]
fn astar_finds_paths_without_corner_cutting() {
    let c = content();
    let w = World::new_story(&c, Difficulty::Cozy);
    let g = w.grid();
    let from = g.street_edges()[1];
    let to = (4, 4);
    let path = grid::find_path(g, from, to, true).expect("path from street to hall");
    assert_eq!(*path.last().unwrap(), to);
    let mut prev = from;
    for &t in &path {
        let (dx, dy) = (t.0 - prev.0, t.1 - prev.1);
        assert!(dx.abs() <= 1 && dy.abs() <= 1);
        if dx != 0 && dy != 0 {
            assert!(g.walkable(prev.0 + dx, prev.1) && g.walkable(prev.0, prev.1 + dy));
        }
        assert!(g.guest_ok(t.0, t.1) || t == to);
        prev = t;
    }
    // guests never route through the bar; staff may
    assert!(grid::find_path(g, (4, 4), (24, 12), false).is_some());
    let guest = grid::find_path(g, (4, 4), (20, 9), true).unwrap();
    assert!(guest.iter().all(|t| !g.room(t.0, t.1).staff_only()));
    assert!(grid::find_path(g, (4, 4), (-5, 2), false).is_none());
}

#[test]
fn path_cache_hits() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    let g = w.grid.clone();
    let a = w.paths.get(&g, (2, 2), (20, 8), true);
    let b = w.paths.get(&g, (2, 2), (20, 8), true);
    assert_eq!(a, b);
    assert!(w.paths.hits >= 1);
}

// ---------------------------------------------------------------------------------------------
// Building & undo
// ---------------------------------------------------------------------------------------------

#[test]
fn place_remove_and_undo_furniture() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    let money = w.money();
    let n = w.furniture().len();
    w.apply(&c, place("plant_small", 8, 7)).unwrap();
    assert_eq!(w.furniture().len(), n + 1);
    assert_eq!(w.money(), money - 350);
    assert!(!w.grid().walkable(8, 7));
    assert_eq!(
        w.apply(&c, place("plant_small", 8, 7)),
        Err(CommandError::Occupied)
    );
    assert_eq!(
        w.apply(&c, place("plant_small", 0, 0)),
        Err(CommandError::OutOfBounds)
    );
    w.undo(&c).unwrap();
    assert_eq!(w.furniture().len(), n);
    assert_eq!(w.money(), money);
    assert!(w.grid().walkable(8, 7));
    assert_eq!(w.undo(&c), Err(CommandError::NothingToUndo));
    // remove refunds half, undo charges it back
    let chair = w.furniture().iter().find(|p| p.def == "chair").unwrap().uid;
    w.apply(&c, Command::RemoveFurniture { uid: chair })
        .unwrap();
    assert_eq!(w.money(), money + 225);
    w.undo(&c).unwrap();
    assert_eq!(w.money(), money);
    assert!(w.furniture_by_uid(chair).is_some());
    // move + undo
    let plant = w
        .furniture()
        .iter()
        .find(|p| p.def == "plant_tall")
        .unwrap()
        .clone();
    w.apply(
        &c,
        Command::MoveFurniture {
            uid: plant.uid,
            x: 9,
            y: 9,
            rot: 0,
        },
    )
    .unwrap();
    assert_eq!(w.furniture_by_uid(plant.uid).unwrap().x, 9);
    w.undo(&c).unwrap();
    assert_eq!(w.furniture_by_uid(plant.uid).unwrap().x, plant.x);
}

#[test]
fn placement_never_blocks_the_door() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    let (ex, ey) = w.grid().entrance();
    let mut blocked = false;
    for (x, y) in [(ex, ey - 1), (ex + 1, ey), (ex + 1, ey - 1), (ex, ey)] {
        if let Err(e) = w.apply(&c, place("plant_small", x, y)) {
            assert_eq!(e, CommandError::BlocksPath);
            blocked = true;
        }
    }
    assert!(blocked, "sealing the entrance must be refused");
    assert!(grid::find_path(w.grid(), w.grid().entrance(), (20, 8), true).is_some());
}

#[test]
fn unique_and_locked_furniture() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    assert_eq!(
        w.apply(&c, place("grandma_photo", 9, 9)),
        Err(CommandError::AlreadyPlaced)
    );
    // grandma's jukebox stands in the café from night 1 (unique)
    assert_eq!(
        w.apply(&c, place("jukebox", 9, 9)),
        Err(CommandError::AlreadyPlaced)
    );
    w.debug_set_money(0);
    assert!(matches!(
        w.apply(&c, place("chair", 9, 9)),
        Err(CommandError::NotEnoughMoney { .. })
    ));
}

#[test]
fn duty_and_schedule_edits_undo() {
    let c = content();
    let mut w = sandbox(&c, 2, Difficulty::Cozy);
    let owner = w.staff()[0].id;
    let hire = w.staff()[1].id;
    assert!(w.staff()[0].owner && w.staff()[0].duties.is_empty());
    assert_eq!(
        w.staff()[0].priorities,
        [0; 5],
        "Karolina has no job priorities"
    );
    assert!(matches!(
        w.apply(
            &c,
            Command::SetDuty {
                staff: owner,
                duty: Duty::Dishes,
                on: true,
            },
        ),
        Err(CommandError::Invalid(_))
    ));
    let before = w.staff()[1].duties.clone();
    w.apply(
        &c,
        Command::SetDuty {
            staff: hire,
            duty: before[0],
            on: false,
        },
    )
    .unwrap();
    assert!(w.staff()[1].duties.is_empty());
    w.apply(
        &c,
        Command::SetShift {
            staff: owner,
            weekday: 0,
            shift: Shift::Early,
        },
    )
    .unwrap();
    assert!(w.app().draft_dirty);
    w.undo(&c).unwrap();
    assert_eq!(w.app().draft[&owner][0], Shift::Full);
    assert!(!w.app().draft_dirty);
    w.undo(&c).unwrap();
    assert_eq!(w.staff()[1].duties, before);
    // the old priorities grid still works as a shim onto duties
    w.apply(
        &c,
        Command::SetPriority {
            staff: hire,
            job: Job::Cleaning,
            priority: 2,
        },
    )
    .ok();
    assert_eq!(
        w.staff()[1].priorities[Job::Cleaning.index()],
        u8::from(w.staff()[1].duties.contains(&Duty::Dishes))
    );
}

// ---------------------------------------------------------------------------------------------
// Staff & jobs
// ---------------------------------------------------------------------------------------------

#[test]
fn helpers_only_do_their_duties() {
    let c = content();
    let mut w = sandbox(&c, 3, Difficulty::Cozy);
    let owner = w.staff()[0].id;
    let hire = w.staff()[1].id;
    for d in Duty::ALL {
        w.apply(
            &c,
            Command::SetDuty {
                staff: hire,
                duty: d,
                on: false,
            },
        )
        .unwrap();
    }
    w.apply(
        &c,
        Command::SetDuty {
            staff: hire,
            duty: Duty::Register,
            on: true,
        },
    )
    .unwrap();
    // nobody behind the counter: the helper takes plain orders and nothing else
    let events = run_night(&mut w, &c);
    let takers: Vec<u32> = events
        .iter()
        .filter_map(|e| match e {
            SimEvent::OrderTaken { staff, .. } => Some(*staff),
            _ => None,
        })
        .collect();
    assert!(!takers.is_empty());
    assert!(
        takers.iter().all(|s| *s == hire),
        "only the register helper"
    );
    assert!(
        !events.iter().any(|e| matches!(
            e,
            SimEvent::OrderReady { .. } | SimEvent::OrderServed { .. }
        )),
        "nobody holds the bar, kitchen or serving duty"
    );
    assert!(!events.iter().any(|e| matches!(
        e,
        SimEvent::PlayerOrderTaken { .. } | SimEvent::PlayerServed { .. }
    )));
    let _ = owner;
    // Cozy: the guests waited and forgave — nobody left angry
    let s = w.last_summary().unwrap();
    assert_eq!(s.guests_left_unhappy, 0);
    assert!(s.guests_forgiven > 0);
}

#[test]
fn a_helper_without_duties_does_nothing() {
    let c = content();
    let mut w = sandbox(&c, 4, Difficulty::Cozy);
    let hire = w.staff()[1].id;
    for d in w.staff()[1].duties.clone() {
        w.apply(
            &c,
            Command::SetDuty {
                staff: hire,
                duty: d,
                on: false,
            },
        )
        .unwrap();
    }
    let events = run_night(&mut w, &c);
    assert!(!events.iter().any(|e| matches!(e,
        SimEvent::OrderTaken { staff, .. } | SimEvent::OrderReady { staff, .. }
            | SimEvent::OrderServed { staff, .. } if *staff == hire)));
    assert_eq!(w.last_summary().unwrap().helper_items, 0);
}

#[test]
fn late_shift_arrives_and_is_paid_half() {
    let c = content();
    let mut w = sandbox(&c, 5, Difficulty::Cozy);
    let hire = w.staff()[1].id;
    let wd = w.weekday;
    w.apply(
        &c,
        Command::SetShift {
            staff: hire,
            weekday: wd,
            shift: Shift::Late,
        },
    )
    .unwrap();
    w.apply(&c, Command::PublishSchedule).unwrap();
    w.start_night(&c);
    finish_cutscenes(&mut w, &c);
    let st = w.staff_by_id(hire).unwrap().status;
    assert!(
        matches!(st, StaffStatus::Expected { minute } if minute >= 240) || st == StaffStatus::Sick,
        "{st:?}"
    );
    let events = run_night(&mut w, &c);
    if st != StaffStatus::Sick {
        assert!(
            events
                .iter()
                .any(|e| matches!(e, SimEvent::StaffArrived { staff } if *staff == hire))
        );
    }
    let s = w.last_summary().unwrap();
    let wage = w.staff_by_id(hire).unwrap().wage as i64;
    assert!(s.wages <= wage / 2 + 1, "{} vs {}", s.wages, wage);
}

// ---------------------------------------------------------------------------------------------
// Guests & economy
// ---------------------------------------------------------------------------------------------

#[test]
fn guest_lifecycle_arrive_order_eat_pay_leave() {
    let c = content();
    let mut w = sandbox(&c, 6, Difficulty::Cozy);
    let money = w.money();
    let events = play_night(&mut w, &c);
    let has = |f: &dyn Fn(&SimEvent) -> bool| events.iter().any(f);
    assert!(has(&|e| matches!(e, SimEvent::NightStarted { .. })));
    assert!(has(&|e| matches!(e, SimEvent::GuestArrived { .. })));
    assert!(has(&|e| matches!(e, SimEvent::OrderTaken { .. })));
    assert!(has(&|e| matches!(e, SimEvent::OrderServed { .. })));
    assert!(has(&|e| matches!(e, SimEvent::Payment { .. })));
    assert!(has(&|e| matches!(e, SimEvent::GuestLeft { .. })));
    assert!(has(&|e| matches!(e, SimEvent::NightEnded { .. })));
    assert_eq!(w.phase(), Phase::Day);
    assert!(w.parties().is_empty() && w.guests().is_empty());
    let s = w.last_summary().unwrap().clone();
    assert!(s.guests_served > 0);
    let paid: i64 = events
        .iter()
        .filter_map(|e| match e {
            SimEvent::Payment { amount } => Some(*amount),
            _ => None,
        })
        .sum();
    assert_eq!(paid, s.revenue);
    assert_eq!(
        w.money(),
        money + s.revenue + s.tips + s.other_income
            - s.wages
            - s.supplies
            - s.rent
            - s.other_expenses
            - s.payment,
        "ledger adds up"
    );
}

#[test]
fn story_payment_is_collected_and_missed() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    for _ in 0..6 {
        run_night(&mut w, &c);
    }
    assert_eq!(w.night(), 7);
    w.debug_set_money(50_000);
    let events = run_night(&mut w, &c);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, SimEvent::PaymentMade { amount: 15_000, .. }))
    );
    assert_eq!(w.debt(), c.story.debt - 15_000);
    assert!(super::logic::payments_all_made(&w));
    let mut w = World::new_story(&c, Difficulty::Classic);
    for _ in 0..6 {
        run_night(&mut w, &c);
    }
    w.debug_set_money(100);
    let events = run_night(&mut w, &c);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, SimEvent::PaymentMissed { .. }))
    );
    assert_eq!(w.story().counters.get("missed_payments"), Some(&1));
    assert!(!super::logic::payments_all_made(&w));
}

#[test]
fn trains_pass_on_schedule() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    let events = run_night(&mut w, &c);
    let trains: Vec<&String> = events
        .iter()
        .filter_map(|e| match e {
            SimEvent::TrainPassing { train, .. } => Some(train),
            _ => None,
        })
        .collect();
    assert_eq!(trains.len(), c.trains.trains.len() - 1, "{trains:?}");
    assert!(
        !trains.iter().any(|t| *t == "Rn 333"),
        "Rn 333 only on night 30"
    );
}

// ---------------------------------------------------------------------------------------------
// Storyteller
// ---------------------------------------------------------------------------------------------

#[test]
fn storyteller_paces_quiet_and_chaotic_nights() {
    let c = content();
    let count = |character: &str| -> (f32, u32) {
        let mut w = World::new_sandbox(
            &c,
            SandboxSetup {
                seed: 21,
                character: Some(character.into()),
                ..Default::default()
            },
        );
        let mut total = 0;
        let mut max_bad = 0;
        for _ in 0..8 {
            run_night(&mut w, &c);
            let s = w.last_summary().unwrap();
            total += s
                .events
                .iter()
                .filter(|e| {
                    c.event(e)
                        .is_some_and(|d| d.trigger == crate::content::EventTrigger::Random)
                })
                .count();
            max_bad = max_bad.max(w.narrator.bad_tonight);
        }
        (total as f32 / 8.0, max_bad)
    };
    let (quiet, quiet_bad) = count("quiet");
    let (chaotic, _) = count("chaotic");
    assert!(quiet <= 4.0, "quiet: {quiet} random events/night");
    assert!(chaotic > quiet, "chaotic {chaotic} vs quiet {quiet}");
    assert!(quiet_bad <= 3, "quiet night had {quiet_bad} bad events");
}

#[test]
fn choice_events_wait_and_default() {
    let c = content();
    let mut w = sandbox(&c, 8, Difficulty::Cozy);
    w.start_night(&c);
    finish_cutscenes(&mut w, &c);
    w.drain_events();
    w.debug_fire_event(&c, "station_cat");
    let ev = w.drain_events();
    assert!(
        ev.iter()
            .any(|e| matches!(e, SimEvent::ChoiceNeeded { id, .. } if id == "station_cat"))
    );
    assert_eq!(w.pending_choices().len(), 1);
    let money = w.money();
    w.apply(
        &c,
        Command::AnswerEvent {
            event: "station_cat".into(),
            choice: "feed".into(),
        },
    )
    .unwrap();
    assert_eq!(w.money(), money - 10);
    assert_eq!(w.story().counters.get("cat_fed_nights"), Some(&1));
    assert!(w.pending_choices().is_empty());
    w.debug_fire_event(&c, "drone_lost");
    for _ in 0..(narrator::CHOICE_MINUTES + 2) * TICKS_PER_MINUTE {
        w.tick(&c);
    }
    let ev = w.drain_events();
    assert!(
        ev.iter()
            .any(|e| matches!(e, SimEvent::ChoiceResolved { id, choice }
        if id == "drone_lost" && choice == "return_it"))
    );
}

#[test]
fn blackout_and_later_effects_survive_a_save() {
    let c = content();
    let mut w = sandbox(&c, 9, Difficulty::Cozy);
    w.start_night(&c);
    finish_cutscenes(&mut w, &c);
    w.debug_fire_event(&c, "power_outage");
    assert!(w.blackout());
    assert!(w.drain_events().contains(&SimEvent::Blackout { on: true }));
    for id in w.menu_available(&c) {
        assert!(!c.menu_item(&id).unwrap().needs_power, "{id}");
    }
    for _ in 0..61 * TICKS_PER_MINUTE {
        w.tick(&c);
    }
    assert!(!w.blackout());
    w.debug_fire_event(&c, "health_inspection");
    assert_eq!(w.narrator.later.len(), 1);
    let save = w.save_to_string().unwrap();
    let mut loaded = World::load_from_str(&save).unwrap();
    for _ in 0..61 * TICKS_PER_MINUTE {
        loaded.tick(&c);
    }
    assert!(loaded.narrator.later.is_empty());
    assert!(loaded.drain_events().iter().any(
        |e| matches!(e, SimEvent::Toast { key } if key.starts_with("event.health_inspection"))
    ));
}

// ---------------------------------------------------------------------------------------------
// Story
// ---------------------------------------------------------------------------------------------

#[test]
fn story_opening_cutscene_pauses_the_clock() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    assert!(
        w.drain_events()
            .iter()
            .any(|e| matches!(e, SimEvent::BotMessage { template, .. } if template == "hello"))
    );
    w.start_night(&c);
    assert!(
        w.drain_events()
            .contains(&SimEvent::PlayCutscene("opening".into()))
    );
    assert_eq!(w.playing_cutscene(), Some("opening"));
    let t = w.minute();
    for _ in 0..50 {
        w.tick(&c);
    }
    assert_eq!(w.minute(), t, "clock paused during a cutscene");
    assert_eq!(
        w.apply(&c, Command::SetMusic { on: true }),
        Err(CommandError::CutscenePlaying)
    );
    w.apply(
        &c,
        Command::CutsceneFinished {
            id: "opening".into(),
            choices: vec![],
        },
    )
    .unwrap();
    assert!(w.story().items.contains("cafe_keys"), "Apply step ran");
    for _ in 0..50 {
        w.tick(&c);
    }
    assert!(w.minute() > t);
}

#[test]
fn cutscene_choices_apply_effects_and_flags() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    super::story::queue_cutscene(&mut w, &c, "lori_platform");
    assert_eq!(w.playing_cutscene(), Some("lori_platform"));
    w.apply(
        &c,
        Command::CutsceneFinished {
            id: "lori_platform".into(),
            choices: vec![0],
        },
    )
    .unwrap();
    assert!(w.flag("lori_invited"));
    assert!(w.story().trust.get("lori").copied().unwrap_or(0.0) >= 10.0);
    super::story::queue_cutscene(&mut w, &c, "lori_platform");
    assert_eq!(w.playing_cutscene(), None, "plays once per save");
}

#[test]
fn notes_regulars_and_arcs_progress() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    let mut regulars = Vec::new();
    for _ in 0..6 {
        for e in run_night(&mut w, &c) {
            if let SimEvent::RegularArrived { regular, .. } = e {
                regulars.push(regular);
            }
        }
    }
    assert!(regulars.iter().any(|r| r == "mirek"), "{regulars:?}");
    assert!(regulars.iter().any(|r| r == "hana"), "{regulars:?}");
    assert!(w.notes_found().iter().any(|n| n == "note_01"));
    assert!(w.story().regulars["mirek"].visits.len() >= 3);
}

#[test]
fn story_reaches_exactly_one_ending() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    let mut endings = Vec::new();
    for _ in 0..31 {
        if w.is_closed() {
            break;
        }
        for e in run_night(&mut w, &c) {
            if let SimEvent::EndingReached(id) = e {
                endings.push(id);
            }
        }
    }
    assert_eq!(endings.len(), 1, "{endings:?}");
    assert!(c.story.endings.iter().any(|e| e.id == endings[0]));
    assert_eq!(w.ending(), Some(endings[0].as_str()));
    assert_eq!(w.night(), 30);
    assert!(w.apply(&c, Command::StartNight).is_err());
}

#[test]
fn talk_and_ambient_lines_for_regulars() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    w.start_night(&c);
    finish_cutscenes(&mut w, &c);
    let mut mirek_guest = None;
    for _ in 0..90 * TICKS_PER_MINUTE {
        w.tick(&c);
        finish_cutscenes(&mut w, &c);
        if let Some(pid) = w.story().regulars.get("mirek").and_then(|r| r.party)
            && let Some(p) = w.parties().iter().find(|p| p.id == pid)
        {
            mirek_guest = p.members.first().copied();
            break;
        }
    }
    let g = mirek_guest.expect("mirek visits on night 1");
    let r = c.regular("mirek").unwrap();
    if !r.ambient_lines.is_empty() {
        let line = w.ambient_line_for(&c, g).expect("a line");
        assert!(r.ambient_lines.iter().any(|l| l.key == line));
    }
    if let Some(other) = w.guests().iter().find(|x| x.id != g) {
        assert_eq!(w.talk_available(&c, other.id), None);
    }
}

// ---------------------------------------------------------------------------------------------
// Shiftella app
// ---------------------------------------------------------------------------------------------

#[test]
fn shiftella_app_schedule_requests_checklists_payroll() {
    let c = content();
    let mut w = sandbox(&c, 12, Difficulty::Cozy);
    let ids: Vec<u32> = w.candidates().iter().map(|c| c.id).collect();
    for id in ids.into_iter().take(2) {
        w.apply(&c, Command::Hire { candidate: id }).unwrap();
    }
    w.drain_events();
    w.apply(&c, Command::PublishSchedule).unwrap();
    assert!(
        w.drain_events()
            .iter()
            .any(|e| matches!(e, SimEvent::BotMessage { template, key, .. }
        if template == "schedule_published" && key == "app.bot.schedule_published"))
    );
    // swap request via the bot, approved with its button
    super::app::bot(&mut w, &c, "swap_request", &EffectCtx::default());
    let msg = w.app().inbox.last().unwrap().clone();
    let swap = w.app().swaps.last().unwrap().clone();
    let wd = swap.weekday;
    let to_before = w.app().published[&swap.to][wd as usize];
    w.apply(
        &c,
        Command::SetShift {
            staff: swap.from,
            weekday: wd,
            shift: Shift::Early,
        },
    )
    .unwrap();
    w.apply(&c, Command::PublishSchedule).unwrap();
    w.apply(
        &c,
        Command::BotAction {
            msg: msg.id,
            action: "approve".into(),
        },
    )
    .unwrap();
    assert_eq!(w.app().published[&swap.to][wd as usize], Shift::Early);
    assert_eq!(w.app().published[&swap.from][wd as usize], to_before);
    assert!(
        w.app().manual_rozpis,
        "an approved swap is a hand-made change"
    );
    for t in &c.app.bot {
        assert!(t.actions.iter().all(|a| !a.id.contains("clock")));
    }
    // checklists: staff tick job items; the player lights the window
    w.start_night(&c);
    finish_cutscenes(&mut w, &c);
    // at night the window light is Karolina's act: she walks to the lamp
    w.apply(
        &c,
        Command::TickChecklist {
            id: "light_window".into(),
        },
    )
    .unwrap();
    assert!(!w.window_light_on(&c));
    for _ in 0..600 {
        w.tick(&c);
        if w.window_light_on(&c) {
            break;
        }
    }
    assert!(w.window_light_on(&c));
    let mut bot = bot::PlayerBot::new(bot::BotConfig {
        hire_nights: Vec::new(),
        ..Default::default()
    });
    bot.run_night(&mut w, &c);
    let done = w.last_summary().unwrap().checklist_done;
    assert!(done >= 4, "checklist done {done}");
    while w.night() <= 7 {
        bot.run_night(&mut w, &c);
    }
    let week = w.app().payroll.last().expect("payroll week");
    assert_eq!(week.week, 1);
    assert!(week.rows.len() >= 3 && week.total > 0);
    assert!(week.rows.iter().all(|r| r.personal_number > 0));
}

#[test]
fn time_off_is_respected() {
    let c = content();
    let mut w = sandbox(&c, 13, Difficulty::Cozy);
    let hire = w.staff()[1].id;
    let wd = w.weekday;
    w.apply(
        &c,
        Command::RequestTimeOff {
            staff: hire,
            kind: LeaveKind::Vacation,
            weekday: (wd + 1) % 7,
        },
    )
    .unwrap();
    let req = w.app().time_off.last().unwrap().id;
    w.apply(
        &c,
        Command::AnswerTimeOff {
            id: req,
            approve: true,
        },
    )
    .unwrap();
    run_night(&mut w, &c);
    w.start_night(&c);
    assert_eq!(w.staff_by_id(hire).unwrap().status, StaffStatus::OffDuty);
}

// ---------------------------------------------------------------------------------------------
// Saves & determinism
// ---------------------------------------------------------------------------------------------

#[test]
fn save_round_trip_mid_night_continues_identically() {
    let c = content();
    let mut w = sandbox(&c, 14, Difficulty::Cozy);
    w.start_night(&c);
    finish_cutscenes(&mut w, &c);
    for _ in 0..1500 {
        w.tick(&c);
    }
    let text = w.save_to_string().unwrap();
    let mut a = World::load_from_str(&text).unwrap();
    let mut b = World::load_from_str(&text).unwrap();
    for _ in 0..1500 {
        a.tick(&c);
        b.tick(&c);
        w.tick(&c);
    }
    let sa = a.save_to_string().unwrap();
    assert_eq!(sa, b.save_to_string().unwrap());
    assert_eq!(
        sa,
        w.save_to_string().unwrap(),
        "loaded world matches the original"
    );
    assert!(World::load_from_str("(version: 999, world: ())").is_err());
}

#[test]
fn same_seed_same_story() {
    let c = content();
    let run = |seed: u64| -> Vec<NightSummary> {
        let mut w = sandbox(&c, seed, Difficulty::Cozy);
        for _ in 0..2 {
            run_night(&mut w, &c);
            let ids: Vec<u32> = w.candidates().iter().map(|c| c.id).take(1).collect();
            for id in ids {
                w.apply(&c, Command::Hire { candidate: id }).unwrap();
            }
        }
        w.summaries().to_vec()
    };
    assert_eq!(run(77), run(77));
    assert_ne!(run(77), run(78));
}

// ---------------------------------------------------------------------------------------------
// Integration: 3 full nights headless
// ---------------------------------------------------------------------------------------------

fn three_nights(mut w: World, c: &Content) -> Vec<NightSummary> {
    let mut out = Vec::new();
    let mut bot = bot::PlayerBot::new(bot::BotConfig {
        hire_nights: Vec::new(),
        ..Default::default()
    });
    for _ in 0..3 {
        let ids: Vec<u32> = w.candidates().iter().map(|c| c.id).take(1).collect();
        if w.staff().len() < 3 {
            for id in ids {
                w.apply(c, Command::Hire { candidate: id }).unwrap();
            }
        }
        bot.day(&mut w, c);
        finish_cutscenes(&mut w, c);
        w.start_night(c);
        let mut stuck = 0u32;
        while w.phase() == Phase::Night {
            finish_cutscenes(&mut w, c);
            bot.step(&mut w, c);
            w.tick(c);
            // nobody stands still on a path for more than ~20 game minutes
            for g in &w.guests {
                assert!(g.stuck_ticks <= 200, "guest stuck {} ticks", g.stuck_ticks);
            }
            for s in &w.staff {
                stuck = stuck.max(s.stuck_ticks);
            }
            assert!(w.money().abs() < 10_000_000);
        }
        assert!(stuck <= 200, "staff stuck {stuck} ticks");
        out.push(w.last_summary().unwrap().clone());
    }
    out
}

#[test]
fn three_nights_cozy_story_and_sandbox() {
    let c = content();
    for w in [
        World::new_story(&c, Difficulty::Cozy),
        sandbox(&c, 31, Difficulty::Cozy),
    ] {
        let nights = three_nights(w, &c);
        for s in &nights {
            assert!(s.guests_served > 0, "{s:?}");
            assert!(s.avg_satisfaction.is_finite() && s.reputation_end.is_finite());
            assert!(
                s.exhausted_minutes < 60,
                "staff exhausted {}",
                s.exhausted_minutes
            );
            assert_eq!(s.stuck_resets, 0, "stuck resets");
        }
        let served: u32 = nights.iter().map(|s| s.guests_served).sum();
        let unhappy: u32 = nights.iter().map(|s| s.guests_left_unhappy).sum();
        assert!(served >= 40, "served {served}");
        // Cozy: nobody leaves angry — they forgive
        assert_eq!(unhappy, 0, "served {served} vs unhappy {unhappy}");
    }
}

#[test]
fn three_nights_classic_story_and_sandbox() {
    let c = content();
    for w in [
        World::new_story(&c, Difficulty::Classic),
        sandbox(&c, 32, Difficulty::Classic),
    ] {
        for s in three_nights(w, &c) {
            assert!(s.guests_served > 0);
            assert!(s.money_end > -50_000);
            assert_eq!(s.stuck_resets, 0);
        }
    }
}

#[test]
fn sixty_agents_tick_fast() {
    let c = content();
    let mut w = sandbox(&c, 40, Difficulty::Cozy);
    let ids: Vec<u32> = w.candidates().iter().map(|c| c.id).collect();
    for id in ids {
        w.apply(&c, Command::Hire { candidate: id }).unwrap();
    }
    w.start_night(&c);
    finish_cutscenes(&mut w, &c);
    w.debug_spawn_guests(&c, "students", 60, 5);
    let mut peak = 0;
    let t0 = std::time::Instant::now();
    for _ in 0..600 {
        w.tick(&c);
        peak = peak.max(w.guests().len() + w.staff().len());
    }
    let per_tick = t0.elapsed().as_secs_f64() / 600.0;
    assert!(peak >= 40, "peak agents {peak}");
    // generous bound for debug builds (release: see `sim_headless --stress`)
    assert!(per_tick < 0.02, "{:.3} ms per tick", per_tick * 1e3);
}
