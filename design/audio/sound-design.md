# Shiftella — sound design

This document sets the game's sound direction, music states, the adaptive scheme, the full SFX list, loudness rules, file names and the folder layout.
There is no character voice acting, and none is planned (the owner's decision).

> **The generated audio is not in the public repository.** The music and SFX were generated with
> ElevenLabs, whose licence lets us use them inside the game but not redistribute them on their own. So
> `assets/audio/` and `design/audio/raw/` are kept out of the public repo and ship only inside release
> builds; the game runs silently without them. To hear the game from source, generate your own copies with
> your own ElevenLabs key — see [§8](#8-how-to-generate).

Related files:
- `design/audio/prompts.json` — the generation prompts (change the text, regenerate).
- `design/audio/generate.py` — the generation script (ElevenLabs API).
- `design/audio/credits-log.md` — what was generated, credits used, licences.
- `design/audio/sources.md` — free tracks and packs from the web (candidates; nothing downloaded).

---

## 1. Direction

**In one sentence:** warm night-time lo-fi in a small café by the station. Outside — rain and trains; inside — the coffee machine and quiet jazz. Now and then something sounds slightly "off", but it's never scary.

**Pillars:**

| Pillar | What it means for the sound |
|---|---|
| Coziness | Warm timbres: Rhodes, felt piano, upright bass, brushes. Soft attacks, no harsh highs. Everything sounds as if heard from behind the counter. |
| Melancholy | Minor and dorian, seventh chords, slow tempos (64–80 BPM). Gaps in the arrangement, "loneliness at 4 a.m.". |
| Night and the town | Rain, the hum of holo signs, distant (almost silent, autonomous) trains and wordless station chimes (just a "ding-dong"). The street is heard through the glass: muffled, with calm lows. |
| A touch of the uncanny | A slightly detuned pad, wobbling tape, reversed piano, a ticking clock that suddenly stops. Only a hint. No jump scares, screeches or horror drones. |
| Czech small-town colour | Sometimes (in the story, at dawn) a clarinet, an accordion or a distant brass band. A memory of grandma's café, not folk music. |

**Genre and mood references (style, not copying):**
- lo-fi hip-hop and "study beats": dusty beats, vinyl, jazz chords;
- a late-night jazz trio (piano, upright bass, brushes) — for the late hours;
- jazz-hop and nu-jazz — for the rush;
- field-recording ambience (rain, the station) — the bed;
- a tape "hauntology" effect: tape degradation and detune — for the 3:33 guest;
- music for 1960s–70s European TV films (clarinet, vibraphone) — for story moments.

**Not allowed:** vocals (including AI vocals), EDM drops, epic orchestra, horror stingers, imitating specific composers or games.

**Instrument palette:** Rhodes/Wurlitzer, felt piano, nylon guitar, upright bass (or a soft synth bass), brushes and dusty drum loops, vibraphone, muted trumpet, clarinet, accordion (rarely), detuned pad, vinyl and tape noise.

---

## 2. Music states

| State | When it plays | Mood and sound | Tempo / key | Length, loop | File | Status |
|---|---|---|---|---|---|---|
| **Main menu** | The title screen | Warm nostalgic piano, brushes come in, a little mystery in the pad | ~75 BPM, D minor | 60–90 s, soft ending, repeats | `mus_menu_theme.ogg` | generated (first batch) |
| **Day / planning** | The screen between shifts: supplies, hiring, the rozpis | Brighter and simpler: guitar, piano, no drums or very quiet ones. "Morning coffee after the shift" | ~85 BPM, major/mixolydian | 60–90 s, loop | `mus_day_planning_loop.ogg` | P1 |
| **Calm night** | 22:00–01:00 and any time with few guests | Lo-fi beat, Rhodes, lots of room for the rain | ~72 BPM, A minor / dorian | 90–120 s, loop | `mus_night_calm_loop.ogg` | generated (first batch) |
| **Rush** | A queue, or the hall is nearly full | Jazz-hop: swinging drums, walking bass, vibraphone, trumpet. Lively, but cozy | ~96 BPM | 60–90 s, loop | `mus_rush_loop.ogg` | generated (first batch) |
| **Deep night** | 03:00–05:00 | Sparse: solo piano and pad, the drums almost disappear. Silence is part of the music | ~64 BPM, minor | 90–120 s, loop | `mus_late_night_loop.ogg` | P1 |
| **3:33 — the strange guest** | At exactly 03:33 (story) | The music doesn't change, it "spoils": slows by 3–5 %, the tape wobbles, the clock stops. A separate short stinger and a bed | — | stinger 3–5 s + a 20–30 s bed loop | `stg_333_arrival.ogg`, `amb_333_bed_loop.ogg` | P1 |
| **Dawn** | 05:00–06:00 | Warmth returns: clarinet or accordion, birds, the first tram | ~70 BPM, major | 60 s, loop or once | `mus_dawn.ogg` | P2 |
| **Shift summary** | The results screen | A short jingle: a good night — a warm cadence, a bad one — a gentle minor | — | 4–8 s | `stg_shift_good.ogg`, `stg_shift_bad.ogg` | P1 |
| **Key story moments** | Night 1 (the inheritance, the first shift), grandma's notes, the regulars' stories, night 30 | Grandma's leitmotif (a simple 4-bar clarinet or piano melody). It returns in variations in the menu, at dawn and in the finale | — | 30–60 s, once | `mus_story_grandma_theme.ogg`, `mus_story_night01.ogg` … | P2 |
| **Finale** | After night 30 («Light in the Window» or «Last Train») | The full version of the leitmotif: piano, strings, clarinet. Bittersweet, without pathos | ~70 BPM | 2–3 min | `mus_ending.ogg` | P2 |

**Leitmotifs:**
1. *Grandma / the café* — a 4-bar melody. A hint of it in the menu, the whole thing in the finale.
2. *The train* — a "ta-da-dum, ta-da-dum" rhythm (rail joints). It can be woven into the calm-night drums.
3. *3:33* — a tritone in the pad and the ticking stopping.

For ElevenLabs generation it currently works best to describe the leitmotif in words in the prompt. An exact note match needs MIDI or a human composer (see "Next steps").

---

## 3. The adaptive scheme

The sound is built from three independent layers, each with its own volume.

```
[Music]     calm_loop ⇄ rush_loop (crossfade)        — depends on the number of guests
[Ambience]  rain_loop (intensity) + room tone + holo — depends on weather and time
[Events]    SFX and stingers, on top of everything, ducking the music
```

### 3.1 Music intensity by number of guests
- Metric: `load = guests_in_hall + 1.5 × queue`, smoothed with a 10 s moving average.
- `load < 5` — calm night. `load ≥ 7` — rush. Between 5 and 7 nothing switches (hysteresis), so the music doesn't twitch.
- A transition takes 4–6 s: one track fades to 0 while the other comes up. Both tracks play in parallel (`looped: true`) and the inactive one is held at volume 0, so they stay in step when you come back. The tempos differ (72 and 96 BPM), so the beats won't line up perfectly; the crossfade hides it.
- Minimum time in a state — 20 s.
- **Later (v2):** stems (bass+keys / drums / solo) of one track. Drums come in at `load ≥ 3`, the solo at `load ≥ 7`. ElevenLabs currently gives a finished stereo mix, so stems are a job for a composer or separate section generation.

### 3.2 Time of night
- 22:00–03:00 — calm / rush as above.
- 03:00–05:00 — `late_night_loop` plays instead of calm. Rush stays, but at −3 dB.
- 05:00–06:00 — dawn. The rain dies down, birds.
- quad-snd has no filters or pitch. "Muffled", "slowed" and "far away" effects are pre-rendered as separate files with ffmpeg (e.g. `mus_night_calm_loop_muffled.ogg` with a ~1.5 kHz low-pass) and crossfaded to.

### 3.3 Weather and ambience
- `amb_rain_window_loop` always plays while it rains. The weather sets the volume: light rain −12 dB, medium −6 dB, downpour 0 dB. `amb_rain_heavy_loop` can be added later.
- When the door opens (a guest comes in or leaves), the rain gets +6 dB louder for 1–2 s: "the street bursts in". Cheap, and it works well.
- The café room tone (fridge, holo signs, a quiet hum): `amb_cafe_roomtone_loop`, −28 LUFS. It cuts out abruptly in a power cut (see `sfx_power_outage`).
- Trains: `amb_train_pass_far` follows the timetable (night trains at 22:47, 00:15, 01:52 and 05:20, plus the phantom Rn 333 at 03:33 on night 30); in Sandbox, randomly every 8–15 game minutes.

### 3.4 Ducking
- Event alert: music −4 dB for 1.5 s (attack 0.1 s, release 0.8 s).
- Story stinger: music −8 dB, or a pause.
- 3:33: the music fades to 0 over 3 s; `amb_333_bed_loop` remains.

### 3.5 macroquad / quad-snd limits (for programmers)
- API: `load_sound`, `play_sound(sound, PlaySoundParams { looped, volume })`, `play_sound_once`, `set_sound_volume`, `stop_sound`. Volume can change on the fly, which is enough for crossfades. No panning, pitch or filters.
- Against monotony: frequent sounds (footsteps, cups, the register) get 3–4 variants `_01.._04`, picked at random with a slight volume change (±2 dB).
- Sound is probably kept in memory decoded. Check this in the quad-snd code. If so, a two-minute stereo track takes tens of MB, so music is loaded lazily and unused tracks are unloaded.
- ffmpeg's built-in Vorbis encoder writes stereo only, so all our OGGs are stereo. Short SFX are WAV and may be mono.
- The game must run without any audio files (they are not in the public repo).

---

## 4. The full SFX list

Priority: **P0** — the first batch (in `prompts.json`, generated), **P1** — needed for a vertical slice, **P2** — later.

### 4.1 Café and service
| File | Game event | Description | Length | Loop | Var. | Prio. |
|---|---|---|---|---|---|---|
| `sfx_door_bell.wav` | A guest comes in / leaves | A brass bell above the door | 1–1.5 s | — | 3 | P0 |
| `sfx_door_open_close.wav` | The door (no bell, for staff) | Creak and thud of a glass door | 1 s | — | 2 | P1 |
| `sfx_coffee_machine.wav` | The barista makes coffee | Pump, pour, steam | 4–6 s | — | 2 | P0 |
| `sfx_coffee_grinder.wav` | Grinding | A short grind of beans | 2 s | — | 1 | P1 |
| `sfx_kettle_boil_loop.ogg` | Kettle / tea | Boiling, a whistle at the end | 6 s | — | 1 | P2 |
| `sfx_cash_register.wav` | Payment | A mechanical register, "ka-ching" and the drawer | 1–1.5 s | — | 2 | P0 |
| `sfx_coins.wav` | Tips | Coins on the counter | 0.7 s | — | 3 | P1 |
| `sfx_card_beep.wav` | Card payment | The terminal's beep | 0.4 s | — | 1 | P2 |
| `sfx_cup_saucer.wav` | Serving a drink | A cup set on a saucer | 0.5–1 s | — | 4 | P0 |
| `sfx_plate_set.wav` | Serving food | A plate on a table | 0.6 s | — | 3 | P1 |
| `sfx_dishes_wash.wav` | Washing up | Water and clinking | 3 s | — | 2 | P1 |
| `sfx_dish_break.wav` | Broken dishes (event) | A cup falls and breaks | 1.2 s | — | 2 | P1 |
| `sfx_oven_ding.wav` | Pastry is ready | The oven timer | 1 s | — | 1 | P1 |
| `sfx_fridge_open.wav` | Storeroom / fridge | Opening the fridge | 1 s | — | 1 | P2 |
| `sfx_pour_drink.wav` | Pouring (juice, beer) | A stream into a glass | 1.5 s | — | 2 | P2 |

### 4.2 Staff and guests (no voices)
| File | Event | Description | Length | Loop | Var. | Prio. |
|---|---|---|---|---|---|---|
| `sfx_footsteps_tile.wav` | Staff or guests walking | Footsteps on tiles | 3 s (cut into steps) | — | 4 | P0 |
| `sfx_footsteps_wet.wav` | A guest came in from outside | Wet footsteps, the first 2–3 steps | 2 s | — | 2 | P2 |
| `sfx_chair_move.wav` | A guest sits down / gets up | A chair on tiles | 0.8 s | — | 3 | P1 |
| `sfx_umbrella_shake.wav` | Coming in from the rain | An umbrella shaken out | 1 s | — | 2 | P2 |
| `sfx_coat_rustle.wav` | Taking off a coat | Rustling clothes | 1 s | — | 2 | P2 |
| `amb_cafe_murmur_loop.ogg` | The hall is full | An indistinct murmur of voices, no words. Volume follows the number of guests | 30 s | yes | 1 | P1 |
| `sfx_yawn_soft.wav` | Staff are tired (energy need) | A quiet yawn, no words | 1.5 s | — | 2 | P2 |
| `sfx_stomach.wav` | Staff are hungry | A comic rumble, quiet | 1 s | — | 1 | P2 |

### 4.3 Surroundings and weather
| File | Event | Description | Length | Loop | Var. | Prio. |
|---|---|---|---|---|---|---|
| `amb_rain_window_loop.ogg` | Rain (main layer) | Rain on the glass, drops, a gutter | 30 s | yes | 1 | P0 |
| `amb_rain_heavy_loop.ogg` | Downpour | Dense rain, sometimes distant (quiet) thunder | 30 s | yes | 1 | P2 |
| `amb_cafe_roomtone_loop.ogg` | Always | The fridge, holo signs, a quiet building hum | 30 s | yes | 1 | P1 |
| `amb_neon_buzz_loop.ogg` | By the sign / outside | The hum and crackle of the holo sign | 10 s | yes | 1 | P2 |
| `amb_train_pass_far.ogg` | A train passes (timetable or random) | A rising rumble, rail joints, fading away | 12 s | — | 2 | P0 |
| `sfx_station_chime.wav` | A station announcement | A distant station "ding-dong", no words | 2.5 s | — | 1 | P1 |
| `sfx_thunder_far.wav` | Storm | A distant, soft roll of thunder | 4 s | — | 2 | P2 |
| `amb_dawn_birds_loop.ogg` | 05:00–06:00 | The first birds, an occasional tram | 30 s | yes | 1 | P2 |
| `sfx_clock_tick_loop.ogg` | Silence between 3 and 5, story | A ticking wall clock | 4 s | yes | 1 | P1 |

### 4.4 Night events
| File | Event | Description | Length | Loop | Var. | Prio. |
|---|---|---|---|---|---|---|
| `sfx_power_outage.wav` | Power cut | The hum stops abruptly, a relay clicks | 2–3 s | — | 1 | P0 |
| `sfx_power_restore.wav` | Power is back | The holo sign flickers on, the hum returns | 2 s | — | 1 | P1 |
| `sfx_pipe_leak.wav` | A leak | Drips, hissing | 3 s | — | 1 | P2 |
| `sfx_machine_break.wav` | The coffee machine broke | The pump chokes, a knock, silence | 2 s | — | 1 | P1 |
| `sfx_phone_ring_old.wav` | The old phone rings (story) | A rotary phone, muffled | 3 s | — | 1 | P2 |
| `stg_333_arrival.ogg` | 03:33 | The clock stops, the tape slows, a soft detuned chord | 3–5 s | — | 1 | P1 |
| `amb_333_bed_loop.ogg` | The strange guest's presence | A quiet, warm but "wrong" pad, a distant train that isn't there | 20–30 s | yes | 1 | P1 |
| `sfx_cat_meow.wav` | A cat at the door (random event) | A short meow behind the door | 1 s | — | 2 | P2 |

### 4.5 Economy, reputation, progress
| File | Event | Description | Length | Loop | Var. | Prio. |
|---|---|---|---|---|---|---|
| `ui_alert_event.wav` | A new night event | A soft two-note glass bell | 1–1.5 s | — | 1 | P0 |
| `ui_reputation_up.wav` | Reputation went up | A warm rising three-note arpeggio | 1 s | — | 1 | P1 |
| `ui_reputation_down.wav` | Reputation went down | Two falling notes, gentle, not hurtful | 1 s | — | 1 | P1 |
| `ui_money_gain.wav` | The night's income is counted | A short "ding" plus coins | 0.8 s | — | 1 | P1 |
| `ui_unlock.wav` | A new recipe / upgrade | A sparkly bell | 1.2 s | — | 1 | P2 |
| `ui_need_warning.wav` | A staff need is in the red | A quiet knock-knock on glass | 0.6 s | — | 1 | P1 |
| `stg_shift_good.ogg` | Summary: a good night | Jingle | 4–6 s | — | 1 | P1 |
| `stg_shift_bad.ogg` | Summary: a bad night | Jingle | 4–6 s | — | 1 | P1 |
| `stg_night_start.ogg` | 22:00, opening | A bell and a breath of silence | 3 s | — | 1 | P1 |
| `stg_night_end.ogg` | 06:00, closing | The lock, the light switch, the first tram | 3 s | — | 1 | P1 |

### 4.6 Interface
| File | Event | Description | Length | Var. | Prio. |
|---|---|---|---|---|---|
| `ui_click.wav` | A button press | A soft wooden "tick" | 0.1–0.5 s | 2 | P0 |
| `ui_hover.wav` | Hover | Even quieter and shorter, barely audible | 0.05–0.1 s | 1 | P1 |
| `ui_back.wav` | Back / close | Slightly lower than click | 0.2 s | 1 | P1 |
| `ui_confirm.wav` | Confirm (hire, purchase) | Two rising notes | 0.4 s | 1 | P1 |
| `ui_error.wav` | Not allowed (no money) | A soft, dull "boom" | 0.3 s | 1 | P1 |
| `ui_page_turn.wav` | The log, grandma's notes | Rustling paper | 0.5 s | 2 | P1 |
| `ui_typewriter.wav` | Typing story text | Quiet keys, cut up | 0.1 s | 4 | P2 |
| `ui_speed_change.wav` | Pause / ×1 / ×2 / ×3 | A switch click | 0.2 s | 1 | P1 |

UI sounds are 6 dB quieter than game SFX. Repeat at most once per 80 ms, so hover doesn't "crackle".

---

## 5. Loudness and mix

**Target file levels** (set by `generate.py` with a two-pass, linear `loudnorm`, so loop seams don't suffer):

| Category | Integrated | True peak | Note |
|---|---|---|---|
| Music (`mus_`) | −18 LUFS | −1.5 dBTP | Leaves headroom for SFX |
| Ambience (`amb_`) | −24 LUFS | −2 dBTP | Sits under the music |
| SFX (`sfx_`) | peak −1…−3 dBFS | — | No auto-normalisation; the game sets the volume |
| UI (`ui_`) | peak −6 dBFS | — | |
| Stingers (`stg_`) | −16 LUFS | −1 dBTP | Short, may be a little louder than the music |

**Buses in the game** (sliders in the settings): Master, Music, Ambience, Effects, Interface. Defaults: 80 / 60 / 70 / 80 / 60 %. `M` toggles all sound.

**Rules:**
- The same SFX plays in at most three instances at once. A fourth call is skipped.
- All loops start and stop with a 0.3–0.5 s fade (via `set_sound_volume` per frame).
- Format: 44.1 kHz. SFX shorter than ~3 s are stored as 16-bit PCM WAV. Music, ambience and stingers as OGG Vorbis (q6, stereo).
- Music API loops aren't guaranteed to be seamless. Before use, trim them on a bar boundary and check the seam (Audacity or ffmpeg `acrossfade`). SFX have a `loop: true` parameter that gives a seamless loop.
- Test on laptop speakers and on headphones. It's a night game: no loud peaks and no piercing highs above 8 kHz.

---

## 6. File names

```
<prefix>_<subject>[_<detail>][_<variant>][_loop].<ext>
```

- Latin letters only, lower case, `snake_case`, no spaces or diacritics.
- Prefixes: `mus_` music, `amb_` ambience and beds, `sfx_` game effects, `ui_` interface, `stg_` stingers and jingles.
- Variants: `_01`, `_02` … (`sfx_cup_saucer_01.wav`).
- `_loop` — the file is made to repeat seamlessly.
- Don't put versions in the name when regenerating. The history is in `generation-log.jsonl`. To compare, add `_alt` temporarily.
- The id in `prompts.json` matches the "subject" (`rain_window_loop` → `amb_rain_window_loop.ogg`).

---

## 7. Folder layout

```
shiftella-game/
├── .env                      # ELEVENLABS_API_KEY=... (git-ignored, never commit it)
├── assets/audio/             # generated, NOT in the public repo (ships in release builds only)
│   ├── music/                # mus_*.ogg, stg_*.ogg
│   └── sfx/                  # sfx_*.wav, ui_*.wav, amb_*.ogg
└── design/audio/
    ├── sound-design.md       # this document
    ├── prompts.json          # generation prompts
    ├── generate.py           # generation script
    ├── generation-log.jsonl  # machine log of generations (no keys in it)
    ├── credits-log.md        # human log: what, how much, licences
    ├── sources.md            # candidates from the web
    └── raw/                  # source mp3s from ElevenLabs, NOT in the public repo
```

When there are more than 30 files, `assets/audio/sfx/` can be split into `sfx/`, `ui/`, `ambience/`. The paths in the code then change — agree on it with the programmers.

---

## 8. How to generate

You need your own ElevenLabs account. The Music API works only on paid plans; check the licence notes in
`credits-log.md` before you use the results anywhere.

1. Put **your own** key into `.env` in the repository root: `ELEVENLABS_API_KEY=...` (or export it as an environment variable). `.env` is git-ignored — never commit it, and never paste a key into an issue or a log. The script never prints the key.
2. From the repository root:
   ```
   python3 -I design/audio/generate.py --dry-run     # what would be generated (no requests)
   python3 -I design/audio/generate.py               # everything that doesn't exist yet
   python3 -I design/audio/generate.py --only ui_click --force   # regenerate one sound
   python3 -I design/audio/generate.py --kind sfx    # only the SFX
   python3 -I design/audio/generate.py --reconvert   # rebuild OGG/WAV from raw/ (no API, no credits)
   ```
3. The script downloads mp3s into `design/audio/raw/` and uses ffmpeg to make OGG Vorbis or WAV in `assets/audio/`. The `normalize` field in `prompts.json` sets the loudness target. Credits used (the difference in the subscription counter) are written to `generation-log.jsonl`.
4. Without ffmpeg, an mp3 is put into `assets/` and the script prints a warning. Note that macroquad can't play mp3, so run `--reconvert` once ffmpeg is installed.
5. Generated audio will sound different from the release build — that's expected; the prompts are only a starting point.

**ffmpeg:** if your ffmpeg is built without `libvorbis`, the script falls back to ffmpeg's built-in experimental `vorbis` encoder: it works, stereo only, slightly lower quality. For better OGGs install ffmpeg with libvorbis and run `--reconvert`.

---

## 9. Next steps
1. ~~Get a key and generate the first batch (3 tracks and 10 SFX), listen, adjust the prompts.~~ Done on 2026-11-25.
2. Check the loop seams and trim on a bar boundary if needed.
3. The P1 list: late night, day planning, the 3:33 and summary stingers, room tone, the murmur of voices, the UI set. The UI set could come from a CC0 pack (see `sources.md`) to save credits.
4. Grandma's leitmotif: write the 4 bars in MIDI, then ask for variations (ElevenLabs Music can take an audio reference on paid plans) or commission a composer.
5. Re-check the licence before every release (see `credits-log.md`): a paid plan is required, and for Music also the "Indie Game" condition.
