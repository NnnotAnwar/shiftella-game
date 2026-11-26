# Shiftella — game brief (source of truth for all agents)

Read this first. Where `design/*` docs disagree with this file, **this file wins**.

## 1. What the game is
- **Name:** Shiftella (repo/crate `shiftella-game`). A cozy, very atmospheric management game
  for relaxing in the evening after a shift. Low stress, gentle pacing, beautiful night mood.
- **Setting:** near future, **2039**. Night café **Café Nocturne** (sign subtitle: "kavárna u nádraží")
  next to **Praha-Holešovice** station in Prague. Open 22:00–06:00. Rain, warm lamps, quiet autonomous
  night trains, soft holographic signs, small delivery drones over wet roofs — near-future but intimate
  and human, never cyberpunk-grim. International feel with a Czech note (renamed 2026-11-26 from the
  former fictional town "Mlhov nad Šeřavou" and café "U Výhybky").
- **Names:** simplified, international-friendly with a Czech note: grandma **Lida**, her fiancé **Jindra**,
  regulars Mirek, Hana, **Vera**, Filip, Radek, Lori; the 3:33 guest; Anwar (cameo). Staff/guest name pools: an
  international mix (Czech, Slovak, Ukrainian, European) with few diacritics.
- **Story & content:** `design/story/*` (written in Russian — the *ideas* are canon, the text must be
  written fresh in the four game languages). Grandma Lida's café, 30 nights, regulars with arcs,
  the 3:33 guest, endings. Dates: grandma's fiancé left on night train Rn 333 in **1979**;
  the game is 60 years later (2039).
- **Cutscenes & dialogue:** story moments play as cutscenes — letterbox, camera moves, actors walking,
  glass dialogue box with portrait + typewriter text, 2–4 choices, narration, fades. Data-driven (`CutsceneDef`).
- **Talking to guests:** hovering/clicking a regular shows a small speech bubble with a context mood line
  (`ambient_lines`); a "Talk" action on a regular plays their talk dialogue for that night if one exists.
- **Anwar (cameo):** appears only on nights 9 and 24, quiet guy working on a laptop; identity not obvious;
  talk dialogue about making a small game about a night café; no story impact.
- **Exactly two endings:** «Light in the Window» (café saved, stays itself) and «Last Train» (café closes /
  sold to NONSTOP 24, bittersweet farewell). The letter / 3:33-guest resolution is an optional extra scene
  inside either ending, not a third ending.
- **Modes:** Story (30 nights) and Sandbox (endless, random town/seed). Difficulty presets:
  **Cozy (default)** — no game over, debt payments soft (miss = mild story consequence),
  generous money; **Classic** — payments matter, the café can close (still an ending, not a fail screen).
- **Shiftella references (owner's real app — weave them in lovingly):**
  - The shift schedule, staff management and messages happen in an in-game phone/tablet app
    called **Shiftella** (same glass look as the UI). Features mirrored from the real app:
    publishing the weekly rozpis (schedule), shift swap requests, time-off / leave requests
    (dovolená, náhradní volno), late-arrival alerts, handover notes between shifts (předávka),
    announcements board, opening/closing checklists, a night digest, a weekly
    "payroll basis" export (podklady pro mzdy) as a cute summary screen.
  - Notifications arrive from **@shiftella_bot**. Never implement clock-in/out through the bot.
  - Easter eggs: stations on the train line are named after Shiftella release codenames —
    **Adamov, Beroun, Cheb, Děčín, Ejpovice, Frýdlant** … then Praha-Holešovice. Train announcements use them.
- **Languages:** **CZ / EN / SK / UA** → codes `cs`, `en`, `sk`, `uk`. Every player-visible string goes
  through i18n keys. First launch shows a language picker; choice saved in settings. Fallback: `en`, then the key.
  No Russian in the game.

## 2. Visual style — our own, not a RimWorld copy
- **Palette = Shiftella's universe (purple/indigo):** accent indigo `#6366F1` (600 `#4F46E5`,
  glow `#818CF8`/`#A5B4FC`), second accent violet `#7C3AED` / `#A78BFA`; ink `#1B1A2E`;
  night canvas gradient `#0D0C17 → #121022 → #1A1730`; violet night `#120C1F → #201634`.
  Warm counterpoint: amber lamp light `#FFC46B`/`#F99C4E`. Status: emerald `#10B981`, red sparingly.
  This **replaces** the neon pink `#FF4F9A` in `design/ui/*` — use indigo/violet for active/primary.
