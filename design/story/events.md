# Events — the Night storyteller

> The Night decides for itself what happens. Below are 35 events (some random, some "auto" — triggered by the game state) and the pacing rules.
> How to write effects — traits.md §1. Story events by night number are in story.md.
> Mode: **S** — Story, **P** — Sandbox ("playground"), **S+P** — both. **MVP: yes** — 10 events for the first prototype (3 of them "auto").
>
> **Design notes, written before implementation.** The shipped game has 41 events in
> `assets/data/events.ron` (texts in `assets/i18n/src/events.tr`), with the `ev_` prefix dropped from the
> ids and a few 2039 additions such as a lost delivery drone, a flickering holo sign and a drone-depot
> shift change. The shipped event and narrator format is in [`docs/CONTENT.md`](../../docs/CONTENT.md) §4.
> Where they differ, the data wins.

## 0. The format of one event

| field | meaning |
|---|---|
| `id` | unique key |
| `kind` | `good` / `bad` / `neutral` / `uncanny` — the storyteller needs it for balance |
| `severity` | 1–3: how hard it hits (its cost in the "tension budget") |
| `when` | conditions: time, weather, night number, traits, state |
| `effects` | list of effects + duration |
| `weight` | base weight in the random pick (bigger = more often) |
| `cooldown` | how many nights before it can repeat |

```ron
Event(
    id: "ev_downpour",
    kind: Neutral, severity: 2, weight: 30, cooldown: 2,
    when: [Weather(Rain), TimeBetween("22:30", "03:00")],
    effects: [
        Mul(stat: "guests.rate", value: 2.0, minutes: 120),
        Mul(stat: "dirt.rate", value: 2.0, minutes: 120),
    ],
    // The text is an i18n key in the shipped game: event.downpour.title / .text
)
```

---

## 1. Weather and the town

| id | name | text | conditions | effects | weight | mode | MVP |
|---|---|---|---|---|---|---|---|
| `ev_rain` | Rain | The glass is beaded with drops, the holo signs blur. People come in to wait it out. | weather = rain (start of the night) | `mul(guests.rate, 1.2)` all night; `add(dirt, +)` by the entrance | 50 | S+P | **yes** |
| `ev_downpour` | Downpour | The sky has burst. Half the station is hiding at your place. | `ev_rain` is on, 22:30–03:00 | `mul(guests.rate, 2.0)` 120 min; `mul(dirt.rate, 2.0)` 120 min | 30 | S+P | **yes** |
| `ev_fog` | Fog from the river | Milk rises from the Vltava. You can see the café window — and nothing else. | autumn/winter, after 01:00 | `mul(guests.rate, 0.6)`; odd-guest chance ×3 until morning | 20 | S+P | no |
| `ev_storm` | Storm | The station lamps go out. Only your window is lit. | summer/autumn, `kind=bad` allowed | `mul(guests.rate, 1.5)`, guests stay longer (+30 min); 30 % chance of `ev_power_outage` | 10 | S+P | no |
| `ev_first_snow` | First snow | Silence. Guests whisper, as if afraid to scare it away. | once per season, night ≥ 20 (Story: night 27) | everyone `add(need.mood, 10)`; guests `add(satisfaction, 10)` | 5 | S+P | no |
| `ev_roof_leak` | Leaking roof | Drip. Drip. Straight into the bucket by the counter. | rain + the "roof" upgrade not bought | 1 hall tile blocked; `add(dirt, +)` every 30 min until cleaned | 15 | S+P | no |

## 2. Rushes of guests

| id | name | text | conditions | effects | weight | mode | MVP |
|---|---|---|---|---|---|---|---|
| `ev_concert_crowd` | Crowd after a concert | The concert at the culture house is over. Twelve wet jackets — and they're all hungry. | Friday/Saturday, 23:00–00:30 | a group of 8–12 guests `pool=young` within 10 min; patience −30 % | 25 | S+P | **yes** |
| `ev_late_train` | Late train | The express is stuck for 2 hours. Angry passengers with suitcases on the platform. | 00:00–03:00 | a group of 5–8 `pool=travelers`; they order food; if Radek's arc is at stage ≥ 1 — a 30-minute warning | 20 | S+P | no |
| `ev_taxi_break` | Taxi drivers' break | Cars lined up by the door. Coffee, quick, and off they go again. | 03:00–04:00, there's a taxi rank | 4–6 guests, black coffee only, patience 3 min; tips ×2 | 20 | S+P | no |
| `ev_hospital_shift` | Hospital shift change | Scrubs under the jackets. They'd love some soup and five minutes of quiet. | 05:30–06:00, there's a hospital | 3–5 guests `pool=medics`, garlic soup; if Hana's arc ≥ 3, +2 guests | 20 | S+P | no |
| `ev_student_party` | Students after a party | Loud, cheerful and very sticky. | Thu–Sat, 01:00–03:00, there's a college | 6–10 guests `pool=students`; `mul(dirt.rate, 3)` 60 min; `ev_dine_and_dash` chance ×2 | 15 | S+P | no |
| `ev_lost_bus` | Lost bus | A tour bus took a wrong turn. The guide asks whether you have 30 portions of soup. | night ≥ 10, rare | 15 guests within 20 min; revenue ×1.5 on their orders; reputation ±5 depending on the result | 3 | P | no |
| `ev_open_mic` | Open-mic night | Lori tunes her guitar and the hall goes quiet. | the "small stage" furniture + Lori's arc ≥ 2 (P: any musician from the pool); Fri/Sat | `mul(guests.rate, 1.6)` 120 min; guests `add(satisfaction, 10)`; `music_lover` `add(need.mood, 10)`; noise: `mul(register.speed, 0.9)` | 10 | S+P | no |

