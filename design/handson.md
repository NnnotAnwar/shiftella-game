# Hands-on update — Karolina behind the counter

Status: design, 2026-11-26. Owner: lead game designer. Implements `docs/GAME-BRIEF.md` §6 (incl. §6.4 seasons,
themes, coziness, journal and §6.5 the rozpis puzzle) and refines it. The heroine is **Karolina**, grandma Lida's
granddaughter and heir (uk: Кароліна; feminine forms are fine in cs/sk/uk).
Readers: sim agent (`src/sim/**`), content agent (`assets/data/**`, `assets/i18n/**`, `src/content/**`),
UI-kit and mini-game agent (`src/ui/**`, `src/minigame/**`), integration agent (`src/screens/game/**`, `src/game.rs`, saves).

Tags: **MUST** = ships in v0.1.0 · **SHOULD** = right after, if time allows · **LATER** = backlog.

---

## 0. The principle, and what changes

> Owner, 2026-11-26: *"I don't want to just watch the game play itself."*

This principle overrides anything softer in the brief. Karolina is the main actor **every night**.
The café never runs on autopilot.

1. **Early nights (1–2): she works alone.** The player does the whole loop by hand: greet → take the
   order → read the wish → pick a recipe → brew (mini-game or quick brew) → serve → collect the tip →
   clear → wash → mop. Every one of these is a click or a short gesture in the world.
2. **Staff are helpers, not autopilot.** From night 3 the player hires helpers and **delegates
   duties** to them ("Kuba does the dishes", "Oleh takes plain coffee orders"). Each helper has limited
   capacity (1 to 3 duties). They are slower and less charming than Karolina, and their work tops out
   at quality 0.75.
3. **Player-only:** wishes, regulars, recipe-book specials, night moments, talking, story beats,
   the window light, experiments. Helpers can never take these over, except one wish at a time through
   an explicit **Hand off** button, at reduced quality.
4. **No idle stretches.** At ×1 there should almost always be 1–3 meaningful things to do. A **lull
   director** (§1.6) fills quiet gaps with a moment, a regular, a walk-in or a recipe to try.
5. **Rushes are exciting, not stressful.** On Cozy, waiting guests are patient and forgiving, nobody
   leaves angry, and quick serves build a small, purely positive "rhythm" glow.
6. **×2/×3 and Day planning stay** for pacing. The heart of the game is hands-on play with the mouse.

### 0.1 MUST list (v0.1.0)

| # | Item | Section |
|---|---|---|
| M1 | Karolina is player-controlled only: no auto job picking; action queue (up to 5) | §1.1, §7.2 |
| M2 | In-world micro-actions: greet, clear table, wash (hold), mop (swipe), collect tip coin, light window | §1.2 |
| M3 | Order card: wish bubble, recipe pick, **Quick brew / By hand**, **The usual?**, **Hand off** | §1.2, §7.4 |
| M4 | Ticket rail (pending orders, who is on them, click to claim) | §7.4 |
| M5 | Delegation: 6 duties, capacity 1–3, player-only rules, remove the "everyone helps" fallback | §1.3, §7.2 |
| M6 | Wait clock + Cozy forgiveness + overlay time scale + ×1 tick length 0.15 s | §1.4 |
| M7 | Mini-games MilkFroth, LatteArt (heart/tulip/rosetta + 3 seasonal), PourOver, TeaSteep, Plate; skip = 0.7; Relaxed setting | §2 |
| M8 | 12 Taste tags, 43 wishes, match score, reactions, tips, trust | §3 |
| M9 | 16 base recipes + 6 specials, pantry of 22 ingredients, experiments with hints, recipe book (K) | §4 |
| M10 | 10 night moments + moment director (2–4 a night) + lull director | §5, §1.6 |
| M11 | Station cat affection (levels 0–10) | §6.2 |
| M12 | Tutorial nights 1–3; first hire moves from night 2 to night 3 | §8 |
| M13 | Save version 2 (all new state serde-defaulted) | §7.3 |
| M14 | **Jukebox**: 6 real records, picker, crossfade, house playlist, regulars' likes/dislikes, unlocks (start with 3) | §10 |
| M15 | **Seasons** over 30 nights (late autumn → frost → Advent garlands → first snow); sandbox cycles | §11.1 |
| M16 | **Decor themes** (4 + seasonal) and the **coziness** value + atmosphere meter | §11.2–11.3 |
| M17 | **Polaroid journal** (J): the night's best moment re-rendered from a scene snapshot, chosen in the morning | §11.4 |
| M18 | **Rozpis puzzle**: staff preferences, wellbeing & fairness score, effects, wish-aware auto-fill | §12 |
| M19 | **The Shiftella app as a real tool**: every tab feeds the night (checklist rituals, bot prompts, handover, digest, payroll) | §13 |
| M20 | No new generated audio: every sound maps to the existing 10 SFX + 9 music files | §14 |

### 0.2 What the player DOES each night (the contract)

| Nights | Helpers | What the player's hands do | What helpers do |
|---|---|---|---|
| 1–2 (solo) | 0 | everything: greet, take orders, wishes, quick/by-hand brewing, serve, tip coins, clear, wash, mop, 1–2 moments, the window light | — |
| 3–7 | 1–2 (cap 1 each) | regulars + every wish, specials, moments, talking; picks up whatever the ticket rail shows as unowned (often plain food or dishes) | one duty each: usually dishes, then register (plain orders) |
| 8–16 | 2–3 (cap 1–2) | regulars and wishes (about half of all guests), 2–4 moments, recipe experiments in lulls, the quiet hour, arc choices; rush rhythm after trains | plain orders end to end (register + bar/kitchen + serving), dishes, floor |
| 17–30 | 3–5 (cap 1–3) | the same, plus story nights: specials for regulars' arc nights, seasonal latte art, the 3:33 table, the letter | they carry the plain half; the player is never idle, because wish share and moment count rise with act |

A sample night (night 5, Friday, rain, one helper on dishes), at ×1:

| Clock | Player does | |
|---|---|---|
| day | reads the handover note and the digest's tip; fixes one rozpis conflict (Ema has lectures on Tuesday); taps "Opening" | app |
| 22:00 | lights the window and warms up the machine (opening ritual), puts on *Bossa for a Rainy Platform*, greets two travellers | micro |
| 22:06 | takes a traveller's wish "something warm to hold" → Chai → TeaSteep (11 s) | mini-game |
| 22:15 | second traveller, plain cappuccino → Quick brew; meanwhile clicks the tip coin | parallel |
| 22:40 | "Wipe the fogged window": the Os 2141 sweeps light through the room | moment |
| 23:05 | Mirek: "The usual?" → one click → PourOver | regular |
| 23:30 | experiments in the book during a lull: honey + linden → *Linden tea* discovered | book |
| 00:15 | EN 471 arrives: 4 parties at once; ticket rail fills; 2 wishes by hand, 2 plain tickets quick-brewed; rhythm ×3 | rush |
| 00:40 | Vera: "Tea, properly made" → TeaSteep, narrow band | regular |
| 01:30 | station cat at the door → ham from the chlebíček | moment |
| 02:10 | Hana, 12 minutes: hot and fast → Quick cappuccino; serves within 3 min, Hana smiles | regular |
| 03:00–04:00 | quiet hour: a student asleep → blanket; she makes herself a coffee | moment |
| 05:00 | clears the last tables, mops the entrance; collects tips; closing ritual: cash up, handover note | micro |
| 06:00 | morning: picks tonight's polaroid (Vera laughing over her tea) for the journal | journal |

---

## 1. Karolina behind the counter

### 1.1 The actor (MUST)

* The existing owner `Staff { owner: true }` becomes the **player actor**. Look preset `player`.
* The sim never assigns her tasks. Her task comes only from `PlayerState.queue` (FIFO, max 5 actions).
  - Click = append. **Shift+click** = do next (insert at the front). Click a queued icon above her head = remove it.
  - Queue icons (16 px, glass chips) float over her head. The current action shows a thin progress ring.
* Speed: walks at `PLAYER_WALK_MUL = 1.4` × staff speed. Her work speed is skill-independent: `PLAYER_WORK_MUL = 1.0`
  (helpers use `0.5 + skill × 0.1`, then × `HELPER_SPEED = 0.85`).
* She never tires on Cozy. On Classic her energy drains at 50 % of the staff rate. At energy < 25 a
  "make yourself a coffee" chip appears (§6.10), and the drink restores 40.
* **Step back** (MUST): the player may simply stop clicking. The café does not collapse. Helpers keep
  doing their duties, and wish guests wait (on Cozy they forgive and leave content, §1.4). There is no
  hidden autopilot. An *accessibility* setting `assist_wishes` (default **off**) lets helpers hand off
  wishes after 15 unclaimed game minutes, capped at match 0.5.

### 1.2 The loop, step by step (MUST)

| Step | Player input | World / sim | Feedback |
|---|---|---|---|
| **Greet** (optional) | click an arriving guest within 5 game min of entry | she waves from where she stands (no walk) | guest +3 satisfaction, `Smile` emote; regulars +0.5 trust |
| **Seat** | automatic (existing seat logic) · regulars: their favourite seat if free (§6.3 reserve) | — | — |
| **Take order** | click a seated guest with the `!` bubble (or the ticket on the rail) | `PlayerTakeOrder { guest }` → she walks over → `PlayerOrderTaken { guest, wish }` | order card opens near the guest |
| **Read wish** | — | order card: wish text + faint taste icons; plain orders show the item instead | — |
| **Pick recipe** | click one of up to 4 suggested recipe chips, or **K** for the whole book; **The usual?** for regulars at trust ≥ 50 | hovering a chip lights up its tags that match the wish | — |
| **Brew** | **By hand** (mini-game, 8–20 s) or **Quick** (no mini-game) | by hand: overlay opens at once, she walks to the station in the background; quick: she goes and works | §2 |
| **Serve** | automatic: `PlayerBrew { guest, recipe, quality, mode }` → walks, works the remaining time, serves | `PlayerServed { guest, recipe, match_score, quality }` | reaction bubble + emote (§3.5) |
| **Pay / tip** | when the party leaves, a coin glints on the table; click it | `TipLeft { table, amount }` → `CollectTip` | coin arcs to the tip jar on the counter, soft chime, `+38 Kč` float |
| **Clear** | click a table with dishes | she picks up the cups and carries them to the sink | clink |
| **Wash** | click the sink with a stack → **hold** LMB/Space (1 s per 2 dishes) | stack shrinks; releasing early keeps the progress | sponge squeak, bubbles |
| **Mop** | click a dirt blot → **drag** across it once (keyboard: Space) | tile dirt → 0 | swish, a little shine |

* **Plain vs wish orders.** Every party's leader gets `OrderKind::Wish` or `OrderKind::Plain`:
  regulars always Wish (or "The usual" once trusted); other guests Wish with chance 0.4 on nights 1–7
  and 0.5 from night 8 (sandbox: 0.45). Other party members always order Plain.
* **Quick brew:** the base recipe of the plain item is pre-selected. She works the full `prep_minutes`,
  and quality = `QUICK_QUALITY 0.6` (0.8 once that recipe is *practised*, §2.0). For wishes, Quick is
  allowed too. The match counts, the quality is lower, and there is no mini-game flourish.
* **By hand:** the mini-game *is* the work. After it she works only `prep_minutes × 0.3`, then serves.
* Defaults: plain = Quick, wish/regular = By hand (the toggle remembers the last choice per order kind).
  This is the main guard against mini-game fatigue.
* **Hand off** (on the order card; visible when a helper with `register` + `bar`/`kitchen` duty is on
  shift): the wish becomes a plain ticket for the best-matching base recipe. Match is capped at 0.5,
  and it earns no story or trust credit beyond the base serve.

### 1.3 Delegation: helpers and duties (MUST)

Duties replace the 5×4 priority grid for gameplay. The `priorities` field stays for save compatibility,
derived from duties: on → 1, off → 0.

