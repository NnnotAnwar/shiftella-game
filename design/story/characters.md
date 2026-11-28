# Characters — Shiftella

> Trait scales and ids are in traits.md. **[MVP]** = needed for the first prototype.
> Skills: kitchen, bar, register, cleaning — 0..10. *(The shipped game adds a fifth skill, `serving`.)*
>
> **Design notes, written before implementation.** The shipped people live in `assets/data/staff.ron`,
> `regulars.ron` and `guests.ron`; the menu in `menu.ron` (see [`docs/CONTENT.md`](../../docs/CONTENT.md) §2).
> Shipped ids are shorter than the draft ones below (e.g. `barista` instead of `arch_student_barista`,
> `mirek` instead of `guest_mirek`). Where the notes and the data differ, the data wins.

---

## 1. Staff — archetypes for hiring

In Story mode these are specific people with names. In Sandbox they are templates: the generator takes an
archetype, a random name and a ±2 spread of skills. Staff wear indigo aprons with a small Shiftella badge.

| draft id (shipped id) | example name | role | skills (kitchen/bar/register/cleaning) | traits | likes | dislikes | story hook |
|---|---|---|---|---|---|---|---|
| `arch_student_barista` (`barista`) **[MVP]** | Bára Nováková, 21 | barista | 2 / 6 / 4 / 2 | `night_owl`, `chatterbox` | music in the hall, chatting with guests | silence, washing up | Dropped out of university in Prague and hasn't told her parents. After about ten nights she may ask for a night off "for an exam". |
| `arch_old_cook` (`cook`) **[MVP]** | Mr Zdeněk Holub, 58 | cook | 7 / 2 / 2 / 3 | `perfectionist`, `grumpy` | an orderly kitchen, his own knife | being rushed, people touching his things | Cooked for twenty years in a factory canteen in Holešovice. Knew Lida and grumbles that "nothing is the way it was in her day". |
| `arch_gentle_giant` (`porter`) **[MVP]** | Kuba Šrámek, 26 | cleaning / helper | 3 / 1 / 2 / 6 | `clumsy`, `kind` | animals (the cat!), food | loud quarrels | A former warehouse porter. Kind-hearted, drops everything. Secretly learning to bake — leave him be and he may become a second cook. |
| `arch_single_mom` (`cashier`) | Ivana Pešková, 34 | register / floor | 2 / 3 / 7 / 3 | `hard_worker`, `anxious` | accuracy, being praised | lateness, chaos | While she works nights, her daughter sleeps at the neighbour's. A call saying "my daughter is ill" can wreck a shift (see the staff-sick event). |
| `arch_sound_guy` (`sound_tech`) | Oleh Melnyk, 30 | bar / floor | 2 / 5 / 3 / 3 | `music_lover`, `calm` | old records, the radio | cheap coffee | A former sound engineer from Lviv, here "temporarily" for two years now. Can repair the old radio (unlocks `note_09`). |
| `arch_quiet_one` (`quiet_one`) | Ema Říhová, 19 | all-rounder | 3 / 3 / 3 / 4 | `light_sleeper`, `loner` | rain, the quiet hours 2–4 | crowds, harsh light | Hears the horns of trains that aren't in the timetable. The only one on the staff who notices the clock stopping at 3:33. |
| — (`server`) | Natália Horváthová, 24 | server (added in the shipped game) | 2 / 3 / 4 / 3, serving 7 | `neat`, `romantic` | clean tables, rainy nights | sticky floors | Came from Košice for a week and stayed. Carries four plates at once and remembers every name. |

**Hiring in Story mode:** on night 2 three candidates are offered from the three MVP archetypes. The rest open up from night 8.

---

## 2. Menu (orders refer to it)

*(historical)* The MVP draft menu. The shipped menu has 16 items (`assets/data/menu.ron`), including an oat
flat white, chai, varenyky, bryndzové pirohy, medovník and a "night train" special; grandma's recipe became
`grandma_kulajda`.

| id | dish | station | time, min | price, Kč | MVP |
|---|---|---|---|---|---|
| `coffee_black` | black coffee | bar | 2 | 45 | yes |
| `cappuccino` | cappuccino | bar | 3 | 65 | yes |
| `tea_lemon` | tea with lemon | bar | 2 | 40 | yes |
| `hot_choco` | hot chocolate | bar | 3 | 60 | later |
| `topinka` | topinka (fried bread with garlic) | kitchen | 4 | 55 | yes |
| `cesnecka` | garlic soup | kitchen | 5 | 75 | yes |
| `chlebicek` | chlebíček (open sandwich) | kitchen | 3 | 40 | later |
| `kolac` | koláč (round pastry) | kitchen (ready-made) | 1 | 35 | later |
| `lida_soup` | "Lida's soup" (recipe from `note_05`) | kitchen, skill ≥ 5 | 6 | 95 | later |
| `svarak` | mulled wine (needs a "licence" upgrade) | bar | 3 | 70 | later |