## 3. Staff

| id | name | text | conditions | effects | weight | mode | MVP |
|---|---|---|---|---|---|---|---|
| `ev_staff_sick` | Someone's ill | A call at 21:40: "Sorry, I can't make it." | start of the night; weight ×2 for `anxious` / the cashier archetype | 1 random person on the shift is out for the night | 15 | S+P | **yes** |
| `ev_staff_walkout` | Walked out | The apron on its hook, the door slammed. | a staff member's `need.mood < 15` | they leave for the rest of the night; everyone else `add(need.mood, −5)` | auto (not random) | S+P | **yes** |
| `ev_broken_dishes` | Broken dishes | Clink. Everyone looks at the kitchen. | triggered by the `clumsy` trait (6 % per action) or `anxious` with a queue ≥ 4 | −30 Kč; `add(dirt, +)`; the order is made again | auto | S+P | **yes** |
| `ev_staff_quarrel` | Quarrel in the kitchen | "That's my knife!" — "And this is my shift!" | `relationship(a,b) < −30`, both on shift | both `add(need.mood, −15)`; `mul(work.speed, 0.8)` 60 min | 15 | S+P | no |
| `ev_staff_romance` | Two people in love | They laugh at the same joke longer than anyone else. | `relationship(a,b) > 80` (60 for `romantic`) | together: `mul(work.speed, 1.1)`; on different shifts — `add(need.mood, −10)` for both; later — a chance of a break-up | 10 | S+P | no |
| `ev_staff_inspired` | Inspiration | "Listen, what if we put … in the topinka?" | `need.mood > 80`, skill ≥ 6 | `add(skill.<best>, 1)` for 3 nights; 20 % chance of a new menu item | 10 | S+P | no |
| `ev_staff_birthday` | Birthday | Someone brought a koláč with a candle in it. | once a year per staff member (Sandbox); Story — a random person in act II | everyone `add(need.mood, 15)`; if the player bought a cake (−150 Kč) — `rel(all, +5)` | auto | S+P | no |

## 4. Equipment and business

| id | name | text | conditions | effects | weight | mode | MVP |
|---|---|---|---|---|---|---|---|
| `ev_power_outage` | Lights out | A click — and darkness. Only the station's holo signs through the window. | any night; storm ×3; in Story — night 10 | 30–60 min: the coffee machine and stove don't work; only cold items sell; candles in stock → guests `add(satisfaction, 5)` ("cozy"); `calm` staff — no penalty | 12 | S+P | **yes** |
| `ev_coffee_machine` | Moody coffee machine | It hisses as if it took offence. | machine wear > 50 % | `mul(bar.speed, 0.5)` until repaired; repair — a staff member with bar ≥ 5 for 20 min, or −400 Kč during the day | 18 | S+P | **yes** |
| `ev_fridge_fail` | The fridge hums | It hums louder than the radio. Then it goes silent. | wear > 60 %, summer ×2 | −30 % food stock; garlic soup unavailable until restocked | 8 | S+P | no |
| `ev_supplier_late` | Late supplier | The milk will come. Eventually. | start of the night | cappuccino and hot chocolate unavailable until 02:00 | 12 | S+P | no |
| `ev_health_inspection` | Health inspection | Strict glasses, a tablet. "Don't mind me." | night ≥ 5; Story — night 13 | a 60-min inspection: dirt < 30 % → reputation +5; 30–60 % → a warning; > 60 % → a 2 000 Kč fine, reputation −10 | 6 | S+P | no |
| `ev_night_blogger` | Night blogger | Films the cup from three sides before tasting it. | reputation ≥ 30 | 1 guest; their review counts ×3 (for better or worse) | 8 | S+P | no |
| `ev_dine_and_dash` | Left without paying | An empty table and half a bowl of soup. | queue at the register > 3 | −the order's price; if the cashier has register ≥ 6 — a 50 % chance to catch them | 10 | S+P | no |
| `ev_big_tip` | A generous stranger | Under the cup lies a banknote and a napkin: "Thank you for being open." | `kind=good`, any hour | +300…800 Kč; everyone `add(need.mood, 5)` | 12 | S+P | **yes** |
| `ev_police_patrol` | Night patrol | Two police officers, two coffees, one "all quiet?". | 01:00–04:00 | 2 guests, quick; for 2 nights `mul(ev_dine_and_dash.weight, 0)`; reputation +1 | 12 | S+P | no |

