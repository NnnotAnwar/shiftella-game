# Shiftella — simulation (`src/sim`)

Pure, deterministic game logic behind one facade, `sim::World`. No rendering, no macroquad calls,
no new crates (own SplitMix64 RNG). Commands in, events out. Content semantics: `docs/CONTENT.md`.

```rust
use shiftella_game::{content::Content, sim::*};
let content = Content::load("assets/data")?;
let mut world = World::new_story(&content, Difficulty::Cozy);          // fixed seed
// or World::new_sandbox(&content, SandboxSetup { seed, difficulty, ..Default::default() })
```

## 1. Frame loop (integration)

```rust
// accumulate real time × speed (1/2/3); one tick = 0.1 s at ×1 (10 ticks = 1 game minute)
if !paused {
    acc += dt * speed;
    while acc >= 0.1 { world.tick(&content); acc -= 0.1; }
}
for e in world.drain_events() { audio.on(&e); ui.on(&e); cutscenes.on(&e); }
// draw from read-only queries (never mutate the world from render)
```

* `tick` is a no-op while `world.is_paused()`: day phase, a cutscene playing, or the game over
  (`is_closed()` after an ending / bankruptcy).
* `SimEvent::ClockStop { seconds }` (3:33): stop calling `tick` (and animations, except
  `SeesUncanny` staff if you like) for that many **real** seconds.
* `SimEvent::PlayCutscene(id)`: the sim pauses its clock and waits. Play it, then send
  `Command::CutsceneFinished { id, choices }` — `choices` = the chosen option index (into the
  **full** `options` list) of each `Choice` step in the order shown (skipped choice → leave it out).
  The sim replays `Apply` / `Choice` effects / `ShowNote` / `Call` itself, so the player must not
  apply effects. For `If` steps during playback use `world.eval_conditions(&content, &conds)`
  (read-only; clones the world — fine per step, not per frame). Queued cutscenes start one after
  another (each emits its own `PlayCutscene`).
* Day phase: planning (hire, build, schedule, prices). `world.start_night(&content)` (or
  `Command::StartNight`) → 22:00. At 06:00 the sim emits `NightEnded { summary }`, then (story)
  `NightEnd` cutscenes / payment, the ending check, and `DayStarted { night }` for the next night.
* Headless: `world.run_night_headless(&content)` runs a whole night (auto-finishes cutscenes,
  choices time out to their defaults).

Interpolation: agents move up to ~0.5 tile per tick. For smooth 60 fps keep the previous
`AgentView.feet` per agent and lerp by the tick accumulator fraction.

## 2. Commands (`world.apply(&content, Command) -> Result<(), CommandError>`)

| command | notes |
|---|---|
| `PlaceFurniture { def, x, y, rot }` | footprint, layer (rugs/stage/pendant lamps share tiles), money, unlocks, `unique`, never blocks the door / any piece (`BlocksPath`). Floors (`floor_*`) paint a tile. |
| `MoveFurniture`, `RotateFurniture`, `RemoveFurniture` | not while in use (seat taken, station busy) → `InUse`; remove refunds 50 % |
| `SetFloor { x, y, floor }`, `SetFurnitureOn { uid, on }`, `SetMusic { on }` | lamps / jukebox / radio |
| `Hire { candidate }`, `Fire { staff }` | candidates: `world.candidates()` (story persons from `hire_from_night`; sandbox: 3 generated, refreshed every 3 nights). The owner (`Staff::owner`, cast `player`) can't be fired. |
| `SetPriority { staff, job, priority }` | 0 off, 1 highest … 4. Nobody on shift with a job enabled → everyone helps at 4. |
| `SetShift { staff, weekday, shift }` → edits the **draft**; `PublishSchedule` | `Shift::{Off, Full 22–06, Early 22–02, Late 02–06}`; the night uses `published`. Unpublished edits at night start: crew mood −2. |
| `AnswerSwap`, `AnswerTimeOff`, `RequestTimeOff { staff, kind, weekday }` | dovolená (`Vacation`, paid) / náhradní volno (`Toil`, from >40 h weeks) |
| `BotAction { msg, action }` | buttons of `@shiftella_bot` messages (`app.bot.<t>.action.<a>`); approve/decline link to requests. Never clock-in/out. |
| `AnswerEvent { event, choice }` | answers a `ChoiceNeeded` toast (works while paused); timeout = `default_choice` after 20 game min |
| `CutsceneFinished { id, choices }`, `Talk { guest }` | see §1 / §4 |
| `BuySupplies { amount }`, `SetAutoRestock { target }`, `SetPrice { item, price }`, `SetMenuItem { item, on }`, `RenameCafe { name }` | |
| `TickChecklist { id }` (night), `PostAnnouncement { id, args }`, `WriteHandover { text }` | jobless checklist items (`light_window`, `handover_note`, `light_off_at_six`) are the player's |

