# Shiftella v0.1.0 — QA report (T10)

Branch `agent/t10-qa` (from master `746e253`), 2026-11-26. macOS (Apple Silicon, Retina),
release and dev builds. Scope: `design/handson.md` §9, §15 (T10 + extra acceptance), §16.

**How to reproduce.** Everything below is either a test (`cargo test`) or a headless example
(`sim_balance`, `game_autoplay`), plus screenshot sweeps of the real game. Screenshot and
autoplay runs are **invisible** on macOS (no window, no Dock icon, no focus taken, no sound —
`src/platform.rs`; `SHIFTELLA_HIDDEN=1`, implied by `SHIFTELLA_SHOT` / `SHIFTELLA_AUTOPLAY`,
`SHIFTELLA_HIDDEN=0` to watch). New QA helpers: `SHIFTELLA_AUTOPLAY=skip`,
`game_autoplay -- N speed diff skip`, `sim_balance … --inattentive`, `SIM_CHARACTER=…`,
`SHIFTELLA_LAYOUT_DEBUG=1` now also logs every text cut with "…", `examples/qa_glyph_atlas`.

## Summary

| # | Area | Result |
|---|---|---|
| 1 | §9 targets + T10 acceptance | pass, one balance target slightly over (rush peak) |
| 2 | Keyboard-only night 1, Relaxed + skip-all, Classic, sandbox | pass |
| 3 | Save / load | pass |
| 4 | No-audio run, tests without audio | pass (fixed the test) |
| 5 | Content gaps: arcs, endings, Answer | **fixed** (blockers) |
| 6 | Rendering: black-box captions, dark polaroids | **fixed** |
| 7 | UI sweep (4 languages, 2 scales, 2 sizes) | fixed what mattered, minor cuts left |
| 8 | Performance | pass |
| 9 | Polish | several fixes, see below |

Blockers found and fixed: the franchise offer / «Last Train (sold)» never happened (day beats
skipped), Filip's and Lori's arcs could never progress for the balance bot and could be lost
by a player who held Esc, black boxes in handwritten text (glyph atlas), dead bot buttons
("Remind me" by day, "Reserve the seat"), raw ids in handover notes.

## 1. §9 targets and the T10 acceptance list

`sim_balance` (6 seeds, player-bot q 0.75, react 3 s, hires 3/5/9, Cozy):

| Metric (nights 1–10) | Target | Result |
|---|---|---|
| Karolina idle share at ×1 | ≤ 15 % | 6.7 % |
| Pending things median / Cozy rush peak | 1–3 / ≤ 7 | 0.9 / worst avg peak 8.5 (**over**, was 7.8 before T10) |
| Guests leaving unhappy on Cozy; forgiven parties | 0; ≤ 10 % | 0; 6.1 % |
| Moments per night; two open on nights 1–3 | 2–4; never | 2.8; 0 |
| Orders by Karolina: nights 1–2 / 8–16 / regulars | 100 % / ≥ 45 % / ≥ 70 % | 100 % / 51 % / 100 % |
| Mini-games per night; same kind in a row | 6–14; ≤ 3 | 9.8; 2.0–3.0 |
| Real length of a night at ×1 | 12 ± 1 + 1–2 overlays | 12.9 min (overlays 3.3) |

30 nights Cozy: payments 30/30, «Light in the Window» (full) 6/6, every arc complete 6/6.
30 nights Classic: the final 30 000 Kč payment is the real test — the attentive bot (who now
builds the socket table and the stage, gives Filip a coffee…) misses it in 6/6 runs and ends
on «Last Train» (farewell); without those purchases (`--inattentive`) 4/6 make it. The bot now
keeps the next payment aside before buying decor. Locked by `classic_story_is_a_fair_challenge`.

T10 acceptance:
* **A record change is audible within 3 s** — pass: the new track starts at once (needle
  click) and crossfades in 2.5 s (`audio::FADE_SECONDS`, `records.ron crossfade_s`); a liked
  record gives trust at most once per visit (`a_liked_record_gives_trust_once_per_visit…`).
* **Coziness meter moves on cleaning / lamps** — pass (`chores_and_lamps_move_the_coziness_meter`).
* **Exactly one polaroid per night; 30 entries re-render after load** — pass
  (`every_night_leaves_a_polaroid…`, new `thirty_polaroids_survive_a_save_and_rebuild_their_scenes`).
  Night 30's polaroid stays "pending" (the ending closes the story before the morning).
