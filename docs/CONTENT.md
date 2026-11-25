# Shiftella — content & i18n reference

For the sim (wave 2) and integration (wave 3) agents. Code: `src/content/**`, `src/i18n/**`.
Data: `assets/data/*.ron`. Text: `assets/i18n/src/*.tr` → generated `assets/i18n/{cs,en,sk,uk}.ron`.

```rust
use shiftella_game::{content::Content, i18n::{I18n, Lang}};
let i18n = I18n::load("assets/i18n")?;          // Result<I18n, String>
let content = Content::load("assets/data")?;    // Result<Content, String>
content.validate(&i18n).map_err(|v| v.join("\n"))?;
```

`cargo run --example content_check` prints counts + validation. Tests load and validate the real files.

---

## 1. i18n

| API | meaning |
|---|---|
| `Lang { Cs, En, Sk, Uk }`, `Lang::ALL` | picker order CZ, EN, SK, UA. `code()` → `cs/en/sk/uk`, `short_label()` → `CZ/EN/SK/UA`, `native_name()`, `from_code("ua")`. Serde-serializable (for settings). |
| `I18n::tr(lang, key) -> &str` | fallback `lang → en → key` |
| `I18n::trf(lang, key, &[("name", "Mirek")])` | fills `{name}` |
| `I18n::trn(lang, "ui.count.guests", n, &[])` | plural: picks `.one/.few/.many`, sets `{n}` |
| `plural_form(lang, n)` | en: 1/other · cs, sk: 1 / 2–4 / other · uk: proper rules (21 → one, 22 → few, 11–14 → many) |
| `format_money(lang, czk)` | `1 250 Kč` (cs/sk/uk, no-break spaces) · `1,250 CZK` (en); negatives use `−` |
| `format_money_delta`, `format_int`, `format_decimal(lang, v, places)` | `+1 860 Kč`, `12 480`, `4,2` / `4.2` |
| `format_clock(h, m)` | `03:33` (24 h in every language) |
| `I18n::format_duration(lang, minutes)` | `45 min`, `2 h 15 min` (`ui.unit.*`) |
| `I18n::check()` | key parity + identical `{placeholders}` across languages |

**Editing text:** edit `assets/i18n/src/*.tr` (all four languages side by side), then
`cargo run --example content_i18n_build`. A test fails if the `.ron` files are stale.

```text
trait.night_owl.name        ← key line
cs Sova                     ← one line per language
en Night Owl
sk Sova
uk Сова

cast.bot.name
* @shiftella_bot            ← "*" = same text in all four
```

