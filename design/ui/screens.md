# Shiftella — screens

> **Layout spec, written before implementation (v2).** The shipped screens live in `src/screens/**` and
> follow this spec loosely: the panels, toasts, tooltips and one-click/undo rules below all made it in, but
> exact positions changed, the schedule became the weekly **rozpis** inside the in-game **Shiftella** app
> (`T`), and the hotkeys changed to `C/P/T/B/L` with a `WASD` camera. The hotkey table below is the shipped
> one; the per-screen details are kept as design history. For how the game plays today, see
> [`docs/PLAYING.md`](../../docs/PLAYING.md).

All coordinates are in **logical points (pt) on the 1280 × 720 reference screen** (`x, y, w × h`, origin at the top left).
Every block has an **anchor**: in a different window size the block holds on to that edge or corner, and its sizes stay in pt, multiplied by `ui_scale` (see `style-guide.md`, §1).
Notation: `W`, `H` — the window size in pt; "baseline" — the text baseline.

Mockups: `hud.svg`, `staff.svg`, `priorities.svg`, `schedule.svg`, `build.svg`, `tooltip-and-event.svg`, `main-menu.svg`, `sandbox-setup.svg`, plus `components.svg` (all the components and tokens). Preview — `index.html`.

---

## Hotkeys (summary, shipped)

| Key | Action | Where |
|---|---|---|
| `Space` | pause / resume | the whole night |
| `1` `2` `3` | speed ×1 / ×2 / ×3 (starts the clock if paused) | the whole night |
| `C` | Crew — needs, skills, traits, relationships; hire, let go | day and night |
| `P` | Work priorities | day and night |
| `T` | The Shiftella app — rozpis, requests, messages, board, checklists, digest, payroll | day and night |
| `B` | Build | day and night |
| `L` | Event log (tonight / story) — also a click on the ticker | everywhere |
| `M` | sound on / off | everywhere |
| `Esc` | close the top panel; with nothing open — the pause menu | everywhere |
| `Ctrl+Z` | undo (build, rozpis, priorities, demolish) | everywhere |
| `R` · `X` | rotate · demolish mode | Build |
| `Enter` | "Start the night" (day) · close the morning summary | planning / summary |
| `WASD` / arrows, right-drag, wheel, `Home` | camera: pan, drag, zoom, frame the whole café | the world |

In cutscenes: click / `Space` — next line, `1–4` — choices, hold `Esc` — skip, `A` — auto-advance, hold `Ctrl` — faster.

Digits are taken by the speed controls, so priorities and the rozpis **don't use them** — everything there is done with the mouse.
Keys are bound to physical keys (KeyCode), so they also work with a Czech, Slovak or Ukrainian keyboard layout.

*(historical)* The v2 spec used `Q` crew, `W` priorities, `E` schedule, `F` / `Shift+F` to expand or hide toasts,
`H` hire, `C` "same as yesterday", `Tab` for build tabs and arrow / middle-button panning.

---

## 1. HUD over the map — `hud.svg`

The map fills the whole screen. HUD cards (translucent glass) overlap it at the top and bottom.

