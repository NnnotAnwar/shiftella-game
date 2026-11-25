# Shiftella — story and content design

> **Design notes, written before implementation.** The ideas here are canon; the numbers, ids and
> final texts that shipped in v0.1.0 live in `assets/data/*.ron` and `assets/i18n/`, documented in
> [`docs/CONTENT.md`](../../docs/CONTENT.md). Where these notes and the data disagree, the data and
> [`docs/GAME-BRIEF.md`](../../docs/GAME-BRIEF.md) win. Notes that only describe the early plan are
> marked *(historical)*.

A cozy management game in a smooth vector "violet dollhouse" style. The year is **2039**. The night
café **Café Nocturne** ("kavárna u nádraží") stands next to **Praha-Holešovice** station in Prague and is
open 22:00–06:00. *(Earlier drafts used the fictional town "Mlhov nad Šeřavou" and the café «U Výhybky»;
renamed on 2026-11-26, together with simpler names: Lida, Jindra, Vera.)* The staff live their own lives: each person has skills, needs, traits and
relationships. The player never controls people directly — you write the rozpis (shift schedule) in the
in-game **Shiftella** app, set work priorities, arrange furniture and buy supplies. Events are dealt by
the **Night**, the game's storyteller.

**Story hook:** you inherit your grandmother's café along with a 90 000 Kč debt, and you have 30 nights to
save it. For sixty years grandma Lida kept a light burning in the window until morning. Every night at
exactly 3:33 a quiet guest in an old coat comes in, and an old taxi driver keeps a letter he never delivered.

## Files

| file | what's inside |
|---|---|
| [story.md](story.md) | tone, world, grandma Lida, the 3:33 mystery, the 30 nights by act, notes, branching, the two endings |
| [characters.md](characters.md) | staff archetypes, the menu, the 7 regulars and their arcs, the Anwar cameo, odd guests |
| [traits.md](traits.md) | base scales (needs, skills, relationships), 16 traits, an example in RON |
| [events.md](events.md) | the event catalogue + how the Night paces things (night curve, 30-night curve, calm/chaotic) |
| [sandbox.md](sandbox.md) | town generation, guest pools, difficulty, town legends, long-term goals |

**Tags:** **[MVP]** — needed for the first prototype, **[later]** — after it. Content is described as data
(`id`, conditions, effects, weights) so it could be moved into RON files — which is exactly what happened.

## The 10 most important things for the MVP *(historical)*

The goal of the prototype was **one playable night, 22:00–06:00**: guests arrive, the staff work on their own,
and in the morning we show a summary. All of this (and much more) is in the shipped game.

1. **Night and day loop** — game clock 22:00→06:00, then a paused planning phase (hiring, schedule, supplies). *(story.md §5, act I)*
2. **3 staff archetypes** — barista, cook and a gentle-giant helper. *(characters.md §1)*
3. **Needs `energy`, `hunger`, `mood` and the skills**, with the speed formula `0.5 + skill × 0.1`. *(traits.md §1)*
4. **4 traits** — `night_owl`, `chatterbox`, `clumsy`, `perfectionist`, using only `Mul`/`Add` effects. *(traits.md §2–3)*
5. **A five-item menu** — black coffee, cappuccino, lemon tea, topinka, garlic soup; stations bar/kitchen. *(characters.md §2)*
6. **An ordinary guest**: arrives → sits down → orders → waits (patience) → pays → leaves happy or unhappy → reputation ±. One pool, `travelers`.
7. **Mirek the taxi driver** — the first regular (fixed time, favourite order, no arc yet). *(characters.md §3.1)*
8. **The 3:33 guest + the `clock_333` event** — the clock stops, the guest drinks tea by the window and leaves coins. The main atmospheric hook, and cheap to build. *(characters.md §3.3)*
9. **Night storyteller v0** — `base_rate × curve(time)`, weights, `cooldown`. Ten MVP events, starting with five: rain, downpour, concert crowd, staff sick, big tip. *(events.md)*
10. **Lida's note no. 1** under the till + a morning summary screen (revenue, reviews, debt). Gives a sense of story from the very first night. *(story.md §6)*

## Suggested order after the MVP *(historical)*

staff relationships → Hana's and Mirek's arcs → payments and the 30 nights → the franchise offer and the
endings → the remaining notes → sandbox and generation.

## Originality

All the characters, the café, the NONSTOP 24 chain, the events and the Night storyteller were invented for
this game. Shiftella is related to colony sims by genre, not by borrowing: there are no other games'
storytellers, factions, characters or texts here.
