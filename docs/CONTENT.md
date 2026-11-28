# Shiftella — content & i18n reference

For the sim (wave 2) and integration (wave 3) agents. Code: `src/content/**`, `src/i18n/**`.
Data: `assets/data/*.ron`. Text: `assets/i18n/src/*.tr` → generated `assets/i18n/{cs,en,uk,ru}.ron`.

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
| `Lang { Cs, En, Uk, Ru }`, `Lang::ALL` | picker order CZ, EN, UA, RU. `code()` → `cs/en/uk/ru`, `short_label()` → `CZ/EN/UA/RU`, `native_name()`, `from_code("ua")`. Serde-serializable (for settings); a saved `Sk` and the code `sk` read as `Cs`. |
| `I18n::tr(lang, key) -> &str` | fallback `lang → en → key` |
| `I18n::trf(lang, key, &[("name", "Mirek")])` | fills `{name}` |
| `I18n::trn(lang, "ui.count.guests", n, &[])` | plural: picks `.one/.few/.many`, sets `{n}` |
| `plural_form(lang, n)` | en: 1/other · cs: 1 / 2–4 / other · uk, ru: East Slavic rules (21 → one, 22 → few, 11–14 → many) |
| `format_money(lang, czk)` | `1 250 Kč` (cs/uk/ru, no-break spaces; decimal comma) · `1,250 CZK` (en); negatives use `−` |
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
uk Сова
ru Сова

