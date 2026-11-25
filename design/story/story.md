# Story — Shiftella

> Story mode: 30 nights to save grandma's little café by the station.
> Tags: **[MVP]** — needed for the first prototype, **[later]** — after it.
>
> **Design notes, written before implementation.** The shipped nights, beats, notes and endings are in
> `assets/data/story.ron` and `assets/data/cutscenes.ron` (texts in `assets/i18n/src/story.tr`,
> `notes.tr`, `scenes.tr`); see [`docs/CONTENT.md`](../../docs/CONTENT.md) §5–6. Where they differ, the data wins.

---

## 1. Tone and themes

- **Tone:** cozy, but a little sad. Rain, soft holographic signs, steam over a cup, quiet music from an old radio. A touch of the uncanny — never horror: nothing is frightening, it only makes you wonder.
- **Feeling:** "the only window with a light on". At night the town sleeps, and the café is there for those who can't sleep or mustn't.
- **Near future, never grim:** it is 2039 — autonomous night trains glide in almost silently, small delivery drones cross the wet roofs, signs are holograms. But the café is intimate and human, never cyberpunk.
- **Themes:**
  - **Waiting.** Everyone is waiting for something: a train, the end of a shift, an answer, the morning.
  - **A light for strangers.** A small place that quietly holds up the people around it.
  - **Inheritance.** You don't just receive a business — you decide what it becomes next.
  - **Letting go or holding on.** Closing an old story, or carrying it further.
- **What we DON'T do:** deaths on screen, jump scares, blood, cartoon villains. A bad outcome is sadness, not horror.
- **Humour:** quiet and everyday (a coffee machine with a personality, a cat that "also works here").

---

## 2. The world

### The town — Mlhov nad Šeřavou
- A fictional little town in Czechia, ~9 000 inhabitants. The name comes from *mlha* (fog); the river Šeřava from *šero* (dusk).
- At night fog lies over the river, so the café window can be seen from far away.
- Once: the glassworks «Sklárna Mlhov» ran three shifts and closed in 2009. The town's night life nearly disappeared with it.
- Now: a district hospital (night shifts), a branch of a technical college (students), a culture house (concerts on Fridays), a taxi rank by the station, a highway nearby, a drone delivery depot.

### The station — «Mlhov»
- A small junction station: two lines, local trains plus a couple of night expresses — by 2039 mostly autonomous.
- The station building dates from 1912 and is half empty. The ticket office is open only during the day. At night there is a single dispatcher left — and soon not even him.
- The departure board sometimes "blinks". On the old timetable on the wall there is a train **Rn 333, arriving 3:33** — officially cancelled in 1979.
- Easter egg: the stations on the line are named after Shiftella release codenames — Adamov, Beroun, Cheb, Děčín, Ejpovice, Frýdlant … and then Mlhov.

### The café — «U Výhybky» ("At the Switch")
- It occupies the former station buffet in the left wing of the building. A 16×16-tile plan: a hall with 6–8 tables, a bar counter, a small kitchen, a storeroom, a toilet, a back yard.
- **History:**
  - 1968 — «Bufet ČSD» opens for the night shifts of the glassworks and for passengers.
  - 1979 — young Ludmila "Lída" Kratochvílová (grandma) takes over the buffet.
  - 1991 — Lída buys the business and renames it «U Výhybky».
  - 2009 — the glassworks closes and the café slowly grows poorer.
  - 2039, autumn — Lída dies. She leaves debts, a lease that is running out, and the keys — to the player.
- **Lída's rule:** the café is open **only at night, 22:00–06:00**, and the light in the window stays on until six. Why — that is the central mystery.

---

## 3. Grandma — Ludmila "Lída" Kratochvílová (1956–2039)

- Short, sharp with words, generous with deeds. She remembered what everyone drank but mixed up names.
- For sixty years she kept the light in the window. Friends called it stubbornness; the town called it a habit.
- She left **10 notes** (see §6) — on napkins, on the back of bills, in the recipe book. She writes them "to whoever keeps the light".
- The player is her grandchild (player lines are gender-neutral in every language). You were last here as a child.

---

## 4. The central mystery

**The question:** why did Lída keep the café open at night for sixty years — and who is the quiet guest who comes **at exactly 3:33**?

**The truth (revealed gradually; the player is never told it outright):**
1. In 1979 Lída's fiancé, **Jindřich**, left to work abroad on the night train Rn 333. They had quarrelled the day before.
2. That night Lída wrote him a letter: *"I'll wait. The light in the window will be on."* She asked a young taxi driver, **Mirek**, to take it to the platform.
3. Mirek was in love with Lída and **never delivered the letter**. Jindřich left believing nobody was waiting for him, and he never came back. No letters came from him either.
4. Lída kept waiting — and kept the light on. Mirek has carried the letter all his life and said nothing. It is his secret.
5. The 3:33 guest wears an old coat, and in his pocket is a ticket for Rn 333 from 1979. Is it Jindřich? His echo? Or just a strange old man? **The game doesn't say.** One thing is clear: he is still waiting for an answer.

