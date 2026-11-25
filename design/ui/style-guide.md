# Shiftella — UI style guide

Version 3 (v0.1.0). Style: **smooth vector 2D illustration — the "violet dollhouse"** (no pixel art).
A café by the station of a small Czech town in 2039, 22:00–06:00: rain, soft holo signs, lo-fi, cozy melancholy.

> **History.** Version 2 (2026-11-25) of this guide was written before implementation, with a neon-pink
> accent (`#FF4F9A`), the Manrope font for headings, `Q/W/E` hotkeys and a strictly top-down world. The
> shipped game replaced those with the **Shiftella indigo/violet palette**, Inter only, `C/P/T/B/L` hotkeys
> and a 3/4 front-view "dollhouse" world (see [`docs/GAME-BRIEF.md`](../../docs/GAME-BRIEF.md) §2). This
> version describes what shipped; the source of truth for the tokens is `src/ui/theme.rs`. The SVG mockups
> in this folder are the v2 layouts, regenerated with the shipped indigo palette and the shipped hotkeys,
> but their world art still shows the old top-down look — treat them as layout references; the real game
> is shown in [`docs/screenshots/`](../../docs/screenshots/).

Mockups: the `*.svg` files in this folder, preview: `index.html`, generator: `tools/gen_mockups.py`
(run `python3 -I design/ui/tools/gen_mockups.py` and every SVG is rebuilt from the same tokens).

---

## 0. The main rules (in short)

1. **Compact, but legible.** Body text 13 pt, small 12, buttons 28 pt tall. Nothing big for the sake of being big.
2. **One click instead of a dialog.** No "Are you sure?". The action happens at once, and Undo fixes mistakes (`Ctrl+Z` + an "Undone" toast).
3. **Nothing blocks the game.** Events arrive as toasts in the bottom-right corner, panels open over the map, and time keeps running.
4. **Hotkeys for everything frequent:** `Space`, `1/2/3`, `C/P/T/B/L`, `M`, `Esc`, `Ctrl+Z`; camera on `WASD`/arrows.
5. **One accent — Shiftella indigo.** It marks what is selected, active, and the primary action. Violet is the second accent; amber is the warm counterpoint. Other colours describe states (money, good, bad, info).
6. **Our own look.** We don't copy RimWorld's UI, icons or limbless pawns.

---

## 1. Coordinate system and scale

| Parameter | Value |
|---|---|
| Reference screen | **1280 × 720 logical points (pt)**. All sizes in these docs are in pt |
| Rendering | the window's native resolution, no virtual buffer and no pixel grid |
| World camera | a separate pan/zoom camera; the map fills the whole screen and the UI floats on top |
| `ui_scale` | `dpi_scale × user_scale`. Retina gives dpi ×2. `user_scale` is chosen in the settings: 75 / 90 / 100 / 110 / 125 / 150 %, and "Auto" = `clamp(floor(min(W/1280, H/720)·4)/4, 1.0, 1.5)` (1.5 on 1920×1080) |
| Anchoring | every HUD block is anchored to an edge or corner of the screen with a 12 pt margin (see `screens.md`). Central elements hold on to the horizontal centre |
| Ultra-wide screens | corners stay in the corners, the event ticker stays centred. Wide screens (the rozpis) are capped at `max-width 1440 pt` and centred |
| Minimum window | 1024 × 600 pt. Below that, `user_scale` drops to 90/75 % automatically |

Position formula: `x_px = anchor_px + offset_pt × ui_scale`. For right and bottom anchors the offset is negative.
Text and icons are rasterised at physical pixel size (`font_size = pt × ui_scale`), so they stay crisp on Retina.

---

## 2. Palette (tokens)

The Shiftella universe: a deep indigo-violet night canvas, frosted-glass panels, one indigo accent,
warm amber lamp light as the counterpoint. Gradients are allowed, but they are soft and stay within one hue.

### 2.1 UI (shipped, `src/ui/theme.rs`)