cast.bot.name
* @shiftella_bot            ← "*" = same text in all four
```

`\n` inside a text = line break. The train-line stations (Adamov, Beroun, Cheb, Děčín, Ejpovice,
Frýdlant) stay Czech in Latin script on boards, tickets, train names and station announcements, like
the `{from}`/`{to}` the data fills in. In uk/ru narration and dialogue they are written in Cyrillic
and declined like any noun («до Бероуна», «із Фрідланта», «из Адамова»), never Latin in a case
position. Prague and Holešovice are written the local way in Cyrillic (Прага-Голешовиці / Прага-Голешовице).
Forms of address in uk/ru: Mirek, Vera, Lori, Hana and Filip say ти/ты to Karolina; Radek, the
landlord and Šindelář say ви/вы; Karolina says ви/вы to Mirek and Vera. «Ваш» meaning the café
itself («ваш фирменный») is fine either way.
Person names are transliterated in Cyrillic prose (Мірек, Ліда / Мирек, Лида); record, band and cat
names stay as written. Lines around a `{name}` avoid case endings and gendered past tenses. The player is **Karolina**, Lida's
granddaughter: feminine forms are fine where a language needs them (brief §1).

**Languages** (owner, 2026-11-26): CZ / EN / UA / **RU** (`cs`, `en`, `uk`, `ru`); Slovak was dropped
in T9. Every key has real text in all four; `*` lines are only for names and symbols.

**Hands-on text** lives in `assets/i18n/src/handson_*.tr` (`core`, `wishes`, `recipes`, `moments`,
`world`, `app`, `ui`, `sim`). `minigame.*` and `latte.*` keys belong to the
mini-game agent (`minigame.tr`) — never define them elsewhere (duplicate keys break the build).
The preparation half hour (21:30–22:00: the door sign, the "Open now" chip, the 21:30 / 22:00
toasts, the `Nex 9047` freight) lives in `prep.tr` (`prep.*`).

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
| cutscenes.ron | `CutsceneLib { spots, cast, cutscenes }` | 38 cutscenes (36 story + 2 Anwar talks), 7 cast members, named map spots |
| shiftella.ron | `ShiftellaApp` | announcements, checklists, bot templates, digest / payroll / handover lines |
| trains.ron | `TrainLine` | stations Adamov → … → Praha-Holešovice (home), 5 night trains incl. `Rn 333` + the `Nex 9047` freight at 21:47 (a train before 22:00 only passes by in the preparation half hour, nobody gets off) |
| difficulty.ron | `Vec<DifficultyDef>` | `cozy` (default), `classic` |
| sandbox.ron | `SandboxDef` | 5 night characters, town-name parts + stoplist, institutions, café names, options |
| handson.ron | `HandsonTuning` | hands-on tuning: tick length, overlay scale, helper speed/quality, wish chance, wait clock, rush/rhythm, serve formulas, lull, moment budget, experiments, capacity, quiet hour, rozpis puzzle (§10.1) |
| recipes.ron | `RecipesFile` | 28 pantry ingredients, 28 recipes (22 MUST + 6 SHOULD), experiment rules (§10.2) |
| wishes.ron | `WishesFile` | 44 wishes (27 generic + 17 personal), reaction lines (§10.3) |
| moments.ron | `Vec<MomentDef>` | 25 night moments: 10 MUST + `m_hang_garlands` (MUST) + 14 SHOULD (§10.4) |
| minigames.ron | `MiniGamesFile` | tuning per mini-game kind and variant, quality tiers, 11 latte patterns (§10.5) |
| records.ron | `RecordsFile` | grandma's jukebox: house playlist, 6 records (3 starters), hall-mood effects, music rules (§10.6) |
| seasons.ron | `SeasonsFile` | 4 story phases + 2 sandbox phases, sandbox mapping (§10.7) |
| decor.ron | `DecorFile` | 5 decor themes, the coziness formula (§10.7) |
| journal.ron | `JournalFile` | polaroid highlight rules, captioned scenes and moments (§10.8) |
| tutorial.ron | `Vec<TutorialStep>` | 26 coach marks, nights 1–4 + 8 + 21 (§10.9) |
| extras.ron | `ExtrasFile` | station cat levels, tip-jar goals, 40 napkins, rituals, mug lines (§10.10) |
| guest_names.ron | `sim::names` (compiled in) | strangers: 30 + 30 first names (Latin + Cyrillic, optional Russian spelling), per archetype the share of women / elderly guests, tops, extras and whole-phrase descriptions (`needs` a visible feature); texts in `guest_names.tr` (`stranger.<phrase>.<f|m>[.short]`, `stranger.intro.*`, `stranger.party`, `stranger.log.met`) |
| stories.ron | `sim::stories` (compiled in) | 22 little stories of strangers (filters by archetype / persona / party / time / weather; 2–3 answers each, `good` ones with `Fx`), the storm night (story night 29, sandbox chance, times, blankets / soup, the 03:00 talks); texts in `night_stories.tr` (`story.<id>.text/.opt.<a>/.react.<a>/.log`, `journal.caption.story.<id>`, `storm.*`, `app.bot.storm_cancel`) |

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
* Hands-on additions (all `#[serde(default)]`): `StaffArchetype.default_duty: Option<Duty>` and
  `prefs: Vec<Pref>`; `GuestArchetype.music_likes / music_dislikes` (record ids) and
  `weather_mult: Vec<(Weather, f32)>` (`weather_mul(w)`); `FurnitureDef.needs_power` and `skin`
  (render variant on a contract sprite — draw `sprite` when the skin is unknown).
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
`WindowLightOn` (the window lamp is on), `TrainNear(min)`. Café/crew: `HasFurnitureTag`, `FurnitureTagCount`, `HasFurniture(id)`, `MenuHas`,
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
* `ending_extra` "Answer" (letter delivered + offer not accepted) is **not an ending**: both ending cutscenes `Call` the `answer` cutscene when it holds; show `ending.extra.answer.title` as an epilogue badge. Eight notes or more add Lida's postscript inside the scene (a reward, not a lock).
* State: vars `letter` (hidden → confessed → with_player / with_mirek / burned / persuaded → delivered / delivered_by_mirek; after the sale `bench`: Mirek leaves it on the platform bench, shown in `last_333_sold`), `offer` (none/pending/accepted/rejected), `answer_333`; counters `missed_payments`, `payments_made`, `cat_fed_nights`, `coins_collected`, `filip_kicked_out`. The sim owns `payments_made`/`missed_payments` bookkeeping.

## 6. Cutscenes

`CutsceneDef { id, trigger, conditions, skippable, repeat, steps }` — plays once per save unless `repeat`.
Triggers: `NightStart(n)` (before the night), `NightEnd(n)` (after 06:00, before the summary), `At{night, time}`,
`FlagSet(flag)`, `ArcStage{regular, stage}`, `Ending(id)`, `Talk{regular, night}` (see §9), `Manual` (only via `Call` / `StartCutscene`).

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
languages; placeholder parity. Hands-on: see §10.12.

## 9. Talking to regulars & the Anwar cameo

* **Ambient bubble (hover / click):** `RegularDef.ambient_lines: Vec<AmbientLine { key, when: Vec<Condition> }>`.
  Pick randomly among lines whose non-empty `when` holds; if none, among lines with empty `when` (fallbacks).
  Shown as a small speech bubble, no cutscene. (Arc-stage `lines` remain the generic per-stage bubbles.)