| Block | Anchor | Frame (pt) | Contents |
|---|---|---|---|
| **Clock and speed** | top left | `12, 12, 340 × 64`, radius 10 | `clock` icon 20 @ (24, 24) · time `01:40` 22/Bold, x 50, baseline 42 · "Fri · night 3" 12 `text_dim`, x 118 · 4 speed buttons 28×28 @ x 222 / 252 / 282 / 312, y 20 (step 30): `pause`, `play` ×1, `ff2` ×2, `ff3` ×3, the chosen one active · **night progress bar** `24, 58, 316 × 6`, radius 3 |
| **Event ticker** | top centre | `W/2 − 200, 12, 400 × 32`, radius 8 | category icon 16 @ (+12, +8) · time 12/SemiBold muted @ +36 · text 13/Medium @ +76 (cut with an ellipsis) · divider @ w − 44 · a `log` icon 16 and the letter "L" on the right |
| **Resources** | top right | `W − 352, 12, 340 × 64`, radius 10 | row 1 (baseline 38): `coin` 18 `amber` + "12 480 Kč" 16/Bold `amber` · `star` 18 + "4.2" 15/Bold and tonight's change "+0.1" 11 `good` · `guest` 18 + "14 / 20" 15/Bold · divider y 48 · row 2 (baseline 66, 12 pt): "+1 860 Kč tonight" `good` · "queue 3" · `rain` 14 + "rain, 9 °C" |
| **Crew on shift** | left edge | `12, 88, 56 × (12 + n·52)` | portraits 40×40, radius 8, @ (20, 96 + i·52) · under each, a 40×3 bar of their worst need · a red r 5 dot @ (57, y+3) if a need is < 25 % · the selected one has an `indigo` ring |
| **Screen toolbar** | bottom left | `12, H − 48, auto × 36`, radius 10 | ghost buttons h 28 @ y + 4, step 4: [`staff` Crew `C`] [`prio` Priorities `P`] [`calendar` Shiftella `T`] [`hammer` Build `B`] [`log` Log `L`] [`menu` 28×28]. Button width = 8 + 18 + 6 + text + 10 + 18 + 6. The open screen is shown as active |
| **Toast stack** | bottom right | `W − 332, bottom H − 12, 320 × 56` each, gap 8 | newest at the bottom, at most 3 visible, older ones go to the log (see §7) |
| **Name tag** | above the selected character (world) | 64 × 20, radius 10 | name 12/SemiBold + a mini 8×3 bar of their worst need |

**Night progress bar** (22:00 → 06:00, 8 hours, 39.5 pt per hour):
- track white 8 %, the elapsed part in a `violet → indigo` gradient, the current moment a white r 5 circle with a shadow;
- hour ticks 1×4, black 35 %; the last hour (05–06) tinted with "dawn" (`amber` 0 → 45 %);
- **trains** are small `amber` 6×4 triangles above the bar (e.g. 22:47, 00:15, 01:52, 05:20) and warn of a rush of guests;
- hovering the bar shows a tooltip with the time and the nearest event; clicking a triangle — "Train 00:15 · ~8 guests expected".

**Interactions.** Clicking a portrait in the crew column selects that person and centres the camera (a double click opens Crew on them). Clicking the ticker opens the log. Hovering the resources shows a breakdown (revenue, wages, supplies). The speed buttons show a tooltip with their key.

---

## 2. Crew — `staff.svg` (key `C`)

The panel is **docked on the left** and replaces the crew column. The game **doesn't pause**. Clicking the map outside the panel doesn't close it; `Esc`, `C` or ✕ do.

| Element | Frame / position |
|---|---|
| Panel | `12, 88, 600 × min(H − 148, 760)` (572 at 720), radius 12 |
| Header | "Crew" `h1` @ (28, baseline 116) · keycap `C` · "5 on shift · 7 in total" 12 `text_dim` · ✕ 24×24 @ (576, 100) · divider y 132 |
| **List** | x 24 … 328 · column headers baseline 152: "CREW MEMBER" (`micro`) and the `bolt` / `smile` / `bowl` 16 icons centred at x 230 / 266 / 302 |
| Row | `24, 160 + i·48, 304 × 44`, radius 8 · portrait 32 @ (34, y+6) · name 13/SemiBold @ (76, y+19) · status dot r 3 (indigo working, `amber` on a break, muted off shift) + the current activity 12 `text_dim` @ (89, y+36) · three need bars 28×4 @ (centre − 14, y+20) |
| Row states | hover — white 5 % · **selected** — `indigo` 13 % + a 3×28 bar on the left · off shift — name in `text_dim` |
| Sorting | "Sort by" 12 muted @ (32, 520) + h 24 segments @ y 506: [Name] [Energy] [Mood]. Clicking the selected segment flips the direction |
| Hire | button `24, 620, 132 × 28` (`plus`) + "candidates: 3" 12 muted |
| Divider | vertical x 340, y 140 … 648 |
| **Card** | x 352 … 600 |
| · header | portrait 64 @ (352, 144), radius 12 · name `h2` @ (428, 162) · "Server · 22 years · 150 Kč/h" 12 @ baseline 181 · current-job chip `info` h 22 @ (428, 192) |
| · NEEDS | header baseline 236 · rows y 246 + i·24: icon 16 @ x 352, label 13 @ x 376, bar 96×6 @ x 466, value 12/SemiBold right-aligned at x 600 (red if < 25 %) |
| · TRAITS | header baseline 340 · chips h 22 from y 350, gap 6, wrapping +28 · colour: positive `good`, neutral white 8 %, negative `bad` |
| · SKILLS | header baseline 420 · rows y 430 + i·22: name 13, a `heart` 14 = "favourite job", an `amber` bar 132×6 @ x 440 (0–10), the number right-aligned at x 600 |
| · RELATIONSHIPS | header baseline 560 · rows y 570 + i·28: portrait 22, heart / broken heart 14, name 13/SemiBold + relationship type 12 `text_dim`, value on the right (`good` / `bad`) |