`world.undo(&content)` reverts the last reversible command (furniture place/move/rotate/remove,
floors, lamps, priorities, draft shifts, prices, menu). The stack clears at phase changes.

## 3. Queries (read-only)

* Time: `phase()`, `night()`, `minute()` (since 22:00), `tick_count()`, `clock()` / `clock_label()`,
  `night_progress()` (for `render::ambient_for`), `weekday()`, `weather()`, `blackout()`, `music_on()`.
* Map: `grid()` → `Grid { width: 30, height: 18 }` — rows 0–15 = `map::CAFE_LAYOUT` frame (cutscene
  spots match), rows 16–17 = street/sidewalk in front of the door. `tile(x, y) → TileInfo { kind:
  Floor|Wall|Window|Door|Street, floor: Floor (id(): floor_wood|floor_tile|floor_carpet|pavement), room:
  Hall|Bar|Kitchen|Outside|None }`, `walkable`, `object_at`. `bucket_tiles()` (roof leak).
* Furniture: `furniture()` (`Placed { uid, def, x, y, rot, on, dirt, dishes, wear, seat_by, … }`),
  `furniture_views(&content)` → `FurnitureView { uid, active, dirty, facing, tile, size }` ≈
  `render::FurnitureState` (`tile × TILE` = `draw_furniture` origin; chairs face their table).
  `window_light_on(&content)`, `buildable(&content)`.
* People: `staff_views()` / `guest_views()` → `AgentView { agent, look: LookRef { seed, staff, preset,
  source }, feet (world px), facing, pose, emote, seat_tile, holding }`. `Facing`/`Pose`/`SimEmote`
  mirror the render enums 1:1. Look: `preset` → `PersonLook::preset(id)` (owner `player`, regulars
  by id / `look_preset`), else `PersonLook::random(seed, staff)`. Seated guests: `seat_tile` for
  `render::sit_anchor`.
* Crew details: `staff()` (`Staff { name, archetype, owner, traits, skills[5], wage, priorities[5],
  energy, mood, hunger, status, task, shift, hours_week, toil_hours, … }`), `relationship(a, b)`.
* Guests: `parties()` (`Party { archetype, regular, state, patience, patience_max, wishes, satisfaction, … }`),
  `guests()`, `order_items()`, `queue_len()`, `guests_inside()`.
  Regulars: `ambient_line_for(&content, guest)` (hover bubble key), `talk_available(&content, guest)`.
* Money: `money()`, `debt()`, `reputation()`, `supplies()`, `rent()`, `price_of`, `menu_available`,
  `menu_unlocked`, `tonight()` (running ledger), `last_summary()` / `summaries()` (`NightSummary`).
* Story: `story()` (flags, vars, counters, items, trust, arcs, played cutscenes), `flag(f)`,
  `notes_found()`, `ending()`, `ending_variant()`, `playing_cutscene()`, `eval_conditions`.
* App: `app()` → `AppState { draft, published, draft_dirty, week, swaps, time_off, handovers,
  announcements, checklist, inbox, payroll }`; `sim::digest_lines(&summary)` → `(app.digest.<id>, args)`.
* Narrator: `pending_choices()`, `tension()`, `event_log()` (`LogEntry { night, minute, key, args }`).

## 4. Events out (`SimEvent`)

Audio/UI: `GuestArrived`, `GuestTurnedAway`, `GuestLeft`, `OrderTaken`, `OrderReady`, `OrderServed`,
`Payment`, `TipReceived`, `DishBroken`, `StaffLate` (+ bot `late_alert`), `StaffArrived`, `StaffLeft`,
`StaffSick`, `StaffWalkedOut`, `StaffBreak`, `StaffHired/Fired`, `TrainAnnounced` (story beat),
`TrainPassing { train, station }` (timetable; `Rn 333` only when `announce_if` holds), `Blackout { on }`,
`WeatherChanged`, `MusicChanged`, `EventStarted { id }`, `ChoiceNeeded { id, choices, expires_in }`,
`ChoiceResolved`, `EventHint` (Foresight), `Toast { key }`, `Hint { key }`, `BotMessage { msg, template,
key: "app.bot.<t>", args }`, `Speech { speaker, agent, key }` (beat lines, arc lines on arrival),
`PlayCutscene`, `EndingReached`, `NoteFound`, `ItemGained/Lost`, `FlagSet`, `RegularArrived`, `ArcAdvanced`,
`ClockStop`, `PaymentMade/Missed`, `CafeClosed`, `SchedulePublished`, `ChecklistTicked`,
`MenuUnlocked`, `FurnitureUnlocked`, `FurniturePlaced/Removed`, `DayStarted`, `NightStarted`, `NightEnded`.

