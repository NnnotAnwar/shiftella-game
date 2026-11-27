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
// one tick = world.real_seconds_per_tick(&content) at ×1 (handson.ron: 0.13 s → a night is
// 10.4 real minutes of sim; sim::REAL_SECONDS_PER_TICK_X1 is the default). Ask for it every
// frame: during the preparation half hour it is × `prep_tick_scale` (0.7). While an overlay is open
// (order card, mini-game, book, moment card, phone app) multiply dt by
// world.overlay_scale(&content) (Cozy 0.1, Classic 0.35) and clamp the speed to ×1.
if !paused {
    acc += dt * speed * overlay;
    while acc >= tick_s { world.tick(&content); acc -= tick_s; }
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
  `Command::StartNight`) → **21:30, the preparation half hour** (see below), then 22:00.
  At 06:00 the sim emits `NightEnded { summary }`, then (story) `NightEnd` cutscenes / payment, the ending check, and `DayStarted { night }` for the next night.
* **The preparation half hour (21:30–22:00).** The night begins with the doors closed:
  `in_prep()` is true, `clock()` / `clock_label()` run 21:30 → 21:59, `minutes_since_open()`
  is −30 … −1, while the night clock (`minute()`, `tick_count()`, `night_progress()`) stands at
  0 / 22:00. `PREP_TICKS` (300) prep ticks run only the quiet systems: Karolina's queue (every
  checklist item, records, specials), the crew on tonight's rozpis walking in from 21:38
  (`PREP_CREW_ARRIVES`, a couple of minutes apart; their opening `Either` items come first
  while nobody waits), tutorial `At("21:xx")` steps, trains timetabled before 22:00 passing by
  outside (`Nex 9047` at 21:47; nobody gets off). **No** guests, rushes, regulars, moments,
  story beats, narrator events, needs or wages: the shift (and the timesheet) still starts
  at 22:00, so everything keyed to 22:00 stays there. The last prep tick (or
  `Command::OpenDoors`, "Open the doors now", only in prep) emits
  `SimEvent::DoorsOpened { early_minutes }`; the next tick is 22:00's first minute. The opening
  ritual deadline is still 22:30. `evening_progress()` is 0–1 over 21:30 → 06:00 (the HUD bar),
  `prep_progress()` 0–1 over the half hour. Tests: `World::skip_prep` (cfg(test)) plays it
  through.
* Headless: `world.run_night_headless(&content)` runs a whole night with **nobody behind the
  counter** (Karolina steps back: helpers do their duties, Cozy guests forgive). To play,
  use `sim::bot::PlayerBot::run_night` (§9).

Interpolation: agents move up to ~0.5 tile per tick. For smooth 60 fps keep the previous
`AgentView.feet` per agent and lerp by the tick accumulator fraction.

## 2. Commands (`world.apply(&content, Command) -> Result<(), CommandError>`)

| command | notes |
|---|---|
| `PlaceFurniture { def, x, y, rot }` | footprint, layer (rugs/stage/pendant lamps share tiles), money, unlocks, `unique`, never blocks the door / any piece (`BlocksPath`). Floors (`floor_*`) paint a tile. |
| `MoveFurniture`, `RotateFurniture`, `RemoveFurniture` | not while in use (seat taken, station busy, guests lined up at a register) → `InUse`; a solid piece never lands on someone standing there → `Occupied`; remove refunds 50 % |
| `MoveGroup { moves: Vec<PieceMove { uid, x, y, rot }> }` | build mode's drag: all pieces are lifted, then put down (a table and its chairs may swap tiles); atomic — any `InUse` / `Occupied` / `BlocksPath` changes nothing; free; **one** undo step (`UndoOp::MoveGroupBack`) |
| `SetFloor { x, y, floor }`, `SetFurnitureOn { uid, on }`, `SetMusic { on }` | lamps / jukebox / radio |
| `Hire { candidate }`, `Fire { staff }` | candidates: `world.candidates()` (story persons from `hire_from_night`; sandbox: 3 generated, refreshed every 3 nights). The owner (`Staff::owner`, cast `player`) can't be fired. |
| `SendHome { staff }` | night, a helper on the floor: their shift ends now (`leave_at` = now, they walk out; wages follow the minutes worked); energy < 35 → mood +4. `WrongPhase` by day, `Invalid` for Karolina / not on the floor / the last 5 minutes |
| `SetDuty { staff, duty, on }` | delegate one of the six duties (§9.3); capacity-checked → `Capacity`; undoable. Karolina has none (`Invalid`). |
| `SetPriority { staff, job, priority }` | legacy shim onto duties: > 0 = the job's duty on (cleaning → dishes), 0 = off. The "everyone helps" fallback is gone. |
| `SetShift { staff, weekday, shift }` → edits the **draft**; `PublishSchedule` | `Shift::{Off, Full 22–06, Early 22–02, Late 02–06}`; the night uses `published`. Unpublished edits at night start: crew mood −2. |
| `AnswerSwap`, `AnswerTimeOff`, `RequestTimeOff { staff, kind, weekday }` | dovolená (`Vacation`, paid) / náhradní volno (`Toil`, from >40 h weeks) |
| `BotAction { msg, action }` | buttons of `@shiftella_bot` messages (`app.bot.<t>.action.<a>`); approve/decline link to requests. Never clock-in/out. |
| `AnswerEvent { event, choice }` | answers a `ChoiceNeeded` toast (works while paused); timeout = `default_choice` after 20 game min |
| `CutsceneFinished { id, choices }`, `Talk { guest }` | see §1 / §4 |
| `BuySupplies { amount }`, `SetAutoRestock { target }`, `SetPrice { item, price }`, `SetMenuItem { item, on }`, `RenameCafe { name }` | |
| `TickChecklist { id }` (night), `PostAnnouncement { id, args }`, `WriteHandover { text }` | at night ticking an item is Karolina's act (she walks to it); jobless items (`light_window`, `handover_note`, `light_off_at_six`) are hers alone |
| `OpenDoors` | "Open the doors now": ends the preparation half hour early (only before 22:00 → `WrongPhase` otherwise); emits `DoorsOpened`, the clock jumps to 22:00 |
| Karolina: `PlayerTakeOrder`, `PlayerBrew`, `PlayerUsual`, `PlayerHandOff`, `PlayerCancel`, `PlayerQueue`, `PlayerUnqueue`, `PlayerWashProgress`, `PlayerGreet` | §9.1 |
| `PlayRecord { id, repeat }`, `RecordFinished { id }`, `SetAssistWishes { on }` | §9.6; accessibility hand-off |
| `UseEquipment { uid, action }`, `EquipmentHold { seconds }` | v0.2 appliances (§11): a queued chore at a piece; holding speeds a hold chore |
| `AnswerStory { guest, answer }`, `StormBlankets`, `StormSoup` | v0.2 H: a stranger's little story, the storm night's chores (§12) |

`world.undo(&content)` reverts the last reversible command (furniture place/move/rotate/remove,
floors, lamps, duties, draft shifts, prices, menu). The stack clears at phase changes.

## 3. Queries (read-only)

* Time: `phase()`, `night()`, `minute()` (since 22:00; 0 through the preparation), `tick_count()`,
  `clock()` / `clock_label()` (21:00 in the day, 21:30→ in the preparation), `minutes_since_open()`
  (signed: −30 at 21:30), `in_prep()`, `doors_open()`, `prep_minute()`, `prep_progress()`,
  `evening_progress()`, `night_progress()` (for `render::ambient_for`), `weekday()`, `weather()`, `blackout()`, `music_on()`.
* Map: `grid()` → `Grid { width: 30, height: 18 }` — rows 0–15 = `map::CAFE_LAYOUT` frame (cutscene
  spots match), rows 16–17 = street/sidewalk in front of the door. `tile(x, y) → TileInfo { kind:
  Floor|Wall|Window|Door|Street, floor: Floor (id(): floor_wood|floor_tile|floor_carpet|pavement), room:
  Hall|Bar|Kitchen|Outside|None }`, `walkable`, `object_at`. `bucket_tiles()` (roof leak).
* Furniture: `furniture()` (`Placed { uid, def, x, y, rot, on, dirt, dishes, wear, seat_by, … }`),
  `furniture_views(&content)` → `FurnitureView { uid, active, dirty, facing, tile, size }` ≈
  `render::FurnitureState` (`tile × TILE` = `draw_furniture` origin; chairs face their table).
  `window_light_on(&content)`, `buildable(&content)`.