---

## 3. Regulars — arcs

Format: **who** · **when** (arrival window) · **order** · **arc stages** (condition → what happens) · **what the player can do** · **reward**.
An arc moves one stage per night when its condition holds. A good arc ending counts towards the endings.

### 3.1 Mirek — the old taxi driver **[MVP: as a guest only, no arc]**
- `mirek`, Miroslav Dvořák, 71. Every night, 23:00–00:30, sometimes again at 04:00.
- **Order:** black coffee + topinka. High patience. Pays exactly, no tip.
- **Hides:** (1) his eyesight is failing and he really shouldn't drive any more; (2) he has Lida's letter (story.md §4).
- **Stages:**
  | stage | condition | event |
  |---|---|---|
  | 0 | — | an ordinary guest who tells you how things were "in Lida's day" |
  | 1 | served quickly (≤ 5 min) three nights in a row | asks for his coffee "the way Lida poured it"; the first hint of the secret |
  | 2 | night 8 (scripted) | clips the kerb, squints at the menu → the player can offer a large-print menu (upgrade, +trust) |
  | 3 | night 20 (scripted) | nearly hits a pedestrian (the nurse is Hana, once you know her). **Choice:** take his keys / call his daughter / say nothing |
  | 4 | nights 25–29 (the storm night at the earliest), trust ≥ 60 (keys or daughter) — or, after "say nothing", from night 26 with trust ≥ 40 | confesses about the letter → `letter = confessed` |
  | 5 | nights 28–30 | **The letter choice** (story.md §7) |
- **Good ending:** Mirek stops driving (his daughter drives him, or he walks) and delivers the letter himself. **Say nothing:** on night 25 there is an accident (nobody hurt), Jana takes his keys (trust −20), he walks in and confesses later — but his arc doesn't end well.

### 3.2 Hana — the night nurse
- `hana`, Hana Janoušková, 38, district hospital. Comes on her break 02:00–02:30 and after her shift 05:40–06:00.
- **Order:** on her break — cappuccino (quickly! low patience — 6 min); after her shift — garlic soup.
- **Arc — burnout:**
  | stage | condition | event |
  |---|---|---|
  | 0 | — | silent, staring at her phone |
  | 1 | served within 4 min, three times | smiles, tells you about a hard night |
  | 2 | night 12 (scripted): falls asleep at a table | **Choice:** wake her (+) / cover her with a blanket (+) / a "quiet corner" (furniture: armchair + lamp; ++) → `note_04`; on night 26 she stays with the blanket, the corner or trust ≥ 50 |
  | 3 | there is a quiet corner or a blanket | says she has been offered a job in Dresden |
  | 4 | night 26 | the decision. If stages 2–3 went well she stays ("people wait for me here"); otherwise she leaves, but says a warm goodbye |
- **What the player can do:** put her order first, give her food for free; the hospital shift-change event brings her colleagues.
- **Reward:** she stays → +5 medical guests in the pool; she leaves → a postcard, reputation +5.

### 3.3 The 3:33 guest — "the gentleman in the grey coat" **[MVP: appearance + the clock, no arc]**
- `guest_333`. Never gives his name. Comes **at exactly 03:33**, leaves at 03:50. Doesn't come if the window light is off or the franchise was accepted.
- **Order:** tea with lemon, "two sugars". Always the same table by the window (if it was removed, he stands by the wall and waits).
- **Pays** with old Czechoslovak coins (item `old_coin`, 0 Kč). You can choose not to charge him — a hidden plus.
- **Special rules:** at 3:33 the clock stops for 60 seconds. Staff with `light_sleeper` notice the guest and get mood +5 (not fear — "a quiet wonder"). Everyone else sees "someone by the window".
- **Stages:**
  | stage | condition | event |
  |---|---|---|
  | 0 | night 4 | first appearance |
  | 1 | his table kept free three nights in a row | the coins add up → `note_06` |
  | 2 | night 15 | "Is she still waiting?" → the player's answer is stored in `answer_333` |
  | 3 | 4 or more notes found | brings an Rn 333 ticket from 1979, lays it on the table and leaves it |
  | 4 | night 30, 3:33 | the letter scene → the optional "Answer" scene |