* **Talk:** `CutsceneTrigger::Talk { regular, night: Option<u32> }`. Offer a "Talk" action on the regular while an
  unplayed talk cutscene matches (`night: None` = any night); choosing it plays the cutscene. Usable for any regular.
* **Visit schedule:** `story_nights` / `sandbox_nights` (non-empty → only on those nights; `sandbox_visit_rule(n)`),
  `sandbox_chance` (random per-night roll when `sandbox_nights` is empty; 0 = never), `look_preset` (render preset id).
* Condition `TrainNear(minutes)`: a train arrives within ±N minutes.

`guest_anwar` ("Anwar", render preset `anwar`) is a quiet guest with a laptop — identity deliberately understated.
Exactly two visits per game: nights 9 and 24 (story and sandbox), 23:00–02:00, socket table, oat flat white, pays, leaves.
8 ambient lines (rain, fog, train, midnight, late, busy, quiet, coffee fallback) and talk cutscenes `anwar_talk_9`,
`anwar_talk_24` (no choices, flags or effects; not in the endings). Credits line: `ui.ending.credits_thanks`.
Texts: `assets/i18n/src/cameo_anwar.tr`.

---

## 10. Hands-on content (`content::handson`, design/handson.md)

Everything is loaded into `content.handson: HandsOn` (`tuning, recipes, wishes, moments, minigames,
records, seasons, decor, journal, tutorial, extras`) and validated with the rest. Lookups:
`recipe(id)`, `ingredient(id)`, `wish(id)`, `moment(id)`, `record(id)`, `theme(id)`, `phase(id)`,
`tutorial_step(id)`, `recipes_for_item(item)`, `starter_recipes()`, `wishes_of(regular)`,
`best_matches(wish, available, ctx, n)`, `summary()`. Shared enums: `Tier { Must, Should, Later }`
(SHOULD data ships; the sim/UI may ignore it until the feature exists) and
`Unlock { Starter, When(Vec<Cond>), Granted }` (ingredients, records, themes: `When` is checked at day
start and each game minute and stays unlocked; `Granted` = only via an `Unlock*` effect;
`sandbox_night` = unlocked from that night at the latest in sandbox).

### 10.0 `Cond` and `Fx` — the hands-on scripting extension (`handson::logic`)

The sim matches `Condition` / `Effect` exhaustively, so new variants live in **extension enums** and the
new data files use two union types: `Cond = Core(Condition) | X(XCondition)` and
`Fx = Core(Effect) | X(XEffect)`. In RON they read as one enum (the variant name picks the side; a
serde shim replays the name), e.g. `[Raining, SeasonPhase("frost"), Any([Weather(Snow), QuietHour])]`.
The combinators `All/Any/Not` (conditions) and `Maybe/Later/If` (effects) are on the extension side so
they can mix both. Evaluate `Core` with the existing code; `X` is new work. `visit_refs` → `XRef`
(`Core(Ref)`, `Recipe`, `Ingredient`, `Moment`, `Record`, `Theme`, `SeasonPhase`, `Latte`,
`MusicTrack`, `Napkin`); `walk_fx` recurses into nested effects. Existing files keep the core types.

**XCondition:** `RecipeKnown(r)`, `ServedRecipe { regular, recipe }`, `WishMatchStreak { regular, min,
visits }` (last N visits each got a Karolina-made item with match ≥ min), `HallMood(m)`, `QuietHour`,
`Blackout`, `StaffLowEnergy(e)`, `MomentResolvedEver(m)`, `IngredientUnlocked(i)`, `RecordUnlocked(r)`,
`SeasonPhase(p)`, `CozinessAtLeast(n)`, `HiredArchetype(s)` (employed, on shift or not),
`ShiftsWorked(s, n)`, `SharedShifts { a, b, shifts }` (≥ 4 h together), `TipsLifetimeAtLeast(czk)`,
`NapkinsAtLeast(n)`, `ThemeOwned(t)`.

**XEffect** (durations in game minutes, `None` = rest of the night; "target" = the agent a moment is
anchored to, or the subject of a handover note / announcement):
* flow — `Maybe`, `Later` (dropped at night end), `If`, `NextNight(fx)` (applied when the next night starts);
* knowledge — `LearnRecipe`, `UnlockIngredient`, `RevealIngredient(recipe)` (one more ingredient on the
  silhouette), `UnlockRecord`, `UnlockTheme` (owned for free), `UnlockLatte`;
* music — `HallMood { mood, minutes }`, `PlayRequestedRecord`, `RadioTrack { track, volume, minutes }`,
  `StreetMusic(min)`;