* Build mode (`sim/build_ux.rs`): `move_group(&c, uid)` (a table + the seat pieces whose
  every seat belongs to it; anything else → just itself), `group_moves(&c, uid, to, turns)`
  (the group's `PieceMove`s when the first piece's top-left goes to `to`, turned `turns`
  quarter turns around it: offsets `(dx, dy) → (−dy, dx)` per turn), `can_move_pieces(&c,
  &moves)` (dry run), `in_use_reason(uid) -> Option<InUseReason { Seated, Busy }>`,
  `coziness_preview(&c, def, x, y, rot) -> f32` and `coziness_preview_moves(&c, &moves)` —
  the **change** of the 0–100 coziness value if the piece stood there (pure; on a copy;
  floors → 0). `build_wishes(&c) -> Vec<BuildWish { who, def, tag, reason_key }>`
  (`sim/build_wishes.rs`; package A's stub lists Filip → `power_socket` from night 7,
  Lori → `stage` once `lori_invited`, Hana → `armchair` + `floor_lamp` until her corner
  choice; package C replaces the body).
* People: `staff_views()` / `guest_views()` → `AgentView { agent, look: LookRef { seed, staff, preset,
  source }, feet (world px), facing, pose, emote, seat_tile, holding }`. `Facing`/`Pose`/`SimEmote`
  mirror the render enums 1:1. Look: `preset` → `PersonLook::preset(id)` (owner `player`, regulars
  by id / `look_preset`), else `PersonLook::random(seed, staff)`. Seated guests: `seat_tile` for
  `render::sit_anchor`.
* Crew details: `staff()` (`Staff { name, archetype, owner, traits, skills[5], wage, priorities[5],
  energy, mood, hunger, status, task, shift, hours_week, toil_hours, … }`), `relationship(a, b)`.
* The character card (`sim::crew`, design/v0.2.md §D): `activity(id)` / `next_activity(&c, id)`
  → `Activity` (`id()` → `crew.do.<id>`: the task with its item / `TaskFor` guest, the status,
  `Idle { why: NoDuties | NothingToDo | BeforeDoors | NothingQueued }`, predictions `BreakAt`,
  `BreakSoon`, `HomeAt`; Karolina by day `Planning`), `karolina_plan()` (her current act + queue
  in words), `shift_tonight(id)` → `ShiftTonight { shift, from, to, leave }` (late arrival and
  early leave included; by day the published rozpis of the coming night),
  `feelings(&c, id)` → `Feelings { energy, mood }` (`Feeling::id()` → `crew.why.<id>`, most
  relevant first: a break, exhaustion, last night's shift, a heavy week, hours on shift, trait
  situations; broken prefs on the published week, a happy week, declined requests, a friend /
  sweetheart / friction on shift, trait moods), `karolina_todo_parts(&c)` → `TodoParts` (the
  same count as `karolina_todo`, thing by thing; a test keeps them equal),
  `duties_left_for_karolina()` (duties no helper holds tonight; by day: no helper on the
  coming night's rozpis).
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
* Appliances (§11): `equipment()` (`EquipState`), `equipment_kind(&c, uid)`,
  `equipment_action(&c, uid)` (what a click does now), `equipment_views(&c)` (`EquipView`).

## 4. Events out (`SimEvent`)

Audio/UI: `GuestArrived`, `GuestTurnedAway`, `GuestLeft`, `OrderTaken`, `OrderReady`, `OrderServed`,
`Payment`, `TipReceived`, `DishBroken`, `StaffLate` (+ bot `late_alert`), `StaffArrived`, `StaffLeft`,
`StaffSick`, `StaffWalkedOut`, `StaffBreak`, `StaffHired/Fired`, `TrainAnnounced` (story beat),
`TrainPassing { train, station }` (timetable; `Rn 333` only when `announce_if` holds), `Blackout { on }`,
`WeatherChanged`, `MusicChanged`, `EventStarted { id }`, `ChoiceNeeded { id, choices, expires_in }`,
`ChoiceResolved`, `EventHint` (Foresight), `Toast { key }`, `Hint { key }`, `BotMessage { msg, template,
key: "app.bot.<t>", args }`, `Speech { speaker, agent, key }` (beat lines, arc lines on arrival),
`PlayCutscene`, `EndingReached`, `NoteFound`, `ItemGained/Lost`, `FlagSet`, `RegularArrived`, `ArcAdvanced`,
`ClockStop`, `GuestIntroduced { guest, key }` (§11), `DoorsOpened { early_minutes }` (22:00, the preparation is over), `PaymentMade/Missed`, `CafeClosed`, `SchedulePublished`, `ChecklistTicked`,
`MenuUnlocked`, `FurnitureUnlocked`, `FurniturePlaced/Removed`, `DayStarted`, `NightStarted`, `NightEnded`.

Appliances (§11): `EquipmentUsed { uid, action, by }`, `EquipmentCue { uid, cue: Bubbling | Ding |
Sputter }`, `TillCounted { revenue, guests, tips }`, `TrashOut { cat }`.

Hands-on (§9): `PlayerOrderTaken { guest, party, wish, kind }` (open the order card),
`PlayerServed { guest, recipe, match_score, quality }`, `PlayerQueueChanged`, `PlayerGreeted`,
`GuestReaction { guest, tier, key }` (`react.*` bubble), `GuestPatienceRefilled` (glass of water),
`GuestForgiven { party, key }` (`ui.guest.forgiven`), `TipLeft { table, amount }`,
`TipCollected { amount, by }` (`TipReceived` still fires per coin), `RecipeLearned { recipe, source }`,
`IngredientUnlocked`, `PractisedRecipe`, `RushStarted/Ended`, `RhythmChanged { step }`,
`KarolinaIdle { minutes }`, `LullSuggestion { hint: Talk | WalkIn | Mud | Speed }`,
`RegularIncoming { regular, party, minutes }`, `RecordChanged { record, repeat }`,
`HallMoodChanged { mood }`, `RecordUnlocked { id }`, `RecordReaction { guest, record, liked }`,
`QuietHour { on }`. New emotes `SimEmote::Sparkle` / `Smile`.

## 5. How it works

* **Work loop (helpers only):** an idle helper picks, among the tasks of **their duties**, by rank
  & distance: register = plain orders at the queue front / seated tables; bar / kitchen =
  unowned plain items (and handed-off wishes) at a free station; serving = items helpers made
  (tray ≤ 4); dishes = clear tables (+ their tip coins) → sink, wash; floor = mop, buckets;
  checklist items of the duty's job. Never wishes, regulars or Karolina's items. Speed =
  `(0.5 + skill×0.1) × helper_speed (0.85)` × trait `Speed` × event `WorkSpeed` × energy/mood ×
  station slow-downs × hall mood. Karolina is driven by her queue only (§9.1).
* **Guests:** arrivals per minute from `hour_weights × weekday_mult` of the town's pools × reputation ×
  weather × `GuestRate` (× 0.5 in the quiet hour); trains add passengers (Cozy: spread over
  `COZY_TRAIN_SPREAD` = 24 min); rushes from events. **Cozy: full hands** (0.2 polish) — while
  `guests::cozy_pressure` (`karolina_todo` + parties arriving / in the queue / waiting to order)
  ≥ `COZY_HOLD_PRESSURE` (3), a crowd party (`PendingSpawn.crowd`: train passengers, event-rush
  guests; a random walk-in is deferred into one) lingers outside a minute longer (≤
  `COZY_HOLD_MAX` 60 min, none from 05:00), and the ones who waited come in one party a
  minute (all in by `COZY_HOLD_LAST` 05:20, so the closing checklist still fits). The Cozy
  train spread is cut so passengers are in by last call (05:40). Classic never holds. Seats are reserved on arrival, one chair per guest (no seat →
  they walk on). Paths go around chairs and stools (`grid::SEAT`) unless there is no other way; a
  guest with no way to the goal waits and retries (never walks through the furniture). Plain parties queue at the register (or sit when the line is long); wish
  parties always sit and wait for Karolina. Patience runs on the wait clock (§9.4); food waits
  get `FOOD_PATIENCE` 2.4× (Cozy `COZY_FOOD_PATIENCE` 3.0×). Satisfaction: wait times, item
  `mood_bonus`, quality, server traits, seat comfort, beauty, dirt, music, + the hands-on serve
  formula. Pay = (price + recipe `price_add`) × spend × `income_mul`; tips → coins (§9.5); review
  → reputation (forgiven guests write none).
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

`world.save_to_string()` → RON with `version` (`SAVE_VERSION = 2`); `World::load_from_str(&text)`.
Caches (walk masks, paths) rebuild on the next tick (or call `rebuild_caches(&content)` before queries).
Same seed + same commands ⇒ identical saves (tested, also with the player-bot).
A save made in the preparation half hour keeps `prep_left` (ticks until the doors open) and
continues identically; saves without the field (0.1.0) load with the doors open.

**v1 → v2:** every new field is `#[serde(default)]`. On load a version-1 world gets helpers'
duties from their priority-1 jobs (cleaning → dishes), Karolina's autopilot task dropped, and on
the first tick / command / `rebuild_caches`: starter + menu-unlocked recipes, the starter pantry
and the 3 starter records; grandma's jukebox is placed at `JUKEBOX_TILE` (now, or the next Day
for a mid-night save). Fixtures: `src/sim/testdata/save_v1_*.ron` (made by the pre-update code).