- **UI:** frosted-glass panels with a lit 1 px gradient edge and soft shadow (like the app),
  rounded 8–12 pt, compact and legible (body 13 pt, small 12, headings 16–18; Inter from
  `assets/fonts/`). No blocking modals in frequent flows: one-click actions + undo, toasts.
  Layout/screens from `design/ui/screens.md`, but hotkeys: camera **WASD/arrows/right-drag/wheel**;
  screens **C** crew/staff, **P** priorities, **T** Shiftella app (schedule), **B** build, **L** log,
  **Space** pause, **1/2/3** speed, **M** sound, **Esc** back/menu.
- **World art — "violet dollhouse":** floor and rugs seen from above, but furniture, walls and
  characters drawn as a gentle **3/4 front view** (visible front face + top, height, y-sorted),
  like a lit dollhouse at night. Smooth vector shapes with soft gradients, no black outlines,
  rim light in lavender from windows, warm amber pools from lamps, subtle bloom/glow on screens
  and holo signs. Characters: original, upright, friendly proportions (~1.3 tiles tall),
  readable faces/hair/clothes, staff in indigo aprons with a small Shiftella badge.
  Not RimWorld's strict top-down, not its limbless pawns, not its UI.
- **Atmosphere is the core feature:** day→night lighting, rain on windows (animated streaks),
  puddle reflections outside, trains passing with light sweeping across the room, flickering
  holo sign, steam from cups and coffee machine, ambient sound layers.

## 3. Tech & architecture
- Rust edition 2024, macroquad 0.4.16 (`audio` feature), serde + ron. No other crates unless an
  agent really needs one (prefer writing small code — e.g. own seeded RNG for the sim).
- Crate = library (`src/lib.rs`) + thin binary (`src/main.rs`). Demos in `examples/*.rs` may use
  the library; each demo supports `SHIFTELLA_SHOT=path.png` → render ~30 frames, save
  `get_screen_data().export_png(path)`, exit (note: the image may come out vertically flipped
  on some setups — compare with what you drew). On this macOS setup, Camera2D for the
  **screen** needs a **positive** `zoom.y` (macroquad inverts Y for non-render-target cameras);
  `Camera2D::from_display_rect` is only correct for render targets — for the screen flip `zoom.y`.
- World coordinates: world pixels, `map::TILE = 64.0`. UI: logical points (`screen / screen_dpi_scale()`),
  text rasterised at physical size (see `hud.rs::text`).
- Simulation is pure logic in `tick()` (10 ticks = 1 game minute); it emits events; audio/UI react.
  Nothing in rendering mutates the sim.
- Content is data: `assets/data/*.ron`; text in `assets/i18n/{cs,en,sk,uk}.ron`.
- Run before every commit: `cargo fmt && cargo clippy --all-targets && cargo test` — zero warnings.
- **Repository language: English.** Code comments, doc comments, docs, README and commit messages are in
  English (the repo will be public). Player-visible text lives only in `assets/i18n` (cs/en/sk/uk).
- **Audio is not committed** (ElevenLabs licence: no redistribution outside the game). `assets/audio/` and
  `design/audio/raw/` are git-ignored; they ship only inside release builds. The game must run without them.

## 4. Module ownership (to avoid conflicts between parallel agents)
| Area | Owner | Files |
|---|---|---|
| Content + i18n | content agent | `src/content/**`, `src/i18n/**`, `assets/data/**`, `assets/i18n/**`, `examples/content_*.rs` |
| World rendering | render agent | `src/render/**`, `examples/render_*.rs` |
| UI kit | ui-kit agent | `src/ui/**`, `examples/ui_*.rs` |
| Simulation | sim agent (wave 2) | `src/sim/**`, `examples/sim_*.rs` |
| Cutscene player, main menu, language picker, settings | cutscene agent (wave 2) | `src/cutscene/**`, `src/screens/menu/**`, `src/settings/**`, `examples/cutscene_*.rs`, `examples/menu_*.rs` |
| Gameplay screens, flow, save/load, integration | integration agent (wave 3) | `src/main.rs`, `src/game.rs`, `src/hud.rs`, `src/screens/**` (except menu), `src/save/**` |