* presentation — `SpawnMoment`, `CameraHint(Anchor|Window|Door|Counter|Jukebox|Street)`, `LogTrain`
  (LATER, may no-op), `Highlight(kind)` (journal candidate), `ThankYouNote(Option<id>)`;
* target — `TargetTrust`, `TargetSatisfaction`, `TargetStay(±min)`, `TargetLeave` (content),
  `TargetStaffMood`, `TargetStaffEnergy`, `TargetStaffBreak`, `TargetRelationship` (with Karolina),
  `ArchetypeMood { staff, add }`;
* hall — `GuestSatisfactionNear { add, tiles }`, `WishBoost { tags, mul, minutes }`,
  `Coziness { add, minutes }`, `Patience { mul, minutes }`, `LightCandles`, `PlaceBucket`, `EmptyBucket`,
  `WindowOpen(min)`, `Tip { min, max }` (into the tip jar);
* rituals / handover — `QuickQuality { add, minutes }`, `SlowFirstItems { job, count, mul }`,
  `StockoutRisk { chance, minutes }`, `StockChecked`, `RestockFirst`, `Wear { tag, add }`, `CashUp`,
  `ChecklistSpeed { list, mul }`, `HelpersReady { minutes }`, `RememberRegular { trust }`,
  `FixTable { beauty }`, `ItemSpotlight { prep_mul }`, `SwapBoard`.

### 10.1 Tuning (`handson.ron` → `HandsonTuning`)

`real_seconds_per_tick_x1` (0.15 → 12-minute nights), `prep_tick_scale` (0.7: ticks of the
21:30–22:00 preparation half hour run at this share of a tick), `overlay_scale { cozy, classic }`,
`player_walk_mul`, `player_work_mul`, `helper_speed`, `helper_quality { base, per_skill, max }`
(`at(skill)`), `quick_quality`, `practised_quick_quality`, `practised_after` (+ `practised_min_quality`),
`by_hand_work_share`, `by_hand_streak_max`, `match_tiers { perfect, good, okay }`,
`wish_chance` (`wish_chance_on(night)`), `sandbox_wish_chance`, `fair_wishes { boost_at, boost_mul,
skip_below }`, `usual_trust`, `hand_off_match_cap`, `assist_wishes_after`, `claimed_wait_rate`,
`cozy_rush_wait_rate`, `cozy_forgiveness { refill, runs, leave_satisfaction }`,
`cozy_regular_patience_floor`, `regular_announce_minutes`, `greeting`, `rush { parties, window_minutes,
minutes, from_night }`, `rhythm`, `serve { satisfaction_*, trust_*, tip_base, tip_span,
classic_not_quite, latte_neighbour }`, `lull_minutes`, `quiet_hour_lull_minutes`,
`lull_walk_in (mul, minutes, from_night)`, `moments_per_night` (`moment_budget(act)`),
`moment_gap_minutes`, `moments_max_per_night`, `experiment_cost`, `experiment_free_until_night`,
`experiment_minutes`, `margin_note_every`, `thanks_chance`, `capacity { base, after_nights,
skill_bonus_at, trait_bonus, max }`, `player_energy`, `quiet_hour { from, to, lamp_dim, rain_volume_mul,
guest_rate_mul, wish_share, toast }`, and `rozpis` (§10.11).

### 10.2 Recipes and the pantry (`recipes.ron`)

* `Taste` (12): Warm, Cold, Sweet, Bitter, Strong, Mild, Comforting, Fancy, Quick, Filling, Spiced,
  Fresh — `taste.<id>`. `Family` (9) drives hints — `ingredient.family.<id>` is an adjective used
  inside sentences ("swap the {family} one", "it wants something {family}").
* `IngredientDef { id, family, shelf: Bar|Kitchen, unlock, sandbox_night }` — `ingredient.<id>.name`.
  Mushrooms unlock with the varenyky (night 8) *or* Vera's kulajda card (deviation from the design
  table, so the known varenyky never show a dim jar).
* `MiniGameKind` — **canonical** `MilkFroth, LatteArt, PourOver, TeaSteep, Plate` (MUST) +
  `SoupStir, MulledSpice` (SHOULD, not built: their recipes carry `fallback_minigame` +
  `fallback_variant`; `RecipeDef::playable_minigame(should_games)` picks). `src/minigame::MiniGameKind`
  has the same variant names. `title_key()` = `minigame.<id>.title` (owned by `minigame.tr`).