| Duty | Tasks it allows | Job (skill used) |
|---|---|---|
| `dishes` | ClearTable, Wash; puts left tips in the jar | Cleaning |
| `floor` | Mop, Bucket, cleaning checklist items | Cleaning |
| `register` | TakeOrder for **Plain** orders (register queue + tables), payments | Register |
| `bar` | Prepare **plain** Bar items, base recipes only | Bar |
| `kitchen` | Prepare **plain** Kitchen items, base recipes only | Kitchen |
| `serving` | Serve items prepared by helpers (never Karolina's: she carries her own) | Serving |

* **Capacity** = `1 + [nights_worked ≥ 3] + [assigned-duty skill ≥ 7 or trait hard_worker]`, max 3.
  Shown as dots on the helper card. Growing capacity is the helpers' progression ("Kuba can take one
  more duty").
* **Helper quality** (plain items) = `0.35 + 0.04 × skill`, max 0.75. Helper items never get latte art.
* **Hiring is the delegation moment.** The hire card proposes one duty from the archetype's likes
  (barista → bar, cook → kitchen, porter → dishes, cashier → register, sound_tech → bar,
  quiet_one → floor, server → serving). One click accepts it.
* **Duties board (P)** replaces the priorities screen: rows = helpers, columns = 6 duty chips, capacity
  dots, and a "Suggest" button that fills by skill. It is also editable per helper in the crew panel (C).
* **Ticket rail** shows any task with **no owner** (no helper holds that duty) with a hollow ring.
  The player takes those by hand. Nothing is ever stuck: Karolina can do any task.
* Removed: "Nobody on shift with a job enabled → everyone helps at 4" (SIM.md §2). Helpers only do their duties.
* Helpers emote `Note` ("on it!") when they pick up a duty task. The morning summary credits them
  ("Kuba washed 46 cups") via new digest lines.
* Never delegable (enforced in sim): Wish orders, any regular's order, recipe-book specials, moments,
  Talk, story beats, the window light and jobless checklist items, experiments, greeting.

### 1.4 Pacing and time (MUST)

* **Tick length:** `REAL_SECONDS_PER_TICK_X1 = 0.15` (currently 0.1). A night is 12 real minutes at ×1,
  6 at ×2 and 4 at ×3. This is a tuning constant in `handson.ron`, confirmed by the player-bot test (§9).
* **Overlay time scale:** while the order card, a mini-game, the book or a moment card is open,
  the integration multiplies the sim dt by `overlay_scale`: **Cozy 0.1**, **Classic 0.35**. While an
  overlay is open, speed is clamped to ×1. Rain, steam and people keep drifting slowly, so the room never freezes.
* **Wait clock** (per party, in sim): it drives patience and every `ServedFast` measurement.

  | Party state | Wait clock rate |
  |---|---|
  | unclaimed, waiting to order or for food | 1.0 |
  | claimed by Karolina (she is walking over or brewing for them) | 0.5 |
  | Cozy, during a rush (`Rush` active, §1.5) | × 0.67 on top |

* **Cozy forgiveness:** when patience reaches 0, a guest does not leave angry. They take a glass of water
  (a small visual), and patience refills once to 50 %. After the second run-out they leave *content*
  (satisfaction 50, no review penalty, line "I'll come back tomorrow."). Regulars never leave before
  their `leave_by`. Classic keeps today's behaviour, with a mild review.
* **Regular floor:** on Cozy, regular patience is at least 12 game min. A regular's arrival is announced
  2 game min ahead by a portrait chip on the rail ("Radek — to go"), while they are visible crossing the
  street. Content: loosen streak thresholds Hana 4→6 and Radek 3→5 min (they now measure the wait clock).
* **Speeds:** ×2/×3 are for quiet stretches and helper-heavy late nights. In a lull the HUD shows a
  one-click "It's quiet — ×2?" chip, and it never auto-speeds.

### 1.5 Busy nights (MUST)

* Rush sources: train arrivals (`TrainPassing` + `guests`), `GuestRush` events, concert crowds. The sim
  sets `Rush { until }` for 20 game min after a burst of ≥ 3 parties.
* During a rush: the rail auto-sorts by wait clock, the café music layer shifts to `mus_rush_loop`
  (existing), and Cozy patience runs at 0.67.
* **Rhythm** (purely positive): serves by Karolina within 4 game min of the previous one build a streak
  shown as a small amber glow on the tip jar: tips +4 % per step, max +20 %. A gap simply ends it, with
  no penalty and no "combo broken" message.
* Story: there are no rushes before night 3. Night 6's `concert_crowd` comes after the first hire.

### 1.6 The lull director (MUST)

Karolina is **idle** when her queue is empty, no party waits unclaimed, no moment is open and nothing
dirty is unowned. If she has been idle ≥ `lull_minutes` (6 game min ≈ 9 s at ×1; 15 in the quiet hour),
the director picks the first that applies:

1. A present regular with `talk_available` → their Talk chip pulses.
2. An eligible night moment → spawn it (ignores the moment gap, still ≤ 5 a night).
3. Recipe-book nudge: an undiscovered recipe has a pending hint → a "Grandma's margin note" chip (opens K).
4. A walk-in: `GuestRate ×2` for 10 game min (sandbox and story nights ≥ 3).
5. The ×2 chip.

### 1.7 Feedback and story (MUST)

* Served → reaction bubble (text key by match tier, §3.5) + emote: `Sparkle` (perfect), `Heart` (good),
  `Smile` (okay), `Coffee` (not quite). New emotes: `Sparkle`, `Smile`; the render agent mirrors them.
* Regulars: a floating "+♥" over their portrait chip when trust rises; a trust bar in their hover card.
* Latte art stays visible on the served cup (a tiny glyph); guests at adjacent tables get +1 satisfaction once.
* **Satisfaction** += `10 × match + 6 × quality` (on top of the existing serve calculation).
  **Trust** (regulars) += `1 + 3 × match + 2 × quality` per Karolina-made item.
  **Tip multiplier** = `0.6 + 0.8 × match × (0.5 + 0.5 × quality)` × rhythm.
* New story hooks (conditions, §7.3): `WishMatchStreak { regular, min, visits }`, `ServedRecipe { regular, recipe }`,
  `RecipeKnown`. Suggested content changes:
  - Mirek stage 1: `Any([ServedFastStreak(...), WishMatchStreak(regular: "mirek", min: 0.8, visits: 2)])`.
  - Hana stage 1: same pattern.
  - Vera stage 3: `ServedRecipe(regular: "vera", recipe: "r_kulajda")` (replaces `ServedItem`).
  - Lori stage 1 bonus: serving her `r_lori_encore` → `Trust("lori", 5)`.
  - 3:33 guest: serving `r_rn333_tea` → `AddCounter("tea_two_sugars", 1)`. The existing `strange_order`
    event becomes the discovery path (§4.4).

---

## 2. Mini-games

### 2.0 Common rules (MUST)

* API (brief §6): `start(kind, MiniGameParams { seed, variant, relaxed, regular })`,
  `update(&mut Ui, dt) -> MiniGameStatus { Running, Done { quality } }`, `draw(&mut Ui, rect)`.
* Overlay: glass panel 520×340 pt, bottom-centre, the café dimmed 35 % behind it. The title is the recipe
  name, with the guest's portrait chip and the wish line in small type at the top. The skip button sits
  bottom-right (**Tab**).
* Every game: **mouse first**, with a full keyboard alternative (Space = hold/primary, arrows/WASD = direction,
  Enter = done, Tab = skip, Esc = skip and close). Duration 8–20 s.
* **Skip = quality 0.7.** No timer pressure beyond the natural end of the gesture.
* Quality labels (never negative): ≥ 0.9 "Beautiful", ≥ 0.7 "Lovely", ≥ 0.4 "Good", else "Homely".
* **Relaxed** setting (`settings.relaxed_minigames`, default off; on by default if the player picked
  Cozy *and* turned on "reduce motion"): tolerance windows ×1.6, quality floor 0.5, no auto-end (a
  **Done** button), and the guide stays visible.
* **Audio cues** for every timing window (a soft chime at the sweet spot), so nothing depends on colour alone.
* **Mastery:** after 10 by-hand plays of a recipe at ≥ 0.7 it becomes *practised* (a small star in the
  book). Quick brew for it rises to 0.8, and its sweet spot shows a sparkle. Long sessions get less grindy.
* **Anti-repetition:** the order card suggests By hand for at most 3 same-kind mini-games in a row, then
  defaults to Quick for that kind until another kind is played (the player can still choose). Each recipe
  has 2–3 variants picked by seed.

### 2.1 MilkFroth (MUST)

Recipes: cappuccino, hot chocolate, raf, Lori's encore; svařák until MulledSpice ships.
* **Look:** side view of a steel pitcher; a thermometer arc on the right; foam rises in soft bubbles.
* **Controls:** hold LMB / Space to steam. Temperature rises steadily, and foam rises while held.
  Release when the needle sits in the amber band. Re-pressing is allowed (pulsing).
* **Duration:** 6–10 s.
* **Quality:** `q = 0.75 × clamp(1 − |T_release − T_target| / tol) + 0.25 × clamp(1 − |foam − foam_target| / 0.3)`.
  Cappuccino: `T 62 °C ±4`, foam 0.6. Hot chocolate: `T 58 ±6`, foam 0.3. Raf: plus one tap when the
  vanilla drop forms (±0.4 s, worth 0.15 of q).
* **Feel:** rising hiss → soft rumble → a gentle "chirp" when entering the band; steam curls over the panel edge.
* **Variety:** Lori's encore has a chili flake "pinch" tap; first-snow nights get a cinnamon dust swirl
  (cosmetic); practised = a thin sparkle on the band.

### 2.2 LatteArt (MUST)

Recipes: oat flat white, moon latte. Optional finish after MilkFroth for a *Fancy* wish (the order card shows "+ art").
* **Look:** top-down cup, crema gradient; a faint guide pattern.
* **Controls:** hold LMB to pour while dragging the spout from the start dot along the guide; release lifts.
  Keyboard: hold Space to pour. The spout follows the guide automatically, and Left/Right taps at the
  marked wobble points (rosetta leaves) are scored for timing instead of tracing.
* **Duration:** 8–14 s.
* **Quality:** resample the guide polyline to 48 points. `coverage` = share of guide points within 12 px
  of the trace; `stray` = share of trace samples farther than 24 px. `q = 0.6 × coverage + 0.4 × (1 − stray)`.
  Keyboard mode: `q = mean(1 − |Δt| / 0.35)` over the wobble taps.
* **Patterns** (`minigames.ron`): heart (night 1+), tulip (5+), rosetta (10+), leaf (14+), swan (20+).
  Seasonal/special: snowflake (nights 21–30 or Snow), crescent moon (moon latte), little train (Radek's
  night 23, night 30), cat face (cat affection ≥ 3), music note (when Lori is present), a heart with an
  "L" (Vera's memory night). Regular asks: Filip "a cat?", Hana "a heart, I need one" (the pattern is
  pre-selected by the wish).
* **Feel:** milk ribbon sound, crema ripples; the finished art fades into the cup glyph on the table.
* Anwar cameo (flavour only): a Beautiful heart for him switches his next ambient line to
  `regular.guest_anwar.ambient.art` ("…that's going in the game."). No flags.

### 2.3 PourOver (MUST)

Recipes: black coffee, turek, night train, espresso tonic.
* **Look:** top-down dripper with three faint rings; a gooseneck kettle stream follows the cursor.
* **Controls:** phase 1 *bloom*: a short pour (hold ~1 s), then the prompt "let it bloom…" (2 s; don't
  pour). Phase 2: hold LMB and move in slow circles over the rings. A soft metronome ticks in time with
  the rain (0.8 Hz). Keyboard: hold Space to pour, alternate Left/Right on the ticks.
* **Duration:** 12–18 s.
* **Quality:** `0.5 × rhythm` (share of time the angular speed is within 0.6–1.2 rev/s; keyboard: on-beat
  taps ±0.2 s) `+ 0.3 × centred` (radius inside the ring band) `+ 0.2 × bloom` (1 if no pour during the wait).
* **Variants:** *Turek* = three stir clicks, then "let the grounds settle" (3 s, don't touch), scored on
  stillness. *Espresso tonic* = hold to pour slowly over ice (a fast pour fizzes up in a cute way, which
  lowers q). *Night train* = a final cocoa dust tap.
* **Feel:** the most meditative game. The ticks sit under the rain, and the bloom puffs up visibly.

### 2.4 TeaSteep (MUST)

Recipes: lemon tea, chai, linden tea, Rn 333 tea, Tatra tea, uzvar.
* **Look:** side-view glass; colour spreads from the bag in soft plumes; a target colour chip on the rim
  **plus a 1–5 strength scale and a tiny sand-timer**, so it is colour-blind safe.
* **Controls:** click / Space to lift the bag when it looks right. Optional dunking (drag up/down) speeds it up.
  Then add-ons: a lemon slice swings over the cup (click to drop it); sugar cubes (click per cube).
* **Duration:** 8–14 s.
* **Quality:** `q = clamp(1 − |strength − target| / 1.5) × addons`, where `addons` = mean of add-on
  accuracies (lemon drop ±25 px; sugar count exact = 1, ±1 = 0.8).
* **Variants:** Vera's "properly made" uses tolerance /1.5 and a bonus reaction line if ≥ 0.9. Rn 333
  tea needs exactly two sugars. Chai has a "pour the warm milk" hold at the end.

### 2.5 Plate (MUST)

Recipes: topinka, chlebíček, koláč, medovník, varenyky, pirohy, buchty, Vera's koláč; soups until SoupStir ships.
* **Look:** top-down plate on a wooden board; 3–5 components beside it.
* **Controls:** at the start "Lida's way" ghost layout shows for 1.5 s (Relaxed: stays). Drag components
  onto the plate; click garnish dots (dill, sour cream, onion, sugar dust). Keyboard: Tab cycles items,
  arrows move, Space drops (snaps to the nearest slot, scored 0.85).
* **Duration:** 8–15 s.
* **Quality:** `0.7 × mean(slot_score)` (greedy nearest free slot, `1 − dist / 40 px`)
  `+ 0.3 × garnish` (placed / needed, extras free).
* **Variety:** per-dish layouts. Mirek's topinka: "rub the garlic twice" (a drag back-and-forth gesture on
  the toast). Advent nights 21–30: a spruce twig garnish slot. Varenyky: crispy onion "sprinkle" (hold over the plate).

### 2.6 SoupStir — new (SHOULD)

Recipes: garlic soup, grandma's kulajda, borscht.
* **Look:** top-down pot, a wooden ladle, a "simmer" ring around the pot rim glowing green-amber.
* **Controls:** circle with the mouse (held) to stir; keep the simmer meter in the gentle zone (too fast
  = small splashes, too slow = "it's catching"). When a bubble ring glows (2–3 times), click the
  ingredient chip (garlic, croutons; mushrooms, dill; beetroot, smetana). Keyboard: Left/Right set stir speed, Space adds.
* **Duration:** 12–18 s.
* **Quality:** `0.6 × time_in_zone + 0.4 × mean(add timing, ±0.5 s)`.
* **Variety:** kulajda ends with a cream swirl tap (the soup turns pale gold); borscht turns deep ruby if
  the beetroot went in on time; first-snow nights add steam fog.

### 2.7 MulledSpice — new (SHOULD)

Recipe: svařák (and LATER a "white svařák" Advent variant).
* **Look:** side-view pot of red wine; a spice tray: cinnamon, clove, star anise, orange slice, honey, ginger.
* **Controls:** the recipe card lists 3–4 spices; drag them in (any order). Then hold to warm, keeping the
  gauge under the "never boil" line and in the band for 3 s in total. Keyboard: Tab/Space to pick and drop, then hold Space.
* **Duration:** 12–16 s.
* **Quality:** `0.5 × spices` (needed found / needed, −0.15 per wrong one, floor 0) `+ 0.5 × band_time / 3 s`.
* **Feel:** the room's lamp light turns warmer for a second when it is done; orange and clove aroma
  particles drift up.

### 2.8 Micro-actions, not mini-games (MUST)

Wash (hold), Mop (swipe), Clear (click), Tip (click), Greet (click), and Wipe window (one swipe, moment §5).
They take 0.5–2 s, happen in the world with no overlay, and are never scored.

---

## 3. Wishes

### 3.1 Taste tags (MUST)

`Taste { Warm, Cold, Sweet, Bitter, Strong, Mild, Comforting, Fancy, Quick, Filling, Spiced, Fresh }`.
The brief's 10 tags, plus **Spiced** (chai, svařák, Tatra tea, night train) and **Fresh** (lemon, mint,
tonic, cold drinks), so that teas and cold drinks can be told apart. Each tag has a 12 px icon and the
key `taste.<tag>` (`taste.warm` …).

### 3.2 Match score (MUST)

A wish has `want` (1–3 tags), `avoid` (0–2 tags), optional `favourite` recipes and an `any` flag.

```
base   = |want ∩ recipe.tags| / |want|                  (any: true → 0.6 for every recipe)
avoid  = 0.5 if (avoid ∩ recipe.tags) ≠ ∅ else 0
fav    = 1.0 if recipe ∈ favourite (exact) · +0.15 if recipe's base_item ∈ favourite items
season = +0.1 if the recipe is on tonight's specials board (§6.8, SHOULD)
match  = clamp(base − avoid + fav_bonus + season, 0, 1)      (exact favourite → 1.0)
```

Tiers: **perfect** ≥ 0.99, **good** ≥ 0.6, **okay** ≥ 0.3, **not_quite** < 0.3. Even `not_quite` adds
satisfaction (never negative on Cozy; Classic −2).

**Fair wishes:** when picking a wish, the sim computes the best match among Karolina's known,
currently available recipes. Wishes with best ≥ 0.6 get weight ×3, and wishes with best < 0.3 are
skipped. The guest never asks for something the café can't make.

### 3.3 Who wishes what

`who` lists guest archetype ids and/or regular ids; empty = anyone. `when` = standard `Condition`s.
Per night a wish id is used at most twice (personal wishes once).

### 3.4 The wish list (MUST: all 43)

English texts. Translators write cs/sk/uk. Keys: `wish.<id>` (generic), `wish.<regular>.<id>` (personal).

| key | text (en) | want | avoid | who / when |
|---|---|---|---|---|
| `wish.long_day` | Something warm, not too sweet — long day. | Warm | Sweet | anyone |
| `wish.wake_me` | Something strong. I've still got four hours of driving. | Strong, Bitter | — | taxi_drivers, truckers |
| `wish.hands_cold` | Anything warm to hold. My hands are frozen. | Warm, Comforting | Cold | anyone · Raining or Snow |
| `wish.train_soon` | Whatever's fastest — my train's in ten minutes. | Quick | — | travelers · TrainNear(15) |
| `wish.earned_it` | Something sweet. I've earned it. | Sweet | — | anyone |
| `wish.surprise_me` | Surprise me. Something I'd never order. | Fancy | — | clubbers, night_wanderer |
| `wish.skipped_dinner` | Something filling. I skipped dinner. And lunch. | Filling, Warm | — | shift_workers, train_crew, truckers |
| `wish.sleepy_not_awake` | Something gentle. I'm trying to get sleepy, not awake. | Mild, Comforting | Strong | insomniacs |
| `wish.club_oven` | Something cold — the club was an oven. | Cold, Fresh | Warm | clubbers |
| `wish.taste_of_home` | Something that tastes like home. Anyone's home. | Comforting, Filling | — | travelers |
| `wish.exam_at_eight` | Fuel. I've got an exam at eight. | Strong, Quick | — | students |
| `wish.soaked` | Something with spice — I'm soaked to the bone. | Warm, Spiced | — | anyone · Raining |
| `wish.not_coffee` | Anything but coffee. I've had six. | Mild | Bitter, Strong | students, nurses |
| `wish.for_my_friend` | Something pretty — it's my friend's birthday. | Fancy, Sweet | — | parties ≥ 2 |
| `wish.goes_with_snow` | Something that goes with snow. | Warm, Sweet, Spiced | — | Snow |
| `wish.on_the_side` | Just a little something on the side. | Quick | Filling | anyone (food) |
| `wish.bitter_mood` | Bitter. Like my mood. No sugar. | Bitter | Sweet | anyone |
| `wish.headache` | Something fresh, please. I've got a headache. | Fresh, Mild | Strong | anyone |
| `wish.slow_sip` | Something to sip slowly while the rain does its thing. | Warm, Mild | Quick | insomniacs · Raining |
| `wish.engaged` | We just got engaged! Something special! | Fancy, Sweet | — | parties ≥ 2 · rare (weight 0.3) |
| `wish.one_hand` | Something I can drink with one hand — I'm flying a drone with the other. | Quick | Filling | drone_operators |
| `wish.patrol` | Strong, to go. Quiet night so far — let's keep it that way. | Strong, Quick | — | patrol |
| `wish.like_grandma_made` | Something like my grandmother made? She was from Lviv. | Comforting, Filling | — | travelers, train_crew |
| `wish.childhood` | Something from when I was a kid. | Sweet, Comforting | Bitter | anyone |
| `wish.three_am` | It's three in the morning. I don't know what I want. | any | — | anyone · 02:30–04:00 |
| `wish.kind_coffee` | Wake me up gently — coffee, but kind. | Mild, Comforting | — | nurses, shift_workers |
| `wish.fog_outside` | Something warm. You can't see the station through this fog. | Warm | Cold | anyone · Fog |
| `mirek.like_lida` → `wish.mirek.like_lida` | Black, Karolina. The way Lida poured it. | Strong, Bitter, Warm | Sweet | mirek · fav `r_black`, `r_turek` |
| `wish.mirek.tired_eyes` | Something for tired eyes, eh? Not that mine are tired. | Warm, Strong | — | mirek · ArcAtLeast(mirek, 2) |
| `wish.mirek.walked` | I walked here tonight. Something to warm an old man. | Warm, Comforting, Filling | — | mirek · Any(Flag mirek_keys_taken, Flag mirek_daughter_called) |
| `wish.hana.twelve_minutes` | I've got twelve minutes. Something hot and fast. | Warm, Quick | — | hana · 02:00–02:30 |
| `wish.hana.blanket` | Shift's over, Karolina. Something that feels like a blanket. | Comforting, Filling, Warm | — | hana · 05:40–06:00 |
| `wish.vera.properly` | Tea. Properly made, mind you — no bag-dunking. | Mild, Warm | Sweet | vera · fav `r_lemon_tea`, `r_linden_tea` · narrow TeaSteep |
| `wish.vera.lidas_kolac` | Your grandmother's koláč was better, Karolina. Show me yours. | Sweet, Comforting | — | vera · fav `r_vera_kolac` |
| `wish.vera.she_would_make` | Make me something Lida would have made. | Comforting, Filling | — | vera · ArcAtLeast(vera, 2) · fav `r_kulajda` |
| `wish.filip.cheapest` | The cheapest coffee that still counts as coffee? | Strong, Quick | Fancy | filip |
| `wish.filip.passed` | I passed! Something fancy — my treat. | Fancy, Sweet | — | filip · ArcAtLeast(filip, 3) |
| `wish.radek.to_go` | To go. The 01:12 won't dispatch itself. | Quick, Strong | — | radek |
| `wish.radek.last_shift` | Last shift, Karolina. Make it a good one. | Fancy, Warm | — | radek · night 23 |
| `wish.radek.no_rush` | No rush tonight. Something slow. | Mild, Warm | Quick | radek · ArcAtLeast(radek, 3) |
| `wish.lori.one_song` | One hot chocolate, paid in one song? | Sweet, Warm | — | lori · fav `r_hot_chocolate`, `r_lori_encore` |
| `wish.lori.my_voice` | Something for my voice. Honey, maybe? | Warm, Mild, Sweet | Cold | lori · fav `r_linden_tea` |
| `wish.guest_333.two_sugars` | Tea. With lemon. Two sugars. | Warm, Sweet, Mild | — | guest_333 · fav `r_rn333_tea` (lemon tea = 0.8) |
| `wish.guest_anwar.oat` | An oat flat white, please. And maybe the Wi-Fi password. | Warm, Mild | — | guest_anwar · fav `r_oat_flat_white` |