## 5. How it works

* **Work loop:** idle staff pick work by priority level (1→4), then rank & distance: take orders
  (register queue front / seated parties), prepare items at a free station of the item's tag
  (one user per station), serve (tray: up to 4 items of a party), clear tables → sink, wash, mop
  hall dirt, clear buckets, opening/closing checklist items of their job. Speed = `0.5 + skill×0.1`
  × trait `Speed` × event `WorkSpeed` × energy/mood × station slow-downs.
* **Guests:** arrivals per minute from `hour_weights × weekday_mult` of the town's pools × reputation ×
  weather × `GuestRate`; trains add passengers; rushes from events. Seats are reserved on arrival
  (no seat → they walk on). Leader queues at the register, or the party sits and orders at the table
  when the line is long. Patience runs only once inside; food waits get 2× patience. Satisfaction:
  wait times, item `mood_bonus`, preparer quality, server traits, seat comfort, beauty, dirt, music.
  Pay = price × spend × `income_mul`; tips by satisfaction; review → reputation.
* **Needs:** energy/hunger drain (× `need_decay_mul`, trait `NeedRate`), mood drifts to ~62 plus trait
  situations, aura; breaks (≤ ⅓ of the crew at once) restore energy/hunger. Relationships per hour
  (base +0.4, trait `Relationship`, `irritated_by` −2) feed quarrel/romance events.
* **Narrator:** per minute `base_rate × curve × act_mult` (story: `calm` character + `story_acts`;
  Classic story uses `changeable`), weighted pick with cooldowns, tension budget, breathing room,
  `max_bad_in_row`, staff-event cap, witching hour, good share, poverty help, calm before payment,
  warm event guarantee. Auto events with conditions are checked each minute; `broken_dishes` comes
  from mishaps. `Later` effects survive saves (stored as content paths).
* **Story:** beats before 22:00 fire at day start; others at their minute; 06:00 beats (payments)
  at night end. Regular visits by `visits_story_night` / `sandbox_visit_rule`, weather, `absent_if`;
  one arc stage per night while present. Notes by `found_when` (every 5 min + night end). Endings
  checked after night 30 or a Classic closure; `EndingReached(id)` + its cutscene, then the world is closed.
* **Economy:** night start auto-restock (Classic: max 1 500 Kč on credit), ingredients consumed per
  order, wages by minutes worked (vacation paid), story utilities 350 Kč/night, sandbox rent (+growth).
  Sandbox Classic: `bankrupt_after` nights in the red → `CafeClosed`.

## 6. Saves

`world.save_to_string()` → RON with `version` (`SAVE_VERSION = 1`); `World::load_from_str(&text)`.
Caches (walk masks, paths) rebuild on the next tick (or call `rebuild_caches(&content)` before queries).
Same seed + same commands ⇒ identical saves (tested).

## 7. Balance (headless)

`cargo run --release --example sim_headless -- [nights] [story|sandbox] [cozy|classic] [seed]`
(`SIM_STRESS=1` adds 60 guests nightly). `sim_trace` prints staff/party states for debugging.
Knobs: `guests::{BASE_PARTIES_PER_HOUR, FOOD_PATIENCE, SECOND_ITEM_CHANCE, THIRD_ITEM_CHANCE,
REVIEW_WEIGHT, story_ramp}`, `staff::{STAFF_SPEED, TRAY}`, `narrator::CHOICE_MINUTES`,
`economy::CLASSIC_OVERDRAFT`.

## 8. Types defined in sim (not in content)

`Difficulty { Cozy, Classic }`, `Mode`, `Phase`, `SandboxSetup`, `Town`, `Shift`, `LeaveKind`,
`RequestStatus`, `Facing`/`Pose`/`SimEmote` (render mirrors), `LookRef`, `AgentView`, `FurnitureView`,
`NightSummary`, `Task`, `StaffStatus`, `PartyState`, `PayrollWeek`, `Handover`, `BotMsg`;
`sim::grid::DEFAULT_LAYOUT` (CAFE_LAYOUT + register/fridge/sink/lamp/decor letters + street) and
`world::STORY_INSTITUTIONS` (the story station: taxi rank, hospital, college).