* `RecipeDef { id, base_item, price_add, cost_add, tags, minigame, variants, fallback_minigame,
  fallback_variant, ingredients (1–3, unique set), unlock, experimentable (default true), hint, gated_hint,
  revealed, art_finish, tier }` — keys `recipe.<id>.name/.desc/.lore` (+ `.teach` for `Taught`).
  `RecipeUnlock { Starter, WithMenu (grandma's card when the base item unlocks), Taught { by, when }
  (learned while `by` — a regular in the café or the staff archetype on shift — is present and `when`
  holds), Story { when }, Experiment }`. `book_tab(&menu_item)` → Drinks / Food / Specials.
* `experiment(&file, chosen, known) -> ExperimentOutcome { Known, Discovered, Gated, Close { recipe,
  kind: Swap|Extra|Missing, family }, Nothing }` implements §4.4 (ties: most overlap, fewest
  ingredients, data order). Keys: `experiment.known/.discovered` `{recipe}`, `.close.swap/.missing`
  `{family}`, `.close.extra`, `.nothing.1..6`, `.cat_sniffs`.

### 10.3 Wishes (`wishes.ron`)

`WishDef { id, want, avoid, any, favourite (recipes), favourite_items (menu), who (archetypes or
regulars), when: Vec<Cond>, min_party, weight, weight_mods: [(when, mul)], kind: Any|Drink|Food,
strict, latte, max_per_night }` — key `wish.<id>`; personal ids are `<regular>.<id>` with
`who: [<regular>]` and replace `RegularDef.orders` for Karolina (orders stay the hand-off fallback).
`uses_per_night()` defaults to 2 (personal 1). `match_score(wish, recipe, MatchContext {
on_specials_board })` = §3.2; `MatchTier::of(score, &tuning.match_tiers)`. Reactions: `react.<tier>.1..5`
and `react.<regular>.<tier>` for `reactions.regulars` (tiers `perfect/good/okay/not_quite`).
Test guarantee: every wish has a recipe ≥ 0.6 in the full book and ≥ 0.3 among the MUST recipes.
`r_signature` (`handson::SIGNATURE_RECIPE`) may be named in `favourite` and `RecipeKnown` although
it isn't in `recipes.ron`: it is the café's own drink, composed in the game and kept in the world
(v0.2 package I, `sim::signature`; data `signature.ron`, compiled in; text `signature_photo.tr`).
The `house_special` wishes ask for it by name — their `wish.*` lines are fallbacks, the game fills
the name into `signature.wish.<id or regular>`.

### 10.4 Moments (`moments.ron`)

`MomentDef { id, kind, tier, anchor, conditions, from_event, weight (10), per_night_max (1),
max_per_game, cooldown_nights, expires_minutes (25), quiet, follow_up, options, on_expire }`.
`MomentAnchor { Tile, Tag(furniture tag), Furniture(id), Door, Street, Counter, Leak, Bucket,
Agent(RegularWantsRecord | StaffLowEnergy(e) | Guest(GuestFilter { archetypes, regulars,
not_regulars, at_door, seated_minutes, fed, party_min, accessory })) }` — an agent anchor is also a
condition. A `Furniture` anchor that isn't placed falls back to `Counter`.
`MomentOption { id, cost (display), effects: Vec<Fx>, interaction, show_if, within_minutes }`;
`Interaction { Swipe, Hold(s), TrainPicker { trains }, LightEachTable, HangSpots(n), QuickBrews {
recipe, count }, ByHandFavourite }`. Keys `moment.<id>.title/.text/.opt.<o>/.result.<o>` and
`.expired` when `on_expire` is set (plus toasts named in effects).
`from_event` replaces an event's choice toast: the event still applies its base effects, then spawns the
moment; `on_expire` holds the event's old default choice. **The events keep their `choices` in
`events.ron` until the sim's moment director exists** — then ignore `choices` of events a moment names.

### 10.5 Mini-game tuning (`minigames.ron`)

`common { skip_quality, relaxed_tolerance_mul, relaxed_floor, seconds, toast_seconds }`,
`quality_tiers` (`label(q)` → `beautiful/lovely/good/homely`, keys `minigame.quality.*`), rows per kind:
`milk_froth [MilkFrothTuning { variant, t_target, tol, foam, foam_tol, t_weight, tap, seasonal_dust }]`,
`latte_art { samples, cover_px, stray_px, coverage_weight, wobble_window_s, variants }`,
`pour_over [{ variant, bpm, bloom_hold_s, bloom_s, rev_per_s, beat_window_s, weights, stirs, settle_s,
slow_pour, dust_tap }]`, `tea_steep [{ variant, target, tol, lemon, lemon_px, sugar, milk_hold,
tea_kind }]`, `plate [{ variant, dish, slot_px, keyboard_score, ghost_s, gesture, advent_twig }]`,
`soup_stir`, `mulled_spice`, `latte_patterns [{ id, from_night, when, recipe, granted, tier }]`.
Field names match `src/minigame` (`FrothTuning`, `PourTuning`, `TeaTuning`); `tea_kind` ∈ `TEA_KINDS`,
`dish` ∈ `PLATE_DISHES`, latte pattern ids = `LatteShape` ids. Variant ids `src/minigame` doesn't know
yet (`encore`, `svarak`, `chai` …) fall back to its defaults.