**Resolving it in play:** if the letter reaches the guest at 3:33 on night 30, the story closes — the optional **"Answer"** scene (§8), played inside either ending.

---

## 5. The 30-night arc

### Story state (flags for the code)
In the shipped data these are vars and counters declared in `story.ron` (see `docs/CONTENT.md` §5).

| id | what it is | range |
|---|---|---|
| debt | rent and supplier debt | starts at 90 000 Kč |
| payments | 15 000 Kč on nights 7 / 14 / 21 / 28, a final 30 000 Kč on night 30 | made or missed |
| `missed_payments` | how many payments were missed | on Classic, 2 → the «Last Train» ending early; on Cozy, a mild story consequence |
| reputation | the café's reputation | 0–100, starts at 25 |
| `offer` | the franchise offer | `none` / `pending` / `accepted` / `rejected` |
| notes found | Lída's notes found | 0–10 |
| `arc.<regular>` | stage of a regular's personal arc | 0..N (see characters.md) |
| `letter` | the letter's fate | `hidden` → `confessed` → `with_player` / `with_mirek` / `burned` / `persuaded` → `delivered` / `delivered_by_mirek` |

### Act I — "Keys" (nights 1–7) — learning the ropes **[MVP: nights 1–3 as the tutorial]**
| night | key moment |
|---|---|
| 1 | You arrive on the last train at 21:50. Dust, the key under the mat. The first guest is Mirek the taxi driver: "Black, like Lída made it." You learn to take an order, make coffee, take money. |
| 2 | You hire your first colleague (a choice of candidates). You learn the rozpis in the Shiftella app and work priorities. Under the till lies **note no. 1**. |
| 3 | At 2:10 Hana the nurse comes in for the first time. The first random event is always gentle (rain). |
| 4 | **The first 3:33.** The clock on the wall stops for a minute. A guest in a grey coat comes in, orders tea with lemon and leaves without a word. Old Czechoslovak coins are left on the table. |
| 5 | Mrs Věra arrives, Lída's old friend and rival. She is cold: "Lída wouldn't have done it like that." |
| 6 | A scripted crowd after a concert — the first stress test. Filip the student appears. |
| 7 | **Payment no. 1.** During the day a letter comes from the landlord: the lease ends after night 30 and will be renewed only if the debt is cleared. |

### Act II — "Regulars" (nights 8–16) — the guests' arcs **[later]**
| night | key moment |
|---|---|
| 8 | Mirek's taxi clips the kerb by the entrance. He laughs it off. The first hint that his eyesight is failing. |
| 9 | Radek, the night dispatcher: "They're automating the station. I'm the last one." He remembers the cancelled Rn 333. A quiet guest with a laptop sits at the socket table (the Anwar cameo, see characters.md §3.8). |
| 10 | A scripted power cut — candlelight service. **Note no. 3** turns up in the fuse box. |
| 11 | Lori plays on the platform in the rain. Choice: invite her in or not (starts her arc). |
| 12 | Hana falls asleep at a table. Choice: wake her / cover her with a blanket and let her be / set up a "quiet corner". |
| 13 | A scripted health inspection. |
| 14 | **Payment no. 2.** If Věra likes you enough, she brings Lída's recipe book (**note no. 5** + a new recipe). |
| 15 | The 3:33 guest speaks for the first time: "Is she still waiting?" The player's answer (yes / I don't know / she's gone) is stored. |
| 16 | A quiet night (guaranteed). Time for staff relationships: a romance or a quarrel may happen. |

### Act III — "The Offer" (nights 17–24) — the big choice **[later]**
| night | key moment |
|---|---|
| 17 | Mr Šindelář from the **NONSTOP 24** chain visits. The offer: the chain pays off the whole debt and you stay on as manager. The terms: open 24/7, their menu, their sign, "none of grandma's rules". You have until night 22. `offer = pending` |
| 18 | Filip has failed an exam and sits there until morning. You can support him (arc) or stay out of it. |
| 19 | A scripted downpour: twice the guests — a chance to earn. |
| 20 | **Mirek nearly hits a pedestrian** by the station. Choice: take his keys / call his daughter / say nothing. If his trust is high, he confesses about the letter (`letter = confessed`). |
| 21 | **Payment no. 3.** |
| 22 | **The franchise decision.** Accept → `accepted`: the debt is paid off, but the café becomes a NONSTOP 24 — this leads to the «Last Train» ending. Decline → `rejected`. |
| 23 | Radek's last shift. He gives you the old timetable with the line "Rn 333 Mlhov 3:33" (**note no. 8** tucked inside). |
| 24 | If Lori was invited in — her evening at the microphone: reputation +10, a full house. The quiet guest with the laptop comes back for his second and last visit (the Anwar cameo). |