**Interactions.** Clicking a row selects the person and moves the camera to them (no zoom). Hovering a bar shows the number and the reasons ("−12: a long time without a break"). Clicking a skill opens Priorities with this person highlighted. Everything is edited right here — no nested windows. "Let go" sits at the bottom of the card as a `danger` text button (not shown in the mockup): it acts at once, and Undo works for a while afterwards.

---

## 3. Work priorities — `priorities.svg` (key `P`)

| Element | Frame / position |
|---|---|
| Panel | `12, 88, 516 × 420`, anchored to the left edge, radius 12 |
| Header | "Work priorities" `h1` + keycap `P` · ✕ @ (492, 100) · divider y 132 |
| Job columns | `x0 = 216 + i·72`, width 72 · icon 24 @ (x0 + 24, 146) · label 12/SemiBold centred at x0 + 36, baseline 186 · the column under the cursor is highlighted white 5 % (`x0 + 4, 140, 64 × 52`) |
| Jobs | `pan` Kitchen · `cup` Bar · `register` Register · `broom` Cleaning *(the shipped game adds Serving)* |
| Row | `24, 200 + r·36, 492 × 34` (even rows white 2.5 %) · portrait 24 @ (30, y+5) · name 13/Medium @ (62, y+22) |
| Cell | `x0 + 12, y + 4, 48 × 26`, radius 6 (styles in style-guide §5.3) · a 2 pt skill bar at the bottom |
| Cell hover | a 2 pt indigo ring + a 196×52 tooltip to the right: "Míša · Register" / "skill 5 · wheel ↑↓ — change" |
| Legend | divider y 456 · 12 `text_dim` baseline 478: "Left-click — higher · right-click — lower · wheel · drag — paint" · 12 muted baseline 496: "Shift + click a column — everyone · 1 = most important, — = never" |

**Values:** `1` (most important) → `2` → `3` → `4` → `—` (never). A person takes the job with the lowest number; on a tie, the nearest one.
**Input — mouse only, nothing to confirm:**
- left-click raises the priority one step (`—` → 4 → 3 → 2 → 1), right-click lowers it, the wheel over a cell cycles it;
- **paint**: hold the left button and drag across cells, and they all get the first cell's value;
- `Shift` + click a column header sets the value for everyone; `Shift` + click a name — for all of that person's jobs;
- "can't" (a trait, an injury) is shown with hatching and ✕; clicks are ignored and the reason is in the tooltip.

---

## 4. Shift schedule (day phase) — `schedule.svg` (key `T`)

*(historical)* The v2 layout below is a free-form half-hour slot grid. The shipped game replaced it with the
**rozpis** tab of the in-game Shiftella app: a weekly grid of shifts per person (22–06, 22–02, 02–06, off),
edited as a **draft** and sent to everyone with **Publish**; the night uses the published rozpis. The app's
other tabs are Requests (swaps, dovolená, náhradní volno), Messages (@shiftella_bot), Board (announcements and
the předávka), Checklists, the Night digest and the Payroll basis (podklady pro mzdy).

A full-screen screen of the planning phase. The map is visible underneath, dimmed (darkness 55 % + `ink` 55 %).