### 10.6 The jukebox (`records.ron`)

`RecordDef { id, mood: HallMood, track (∈ MUSIC_TRACKS), seconds (ffprobe), unlock, sandbox_night,
liked_by, disliked_by, sleeve { color, motif } }` — keys `record.<id>.title/.artist`, non-starters
`.hint/.source`, bubbles `record.<id>.liked.<regular>` / `.disliked.<regular>`. `house { calm, rush,
quiet, quiet_volume, day }`, `rules { like_minutes, like_trust, request_trust, dislike_satisfaction,
archetype_like, archetype_dislike, crossfade_s, needle_volume, sandbox_nights }`,
`moods [MoodEffect { mood, tips_mul, helper_speed_mul, stay_mul, coziness, wish_mul, satisfaction,
patience, moment_weight, elder_trust }]`. `HallMood { Lively, Calm, Dreamy, Tender, Nostalgic }`
(`hall_mood.<id>`). `content::MUSIC_TRACKS` now lists the six `jukebox_*` tracks (files are not in
git). The jukebox furniture is `unique`, tagged `grandma`, `needs_power`.

### 10.7 Seasons, themes, coziness (`seasons.ron`, `decor.ron`)

* Story calendar (`story.ron` → `calendar { start: (year, month, day), days: [(id, night)] }`):
  night 1 = Mon 7 Nov 2039 … night 30 = Tue 6 Dec 2039; named days `calendar.<id>` (1st Advent Sunday 21,
  first snow 27, 2nd Advent 28, Mikuláš 29, the last night 30). `date_of_night(n)`, `days_on(n)`,
  `Date::weekday()`; validation checks every night's weekday.
* `PhaseDef { id, story_nights, visuals: SeasonVisuals { leaves, frost, breath, garlands (allowed),
  wreath, snow, petals, sign_star, steam }, wish_mul, latte, moments, moment_weight, specials }` —
  `season.<id>.name/.toast`; `story_phase(night)`, `sandbox_phase(season, night_of_season)`. Story
  phases cover 1..30 exactly once.
* `ThemeDef { id, default, unlock, sandbox_night, price, coziness (0–10), coziness_in [(phase, c)],
  wall_pattern, tints { wall, wall_trim, floor, textile, lampshade, lamp_warmth }, accent_tag
  (`theme:<id>`), perks: BeautyPerLamp | RegularTrustOnEnter | ArchetypeSatisfaction | MissedBy }` —
  `theme.<id>.name/.desc`. Furniture tags `theme:<id>` are valid tags.
* `CozinessDef { weights [(CozyComponent, w)] (sum 100), window_light_bonus, beauty_per_tile,
  comfort_div, dishes_penalty, music { off, house, record }, theme_div, accent_bonus, accent_max,
  touches [(CatLevel(n) | GarlandsInSeason | Plants(n) | Furniture(id), points)], mood_bonus, effects {
  satisfaction_center, satisfaction_per, patience, stay, tips (CozyCurve { base, div }) }, levels,
  every_minutes }` — `level(coz)` 1..5 (`atmo.level.<n>`), components `atmo.component.<c>` /
  `atmo.tip.<c>`.

### 10.8 Journal (`journal.ron`)

`HighlightKind` (17; `caption_key(sub)`): `journal.caption.<id>`; `ArcBeat` →
`journal.caption.arc.<regular>.<stage>` (every stage ≥ 1, not the cameo); `StoryScene` →
`journal.caption.scene.<cutscene>` (`scenes`); `MomentWarm` → `journal.caption.moment.<id>` (`moments`;
validated against the moments that emit `Highlight(MomentWarm)`). `JournalFile { keep_per_night,
repeat_kind_penalty, arc_good_bonus, snap_tiles, fallback { at, focus_tag }, full_house_share,
beautiful_latte_quality, cat_level_min, rules [{ kind, score, repeat_score, placeholders }] }`.
UI keys: `journal.title/.empty/.keep/.shake/.night/.date {day} {month} {year}/.month.1..12`.