`\n` inside a text = line break. No Russian anywhere. Place names stay Czech (Latin script) in every
language; person names are transliterated in Ukrainian prose (Мірек, Ліда). **Player lines are
gender-neutral** in cs/sk/uk (the player is Lída's grandchild of either gender) — keep it that way.

### Key naming

| prefix | source |
|---|---|
| `ui.*` | base UI vocabulary (`ui.menu.*`, `ui.settings.*`, `ui.hud.*`, `ui.speed.*`, `ui.toolbar.*`, `ui.crew.*`, `ui.priorities.*`, `ui.app.*` (+ `ui.app.tab.*`), `ui.build.*` (+ `ui.build.tab.*`), `ui.plan.*`, `ui.summary.*`, `ui.log.*`, `ui.save.*`, `ui.cutscene.*`, `ui.ending.*`, `ui.guest.*`, `ui.sandbox.*`, `ui.pause.*`, `ui.lang.*`, `ui.common.*`, `ui.job.*`, `ui.skill.*`, `ui.need.*`, `ui.weather.*`, `ui.weekday.<d>` / `.long`, `ui.unit.*`, `ui.count.*` plurals) |
| `trait.<id>.name/.desc` | traits.ron |
| `staff.<id>.role/.bio/.likes/.dislikes` | staff.ron |
| `menu.<id>.name/.desc` | menu.ron |
| `guest.<id>.name/.desc` | guests.ron |
| `regular.<id>.name/.bio`, `regular.<id>.line.<stage><a..>` | regulars.ron |
| `furniture.<id>.name/.desc` | furniture.ron |
| `event.<id>.title/.text`, `event.<id>.choice.<c>`, `event.<id>.result.<c>` (+ toast keys used by `Effect::Toast`) | events.ron |
| `story.act.<n>.title`, `story.night.<NN>.title/.hint`, `story.hint.*`, `story.line.*` | story.ron |
| `note.<id>.text/.where`, `ending.<id>.title/.summary/.variant.<v>`, `ending.extra.<id>.title`, `item.<id>.name/.desc` | story.ron |
| `scene.<cutscene>.<nn>` | cutscenes.ron |
| `cast.<id>.name` | cutscenes.ron cast |
| `app.announcement.<id>.title/.body`, `app.checklist.<id>`, `app.bot.<id>` (+ `.action.<a>`), `app.digest.<id>`, `app.payroll.<id>`, `app.handover.<id>` | shiftella.ron |
| `train.announce.arrival/.delay`, `train.<id lowercased, space→_>.name` (`content::validate::train_key`) | trains.ron |
| `difficulty.<id>.name/.desc`, `sandbox.character.<id>.name/.desc`, `sandbox.institution.<id>` | difficulty.ron, sandbox.ron |

Definitions expose key helpers (`TraitDef::name_key()`, `MenuItem::desc_key()`, `EventDef::choice_key(c)`,
`StoryNight::title_key()`, `Job::name_key()`, `Weekday::long_key()` …). Speaker name for a cutscene line:
regular → `regular.<id>.name`; staff archetype → its `story_name` (not translated); cast → `cast.<id>.name`.

---

## 2. Data files (`assets/data`)

All files use `#![enable(implicit_some)]`, so `Option` fields can be written without `Some(...)`.
Times are `"HH:MM"` strings → `ClockTime`. Money is CZK (`i64`/`u32`). IDs are plain strings.

| file | type | contents |
|---|---|---|
| traits.ron | `Vec<TraitDef>` | 16 traits |
| staff.ron | `StaffFile { archetypes, name_pools }` | 7 archetypes (barista, cook, porter, cashier, sound_tech, quiet_one, server), 4 name pools (cs, sk, uk, intl) |
| menu.ron | `Vec<MenuItem>` | 16 items |
| guests.ron | `Vec<GuestArchetype>` | 12 pools |
| regulars.ron | `Vec<RegularDef>` | 7 story regulars with staged arcs + the `guest_anwar` cameo |
| furniture.ron | `Vec<FurnitureDef>` | 38 pieces incl. floors / walls / window / door |
| events.ron | `Vec<EventDef>` | 41 events |
| narrator.ron | `NarratorDef` | night curve, witching hour, story act pacing |
| story.ron | `StoryDef` | 30 nights, payments, 10 notes, **2 endings** + the optional "Answer" extra, declared flags/vars/counters/items |
| cutscenes.ron | `CutsceneLib { spots, cast, cutscenes }` | 39 cutscenes (34 story + 5 cameo), 7 cast members, named map spots |
| shiftella.ron | `ShiftellaApp` | announcements, checklists, bot templates, digest / payroll / handover lines |
| trains.ron | `TrainLine` | stations Adamov → … → Mlhov nad Šeřavou, 5 night trains incl. `Rn 333` |
| difficulty.ron | `Vec<DifficultyDef>` | `cozy` (default), `classic` |
| sandbox.ron | `SandboxDef` | 5 night characters, town-name parts + stoplist, institutions, café names, options |

### Core types (`content::defs`)

* `Job { Bar, Kitchen, Register, Cleaning, Serving }`, `Job::ALL`, `type Skill = Job`. `Skills { bar, kitchen, register, cleaning, serving }` 0–10, `Skills::speed(job) = 0.5 + skill × 0.1`.
* `Need { Energy, Mood, Hunger }` (0–100, hunger 100 = full).
* `ClockTime` — ordered by **night order** (day starts 12:00): `21:50 < 22:00 < 00:00 < 06:00`. `minutes_since_open()` (22:00 = 0, matches `clock::Clock`), `from_minutes_since_open`, `is_between(from, to)` (half-open, across midnight), `open_hour_index()` (22:xx → 0 … 05:xx → 7).
* `Weather { Clear, Rain, Downpour, Fog, Storm, Snow }` (`is_wet()`), `Season`, `Weekday`, `GameMode { Story, Sandbox }`, `Modes { Story, Sandbox, Both }`.
* `Look` — render hints; list fields are *options* to pick with the seed.
* `TraitDef { id, group, conflicts, irritated_by, effects: Vec<TraitEffect> }`. `conflicts` never roll on one person (symmetric); `irritated_by` = −2 relationship/h when those colleagues share a shift.
  `TraitEffect` multipliers stack multiplicatively, adds additively. `Mood { add, when }`: continuous `Situation` → per game hour; one-shot (`ShiftStart`, `BadEvent`, `MetGuest333`, `RomanceEnded`) → once.
* `StaffArchetype` — story person (`story_name`, `story_traits`, `hire_from_night`) + generator data (`trait_pool` pick 2, `skill_spread` ±, `age`, `name_pools`). `wage` = CZK per night.
* `MenuItem { price, cost, job, prep_minutes, station (furniture tag), min_skill, unlock_night, unlock_flag, mood_bonus, needs_power, to_go }`.
* `GuestArchetype { hour_weights[8], weekday_mult[7]?, favourites, patience_minutes, spend_mult, tip_chance, group, stay_minutes, institution, uses_shiftella, review_mult, look }`.
* `FurnitureDef { sprite (render contract, `SPRITES`), category, size, price, tags (`FURNITURE_TAGS`), comfort, beauty, walkable, light_radius, seats, unlock_night, unlock_flag, unique, wear_per_night }`. Stations: `station:coffee|stove|oven|fridge`. Note: `table_socket` and `stage_corner` reuse sprites `table_round_2` and `rug`.

### Regular arcs

`RegularDef { visits, weather, orders, patience_minutes, pays, seat, absent_if, stages }`.
Arc counter = `arc.<id>` (starts 0). Each night, during a visit, check stage `current + 1`: if
`night ∈ stage.nights` and all `conditions` hold → advance, apply `effects`, play cutscenes with trigger
`ArcStage { regular, stage }` (one stage per night). `lines` are ambient bubbles at that stage.
Ending an arc well: `Effect::ArcGood(id)` sets flag `arc.<id>.good` (counted by `ArcsGoodAtLeast`).
`story_nights` (non-empty → story visits only on those nights; `visits_story_night(n)` helper), `sandbox_chance` (per-night chance in sandbox, 0 = story only), `look_preset` (render preset id); `absent_if` (any true → doesn't come); `pays: OldCoins` → 0 CZK + item `old_coin`; `pays: Song` → 0 CZK, mood boost.
The sim must track per regular: trust (0–100, `Effect::Trust`), fast-service streak, seat-free streak,
clean streak, items served (`ServedItem`).

---

## 3. Condition & Effect (`content::logic`)

A `Vec<Condition>` = **all** must hold (empty = true). `All`, `Any`, `Not` combine.
Sim state needed: `flags: HashSet<String>`, `vars: HashMap<String,String>`, `counters: HashMap<String,i32>`,
`items`, `notes_found`, per-regular arc/trust/streaks, café/crew/guest state. Every flag/var/counter/item
used in data is declared in `story.ron` (validated); `arc.<id>`, `arc.<id>.good/.bad` are implicit.

**Conditions** — calendar: `NightAtLeast/AtMost/Between`, `TimeBetween(from, to)` (night order, half-open),
`Weekday([..])`, `Weather(w)`, `Raining` (any wet), `Season`, `Mode`, `Difficulty("cozy")`.
Economy: `MoneyAtLeast/Below`, `ReputationAtLeast/Below` (0–100), `AllPaymentsMade` (every payment due so far paid).
Story: `Flag`, `NotFlag`, `VarIs(var, value)`, `CounterAtLeast/Below` (missing = 0), `NotesFoundAtLeast(n)`,
`NoteFound`, `HasItem`, `ArcAtLeast(reg, n)`, `ArcsGoodAtLeast(n)`, `RegularPresent`, `ServedFastStreak{regular, minutes, visits}`,
`SeatFreeStreak{regular, visits}`, `CleanStreak{regular, clean, visits}`, `ServedItem{regular, item}`, `TrustAtLeast(reg, n)`,
`WindowLightOn` (the window lamp is on). Café/crew: `HasFurnitureTag`, `FurnitureTagCount`, `HasFurniture(id)`, `MenuHas`,
`StaffWithTrait` / `StaffArchetype` (on shift), `StaffOnShiftAtLeast`, `StaffMoodBelow`, `StaffInspired{mood, skill}`,
`RelationshipBelow/Above` (some on-shift pair — that pair is the event's "pair"), `QueueAtLeast`, `GuestsAtLeast`,
`DirtAtLeast(0–1)`, `WearAtLeast(tag, 0–1)`, `Institution` (sandbox), `EventActive`, `EventHappenedTonight`,
`HardNightsInRow(n)`, `Chance(p)` (rolled per check).

**Effects** — durations are game minutes; `None` = rest of the night. Money: `Money(±)`, `MoneyRange(a, b)`,
`Debt(±)` (clamped ≥ 0; at 0 remaining payments count as made), `Reputation(±)`, `ReviewWeight{guest, mult, reviews}`.
Guests: `GuestRate{mul, minutes}`, `GuestRush{guest, count, within, patience_mul, spend_mul, orders}`, `GuestStay(±min)`,
`GuestSatisfaction(±)` (all present), `Tips{mul, minutes}`, `SpawnRegular`, `SpawnGuest(archetype)`.
Staff: `StaffMood(±)` (all on shift), `TraitMood(trait, ±)`, `StaffSick` (random on-shift person out tonight),
`StaffWalkout` (lowest mood leaves), `PairMood(±)` (the triggering pair), `Relationship{add, minutes}` (per hour while
active; once if `None`), `WorkSpeed{job, mul, minutes}`, `SkillBoost{add, nights}` (best skill of the triggering person).
Café: `Dirt(±)`, `DirtRate{mul, minutes}`, `PowerOutage{minutes: (min, max)}` (stations with power stop, `needs_power:false` items still sell),
`StationDown{tag, minutes}` (`None` = until repaired), `StationSlow{tag, mul}` (until repaired), `MenuUnavailable{item, until}`,
`LoseStock(0–1)`, `BlockTile`, `ClockStop{seconds}` (real seconds; freezes everything except `SeesUncanny` staff), `SetWeather`.
Story: `SetFlag`, `ClearFlag`, `SetVar`, `AddCounter`, `GiveItem`, `TakeItem`, `FindNote` (marks found + shows card),
`AdvanceArc`, `ArcGood`, `ArcBad`, `Trust(reg, ±)`, `UnlockMenu`, `UnlockFurniture`.
Chaining: `StartEvent`, `EventWeight{event, mul, nights}` (0 disables), `StartCutscene`, `Toast(key)`, `Bot(template)`.
Control: `Maybe{chance, effects}`, `Later{minutes, effects}` (dropped at night end), `If{conditions, then, otherwise}` (evaluated when run).

`Condition::visit_refs` / `Effect::visit_refs` enumerate referenced ids (`Ref`) — used by validation.

---

## 4. Events & narrator

`EventDef { kind (Good/Bad/Neutral/Uncanny), severity 1–3, trigger, weight, cooldown (nights), modes, min_night, conditions, effects, choices, default_choice }`.
* `Random` — narrator rolls once per game minute: `p = character.base_rate × narrator.curve_at(t) × act_mult(night)` (sandbox: `NightCharacter`; story: use the `calm` character + `story_acts`). Pick by `weight` among events whose conditions hold, not on cooldown, `severity ≤ act.max_severity`, tension budget (`severity×10`, Good subtracts). Uncanny × `witching_hour` multiplier between its times. After a Bad event: `breath_minutes` without Bad. `calm_before_payment`: no severity-3 Bad the night before a payment.
* `Auto` — fired by the sim when conditions become true (checked each game minute, once per night unless cooldown 0): `staff_walkout`, `broken_dishes` (from `TraitEffect::Mishap`), `clock_333`, `cat_stays`, `lonely_night`.
* `Scripted` — only via story beats / `StartEvent` (`mirek_accident`, `storm_blackout`). Beats may also fire Random events directly.
* Firing: apply `effects`, show a toast (`event.<id>.title/.text`, colour `kind.toast_category()`). With `choices`, the toast shows buttons (`event.<id>.choice.<c>`, work while paused); on click apply that choice's `effects` + `set_flag`, collapse to `event.<id>.result.<c>`. On timeout apply `default_choice`.

## 5. Story

* 30 `StoryNight { act, weekday, weather?, beats }`. `Beat { at, what, conditions }`, `at < 22:00` = day phase. `BeatKind`: `Line{speaker, text_key}` (bubble), `Event(id)`, `Note(id)`, `PaymentDue` (collect `payment_on(night)` at that time), `Train(id)` (board announcement with `train.announce.arrival`), `Hint(key)`, `Bot(template)`, `Weather`, `Apply(effects)`.
* Payments: nights 7/14/21/28 × 15 000 + night 30 × 30 000 (final) = debt 90 000. Missing one: apply `difficulty.missed_payment_effects`; Classic closes after `close_after_missed` (2) or a missed final (`final_payment_missed` flag) → Last Train ending early.
* Notes: `found_when` (empty = only via `FindNote` / `Beat::Note` / `ShowNote`) checked each game minute + at night end.
* **Endings (exactly two)**, checked in `order` at the end of night 30 (or on a Classic closure): `last_train` (franchise accepted, or Classic payment failure, or reputation < 20 with no good arc) → otherwise `light` (fallback). Each plays its `cutscene` (trigger `Ending(id)`), which branches with `If` steps. `variants` (first match) name the outcome for the summary screen: last_train = `sold` / `farewell` / `closed`; light = `full` / `quiet`.
* `ending_extra` "Answer" (letter delivered + ≥ 8 notes + offer not accepted) is **not an ending**: both ending cutscenes `Call` the `answer` cutscene when it holds; show `ending.extra.answer.title` as an epilogue badge.
* State: vars `letter` (hidden → confessed → with_player / with_mirek / burned / persuaded → delivered / delivered_by_mirek), `offer` (none/pending/accepted/rejected), `answer_333`; counters `missed_payments`, `payments_made`, `cat_fed_nights`, `coins_collected`, `filip_kicked_out`. The sim owns `payments_made`/`missed_payments` bookkeeping.

## 6. Cutscenes

`CutsceneDef { id, trigger, conditions, skippable, repeat, steps }` — plays once per save unless `repeat`.
Triggers: `NightStart(n)` (before the night), `NightEnd(n)` (after 06:00, before the summary), `At{night, time}`,
`FlagSet(flag)`, `ArcStage{regular, stage}`, `Ending(id)`, `Manual` (only via `Call` / `StartCutscene`).

Steps run in order; dialogue steps wait for a click, `Camera`/`Music`/`Sfx`/`Letterbox` don't block:
`Narration{text_key}` (no speaker) · `Line{speaker, expression, text_key}` (portrait + name; `Expression` Neutral/Smile/Sad/Surprised/Thinking/Laugh/Tired) ·
`Choice{options: [ChoiceOption{label_key, effects, set_flag, conditions}]}` (blocking inside the scene; options whose conditions fail are hidden) ·
`Camera{focus: Tile(x,y) | Spot(name) | Actor(id) | Overview, zoom}` · `ActorEnter{actor, to_tile}` (appears at the door, walks) · `ActorMove` · `ActorExit` (walks there, disappears) ·
`Wait{seconds}` · `Fade{to_black, seconds}` · `Music{track: Option}` (`MUSIC_TRACKS`; `None` fades out) · `Sfx{id}` (`SFX_IDS`) · `TrainPass` · `Letterbox{on}` ·
`ShowNote{note_id}` (also marks it found) · `If{conditions, then, otherwise}` · `Call{cutscene}` · `Apply{effects}`.
`Place`/`Focus` spots are named tiles on the default café map (`cutscenes.ron` → `spots`); move them if the map changes.
Actors/speakers: regular ids, staff archetype ids, cast ids (`player`, `grandma`, `bot`, `announcer`, `sindelar`, `landlord`, `jana`), or guest archetype ids (extras).
Skipping (`skippable: true`) jumps to the end but still applies every `Apply` step and runs `ShowNote`; a skipped `Choice` is not answered
(scenes whose choices matter are `skippable: false`). If an actor a scene enters is already in the café (a regular on their
visit), reuse that actor instead of spawning a second one.

## 7. Shiftella app, trains, difficulty, sandbox

* `ShiftellaApp`: `announcements` (story ones post at `night`, others are templates with `placeholders`), `checklists` (Opening/Closing; sim ticks them as work items; `done`/`missed` effects at 06:00), `bot` templates (`BotKind`, declared `placeholders` are validated against every language; `actions` are message buttons — **never clock-in/out**, validation rejects ids containing "clock"), `digest`, `payroll` (podklady pro mzdy, `row` has `{number}` = osobní číslo), `handover` (předávka) lines.
* `TrainLine`: stations in line order (names never translated), `home = "mlhov"`, trains at 22:47, 00:15, 01:52, 03:33 (`Rn 333`, `Phantom`, announced only when `announce_if` holds — night 30 with the light on), 05:20. `guests` = passengers who come in, of archetype `guest`.
* `DifficultyDef`: `cozy` (default: Soft payments, no game over, ×1.25 income, patient guests) / `classic` (Hard payments, closes after 2 missed, sandbox bankruptcy after 3 nights in the red).
* `SandboxDef`: night characters `quiet, calm (default), changeable, stormy, chaotic` (slider steps), town names `[prefix] root+suffix [nad river]` minus `stoplist`, institutions (enable guest pools), start money options, rent range, café names, checkbox defaults.

## 8. Validation (`Content::validate`)

Unique ids; trait conflict symmetry; staff traits/name pools; menu stations exist in furniture; sprites in the render
contract (and every contract sprite used); guest hour/weekday arrays; every `Ref` in conditions/effects/choices/beats/
cutscenes resolves (menu, guest, regular + stage, event, cutscene, note, trait, staff, furniture/tag, declared flag/var
value/counter/item, difficulty, institution, bot template, text key); story nights exactly 1..=30 with sorted beats and
matching `PaymentDue`; payments sum to the debt; notes numbered 1..10; exactly 2 endings, last one is the fallback and its
cutscene has trigger `Ending(id)`; cutscene speakers/actors/spots/music/sfx; every required i18n key in all four
languages; placeholder parity.

## 9. Cameo: Anwar

`guest_anwar` ("Anwar", render preset `anwar`) is a loving reference to the developer of the real Shiftella app.
**Flavour only:** one arc stage (ambient lines), no flags, effects, choices or gating, not in the endings.
Story: nights 3, 9, 16, 24, 29 (23:00–02:00, socket table, oat flat white / medovník, pays like a normal guest) with five tiny
choice-free cutscenes `anwar_sticker`, `anwar_codenames`, `anwar_bugfix`, `anwar_game`, `anwar_cake` (`At` triggers).
Sandbox: rare drop-in (`sandbox_chance: 0.05`). Credits line: `ui.ending.credits_thanks`. Texts: `assets/i18n/src/cameo_anwar.tr`.