| Element | Frame / position |
|---|---|
| Top bar | `0, 0, W × 48`, a panel with no radius · "Planning · day 4" `h1` @ (24, baseline 31) · "next night: Friday, 22:00–06:00" 12 `text_dim` @ x 232 · money on the right (`coin` + 16/Bold `amber`) |
| Main panel | `24, 64, W − 48 × 520` (max-width 1440, centred), radius 12 |
| Title | "Shift rozpis" 16/Bold @ (40, 94) + keycap `T` · forecast chips h 24 @ y 76 on the right: `rain` "rain 80 %" (`info`), `train` "trains 22:47 · 00:15 · 04:52", `guest` "forecast ~46 guests" · divider y 110 |
| Grid | `GX = 264`, slot = **30 min = 44 pt**, hour = 88 pt, 16 slots (264 … 968) · hour labels 11/SemiBold `text_dim` baseline 128 at hour boundaries · verticals: hour — white 7 %, half hour — 4 % |
| Right columns | "HOURS" centred at x 1010 · "KČ" centred at x 1090 · "ENERGY" bar 70×5 @ x 1150 + a warning icon 14 @ x 1226 if < 25 % |
| Row | `40, 136 + i·40, 1200 × 36` (even rows white 3.5 %) · portrait 24 @ (46, y+6) · name 13/SemiBold @ (80, y+17) · role 11 `text_dim` @ (80, y+31) |
| **Shift block** | `GX + a·44 + 2, y + 6, (b − a)·44 − 4 × 24`, radius 6, gradient `indigo_400 → indigo_600`, a 2 pt highlight on top (white 25 %) · label "22:00–01:00" 12/SemiBold white if the width is ≥ 90 · a problem shift has a 1.5 pt warning outline |
| Break | slot `amber` 22 % + a `cup` 14 `amber` in the middle |
| Day off | "day off" 12 muted, centred in the grid |
| Drag ghost | a dashed `indigo_300` outline + `indigo` 22 % fill, a "+1 h" tag above the cursor |
| **Coverage** | row y 424: "Coverage" 13/SemiBold + "have / need" 11 muted · in every slot a 38×22 "3/4" tag in `good` (enough) / warning (−1) / `bad` (−2 or worse) at 18 % |
| **Guests (forecast)** | area y 460 … 516: a 2 pt indigo curve + a gradient fill · trains — vertical `amber` 60 % lines + `train` 14 + time 11/SemiBold |
| Panel footer | divider y 552 · hint 12 `text_dim` baseline 572 · "Total 30.5 h · 4 575 Kč" 14/Bold on the right |
| **Inline warning** | `24, 596, W − 48 × 36`, warning colour 12 % + a 35 % outline, radius 8 · warning icon 18 + text 13 · one-click buttons on the right: "Cut to 4 h" (normal, 120×28) and "Keep" (ghost, 82×28) |
| Bottom actions | y 652, h 32: [`undo` Undo `⌃Z`] `24, w 148` · [`copy` Like yesterday] `180, w 152` · [`trash` Clear] ghost `340, w 120` · on the right **[`play` Start the night `Enter`] primary** `W − 204, w 180` |

**Input:** drag the left button along a row to draw a shift, `Shift` + drag for a break, right-click / right-drag to erase. Block edges can be dragged by their handles (↔ cursor). A click on an empty spot places one slot. "Clear" acts at once and can be undone.
Warnings (over 8 h, energy < 25 %, not enough people) are shown **in the strip under the grid** and in the coverage cells — no modal windows. At night the screen is read-only: blocks are translucent and the top says "Changes — during the day". *(Shipped: last-minute rozpis edits make the crew a little grumpy instead.)*

---

## 5. Build and furniture — `build.svg` (key `B`)

| Element | Frame / position |
|---|---|
| Panel | `12, H − 208, 1000 × 148` (y 512 at 720), anchored bottom left above the toolbar, radius 12 |
| Tabs | pills h 26, radius 13, @ y 524 from x 24, step 4: **[All]** Furniture · Appliances · Counters · Decor (the selected one active, the rest ghost) · then a `danger` "Demolish" `X` button · a hint 12 muted on the right, baseline 541, ending at x 1000: "R — rotate · right-click / Esc — cancel" |
| Cards | `24 + i·92, 560, 84 × 88`, radius 10, background white 5 % · a backing circle r 22 · icon/thumbnail 32 @ (+26, +12) · name 12/Medium centred at baseline +64 · price 12/Bold `amber` at baseline +80 |
| Card states | hover — white 8 % · **selected** — `indigo` 14 % + a 1.5 `indigo` outline · **not enough money** — icon and name muted, price `bad`; a click gives the toast "2 420 Kč short" |
| Selected item info | divider x 772 · from x 788: name 15/Bold baseline 578 · price and constraint 12/SemiBold `amber` baseline 598 · effect 12 `good` baseline 618 · other 12 `text_dim` baseline 636 |
| On the map | a tile grid (white 9 %, 1 pt) while build mode is open · the object's **ghost** at 55 % + its footprint: `good` (allowed, dashed 6/4) / `bad` (not allowed) · a price tag by the cursor: a 92×26 tag, radius 8, "−3 400 Kč" 12/Bold `amber`, offset (+8, −44) |

