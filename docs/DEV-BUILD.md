# Shiftella — developer / test build

A build of the game with a **developer panel** over every screen: jump to any night, open
any screen or overlay directly, play any cutscene or talk, switch the language live, cheat
the running night and switch debug drawing on. It is the normal game plus one cargo
feature, `dev`; the release build has none of it (the `dev` module isn't compiled — a
`compile_fail` doctest in `src/platform.rs` checks that).

## Build and run

```sh
cargo run --release --features dev              # play the dev build
./scripts/package-macos.sh --dev                # dist/Shiftella Dev.app + dist/Shiftella-Dev-macos.zip
```

The packaged app is called **Shiftella Dev** (bundle id `app.shiftella.game.dev`), so it sits
next to the real Shiftella without replacing it. Generated music / sounds are bundled only if
`assets/audio/music` and `assets/audio/sfx` exist in the checkout (they are gitignored —
ElevenLabs licence; link or copy them in from the main checkout before packaging, never
commit them).

## Its own saves and settings

The dev build never touches the real game's files:

| | release | dev build |
|---|---|---|
| macOS | `~/Library/Application Support/Shiftella/` | `~/Library/Application Support/Shiftella Dev/` |
| Windows | `%APPDATA%\Shiftella\` | `%APPDATA%\Shiftella Dev\` |
| Linux | `~/.config/shiftella/` | `~/.config/shiftella-dev/` |

(saves in `saves/`, `settings.ron`, the crash log). `SHIFTELLA_SAVES` / `SHIFTELLA_SETTINGS`
still override both. The window title says "Shiftella Dev" and a small amber **DEV** badge
sits in the bottom-left corner of every screen.

## Keys

| key | |
|---|---|
| `` ` `` (backtick) | open / close the panel (not while typing in a text field) |
| `F1` | open / close the panel — in the dev build F1 no longer opens the game's Help (use the HUD's help button or rebind Help in Settings → Controls) |
| `F3` | FPS overlay (also in the panel's Overlay section) |

On the main menu a **Developer** button (top right) opens the panel too. The panel sits on
the right, takes the clicks over it (the game under it doesn't get them) and never takes keys
from a focused text field. Its texts are English only — it is a developer tool; everything it
opens is the game's own localized UI.

## Sections

**Jump** — start a story at night N (1–30) in Cozy / Carefree / Classic: the balance bot
(`sim::bot::PlayerBot`, the player `examples/sim_balance` sweeps with) plays every night
before N headless, a night or more per frame with a progress card, then you take over at
N's day screen. Same seed → the same café (the very world `GameScreen::fast_forward` — every
screenshot scene — reaches). "Bot hires helpers" hires on nights 3, 5, 9, 14. Also: continue
the running story to night N, an Endless sandbox or a Challenge run at night N, back to the
main menu.

**Screens** — opens any screen directly, each set up like its screenshot scene
(`SHIFTELLA_SCENE` + options; see `src/dev/catalog.rs` and the docs of `src/app/mod.rs`):
main menu, language picker, new game, sandbox setup (+ Challenge), settings tabs; the opening,
day 1, tutorial coach marks; nights (preparation, closing, pause menu, order card, jukebox,
full house, strangers' names, Anwar, moments, garlands, the cat, auto-pause, stories, storm
and its talk, nights 14 / 22 / 27, equipment states, a talk, a cutscene choice); crew panel
and every crew card, duties, log, build mode (+ dragging), themes, attic, back room,
requests, corner, before / after, recipe book, pantry, napkins, payroll, the house drink;
the Shiftella app's tabs; the journal's pages incl. Lida's notes, an enlarged polaroid, extra
photos, the morning polaroid, photo mode / print; rituals & habits (checklist states, queue,
follow chip, gestures, all-by-hand, grandma's habit card); every mini-game; the morning
summary (+ fridge, purchases), the walk home (route picker and each route, rain, snow), the
back step; both endings and the credits; the sandbox and the Challenge morning / final page.
The search field filters by name or group. Late-night screens take a moment to set up
(the bot plays the nights before).

**Scenes** — every cutscene of `cutscenes.ron` and every talk of `talks.ron`, searchable by
id or trigger. *Play* plays it from the start, *Choice* plays it up to its choice (greyed when
it has none). By default over the running café; with "At its own night" (or with no game
running) over a fresh café at the night its trigger names (a talk: its regular's visit).

**Lang** — cs / en / uk / ru, live on every screen (not saved).

**Cheats** (a game must be running) — money ±, reputation ±; jump the clock to 21:30,
22:00, 00:30, 03:30, 05:15, 06:00 (forward only; from the day the night starts) or run to
the morning; pause / speed ×1–3; weather; clean / dirty floor; fill / empty the fridge; hire
the first candidate / fire the last helper; find all of Lida's notes, learn every recipe,
unlock all furniture, own every theme; for any regular or cameo (★): walk in now, arc
stage ±, trust ±; set / clear a flag, set a var or a counter by name; fire any event of
`events.ron` (searchable).

**Overlay** — FPS & frame times, layout debug (text / widgets out of their box, as
`SHIFTELLA_LAYOUT_DEBUG`), UI regions by layer, nav grid (walkable / guests / seats /
blocked), seat claims (who sits where, which staff uses or claims a piece).

## Screenshot hooks (dev build only)

`SHIFTELLA_DEV=jump|screens|scenes|lang|cheats|overlay` opens the panel on a section,
`SHIFTELLA_DEV_FILTER=<text>` types into its search field, `SHIFTELLA_DEV_JUMP=<night>` starts
a Cozy jump at boot (the progress card), `SHIFTELLA_DEV_OVERLAYS=nav,seats,regions,layout,fps`,
`SHIFTELLA_DEV_OPEN=<Screens label>` opens a screen as its button does (e.g. `Journal · Lida's
notes`), `SHIFTELLA_DEV_CUTSCENE=<id>` plays a scene as *Play* does.
Always hidden:

```sh
SHIFTELLA_HIDDEN=1 SHIFTELLA_DEV=screens SHIFTELLA_SHOT=/tmp/dev_screens.png \
    cargo run --release --features dev
```

## Code

* `src/dev/` — the panel (`panel.rs`), the screen catalog (`catalog.rs`), scene options for
  the scene code (`opts.rs`), the jump (`jump.rs`), cheats (`cheats.rs`), overlays
  (`overlays.rs`), tests (`tests.rs`).
* `src/app/dev.rs` — keys, drawing over each screen, carrying out the panel's actions.
* `src/sim/dev_cheats.rs` — the world-side cheats.
* Hooks in shared code are `#[cfg(feature = "dev")]`: `app::scene_env` (the scene options),
  the run loop, `ui::Ui::dev_*`, `platform::DATA_DIR`.
* The scene code itself (`app::SceneBuilder::scene`) is shared with the screenshot runs: a
  new `SHIFTELLA_SCENE` scene becomes openable by adding one line to `catalog.rs`.

Tests: `SHIFTELLA_HIDDEN=1 cargo test --release --features dev` (the jump's determinism, the
dev folder, every opener and cheat headless).