* **Auto-fill keeps every Need; a happy week with the story crew** — pass
  (`the_auto_fill_alone_plans_a_happy_week…`, `a_happy_week_is_reachable…`).
* **Every checklist item, request answer and bot button has a visible effect** — pass after
  fixes. New `every_bot_button_acts_or_says_why_and_stays_open` presses every button of every
  bot template on a staged night (approve/decline are covered by the request tests);
  `a_reserved_seat_waits_for_its_regular_and_a_day_reminder_rings_at_night`. Fixed:
  "Reserve the seat" didn't hold a seat (now it does, with a «Reserved» tent card on the
  table); "Remind me 5 minutes before" pressed by day never rang (cleared at night start);
  "Ask someone to cover" with nobody free was a silent no-op; "We'll manage" did nothing;
  refusals showed a generic or wrong reason (now localized: nobody free / only at night /
  no free seat / needs a drone dock). "Noted" only closes the message (by design).

## 2. Play-throughs

* **Keyboard-only night 1** — pass. Every night-1 action has a key (G, Enter + `[ ]`, 1–4 /
  Q / U / H / K on the card, E for chores incl. hold-E wash and the window, F + 1–3 moments,
  V jukebox, Backspace, Enter for the summary); mini-games by keyboard
  (`SHIFTELLA_AUTOPLAY=keys`): night 1 played to the morning and into night 2. Coach marks
  can't be dismissed by keyboard but complete on their own when the action is done.
* **Relaxed + skip every mini-game → an ending on Cozy** — pass:
  `game_autoplay 30 3 cozy skip` reaches «Light in the Window» (journal 29 + night 30).
* **Classic run** — `game_autoplay 30 3 classic`: plays to night 30, ends on «Last Train»
  (final payment missed — Classic working as designed).
* **Sandbox 30 nights** — `SIM_CHARACTER=calm|chaotic sim_balance 30 sandbox cozy 4`: no
  problems, no stuck agents, money grows (36–87 k), chaotic has clearly more events.

## 3. Save / load

All pass: v1 → v2 migration (day and mid-night fixtures), mid-night save continues
identically, same seed + commands ⇒ same save; new `manual_slots_continue_and_a_finished_story`
(auto + 3 manual slots side by side, Continue = newest, a finished story's save is marked
`finished` — the menu hides Continue — and loads straight to the credits).

## 4. No audio

* The game started and played 25 s of autoplay with `assets/audio/` removed: exactly one log
  line, `info: assets/audio not found — playing without sound`.
* `cargo test` passes with and without `assets/audio/`:
  `content::tests::audio_ids_exist_on_disk` skips when the folder is absent (a release tree
  with the folder must still have every file). `assets/audio/` and `design/audio/raw/` are
  now in `.gitignore` (tracked files stay until they are removed from git).

## 5. Content gaps — arcs, endings, the Answer

Why Filip and Lori never moved: the balance bot ended every scene on its defaults (no choice
picked), so Lori was never invited and Filip's exam set no flag; it never built the socket
table Filip asks for. A player could lose them the same way: holding Esc skipped a scene's
choices. And the franchise offer (night 17, 21:59) never became pending because a day's beats
were skipped when the previous night had a beat with the same index — so the franchise
decision never played and «Last Train (sold)» was unreachable by playing. Fixes:

* `World::begin_day` clears the beat bookkeeping (also restores payment reminders and the
  hire / schedule / armchair / inspection hints).
* Hold-Esc skip stops at a choice that has options; skip again after answering.
* The player-bot is attentive by default: kind story choices, a socket table + chairs once
  Filip asked, the stage once Lori is invited. The game's autoplayer answers the same way.
* Single-night arc windows widened (Hana 4: 26–29, Filip 2: 18–23, Filip 4: 29–30).
* Tests (`src/sim/tests_t10.rs`): an attentive Cozy story finishes all seven arcs (Mirek,
  Hana, Vera, Filip, Radek, Lori, the 3:33 guest), ends «Light in the Window (full)» and plays
  the Answer; accepting the franchise ends «Last Train (sold)» without the Answer; skipping
  every choice keeps the choice-free arcs and still ends in the light.

## 6. Rendering