## 5. The uncanny (a quiet wonder, not fear)

| id | name | text | conditions | effects | weight | mode | MVP |
|---|---|---|---|---|---|---|---|
| `ev_clock_333` | The clock stops | 3:33. The second hand freezes, and the radio hisses for a minute. | every night at 03:33 while the light is on | 60 s pause of animations (guests and staff "freeze", except `light_sleeper`); spawns `guest_333` in Story | auto | S (in P — rare, weight 5) | **yes** (visual only) |
| `ev_phantom_horn` | A horn with no train | A long horn. The board is empty. | 02:00–04:00, fog ×2 | `light_sleeper`: `add(need.mood, 5)`; `anxious`: `add(need.mood, −5)`; everyone else shrugs | 10 | S+P | no |
| `ev_station_cat` | The station cat | Ginger, cheeky, with a torn ear. Eyeing the sour cream. | autumn/winter | feed it three nights in a row (−10 Kč/night) → the cat stays: everyone `add(need.mood, 3)`/night, ×2 for `kind` / the porter | 10 | S+P | no |
| `ev_forgotten_item` | A forgotten item | On a chair — an old umbrella with a monogram. | after a guest leaves | the item goes to "lost and found"; if the owner comes back the next night and it's intact — reputation +3 | 12 | S+P | no |
| `ev_strange_order` | A strange order | "I'll have what Lida used to drink." | Story: night ≥ 8; Sandbox: "what the previous owner drank" | the guest waits for grandma's recipe or a hint from the notes; guess right — reputation +5 and a note found (if not yet found) | 6 | S+P | no |
| `ev_lonely_night` | An empty night | Nobody. Just the rain, the hum of the fridge and the radio. | after 2 hard nights in a row (the storyteller's safety valve) | `mul(guests.rate, 0.3)`; staff `rel(all, +3)/hour`; handy for a deep clean | auto | S+P | no |

---

## 6. How the Night tells the story (pacing)

### 6.1 The tension budget
- The Night has a **`tension`** value (0–100). Each event costs `severity × 10`. `kind=good` has a negative cost (it lowers tension).
- Every game minute the storyteller rolls a die: `chance = base_rate × curve(time) × act_mult(night)`.
- On a hit, it picks an event by weight from those whose `when` holds, that aren't on `cooldown`, and whose cost fits into `max_tension − tension`.
- After a bad event comes a **breather**: `breath_min` minutes with no new `bad` events.

### 6.2 The curve of one night (22:00–06:00)

| hours | phase | what we turn up |
|---|---|---|
| 22:00–00:00 | **Warm-up** | few guests → more; light events (`severity ≤ 1`), weather |
| 00:00–02:00 | **Peak** | crowds, concerts, breakdowns; the highest `curve` |
| 02:00–04:00 | **Witching hour** | few guests; `uncanny` ×3; tired staff → mistakes ×1.5; at exactly 03:33 — `ev_clock_333` |
| 04:00–05:30 | **Wind-down** | taxi drivers, patrols; almost no `bad` |
| 05:30–06:00 | **Dawn** | the hospital shift change; only `good`/`neutral`; the night's summary |

```
curve:  22 ▁▂▃ 00 ▅▇█ 02 ▄▃▃ 04 ▂▂▁ 05:30 ▃ 06
```

### 6.3 The curve of 30 nights (Story)

| nights | `act_mult` | `max_severity` | idea |
|---|---|---|---|
| 1–3 | 0.3 | 1 | tutorial: only rain, big tip, broken dishes |
| 4–7 | 0.6 | 2 | the first problems, gentler just before payment no. 1 |
| 8–16 | 0.9 | 2 | a steady load + the arcs |
| 17–24 | 1.1 | 3 | pressure: the franchise, hard nights |
| 25–29 | 1.2 | 3 | the storm and the resolutions, but at least one "warm" event per night |
| 30 | 0.5 | 1 | the final night is about people, not breakdowns |

**The payment rule:** the night before a payment, `bad` events with `severity 3` are not allowed. Never punish the player for bad luck right before a deadline.

### 6.4 The night's character: calm / chaotic (Sandbox; Story = "calm+")

| parameter | calm | chaotic |
|---|---|---|
| `base_rate` (per min) | 0.004 | 0.009 |
| `max_tension` | 50 | 100 |
| share of `good` | 40 % | 20 % |
| `breath_min` after a `bad` | 60 | 15 |
| max `bad` in a row | 1 | 3 |
| `uncanny` multiplier | ×1 | ×1.5 |
| `ev_lonely_night` valve | after 2 hard nights | off |
| help when broke (money < 1 day's rent) | `good` ×2 | none |
| staff events per night, max | 1 | no limit |

The shipped game has five night characters — quiet, calm (default), changeable, stormy, chaotic — as slider steps (`assets/data/sandbox.ron`).

**For the MVP:** only `base_rate`, `curve(time)`, weights and `cooldown`. The `tension` budget and the night's character came later.