(Row 28's key is `wish.mirek.like_lida`. 27 generic + 16 personal = 43.)

### 3.5 Reactions (MUST)

Keys `react.<tier>.<n>`: 5 generic lines per tier (20 lines), for example:
* perfect: "Oh. That's exactly it." · "How did you know?" · "I'm coming back tomorrow just for this."
* good: "Mm. Lovely." · "That hits the spot." · "Thank you, really."
* okay: "Nice, thanks." · "Not what I pictured, but nice." · "Warm. Good."
* not_quite: "Not quite it — but thank you for trying." · "Hm. Different. I like that you tried."

Regulars: `react.<regular>.<tier>` (7 regulars × 4 = 28 lines), for example `react.mirek.perfect` "…Almost
like hers, Karolina. Almost." and `react.vera.perfect` "Well. Lida would have grumbled, but she'd have drunk it."
`react.guest_333.*` lines are wordless: `"…"` with a nod. The emote carries them.

---

## 4. Recipes and the recipe book

### 4.1 Pantry (MUST: 22 ingredients)

`IngredientDef { id, family, shelf: Bar|Kitchen, unlock }`. Families drive hints: Coffee, Milk, Tea,
Sweet, Spice, Fruit, Cold, Savoury, Dough. Ingredients appear on the shelf when unlocked (the default is
unlocked; the others are listed below).

| Bar shelf | family | unlock | Kitchen shelf | family | unlock |
|---|---|---|---|---|---|
| espresso | Coffee | — | bread | Dough | — |
| milk | Milk | — | garlic | Savoury | — |
| oat_milk | Milk | night 3 | potato | Savoury | — |
| cream | Milk | — | mushroom | Savoury | flag recipe_kulajda |
| black_tea | Tea | — | beetroot | Savoury | moment `m_parcel_lviv` or hire sound_tech |
| linden | Tea | grandma card night 2 | tvaroh | Dough | night 2 |
| mountain_herbs | Tea | hire server (Natália) or sandbox night 6 | bryndza | Savoury | night 8 |
| lemon | Fruit | — | poppy_seed | Sweet | Vera arc ≥ 1 |
| honey | Sweet | — | plum_jam | Fruit | night 2 |
| sugar | Sweet | — | | | |
| spice (cinnamon & clove) | Spice | night 4 | | | |
| chili | Spice | Lori arc ≥ 1 | | | |
| cocoa | Sweet | night 6 | | | |
| vanilla | Sweet | night 8 | | | |
| dried_fruit | Fruit | moment `m_parcel_lviv` | | | |
| ice | Cold | — | | | |
| tonic | Cold | Filip arc ≥ 1 | | | |
| butterfly_pea | Fruit | moment `m_drone_wrong_address` (thank-you parcel) | | | |
| red_wine | Fruit | night 12 | | | |

(19 bar + 9 kitchen = 28 shelf slots; red_wine, chili, tonic, butterfly_pea, dried_fruit and the 3
gated kitchen items unlock through play. 22 of them are needed by the MUST recipes.)

### 4.2 Recipes (MUST: 16 base + 6 specials; SHOULD: 6 more)

Every recipe rides on a `base_item` from `menu.ron`, which supplies station, job, prep, price, cost,
power and availability. Specials add `price_add` / `cost_add`. "Base" recipes are the menu item done the
Karolina's way.

| id | name (en) | base_item | tags | mini-game | ingredients | discovery | tag |
|---|---|---|---|---|---|---|---|
| `r_black` | Black, like Lida made it | black_coffee | Warm, Bitter, Strong, Quick | PourOver | espresso | starter | MUST |
| `r_cappuccino` | Cappuccino | cappuccino | Warm, Mild, Comforting | MilkFroth | espresso, milk | starter | MUST |
| `r_lemon_tea` | Tea with lemon | lemon_tea | Warm, Mild, Fresh, Quick | TeaSteep | black_tea, lemon | starter | MUST |
| `r_topinka` | Topinka | topinka | Warm, Filling, Quick | Plate | bread, garlic | starter | MUST |
| `r_garlic_soup` | Česnečka | garlic_soup | Warm, Filling, Comforting | Plate→SoupStir | garlic, potato, bread | starter | MUST |
| `r_chlebicek` | Chlebíček | chlebicek | Cold, Quick, Fancy | Plate | bread, potato | starter | MUST |
| `r_kolac` | Koláč | kolac | Sweet, Quick, Comforting | Plate | tvaroh, plum_jam | menu unlock (n2) | MUST |
| `r_oat_flat_white` | Oat flat white | oat_flat_white | Warm, Mild, Fancy | LatteArt | espresso, oat_milk | menu unlock (n3) | MUST |
| `r_medovnik` | Medovník | medovnik | Sweet, Fancy, Cold | Plate | honey, cream | menu unlock (n3) | MUST |
| `r_chai` | Chai | chai | Warm, Sweet, Spiced, Comforting | TeaSteep | black_tea, milk, spice | menu unlock (n4) | MUST |
| `r_hot_chocolate` | Hot chocolate | hot_chocolate | Warm, Sweet, Comforting | MilkFroth | cocoa, milk | menu unlock (n6) | MUST |
| `r_varenyky` | Varenyky with potato & mushroom | varenyky | Warm, Filling, Comforting | Plate | potato, mushroom | menu unlock (n8) | MUST |
| `r_pirohy` | Bryndzové pirohy | bryndzove_pirohy | Warm, Filling | Plate | potato, bryndza | menu unlock (n8) | MUST |
| `r_night_train` | Night Train | night_train | Strong, Bitter, Spiced, Fancy | PourOver | espresso, cocoa, spice | menu unlock (n9) | MUST |
| `r_svarak` | Svařák | svarak | Warm, Sweet, Spiced, Fancy | MilkFroth→MulledSpice | red_wine, spice, sugar | menu unlock (n12) | MUST |
| `r_kulajda` | Grandma's kulajda | grandma_kulajda | Warm, Filling, Comforting, Fancy | Plate→SoupStir | potato, mushroom, cream | Vera arc 2 (`recipe_kulajda`), not experimentable | MUST |
| `r_turek` | Turek, taxi-rank style | black_coffee (+5) | Strong, Bitter, Warm | PourOver (turek) | espresso, sugar | Mirek teaches at arc ≥ 1; experiment | MUST |
| `r_vera_kolac` | Vera's poppy-seed koláč, warmed | kolac (+10) | Warm, Sweet, Comforting | Plate | poppy_seed, tvaroh | Vera teaches at trust ≥ 40; experiment | MUST |
| `r_rn333_tea` | Tea, two sugars | lemon_tea (+0) | Warm, Sweet, Mild | TeaSteep (2 sugars) | black_tea, lemon, sugar | `strange_order` choice `tea_two_sugars`, or 3:33 arc ≥ 1; experiment | MUST |
| `r_lori_encore` | Lori's encore (chili chocolate) | hot_chocolate (+10) | Warm, Sweet, Spiced | MilkFroth (pinch) | cocoa, milk, chili | Lori teaches when invited; experiment once chili unlocks | MUST |
| `r_linden_tea` | Linden tea with honey | lemon_tea (+10) | Warm, Mild, Comforting, Sweet | TeaSteep | linden, honey | grandma's card hint (night 2); experiment | MUST |
| `r_moon_latte` | Moonlight latte | oat_flat_white (+20) | Warm, Fancy, Mild, Sweet | LatteArt (moon) | oat_milk, butterfly_pea, honey | experiment only (butterfly pea from the drone parcel) | MUST |
| `r_raf` | Raf coffee (Kyiv-style) | cappuccino (+15) | Sweet, Mild, Fancy, Comforting | MilkFroth (vanilla drop) | espresso, cream, vanilla | Oleh (sound_tech) teaches after 2 shifts; experiment | SHOULD |
| `r_borscht` | Borscht with smetana | garlic_soup (+20) | Warm, Filling, Comforting, Fresh | SoupStir | beetroot, potato, cream | Oleh or the Lviv parcel; experiment | SHOULD |
| `r_buchty` | Kuba's buchty with povidla | kolac (+10) | Sweet, Comforting, Filling | Plate | plum_jam, milk | Kuba (porter) after 5 shifts ("he's been baking secretly"); experiment | SHOULD |
| `r_tatra_tea` | Tatra herb tea | chai (+5) | Warm, Spiced, Comforting, Fresh | TeaSteep | mountain_herbs, honey, lemon | Natália (server) teaches; experiment | SHOULD |
| `r_espresso_tonic` | Platform espresso tonic | black_coffee (+25) | Cold, Fresh, Bitter, Fancy | PourOver (tonic) | espresso, tonic, ice | Filip ("exam fuel") at arc ≥ 1; experiment | SHOULD |
| `r_uzvar` | Uzvar (warm dried-fruit drink) | lemon_tea (+10) | Warm, Sweet, Fresh, Comforting | TeaSteep | dried_fruit, honey | Lviv parcel; experiment | SHOULD |

Coverage check: every Taste appears in ≥ 4 MUST recipes. Cold (chlebíček, medovník, and later the espresso
tonic) is the thinnest; `wish.club_oven` is therefore weighted 0.5 until a Cold drink is known.

### 4.3 Discovery paths

1. **Starters** (6), known on night 1.
2. **Grandma's recipe cards** (menu unlocks): when a base item unlocks, its recipe arrives as a card in
   Lida's handwriting (`recipe.<id>.lore`), with a toast "Found in the drawer: grandma's card".
   These are **not** the 10 story notes.
3. **Regulars and helpers teach**: an effect `LearnRecipe(id)` placed on arc stages / trust thresholds
   (content). A short bubble: "Let me show you how we did it in '79." + a card.
4. **Experiments** (§4.4).
5. **Story-gated** recipes (`experimentable: false`): kulajda only via Vera. An exact match before the
   story gives "Vera would know the rest of this one."

### 4.4 Experiments (MUST)

* Where: the recipe book's **Pantry** tab, any time (Day: free; Night: Karolina spends 2 game min at the counter).
* Choose 2–3 unlocked ingredients → **Try** (costs 10 Kč of supplies; free on Cozy nights 1–7).
* Logic (`S` = chosen set; recipe sets `R`; order never matters):
  1. `S == R` for a known recipe → "You know this one: *Chai*." (the card wiggles).
  2. `S == R` for an unknown experimentable recipe → **Discovered!** Card flip, a chime, `RecipeDiscovered`.
  3. `S == R` for a story-gated one → its `gated_hint` key.
  4. Otherwise find the undiscovered experimentable recipe with the most overlap (ties: fewest ingredients).
     - `|S∩R| = |R| − 1` and `|S| = |R|` → **"So close — swap the {family of the wrong one}."**
     - `|S∩R| = |R|` and `|S| > |R|` → **"Almost — one thing too many."**
     - `|S∩R| = |R| − 1` and `|S| < |R|` → **"Almost — it wants something {family of the missing one}."**
     - else → a "nothing" line: "It's… drinkable. Kuba drank it anyway." (pick from 6 lines;
       `experiment.nothing.<n>`), and the cat sniffs it if present.
* **Grandma's margin notes:** every 3 failed tries, the next undiscovered recipe (lowest unlock night)
  reveals one ingredient on its silhouette ("Lida's margin note: *…honey, always honey.*").
* Silhouette cards always show: tag icons, ingredient-count dots, mini-game icon, and a source hint
  (`book.hint.<source>`: "Someone at the taxi rank might know", "Try the pantry", "Ask Vera").
  Everything is discoverable without a wiki.

### 4.5 Recipe book screen (K) (MUST)

* A two-page glass "book", 760×480 pt, opening from the right.
* **Left page:** tabs *Drinks · Food · Specials · Pantry* (+ SHOULD: *Napkins* §6.1, *Trains* §6.7),
  then a 4×4 grid of cards (96×72 pt): known = illustration tile + name, practised = small star,
  unknown = indigo silhouette + tag icons + count dots.
* **Right page:** the selected recipe: name, Lida's handwritten lore line (italic, amber), ingredients,
  taste icons, mini-game icon, base item & price, "served 12× · best: Beautiful", unlocked latte patterns.
  For an unknown recipe: the source hint + any revealed ingredients.
* **Pantry tab:** two shelves of jars (unlocked jars lit, locked jars dim with "?"), 3 slots, **Try**,
  a result card. Keyboard: arrows move the selection, Enter adds, Backspace removes, T tries.
* Opened from the order card, the book is in *pick mode*: clicking a known recipe picks it for the guest.

---

## 5. Night moments

### 5.1 Rules (MUST)

* `MomentDef` → `SimEvent::MomentAvailable { id, anchor }` → a glowing marker in the world (indigo dot +
  icon, pulse 1.2 s; hover = title) → click → a small glass card next to the anchor with 2–3 options →
  `Command::ResolveMoment { id, option }`. Karolina walks to the anchor (the option applies on arrival;
  the card closes at once). The card is non-blocking.
* **Director:** target **2–4 a night** (`per_night: (2, 4)`; act 1: (1, 3)); min gap 40 game min, except
  the lull director; each moment has `weight`, `cooldown_nights`, `max_per_game` and
  `expires_minutes` (default 25). On expiry `on_expire` effects apply (usually none, or a mild default:
  "Kuba put a bucket under it").
* Some moments **replace** event choice toasts (the event fires its base effects and spawns the moment
  instead of `choices`): `roof_leak`, `power_outage`, `station_cat`, `drone_lost`, `forgotten_item`.
  Their default choices become `on_expire`.

### 5.2 The 20 moments

| id | when (conditions) | anchor | options → effects | freq | tag |
|---|---|---|---|---|---|
| `m_record_request` | a present regular whose liked record (§10) is unlocked and not playing; ≤ 1 a night | the regular | **Put it on** → Karolina walks to the jukebox, `PlayRecord`; the like bonus is +3 trust instead of +2 · **Maybe later** → nothing | 1/night | MUST |
| `m_blackout_candles` | `Blackout { on }` (power_outage, storm_blackout) | counter | **Light the candles** (−0, she lights them table by table: a quick click per table, or one click lights all) → GuestSatisfaction +6, beauty +1 while dark · **The old torch** → nothing | per outage | MUST |
| `m_leak_bucket` | roof_leak fired | leak tile | **Bucket** → tile blocked, a "plink" rhythm; after 60 min `m_leak_empty` · **Call a roofer** (−900) → roof_fixed | event | MUST |
| `m_station_cat` | station_cat fired | door | **A saucer of milk** (−10) → affection +1 · **A bit of ham** (−15) → affection +2 · **Scratch its ears** → affection +1 if ≥ 2, else the cat leaves | 1/night | MUST |
| `m_lost_traveller` | a travelers party at the door, 30 % on train nights | door | **Check the Shiftella app** → a mini card with 3 trains (from trains.ron); the right one is highlighted; click it → Reputation +2, tip 40–80 · **Draw a map on a napkin** → satisfaction +8 | 1/night, cd 1 | MUST |
| `m_stuffy_window` | GuestsAtLeast(8), not Storm, not Snow | a window | **Open the window** → satisfaction +4 all, Cold/Fresh wishes ×2 for 30 min; closes itself · **Leave it** | cd 1 | MUST |
| `m_fogged_window` | Raining or Fog, TrainNear(3) | a window | **Wipe the glass** (one swipe) → the camera eases to the window, the train's light sweeps the room; guests within 3 tiles +3; logbook entry (§6.7) | ≤ 2/night | MUST |
| `m_sleeping_guest` | an insomniacs/students/nurses guest seated > 60 min after eating, 01:00–05:00 (not Hana on night 12: a cutscene) | the guest | **Cover with a blanket** → they stay, satisfaction +10, 30 % thank-you note, regulars Trust +3 · **Let them sleep** · **A glass of water, gently** → satisfaction +4, they leave refreshed | cd 1 | MUST |
| `m_drone_wrong_address` | replaces drone_lost; 23:00–04:30 | a window | **Re-route it in the Shiftella app** → Reputation +2; next night 50 %: "Thanks!" toast + 50 Kč · **Keep the parcel for pickup** → next night the owner collects it and leaves `butterfly_pea` (first time) | cd 4 | MUST |
| `m_coffee_for_helper` | a helper with energy < 30 | that helper | **Make them a coffee** → a by-hand mini-game (their favourite) → StaffMood +10, energy +15, Relationship +3 with Karolina · **Send them on a break** → break now | ≤ 2/night | MUST |
| `m_leak_empty` | 60 min after the bucket | bucket | **Empty it** → tile unblocked (floor helper does it if present) | follow-up | SHOULD |
| `m_busker` | Raining, 22:30–01:00, no Lori tonight (story: before night 11 as foreshadowing; sandbox any) | street | **Bring a tea** (−20) → Reputation +1, music outside 20 min · **A coin in the case** (−10) · **Ask them in** (needs `stage`) → HallMood(Lively) for 30 min | cd 3 | SHOULD |
| `m_first_snow` | weather Snow first time | window | **Call everyone to look** → GuestSatisfaction +5, StaffMood +5, a snowflake latte unlock · **A candle in the window** → beauty +1 tonight | once | SHOULD |
| `m_umbrellas` | Downpour | door | **Put out the umbrella stand** → DirtRate ×0.6 tonight · **Newspapers on the floor** → DirtRate ×0.8, Vera trust +1 | cd 2 | SHOULD |
| `m_forgotten_item` | replaces forgotten_item | the table | **Run after them** (only within 2 min) → Reputation +3, tip 100 · **Lost & found box** → existing result | cd 2 | SHOULD |
| `m_dead_phone` | a students/travelers guest, or Filip | the guest | **Lend the counter charger** → satisfaction +6 (Filip Trust +3) · **Point to the socket table** | cd 2 | SHOULD |
| `m_tired_parent` | travelers party with a child (look accessory `child`), after a train | the table | **Warm milk, on the house** (−15) → Reputation +2, thank-you note 50 % · **A blanket for the little one** | cd 4 | SHOULD |
| `m_delay_board` | a train delay (late_train) or radek_warns_trains | the holo board | **Free refills for the waiting** (−60) → Reputation +3, GuestStay +20 · **Put the station board on the café screen** → patience +30 % tonight | cd 2 | SHOULD |
| `m_radio_tuning` | radio placed, 02:00–04:00 | radio | **Night jazz** → plays `jukebox_piano_trio` through the radio (volume 0.6) · **An old waltz** → `jukebox_cafe_waltz` (if unlocked) · **Static…** (night ≥ 15, 03:00–03:33) → `amb_rain_window_loop` at radio volume and a faint 1979 announcement *as text* (`moment.m_radio_tuning.result.static_1979`); light_sleeper staff mood +5 | cd 1 | SHOULD |
| `m_bulb_flicker` | 03:15–03:30, night ≥ 4, window light on | the window lamp | **Grandma's spare bulb** (in the drawer) → nothing changes, a warm line "she kept spares for sixty years". Never fails: on expiry the light holds anyway | 1 in 4 nights | SHOULD |
| `m_parcel_lviv` | sound_tech hired, night ≥ 10 | counter | **Open it with Oleh** → `dried_fruit` + `beetroot`, Oleh mood +10, learns `r_uzvar` hint · **Save it for the morning** → the same at night end | once | SHOULD |

(10 MUST + 11 SHOULD; `m_leak_empty` is a follow-up, not counted in the night budget.)

Every moment is a **player** action: helpers never resolve moments, though the floor helper may empty a bucket.

---

## 6. Extra features ("фишки"), ranked by charm ÷ cost

| rank | feature | charm (1–5) | cost (1–5) | tag |
|---|---|---|---|---|
| 1 | Thank-you napkins | 5 | 1 | SHOULD |
| 2 | Station cat friendship | 5 | 2 | MUST (levels), SHOULD (visuals) |
| 3 | Regulars' rituals & "The usual?" | 5 | 2 | MUST ("the usual"), SHOULD (rituals) |
| 4 | The quiet hour 03:00–04:00 | 4 | 1 | SHOULD |
| 5 | Tip jar goals → decor | 4 | 1 | SHOULD |
| 6 | Weather changes who comes in | 3 | 1 | SHOULD |
| 7 | Trainspotting logbook | 3 | 1 | LATER |
| 8 | Specials board / seasonal menu | 4 | 2 | SHOULD |
| 9 | Night playlist reactions (now part of the jukebox, §10) | 4 | 3 | MUST via §10 |
| 10 | Karolina's own mug | 3 | 2 | SHOULD |

### 6.1 Thank-you napkins
After an Karolina-made serve with match ≥ 0.8 and quality ≥ 0.7, there is a 12 % chance (regulars 25 %) that
a folded napkin lies next to the tip coin. Clicking the coin collects both. The napkin card shows a
handwritten line (`thanks.<id>`, 40 lines, some doodles: a cat, a train, a heart). It is collected in the
book's *Napkins* tab. The 10th, 20th and 40th napkin unlock decor: "napkin wall" (beauty +2), etc.
Regular napkins are personal (`thanks.mirek.1` "For the coffee. And for not asking. — M.").

### 6.2 Station cat friendship
Counter `cat_affection` 0–10 (existing `cat_fed_nights` stays for `cat_stays`).
Levels (`cat.level.<n>`): 0 stranger → 2 visitor (sits on the doormat) → 4 friend (comes inside, sleeps
under the radiator) → 6 regular (its own spot on the window seat: nearby guests +2) → 8 "works here"
(follows Karolina; staff mood +2/night) → 10 family: a naming moment (choices `Vlak`, `Kofi`, `Lidka`).
A cat-face latte pattern unlocks at 3. MUST: the counter, the levels and the toast; SHOULD: sprite poses.

### 6.3 Regulars' rituals and "The usual?"
* MUST: at trust ≥ 50, a regular's order card shows **The usual?**, which picks their favourite recipe
  and By hand with one click.
* SHOULD: rituals, small tells that reward attention. If the player responds within 2 game min, +2 trust:
  Mirek taps the counter twice (greet him); Vera straightens the sugar bowl (wipe her table first);
  Radek checks his watch at the door (have his to-go cup ready: by-hand brew started before he reaches the counter);
  Hana rubs her eyes (a glass of water); Filip plugs in his charger (the socket table must be free);
  Lori shakes off her raincoat (the coat rack nearby); the 3:33 guest looks at the window (light on).
* SHOULD: **Reserve a seat** from the Day screen or by right-clicking a seat (a little card "Reserved —
  Mirek"). It holds until 30 min after their window; reserved → `SeatFreeStreak` is easier and gives +1 trust.

### 6.4 The quiet hour (03:00–04:00)
With the house playlist on, the music switches to `jukebox_piano_trio` at volume 0.7 (a record the player chose keeps playing); lamps dim 15 %; rain louder; GuestRate ×0.5; wish share 0.8 with the
personal pool; the moment pool prefers `quiet`-tagged moments (sleeping guest, radio, bulb, cat); lull
threshold 15 min. The 3:33 sits inside it. A toast at 03:00: "The quiet hour." No mechanics beyond mods and mood.

### 6.5 Tip jar goals
The jar on the counter shows tonight's tips and a lifetime total. Lifetime goals: 500 / 1 500 / 3 000 /
6 000 / 10 000 Kč unlock fairy lights, grandma's lace tablecloths, a record shelf, a cat bed and a brass
espresso bell (decor via `UnlockFurniture`). Goal reached → a "Count the jar" moment-like chip, then a choice of 2 decor.

### 6.6 Weather changes who comes in
`GuestArchetype.weather_mult: { Rain: 1.3, Fog: 0.7, … }` (default 1). Downpour: travellers and wanderers
up, clubbers down. Fog: insomniacs up and the uncanny pool up. Clear Fri/Sat: students and clubbers up.
Snow: everyone ×0.8, stay +20 min. Data-only plus one multiplication in `arrival_rate`.

### 6.7 Trainspotting logbook (LATER)
Each `m_fogged_window` wipe logs the passing train (`train.<id>.name`) and its station
(Adamov … Frýdlant). Rn 333 can only be logged on night 30 at 03:33. A full page earns a small train
figurine decor.

### 6.8 Specials board / seasonal menu
The Day screen has a chalkboard with 2 slots: pick known specials → those recipes get match +0.1 and +5 %
price tonight; guests' ambient lines mention them. Seasonal suggestions by act: nights 1–10 autumn (chai,
linden), 11–20 late autumn (svařák, kulajda, borscht), 21–30 winter/Advent (svařák, snowflake latte,
buchty; LATER: "vánočka toast", white svařák).

### 6.9 Night playlist reactions
Promoted to MUST and designed in §10.4 (archetype `music_likes` + regulars' `liked_by` / `disliked_by`).

### 6.10 Karolina's own mug
A mug on the counter. In a lull, or at energy < 25 on Classic, the chip "Make yourself a coffee" plays any
known recipe by hand for herself: energy +40, and a short internal line (`karolina.mug.<n>`, 12 lines:
"The rain sounds different from behind the counter."). It pairs with the quiet hour. Mug designs unlock with napkins.

---

## 7. Data, sim API, UI — the work split

This section covers §1–§8. Sections §10–§13 (jukebox, seasons/themes/coziness/journal, rozpis, the app)
carry their own data schemas and APIs next to the design; the sequential task plan for everything is §15.

### 7.1 New data files (content agent)

All files use `#![enable(implicit_some)]`. Validation: ids unique, refs resolve, i18n keys present in 4 languages.

**`assets/data/handson.ron`** (tuning, MUST)
```ron
#![enable(implicit_some)]
(
    real_seconds_per_tick_x1: 0.15,
    overlay_scale: (cozy: 0.1, classic: 0.35),
    player_walk_mul: 1.4,
    helper_speed: 0.85,
    helper_quality: (base: 0.35, per_skill: 0.04, max: 0.75),
    quick_quality: 0.6,
    practised_quick_quality: 0.8,
    practised_after: 10,
    wish_chance: [(from_night: 1, chance: 0.4), (from_night: 8, chance: 0.5)],
    sandbox_wish_chance: 0.45,
    claimed_wait_rate: 0.5,
    cozy_rush_wait_rate: 0.67,
    cozy_regular_patience_floor: 12.0,
    regular_announce_minutes: 2,
    lull_minutes: 6, quiet_hour_lull_minutes: 15,
    rhythm: (window_minutes: 4, step: 0.04, max: 0.20),
    moments_per_night: [(act: 1, range: (1, 3)), (act: 2, range: (2, 4)), (act: 3, range: (2, 4)), (act: 4, range: (2, 4))],
    moment_gap_minutes: 40,
    experiment_cost: 10, experiment_free_until_night: 7, margin_note_every: 3,
    thanks_chance: (guest: 0.12, regular: 0.25),
    capacity: (base: 1, after_nights: 3, skill_bonus_at: 7.0, max: 3),
)
```

**`assets/data/recipes.ron`** (MUST)
```ron
#![enable(implicit_some)]
(
    ingredients: [
        (id: "espresso", family: Coffee, shelf: Bar),
        (id: "oat_milk", family: Milk, shelf: Bar, unlock: NightAtLeast(3)),
        (id: "butterfly_pea", family: Fruit, shelf: Bar, unlock: Flag("ing_butterfly_pea")),
        (id: "mushroom", family: Savoury, shelf: Kitchen, unlock: Flag("recipe_kulajda")),
        // …
    ],
    recipes: [
        (id: "r_black", base_item: "black_coffee", tags: [Warm, Bitter, Strong, Quick],
         minigame: PourOver, variants: ["classic"], ingredients: ["espresso"],
         unlock: Starter),
        (id: "r_kolac", base_item: "kolac", tags: [Sweet, Quick, Comforting], minigame: Plate,
         ingredients: ["tvaroh", "plum_jam"], unlock: WithMenu),          // learned when kolac unlocks
        (id: "r_turek", base_item: "black_coffee", price_add: 5, tags: [Strong, Bitter, Warm],
         minigame: PourOver, variants: ["turek"], ingredients: ["espresso", "sugar"],
         unlock: Taught(by: "mirek"), hint: "book.hint.taxi_rank"),     // via LearnRecipe in regulars.ron
        (id: "r_kulajda", base_item: "grandma_kulajda", tags: [Warm, Filling, Comforting, Fancy],
         minigame: SoupStir, fallback_minigame: Plate, ingredients: ["potato", "mushroom", "cream"],
         unlock: Story, experimentable: false, gated_hint: "book.gated.kulajda"),
        (id: "r_moon_latte", base_item: "oat_flat_white", price_add: 20, cost_add: 6,
         tags: [Warm, Fancy, Mild, Sweet], minigame: LatteArt, variants: ["moon"],
         ingredients: ["oat_milk", "butterfly_pea", "honey"], unlock: Experiment,
         hint: "book.hint.pantry"),
        // …
    ],
)
```
`RecipeUnlock { Starter, WithMenu, Taught { by }, Story, Experiment }`. `Taught`/`Story` recipes are
still experimentable unless `experimentable: false`. Key helpers: `recipe.<id>.name/.desc/.lore`, `ingredient.<id>.name`.

**`assets/data/wishes.ron`** (MUST)
```ron
#![enable(implicit_some)]
[
    (id: "long_day", want: [Warm], avoid: [Sweet]),
    (id: "wake_me", want: [Strong, Bitter], who: ["taxi_drivers", "truckers"]),
    (id: "hands_cold", want: [Warm, Comforting], avoid: [Cold], when: [Any([Raining, Weather(Snow)])]),
    (id: "three_am", any: true, when: [TimeBetween("02:30", "04:00")]),
    (id: "engaged", want: [Fancy, Sweet], min_party: 2, weight: 0.3),
    (id: "mirek.like_lida", want: [Strong, Bitter, Warm], avoid: [Sweet], who: ["mirek"],
     favourite: ["r_black", "r_turek"]),
    (id: "guest_333.two_sugars", want: [Warm, Sweet, Mild], who: ["guest_333"],
     favourite: ["r_rn333_tea"], favourite_items: ["lemon_tea"]),
    (id: "vera.properly", want: [Mild, Warm], avoid: [Sweet], who: ["vera"],
     favourite: ["r_lemon_tea", "r_linden_tea"], strict: true),   // strict → narrower mini-game band
    // …
]
```
Key = `wish.<id>`, so personal ids carry the regular prefix (`mirek.like_lida` → `wish.mirek.like_lida`).
Personal wishes (`who` = a regular) are used instead of `RegularDef.orders`. `orders` remains the plain
"the usual" fallback and the helper/hand-off fallback.

**`assets/data/moments.ron`** (MUST)
```ron
#![enable(implicit_some)]
[
    (id: "m_record_request", kind: Music, anchor: Agent(RegularWantsRecord), per_night_max: 1,
     expires_minutes: 20,
     options: [
        (id: "put_on", effects: [PlayRequestedRecord, TargetTrust(1.0)]),   // +1 on top of the like bonus
        (id: "later"),
     ]),
    (id: "m_station_cat", kind: Cat, anchor: Door, from_event: "station_cat",
     options: [
        (id: "milk", cost: -10, effects: [Money(-10), AddCounter("cat_affection", 1), AddCounter("cat_fed_nights", 1)]),
        (id: "ham", cost: -15, effects: [Money(-15), AddCounter("cat_affection", 2), AddCounter("cat_fed_nights", 1)]),
        (id: "scratch", effects: [If(conditions: [CounterAtLeast("cat_affection", 2)],
                                     then: [AddCounter("cat_affection", 1)], otherwise: [])]),
     ],
     on_expire: [AddCounter("cat_fed_nights", 1)]),   // a helper fed it
    (id: "m_fogged_window", kind: Window, anchor: Tag("window"), weight: 14.0, per_night_max: 2,
     conditions: [Any([Raining, Weather(Fog)]), TrainNear(3)], expires_minutes: 4,
     options: [(id: "wipe", gesture: Swipe, effects: [CameraHint(Window), GuestSatisfactionNear(add: 3.0, tiles: 3), LogTrain])]),
    (id: "m_coffee_for_helper", kind: Staff, anchor: Agent(StaffLowEnergy(30.0)), per_night_max: 2,
     options: [
        (id: "coffee", minigame: Favourite, effects: [TargetStaffMood(10.0), TargetStaffEnergy(15.0), Relationship(add: 3.0, minutes: None)]),
        (id: "break", effects: [TargetStaffBreak]),
     ]),
    // …
]
```
`MomentAnchor { Tile(x, y), Tag(furniture tag), Door, Street, Counter, Agent(selector), Leak }`.
`kind` drives the marker icon and the quiet-hour filter (`quiet: true` flag). `from_event` = replaces
that event's choices. Keys: `moment.<id>.title/.text`, `moment.<id>.opt.<opt>`, `moment.<id>.result.<opt>`.

**`assets/data/minigames.ron`** (MUST): tuning per kind & variant; latte patterns.
```ron
#![enable(implicit_some)]
(
    milk_froth: [(variant: "cappuccino", t_target: 62.0, tol: 4.0, foam: 0.6),
                 (variant: "chocolate", t_target: 58.0, tol: 6.0, foam: 0.3),
                 (variant: "raf", t_target: 60.0, tol: 4.0, foam: 0.5, tap: "vanilla")],
    pour_over: [(variant: "classic", bpm: 48, bloom_s: 2.0), (variant: "turek", stirs: 3, settle_s: 3.0),
                (variant: "tonic", slow_pour: true)],
    tea_steep: [(variant: "lemon", target: 3.0, tol: 1.5, lemon: true),
                (variant: "two_sugars", target: 3.0, tol: 1.5, lemon: true, sugar: 2)],
    latte_patterns: [
        (id: "heart", from_night: 1), (id: "tulip", from_night: 5), (id: "rosetta", from_night: 10),
        (id: "leaf", from_night: 14), (id: "swan", from_night: 20),
        (id: "snowflake", when: [Any([NightAtLeast(21), Weather(Snow)])]),
        (id: "moon", recipe: "r_moon_latte"),
        (id: "train", when: [Any([NightBetween(23, 23), NightBetween(30, 30)])]),
        (id: "cat", when: [CounterAtLeast("cat_affection", 3)]),
        (id: "note", when: [RegularPresent("lori")]),
    ],
)
```
Pattern guide polylines live in code (`src/minigame/latte_shapes.rs`, normalised 0..1).

**`assets/data/extras.ron`** (SHOULD): `thanks` (40 ids + `who`), `cat_levels`, `tip_goals`, `records`
(jukebox options' moods), `rituals` (per regular: tell + window + response).

**`assets/data/tutorial.ron`** (MUST) — §8.

**Edits to existing files** (content agent): `staff.ron` hire_from_night 2→3 for barista/cook/porter and
a `default_duty` per archetype; `story.ron` move `story.hint.hire`/`schedule` to night 3, replace
`story.hint.priorities` with `story.hint.duties`, add tutorial beats; `regulars.ron` streak thresholds
(Hana 6, Radek 5), the `Any([... WishMatchStreak])` alternatives and `LearnRecipe` effects;
`events.ron` remove `choices` from the 5 events replaced by moments (keep `default_choice` effects as
`on_expire`); `guests.ron` optional `weather_mult` (SHOULD).

### 7.2 Sim changes (sim agent)

**State (all `#[serde(default)]`):**
```rust
pub struct PlayerState {
    pub queue: VecDeque<PlayerAct>,        // max 5
    pub current: Option<PlayerAct>,
    pub brewing: Option<PlayerBrewJob>,    // guest, recipe, quality, mode, work_left
    pub idle_minutes: f32,
    pub rhythm: u8, pub last_serve_minute: Option<u32>,
}
pub enum PlayerAct { Greet { guest }, TakeOrder { guest }, ClearTable { table }, Wash { sink },
                     Mop { tile }, CollectTip { table }, Moment { id, option }, Checklist { id },
                     Talk { guest }, Experiment, GoTo { tile } }
pub enum OrderKind { Plain, Wish { wish: String }, Usual }
// Party: + order_kind, claimed_by_player: bool, wait_clock: f32, handed_off: bool
// OrderItem: + origin: ItemOrigin { Player { recipe, quality, match_score }, Helper { quality } }
// Staff: + duties: Vec<Duty>, nights_worked: u32
pub enum Duty { Dishes, Floor, Register, Bar, Kitchen, Serving }
pub struct Knowledge { recipes: BTreeMap<String, RecipeStats>, ingredients: BTreeSet<String>,
                       failed_tries: u32, revealed: BTreeMap<String, Vec<String>> }
pub struct MomentState { active: Vec<ActiveMoment>, tonight: u32, last_minute: Option<u32>,
                         history: BTreeMap<String, (u32 /*count*/, u32 /*last night*/)> }
pub struct HallMoodState { mood: Option<HallMood>, until: Option<u32> } // from the record (§10) or an effect
pub struct Rush { until: u32 }
```

**Work loop changes (`staff.rs`):**
1. Skip the owner in the auto job picker; drive her from `PlayerState`.
2. Helpers' candidate tasks are filtered by `duties`: `TakeOrder` only for `OrderKind::Plain` (or
   `handed_off`), never regulars; `Prepare`/`Serve` only for `ItemOrigin::Helper`; Dishes/Floor as listed.
3. Delete the "everyone helps at 4" fallback.
4. Tips: on party leave, put `TipLeft` on the table (not straight into money). `CollectTip` (Karolina) or a
   Dishes helper clearing that table moves it to `money`. At night end uncollected tips auto-collect,
   so nothing is ever lost.
5. Wait clock replaces direct patience decay (§1.4); `ServedFastStreak` measures wait-clock minutes.

**Commands** (extends brief §6):
| command | notes |
|---|---|
| `PlayerTakeOrder { guest }` | queues TakeOrder; marks the party claimed; error if a helper already took it |
| `PlayerBrew { guest, recipe, quality, mode: BrewMode { Quick, ByHand } }` | `quality` ignored for Quick; validates the recipe is known and base_item available; she walks, works, serves |
| `PlayerUsual { guest }` | = PlayerBrew with the regular's favourite recipe, ByHand (UI plays the mini-game first) |
| `PlayerHandOff { guest }` | wish → plain ticket (match cap 0.5) for helpers |
| `PlayerCancel` | cancels the current act; a claimed party is released (wait clock back to 1.0) |
| `PlayerQueue { act, front: bool }` / `PlayerUnqueue { index }` | micro-actions |
| `PlayerWashProgress { sink, seconds }` | hold-to-wash, sent per release |
| `SetDuty { staff, duty, on }` | capacity-checked → `CommandError::Capacity`; reversible (undo) |
| `ResolveMoment { id, option }` | Karolina walks there; effects on arrival |
| `TryExperiment { ingredients }` | returns via event; Night: queues an Experiment act |
| `SetSpecials { recipes }` (SHOULD), `ReserveSeat { seat, regular }` (SHOULD) | |

**Events:**
`PlayerOrderTaken { guest, wish: Option<String>, kind }`, `PlayerServed { guest, recipe, match_score, quality }`,
`PlayerQueueChanged`, `GuestReaction { guest, tier, key }`, `TipLeft { table, amount }`, `TipCollected { amount }`,
`RecipeLearned { recipe, source }`, `IngredientUnlocked { id }`, `ExperimentResult { result: Known(id) | Discovered(id) | Gated(key) | Close { key, family } | Nothing(key) }`,
`MarginNote { recipe, ingredient }`, `MomentAvailable { id, anchor }`, `MomentExpired { id }`, `MomentResolved { id, option }`,
`HallMoodChanged { mood }`, `RushStarted`, `RushEnded`, `RegularIncoming { regular, minutes }`, `KarolinaIdle { minutes }`,
`RhythmChanged { step }`, `CatLevel { level }`, `ThankYouNote { id }` (SHOULD), `PractisedRecipe { recipe }`.

**Queries:** `player()` (queue, current, brewing), `known_recipes()`, `recipe_available(&content, id)`,
`best_matches(&content, guest) -> Vec<(recipe, match)>` (top 4 for the order card), `pantry()`,
`active_moments()`, `ticket_rail()` (→ `Ticket { party, items, owner: Karolina|Helper(id)|None, wait_frac, regular }`),
`tips_on_tables()`, `hall_mood()`, `rush()`, `staff_duties(id)`, `capacity(id)`.

**New Conditions:** `RecipeKnown(id)`, `ServedRecipe { regular, recipe }`, `WishMatchStreak { regular, min, visits }`,
`HallMood(mood)`, `QuietHour`, `StaffLowEnergy(f32)`, `MomentResolvedEver(id)`, `IngredientUnlocked(id)`.
**New Effects:** `LearnRecipe(id)`, `UnlockIngredient(id)`, `HallMood { mood, minutes }`, `UnlockRecord(id)`, `PlayRequestedRecord`, `TargetTrust(f32)`, `SpawnMoment(id)`,
`GuestSatisfactionNear { add, tiles }`, `TargetStaffMood/Energy/Break` (moment target), `CameraHint(target)`
(→ event for render), `LogTrain` (LATER), `ThankYouNote(id)` (SHOULD).
All need `visit_refs` + validation.

**Balance harness (MUST):** `sim_headless --player-bot [quality] [react_seconds]`, a bot that acts like
a calm human (one act per `react_seconds` real s at ×1; mini-game quality fixed; Quick for plain orders).
It prints the idle share, max rail length, unserved/forgiven guests and money vs the pre-update baseline.

### 7.3 Story, trust and saves

* Trust and arcs: §1.7. All regular streaks and served-item checks now also accept the recipe variants
  (`ServedItem` matches the base_item of a Karolina-made recipe).
* Saves: `SAVE_VERSION = 2`. v1 saves load with: duties derived from priorities (priority 1 → duty),
  starter + menu-unlocked recipes known, an empty moment state. `World` tests: same seed + same commands ⇒ same save.

### 7.4 UI widgets and screens (UI-kit / mini-game / integration)

| Widget | Where | Notes | tag |
|---|---|---|---|
| **Order card** | glass popover 300×180 near the guest | portrait + name, wish text (13 pt) + taste icons (setting `show_taste_hints`, default on), up to 4 recipe chips (icon, name, tag icons; hover lights matching tags), **By hand/Quick** segmented toggle, **The usual?**, **Hand off**, "Book (K)"; Esc closes; keyboard 1–4 picks a chip | MUST |
| **Mini-game overlay** | bottom-centre 520×340 | §2.0; then a 1.2 s quality toast ("Lovely") | MUST |
| **Ticket rail** | top-centre under the clock card, a horizontal strip of chips (max 8, overflow "+3") | chip: guest portrait, item/wish icon, owner badge (Karolina / helper initial / hollow ring), wait ring (fills amber; never red on Cozy), regular gold edge; click = claim/focus camera | MUST |
| **Karolina's queue** | above her head (world space) | up to 5 icons; click to remove; current = progress ring | MUST |
| **Hold/swipe affordances** | at the sink / dirt tile | hold ring, swipe arrow hint (first 3 times) | MUST |
| **Tip coins** | on tables (world) | glint every 2 s, hover shows amount; click → arc to jar | MUST |
| **Tip jar** | HUD, next to the money chip | tonight total, rhythm glow; SHOULD goal marker | MUST |
| **Moment marker + card** | world marker; card 260×120 next to the anchor | 2–3 option buttons with cost chips; expiry = a thin ring around the marker | MUST |
| **Recipe book (K)** | §4.5 | pick mode from the order card | MUST |
| **Duties board (P)** | replaces priorities | rows helpers × 6 duty chips, capacity dots, Suggest; hire card with a one-click default duty | MUST |
| **Regular incoming chip** | on the rail | portrait + "to go"/"usual" hint, 2 game min ahead | MUST |
| **Reaction bubble + trust heart** | existing speech bubble | `GuestReaction` key, floating "+♥" | MUST |
| **Coach marks** | tutorial | arrow + glass tip, "Got it" link, skippable in settings | MUST |
| **Lull chips** | HUD right | "It's quiet — ×2?", "Grandma's margin note", "Make yourself a coffee" | MUST (first two) |
| Napkins tab, cat level chip, specials chalkboard, seat reserve card, trains tab | | | SHOULD/LATER |

Hotkeys: **K** recipe book, **Q** toggle Quick/By hand on the open order card, **H** hand off, **U** the usual,
**Tab** skip mini-game, **Shift+click** do next. **R** stays rotate in build.

### 7.5 i18n key scheme

New source files: `assets/i18n/src/handson.tr` (UI, duties, tutorial), `wishes.tr`, `recipes.tr`, `moments.tr`,
`extras.tr`, `records.tr`, `seasons.tr`, `journal.tr`, `prefs.tr`. Karolina is addressed by name and may use feminine
forms in cs/sk/uk (brief §1); other player-facing UI stays neutral.

| prefix | example | count (MUST) |
|---|---|---|
| `taste.<tag>` | `taste.comforting` | 12 |
| `wish.<id>` / `wish.<regular>.<id>` | `wish.long_day`, `wish.vera.properly` | 43 |
| `react.<tier>.<n>` / `react.<regular>.<tier>` | `react.good.3`, `react.mirek.perfect` | 20 + 28 |
| `recipe.<id>.name/.desc/.lore` | `recipe.r_turek.lore` | 22 × 3 |
| `ingredient.<id>.name`, `ingredient.family.<family>` | `ingredient.linden.name` | 28 + 9 |
| `book.*` (`book.tab.drinks`, `book.hint.<source>`, `book.gated.<recipe>`, `book.margin_note`) | | ~20 |
| `experiment.known/.discovered/.close.swap/.close.extra/.close.missing/.nothing.<n>` | `{family}`, `{recipe}` placeholders | ~12 |
| `minigame.<kind>.title`, `minigame.<kind>.prompt.<step>`, `minigame.quality.<tier>`, `minigame.skip`, `minigame.done` | `minigame.pour_over.prompt.bloom` | ~30 |
| `latte.<pattern>` | `latte.rosetta` | 10 |
| `moment.<id>.title/.text`, `moment.<id>.opt.<opt>`, `moment.<id>.result.<opt>` | `moment.m_station_cat.opt.ham` | 10 × ~7 |
| `duty.<id>.name/.desc`, `ui.duties.*` | `duty.dishes.name` | ~20 |
| `ui.order.*` (`by_hand`, `quick`, `usual`, `hand_off`, `book`), `ui.rail.*`, `ui.tipjar.*`, `ui.lull.*` | | ~25 |
| `hall_mood.<mood>` | `hall_mood.nostalgic` | 3 |
| `cat.level.<n>`, `cat.name.<opt>` | | 6 + 3 |
| `tutorial.<step>` | `tutorial.take_order` | ~14 |
| `app.digest.helper_<duty>` | "{name} washed {n} cups" | 6 |
| SHOULD: `thanks.<id>`, `ritual.<regular>`, `karolina.mug.<n>`, `record.<id>.name` | | 40 + 7 + 12 + 3 |

### 7.6 Who does what

| Agent | Takes |
|---|---|
| **sim** | §7.2 in full: PlayerState + queue + micro-acts, OrderKind/wishes/match, wait clock + Cozy forgiveness, duties + capacity + filters, tips on tables, recipes knowledge + experiments, moment director + lull director + rush/rhythm, new conditions/effects, save v2, the player-bot harness |
| **content** | `handson.ron`, `recipes.ron`, `wishes.ron`, `moments.ron`, `minigames.ron`, `tutorial.ron` (+ SHOULD `extras.ron`), the edits to existing files, all English text + i18n keys and cs/sk/uk translation, validation of new refs |
| **mini-game** (new area `src/minigame/**`, `examples/minigame_*.rs` with `SHIFTELLA_SHOT`) | the 5 MUST games (+2 SHOULD), the common frame, keyboard paths, Relaxed mode, latte shapes, audio hooks mapped to **existing** files only (§14) |
| **UI-kit** | order card, ticket rail, moment card, duties board, recipe book, tip jar, coach marks, taste icons (12) |
| **render** | `Sparkle`/`Smile` emotes, tip coin, queue icons in world, moment markers, cup latte glyphs, candles on tables, bucket, cat poses (SHOULD), the fogged window wipe |
| **integration** | input routing (world clicks → acts), overlay time scale + speed clamp, tick length, hotkeys, the ×2 chip, wiring events → UI/audio, saves |

---

## 8. Tutorial (MUST)

Data-driven coach marks: `assets/data/tutorial.ron`.
```ron
#![enable(implicit_some)]
[
    (id: "greet", night: 1, trigger: Event(GuestArrived), anchor: Guest(First), text_key: "tutorial.greet",
     done_when: Act(Greet), optional: true),
    (id: "take_order", night: 1, trigger: Regular("mirek"), anchor: Guest(Regular("mirek")),
     text_key: "tutorial.take_order", done_when: Event(PlayerOrderTaken)),
    (id: "pick_recipe", night: 1, trigger: After("take_order"), anchor: Widget("order.chip.0"),
     text_key: "tutorial.pick_recipe", done_when: Act(BrewChosen)),
    (id: "minigame", night: 1, trigger: After("pick_recipe"), anchor: Widget("minigame"),
     text_key: "tutorial.pour_over", done_when: Event(PlayerServed)),
    // quick_brew, tip_coin, clear_table, moment_window, wash (night 2), mop (night 2),
    // book_experiment (day 2), hire_duty (night 3), rail (night 3), the_usual (night 4)
]
```
Coach marks are glass tips with an arrow. They never block input or pause the clock (the order and mini-game
overlays already slow it). Each can be dismissed with "Got it", and the whole tutorial is toggled in settings.

Coach-mark texts are written as Karolina's own thoughts, or as Lida's notes in amber italics. They are
short, and none of them starts with "Click".

**Night 1 (solo, rain, ramp 0.45):**
1. 22:00 (after the `opening` cutscene): the opening ritual (§13.1). `tutorial.light_window`, a Lida note:
   *"The light in the window first, Karolinko. Always first."* → click the window lamp. Then
   `tutorial.machine`: "The old machine needs a minute to wake up." → hold on the coffee machine (1 s).
2. 22:04: Mirek enters (a scripted early visit for night 1). `tutorial.greet` (optional wave): "That's
   Mirek. Grandma's oldest regular."
3. `tutorial.take_order`: "He's waiting for me." → the order card shows `wish.mirek.like_lida`
   ("Black, Karolina. The way Lida poured it."). Only `r_black` and `r_lemon_tea` are offered, and r_black is highlighted.
4. `tutorial.pour_over`: inline ghost circles and the metronome; the skip button is shown ("Skip — it'll
   still be good"). Any result → Mirek's `react.mirek.<tier>` + trust.
5. 22:20: a traveller with a plain cappuccino → `tutorial.quick_brew`: "A plain order. I can let the
   machine work and do something else." While she brews, the tip coin on Mirek's table glints →
   `tutorial.tip_coin`, then `tutorial.clear_table`.
6. 22:47: Os 2141 passes and `m_fogged_window` is forced → `tutorial.moment`: "Something's glowing by the
   window…" → wipe → the train's light sweeps the room.
7. The rest of the night is a gentle flow. The lull chip "It's quiet — ×2?" appears once (`tutorial.speed`).
8. 05:45: the closing ritual (§13.1): cash up, handover note (one tap: "Write it for tomorrow's me").
9. Morning 1: the first polaroid develops in the summary (§11.4) → `tutorial.journal`: "Grandma kept a box
   of photos behind the till. I'll keep one too." (J).

**Night 2 (solo):**
* Day: grandma's card for koláč + the linden hint → `tutorial.book` (K) → one guided experiment (linden +
  honey) → *Linden tea* discovered.
* Night: Lida's jukebox (§10) glows softly at 22:10 → `tutorial.jukebox`: "Grandma's records. Three still
  play." → pick one → the music really changes.
* Dishes pile up → `tutorial.wash` (hold); a dirt blot by the door → `tutorial.mop` (swipe). `note_01` under the till (existing).

**Night 3:**
* Day: `story.hint.hire` → the hire card with the one-click default duty → `tutorial.duties`: "Kuba takes
  the dishes. I take the guests." → the rozpis opens with Kuba's preference chip (§12) →
  `tutorial.rozpis`: "He wants to work with the cook. Noted." → Publish.
* Night: `tutorial.rail` when the first unowned ticket appears; the first `m_station_cat` (`tutorial.cat`).

**Night 4:** `tutorial.usual` (Mirek reaches trust 50 around here; if not, it shows the first time it
triggers), and `tutorial.atmosphere` the first time the atmosphere meter (§11.3) moves a level: "It
feels warmer in here when the lamps are on." Then no more coach marks; the hints move to the existing
`story.hint.*` toasts and @shiftella_bot (§13).

**Later one-offs:** night 8 (the first full week with 3+ helpers) `tutorial.fairness` on the rozpis
wellbeing ring; night 21 `tutorial.garlands` (§11.1).

---

## 9. Acceptance and balance targets

Measured with the player-bot (`react_seconds = 3`, quality 0.75) over story nights 1–10 on Cozy:

| Metric | Target |
|---|---|
| Karolina idle share at ×1 (no meaningful action available) | ≤ 15 % of night minutes (quiet hour excluded) |
| Pending things for Karolina (rail + markers + dirty) | median 1–3; Cozy rush peak ≤ 7 |
| Guests leaving unhappy on Cozy | 0; "forgiven" leaves ≤ 10 % of parties |
| Moments per night | 2–4 (act 1: 1–3); never two open at once on nights 1–3 |
| Share of orders made by Karolina by hand | nights 1–2 100 %; nights 8–16 ≥ 45 %; ≥ 70 % of regulars' orders |
| Mini-games per night | 6–14, with ≤ 3 of the same kind in a row by default |
| Money vs the pre-update baseline | within ±15 % by night 14 (retune `BASE_PARTIES_PER_HOUR`, helper speed) |
| Real length of a night at ×1 | 12 min ±1 (overlays add ≈ 1–2 min) |

Manual checks: a whole night can be played keyboard-only; Relaxed mode plus skipping every mini-game
still reaches «Light in the Window» on Cozy; nothing on Cozy produces a red indicator.

---

## 10. The jukebox: real records (MUST)

Owner: choosing a record must **really** change the music in the café, not just a stat. Six loopable
tracks of about 95 s exist (`assets/audio/music/jukebox_*.ogg`). No further audio is generated (§14).

### 10.1 Grandma's jukebox
* Lida's jukebox stands in the default café from night 1: the sim's `DEFAULT_LAYOUT` places `jukebox`.
  The furniture def becomes `unique: true`, with tags `music`, `grandma`, `decor`, `light`, and needs power.
  `unlock_night` only matters if the player removes it and wants it back.
* It is clickable **any time**: at night Karolina walks over and puts the record on (an act, 1 s "lift
  the needle"); in the Day phase it simply sets the record that starts at 22:00.
* It is introduced on night 2 by a soft glow and `tutorial.jukebox` (§8).

### 10.2 The records

| id | title (en, `record.<id>.title`) | sleeve line (`.artist`) | mood | track | unlock | liked by | disliked by | archetypes like |
|---|---|---|---|---|---|---|---|---|
| `rec_prague_swing` | Holešovice Swing, 1938 | The Tram Twelve Band — clarinet & brushes | Lively | `jukebox_prague_swing` | starter | mirek, radek | — | taxi_drivers, clubbers |
| `rec_piano_trio` | Last Tram Trio | Vltava Night Trio | Calm | `jukebox_piano_trio` | starter | radek, guest_anwar | — | insomniacs, night_wanderer |
| `rec_bossa_rain` | Bossa for a Rainy Platform | Lina Morais & friends | Dreamy | `jukebox_bossa_rain` | starter | lori | mirek ("Too much samba for a Czech night, eh?") | travelers, students |
| `rec_folk_lullaby` | Kolyskova — Carpathian Lullabies | bandura, cimbalom & accordion | Tender | `jukebox_folk_lullaby` | Oleh is hired ("he brought it from Lviv"), **or** Hana arc ≥ 2, **or** night ≥ 13 | hana, vera | — | nurses, insomniacs (clubbers dislike) |
| `rec_synth_2039` | Holešovice 2039 | SLEEPER CAR | Dreamy | `jukebox_synth_2039` | flag `filip_friends` (Filip cut it himself), **or** night ≥ 16 | filip, guest_anwar | vera ("That's not music, that's a fridge.") | students, drone_operators |
| `rec_cafe_waltz` | Lida's Waltz | a handwritten label: "Pro J." | Nostalgic | `jukebox_cafe_waltz` | ≥ 3 notes found (in an empty sleeve in the storeroom), **or** night ≥ 20 | vera, mirek, guest_333 | — | insomniacs |

Sandbox: the 3 starters, then the others unlock by their conditions or on nights 4 / 8 / 12, whichever comes first.
Unlocking shows a toast with the sleeve and the source line (`record.<id>.source`).

### 10.3 Flow and music states
* **Picker** (popover from the jukebox, 520×150 pt): 7 sleeves in a row. *House playlist* comes first,
  then the 6 records; locked ones are silhouettes with `record.<id>.hint`. Each sleeve shows a
  procedural motif (tram, raindrops, piano keys, embroidered sunflower, synth grid, a rose), the title,
  a mood chip, and the faces of regulars **whose taste Karolina has already seen** (they are revealed by
  their first reaction). The sleeve now playing spins. A **Repeat ⟳ / Play once** toggle sits below, default Repeat.
  Keyboard: ←/→ and Enter.
* **States:** `House { Calm | Rush | Quiet }` or `Record { id, repeat }`.
  - House picks `mus_night_calm_loop`, `mus_rush_loop` during a Rush (§1.5), and `jukebox_piano_trio` (volume 0.7) in the quiet hour.
  - A record plays until the night ends or another one is chosen (Repeat). With Play once it plays one
    pass and House resumes.
  - Rush never overrides a record: the player chose it.
  - A blackout stops the music (silence + rain, which is lovely with candles); when the power returns,
    the record restarts.
  - A story cutscene with a `Music` step takes over, and the record resumes after it.
    Lori's set on night 24 fades the jukebox out.
  - Night end → House off; the Day phase plays `mus_menu_theme` as today.
* **Crossfade:** 2.5 s real time, two music voices in `audio.rs`; the needle drop is `ui_click` at 0.3.

### 10.4 Effects
The record sets **HallMood** while it plays (no duration). House = no mood.

| mood | records | gentle effect |
|---|---|---|
| Lively | swing | tips ×1.08, helper speed ×1.05, Sweet/Fancy wishes ×1.3, guest stay −5 % |
| Calm | trio | guest stay +10 %, Mild/Comforting wishes ×1.3, coziness +3 |
| Dreamy | bossa, synth | Fancy/Cold/Fresh wishes ×1.3, students & travellers +2 satisfaction |
| Tender | lullaby | coziness +4, `m_sleeping_guest` weight ×2, insomniacs & nurses patience +20 % |
| Nostalgic | waltz | Comforting wishes ×1.3, regulars aged ≥ 60 +1 trust per visit |

* **Likes:** a regular present while a liked record has played ≥ 5 game minutes → once per visit:
  `Music` then `Heart` emote, trust +2 (+3 if it came from `m_record_request`), a bubble
  `record.<id>.liked.<regular>` (13 pairs). The first time also reveals the face on the sleeve.
  The 5-minute rule stops record-flipping from farming trust.
* **Dislikes:** once per visit, a `Note` emote and the bubble `record.<id>.disliked.<regular>`
  (gentle, e.g. "Not my kind of music, but it's your café."), satisfaction −2. **Never** a trust loss.
* **Archetypes:** `music_likes` / `music_dislikes` (record ids) on `GuestArchetype` → +2 / −1 satisfaction.
* **Flavour:** if *Lida's Waltz* is on when the 3:33 guest sits down, his bubble becomes
  `record.rec_cafe_waltz.liked.guest_333` ("He hums along, very quietly."). It gives +2 trust and
  changes no story flags.
* `m_record_request` (§5.2) is the moment form: "Karolina, have you got any swing?"

### 10.5 Data: `assets/data/records.ron`
```ron
#![enable(implicit_some)]
[
    (id: "rec_prague_swing", title_key: "record.rec_prague_swing.title", artist_key: "record.rec_prague_swing.artist",
     mood: Lively, track: "jukebox_prague_swing", seconds: 95.0, unlock: Starter,
     liked_by: ["mirek", "radek"], disliked_by: [], sleeve: (color: "#C9A46B", motif: Tram)),
    (id: "rec_folk_lullaby", title_key: "record.rec_folk_lullaby.title", artist_key: "record.rec_folk_lullaby.artist",
     mood: Tender, track: "jukebox_folk_lullaby", seconds: 95.0,
     unlock: When([Any([HiredArchetype("sound_tech"), ArcAtLeast("hana", 2), NightAtLeast(13)])]),
     sandbox_night: 4, source_key: "record.rec_folk_lullaby.source",
     liked_by: ["hana", "vera"], disliked_by: [], sleeve: (color: "#E0A96D", motif: Sunflower)),
    // …
]
```
`RecordDef { id, title_key, artist_key, mood: HallMood, track /* MUSIC_TRACKS */, seconds, unlock: Starter | When(Vec<Condition>),
sandbox_night, source_key, liked_by, disliked_by, sleeve }`. `content::MUSIC_TRACKS` gains the six jukebox ids, and validation
checks that `track` is listed. Archetype `music_likes` / `music_dislikes` go in `guests.ron`.
`seconds` is measured once with ffprobe and lets the integration end a Play-once pass, since macroquad
has no end-of-sound callback.

### 10.6 API
* `Command::PlayRecord { id: Option<String> /* None = House */, repeat: bool }`. At night it queues
  Karolina's act; in the day it applies at once. Errors: `Locked`, `NoJukebox`, `NoPower`.
* `Command::RecordFinished { id }`: sent by the integration when a Play-once pass ends (real seconds).
* Events: `SimEvent::RecordChanged { record: Option<String>, repeat }`, `HallMoodChanged { mood }`,
  `RecordUnlocked { id }`, `RecordReaction { guest, record, liked }`.
* Queries: `music() -> MusicState { source, house }`, `records()` (unlocked set + `known_tastes: BTreeSet<(record, regular)>`).
* Save: unlocks, tastes and tonight's opening record persist. The music state of a running night does not.

---

## 11. The café changes over time (MUST)

### 11.1 Seasons

**Story calendar** (proposal for the content agent; it fits the consecutive story weekdays, with night 1 on a
Monday): night 1 = **Mon 7 Nov 2039** … night 30 = **Tue 6 Dec 2039**. The 1st Advent Sunday is 27 Nov =
night 21; St Nicholas' Eve (Mikuláš) is Mon 5 Dec = night 29. The date appears on polaroids and on the day card.

| phase | story nights | sandbox season | visuals (render, procedural) | gameplay |
|---|---|---|---|---|
| `late_autumn` | 1–13 | Autumn | wet leaves on the pavement and doorstep, heavy rain, amber street light | Warm wishes ×1.2; chai/linden on the specials suggestions |
| `frost` | 14–20 | late Autumn | frost in the window corners, breath puffs for people outside, more fog | Warm ×1.4, Cold ×0.6; snowflake latte unlocked |
| `advent` | 21–26 | early Winter | garlands + fairy lights (after `m_hang_garlands`), an Advent wreath on the counter, a star on the holo sign | svařák/buchty suggested; `m_advent_candle` on nights 21 & 28 |
| `first_snow` | 27–30 | Winter | snowfall particles, snow caps on the sign and the street edge, frosted glass, steam from cups stronger | `m_first_snow` (night 27 has the story's first snow), Warm ×1.5; Mikuláš on night 29 |

Sandbox: `town.season` already advances every 30 nights. Spring = light rain and petals (pink particles),
the open-window moment ×3; Summer = warm nights, Cold/Fresh wishes ×1.5, open windows; Autumn = late_autumn
visuals; Winter = frost, then garlands and snow for the last 10 nights of the season.

**Seasonal moments:**
* `m_hang_garlands` (MUST), story night 21 / sandbox winter start: a box labelled "Vánoce — L." appears
  on the counter. Opening it shows 4 glowing hang spots (2 windows, the counter, the door). Karolina hangs
  each one: a click, then she steps on a stool for 1.5 s. When all 4 are up → `SetFlag("garlands_up")`,
  coziness touch +2 for the season, crew mood +5, a polaroid candidate (70), and the theme
  `winter_lights` unlocks. Not done → the crew hangs them the next morning. Never failable.
* `m_advent_candle` (SHOULD): light the 1st / 2nd candle on the wreath (one click); coziness +1 each.
* `m_mikulas` (SHOULD), night 29, 22:00–23:00: a student troupe dressed as Mikuláš, an angel and a devil
  asks for something warm "for a poem". **Hot chocolate on the house** (3 Quick brews by Karolina) →
  Reputation +3, a polaroid candidate (75) · **A koláč for each** · **Just applause**.

Data: `assets/data/seasons.ron`:
```ron
#![enable(implicit_some)]
(
    story_start_date: (2039, 11, 7),
    phases: [
        (id: "late_autumn", story_nights: (1, 13), sandbox: [Autumn],
         visuals: (leaves: true, frost: 0.0, breath: false, garlands: false, snow: false, wreath: false),
         wish_mul: [(Warm, 1.2)], latte: []),
        (id: "frost", story_nights: (14, 20), sandbox: [],
         visuals: (leaves: true, frost: 0.5, breath: true), wish_mul: [(Warm, 1.4), (Cold, 0.6)], latte: ["snowflake"]),
        (id: "advent", story_nights: (21, 26), visuals: (frost: 0.6, breath: true, garlands: true, wreath: true),
         wish_mul: [(Warm, 1.4), (Spiced, 1.3)], moments: ["m_hang_garlands", "m_advent_candle"]),
        (id: "first_snow", story_nights: (27, 30), sandbox: [Winter],
         visuals: (frost: 0.8, breath: true, garlands: true, wreath: true, snow: true),
         wish_mul: [(Warm, 1.5)], moments: ["m_first_snow", "m_mikulas"]),
    ],
)
```
`garlands: true` means "allowed": garlands render only once `garlands_up` is set.
Sim: `season_phase() -> &PhaseDef`; condition `SeasonPhase(id)`; event `SeasonPhaseChanged { id }` at day start (a soft toast).

### 11.2 Decor themes

`ThemeDef { id, name_key, desc_key, unlock, price, tints: ThemeTints, wall_pattern, coziness, season, accent_tag }`.
The theme applies to the **hall** (the bar and kitchen keep theirs). It is chosen in Build (B) → *Theme*
tab: live preview tiles, buy once, switch for free in the Day phase only. Render cost: tints + a procedural
wall pattern + a lamp warmth shift. No new sprites are needed for MUST.

| id | look | coziness | unlock | price | touch |
|---|---|---|---|---|---|
| `grandma_original` | faded rose-stripe wallpaper, honey wood, lace textiles, amber lamps | 6 | default | 0 | Vera, nights ≤ 14, when it's swapped away: "Lida's wallpaper…" (ambient) |
| `art_nouveau` | Prague Secession: sage & gold mosaic band, brass lamps, curved trims | 5 | Reputation ≥ 50 or tip jar goal 3 | 6 000 | +1 beauty per lamp |
| `station_retro` | 1979 railway buffet: cream & bottle-green paint, enamel signs, a timetable board | 4 | item `timetable_1979` (Radek arc 2) or night ≥ 18 | 4 000 | Radek/Mirek +1 trust once per night on entering |
| `nocturne_2039` | indigo holo accents, warm LED strips, Shiftella violet | 4 | Filip arc ≥ 3 or sandbox night 10 | 5 000 | students & drone operators +2 satisfaction |
| `winter_lights` (seasonal) | white-gold fairy lights, red ribbons, spruce | 8 in advent/first_snow, else 2 | flag `garlands_up` | 0 | stacks visually with garlands |

SHOULD: themed accent pieces that reuse sprites with a tint (`lamp_brass` = pendant_lamp, `mosaic_rug` =
rug, `enamel_sign` = menu_holo_board without glow, `led_strip`). Each carries `theme:<id>`, and a matching
piece adds +1 coziness (max +2).
```ron
(id: "station_retro", name_key: "theme.station_retro.name", desc_key: "theme.station_retro.desc",
 unlock: When([Any([HasItem("timetable_1979"), NightAtLeast(18)])]), price: 4000, coziness: 4.0,
 wall_pattern: Wainscot,
 tints: (wall: "#E9E1C8", wall_trim: "#2F5D4A", floor: "#8A6A4A", textile: "#2F5D4A", lampshade: "#F3E7C9", lamp_warmth: 0.1),
 accent_tag: "theme:station_retro"),
```
Commands: `BuyTheme { id }`, `SetTheme { id }` (Day only, undoable). Events: `ThemeUnlocked`, `ThemeChanged`.

### 11.3 Coziness (atmosphere)

`coziness: f32` from 0 to 100, recomputed every game minute for the hall:

| component | weight | measure |
|---|---|---|
| light | 20 | share of hall seats inside the radius of a light that is on (lamps; candles during a blackout); window light on +2 |
| beauty | 20 | Σ hall beauty / (hall tiles × 0.35), capped at 1 |
| comfort | 15 | mean seat comfort / 6 |
| clean | 20 | 1 − mean hall dirt, −0.05 per table with dishes |
| music | 10 | off 0 · house 6 · record 10 · blackout 0 |
| theme | 10 | theme coziness + accent pieces, /10 |
| touches | 5 | cat level ≥ 4 (+2), garlands up in season (+2), ≥ 4 plants (+1), napkin wall (+1) |

HallMood adds Calm +3 / Tender +4; the result is clamped to 100.

**Effects.** These *replace* the separate beauty/dirt/music/comfort terms in today's satisfaction formula,
so nothing is counted twice:
* satisfaction at serve += `(coz − 50) × 0.12` (−6 … +6)
* patience × `0.9 + coz / 250` (0.9 … 1.3)
* stay × `0.9 + coz / 500`
* tips × `0.96 + coz / 1250`

Condition `CozinessAtLeast(n)` is available to content.

**Meter** (HUD bottom-left): a candle flame with 5 levels: *Bare* < 20, *Plain* < 40, *Pleasant* < 60,
*Cosy* < 80, *Like home* ≥ 80 (`atmo.level.<n>`). Hovering shows the component bars and **one** tip from
the weakest component (`atmo.tip.<component>`: "The window tables are dark.", "Dishes are piling up.",
"Some music would help.", "A rug or a plant would soften the hall.", "The chairs are hard — maybe a sofa?").
The world highlights it at the same time (the dark seats glow faintly, dirty tables pulse).
A level-up plays `ui_click` 0.4 and the flame grows; the first *Like home* is a polaroid candidate (60).

What the player DOES: at night every hands-on chore (clear, mop, lamps, records) visibly moves the
meter; in the day she builds and picks the theme.

### 11.4 The polaroid journal (J)

**Capture (sim, deterministic).** On notable events the sim records a `Highlight { night, minute, kind, score,
focus: (i32, i32), subjects, caption_key, args, snap: SceneSnap }` and keeps the top 3 of the night. A
kind already kept the previous night scores −20, for variety. Every night is guaranteed one candidate:
a fallback snapshot at 03:00 focused on the window.

`SceneSnap { focus, window: (10, 8) tiles, clock, weather, phase, theme, blackout, window_light,
furniture: Vec<(def, x, y, rot, on)>, agents: Vec<SnapAgent { look: LookRef, feet, facing, pose, emote,
holding, seat_tile }>, cup_art: Vec<((i32, i32), pattern)>, garlands, snow }`. About 1–3 KB per entry,
stored in `World.journal` (saved). No image files are needed. The render can re-draw any polaroid any time,
even after the café has been rebuilt.

| kind | score | caption example (`journal.caption.*`) |
|---|---|---|
| Guest333 (first visit / later) | 90 / 40 | "3:33. The clock stopped, and he was there again." |
| FirstSnow | 85 | "The first snow, through grandma's window." |
| ArcBeat (ArcAdvanced; +10 for ArcGood) | 80 | `journal.caption.arc.<regular>.<stage>`, e.g. "Mirek laughed tonight. First time I heard it." |
| Mikuláš | 75 | "An angel, a devil and three hot chocolates." |
| StoryScene (key cutscenes only) | 70 | `journal.caption.scene.<cutscene>` |
| GarlandsUp | 70 | "Grandma's garlands, up again." |
| CozyLikeHome (first) | 60 | "It felt like home tonight." |
| PerfectServe for a regular | 55 | "{name} said it was exactly right." |
| CatLevel (≥ 4) | 55 | "The cat stayed. I think it lives here now." |
| FullHouse (≥ 90 % seats) | 50 | "Every chair taken. Grandma would have loved it." |
| BeautifulLatte (q ≥ 0.95) | 45 | "A {pattern} for {name}." |
| MomentWarm (blanket, candles, lost traveller…) | 40 | `journal.caption.moment.<id>` |
| RecordLoved | 35 | "{name} hummed along to {record}." |
| RecipeDiscovered | 30 | "My own recipe: {recipe}." |
| RozpisHappy | 25 | "Everyone got the nights they wanted." |
| AnwarCameo | 20 | "A quiet guy with a laptop. Nice smile." |
| QuietNight (fallback) | 10 | "A quiet night. Rain on the window." |

**Look** (render): `render::draw_scene_snap(&snap, rect)` draws the dollhouse view into a 360×270 target
with a photo grade (warm vignette, a soft bloom, procedural grain, a slight flash highlight). It sits in a
white polaroid frame (14 px border, 40 px bottom), rotated −3…+3° by seed. The caption goes in the bottom
band and the date in the corner ("7. 11. 2039", per language). MUST font: Inter. SHOULD: an OFL handwriting
font (e.g. Caveat) in `assets/fonts/` with its licence file; it is a font, not audio.

**Morning (what the player DOES):** the summary shows the best candidate *developing* (white → photo over
2 s; dragging a little "shake" makes it faster). Two smaller alternatives sit underneath; one click keeps
one of them instead. SHOULD: pick 1 of 3 caption variants (`.a/.b/.c`); there is no typing.
`Command::KeepPolaroid { night, index }` (default 0 if the summary is closed).

**Journal (J):** a scrapbook on a procedural kraft-paper glass panel, 760×480: 6 polaroids per page (2×3),
the night number and title under each. Story has 30 slots (empty ones are dashed: "night 12"); sandbox
has endless pages. Clicking one enlarges it with the caption, date and night title; ←/→ flip pages.
SHOULD: the ending screen opens with a collage of the player's polaroids, and the credits roll over them.

Events: `HighlightCaptured { kind, score }` (a tiny white flash in the HUD corner, no sound needed),
`JournalUpdated { night }`. Queries: `journal()`, `tonight_highlights()`.

---

## 12. The rozpis as a light puzzle (MUST)

### 12.1 What the player DOES
* **Once a week** (the Day phase before a Monday night; the T button shows a badge), about a minute: read
  the preference chips, press **Auto-fill**, fix the one or two amber chips by clicking cells, glance at the
  wellbeing rings, then **Publish**.
* **Mid-week:** answer 0–2 requests that come *from* preferences or life events (§13.3).
* Karolina's own row is first and locked to Full every night: "Karolina (you)". She is always there,
  because the game is player-first. **Under-staffing is allowed:** it just means more hands-on work,
  shown in indigo, never red.

### 12.2 Preferences
`Pref { kind, strength: Need | Wish, quote_key }`, with 1–3 per person.

| kind | chip icon | example quote | honoured when |
|---|---|---|---|
| `DayOff(Weekday)` | book + day | "I have lectures on Tuesdays." | that weekday is Off |
| `ShiftPref(Early \| Late \| Full)` | owl / sunrise / moon | "Night owl — late shift, please." | every working night uses it |
| `WeekendOff` | house | "Weekends are for my family." | Need: Fri and Sat Off · Wish: at least one |
| `NoBackToBack` | broken chain | "Not two nights in a row — my back." | no two consecutive working nights (incl. last week's Sunday) |
| `MaxNights(n)` | "≤ n" | "Three nights a week — I've got a little one." | count ≤ n |
| `MinNights(n)` | coin "≥ n" | "I need the hours." | count ≥ n |
| `WorksWith(staff)` | heart + mini portrait | "I love working with Bára." | ≥ 4 h shared on ≥ 2 nights |
| `AvoidWith(staff)` (dynamic) | cloud + portrait | "Not with Zdeněk this week, please." | never shares a shift; appears when relationship < −30, goes away > −10 |

**Story people** (`staff.ron` → `prefs`):
| person | prefs |
|---|---|
| Bára (barista) | ShiftPref(Late) Wish; DayOff(Sun) Wish ("Sunday lunch with my parents. They think I'm still at uni.") |
| Zdeněk (cook) | NoBackToBack **Need** ("My back."); ShiftPref(Full) Wish |
| Kuba (porter) | WorksWith(cook) Wish ("Mr Holub lets me watch him bake.") → ≥ 3 shared shifts teaches `r_buchty` |
| Ivana (cashier) | ShiftPref(Early) **Need** ("Home by half past two — my daughter wakes at night."); MinNights(4) Wish |
| Oleh (sound_tech) | DayOff(Thu) Wish ("Thursday's band rehearsal — and the video call home."); ShiftPref(Late) Wish |
| Ema (quiet_one) | DayOff(Tue) **Need** ("Lectures on Tuesdays."); WeekendOff Wish ("Crowds.") |
| Natália (server) | WorksWith(barista) Wish; MaxNights(5) Wish |

**Sandbox** generation from traits: night_owl → Late, early_bird → Early, lazy → MaxNights(3),
hard_worker → MinNights(5), loner → WeekendOff, anxious → NoBackToBack, romantic → WorksWith(partner) when
a romance exists; plus 1 random of DayOff(random) / WeekendOff / NoBackToBack / MaxNights(4) (30 % Need). Max 3.

**Life events** become one-week prefs: Bára's exam (story ~night 12) is DayOff(that weekday) as a Need, plus a time-off request.

### 12.3 Wellbeing and fairness
Per person, live on the draft:
`wb = 100 − 25 × violated Needs − 12 × violated Wishes − 8 × nights above 5 + 4 × honoured Wishes (max +8)`, clamped 0–100.
Rings: ≥ 85 *Happy* (emerald), ≥ 65 *Fine* (indigo), ≥ 45 *Tired* (amber), else *Stretched* (soft violet).
Never red.

Week: `fairness = mean(wb) − 0.5 × stdev(wb)` → a small weather icon (sunny / mostly sunny / cloudy /
rain-but-cosy). The coverage footer shows per night the helpers suggested by `weekday_demand` (existing) as
dots, plus a busy-ness hint (1–3 cups). For example "Fri: 3 suggested, 2 on — you'll be busier".

### 12.4 Effects (from the published week)
* Mood baseline for the week: `+ (wb − 70) × 0.15`.
* Energy at shift start: `100 − (100 − wb) × 0.25` (≥ 75).
* Late chance × `lerp(1.6, 0.5, wb/100)`, sick chance × `lerp(1.5, 0.6, wb/100)`.
* A violated **Need** on its night: 50 % they send a swap request with a suggested colleague who can cover
  without breaking their own Needs; otherwise they come in a bit low (mood −8 tonight). Never a walkout
  from this alone.
* An honoured `WorksWith`: relationship +1/h extra while together, and +5 % speed for the pair.
* **Happy week** (every Need and Wish honoured, coverage ≥ suggested − 1): event `RozpisHappy` → crew mood
  +3 all week, a bot line `app.bot.rozpis_happy`, a polaroid candidate.

### 12.5 Auto-fill
`AutoFillRozpis` extends the existing `default_rozpis`:
1. Karolina is Full every night.
2. Go through the nights from busiest to quietest (demand). For each slot up to the suggested coverage,
   add the helper with the lowest cost: a Need broken = skip (only used when nobody else can and coverage
   would fall 2 below the suggestion); a Wish broken = +1; balance = +0.3 × nights already given;
   `WorksWith` partner already on = −0.5; `MinNights` unmet = −0.4.
3. Shift type: their ShiftPref; otherwise Full on busy nights and Early/Late pairs to split.
4. A one-line result: "Auto-fill kept 9 of 11 wishes. Ema's 'weekend off' didn't fit Saturday." Click it
   to flash the affected chips. It fills only the **draft**, so the player can tweak it, and undo works as today.

### 12.6 UI (T → Rozpis)
Each row: portrait, name, preference chips (hover shows the quote; honoured = emerald glow, violated =
amber with a small dot), 7 cells (click cycles Off → Full → Early → Late; right-click goes back; drag
paints), and the wellbeing ring. Footer: coverage dots + cups per night, the week weather icon,
**Auto-fill**, **Clear**, **Publish**. A side panel "Tonight" lists who is on tonight and their duties
(§1.3), with a link to the duties board: the rozpis decides who Karolina can delegate to.

### 12.7 API
* Data: `staff.ron` archetypes get `prefs: [(kind: DayOff(Tue), strength: Need, quote: "staff.quiet_one.pref.tuesday")]`;
  sandbox rules go in `handson.ron` → `pref_rules`.
* Sim: `Staff.prefs`, `Staff.temp_prefs: Vec<(Pref, until_week)>`, `AppState.wellbeing`, `AppState.fairness`.
* Commands: `AutoFillRozpis`; existing `SetShift` / `PublishSchedule`.
* Events: `RozpisScored { wellbeing, fairness }` (on every draft change), `PrefViolated { staff, pref }`
  (night start), `RozpisHappy`.
* Queries: `prefs(staff)`, `pref_status(staff) -> Vec<(Pref, Honoured | Violated | Pending)>`,
  `wellbeing(staff)`, `week_fairness()`, `coverage() -> [CoverageNight; 7]`.

---

## 13. The Shiftella app as a real tool (MUST)

Owner: the in-game app must be **useful, not decorative**. Every tab feeds tonight's play, and every
message has a button that does something in the world. The bot never does clock-in/out.

### 13.1 Checklists → opening and closing rituals
Items now have an **owner** (`Karolina | Duty(d) | Either`), an **anchor** in the world (the item shows a
small checkbox chip on its furniture), a **gesture** (`Click | Hold(s) | Swipe`), and real `done` /
`missed` effects. Tapping the item in the app queues it for Karolina. A helper with the matching duty does
`Duty`/`Either` items when free.

| list | item | owner | gesture | done | missed (gentle) |
|---|---|---|---|---|---|
| Opening | `light_window` | Karolina | click lamp | needed for 3:33; window light +2 coziness | Reputation −1 (existing) |
| Opening | `coffee_warmup` | Either (bar) | hold machine 1 s | coffee prep ×0.8 for the first hour, quick quality +0.05 | the first 3 coffees take ×1.3 ("the machine is cold") |
| Opening | `check_stock` | Either (kitchen) | click fridge | the day card's low-stock list is applied: auto-restock prioritises it; tonight's out-of-stock risk is shown | 15 % chance of a "we're out of X" toast later (the item is off for 30 min) |
| Opening | `wipe_tables` | Either (floor/dishes) | swipe across tables | Dirt −0.05 (existing) → coziness | — |
| Opening | `count_float` | Either (register) | click register | payments ×1.2 faster for the first 2 h | — |
| Opening | `menu_board` | Karolina | pick 2 specials on the chalkboard | specials bonus (§6.8) active tonight | no specials tonight |
| Opening | `record_on` (new) | Karolina | pick a record | music component 10 from 22:00 | — |
| Closing | `wash_dishes` | Either (dishes) | hold sink | — | Dirt +0.1 tomorrow (existing) |
| Closing | `clean_machine` | Either (bar) | hold machine 2 s | coffee machine wear −0.02 (fewer breakdowns) | wear +0.02 |
| Closing | `cash_up` | Either (register) | click register | digest insights complete (§13.6), tips booked tonight | the digest says "cash not counted"; tips book tomorrow (nothing is lost) |
| Closing | `fridge_check` | Either (kitchen) | click fridge | fridge wear −0.01 | — |
| Closing | `handover_note` | Karolina | pick notes (§13.5) | effects tomorrow | none |
| Closing | `light_off_at_six` | Karolina | one click at 06:00 (opens the morning summary) | a quiet ritual; the endings use it | — |

**Rituals:** the whole Opening list by 22:30 → toast "The café is ready." → crew mood +2 and coziness +2
tonight. The whole Closing list → "Closed properly." → tomorrow's Opening items take half the time.
Sim: `ChecklistDef` += `owner, anchor, gesture`; `TickChecklist { id }` becomes a queued act for Karolina
(walk + gesture); events `ChecklistTicked { id, by }`, `RitualComplete { list }`.

### 13.2 Rozpis
§12.

### 13.3 Requests with consequences
Requests are generated **from preferences and life events**, not at random: a Need is about to be broken
→ a swap request; Bára's exam → time off; Ivana's daughter is ill → a new `EarlyLeave` request ("Can I
go at 2?"); toil > 8 h → a náhradní volno request.

| answer | consequence |
|---|---|
| approve | their wellbeing +10 this week, relationship with Karolina +5; a swap partner −3 wb unless it suits their prefs (MinNights, WorksWith) |
| decline | mood −6 tonight; for a Need, 25 % they come in late; 3 declines in 2 weeks → a bot note "{name} looks tired" (never a walkout on Cozy) |
| no answer by night start | the bot: "{name}: I'll manage." → mood −3 |

Sim: `Request { kind: Swap | TimeOff | EarlyLeave, reason_key, from_pref }`; `AnswerEarlyLeave { id, approve }`
(alongside the existing `AnswerSwap` / `AnswerTimeOff`); events `RequestCreated`, `RequestAnswered { approved }`.

### 13.4 Bot messages that prompt actions
Each `@shiftella_bot` message carries buttons that *do* something now (`BotAction { msg, action }`, existing):

| template | when | text (en) | buttons → effect |
|---|---|---|---|
| `regular_tonight` (new) | day, a story or likely visit | "Mirek usually comes around 23:00. His seat by the counter is free." | **Reserve his seat** (§6.3) · **Noted** |
| `regular_to_go` (new) | day, Radek nights | "Radek passes at 01:00 — to go, fast." | **Remind me at 00:55** (a rail chip + a soft ping) · **Noted** |
| `late_alert` (existing, extended) | a helper is late | "Bára is 20 min late." | **I'll cover the bar** (her duty tasks go to Karolina's rail until she arrives) · **Ask Oleh to cover** (temporarily moves the duty to an on-shift helper within capacity; −2 mood for him) · **Noted** |
| `sick_call` (existing, extended) | a helper is ill | "Kuba is ill tonight." | **Get well soon** · **Ask Ema to come in** (an Off helper with no Need that night arrives +60 min, wb −8 unless MinNights) · **We'll manage** |
| `train_delay` (existing, extended) | `radek_warns_trains` or a late train | "EN 471 is 30 min late — 6 passengers will wait here." | **Get ready** (spawns `m_delay_board` early, the register helper waits at the register) · **Noted** |
| `low_stock` (new) | stock of an ingredient group < 4 portions | "Oat milk is running low — 3 cups left." | **Order by drone** (needs `drone_dock`; arrives in 30 min, cost) · **Take it off tonight's menu** |
| `weather_alert` (new) | a downpour or storm is coming | "Downpour from 01:00 — more guests, wet floors." | **Put out the umbrella stand** (applies `m_umbrellas` now) · **Noted** |
| `payment_reminder` (existing) | before payments | — | **Open money & payroll** (deep link) |
| `cat_waiting` (new, SHOULD) | the cat moment spawns while Karolina is far away | "Someone with whiskers is at the door." | **Look** (camera to the door) |
| `rozpis_happy` (new) | `RozpisHappy` | "Everyone's happy with this week's rozpis." | **Thanks!** (crew +1 mood) |

New action handlers: `cover_self`, `cover_other(staff)`, `call_in(staff)`, `reserve_seat(regular)`,
`remind_at(minute)`, `order_drone(group)`, `menu_off(item)`, `apply_moment(id)`, `camera(target)`.
None contains "clock" (validation already rejects that).

### 13.5 Board and handover notes (předávka)
* At closing Karolina picks **up to 2 notes** for tomorrow from sim-generated suggestions (one tap each, no
  typing; SHOULD: free text). Each note has a real next-night effect:

| suggestion (when) | next night |
|---|---|
| "Oat milk is low." (stock) | auto-restock prioritises it; `check_stock` comes pre-ticked |
| "The coffee machine sounded odd." (wear ≥ 0.5) | the bar helper cleans it first: wear −0.05 |
| "Mirek seemed tired." (a regular's arc hint) | his order card shows a "remember" mark; +1 trust when served |
| "Busy after the 00:15 train." (a rush happened) | at 00:10 helpers wait at their stations; the rail pre-opens |
| "Great night — thank the crew." (rating ≥ 4) | crew mood +3 |
| "The window table wobbles." (wear on a table) | the floor helper fixes it: beauty +1 |

* At day start the Board tab shows yesterday's note in Karolina's handwriting style, with portraits of
  whom it concerns.
* **Announcements** get real effects: `thank_you {name}` → that person +6 mood; `new_item {item}` →
  helpers prepare it 10 % faster tonight and guests mention it in bubbles; `swap_board` → open swaps are
  more likely to be picked up by colleagues.
* Sim: `HandoverOption { id, when: Vec<Condition>, next_night: Vec<Effect> }` in `shiftella.ron`;
  `Command::WriteHandover { picks: Vec<String> }` (a text field stays optional); event `HandoverApplied { ids }`.

### 13.6 The night digest helps plan
`NightSummary.insights: Vec<Insight { key, args, link: AppLink }>`, computed from night tallies, at most 4
shown, each with a deep link:
* busiest hour → "Next {weekday} will probably be similar." → **Rozpis**
* wishes → "8 wishes, 5 perfect. Vera loved the linden tea." → **Recipe book**
* unmet tastes (a histogram of `not_quite` tags) → "Three guests wanted something Cold — maybe ice and tonic?" → **Pantry**
* coziness average + weakest part → "The hall was Cosy; the window tables were dark." → **Build**
* helpers → "Kuba washed 46 cups, Oleh made 21 coffees." → **Duties**
* tips + best rhythm streak; regulars' trust changes.

### 13.7 Payroll basis that matters
The weekly *podklady pro mzdy* screen (Sunday morning) becomes a small, warm decision screen:
* **Tip split** (remembered): *All to the café* (helps the debt) · *Half to the crew* (default; crew
  mood +4 next week) · *All to the crew* (mood +8, wb +5). It is shown with the real numbers.
* **Star of the week** bonus (optional, 200 Kč): that person gets mood +10 and a napkin-like thank-you card.
* **Náhradní volno owed** rows ("Bára: 6 h owed") → **Grant a night off next week** (a temporary
  DayOff Need-free pref; wb +10).
* **Send to the accountant** books the week (wages are already paid nightly as today; this confirms the tip
  split and bonus). If it isn't sent, it auto-sends at the next day start. Never failable.
* Sim: `SetTipsSplit { share: Cafe | Half | Crew }`, `PayBonus { staff }`, `GrantToil { staff, weekday }`,
  `SendPayroll { week }`; event `PayrollSent { week, total, tips_to_crew }`.

### 13.8 Using the phone at night
Opening the app (T) at night counts as an overlay (clock ×0.1 on Cozy, §1.4). The T button shows a badge
with the number of messages that have actions.

---

## 14. Audio map — no new generated audio (MUST)

Available: SFX `amb_rain_window_loop`, `amb_train_pass_far`, `sfx_cash_register`, `sfx_coffee_machine`,
`sfx_cup_saucer`, `sfx_door_bell`, `sfx_footsteps_tile`, `sfx_power_outage`, `ui_alert_event`, `ui_click`;
music `mus_menu_theme`, `mus_night_calm_loop`, `mus_rush_loop` + the 6 `jukebox_*`. As today, the game
runs without any of them.

| need | file | how |
|---|---|---|
| milk steaming, MulledSpice warming | `sfx_coffee_machine` | looped while held, volume 0.3 → 0.8 with temperature |
| soup simmer | `sfx_coffee_machine` | looped, volume 0.2 |
| pour (PourOver, LatteArt) | `amb_rain_window_loop` | a second instance at 0.25 while pouring: a soft trickle |
| sweet-spot cue, metronome tick | `ui_click` | 0.5 / 0.15, always with a visual pulse |
| tea plop, sugar, lemon, plating, serve, wash | `sfx_cup_saucer` | 0.4–0.7 |
| tip coin, tip jar, payroll sent | `sfx_cash_register` | 0.25 (soft) |
| greeting, arrivals | `sfx_door_bell` | existing |
| moment appears | `ui_alert_event` | 0.3 (softer than events) |
| record needle drop | `ui_click` 0.3 + crossfade | |
| quiet hour | `jukebox_piano_trio` via House | |
| level-ups (coziness, cat, practised, garlands) | `ui_click` 0.5 + glow | |
| blackout, the fogged-window train | `sfx_power_outage`, `amb_train_pass_far` | existing |
| polaroid capture | silent white flash | |

SHOULD: a tiny code-side synth (`audio::synth::chime(freq_hz, ms) -> Vec<u8>` WAV → `load_sound_from_bytes`)
for a softer chime and tick. It is generated by code at runtime, not by ElevenLabs, so it is allowed.
Off by default until it sounds right.

---

## 15. Implementation plan (sequential agent tasks)

The owner is "not in a hurry": quality first. Tasks run in this order; tasks on the same line can run in parallel.

| step | task | agent | depends on | delivers | done when |
|---|---|---|---|---|---|
| T1 | **Content schema + English data** | content | — | `src/content` types for recipes/ingredients, wishes, moments, records, minigame tuning (incl. `MiniGameKind`), seasons, themes, prefs, checklist/handover/bot extensions, tutorial; validation; `MUSIC_TRACKS` += 6; all data files and English i18n | `content_check` clean, `cargo test` green |
| T2 | **Mini-games** | mini-game | T1 (`MiniGameKind`, tuning types) | `src/minigame/**`: the 5 MUST games, frame, keyboard paths, Relaxed, latte shapes, `examples/minigame_*.rs` with shots | each game playable by mouse and keyboard; shots reviewed |
| T3 | **Render additions** | render | T1 | emotes, tip coins, moment markers, queue icons, cup glyphs, candles, bucket, seasonal visuals, theme tints & wall patterns, garlands, snow, `draw_scene_snap` + polaroid frame | `examples/render_*` shots for every phase and theme |
| T4 | **Sim core: Karolina & delegation** | sim | T1 | §1 + §3 + §10 in the sim: PlayerState/acts, OrderKind, match, duties & capacity, wait clock, Cozy forgiveness, tips on tables, rush/rhythm, lull director, music state & likes, save v2, `--player-bot` | headless tests; player-bot meets §9 on nights 1–10 |
| T5 | **Sim systems** | sim | T4 | §4 knowledge & experiments, §5 moment director + moments, §11 coziness/seasons/themes/journal capture, §12 prefs/wellbeing/auto-fill, §13 rituals/requests/bot actions/handover/insights/payroll | headless tests per system; same seed ⇒ same save |
| T6 | **UI widgets** | ui-kit | T1 (runs on mock data, parallel with T4–T5) | order card, rail, moment card, duties board, record picker, recipe book, atmosphere meter, rozpis chips & rings, payroll screen, journal scrapbook, polaroid card, coach marks | `examples/ui_*` shots at 1× and 2× DPI |
| T7 | **Integration 1: the loop** | integration | T2, T4, T6 | input routing (world clicks → acts), overlays + time scale, tick length, hotkeys K/J/Q/H/U/Tab, audio crossfade + §14 map, the order → mini-game → serve flow, tutorial night 1 | night 1 playable end to end, keyboard-only too |
| T8 | **Integration 2: the world** | integration | T3, T5, T7 | moments, book & experiments, jukebox picker, seasons, themes in Build, the meter, the morning polaroid & journal, rozpis UI, app deep links, tutorial nights 2–4 | a 30-night Cozy run with every feature touched |
| T9 | **Content 2: languages & balance** | content | T8 | cs/sk/uk for every new key (feminine forms for Karolina), 40 napkins (SHOULD), caption polish, `handson.ron` tuning from player-bot numbers | i18n check clean; balance targets met |
| T10 | **QA** | qa | T9 | §9 targets; keyboard-only night; Relaxed + skip-all → an ending on Cozy; v1 save → v2; no-audio run; every bot button tested | report with no blockers |

Extra acceptance for the new sections:
* A record change is audible within 3 s; a liked record gives trust at most once per visit.
* The coziness meter moves at least one level when a dirty hall is cleaned or lamps are switched on.
* Every night produces exactly one kept polaroid; the 30 journal entries re-render after load.
* Auto-fill keeps every Need when coverage allows; a happy week is reachable with the story crew.
* Every checklist item, request answer and bot button has a visible effect within the same or next night.

## 16. Integration notes collected by the lead
- Polaroid captions must wrap/shrink inside the frame (one caption overflowed the right border in the T3 demo).
- Render `SnapActor` takes a render `PersonLook`: map the sim's `LookRef` (seed / preset) to it.
- Content must give each recipe a `CupGlyph` family and map `Taste` → render `TasteIcon`.
- T6 (UI widgets) runs on mock data; T7 wires them. Mini-games: disable WASD camera while one is open.
- Polaroid snapshot render target should be larger than 360x270 (looks soft in the enlarged journal view) — T6 note.
- The existing tutorial (src/screens/game/tutorial.rs) can switch to ui::handson::coach_mark.
- (T7) Guest cards show the archetype plural ("Travellers") — content needs singular guest names / "a traveller".
- (T7) Garlic soup (česnečka) plates as "Dill soup" in the Plate mini-game — fix the dish/name mapping.
- (T7) `story.hint.first_order` text is outdated ("Click the guest at the counter…").
- (T7) Duties widget header still shows "P" instead of "H".
- (T7) Event toasts (e.g. Lost drone) stay on screen a long time and stack over the night view — shorter lifetime / collapse.
- (T7) Night 1 at ×1 takes ~15 real minutes with overlays (target 12–14).