### 3.4 Mrs Vera — insomnia
- `vera`, Vera Soukupova, 74, a widow. 00:30–03:00, stays a long time doing crosswords.
- **Order:** tea with lemon, then a koláč. Medium patience, but **strict**: a dirty table makes her review count double.
- **Arc — reconciliation:** Lida's friend and rival since school; she knows the story of Jindra.
  | stage | condition | event |
  |---|---|---|
  | 0 | night 5 | "Lida wouldn't have done it like that" |
  | 1 | hall cleanliness ≥ 80 % on three of her visits | helps the staff with the crossword (a joke) |
  | 2 | night 14, relationship ≥ 30 | brings the recipe book → `note_05`, grandma's recipe |
  | 3 | you served her grandma's recipe | cries and tells you about young Lida |
  | 4 | nights 19–24, 4 or more notes found (later without the notes) | tells you about Jindra and Rn 333 — before Mirek's confession |
- **Reward:** the recipe, a key to the mystery, and in the finale "Vera brought her friends" (+3 guests per night).

### 3.5 Filip — the student
- `filip`, Filip Kadlec, 22, technical college. 23:30–05:00, sits by the power socket with his laptop.
- **Order:** black coffee every hour, pays little. Wants a socket + Wi-Fi (upgrade) — without them his mood drops.
- **Arc — drop out or finish:**
  - 0: "takes a table all night" — a dilemma for the player (a table versus a loyal guest).
  - 1: there is Wi-Fi → he starts bringing friends (+2 students in the pool).
  - 2: night 18, he has failed an exam. **Choice:** talk to him (a staff member with `kind` or `chatterbox`) / free coffee / stay out of it.
  - 3: if supported — he passes the resit; from night 26 he brings a cake "for not throwing me out".
- **Reward:** students as regulars; the downside: kick him off his table three times → a bad online review (reputation −5).

### 3.6 Radek — the night dispatcher
- `radek`, Radek Macek, 52. Comes twice a night "for five minutes": 01:00 and 04:30.
- **Order:** black coffee **to go** — it has to be very fast, patience 3 min.
- **Arc — the last shift:** the station is being automated.
  - 0: night 9 — tells you the news, mentions Rn 333.
  - 1: never waited longer than 3 min over five visits → "I'll let you know if a train's late" (the late-train event now comes with a 30-minute warning).
  - 2: night 23 — his last shift; he gives you the timetable → `note_08`.
  - 3: if the relationship is high — after night 23 he becomes an ordinary "retired" guest (stays long, tells stories = staff mood +).

### 3.7 Lori — the street musician
- `lori`, Loreta "Lori" Benešová, 27. Plays on the platform in the rain. Comes only on rainy nights, 22:30–01:00.
- **Order:** hot chocolate — she has no money and offers "I'll play for a chocolate".
- **Arc — the stage:**
  - 0: night 11 — plays outside. **Choice:** invite her in / don't.
  - 1: invited → plays by the window: guests' mood +5, but the queue moves slower (the hall is louder).
  - 2: you bought a "small stage" (2×1 furniture) → the open-mic event becomes available.
  - 3: night 24 — her evening: a full house, reputation +10. In the finale she stays on as "the Friday musician".
- **Links:** staff with `music_lover` get mood +10 while she plays.

### 3.8 Anwar — a quiet cameo (shipped)
- `guest_anwar`, "Anwar", 28. A quiet young man in an indigo hoodie, working on a laptop with headphones. His identity is deliberately understated.
- **Visits:** exactly two per game — **nights 9 and 24** (in Story and Sandbox), 23:00–02:00, at the socket table.
- **Order:** an oat flat white. Pays, leaves.
- **Talking:** hovering shows one of 8 ambient lines (rain, fog, a passing train, midnight, late, busy, quiet, coffee). The **Talk** action plays `anwar_talk_9` / `anwar_talk_24`: he is making a small game about a night café.
- **Flavour only:** no arc, choices, flags or effects, and no part in the endings. The credits thank him (`ui.ending.credits_thanks`).

### 3.8b Kseniia — a late cameo (shipped)
- `guest_kseniia`, "Kseniia" (uk Ксенія, ru Ксения), 34. A real person referenced with her consent (name and look only). Hair loosely up, an oversized blazer with paint stains, dark lipstick; a bridge piercing between the brows, a ring low on the nose, a labret, one small ring in one ear (render preset `kseniia`, `Accessory::Piercings`, `TopStyle::Blazer`).
- **Who:** manages an art studio in an old factory in Holešovice; comes in after half past two, after hanging an exhibition overnight or seeing off the last artist. Used to work at an Italian restaurant with her ex.
- **Voice:** teasing, double meanings, never explicit, no swearing; keeps bringing up her ex, who is never named (he mixed up burrata and mozzarella; he now "writes some app for night shifts"). Underneath: warm, the most observant guest in the room.
- **Visits:** **nights 3, 15 and 20** (Story and Sandbox), 02:40–04:10 — she is gone before Karolina's back-step call (night 3, 04:20). She and Anwar (9, 24) never meet.
- **Order:** an oat flat white, "a silly habit from my ex" (Anwar orders the same).
- **Talks:** `kseniia_talk_3` — she meets Karolina and is the first to encourage her ("Night shift is when every guest is a bit more honest."). `kseniia_talk_15` — "Hi. How are you?" from him after two years; **choice** "Answer him" / "Don't answer" → var `kseniia_ex` = `answered` / `silent`. `kseniia_talk_20` — answered: coffee, "He was two minutes late. Again."; silent: she deletes his number ("For him I switched it off."); the night-15 talk never opened (var unset): a neutral variant.
- **Easter egg:** if `kseniia_ex == answered`, Anwar's night-24 talk gains "Someone recommended this place to me. Well… not just someone." (and the same ambient line). No names.
- **Flavour only:** no arc, flags, trust or effects beyond the var; no part in the endings. Journal highlight `KseniiaCameo` ("Special guest"). Texts: `assets/i18n/src/cameo_kseniia.tr`.

