# Sandbox — Shiftella

> Endless nights in a random little town. There's no story here — there's **your** story, made by the people, their traits and the Night storyteller.
> All of the sandbox was **[later]** in the plan. The MVP was one endless night in a fixed town (Mlhov) with no generation.
>
> **Design notes, written before implementation.** What shipped is in `assets/data/sandbox.ron` and
> `difficulty.ron` (see [`docs/CONTENT.md`](../../docs/CONTENT.md) §7): a seeded random town with a name
> built from parts (minus a stoplist), institutions that enable guest pools, five night characters, start
> money and rent options, and two difficulty presets — **Cozy** (default) and **Classic**. Some ideas below
> (town legends, collections, careers, challenge seeds) are still ideas. Where they differ, the data wins.

---

## 1. What makes it replayable

1. **Every town has a different flow of guests** (the institutions nearby decide who comes and when).
2. **Random staff** — trait combinations turn the rozpis into a "who works with whom" puzzle.
3. **The night's character** (calm / chaotic) changes the pace (events.md §6.4).
4. **A town legend** — every town has its own "3:33 guest": one odd regular with a mini-arc.
5. **Long-term goals**: reputation levels, collections, seasons, rising rent.

---

## 2. Town generation — parameters

| parameter | options | what it affects |
|---|---|---|
| `seed` | a number | everything below is reproducible (share a town with a friend) |
| `name` | generator: `[prefix] + root + suffix` — prefixes "Horní/Dolní/Nová/Stará", roots "Ml-, Vran-, Lip-, Kos-, Bor-, Hruš-", suffixes "-ov, -ice, -any, -ín, -ná Lhota" | flavour only, but checked so the name doesn't match a real town from the stoplist |
| `size` | village / small town / district town | base flow: 0.6 / 1.0 / 1.5 |
| `station` | halt / junction / terminus | night trains: 0–1 / 2–3 / 1 + a chance of the late-train event |
| `institutions` | 2–4 from the list (§3) | which guest pools are on and at what hours |
| `climate` | dry / rainy / foggy | the chance of rain, fog, storm |
| `season_start` | spring / summer / autumn / winter | starting weather; the season changes every 30 nights |
| `building` | 3 floor-plan templates: "buffet" (narrow), "corner" (L-shaped), "old ticket office" (square, no yard) | hall size, where the kitchen is, whether there's a yard |
| `rent` | 800–2 500 Kč/night (from `size`) | money pressure |
| `competitor` | none / a NONSTOP petrol station / a kebab place by the station | −10…−25 % of guests in its pool; can be "beaten" with reputation ≥ 70 |
| `legend` | 1 from the pool of town legends (§5) | a special odd guest |

---

## 3. Guest pools

*(The shipped game has 12 pools, including 2039 ones such as drone operators — see `assets/data/guests.ron`.)*

| institution | pool | hours | what they order | patience | pay | special |
|---|---|---|---|---|---|---|
| (always) the station | `travelers` | all evening, peaks around trains | coffee, topinka | medium | average | suitcases take up space |
| taxi rank | `taxi` | 23:00–01:00, 03:00–04:00 | black coffee | low | exact + a tip for speed | come one at a time, sometimes in a crowd |
| hospital | `medics` | 02:00–02:30, 05:30–06:00 | cappuccino, soup | low | average | grateful: reviews +50 % |
| college / dormitory | `students` | 23:30–04:00 | coffee, cheap things | high | little | stay long, want sockets and Wi-Fi |
| culture house / club | `young` | 23:00–01:00 (Fri/Sat) | everything at once | low | well | crowds, noise, dirt |
| factory / warehouse on three shifts | `workers` | 22:00, 06:00 (shift change) | soup, chlebíček, lots | medium | well | come as a group on schedule |
| highway / truck stop | `truckers` | 01:00–05:00 | food, coffee to go | medium | well | their trucks fill the yard |
| police / fire station | `uniforms` | 01:00–04:00 | coffee | medium | exact | fewer dine-and-dash events |
| (always) locals | `insomniacs` | 00:00–04:00 | tea, koláč | high | little | become regulars faster than anyone |

