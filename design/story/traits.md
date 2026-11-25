# Character traits — Shiftella

> Traits are modifiers on top of the base rules. Every staff member has 2 traits (maybe up to 3 later). **[MVP]** = 4 traits in the prototype.
>
> **Design notes, written before implementation.** The shipped traits are in `assets/data/traits.ron`
> (names and descriptions in `assets/i18n/src/traits.tr`); the effect format is documented in
> [`docs/CONTENT.md`](../../docs/CONTENT.md) §2. Where they differ, the data wins.

---

## 1. Base scales (shared by all the files)

| scale | range | start | base per game hour | note |
|---|---|---|---|---|
| `need.energy` | 0–100 | 90 | −8 at work, −3 on a break, +25 sleeping in the staff room | < 20 → work ×0.6, < 5 → falls asleep |
| `need.hunger` | 0–100 (100 = full) | 80 | −6 | < 25 → mood −1/hour; eats on their own during a break |
| `need.mood` | 0–100 | 60 | depends on events | < 15 → may walk out of the shift (staff walkout event) |
| `skill.*` | 0–10 | archetype | +0.05 per completed action | speed = 0.5 + skill × 0.1 |
| `relationship(a,b)` | −100…+100 | 0 | +1 per hour working side by side, ± from traits | > 60 friendship, > 80 + `romantic` → staff romance; < −30 → staff quarrel |
| `reputation` | 0–100 | 25 | ± reviews | affects the flow of guests |

**How to write an effect** (the same for traits and events):
- `mul(<stat>, x)` — a multiplier: `mul(need.energy.drain, 0.8)`
- `add(<stat>, n)` — an addition: `add(skill.bar, 2)`
- `when(<condition>)` — when it applies: `when(time >= 02:00)`, `when(near(guest))`
- `rel(<trait or all>, n)` — relationship change per hour of working side by side

---

## 2. Traits (16)

**[later]** spare ideas: `coffee_addict` "Coffee Addict" (drinks coffee from the storeroom → +energy), `sweet_tooth` "Sweet Tooth" (eats koláče → +mood).

| id | name | description | effect | conflicts with |
|---|---|---|---|---|
| `night_owl` **[MVP]** | Night Owl | Comes alive at night, sleeps like a log by day. | `mul(need.energy.drain, 0.7) when(time >= 00:00)`; `add(need.mood, 5)` at shift start | `early_bird` |
| `early_bird` | Early Bird | Great until midnight, in a fog after that. | `mul(need.energy.drain, 0.8) when(time < 00:00)`; `mul(need.energy.drain, 1.4) when(time >= 02:00)` | `night_owl` |
| `chatterbox` **[MVP]** | Chatterbox | Knows everything about every guest. | `add(guest.satisfaction, 10) when(serving)`; `mul(work.speed, 0.85) when(guests_waiting <= 2)`; `rel(all, +1)`, `rel(loner, −2)` | `loner` |
| `loner` | Loner | Works best when nobody is in the way. | `mul(work.speed, 1.2) when(alone_in_room)`; `add(need.mood, −1)/hour when(staff_nearby >= 2)` | `chatterbox` |
| `clumsy` **[MVP]** | Clumsy | Hands do something other than intended. | 6 % chance per action: broken dishes (−30 Kč, +dirt); `add(skill.kitchen, −1)` | `perfectionist` (annoys them: `rel −2/hour`) |
| `perfectionist` **[MVP]** | Perfectionist | Slow, but perfect. | `mul(work.speed, 0.8)`; `add(order.quality, 25)` (+tips); `add(need.mood, −10)` if dirt in their area > 30 % | `lazy` |
| `lazy` | Lazy | A break is sacred. | `mul(work.speed, 0.85)`; breaks +50 % longer; `mul(need.energy.drain, 0.8)` | `hard_worker`, `perfectionist` |
| `hard_worker` | Hard Worker | Won't leave until it's done. | `mul(work.speed, 1.15)`; skips the break when the queue is > 3 → `mul(need.energy.drain, 1.2)` | `lazy` |
| `grumpy` | Grumpy | Grumbles, but knows the job. | `add(need.mood, −10)` to the base; `rel(all, −1)`; towards high-patience guests `add(guest.satisfaction, 5)` ("a real character") | `kind` (weak) |
| `kind` | Kind | Comforts guests and colleagues alike. | Colleagues nearby get `add(need.mood, 1)/hour`; can "have a word" with a sad guest (+15 satisfaction) | — |
| `anxious` | Anxious | Afraid of making mistakes — and makes them under pressure. | `mul(error.chance, 2) when(queue >= 4)`; `add(skill.register, 1)`; on a bad event `add(need.mood, −10)` | `calm` |
| `calm` | Calm | Power cut? We'll light candles. | Bad events cost them only half the mood penalty; `add(need.mood.min, 20)` | `anxious` |
| `music_lover` | Music Lover | Works to the rhythm. | `add(need.mood, 10) when(music_on)`; `add(need.mood, −5) when(!music_on)`; puts music on by themselves | — |
| `light_sleeper` | Light Sleeper | Hears what others don't. | Sees odd guests and the 3:33 guest (`add(need.mood, 5)` on meeting them); 30 % chance to sense an event early (a hint 15 min ahead); `mul(need.energy.drain, 1.1)` | — |
| `neat` | Neat | Can't walk past a stain. | `mul(skill.cleaning.speed, 1.3)`; cleans between orders unasked; `add(need.mood, −5) when(dirt > 40%)` | `clumsy` (weak) |
| `romantic` | Romantic | Falls for colleagues, and for the rain. | Lower staff-romance threshold: 60 instead of 80; on a rainy night `add(need.mood, 5)`; when a romance ends `add(need.mood, −25)` for 2 nights | — |

**Combination rules:**
- Traits in the "conflicts with" column **never roll together** on one person.
- When traits clash **between different people** (e.g. `clumsy` and `perfectionist` on the same shift), it causes `rel −`. That turns the rozpis into a puzzle about people.
- Generator (Sandbox): 1 "positive" + 1 "mixed" trait for 70 % of staff, 2 positive for 15 %, 2 mixed for 15 %.

| group | traits |
|---|---|
| mostly positive | `night_owl`, `hard_worker`, `kind`, `calm`, `neat`, `light_sleeper` |
| mixed | `chatterbox`, `loner`, `perfectionist`, `music_lover`, `romantic`, `early_bird` |
| mostly negative | `clumsy`, `lazy`, `grumpy`, `anxious` |

---

## 3. Example in RON *(historical — how the data might look)*

The shipped format differs (`TraitDef` with `group`, `conflicts`, `irritated_by` and typed `TraitEffect`s, and
no names in the data — those are i18n keys). See `assets/data/traits.ron`.

```ron
Trait(
    id: "night_owl",
    conflicts: ["early_bird"],
    effects: [
        Mul(stat: "need.energy.drain", value: 0.7, when: Some(TimeAfter("00:00"))),
        Add(stat: "need.mood", value: 5.0, when: Some(ShiftStart)),
    ],
)
```

**For the MVP:** implement only `Mul` and `Add`, with a time condition or none. That is enough for `night_owl`, `chatterbox` (without relationships), `clumsy`, `perfectionist`.