* **Black-box captions** (ru/uk handwritten text) — root cause: macroquad caches glyphs
  lazily in one atlas texture per font; when a new glyph doesn't fit, the atlas doubles and
  deletes its old texture, so every glyph of that font already batched in the frame — even
  earlier letters of the same string — sampled a dead texture (macOS: "texture unloadable,
  using zero texture" → black). Cyrillic at the handwritten sizes was often what overflowed.
  Fix: `ui::text::GlyphCache` caches a string's new glyphs before it is measured or drawn,
  after flushing the batch (UI text, world text, the menu sign). Reproduced and verified with
  `examples/qa_glyph_atlas` (before: black boxes; after: clean).
* **Dark polaroids near the door** — the snapshot window was clamped to the grid, so a
  moment at the door framed the street and the void. It now stays inside the café; the 3:33
  guest / the cameo / an arc beat that fires as a regular walks in are photographed once
  they sit down; a soft camera flash lifts the frame a little.

## 7. UI sweep

Every scene (`SHIFTELLA_SCENE` list incl. all 7 app tabs, 7 mini-games, tutorial days/nights
1–4, F1 help, journal open, endings, credits, settings, language picker) in cs/en/uk/ru at
1280×720 with UI scale 1.3 and 1.0, ru also at 1920×1080, with `SHIFTELLA_LAYOUT_DEBUG=1`.
Fixed: settings panel ran off a 720p screen at 130 % (rows tighten now); crew card overlapped
its buttons (sized to content, scrolls); the ticker cut train announcements (shrinks, two
lines, full text in the tooltip); mini-game instructions were cut (shrink / two lines);
pantry jar names (two lines); rozpis cells "Пол…" (shift icon when narrow) and the
Auto-fill / Clear buttons covering the week badge (icon buttons when narrow); handover card
overflow; toasts get 4 lines; labels in general shrink up to 15 % before "…"; shorter labels
(Follow; UA tabs Чат / Підсумок); EN/UA/RU menu say "Topinka" like the recipe book.
See "Known minor issues" for what is left.

## 8. Performance

F3 numbers (`SHIFTELLA_FPS=1`), Retina 2×, MSAA 4, release: see the table below — vsync-capped
60 fps with ample headroom; the dev profile (dependencies at opt-level 3) stays well above 40.

| Build | Scene (autoplay ×3, 1 500 frames) | FPS | frame avg / max | world / UI / sim ms |
|---|---|---|---|---|
| release | night 27 — snow, garlands, Winter Lights, full hall | 60.0 (vsync) | 16.7 / 17.4 ms | 3.2 / 1.1 / 0.2 |
| release | night 22 — busy weekday, rain | 60.0 (vsync) | 16.7 / 16.8 ms | 2.2 / 1.5 / 0.0 |
| dev | night 27 | 42.9 | 23.4 / 26.4 ms | 16.4 / 5.0 / 0.3 |
| dev | night 22 | 47.9 | 20.8 / 24.0 ms | 12.2 / 6.7 / 0.1 |

Release has ~12 ms of headroom per frame; the dev profile meets ≥ 40 fps but not by much
(world drawing dominates; dependencies are already optimized in dev).

## 9. Polish

Besides the above: the order card said "Garlic toast" over a "Topinka" recipe; the handover
note showed raw ids ("black_coffee", "mirek"); a reserved seat now visibly waits; refusals say
why in the player's language; hidden automation runs never disturb the person at the Mac.

## Known minor issues (not blockers)

* Cozy rush peak (worst average of pending things) is 8.5 vs the ≤ 7 target; the median is
  below 1 — rushes are short. Balance knob for v0.1.x.
* Classic final payment: after QA the lead raised Classic `income_mul` 1.35 → 1.55 — the attentive
  bot now makes all payments (6/6 «Light in the Window», 2–17 k left after the final payment).
* At UI scale 1.3 on 720p a few secondary texts are still cut with "…": rail guest names
  (tooltip has them), the duties board role subtitle, the small polaroid captions in the
  morning summary, the second line of very long train announcements in the ticker (tooltip),
  the first-day bot toast (full text in the app).
* On this Mac (1512×982 pt) a 1920×1080 pt window can't be created: macOS clamps it to
  1470×827 pt, so the "1920×1080" sweep ran at that size (larger than 1280×720; nothing new
  showed up there).
* The app's close button sits on the tablet's corner (layout debug flags it; looks intended).
* Captions repeat for the same regular's perfect cup on nearby nights ("Mirek: Just right.").
* `Variance`: about 1 night in 120 has only 2 mini-games (few wish guests); the per-night
  floor in `tests_balance` is 2, the 6–14 average stays locked.
* Night 30's polaroid stays pending (the ending closes the story first).

## Verification

`cargo fmt && cargo clippy --all-targets && cargo test` — zero warnings, all tests pass, with
and without `assets/audio/`.
