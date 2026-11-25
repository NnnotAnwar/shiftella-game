# Shiftella (shiftella-game)

A cozy, very atmospheric night-café management game. 2039, the small Czech railway town
Mlhov nad Šeřavou, grandma Lída's café «U Výhybky», open 22:00–06:00. Rain, warm lamps, quiet
night trains, regulars with stories — and the crew's shifts run through the in-game **Shiftella**
app. Story (30 nights, two endings) and an endless Sandbox. CZ / EN / SK / UA.

Rust (edition 2024) + [macroquad](https://macroquad.rs) 0.4.

## Run

```bash
cargo run --release      # the game
cargo test               # tests (content, i18n, sim, saves, the game flow headless)
cargo clippy --all-targets
scripts/package-macos.sh # dist/Shiftella.app
```

How to play, controls and tips: **[docs/PLAYING.md](docs/PLAYING.md)**.

## Layout

| Path | What |
|---|---|
| `src/main.rs` | the window; calls `app::run()` |
| `src/app/` | the app state machine: boot → language → menu → game → credits (`route` is the pure transition table) |
| `src/screens/game/` | a running game: `world_view` (the sim adapter), `scene` (the live café), `hud`, `panels` (crew, priorities, log), `phone_app` (Shiftella), `build`, `day` (planning + morning summary), `pause` |
| `src/screens/menu/`, `boot.rs`, `credits.rs` | main menu & language picker, loading screen, credits |
| `src/sim/` | the simulation behind `sim::World` (see `docs/SIM.md`) |
| `src/content/`, `src/i18n/` | data model + loader, translations (see `docs/CONTENT.md`) |
| `src/render/`, `src/ui/`, `src/cutscene/` | world renderer, UI kit, cutscene player |
| `src/save/` | save slots (RON, versioned) |
| `src/audio.rs` | music crossfades, rain ambience, SFX |
| `assets/` | data, translations (`assets/i18n/src/*.tr` → `cargo run --example content_i18n_build`), fonts, audio |

Saves and settings live in `~/Library/Application Support/Shiftella/` (macOS; `%APPDATA%\Shiftella`,
`~/.config/shiftella` elsewhere; fallback `./`). Overrides: `SHIFTELLA_SETTINGS=<file>`, `SHIFTELLA_SAVES=<dir>`.

## Test automation

The game binary can jump straight into a state and save a screenshot:

```bash
SHIFTELLA_SCENE=night SHIFTELLA_NIGHT=5 SHIFTELLA_TIME=01:30 SHIFTELLA_LANG=cs \
  SHIFTELLA_SHOT=shots/night.png cargo run --release
```

* `SHIFTELLA_SCENE` = `menu | language | newgame | sandbox_setup | opening | story_day1 | night |
  night9_anwar | crew | app | priorities | log | build | pause | summary | ending_light |
  ending_last_train | credits | sandbox`
* `SHIFTELLA_NIGHT=n`, `SHIFTELLA_TIME=HH:MM`, `SHIFTELLA_T=<s>` (cutscene fast-forward),
  `SHIFTELLA_LANG=cs|en|sk|uk`, `SHIFTELLA_FRAMES=n` (default 45), `SHIFTELLA_MUTE=1`.
* With `SHIFTELLA_SHOT` the game ignores keyboard/mouse world input, never autosaves and uses
  a temporary saves folder unless `SHIFTELLA_SAVES` is set.

Demos: `cargo run --release --example render_demo | ui_demo | menu_demo | cutscene_demo`
(each supports `SHIFTELLA_SHOT`).