## 7. Balance (headless)

`cargo run --release --example sim_headless -- [nights] [story|sandbox] [cozy|classic] [seed] [--player-bot [quality] [react_s]]`
(`SIM_STRESS=1` adds 60 guests nightly). `sim_balance -- [nights] [mode] [diff] [seeds] [--player-bot q r | --no-bot]`
averages seeds and prints the handson.md §9 targets (`SIM_PLAN=3,5,9` = hire nights).
`sim_trace` prints staff/party states for debugging.
Knobs: `guests::{BASE_PARTIES_PER_HOUR, FOOD_PATIENCE, COZY_FOOD_PATIENCE, SECOND_ITEM_CHANCE,
DRINK_WITH_FOOD_CHANCE, REVIEW_WEIGHT, story_ramp}`, `staff::{STAFF_SPEED, TRAY, FIRST_HIRE_NIGHT}`,
`COZY_TRAIN_SPREAD`, `narrator::CHOICE_MINUTES`, `economy::CLASSIC_OVERDRAFT`, and all of
`handson.ron` (content).

## 8. Types defined in sim (not in content)

`PlayerState`, `PlayerAct`, `ActiveAct`, `BrewMode`, `OrderKind`, `ItemOrigin`, `TipCoin`,
`Knowledge`, `RecipeStats`, `MusicState`, `MusicSource`, `HouseState`, `LullHint`, `HandsTally`,
`Ticket`, `TicketOwner`, `TicketStage` (`sim::hands`);
`Difficulty { Cozy, Classic }`, `Mode`, `Phase`, `SandboxSetup`, `Town`, `Shift`, `LeaveKind`,
`RequestStatus`, `Facing`/`Pose`/`SimEmote` (render mirrors), `LookRef`, `AgentView`, `FurnitureView`,
`NightSummary`, `Task`, `StaffStatus`, `PartyState`, `PayrollWeek`, `Handover`, `BotMsg`;
`sim::grid::DEFAULT_LAYOUT` (CAFE_LAYOUT + register/fridge/sink/lamp/decor letters + street) and
`world::STORY_INSTITUTIONS` (the story station: taxi rank, hospital, college).

## 9. Hands-on: Karolina behind the counter (design/handson.md §1, §3, §10)

The owner (`Staff { owner: true }`, `world.karolina()`) is the **player actor**: no duties, no
priorities, no auto breaks; Cozy no tiredness (Classic: half drain). She only does what the
player queues. Staff are helpers who only do delegated duties — the café never runs itself.

### 9.1 Her queue (`world.player()` → `PlayerState { queue, current, rhythm, idle_minutes, … }`)
FIFO, `PLAYER_QUEUE_MAX` = 5 (+ the current act, `current.progress()` for the ring). Every act
claims what it works on (helpers leave it alone); cancel / unqueue releases it. She walks at
`STAFF_SPEED × player_walk_mul` and works at `player_work_mul × WORK_SPEED` (skill-independent).

| command | what she does |
|---|---|
| `PlayerTakeOrder { guest }` (guest or party id) | walks to the table (waits for them to sit) or the register; `PlayerOrderTaken` → open the order card. `InUse` if a helper has it |
| `PlayerBrew { guest, recipe, quality, mode: Quick \| ByHand }` | the leader's open wish (any known, available recipe; match = `content::match_score`) or a plain item (a recipe on that menu item). ByHand: `quality` from the mini-game, work = prep × `by_hand_work_share`; Quick: quality 0.6 (0.8 practised), full prep, and the party's other plain items at the same station join the round. Then she carries everything on a tray and serves (`PlayerServed`). Errors `NotUnlocked` (unknown recipe), `Invalid` (not available tonight / nothing to brew), `QueueFull` |
| `PlayerUsual { guest, quality }` | "The usual?" (`OrderKind::Usual`, trust ≥ `usual_trust`) = PlayerBrew by hand |
| `PlayerHandOff { guest }` | the wish becomes a helper ticket (best starter/card recipe, match ≤ `hand_off_match_cap`, no story credit); needs a bar/kitchen helper (+ register before the order) |
| `PlayerQueue { act, front }` | `TakeOrder`, `Serve { item, extra: [] }` (helper-made, tray auto-filled), `ClearTable`, `Wash { sink }`, `Mop { tile }` (`world.dirt_spots()`), `CollectTip { table }`, `Checklist { id }`, `PlayRecord`, `GoTo`. `front` = shift+click |
| `PlayerWashProgress { sink, seconds }` | while she stands at the sink (`current.stage == 1`): 2 dishes / real second |
| `PlayerGreet { guest }` | a wave, no walk, within `greeting.window_minutes`: +3 satisfaction, regulars +0.5 trust |
| `PlayerCancel`, `PlayerUnqueue { index }` | a cancelled wish brew re-opens the wish |

Queries for the order card: `order_kind(guest)`, `best_matches(&c, guest)` (top 4 `(recipe,
match)`), `usual_available(&c, guest)`, `default_brew_mode(&c, wish, recipe)` (Quick after
`by_hand_streak_max` same-kind mini-games), `quick_quality(&c, recipe)`, `recipe_available`,
`known_recipes()`, `knowledge()`.

### 9.2 Orders & wishes
Every leader gets `OrderKind::Plain | Wish { wish } | Usual { wish, recipe }`: regulars always
wish (personal wishes first, else generic ones of their archetype); others with
`wish_chance_on(night)` (sandbox `sandbox_wish_chance`, quiet hour `wish_share`). Fair wishes:
weight × `boost_mul` when the best available match ≥ `boost_at`, never below `skip_below`;
`weight_mods`, the hall mood's and the season phase's `wish_mul`; `uses_per_night`. A wish
leader may still order something on the side; the others order plain. Serve formula
(Karolina's items): satisfaction += `satisfaction_match × m + satisfaction_quality × q` (Classic
not-quite: −2), regulars' trust += `trust_base + trust_match × m + trust_quality × q`, tip factor
`(tip_base + tip_span × m × (0.5 + 0.5 q)) × rhythm`; plain items use `PLAIN_CREDIT` 0.5 for `m`.
Helpers' items: their quality `helper_quality.at(skill)`; trust the old flat +2/+4. Reaction:
`GuestReaction` (`react.<regular>.<tier>` or `react.<tier>.<n>`), emotes Sparkle/Heart/Smile/Coffee.
Knowledge: starters, grandma's cards (`WithMenu`, at day start), `Taught` (teacher present +
`when`), `Story`; practised after `practised_after` by-hand plays ≥ 0.7.