### 3.9 Karolina — the heroine (the player)
- Lida's granddaughter, about thirty. Last here as a child: Mum (Lida's daughter) and Grandma talked twice a year, "both too proud to call first".
- **Before the café:** three years as a **night operator for NONSTOP 24** in Brno — forty staffless shops on forty screens (doors, fridges, lights), "and not one of them waiting for anyone". She quit in the spring, the week she caught herself saying good night to a fridge.
- **Her arc (K1–K4)**, in cutscenes, no stats:
  | # | scene | when | what happens |
  |---|---|---|---|
  | K1 | `karolina_backstep` | night 3, 04:20 | Mum calls on the back step: Brno, the forty shops, "Grandma would say you came home" |
  | K2 | `karolina_not_lida` | night 13, 01:20 (Vera in the café) | Vera's "Lida's best was better" once too often. **Choice:** Lida's way / my way → var `karolina_way` = `lida` / `own`. Cosmetic: the chalkboard by the counter in Lida's slanted hand or Karolina's own capitals, and her last line in both endings. Vera either way: "Good. Now you're arguing like her." |
  | K3 | inside `franchise_visit` | night 17 | Šindelář recognises her from the Brno hub; the offer becomes "come back to us" |
  | K4 | `karolina_storm` | with Mirek's confession (the storm night 25; later after "Say nothing") | she tells Mirek the truth about herself — "Forty shops lit all night, and nobody inside. Here — one lamp, and everyone came." — "Then I owe you a truth too." → `mirek_confession` |
- **The last word is hers:** night 30 at 05:30, after 3:33 and the letter — sign, or keep the light (story.md §8).
- **Payoff:** Vera's crossword, "Eight letters, 'one who waits'… K-A-R-O-L-I-N-A. It fits." (Karolina / Кароліна / Каролина — eight letters in every language.)

### 3.10 The other side — Mr Šindelář and Mr Kubíček (no villains)
- **Petr Šindelář, NONSTOP 24.** Polite, funny, sure he is right. His mother had a bistro in Cheb; it closed on a Tuesday and nobody noticed until Friday — "That's why I sell lights that never go out. I thought it was the same thing." Scenes: `franchise_visit` (17, knows Karolina from Brno), `franchise_decision` (22: "Sign now" / "Not yet"), `sindelar_storm` (25, 02:40: his driverless car lost the grid at the bridge; "All forty-one of our stores went dark at 22:31… Yours didn't."), `franchise_again` (Classic, 28, money short), `franchise_last` (30, 05:30). If she signs, he takes the lamp to his office — "Don't tell head office."
- **Mr Kubíček, the landlord.** Dry, fair, keeps count. People write to him about the café: one message about the soup (night 14), eleven in a folder called "Not mould" (21), twenty-six and a poem (28), forty-two at the end. As a boy he did his sums at the corner table while his mother worked nights at the depot — Lida checked them. "That's where I learned arithmetic."

---

## 4. Odd guests (a rare pool, Story + Sandbox) **[later]**

| id | who | what they want | special |
|---|---|---|---|
| `odd_postman` | a postman with no letters | "is there anything for me?" | answer "no" three nights running and he leaves an empty envelope (item, +mood) |
| `odd_birdcage` | a woman with an empty birdcage | warm milk | open the window and she leaves happy, reputation +2 |
| `odd_compass` | a traveller whose compass always points at the café | a map of the town | gives you a "town legend" (a Sandbox collection) |
| `odd_twin` | a guest who looks a lot like one of the staff | whatever that staff member likes | the staff member gets mood + or − depending on their traits |
| `odd_moth` | a quiet person who only sits under a lamp | nothing, just to sit | if the lamp stays on all night, by morning there's a 200 Kč tip on the table |