| Token | HEX | Role |
|---|---|---|
| `canvas` | `#0D0C17 → #121022 → #1A1730` | night canvas gradient behind menus and full screens |
| `violet_night` | `#120C1F → #201634` | violet night gradient (menu backdrop, cutscene fades) |
| `ink` | `#1B1A2E` | deepest ink, wells and input fields |
| `glass_top` / `glass_bottom` | `#2A2850` 74 % → `#17162C` 84 % | frosted-glass panel fill (vertical gradient) |
| `glass_solid_*` | `#26244A` 94 % → `#15142A` 96 % | nearly opaque glass for big screens and tooltips |
| `edge_top` / `edge_bottom` | white 24 % → white 5 % | the lit 1 pt gradient edge of every glass panel |
| `divider` | white 7.5 % | separators inside panels |
| `raised` / `raised_hover` | white 6.5 % / 11 % | buttons, cells, raised elements |
| `text` | `#EEEDF7` | main text |
| `text_dim` | `#A9A7C9` | secondary text, captions |
| `text_disabled` | `#67658C` | muted text, section headers, inactive |
| `indigo` | `#6366F1` | **the accent**: selected, active, primary button, staff aprons |
| `indigo_600` | `#4F46E5` | the darker end of accent gradients, shifts in the rozpis |
| `indigo_400` / `indigo_300` | `#818CF8` / `#A5B4FC` | glow, links ("More"), accent icons, keyboard focus |
| `violet` / `violet_400` | `#7C3AED` / `#A78BFA` | second accent: the night progress bar, the mood need, uncanny events |
| `amber` / `amber_deep` | `#FFC46B` / `#F99C4E` | money, prices, warm light, trains, breaks |
| `emerald` / `emerald_400` | `#10B981` / `#34D399` | good, profit, positive traits, hunger need |
| `red` / `red_400` | `#EF4444` / `#F87171` | bad, critical, dangerous actions, "can't" — used sparingly |

Translucencies that come up all the time:
- panel edge — the lit gradient edge (white 24 % at the top → 5 % at the bottom), 1 pt, inside the border;
- a light backing for a row, cell or chip — white **3–8 %**;
- a chip's tone — the state colour at **14–18 %**, with text in the same colour;
- a selected row — `indigo` at **13 %** plus a vertical `indigo` bar 3×28 pt on the left.

*(historical)* The v2 palette used `accent #FF4F9A` (neon pink), `accent_hi #FF78B4`, `accent_lo #B8306F`,
`cyan #52D6E6` for info/focus and `violet #6A5CFF`. Indigo/violet replaced pink everywhere; info and focus moved
to the indigo glow tones.

### 2.2 The world (not UI, but in the same palette)

| What | Colour |
|---|---|
| Floor, wooden boards | warm browns `#6A432F` / `#704732` / `#734A34`, seam `#4E3022` |
| Kitchen floor, tiles | `#262C47` (grout), `#323A5E` / `#2C3354` |
| Walls | indigo-violet tones with a visible front face and a lighter top edge |
| Counter | top gradient `#C79A6B → #A9784E`, front `#5E3A27 → #4A2D1F` |
| Tables | radial `#B98457 → #8C5D3C`, rim `#5C3A26` |
| Window glass | `#3C5591 → #22305A`, drops `#C6D4FF` 50 % |
| Night "darkness" | `#07061A`, 35–40 % (normal), 55 % (day/planning), 80 % (blackout) |
| Lamp light | radial `#FFC46B`/`#F99C4E` amber pools, additive/screen blend |
| Cold window light | lavender rim light from the windows (`#A5B4FC`-ish, 22 % → 0) |
| Rain | pale lavender streaks, 22 %, 1.2 pt stroke, 10–18 pt long, slanted −3.5 pt |

---

## 3. Typography

### 3.1 Fonts

| Font | Role | Licence | Coverage | Files |
|---|---|---|---|---|
| **Inter** (Rasmus Andersson) | the whole UI: text, numbers, buttons, headings | **SIL OFL 1.1** | latin, latin-ext (all Czech and Slovak diacritics: ě š č ř ž ý á í é ů ú ď ť ň ľ ĺ ô ä), cyrillic (Ukrainian), greek, vietnamese | static `Inter-Regular`, `Inter-Medium`, `Inter-SemiBold`, `Inter-Bold` in `assets/fonts/` (licence: `assets/fonts/OFL.txt`) |

*(historical)* v2 also suggested **Manrope** (SIL OFL 1.1) for headings and the clock. The shipped game uses Inter only.