### 9.3 Delegation
`Duty { Dishes, Floor, Register, Bar, Kitchen, Serving }` (`staff_duties(id)`). Capacity
`capacity(&c, id)` = 1 + [nights_worked ≥ 3] + [a held duty's skill ≥ 7 or `hard_worker`], ≤ 3.
New hires (and the hire card's candidates) carry the archetype's `default_duty`.
`suggest_duty(id)` for the board's "Suggest". Story: the first hire is offered on night 3
(`FIRST_HIRE_NIGHT`, whatever `hire_from_night` says). Digest credits:
`NightSummary.helper_credits` → `app.digest.helper_<duty>` lines.

### 9.4 Waiting, Cozy forgiveness, rush, rhythm, quiet hour, the lull director
Per party `wait_clock`: rate 1.0, × `claimed_wait_rate` 0.5 while Karolina is on them (her
claim or her items), × `cozy_rush_wait_rate` in a Cozy rush; patience drains at the same rate.
Cozy: patience out → a glass of water (`GuestPatienceRefilled`, refill 50 %), the second time
they leave content (`GuestForgiven`, satisfaction ≥ 50, no review); regulars keep refilling until
their `leave_by`; the regular patience floor is 12 min. Classic: the old angry leave. Rush: ≥ 3
arrivals in 10 min (story from night 3) → `RushStarted` for 20 min. Rhythm: serves within 4 min
build steps (+4 % tips each, ≤ +20 %), a gap resets silently. Quiet hour 03:00–04:00:
`QuietHour` + toast, guest rate × 0.5, house music → piano trio. Lull director: Karolina idle
(empty queue, `karolina_todo() == 0`, no tip coins) for `lull_minutes` (quiet hour 15) →
`KarolinaIdle` + `LullSuggestion`: a regular's Talk → (T5: moments, margin notes) → a walk-in
with a wish in 1–3 min (+ `lull_walk_in` guest rate from story night 3) → rain mud by the door →
the ×2 chip; then not again for the same time. `karolina_todo` = waiting orders / open wishes /
unowned rounds / trays / dishes / dirt nobody on the floor holds.

### 9.5 Tips & the ticket rail
A tip becomes a `TipCoin` on the leader's table (`tips_on_tables()`, `TipLeft`): Karolina's
`CollectTip` / `ClearTable` or a dishes helper books it (`TipCollected`, `tonight.tips`,
`tips_lifetime()`); at 06:00 every coin goes into the jar. `ticket_rail(&c)` →
`Ticket { party, guest, stage: Order | Wish | Make | Serve, owner: Karolina | Helper(id) |
Nobody, wait_frac, regular }` (sorted by wait during a rush).

### 9.6 The jukebox
Grandma's jukebox stands in `DEFAULT_LAYOUT` (`j`, `JUKEBOX_TILE`). `music()` → `MusicState {
source: House | Record { id, repeat }, unlocked, known_tastes, opening, … }`;
`music_track(&c)` → the track id + volume to play now (house calm / rush / quiet, a record, the
day theme; `None` in a blackout). `PlayRecord { id, repeat }`: Day → tonight's opening record;
Night → Karolina's act (1 s needle). Errors `Locked`, `NoJukebox`, `NoPower`. `RecordFinished`
ends a play-once pass. The record sets the hall mood (`hall_mood(&c)`; tips, helper speed, stay,
wish weights, archetype satisfaction, patience, elder trust). Likes / dislikes after
`like_minutes` while present, once per visit: trust +2 (+3 if requested; never a loss), bubbles
`record.<id>.liked/disliked.<regular>`, `RecordReaction`; archetype `music_likes/dislikes`
±2/−1. Records unlock by `Unlock::When` (day start + every minute) or `sandbox_night`.

### 9.7 The player-bot (`sim::bot`)
`PlayerBot::new(BotConfig { quality, spread, react_seconds, minigame_seconds, card_seconds,
plain_card_seconds, hire_nights, always_by_hand, by_hand_gap, seed })`, `day()`, `step()` before
each tick, or `run_night()`. One decision per `react_seconds`; the order card and mini-games
are overlays (sim slowed by `overlay_scale`); by hand for regulars and calm moments, quick when
busy; regulars first; delegates tonight's crew (dishes, bar, serving, register, kitchen, floor).
`nights` → `BotNight { minigames, same_kind_max, overlay_seconds, real_minutes, max_rail, … }`.
Results (6 seeds, nights 1–10, q 0.75, 3 s): see the T4 report / `sim_balance` output.
T9 tuning: the bot hires the candidate whose hire card covers the chore nobody holds (dishes,
kitchen, bar…), keeps each helper on their card's duty and fills the gaps by need; plays by hand
when ≤ 4 things wait and 10 min passed since the last mini-game. In the sim a regular's wish brew
goes to the front of her queue, the order card proposes Quick once a night had
`BY_HAND_SOFT_CAP` (12) mini-games, and the auto-fill may give a helper one night more when that
keeps a colleague's pref (so the story crew gets a happy week from Auto-fill alone).
`sim_balance` prints a T9 line (todo peak p50/p90, overlay minutes, payments by run, endings,
cat affection); `SIM_TOWN=1` lists sandbox towns. Locks: `src/sim/tests_balance.rs`.
**Preparation half hour:** the bot lights the window and queues the opening items from 21:30;
`Either` items wait for a helper holding the duty until 21:50, then she does what's left.
`BotNight::prep_ticks` count at `prep_tick_scale` in `real_minutes`. Story Cozy, 8 seeds × 30
nights: night 13.4 → 13.9 real min, rituals 0.4 → 1.0 per night (the opening list is ready by
21:50–21:59 every night), mini-games 11.8, forgiven 8.8 %, idle 6.8 %, payments 40/40.

## 10. Sim systems (T5: design/handson.md §4–§6, §11–§13)

All state lives in `World.sys: SystemsState` (`world.systems()`), every field `#[serde(default)]`
(save version stays 2; older v2 saves get it on the first tick / command / `rebuild_caches`).
New `PlayerAct` variants were **not** added (the screens match it exhaustively): a moment's option
and a night experiment are a `PlayerAct::GoTo { tile }` with the work kept in
`world.walk_jobs()` (`WalkJob { tile, kind: WalkKind::Moment { id, option } | Experiment { .. } }`,
`WalkKind::id()` for the queue icon).

### 10.1 `Cond` / `Fx` everywhere (`sim::xlogic`)
Every `XCondition` is evaluated (`MomentResolvedEver`, `CozinessAtLeast`, `SharedShifts`,
`NapkinsAtLeast`, `ThemeOwned` now real) and every `XEffect` runs: `Later` (dropped at 06:00) and
`NextNight` are stored with their `XTarget` (party / staff / tile / furniture / moment / regular /
item / record / train minute); core effects inside `Fx` run through `logic::run`. Users: moments,
napkin rewards, checklist `extra_done/extra_missed`, rituals, handover notes, announcements
(`thank_you {name}`, `new_item {item}`, `swap_board`), tutorial `on_show`. `ArcStage.when: Vec<Cond>`
(content, additive) is checked with `conditions` — Mirek / Hana stage 1 take `WishMatchStreak`,
Vera stage 3 `ServedRecipe("r_kulajda")`.

### 10.2 Recipes & experiments
`Command::Experiment { ingredients }` (alias `TryExperiment`): 2–3 unlocked jars. Day = at once;
night = Karolina walks to the counter and works `experiment_minutes`. Cost `experiment_cost` of
supplies (free on Cozy nights ≤ `experiment_free_until_night`). Result `SimEvent::ExperimentDone {
result: ExperimentResult::Known | Discovered | Gated { key } | Close { key, family } | Nothing {
key, cat } }` (keys `experiment.*`, `family` = `ingredient.family.<id>`); a discovery also sends
`RecipeLearned { source: "experiment" }` and a `RecipeDiscovered` polaroid. Fair hints: recipes
needing a jar that isn't on the shelf never steer a "close" hint. Every `margin_note_every` failed
tries → `MarginNote { recipe, ingredient }` on the silhouette with the most revealed so far (all
jars on the shelf). Queries `pantry()`, `revealed_ingredients(&c, r)`, `knowledge().tried`,
`experiment_cost(&c)`, `margin_note_target(&c)`.

### 10.3 Moments
Director each minute (22:10–05:30): budget per act (`moments_per_night`, rolled at night start),
gap `moment_gap_minutes`, conditions, agent / furniture anchors, `per_night_max`, `max_per_game`,
`cooldown_nights`; the season phase's `moments` × 6, `moment_weight`s, hall mood weights, quiet
hour (`quiet` × 3, others × 0.3). Story nights 1–3: never two open; later ≤ 2. Events named by a
moment's `from_event` spawn it instead of their choice toast (nights 1–3 it waits ≤ 20 min, then
its `on_expire` applies). The lull director tries a moment after a regular's Talk (up to the
night's cap). `SimEvent::MomentAvailable { id, anchor: MomentPos { tile, furniture, agent } }` →
`Command::ResolveMoment { id, option }` (options now: `moment_options(&c, id)`; `show_if`,
`within_minutes`) → Karolina walks (the card closes; `ActiveMoment.resolving`), works by the
interaction (hang spots, quick brews, light each table…) → `MomentResolved`. Expiry →
`on_expire` + toast `moment.<id>.expired` + `MomentExpired`; an agent who left → `MomentExpired`.
Bot buttons can spawn moments (`ApplyMoment`), effects too (`SpawnMoment`, e.g. `m_leak_empty`).
Queries `active_moments()`, `walk_jobs()`.

### 10.4 Extras
* **Cat**: counter `cat_affection` (clamped 0–10) → `CatLevel { level }` + toast `cat.level.<n>`,
  perks (latte `cat` at 3, inside at 4, window seat +2 satisfaction near it, crew mood +2 a night
  at 8, the naming moment `m_cat_name` at 10 → var `cat_name`). `station_cat(&c) -> CatView { tile,
  perch: doormat|radiator|window_seat|follows, level, name }`.
* **Napkins**: Karolina-made serve with match ≥ 0.8, quality ≥ 0.7 → `thanks_chance` → the party
  leaves `TipCoin.napkin` (personal ones for regulars first); collected with the coin →
  `ThankYouNote { id }`; rewards at 10/20/40. `napkins()`.