### Act IV — "The Last Train" (nights 25–30) — the resolution **[later]**
| night | key moment |
|---|---|
| 25 | A storm. The station lights go out; only the café window glows. Everyone who can't sleep comes in. |
| 26 | Hana decides: a job in Dresden, or stay. What the player did in her arc matters. |
| 27 | First snow. Věra tells you about Jindřich — if you have found enough notes. |
| 28 | **Payment no. 4.** Mirek: "If I give him the letter myself — will he see me?" The letter decision (§7). |
| 29 | A farewell night for the arcs: every regular comes to close their story. |
| 30 | **The final night.** The final 30 000 Kč payment at 06:00. At 3:33 the Rn 333 "arrives" at the empty platform — if the light in the window is on. The letter scene. Then the ending. |

---

## 6. Lída's notes (a collection) **[MVP: 1 note; the rest later]**

The shipped wording is in `assets/i18n/src/notes.tr`; this table gives the meaning and where to find them.

| id | where to find it (condition) | meaning |
|---|---|---|
| `note_01` | under the till, night 2 (scripted) | "Don't switch off the light in the window before six. It isn't superstition. It's a promise." |
| `note_02` | clean the storeroom completely | a "who drinks what" list — reveals the favourite orders of the regulars |
| `note_03` | the fuse box, during the power cut | "When the lights go out, people tell the truth. Keep the candles dry." |
| `note_04` | Hana's arc, stage 2 or more | "Those who heal get served without queueing." |
| `note_05` | the recipe book, from Věra (night 14) | the recipe for grandma's kulajda + "Věra knows more than she says" |
| `note_06` | collect the 3:33 guest's coins | "He always pays with coins that no longer exist. I don't take his money." |
| `note_07` | Mirek's arc, stage 2 or more | "M. looks at me as if he were guilty. I forgave him long ago — whatever it is." |
| `note_08` | Radek's timetable (night 23) | "3:33. I reached the platform two minutes late. All my life, two minutes." |
| `note_09` | repair the old radio (upgrade) | "If he comes again, tell him I waited. Not for my sake. For the light's." |
| `note_10` | reputation 70 or more | "A café isn't its walls. It's the people who come. Don't sell them." |

---

## 7. How the choices branch

The story is linear night by night, but **four axes** branch it:

1. **Money** — the payments. This is the mechanics: how much the player earned.
2. **The franchise** — `offer` (night 22). A moral choice: "safe" versus "our own".
3. **People** — the regulars' arcs (how many end well) + reputation.
4. **The letter** — `letter` (nights 20–30):
   - Mirek has confessed → the player can **keep the letter**, **leave it with Mirek**, **burn it** or **persuade Mirek to deliver it himself**.
   - If Mirek hands over the letter himself at 3:33 on night 30, he sees the guest for the first time. This is the best outcome for Mirek's arc.
   - If the player hands it over, it still counts, but Mirek is left with his guilt (the epilogue differs).
   - If the letter was burned or the secret never came out → the 3:33 guest leaves without an answer.

Small choices (waking Hana, inviting Lori, taking Mirek's keys) move the guests' arcs and reputation. Each is described in characters.md.

---

## 8. Endings

There are **exactly two endings**. They are checked in order at the end of night 30 (or when the café closes early on Classic). Each has variants that name the outcome on the summary screen.

| id | title | conditions | what it is |
|---|---|---|---|
| `last_train` | **«Last Train»** | the franchise was accepted, **or** the café failed its payments on Classic, **or** reputation is below 20 with no regular's arc ending well | U Výhybky closes — or lives on under someone else's sign. A bittersweet farewell. Variants: **sold** (NONSTOP 24 — "The window is always lit, and nobody waits in it"; the 3:33 guest never returns), **farewell** (the regulars help you pack and talk you into opening something of your own; you leave on the morning train with the recipe book), **closed** (a quiet goodbye). |
| `light` | **«Light in the Window»** | everything else (the fallback) | The café is saved and stays itself. Morning: you switch off the light at exactly 6:00 — for the first time as its owner. Variants: **full** (a full café at six in the morning; the regulars are your new family) and **quiet** (the café lives, but is empty: "The light is on. For now, there's nobody to wait for."). |

**The "Answer" scene** is **not a third ending**. It is an optional extra scene that either ending plays when it holds: the letter was delivered at 3:33 on night 30, at least 8 notes were found, and the franchise offer was not accepted. The guest reads the letter and smiles: "So she waited." He walks out to the platform, and for a second the board shows "Rn 333 — ODJEL" (departed). If Mirek handed over the letter himself, he sleeps peacefully for the first time in sixty years. The ending screen shows it as an "Epilogue unlocked: The Answer" badge.

*(historical)* Early drafts had a third ending, «Round the Clock» (`end_nonstop`), for accepting the franchise. In the shipped game it became the **sold** variant of «Last Train».