### 10.9 Tutorial (`tutorial.ron`)

`TutorialStep { id, night, trigger: At(time) | Event(TutEvent) | RegularArrives(r) | After(step),
anchor: Guest(First | Regular(r)) | Widget(id) | Tag | Furniture | Moment | Free, done_when:
Event(TutEvent) | GotIt { seconds }, optional, voice: Karolina | Lida, on_show: Vec<Fx> }` — key
`tutorial.<id>`. `At` times 21:30–21:59 fall in the night's preparation half hour (doors closed),
earlier ones in the day phase. `After` fires when the named step is done or dismissed (an optional one also after a
few unanswered seconds). `TUTORIAL_WIDGETS` is the list of UI anchor ids the UI must expose.
`TutEvent` is the list of sim events / UI actions steps wait for.

### 10.10 Extras (`extras.ron`)

`cat { counter: "cat_affection", max, levels [(level, perks: Doormat | Inside | WindowSeat |
WorksHere | Naming { moment } | Latte)], names }` (`cat.level.<n>`, `cat.name.<id>`, var `cat_name`),
`tip_goals [{ lifetime, choices: [furniture; 2] }]` (if one is owned, give the other),
`napkins [{ id, who, doodle }]` (`thanks.<id>`; 30 generic + 10 personal), `napkin_rewards`,
`ritual_rules`, `rituals [{ regular, response }]` (`ritual.<regular>`), `mug_lines`
(`karolina.mug.<n>`). Reward furniture (`fairy_lights`, `lace_tablecloth`, `record_shelf`, `cat_bed`,
`brass_bell`, `napkin_wall`) and theme accents (`lamp_brass`, `mosaic_rug`, `enamel_sign`, `led_strip`)
sit behind the never-set flag `reward_decor`: only `UnlockFurniture` reveals them.

### 10.11 Duties, preferences, the rozpis puzzle

* `Duty { Dishes, Floor, Register, Bar, Kitchen, Serving }` — **canonical** (`duty.<id>.name/.desc`,
  digest credit `app.digest.helper_<duty>` `{name} {n}`); `job()`, `Duty::for_job(job)`.
* `Pref { kind, strength: Need|Wish, quote (key) }`, `PrefKind { DayOff(day), ShiftPref(Early|Late|Full),
  WeekendOff, NoBackToBack, MaxNights(n), MinNights(n), WorksWith(staff), AvoidWith(staff) }` — chip
  `pref.kind.<id>` (`{day}`, `{shift}`, `{n}`, `{name}`). `PreferredShift` maps to the sim's `Shift`.
* `handson.ron → rozpis: RozpisTuning` — wellbeing penalties/bonuses, `rings` (`rozpis.ring.<id>`,
  `ring(wb)`), `stdev_weight`, `week_icons` (`rozpis.week.1..4`), week effects, auto-fill costs,
  `requests: RequestRules`, `pref_rules { from_traits, random, need_chance, max }` (generated quotes
  `pref.quote.<id>`), `life_events [{ id, staff, night, pref, request: Swap|TimeOff|EarlyLeave|Toil,
  placeholders }]` (`app.request.<id>`).

### 10.12 The Shiftella app as a tool (`shiftella.ron`)