* **Tip jar**: lifetime tips past a goal → `TipGoalReached { goal, choices }` →
  `Command::PickTipGoal { choice }` (`UnlockFurniture`; if one is owned the other is given).
* **Specials**: `Command::SetSpecials { recipes }` (≤ 2 known; match +0.1; ticks `menu_board`).
* **Quiet hour**: T4's mods + quiet moments preferred.

### 10.5 Coziness, seasons, themes
`coziness() -> Coziness { value 0–100, level 1–5, parts [7] (CozyComponent::ALL order),
weakest }`, recomputed each minute: light (hall seats under a lit lamp, candles in a blackout,
window light +2), beauty (hall Σ + theme lamp perk + fixed tables), comfort, clean (dirt, dishes),
music (house 6, record 10, blackout 0), theme (+ accent pieces), touches (cat ≥ 4, garlands in
season, ≥ 4 plants, napkin wall), hall mood bonus, `Coziness` effects. Level change at night →
`CozinessLevel`; the first "Like home" → polaroid. Effects (replacing the old beauty / dirt / music
/ comfort ambience; a small seat-comfort term stays): satisfaction once served `(coz − 35) × 0.25`
(decor.ron, tuned so grandma's default hall ≈ +7 like before), patience × `0.9 + coz/250`, stay ×
`0.9 + coz/500`, tips × `0.96 + coz/1250`. **Seasons**: `season_phase(&c) -> PhaseDef` (story
calendar / sandbox `town.season` × night of season), `SeasonPhaseChanged { id }` + toast at day
start, its latte patterns; `garlands_up(&c)` (flag × phase). **Themes**: `theme(&c)`,
`themes_unlocked()`, `theme_owned(&c, id)`; `Unlock::When` / `sandbox_night` → `ThemeUnlocked {
id, owned }` (price 0 = owned); `Command::BuyTheme { id }`, `Command::SetTheme { id }` (Day only,
undoable) → `ThemeChanged`; perks `BeautyPerLamp`, `RegularTrustOnEnter` (once a night),
`ArchetypeSatisfaction`, `MissedBy` (`theme_missed_line(&c, regular)`). `lattes()`.

### 10.6 The journal
Highlights (`journal.ron` scores, −`repeat_kind_penalty` for a kind kept last night, the 3:33
guest / Anwar repeat scores, arc-good bonus) are captured with a `SceneSnapData` (origin, size,
row-major `tiles`, `props` with sprite ids, `actors` with `LookRef` / pose / `SnapExpression` /
emote, served `cups` with recipe, cat, night progress, weather, rain, season, theme, garlands,
snow, blackout, window light, coziness, train, seed, focus, zoom — ≈ 2–5 KB) → map 1:1 onto the
render's `SceneSnap`. Sources: arc beats (`arc.<regular>.<stage>`), key cutscenes, moments'
`Highlight` effects, perfect serves for regulars, beautiful lattes, full house, cat levels,
like-home, recipe discovered, rozpis happy, Guest333 / Anwar, the 03:00 fallback (always one).
Top 3 per night (`tonight_highlights()`, `HighlightCaptured { kind, score }`); at 06:00
`PolaroidsReady { night, count }` and `pending_polaroids()`; `Command::PickPolaroid { index }`
(else the best is kept at the next night start) → `journal() -> [JournalEntry { night, date,
highlight }]`, `JournalUpdated`.

### 10.7 The rozpis puzzle
Prefs: story people from `staff.ron`, generated people from traits (`pref_rules`, deterministic by
seed + id), life events as one-week prefs. `prefs(id)`, `pref_status(&c, id)` (on the draft, approved
leave counted as Off), `wellbeing(id)`, `week_fairness()`, `coverage(&c) -> [CoverageNight {
suggested, on, cups }; 7]`, `RozpisScored` on every draft change. Auto-fill (`AutoSchedule` =
`AutoFillRozpis`, also what the app plans while not edited by hand): T4's base rozpis, then a local
search moving days off / dropping a night for a Need / adding one back for coverage (cost: 100 − wb
+ soft distance to each pref + coverage), then shift kinds by `ShiftPref` (a Need always; a Wish
when a colleague with the same main job covers the other half); `autofill_report()` for
`rozpis.autofill_result`. Published week: mood baseline, shift-start energy, late / sick chance ×,
`PrefViolated` (Need → mood −8), `WorksWith` relationship / +5 % speed, shared shifts; a happy week
(all prefs, coverage ≥ suggested − 1) → `RozpisHappy` + bot + polaroid + crew mood +3 all week.
Requests: Need broken tonight → 50 % swap request with a fitting colleague (day start); toil ≥ 8 h
→ náhradní volno request; life events (`barista_exam` time off, `cashier_daughter` "Can I go at
2?" — `early_leave_requests()`, `Command::AnswerEarlyLeave`, bot `early_leave`). Answers: approve
wb +10 / relationship +5 (partner −3), decline mood −6 (Need: 25 % late), 3 declines in 14 nights →
`looks_tired`; unanswered at night start → `manage_alone`, mood −3. `RequestCreated`,
`RequestAnswered`, `request_meta(id)`.

### 10.8 The app as a tool
Checklists: helpers do `Either(duty)` items of their duty; `extra_done` / `extra_missed` run
(Opening at 22:31, Closing at 06:00); rituals → `RitualComplete { list }` + toast + effects (closing
→ tomorrow's opening work × 0.5). `handover_note` ticked prefills the two best suggestions;
`handover_suggestions(&c)` (from 05:00) → `Command::PickHandover { picks }` → next night
`HandoverApplied { ids }` with the subject (low item, worn machine, a regular's "remember" mark,
helpers ready before the rush train, crew mood, a fixed table). Bot handlers: `CoverSelf`,
`CoverOther` (duty moved until they arrive), `CallIn` (an Off helper without a Need comes +60 min),
`ReserveSeat`, `RemindBefore` (`Reminder` event), `OrderDrone` (needs a dock), `MenuOff`,
`ApplyMoment`, `Camera` (`CameraHint`), `Open` (`OpenApp`); a button that can't act returns an error
and the message stays open. New bot messages: `regular_tonight` / `regular_to_go` (day), `low_stock`,
`weather_alert`, `cat_waiting`, `rozpis_happy`, `looks_tired`, `manage_alone`, `early_leave`.
`NightSummary.insights` (≤ 4 `Insight { key, args, link }`). Payroll: `SetTipSplit`, `PayBonus`,
`GrantToil`, `SendPayroll { week }` → `PayrollSent { week, total, tips_to_crew }` (auto one day
later); `tip_split(&c)`, `payroll_sent(week)`.

### 10.9 Integration requests (T7)
`TutorialShown { id }` / `SetTutorial { on }` (story `At(time)` steps run their `on_show` by
themselves: Mirek at 22:04 on night 1, the 22:20 rush, the fogged window) → `TutorialEffects`;
`TableCleared { table, dishes, by }`, `Washed { dishes }`, `Mopped { tile }`;
`PlayerBrewStart { guest, recipe, mode }` + `PlayerBrewQuality { guest, quality }` (she brews while
the mini-game runs and holds the cup — `BrewAwaitsQuality` — until the quality arrives, ≤ 200
ticks, then the skip quality); `TryExperiment`; Karolina waits for a guest to sit down at the
counter, not by the empty chair. Singular guest names: `guest.<archetype>.one`.

### 10.10 Player-bot & numbers (T5)
The bot also resolves moments (urgent ones first), experiments (following margin notes), keeps a
polaroid, counts the jar, buys / switches themes, auto-fills on Mondays, answers requests and bot
buttons, ticks checklists nobody else does, picks specials, books payroll (`thrifty` keeps money
comparable). `sim_balance` prints a T5 line (moments per night, two-open check, journal
completeness, per-system counts). `--thrifty` for money comparisons.

## 11. Reasons to build (0.2 package C: `build_wishes`, `corners`, `attic`, `backroom`, `build_content`)

State: `World.build: BuildState` (`#[serde(default)]`, every field defaulted — 0.1.x saves load
unchanged; `world.build_state()`). Hooks: day start (attic restock, requests, corners), night
start (requests, corners, the before / after plan), each night minute (requests), 06:00 (the
before / after joins the morning), after every furniture command and its undo (corners,
requests). Nothing consumes the world RNG except the requests' rewards (`Fx`).

### 11.1 Requests (`build_wishes.ron`)
`World::build_wishes(&c) -> Vec<BuildWish { who, def, tag, reason_key }>` — open requests not
answered yet (the build card's "wanted" badge). `who` = a regular id (`"filip"`), a helper's
staff id as a number (`"7"`) or a guest archetype id (`"students"`).
`World::build_requests(&c) -> Vec<BuildRequest { id, who: BuildWho::{Regular, Staff, Guests},
def, tag, reason_key, ask_key, thanks_key, status: BuildRequestStatus::{Waiting, Built,
Thanked(night)}, opened, arc }>` (waiting first; the app's cards), `build_requests_waiting(&c)`.
A request opens once its person is known (a regular who has visited, a helper of that
archetype employed — `Staff(None)` = the first helper — or any guest archetype) and `appear`
holds, unless `done` already holds (`BuildWishOpened { id, who }`). At night, while the person
is in the café (seated / queueing; a helper on duty) they say `.ask` once (`Speech`); once
`done` holds they say `.thanks` (`Speech`), `BuildWishFulfilled { id, who }` and the `reward`
Fx run with the person as target (`TargetTrust`, `TargetStaffMood`, `TargetSatisfaction`,
`ThankYouNote`…). Story arcs keep their own conditions (`arc: true` only marks the card).

### 11.2 Corners (`corners.ron`)
`World::corners(&c) -> Vec<CornerView { id, name_key, tiles, bonus, center, members }>` (pure;
labels in build mode), `World::corners_bonus(&c)`. A corner = its first need (the anchor piece,
in the hall) + every other need's `count` pieces within `radius` tiles (Chebyshev, footprint to
footprint); pieces aren't shared within a kind; `max` per kind; the sum is capped at
`max_bonus` and added to `coziness().value` (not to a part). Arrivals: each standing corner
multiplies its `attracts` archetypes' weight by `attract_mul`. A new instance emits
`CornerFormed { id, tile, first }`. Grandma's default café forms none.

### 11.3 Grandma's attic (`attic.ron`)
`World::attic(&c) -> AtticView { open, offers: [AtticOffer { id, price, new_price, owned, placed
}], upstairs, next_night }`. Restock at day start on `first_night` (3) and then every
`restock_weekday` (Sat): `per_week` pieces with `from_night ≤ night`, not owned, picked by an
RNG of the seed + night (`AtticRestocked`, toast `attic.toast.found/.restocked`).
`Command::BuyAtticPiece { id }` (Day only): buys an offered piece (or brings a bought one down
for free) onto the free hall tile nearest (8, 12) with a free ring around it, never by the door
(`paid: 0` — removing refunds nothing and it waits upstairs); `AtticPieceBought { id, uid, price
}` + `FurniturePlaced`. Errors `Locked`, `AlreadyPlaced`, `NotEnoughMoney`, `Occupied`,
`WrongPhase`. The pieces are `furniture.ron` defs hidden by the never-set flag `attic_piece`.

### 11.4 The back room (`attic.ron` → `back_room`)
`World::back_room(&c) -> BackRoomView { open, available, from_night, price, tiles }`.
`Command::OpenBackRoom` (Day, once, from night 15, 14 000 Kč) widens the grid by
`BACK_COLS` (6) columns to the right: every old tile keeps its coordinates (cutscene spots, the
jukebox, saves), x 30–34 × y 1–14 become hall floor with the new outer wall at x 35 (two
windows in the back wall), the street continues below, and two tiles of the old right wall
(x 29, first free pair of rows 3/4, 2/3…) become the doorway. Grandma's pieces are placed free.
`BackRoomOpened { price, tiles }`, toast `backroom.toast.opened`; the undo stack is cleared.
The back room's floor doesn't count in the beauty denominator (extra floor never thins the
hall out). The render rebuilds its statics by the grid's size.

### 11.5 Before / after (journal kind `BeforeAfter`)
At each night start the sim remembers the hall (furniture coziness = light + beauty + comfort
in day light + corners, the corner set, the uids, the back room, a whole-interior snapshot).
If the next night start finds the back room newly open, a new corner, or a gain ≥ `min_gain`
(1.0), the "after" is shot at `after_at` (00:30) around the change and the remembered hall is
cropped to the same window: `Highlight { kind: BeforeAfter, before: Some(snap), snap: after,
caption_key: journal.caption.before_after, args: [what = @key] }`. At 06:00 it always joins the
morning's candidates (replacing the weakest if the list is full). `Highlight.before` is
`#[serde(default)]`; the journal UI composes the two photos side by side.

### 11.6 The player-bot
`BotConfig.build` (default true): an attentive bot builds for the regulars, opens the back room
once it has the price + 8 000 to spare, and buys the cheapest attic piece with 6 000 to spare
(not when `thrifty`). `build: false` = a player who never builds (tested: the story still ends
in the light).
## 11. Hands-on appliances (v0.2, `sim::equipment`, design/v0.2.md §B)

Karolina can use the coffee machine, the stove, the oven, the fridge, the register and the
trash bin by hand during the preparation half hour and the night. A chore is a queued act
`PlayerAct::Use { uid, action: EquipAction }` (sugar `Command::UseEquipment`; `QueueFull`,
`Invalid` when it isn't on offer now): she walks to the piece's access tile, works
`equipment.work.<action>` ticks (× her work speed), and the effect applies
(`SimEvent::EquipmentUsed { by: Some(karolina) }`). Hold chores (`WarmUp`, `Service`,
`CleanUp`) progress by themselves; `Command::EquipmentHold { seconds }` (the mouse / **E**
held) adds `hold_ticks_per_second` per real second. The trash run is two stops: the bin, then
the street tile outside the door. Lamps have no chore — the UI toggles them with
`SetFurnitureOn` (coziness follows on the next minute).

`World::equipment_action(&c, uid)` is what a click picks (first available of
`EquipKind::actions()`, skipping what is already in her hands):

| kind | action | available | effect |
|---|---|---|---|
| Machine | `Service` | wear ≥ `service_wear` (0.5) | wear → 0; `StationSlow` / `StationDown` mods on its station tag end; their pending event choice is dropped (`ChoiceResolved { choice: "purged" }`) — no cost |
| | `WarmUp` | `coffee_warmup` open | ticks it (by Karolina); `EquipState::warm` (the lamp) |
| | `CleanUp` | from `closing.from_minute` (05:00), `clean_machine` open | ticks it |
| | `PullTray` | doors open, before 05:00, no tray, a tray item on the menu | `tray = 3` until +40 min: the next plain `tray.items` orders get prep × 0.05 |
| Stove | `Stir` | the pot is bubbling | soups (`soup.items`) × 0.6 for 30 min |
| | `SoupOn` | no pot, minute ≤ 120, power, a soup on the menu | the pot: soups × 0.9 (helper's 0.95) all night; bubbles from ~21 min after opening every 35 min for 12 min (`EquipmentCue::Bubbling`); untouched it just keeps warm |
| Oven | `TakeOut` | the cake is ready | `Cake::Out { slices: 8 }` (helper 4): `cake.items` × 0.05 per slice |
| | `Bake` | no cake, minute ≤ 120, power, a cake on the menu | `Cake::Baking` → after 12 min `Ready` (`EquipmentCue::Ding`); a kitchen helper takes it out 10 min after the ding |
| Fridge | `Milk` | not done tonight and minute ≤ 60, or `check_stock` open | `milk_left = 5` (helper 3): `milk.items` × 0.7; ticks `check_stock` |
| Register | `CountTill` | from 05:00, once | ticks `cash_up`; `TillCounted { revenue, guests, tips }` |
| Trash | `Trash` | from 05:00, once | dirt − 0.03; with `cat_chance` (0.35) the cat waits: affection +1 (`check_cat`), `TrashOut { cat }` |

Service applies to every appliance with `wear_per_night > 0` (machine, stove, oven, fridge);
a worn one emits `EquipmentCue::Sputter` every `sputter_every` (25) minutes after opening.

**Item prep:** `equipment::item_prep_mul(w, c, menu, plain)` multiplies a new order item's prep
— in `guests::order_taken` (plain items, the tray only applies here) and in Karolina's wish
brews (`player::brew`). Slices, milk and tray shots are consumed as they are used.

**Helpers:** at 22:00 an on-shift helper with the **kitchen** duty puts the soup on, takes the
milk out and bakes (oven present) if Karolina didn't — the smaller helper numbers above, the
event carries the helper's id. Bar / register chores stay checklist items helpers do as before.

**State:** `HandsState::equip: EquipState` (`#[serde(default)]`; reset at `start_night`; 0.1.x
saves load with the appliances idle). Hooks: `equipment::on_minute` in both the preparation and
the night minute, `on_night_start`. Tuning: `handson.ron` → `equipment`
(`content::handson::EquipmentTuning`, every field defaulted).

**Bot:** each night the player-bot rolls which optional chores it feels like (a separate RNG,
so its other decisions are unchanged): warm-up 60 %, soup 70 %, milk 60 %, bake 70 %, stir
80 %, take out 100 %, a tray 50 % (only within 10 min of a train), service 100 %, clean-up
50 %, till 70 %, trash 60 %; hold chores are held. `BotNight::equipment` counts them.
Balance (`sim_headless 30 story cozy|classic 4242 --player-bot`): see the commit / report —
the money and the served counts move by a few per cent at most; the light ending is reached
with and without appliances.

## 11. Strangers: a description, then a name (`sim::names`, design/v0.2.md §E)

Every non-regular guest gets `Guest.persona: Option<Persona>` at spawn (`#[serde(default)]`;
regulars and story guests keep `None`):
`Persona { female, elder, name /* Latin, cs/en */, name_uk, name_ru, desc, alt, known }`.
`world.persona(guest)`; `persona.name_in(lang_code)`, `desc_key()` → `stranger.<desc>.<f|m>`,
`short_key()` / `alt_short_key()` → the rail's 1–2 word form (the plain visual one when the
phrase's doesn't fit).

* **Deterministic, no balance effect:** everything is derived from the guest's `look_seed`
  (already drawn at spawn) through a local SplitMix stream — no draws from the world RNG, so
  `sim_headless` numbers are unchanged. Names already used by tonight's guests are skipped
  while the pool allows.
* **The look matches:** `sim::stranger_look(seed, archetype) -> StrangerLook { female, elder,
  top: Garment, extra: Extra, bits }` is pure; the screens dress `PersonLook::random(seed)`
  with it (`screens::game::guest_label::stranger_look`: hair / beard by presentation, grey hair
  when elderly, the top and the extra), so "a woman in a flat cap" is drawn as one. Women-only
  extras (earrings, hair clip), calm extras and no hoodies / T-shirts for elderly guests,
  scrubs for nurses only. Data: `assets/data/guest_names.ron` (styles per archetype, phrases
  that `needs` a visible feature, name pools).
* **Getting to know them:** `World::emit` watches `PlayerGreeted { guest }` (a wave) and
  `PlayerServed { guest }` (only Karolina's own items) → `persona.known = true` and
  `SimEvent::GuestIntroduced { guest, key: "stranger.intro.<greet|serve>.<n>" }` (once; the
  line has `{name}`). Helpers serving never introduce; `Talk` is for regulars only.
* **Saves:** a guest loaded without a persona (0.1.x saves, the v1 fixtures) gets one in
  `World::load_from_str` (not yet known). Strangers never come back on another night.
* UI: `screens::game::guest_label::{label, party_label ("Tereza + 2"), rail_label}`; the
  introduction bubble (appended to a reaction shown at the same moment) and a night-log line
  `stranger.log.met`. Scene for shots: `SHIFTELLA_SCENE=guest_names` (night 8, played by the
  autoplayer to `SHIFTELLA_TIME`, then an unknown stranger's card and a pinned bubble).
  Tests: `src/sim/tests_names.rs`, `guest_label::tests`.
## 11. The walk home and the back step (design/v0.2.md F, `sim::walk`)

Karolina's own small life outside the café. Data `assets/data/walk_home.ron` (compiled in with
`include_str!`, parsed once: `sim::walk::data()`); text `assets/i18n/src/walk_home.tr`.
State `World.walk: WalkState` (`#[serde(default)]`, every field defaults — 0.1.x saves load
with nothing planned). Neither feature touches story flags, trust, money or the world RNG; the
player-bot never uses them (a full Cozy story that walks home every morning and steps out every
night still reaches «Light in the Window» — `tests_walk.rs`).

* `Command::WalkHome { route }` — Day only (`WrongPhase`), once a day (`Invalid`), a known
  route (`UnknownId`). Stores `planned = (night, route)`, `usual = route`, `walks[route] += 1`.
  At the next `start_night` (right after the `NextNight` effects, `walk::on_night_start`) the
  route's `effects` (`Fx`, run with an empty `XTarget`) apply and `walk_in` schedules one
  guest of that archetype with a wish at the given minute (`PendingSpawn { wish: true,
  patience_mul 1.2, spend_mul 1.1 }`); `walk_tonight()` names it for that night. Routes:
  `embankment` `QuickQuality(+0.05)`, `park` `HallMood(Calm, 90 min)` + `Patience(×1.08,
  150 min)`, `bakery` `Coziness(+3, 240 min)`, `tram` a `nurses` walk-in at 22:55 (+ a toast
  `walk.route.tram.arrived` via `Later`); each also toasts `walk.route.<id>.tonight` at 21:30.
* `Command::StepOutBack` — Night with the doors open (`WrongPhase` otherwise), not tonight
  already, minute `from_minute..until_minute` (30–460), not in a rush (`Invalid`). Picks the
  message: unseen messages of tonight's story act first, then unseen general ones (`acts: []`,
  the only ones in the sandbox), then the least seen — deterministic, no RNG. The UI holds the
  café while the scene is on screen (`GameScreen::frozen`).
* Queries: `walk()`, `usual_route()` (last pick, else `default_route`), `walked_home_today()`,
  `walk_tonight()`, `walk_lines(&c, route)` (the first matching variant per beat by season
  phase id and weather — `walk::lines`), `backstep_block(&c) -> Option<BackstepBlock>`
  (`None` = the chip shows), `backstep_message() -> Option<&MessageDef>` (`text_key()`,
  `from.name_key()`).
* UI: `src/screens/game/walk_home.rs` (flow, chips, captions, the message card), art
  `src/render/dawn.rs` (the dawn strip) and `src/render/backstep.rs`.
## 12. Little stories of strangers and the storm night (design/v0.2.md H, `sim::stories`, `sim::storm`)

Data `assets/data/stories.ron` (compiled in with `include_str!`, parsed once:
`stories::data()`); text `assets/i18n/src/night_stories.tr`. State `World.stories:
StoriesState` (`#[serde(default)]`, every field defaulted — older saves load with nothing told
and no storm; `world.stories()`). **No draws from the world RNG**: every roll uses
`stories::roll(w, salt)` (seed + night + salt), so a player who never answers — and the
player-bot, which never does — plays the same night (tested: `noticing_and_fading_never_touch_the_world_rng`).

### 12.1 Stories
* **Director** (`stories::on_minute`, each night minute before the trains and guests): a
  budget at night start (`per_night` weights for 0 / 1 / 2 — about 0.8 a night, so the 22
  vignettes last the story; story night 1 has none), from `from_minute` to `until_minute`,
  `gap_minutes` apart, one open at a time, never in a rush or while `karolina_todo > 4`.
  Candidates: a party leader with a persona (never a regular), seated, settled
  `settled_minutes`, not about to leave, not a moment's target, no story yet tonight; the
  vignette's filters (`archetypes`, `female`, `elder`, `party` size, `from` / `until`, `wet`),
  not told in this game (the sandbox starts the pool over once all were told).
  `SimEvent::StoryNoticed { guest, story }`. A story nobody answered fades after
  `expires_minutes` or when the guest leaves (`StoryFaded`).
* **Answering:** `Command::AnswerStory { guest, answer }` (`WrongPhase`, `Invalid` — no story /
  she's on her way, `UnknownId`, `QueueFull`) queues `PlayerAct::GoTo` with
  `WalkKind::Story { guest, answer }` next to the guest; on arrival (work `answer.work` or
  `work` ticks) the answer's `effects` (`Fx`) run with `XTarget { party, tile }`
  (`TargetSatisfaction`, `Tip`, `TargetStay`, `HallMood`, `Coziness`, `ThankYouNote`,
  `GuestSatisfactionNear`, `Reputation`…), the guest gets a Sparkle / Smile emote,
  `SimEvent::StoryAnswered { guest, story, answer, good }`, and `names::introduce` (they tell
  her their name: `GuestIntroduced`). A **good** answer also adds a polaroid candidate
  (`HighlightKind::MomentWarm`, score `journal_score`, caption `journal.caption.story.<id>`
  with `name = persona:<Latin>|<uk>|<ru>` — `stories::persona_arg`; the journal UI picks the
  language). Cancelling her act releases the card.
* Queries: `stories_active()`, `story_of(guest)`, `story_def(id)`, `stories()` (told,
  tonight's answers, `warm_ever`); `debug_story(id, guest)` for tests / shots.

### 12.2 The storm night
* **When:** `storm.story_nights` (night 29 — Mon 5 Dec, Mikuláš eve: after the first snow, not
  a payment night, no franchise beat) or, in the sandbox, a night ≥ `sandbox_from_night` with
  Snow weather, `sandbox_chance` (derived roll), at most every `sandbox_gap` nights. Decided at
  night start (`storm::on_night_start`, after the weather roll — the weather becomes Snow,
  `WeatherChanged` + `StormForecast`).
* **`cancel_at` (00:50):** `StormCancelled { stranded }` + bot `storm_cancel`
  (`shiftella.ron`). From then on `World::storm_line_closed()`: `trains_tick` returns at once
  (no `TrainPassing`, no passengers), random walk-ins stop (`guests::on_minute`), pending
  spawns hurry in within 6 minutes and anything else (lull walk-ins, event rushes, spawns past
  `platform_minutes`) is dropped. Regulars still come (`spawn_regular` is untouched). Every
  non-regular party inside is **stranded** till `stay_until` (05:40): no `leave_by`, an
  eating party's `stay_left` ≥ the minutes left (+ `id % 6` so they go out a few minutes
  apart), a waiting party's patience floor 30 %.
* **Chores:** `Command::StormBlankets` (Karolina walks to the register's access tile) and
  `Command::StormSoup` (the stove's access tile, else the kitchen's) — `WalkKind::Storm { act }`
  with `work + per_guest × stranded` ticks; on arrival every stranded party gets
  `satisfaction`, the chore's `effects` run (coziness), `StormCared { act, guests }`;
  blankets fill `storm().blanketed` (the screens draw a plaid on each seated one), soup fills
  `fed`, sets their `holding` to the menu's soup and puts the big pot on (bubbling).
  `Invalid` before the cancellation, when done / already queued, or with nobody stranded.
* **`talk_at` (03:00):** speakers = seated stranded strangers (leaders first, left to right);
  with ≥ `talk_min` the biggest talk that fits (an unheard one first) →
  `StormTalk { talk, speakers }` (lines `storm.talk.<talk>.<n>`, `who` = speaker index or
  Karolina, `{a} {b} {c}` = names). Their personas become known. A polaroid
  (`HighlightKind::StoryScene`, `journal.caption.storm`, `polaroid_score` 92, no repeat-kind
  penalty) is captured either way.
* **06:00:** `StormMorning { cared }`; `morning` effects (reputation +2) only when she did a
  chore. No flags, no money, no story counters: both endings are untouched (the full-story bot
  tests run through night 29 — a quiet night, ~30 guests instead of ~60).
* Queries: `storm()` (`StormState`), `storm_line_closed()`, `storm_stranded()`,
  `storm_can(&c, act)`, `storm_talk_def(id)`; `debug_storm()` for tests.
* UI: `src/screens/game/stories_ui.rs` (marker, card, Snowed-in card, blankets in the y-sorted
  queue, the talk overlay — the café is held while it plays, `GameScreen::frozen`). Tests:
  `src/sim/tests_stories.rs`.

## 12. The café's own drink and photo mode (design/v0.2.md I, `sim::signature`)

Data: `assets/data/signature.ron` (compiled in, `signature::data()`): `from_night` 6 (sandbox
`sandbox_from_night` 4), `rework_every` 7, `jars` (2, 3), `max_tags` 4, `price_add` 10 +
`price_per_jar` 5 (`cost_per_jar` 2), `name_max` 24 (characters), `bases` (`coffee` →
cappuccino, `tea` / `linden` → lemon_tea, `cocoa` → hot_chocolate; each with its jar, tags
and styles), `styles` (`MiniGameKind` + variant + a tag), `ingredient_tags` (the jars that may
go in a cup and their tastes), `one_of_family` (one milk), `photo` (`per_night` 3, viewfinder
`zoom` and `pan_tiles`). State: `World.sig: SigState { drink, invited, photos, asked_served }`
(`#[serde(default)]`; old saves load with none).

* **Composing** — `Command::ComposeSignature { base, jars, style, name }` (any phase). Errors
  `NotUnlocked` (before `from_night`), `Locked` (the base's jar / menu item or a jar not on the
  shelf), `Invalid` (2–3 different allowed jars, not the base's own, one milk, a style of the
  base, `signature::clean_name` — trimmed, single spaces, ≥ 1 letter or digit, ≤ 24
  characters — or "once a week"), `UnknownId`. The drink is the recipe `SIGNATURE_RECIPE`
  (`r_signature`, also `content::handson::SIGNATURE_RECIPE`): a world-side `RecipeDef` built
  by `signature::build_def` (base item, `price_add`, `cost_add`, tags from
  `signature::tags_for` — base ×2, each jar, the style, the top 4; a Cold jar drops Warm —,
  the style's mini-game and variant, `unlock: Experiment`, `experimentable: false`). The first
  compose `learn`s it (`RecipeLearned { source: "signature" }`); every compose emits
  `SignatureComposed { name, first }`. The night it was made (or last changed) it may be
  tweaked freely; after that `signature_next_change()` says the night it opens again
  (`changed_night + rework_every`). Day start from `from_night`: `SignatureInvite` once
  (grandma's soft prompt).
* **Everywhere a recipe is looked up** the world's drink joins the content's:
  `World::recipe_def(&c, id)` and `World::all_recipes(&c)` — `orders::matches_for` (wishes,
  `best_matches`), `usual_recipe` (favourites), `player::brew` (clones the def), the public
  `recipe_available`, the digest's unmet-taste count.
* **Asked for by name** — `wishes.ron`: `house_special` (generic, `any: true`, `kind: Drink`,
  favourite `r_signature`, `when: [RecipeKnown("r_signature")]`, weight 0.8) and personal
  `<regular>.house_special` for Mirek, Hana, Vera (from arc 1), Filip, Radek and Lori (weight
  0.4–0.45). The favourite makes it a 1.0 match; `asked_served` counts the served ones
  (`signature_asked_served()`). The game fills the name into the line: the i18n layer
  overrides `recipe.r_signature.name/.desc/.lore` and every by-name wish key with the
  player's words (`screens::game::signature_ui::sync_i18n` → `I18n::set_override`; the
  `.tr` lines are fallbacks; the by-name line is `signature::wish_line_key(wish)`).
* **Photos** — `Command::TakePhoto { focus, width }` (Night only, incl. the preparation;
  `WrongPhase` by day, `Invalid` when tonight's `photo.per_night` are used): a
  `journal::snap_sized` snapshot around the frame (region ⌈width⌉ + 2 × ⌈0.75 width⌉ + 3
  tiles), with `focus` = the frame's middle and `zoom` = region / (width × `SNAP_CLOSER` 1.55)
  so the journal's render frames exactly what the viewfinder showed. Stored as `NightPhoto {
  night, minute, date, caption, snap }` (`caption` = `photo.caption.<n>`, deterministic, no
  world RNG); `PhotoTaken { night, left }`. Queries `photos()`, `photos_left()`. The journal's
  kept polaroids and highlights are untouched.
* **Queries** — `signature()`, `signature_unlocked()`, `signature_next_change()`,
  `signature_bases(&c)` (with "ready"), `signature_jars()` (allowed, on the shelf),
  `signature_preview(&c, base, jars, style)` (the def or why not).
* **Player-bot** — on an attentive day, two seeds in three make one (decided by a hash of the
  seed, never the bot's RNG) one to three nights after it opens, from the first base / jar
  pair that passes; it then brews it when asked by name (the best chip). Balance (seed 4242,
  30 story nights, `--player-bot`): Cozy revenue 201 938 → 199 633 Kč, tips 16 655 → 15 288,
  satisfaction 95.8 → 95.3; Classic revenue 190 872 → 186 840, tips 14 550 → 15 078,
  satisfaction 84.9 → 85.7 — within the noise of a diverged run (the drink rides on a cheap
  base, + 20–25 Kč); both still reach «Light in the Window» (full). `sim_headless` prints a
  `house drink:` line (made night, tags, price add, by-name serves). Tests: `tests_signature`.


## 13. Accessibility (design/v0.2.md J, package `access`)

* **`Difficulty::Relaxed`** (data id `relaxed`, shown as «Carefree»; appended to the enum, old saves
  load). `Difficulty::gentle()` is true for Cozy and Relaxed — every Cozy rule (`== Cozy` checks in
  guests, staff, pacing, rushes, trains, knowledge, restock, overlay scale) now asks `gentle()`.
  `difficulty.ron` gives it `payments: Forgiven` (new `PaymentRule`) with Cozy's multipliers.
  `economy::collect_payment` on a Forgiven preset: `debt -= amount`, the payment is marked paid,
  `payments_made += 1`, a log line `access.log.payment_covered`; no `PaymentMade` / `PaymentMissed`
  event, the money is untouched. The ending conditions are unchanged (only Classic can close).
  Tests: `tests_access` (a whole Carefree story with an always-empty till reaches «light» with no
  payment event and the debt at 0).
* Not in the sim: text size (`ui::Theme::text_scale` → `TextStyle::scale`), colour-blind markers
  (`render::set_marker_shapes`), key bindings (`settings::keys`, i18n tokens `{key.<action>}`).