**Important for macroquad.** `load_ttf_font` (fontdue) doesn't apply variable-font axes or OpenType features (`tnum`, `ss01`).
- We therefore ship **static** weights (see above), cut from the variable font.
- For **digits** (clock, money, tables) we align widths ourselves: every digit gets a fixed advance equal to the width of the "0" glyph. That way numbers don't "jump" (`TextStyle::tabular()`).

### 3.2 Scale (pt at ui_scale 1, shipped)

| Style | Font / weight | Size | Where |
|---|---|---|---|
| `display` | Inter Bold, tracking −0.6 | 44 | the logo in the main menu |
| `clock` | Inter Bold, tabular | 22 | the time in the HUD |
| `title` | Inter Bold | 18 | screen title |
| `h1` | Inter Bold | 16 | panel header |
| `h2` | Inter SemiBold | 14 | card titles, names in cards |
| `body` | Inter Regular / Medium | **13** | main text |
| `strong` | Inter SemiBold | 13 | buttons, toast titles, names in lists |
| `number` | Inter SemiBold, tabular | 13 | numbers in tables, money |
| `small` | Inter Regular, `text_dim` | **12** | captions, secondary text, descriptions; chips use Medium |
| `micro` | Inter Bold, tracking 0.6, `text_disabled` | 11 | SECTION HEADERS (uppercase) |
| `key` | Inter SemiBold, `text_dim` | 11 | keycap hints, axis labels |

There is no text below 11 pt. Long lines (descriptions) wrap to the container width; single-line labels are cut with an ellipsis and the full value is shown in a tooltip.

---

## 4. Grid, spacing, radii

- Base step **4 pt**. Allowed spacings: 2, 4, 6, 8, 12, 16, 24.
- Screen edge → HUD: **12 pt**. Between neighbouring HUD blocks: **8 pt**.
- Panel padding: **12 pt** (compact HUD blocks 10–12, big screens 16).
- Heights: button **28** (standard) · **24** (compact: sorting, tabs 26) · **32** (buttons on menu and planning screens) · list row **44** (two lines) · table row **34–36** · input field **32** · chip **22**.
- Radii: **4** (keycaps, small bits) · **6** (buttons, fields, cells) · **8** (ticker, tooltip, cards) · **10** (toolbar, toasts, HUD cards) · **12** (large panels) · **14** (menu panels). Pill chips have radius h/2.
  *The brief asked for 8–12 for panels; elements use 4–8 so corners don't look sharp on a 600-pt panel. It's one constant to change.*
- Panel shadow: `offset 0, +6`, blur 18, near-black 42 %. Small shadow (slider, marker): `0, +2`, blur 6, 30 %.

---

## 5. Components (specs)

All the components are shown together in `components.svg`.

### 5.1 Glass panel
- Fill: vertical glass gradient `glass_top → glass_bottom` (tooltips and big screens use `glass_solid_*`).
- Edge: the lit 1 pt gradient edge (white 24 % → 5 %), inside the border, plus a faint sheen. Radius 12 (10 for HUD cards).
- Panel header: an `h1` title at (16, 28 baseline), a keycap hint to its right, then a `small` counter. A 24×24 close button sits in the top-right corner, 12 pt in. A divider under the header at y = 44 from the top of the panel.
- In macroquad: rounded rectangles and soft shadows are drawn by `src/ui/draw.rs` and `glass.rs`.

### 5.2 Buttons (h 28, radius 6, text 13/SemiBold, icon 18, gap 6, padding 10)

| State | Fill | Text | Other |
|---|---|---|---|
| normal | `raised` | `text` | inner edge white 6 % |
| hover | `raised_hover` | `text` | 80 ms transition |
| pressed | `pressed` (black 22 %) | `text_dim` | content shifts 1 pt down |
| active / selected | indigo gradient `#6366F1 → #4F46E5` | `#FFF` | used for toggles (speed, tab, segment) |
| primary | the same gradient + a soft indigo glow | `#FFF` | one per screen: "Start the night", "Start" |
| ghost (quiet) | none | `text_dim` | hover gives white 6 % |
| danger | `raised` + a `red` outline | `red_400` | "Demolish". No confirmation — there is Undo |
| disabled | `raised` 45 % | `text_disabled` | no hover, "not allowed" cursor, the reason in a tooltip |
| focus (keyboard) | as current | | a 2 pt `indigo_400` ring, 2 pt outside |