## 5. Simulation facade (contract between sim and the rest)
`sim::World` holds the whole game state (`Serialize + Deserialize` for saves). Commands in, events out:
- `World::new_story(&Content, Difficulty)`, `World::new_sandbox(&Content, SandboxSetup)`.
- `tick(&mut self, &Content)` — one tick (10 per game minute); `drain_events() -> Vec<SimEvent>`.
- `phase() -> Phase { Day /*planning, paused clock*/, Night }`, `start_night(&Content)`, night ends at 06:00 → Day.
- `apply(Command) -> Result<(), CommandError>` for every player action (place/move/remove furniture,
  hire/fire, set job priorities, set schedule, publish schedule, approve/deny time off & swaps,
  answer an event choice, buy supplies, set prices, rename café…) and `undo()` for the last reversible one.
- Read-only queries used by render/UI: map/tiles, placed furniture (+ state), staff (look seed, pose,
  facing, feet pos, needs, skills, traits, current job), guests (same + wish/patience/mood),
  money, reputation, night number, clock, story flags, notes found, ending reached, Shiftella-app data
  (schedule, requests, announcements, checklists, digest, payroll basis), event log.
- Story director lives in sim (decides which `CutsceneDef` should play: emits `SimEvent::PlayCutscene(id)`,
  pauses the clock until `Command::CutsceneFinished { id, choices }`).
Do not edit files outside your area. If you need something from another area, write it in
your final report as an interface request.

## 6. Hands-on gameplay (in v0.1.0, owner decision 2026-11-26)
Goal: the player is not only a manager — the heir works behind the counter herself.
**Core principle (owner): "I don't want to just watch the game play itself."** The loop is PLAYER-FIRST:
the heir does the core work by hand every night; staff are helpers who only take what the player
delegates (limited, never the whole café); regulars, wishes, specials, moments and story beats always need
the player; at ×1 there is almost always something meaningful to do. Relaxing,
tactile, never failable. Three features:

1. **The heir behind the counter (direct control).** A `player` actor (look preset `player`) lives in the
   sim like staff but has no job priorities; she only does what the player asks. Click a guest → she walks
   over and takes the order → the guest's **wish** is shown → the player picks a recipe → a short
   **brewing mini-game** plays in a glass overlay (the night keeps running slowly, or pauses on Cozy) →
   she serves it. Staff skip orders the player has claimed. At any time the player can step back and just
   manage. Quality 0..1 from the mini-game affects mood, tips and regulars' trust — but even 0 is "fine".
   Sim API: `Command::PlayerTakeOrder { guest }`, `Command::PlayerBrew { guest, recipe, quality }`
   (she walks to the station, works for the recipe's prep time, then serves automatically),
   `Command::PlayerCancel`; events `SimEvent::PlayerOrderTaken { guest, wish }`, `PlayerServed { guest, recipe,
   match_score, quality }`.
2. **Drinks by mood, recipe book.** Guests (especially regulars) order a *wish*, e.g. "something warm, not too
   sweet — long day". `Wish { tags: Vec<Taste>, text_key }` with `Taste { Warm, Cold, Sweet, Bitter, Strong,
   Mild, Comforting, Fancy, Quick, Filling }`. `RecipeDef { id, name_key, desc_key, tags, base_item /* menu id */,
   minigame: MiniGameKind, unlock: … }`. Match score = tag overlap → satisfaction / trust / story progress.
   Recipes are discovered from grandma's notes, regulars, and **experiments** (combine 2–3 ingredients in the
   recipe book → maybe a new recipe). The **recipe book** (hotkey **R**… use **K** — R is rotate in build) is a
   collection screen with discovered / undiscovered (silhouette) entries.
3. **Small night moments.** Short, warm, one-click interactions instead of dry events: choose a record on the
   jukebox (music changes the hall's mood), light candles during a blackout, put a bucket under a roof leak,
   feed the station cat, help a lost traveller find their train via the Shiftella app, open the window for
   fresh air when it's stuffy, etc. `MomentDef { id, kind, conditions, anchor /* tile or furniture tag */,
   options: [{ label_key, effects }] }` → `SimEvent::MomentAvailable { id, anchor }` (a small glowing
   marker in the world) → `Command::ResolveMoment { id, option }`.

Mini-games (`src/minigame/**`, `MiniGameKind`): **MilkFroth** (hold to froth, release in the sweet spot),
**LatteArt** (draw a heart/rosetta/tulip by dragging; scored by similarity), **PourOver** (pour in slow
circles to a gentle rhythm), **TeaSteep** (lift the tea bag when the colour looks right), **Plate** (arrange a
pastry / soup garnish). Each: `start(kind, seed)`, `update(&mut Ui, dt) -> MiniGameStatus { Running,
Done { quality } }`, `draw(&mut Ui, rect)`; skippable (auto quality 0.7), 8–20 seconds, cosy SFX hooks.
