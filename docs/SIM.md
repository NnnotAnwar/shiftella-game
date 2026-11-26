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
// one tick = world.real_seconds_per_tick(&content) at ×1 (handson.ron: 0.15 s → a night is
// 12 real minutes; sim::REAL_SECONDS_PER_TICK_X1 is the default). While an overlay is open
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
  `Command::StartNight`) → 22:00. At 06:00 the sim emits `NightEnded { summary }`, then (story)
  `NightEnd` cutscenes / payment, the ending check, and `DayStarted { night }` for the next night.
* Headless: `world.run_night_headless(&content)` runs a whole night with **nobody behind the
  counter** (Karolina steps back: helpers do their duties, Cozy guests forgive). To play,
  use `sim::bot::PlayerBot::run_night` (§9).

Interpolation: agents move up to ~0.5 tile per tick. For smooth 60 fps keep the previous
`AgentView.feet` per agent and lerp by the tick accumulator fraction.

## 2. Commands (`world.apply(&content, Command) -> Result<(), CommandError>`)

| command | notes |
|---|---|
| `PlaceFurniture { def, x, y, rot }` | footprint, layer (rugs/stage/pendant lamps share tiles), money, unlocks, `unique`, never blocks the door / any piece (`BlocksPath`). Floors (`floor_*`) paint a tile. |
| `MoveFurniture`, `RotateFurniture`, `RemoveFurniture` | not while in use (seat taken, station busy) → `InUse`; remove refunds 50 % |
| `SetFloor { x, y, floor }`, `SetFurnitureOn { uid, on }`, `SetMusic { on }` | lamps / jukebox / radio |
| `Hire { candidate }`, `Fire { staff }` | candidates: `world.candidates()` (story persons from `hire_from_night`; sandbox: 3 generated, refreshed every 3 nights). The owner (`Staff::owner`, cast `player`) can't be fired. |
| `SetDuty { staff, duty, on }` | delegate one of the six duties (§9.3); capacity-checked → `Capacity`; undoable. Karolina has none (`Invalid`). |
| `SetPriority { staff, job, priority }` | legacy shim onto duties: > 0 = the job's duty on (cleaning → dishes), 0 = off. The "everyone helps" fallback is gone. |
| `SetShift { staff, weekday, shift }` → edits the **draft**; `PublishSchedule` | `Shift::{Off, Full 22–06, Early 22–02, Late 02–06}`; the night uses `published`. Unpublished edits at night start: crew mood −2. |
| `AnswerSwap`, `AnswerTimeOff`, `RequestTimeOff { staff, kind, weekday }` | dovolená (`Vacation`, paid) / náhradní volno (`Toil`, from >40 h weeks) |
| `BotAction { msg, action }` | buttons of `@shiftella_bot` messages (`app.bot.<t>.action.<a>`); approve/decline link to requests. Never clock-in/out. |
| `AnswerEvent { event, choice }` | answers a `ChoiceNeeded` toast (works while paused); timeout = `default_choice` after 20 game min |
| `CutsceneFinished { id, choices }`, `Talk { guest }` | see §1 / §4 |
| `BuySupplies { amount }`, `SetAutoRestock { target }`, `SetPrice { item, price }`, `SetMenuItem { item, on }`, `RenameCafe { name }` | |
| `TickChecklist { id }` (night), `PostAnnouncement { id, args }`, `WriteHandover { text }` | at night ticking an item is Karolina's act (she walks to it); jobless items (`light_window`, `handover_note`, `light_off_at_six`) are hers alone |
| Karolina: `PlayerTakeOrder`, `PlayerBrew`, `PlayerUsual`, `PlayerHandOff`, `PlayerCancel`, `PlayerQueue`, `PlayerUnqueue`, `PlayerWashProgress`, `PlayerGreet` | §9.1 |
| `PlayRecord { id, repeat }`, `RecordFinished { id }`, `SetAssistWishes { on }` | §9.6; accessibility hand-off |

`world.undo(&content)` reverts the last reversible command (furniture place/move/rotate/remove,
floors, lamps, duties, draft shifts, prices, menu). The stack clears at phase changes.

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
  `COZY_TRAIN_SPREAD` = 14 min); rushes from events. Seats are reserved on arrival (no seat →
  they walk on). Plain parties queue at the register (or sit when the line is long); wish
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
THIRD_ITEM_CHANCE, REVIEW_WEIGHT, story_ramp}`, `staff::{STAFF_SPEED, TRAY, FIRST_HIRE_NIGHT}`,
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