An icon button is 28×28 with an 18 icon. A keycap hint: height 18, min width 18, text 11/SemiBold `text_dim`, background `ink` 70 %, a thin outline, radius 4. It sits inside the button on the right, 6 pt in.

### 5.3 Other elements
- **Segmented choice** (difficulty, start money, sorting) — a row of equal-width buttons with a 4 pt gap; the selected one is active. There are no drop-down lists anywhere.
- **Stepped slider** (the night's character): a 4 pt track (white 10 %), filled with a `violet → indigo` gradient, ticks r 3, a white r 9 handle with a small shadow. Ticks are clickable; `←/→` moves one step.
- **Input field**: h 32, `ink` 70 %, thin outline; when focused a 1.5 pt `indigo` outline and a 1.5 pt `indigo` caret; text 13.
- **Checkbox**: 20×20, radius 5. Off: `ink` 60 % + a muted outline. On: the accent gradient + a white 14 tick.
- **Need bar**: height 4 (lists) / 6 (card), radius h/2, track white 8 %. Needs have their own colours (energy amber, mood violet, hunger emerald); a value under 25 % turns `red`, pulses gently (1 Hz) and adds a red r 5 dot by the portrait.
- **Chip / trait**: h 22, radius 11, text 12/Medium, background in the tone colour at 16 %. Tones: good (positive trait), bad (negative), neutral (white 8 %), accent, info.
- **Priority cell**: 48×26, radius 6, number 14/Bold. `1` — the accent gradient, white text. `2` — `amber` 90 %, dark text. `3` — white 14 %. `4` — white 7 %, `text_dim`. `—` (doesn't do it) — a dashed white 10 % outline. "Can't" — `red` hatching at 35 % and ✕. A 2 pt bar at the bottom of the cell, its width ∝ skill.
- **Portrait**: a front-facing bust in a rounded square (radius 8 at 40 pt, 6–7 at 24–32 pt) on a deterministic avatar colour. The selected one gets a 2 pt `indigo` ring.
- **Toast**: 320×56, radius 10, a 3×36 category bar on the left, a 20 icon in an r 16 circle (same colour, 14 %), title 13/SemiBold, subtitle 12 `text_dim`, a "More" link 12/SemiBold `indigo_300` on the right. A 2 pt auto-hide timer along the bottom (category colour, 70 %).

### 5.4 Motion
- Hover and state changes: 80 ms ease-out. Opening a panel: an 8 pt slide + fade over 120 ms. A toast slides in from the right in 160 ms.
- No "jumping" helpers and no windows that open on their own.
- Changing numbers (money) flash briefly (200 ms). A "+120" / "−3 400 Kč" floats up beside them.

---

## 6. Icons

**Style: outline (stroke), a 24×24 pt grid, 2 pt stroke, round caps and joins, a 20×20 live area (2 pt padding), no fills.**
The colour comes from a tint (`text`, `text_dim` or a state colour). A separate 1.5 pt variant is drawn for 16 pt (plain downscaling gives 1.33 pt and looks blurry).
In the shipped game the icons are drawn as vector strokes in code (`src/ui/icons.rs`).

The set (name → where it's used). Every glyph is in `components.svg` and in the `<symbol id="i-…">` blocks of any mockup:

| Group | Icons |
|---|---|
| Time / speed | `clock`, `pause`, `play` (×1), `ff2` (×2), `ff3` (×3), `continue` |
| Resources | `coin` (money), `star` (reputation), `guest` (guests), `rain` (weather), `train` (train) |
| Screens (toolbar) | `staff` (C, crew), `prio` (P), `calendar` (T, the Shiftella app), `hammer` (B), `log` (L), `menu` (Esc) |
| Needs | `bolt` (energy), `smile` (mood), `bowl` (hunger) |
| Jobs | `pan` (kitchen), `cup` (bar), `register` (register), `broom` (cleaning) |
| Guest | `bubble` (wish), `hourglass` (patience), `smile` (mood) |
| Relationships | `heart`, `heart_broken` |
| Events | `bulb_off` (lights), `warn`, `info`, `candle`, `target` (show on the map) |
| Actions | `close`, `chev_d/u/r`, `check`, `plus`, `undo`, `copy`, `trash`, `rotate`, `dice`, `cursor` |
| Build (categories and items) | `table`, `chair`, `machine` (coffee machine), `stove`, `counter`, `leaf` (plant), `neon` (holo sign), `lamp` |
| Menu | `book` (Story), `cube` (Sandbox), `sliders` (Settings), `exit` (Quit) |

---

## 7. World art — the "violet dollhouse"

### 7.1 Camera and tile
- **Floors from above, everything else in a gentle 3/4 front view.** Floors and rugs are seen from above; furniture, walls and characters show a visible front face plus their top, have height and are y-sorted — like a lit dollhouse at night.
- **1 tile = 64 world pixels** at zoom 1 (`map::TILE = 64.0`).
- Camera zoom with the mouse wheel; pan with **WASD / arrows** or by dragging with the right mouse button. `Home` frames the whole café.

*(historical)* v2 planned a strictly top-down orthographic camera with no side faces, head-and-shoulders
characters seen from above and middle-button panning. The brief replaced it with the dollhouse view above.

### 7.2 Shape and outline
- **No black outlines.** Edges are shaped by soft gradients, shading of the same hue and rim light. Silhouettes read through light, shadow and contrast with the floor.
- Simple shapes with generous rounding, 2–3 tones per object: base, a highlight on the lit side, shade.
- Furniture is a tone **lighter than the floor** (a `#B98457` table on a `#6A432F` floor). Appliances are cool and metallic, so they stand out on the warm floor at once.

### 7.3 Characters (an original silhouette)
- **Upright, friendly proportions, about 1.3 tiles tall**, drawn in the same 3/4 front view: readable faces, hair and clothes, visible arms that animate (carrying a tray, wiping a table, holding a mop).
- This is **not** a RimWorld pawn (a limbless front-facing sack with a head on top).
- **Staff** wear **indigo aprons** with a small Shiftella badge. Guests wear a free palette of muted tones.
- A selected character: a 2 pt `indigo` ring at their feet and a name tag above the head.
- Work indicator: a small circle above the head with the job icon and a ring of progress.
- Animation: a 1–2 px bob while walking, arm swings, facing changes. A seated guest sits at the table and faces it.
- UI portraits are drawn **separately** (a front-facing bust) but reuse the same character's hair, skin and clothes colours.

### 7.4 Shadows
- A soft contact shadow under every object and character: an ellipse or rounded shape, offset slightly down, blurred, near-black 35–45 %.

### 7.5 Night lighting
1. **Base darkness:** a `#07061A` layer at 35–40 % over the world, with "holes" under the lamps (a light mask).
2. **Warm lamp pools:** an amber radial gradient r ≈ 2.5–3 tiles, additive/screen blend, 50–60 %. A pendant lamp over each table gives its own little island.
3. **Cool window light:** lavender rim light from the windows into the room. Rain streaks run down the glass, animated.
4. **Holo signs outside:** a soft indigo/violet glow around the «Café Nocturne» holo sign (it flickers now and then) and reflections in the puddles on the platform.
5. **Rain outside:** slanted streaks over the whole outdoor area, 2 parallax layers. No rain indoors.
6. **Trains:** a passing train sweeps a band of light across the room.
7. **Through the night:** the darkness is colour-graded — 22:00 violet, 03:00 the deepest, 05:30 a blue-lavender dawn (darkness drops to 25 %).
8. **Blackout:** darkness 80 %, lamps and holo signs go out, candles on the tables (a pool r ≈ 1.5 tiles, flickering ±8 %).
9. **Small glows:** subtle bloom on screens and holo signs, steam over cups and from the coffee machine.

---

## 8. UI implementation in macroquad (a note for programmers)

- **UI camera** in logical points: a `Camera2D` with zoom for `ui_scale`. The world camera is separate. (On macOS the screen camera needs a **positive** `zoom.y`; see `docs/GAME-BRIEF.md` §3.)
- **Rounded shapes, glass and shadows:** `src/ui/draw.rs`, `src/ui/glass.rs`.
- **Text:** `draw_text_ex` with `font_size = round(pt × ui_scale)` and `font_scale = 1 / ui_scale`; widths come from `measure_text`. All digits use a fixed advance (§3.1). See `src/ui/text.rs`.
- **Hit tests** use rectangles in logical points converted to pixels.
- **Undo:** one shared command stack in the sim (build, rozpis, priorities), `Ctrl+Z`; after undoing, an "Undone" toast.