* `ChecklistItem` += `owner: Karolina | Either(Duty)` (`owner()` falls back to the job's duty, else
  Karolina), `anchor` (furniture tag of the world chip), `gesture: Click | Hold(s) | Swipe | Pick`,
  `extra_done` / `extra_missed: Vec<Fx>`. `done` / `missed` stay core effects (applied by today's sim).
  Opening `extra_missed` applies at the opening ritual's deadline; Closing at 06:00. New item `record_on`.
* `rituals [{ list, by, effects }]` — `app.ritual.opening/.closing`.
* `BotAction` += `handler: CoverSelf | CoverOther | CallIn | ReserveSeat | RemindBefore(min) |
  OrderDrone { minutes } | MenuOff | ApplyMoment(id) | Camera(target) | Open(AppLink)`. New templates:
  `regular_tonight`, `regular_to_go`, `low_stock`, `weather_alert`, `cat_waiting`, `rozpis_happy`,
  `looks_tired`, `manage_alone`; existing ones gained buttons (`late_alert.ask_other`,
  `sick_call.call_in/.manage`, `train_delay.get_ready/.noted`, `payment_reminder.open_money`).
* `Announcement.effects: Vec<Fx>` (thank_you → `TargetStaffMood`, new_item → `ItemSpotlight`,
  swap_board → `SwapBoard`).
* `handover_options [{ id, when, subject: None|LowItem|Machine|Regular|Train|Table|Crew, next_night,
  placeholders }]` — `app.handover.option.<id>`; `insights [{ id, link: AppLink, placeholders }]` —
  `app.insight.<id>`; `payroll_rules { splits [{ split: Cafe|Half|Crew, crew_share, crew_mood,
  crew_wellbeing, default }], star_bonus, star_mood, toil_grant_wellbeing }` — `app.payroll.split.<id>`.
* Fixed UI keys (order card, rail, queue, tip jar, lull chips, duties board, jukebox, themes, atmosphere,
  book, journal, rozpis, payroll, requests, settings): `handson::keys::UI_KEYS` with their placeholders.

### 10.13 Validation (hands-on)

Unique ids per list; every `XRef` resolves; recipes: base item, 1–3 known ingredients, unique ingredient
sets, variants exist for the kind, SHOULD kinds have a MUST fallback, `Taught.by` is a regular or staff,
non-starters have a silhouette hint, `revealed` ⊆ ingredients, every ingredient is used; wishes: tag
rules, personal prefix ⇔ `who`, favourites exist; moments: 1–3 options, `from_event` exists, anchors,
follow-ups are spawned somewhere; records: tracks in `MUSIC_TRACKS`, exactly 3 starters, one effect per
mood; seasons cover the story nights once; themes: one free default, accent tag form, colours; coziness
weights sum to 100; journal rules for every kind, warm moments ⇔ captions; tutorial: `After` names an
earlier step, widgets in `TUTORIAL_WIDGETS`; extras: cat levels ascend from 0, tip goals ascend with two
furniture choices; staff default duty + prefs; story calendar weekdays; all derived keys in every
language, placeholder parity for UI keys and data templates.

### 10.14 Still to do with the sim (T4/T5) — content edits waiting for sim support

* `staff.ron` first hire 2 → 3 for barista/cook/porter (the sim QA tests still hire on night 2;
  `story.ron` already shows the hire hints on night 3).
* `regulars.ron`: `Any([ServedFastStreak…, WishMatchStreak…])` for Mirek/Hana stage 1, Vera stage 3 →
  `ServedRecipe("r_kulajda")`, Lori's encore trust bonus — need `ArcStage.conditions` to accept `Cond`.
  Hana's / Radek's fast-service thresholds are already 9 / 7 min (looser than the design's 6 / 5).
* `events.ron`: drop `choices` of the five events replaced by moments once the moment director runs.

---

## 11. Reasons to build (`content::build`, 0.2 package C)

Loaded into `content.build: BuildContent { wishes, corners, attic }` and validated with the rest
(`validate_build.rs`). Texts: `assets/i18n/src/build_content.tr`.

| file | type | contents |
|---|---|---|
| build_wishes.ron | `BuildWishesFile` | 12 requests: 6 regulars, 3 crew, 3 guest archetypes |
| corners.ron | `CornersFile` | 7 corners, `max_bonus`, `before_after { min_gain, after_at }` |
| attic.ron | `AtticFile` | restock weekday / first night / per week, 11 second-hand pieces, the back room |

* `BuildWishDef { id, who: Regular(id) | Staff(Option<archetype>) | Guests(archetype), def, tag,
  appear: Vec<Cond>, done: Vec<Cond>, reward: Vec<Fx>, arc }` — keys `build_wish.<id>.reason`
  (card title), `.ask`, `.thanks`. Needs a `def` or a `tag`; `done` non-empty.
* `CornerDef { id, needs: [CornerNeed { defs, tags, count }], radius (1–4), bonus (0–10), max,
  attracts (guest archetypes), attract_mul (1–2) }` — keys `corner.<id>.name/.desc`. The first
  need is the anchor (count 1).
* `AtticPiece { id (= furniture id), price, from_night }` — key `attic.<id>.story`. The furniture
  must be `unique` with `unlock_flag: "attic_piece"` (declared in story.ron, never set) and the
  second-hand price below the new one; every furniture hidden that way must be in attic.ron.
* `BackRoomDef { from_night, sandbox_night, price, pieces: [(furniture, x, y, rot)] }`.
* Journal: `HighlightKind::BeforeAfter` (`journal.caption.before_after` with `{what}`).
* Fixed keys: `content::build::UI_KEYS` (with placeholders).