**Regulars:** any ordinary guest who has visited happily five times may become a regular (30 % chance). They get a name, a favourite order and one random mini-arc from the templates:

| arc template | what it's about | 3 stages |
|---|---|---|
| `arc_burnout` | tired of work | notice → give them a quiet corner → a decision (stay/leave) |
| `arc_exam` | studies / an exam | a power socket → support them when they fail → a thank-you cake |
| `arc_lonely` | loneliness | remember their order → introduce them to another guest → "brought a friend" |
| `arc_secret` | hiding something | trust → confession → the player's choice |
| `arc_talent` | a hidden talent | notice → give them a stage or a wall → their evening (reputation +) |

---

## 4. Difficulty knobs

*(historical)* The draft had three levels. The shipped game has two presets: **Cozy** (default — soft payments,
no game over, ×1.25 income, patient guests) and **Classic** (payments matter, the café closes after two missed
payments, Sandbox bankruptcy after three nights in the red).

| knob | easy | normal | hard |
|---|---|---|---|
| start money | 15 000 Kč | 8 000 | 3 000 |
| rent increase | none | +5 % every 10 nights | +10 % every 7 nights |
| guest patience | ×1.3 | ×1.0 | ×0.75 |
| weight of a bad review | ×0.7 | ×1.0 | ×1.5 |
| equipment wear | ×0.5 | ×1.0 | ×1.5 |
| staff needs decay | ×0.8 | ×1.0 | ×1.2 |
| bankruptcy | never (the debt grows) | 3 nights in a row in the red | 2 nights in a row in the red |
| night's character | calm / chaotic / "custom sliders" (`base_rate`, share of `good`, `uncanny`) | | |

All of these are numbers in `difficulty.ron`. A menu isn't needed: a few ready-made presets are enough to start.

---

## 5. Town legends (an odd regular)

| id | legend | when they come | what's needed | arc reward |
|---|---|---|---|---|
| `legend_333` | the 3:33 guest (as in Story, but without Lída) | 03:33 | their table free, the light on | coins → a collection; the clock "stops" forever (decor) |
| `legend_lamplighter` | a lamplighter who lights lamps that no longer exist | 04:44 | a lamp in the window | +5 % guests in fog |
| `legend_conductor` | the conductor of a cancelled train | only in fog | a "ticket" (a ticket punch — furniture) | the rare `odd_*` guest pool ×2 |
| `legend_letters` | a woman who writes letters and leaves them unsent | 01:11 | a postbox (furniture) | letters → staff mood, flavour lore about the town |

---

## 6. Long-term goals

- **Reputation levels** (unlock furniture, menu items, pools):
  | reputation | title | unlocks |
  |---|---|---|
  | 0–19 | "Greasy spoon" | the base menu |
  | 20–39 | "Our place" | Wi-Fi, power sockets, a second coffee machine |
  | 40–59 | "A night address" | a stage, the "quiet corner", the mulled-wine licence |
  | 60–79 | "The light by the station" | rare guests, bloggers, tourists |
  | 80–100 | "Town legend" | the town's own legend is fully revealed |
- **Collections:** "Town legends" (one per town), "Odd guests" (all `odd_*`), "Recipe book" (recipes from regulars and from staff inspiration), "Lost and found" (forgotten items).
- **Staff careers:** after 30+ nights a staff member with skill 9+ may (a) become shift lead (a team buff) or (b) leave to open their own café. Then they turn up in another seeded town as a competitor or a friend — a meta-link between playthroughs.
- **Seasons:** every 30 nights the season changes: weather, pools (more tourists in summer, more insomniacs in winter) and seasonal events.
- **Challenges (challenge seeds):** "No kitchen", "Only two staff", "Chaotic night + village", "Everyone is clumsy".
- **Run score** (to compare seeds): nights without bankruptcy × average reputation + arcs closed × 100.