**Input:** click a card to pick an item, left-click the map to place it. **Money is taken at once, with no confirmation.** The mode stays on so you can place several in a row; leave it with a right-click or `Esc`. `R` rotates, `X` toggles demolish: clicking an object removes it and refunds 50 % of its price. Any action can be undone with `Ctrl+Z`.
Example items: Table 1 200 · Chair 450 · Coffee machine «Rapid» 3 400 · Stove 14 900 · Counter 900 (per tile) · Plant 350 · Holo sign 1 800 · Lamp 600 Kč. *(The shipped catalogue has 38 pieces; see `assets/data/furniture.ron`.)*
While the panel is open, toasts move up above it: the bottom of the stack = the top of the panel − 8 (when the window is narrower than 1340 pt).

---

## 6. Guest tooltip — `tooltip-and-event.svg` (left part)

Appears after **250 ms** of hovering a guest and follows the cursor at an offset of **(+16, +18)**. It flips at the right or bottom edge. Clicking a guest **pins** the tooltip (a pin appears beside it); clicking again or `Esc` unpins it. *(Shipped: regulars show a mood line instead and offer a **Talk** button when they have something to say tonight.)*

| Element | Position (from the tooltip's top-left corner) |
|---|---|
| Frame | `264 × auto` (196 in the mockup), radius 10, nearly opaque glass |
| Header | guest type 14/Bold @ (14, 24) · a chip on the right "`train` train 00:15" 11 h 22 (or "in a hurry", "regular") · divider y 40 |
| Wish | `bubble` 16 @ (14, 50) · "cappuccino + croissant" 13/Medium @ (38, 63) · status 12 `text_dim` @ (38, 80): "order being made · ~2 min" |
| Patience | `hourglass` 16 @ (14, 92) · "Patience" 13 · bar 90×6 @ (110, 97) (colour by value) · "~6 min" 12/SemiBold on the right |
| Mood | `smile` 16 @ (14, 118) · "Happy" 13/SemiBold (`good` / warning / `bad`) · "+12" on the right 13/Bold |
| Modifiers | up to 4 rows from y 152, step 17: sign 12/Bold (`good` / `bad`) @ x 38 + reason 12 `text_dim` @ x 64 |

---

## 7. Event notification — `tooltip-and-event.svg` (right part)

**Never modal, and it doesn't pause the game** (pausing on important events is an option in the settings and in Sandbox, off by default).

| Element | Position |
|---|---|
| Stack | anchored bottom right: `x = W − 332`, bottom = `H − 12`, width 320, gap 8, newest at the bottom, ≤ 3 visible |
| **Collapsed toast** 320×56 | category bar 3×36 @ (0, 10) · circle r 16 + icon 20 @ (20, 18) · title 13/SemiBold @ (56, 24) · subtitle 12 `text_dim` @ (56, 42) · "More" 12/SemiBold `indigo_300` on the right @ (308, 24) · 2 pt timer @ (10, 52) |
| **Expanded** 320 × auto | the same 56 header; instead of "More" on the right: `target` 16 (show on the map) @ (262, 12) and `chev_u` 16 (collapse) @ (290, 12) · divider y 56 · description 12/18 `text_dim` from y 76 (≤ 4 lines) · actions h 28 @ y 132: primary "`candle` Light candles" 160 + normal "Close kitchen" 126 · a note 11 muted @ y 182: "−150 Kč · buttons work while paused" |

Categories (bar and icon colour): `bad` — breakdown, theft, complaint · warning — risk, inspection · `info` — train, guests, weather · `good` — tips, a review · *(shipped: uncanny events use violet)*.
Display time: info 6 s, warning 10 s, bad 14 s. **Hovering stops the timer**, and an expanded toast never disappears on its own.
Input: click the toast or "More" — expand; right-click — dismiss; `target` — move the camera there; an action button acts at once, and the toast then collapses to the result ("Candles lit · −150 Kč · Undo"). If nobody answers, the event falls back to its default choice after a while.
The **event ticker** at the top repeats the latest event in one line (in the category colour), and the full history is in the log (`L`).
The mockup shows a blackout: darkness 80 %, candles on the tables, warning icons above nervous guests, the holo sign off.

---

## 8. Main menu — `main-menu.svg`

| Element | Position |
|---|---|
| Background | an illustration: the night sky, the silhouette of the city with a tower, a passing train, the café front with its holo sign and a warm window, wet asphalt with reflections, rain. A dark gradient on the left (`ink` 85 % → 0 by x 640) keeps the text readable |
| Logo | placeholder text "Shiftella" 52/Bold @ (64, baseline 152) + a soft indigo glow (shipped: 44/Bold). A final logo could be a separate sprite ≈ 440×64 |
| Subtitle | "Café Nocturne · 22:00–06:00" 15/Medium `text_dim` @ (66, 182) |
| Menu panel | `56, 250, 300 × 236`, radius 14 · items h 40 @ y 258 + i·44: icon 20 @ x 80, text 16/SemiBold @ x 112, a hint 12 muted on the right |
| Items | `continue` **Continue** "night 3 · 01:40" (only if there is a save) · `book` Story · `cube` Sandbox · `sliders` Settings · `exit` Quit |
| Selected item | `indigo` 16 % + a 3×20 bar on the left + an `indigo_300` icon + `chev_r` on the right |
| Footer | "↑ ↓ — select · Enter — open · Esc — back" 12 `text_dim` @ (64, 690) · the version 12 muted on the right @ (W − 24, 690) |

Input: mouse (hover = select) or `↑ ↓ Enter`. *(Shipped: the very first launch shows a language picker — CZ / EN / SK / UA — and the choice is saved in the settings. The game autosaves at the start of every day and on quit; there are three manual slots in the pause menu.)*

---

## 9. Sandbox setup — `sandbox-setup.svg`

The background is the same menu screen, dimmed with `ink` 72 %.

| Element | Position |
|---|---|
| Panel | centred: `W/2 − 400, 40, 800 × 640`, radius 14 · title "Sandbox — new game" 18/Bold @ (+32, +42) · ✕ · divider y +62 |
| Columns | labels 13/SemiBold @ x 272 (an 11 muted explanation under them) · controls @ x 460 |
| Café name | y 122: a 320×32 field, "Café Nocturne" |
| Difficulty | y 174: segments 132×32, step 136: **Cozy** · Classic · a description 12 `text_dim` at baseline y + 52 *(v2 had four: Cozy · Normal · Hard · Nightmare)* |
| Night character | y 250: a 420 pt slider with 5 steps (step 105): quiet … chaotic · on the right an accent chip with the step name: "quiet", "calm", "changeable", "stormy", "chaotic" · description at baseline y + 64: "2–4 events a night, now and then a big one…" |
| Start money | y 338: 3 segments 160×32, step 164: 10 000 · **25 000** · 50 000 Kč |
| Seed | y 390: a 200×32 field (14/SemiBold, tracking 1) · `dice` 32×32 · ghost "`copy` Copy" 132×32 · "R — random" 12 muted · explanation at baseline y + 52 |
| Options | y 462: 20×20 checkboxes, step 28: ☑ Rain every night · ☐ Pause on important events · ☑ Allow going into debt |
| Summary | `272, 558, 736 × 48`, `ink` 55 %, radius 8 · `info` 20 · "Expect: 2–4 events a night · 30–50 guests · start 25 000 Kč" 13 · "Best run on this seed: none yet" 12 muted |
| Buttons | y 628, h 32: "Back `Esc`" 132 on the left · **"`play` Start `Enter`" primary 168** on the right |

Input: `Tab` / `Shift+Tab` move the focus (an indigo ring), `← →` change a segment or step, `Space` toggles a checkbox, `R` gives a random seed, `Enter` — "Start". No drop-down lists: everything is one click.
