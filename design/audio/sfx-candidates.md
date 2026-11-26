# SFX candidates for the v0.2 interactions

**Status (2026-11-26):** the owner approved the list and the 25 Freesound files + Kenney packs were
downloaded into `design/audio/raw/` (git-ignored). The cut files are in `assets/audio/cc0/`,
credited in `assets/audio/CREDITS.md` (not `CREDITS.txt`). Wet footsteps came out of the Kenney
concrete steps instead of sean_sd2007's take (too much traffic under the steps).

Originally research only: the owner approved every download first.
Checked on 2026-11-26. Every Freesound licence below was read on the sound's own page. The Kenney licences were read on the pack pages, and the Kenney file names come from public repositories that ship the unchanged packs (for example `godotengine/godot-benchmarks`, `supplemental/audio`).

## What the code expects

- `src/audio.rs`: a one-shot `Sfx` loads from `assets/audio/sfx/<id>.wav`. An id that starts with `amb_` loads from `assets/audio/sfx/<id>.ogg` and can loop.
- `design/audio/sound-design.md`: short SFX are WAV (mono is fine), and loops are OGG (ffmpeg's Vorbis encoder writes stereo only). Frequent sounds get 2–4 variants `_01.._04`.
- Kenney packs ship **OGG**. Convert their one-shots to WAV with ffmpeg, or let the loader accept `.ogg` for `sfx_` ids.
- Most Freesound originals are long, 48–96 kHz stereo field recordings. Plan to cut each one to the 0.2–2 s moment we need, downmix to mono, normalise like the generated files, and loop-cut the three ambience loops. Commit only the cut files, never the raw downloads (`/design/audio/raw/` is already in `.gitignore`).
- Sound hooks waiting in the code:
  - `walk_home.rs`: back door creak (L248). Keys in the lock, wet steps, pigeons, tram bell and front door (L360). Rain on the awning and a phone buzz (L379).
  - `hands.rs` (appliance events, ~L1486–1510): warm-up / tray / service / clean-up, till count, trash, oven ding, bubbling, sputter. These currently reuse `DoorBell`, `CupSaucer` and `CoffeeMachine`.

## Licence rules applied

- **CC0 only** in this shortlist: free for commercial use, may ship inside the build, may be committed to a public repo, and no credit is required (we still list every file in `assets/audio/CREDITS.txt`).
- **Freesound needs a free account (login) to download.** Searching and listening work without one. Licences can differ per file, even within one author's uploads. Re-check the licence badge on the page when downloading.
- **Kenney.nl**: CC0, downloads without a login (the "donate" step is optional).
- **OpenGameArt** blocks automated reading (HTTP 403), so it wasn't used this round.
- **Sonniss GDC bundles** are royalty-free and can be used commercially, but they are **not CC0**. Third-party summaries of the licence say sounds may be redistributed only *embedded in a product*, and standalone redistribution is prohibited. Committing the files to a public repo would most likely break that rule, so Sonniss is left out. (I couldn't open the official licence page; check sonniss.com/gdc-bundle-license if we ever want it.)
- Rejected: [blizzard looped.aif](https://freesound.org/people/m0ments/sounds/679941/) is tagged CC0, but the uploader says it "comes off a classic sfx LP record". The provenance is doubtful.

CC0 link used below: [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/).

## Candidates

Legend: **P** = primary, **A** = alternative. "FS" = Freesound (login needed). "Kenney" = no login.

| Game sound | P/A | Source | Page | File | Licence | Format / length / size | Login? | How it sounds |
|---|---|---|---|---|---|---|---|---|
| Coffee machine hiss / steam (warm-up) | P | FS · sokworks (*Morning Coffee* pack) | https://freesound.org/people/sokworks/sounds/868483/ | Steaming Milk (Low) | [CC0](https://creativecommons.org/publicdomain/zero/1.0/) | WAV 48 kHz stereo · 8.5 s · 1.5 MB | yes | Soft, low steam-wand hiss with the jug submerged. Gentle, not shrill |
| | A | FS · sokworks | https://freesound.org/people/sokworks/sounds/868479/ | Espresso Machine Ambience | CC0 | WAV 48 kHz stereo · 3.8 s · 1.0 MB | yes | The machine's idle boiler hum. Good under the hiss as "warming up" |
| Machine purge / sputter | P | FS · PROpolisher | https://freesound.org/people/PROpolisher/sounds/824466/ | Coffee Machine, cup rinse, boiler hum, steam wand purge | CC0 | WAV 48 kHz stereo · 1:01 · 16.9 MB | yes | A real steam-wand purge plus hot water and boiler hum. Cut the 1–2 s purge burst |
| | A | FS · sokworks | https://freesound.org/people/sokworks/sounds/868481/ | Steaming Milk (Finish) | CC0 | WAV 48 kHz stereo · 1.6 s · 302 KB | yes | A short end-of-steaming sputter. Already the right length |
| Group-head knock / clean | P | FS · sokworks | https://freesound.org/people/sokworks/sounds/868472/ | Knock Box 2 | CC0 | WAV 48 kHz stereo · 0.19 s · 35 KB | yes | A dull wooden thump of the portafilter on the knock box |
| | P (clean) | FS · sokworks | https://freesound.org/people/sokworks/sounds/868497/ | Espresso Machine Empty Shot Pull | CC0 | WAV 48 kHz stereo · 7.6 s · 1.4 MB | yes | A blank shot to flush the group head: a pump hum and a water rush |
| | A | Kenney · Impact Sounds | https://kenney.nl/assets/impact-sounds | `impactWood_light_00x.ogg` / `impactMetal_light_00x.ogg` | [CC0](https://creativecommons.org/publicdomain/zero/1.0/) | OGG · ~6–9 KB each | no | Clean, generic knocks. Fine as a fallback, less "coffee" |
| Soup pot bubbling (loop) | P | FS · alienistcog | https://freesound.org/people/alienistcog/sounds/124584/ | simmering-sauce.aif | CC0 | AIFF 44.1 kHz stereo · 1:43 · 17.4 MB | yes | Thick, slow bubbling of tomato sauce on low heat. Cozy, long enough to cut a seamless 10–20 s loop |
| | A | FS · fthgurdy | https://freesound.org/people/fthgurdy/sounds/496236/ | Boiling in a pot | CC0 | WAV 44.1 kHz stereo · 34 s · 5.7 MB | yes | Food bubbling under a lid that opens (louder) and closes. Gives the bubble and a lid moment in one file |
| Lid clink / stir | P | Kenney · RPG Audio | https://kenney.nl/assets/rpg-audio | `metalPot1.ogg`, `metalPot2.ogg` | CC0 | OGG · 32 KB / 22 KB | no | Short metal-pot clinks. Two variants ready |
| | A | FS · qubodup | https://freesound.org/people/qubodup/sounds/859338/ | Huge Stainless Steel Cooking Pot Open Close | CC0 | FLAC 96 kHz stereo · 14 s · 3.5 MB | yes | A real lid lift and set. Has faint birds and traffic in the open moments, so cut tightly |
| Oven timer ding | P | FS · dland | https://freesound.org/people/dland/sounds/149506/ | Kitchen Timer — Done! | CC0 | WAV 44.1 kHz mono · 8.9 s · 768 KB | yes | A few ticks, then one bell-like chime. Use just the chime (~1 s) |
| | A | FS · sethlind | https://freesound.org/people/sethlind/sounds/265022/ | Toaster oven timer and ding.wav | CC0 | WAV 48 kHz mono · 54 s · 5.0 MB | yes | A toaster-oven countdown that ends in a ding. Cut the last second |
| Oven door | P | FS · bolkmar | https://freesound.org/people/bolkmar/sounds/503477/ | Oven door - open and close | CC0 | WAV 48 kHz stereo · 3.75 s · 1.0 MB | yes | A clean domestic oven door opening and closing, already edited |
| | A | FS · babuhr | https://freesound.org/people/babuhr/sounds/513339/ | Oven door1.wav | CC0 | WAV 96 kHz stereo · 22 s · 12.2 MB | yes | Several open/close takes to choose from |
| Fridge door | P | FS · The_Isot_is_Back | https://freesound.org/people/The_Isot_is_Back/sounds/643537/ | Fridge Door Open & Close | CC0 | WAV 48 kHz stereo · 1.9 s · 714 KB | yes | A short, noise-cleaned fridge door with the soft seal thump |
| | A | FS · bolkmar | https://freesound.org/people/bolkmar/sounds/503478/ | Fridge door - closing | CC0 | WAV 48 kHz stereo · 1.25 s · 358 KB | yes | Just the close. The same recordist as the oven door, so the two match |
| Fridge bottle | P | FS · Luisa_Sanchez | https://freesound.org/people/Luisa_Sanchez/sounds/816964/ | clinking of bottles | CC0 | WAV 44.1 kHz mono · 0.9 s · 123 KB | yes | A short clink of bottles. Recorded with a piezo mic, so it may need a low-pass to soften it |
| | A | Kenney · Impact Sounds | https://kenney.nl/assets/impact-sounds | `impactGlass_light_00x.ogg` | CC0 | OGG · ~6.5 KB each, 5 variants | no | Light glass taps. Clean and gentle |
| Till / cash drawer count (coins) | P | Kenney · RPG Audio | https://kenney.nl/assets/rpg-audio | `handleCoins.ogg`, `handleCoins2.ogg` | CC0 | OGG · small | no | A handful of coins being handled. Short and soft |
| | A | FS · florianreichelt | https://freesound.org/people/florianreichelt/sounds/447454/ | counting coins | CC0 | WAV 48 kHz stereo · 14 s · 5.2 MB | yes | Real coin counting. Gives several 1–2 s cuts (the uploader politely asks for an optional Instagram tag, not required) |
| Trash bag rustle | P | FS · semccab | https://freesound.org/people/semccab/sounds/154371/ | trashbag.wav | CC0 | WAV 48 kHz stereo · 13.6 s · 3.7 MB | yes | A plain plastic trash-bag rustle, close up. Cut 1–2 s |
| | A | FS · kyles | https://freesound.org/people/kyles/sounds/452498/ | garbage bag plastic throw down apartment staircase and handling.flac | CC0 | FLAC 48 kHz stereo · 30 s · 5.0 MB | yes | Bag handling and a drop. Usable for "take it out" |
| Door to outside (trash, back door) | P | Kenney · RPG Audio | https://kenney.nl/assets/rpg-audio | `doorOpen_1.ogg`, `doorOpen_2.ogg`, `doorClose_1..4.ogg` | CC0 | OGG · 17–30 KB each | no | Wooden door open and close sets. Six variants |
| | A | FS · MorenaLuz | https://freesound.org/people/MorenaLuz/sounds/814857/ | Old.Wooden.Door.Open.and.Close | CC0 | WAV 44.1 kHz stereo · 12 s · 3.1 MB | yes | An old wooden door, noise-cleaned in RX. Warm, a light creak |
| Light switch click | P | FS · funkysandwich | https://freesound.org/people/funkysandwich/sounds/870289/ | light_switch10 | CC0 | WAV 96 kHz stereo · 0.15 s · 91 KB | yes | A single crisp wall-switch flip. Already trimmed |
| | A | Kenney · RPG Audio | https://kenney.nl/assets/rpg-audio | `metalClick.ogg` | CC0 | OGG · 14 KB | no | A small metal click. Works for a vintage switch |
| Back door creak | P | Kenney · RPG Audio | https://kenney.nl/assets/rpg-audio | `creak1.ogg`, `creak2.ogg`, `creak3.ogg` | CC0 | OGG · 10–19 KB each | no | Short wooden creaks. Three variants, not horror-long |
| | A | FS · MorenaLuz | https://freesound.org/people/MorenaLuz/sounds/814857/ | Old.Wooden.Door.Open.and.Close | CC0 | WAV · 12 s · 3.1 MB | yes | The same file as the outside-door alternative: the opening has the creak |
| Rain on an awning (loop) | P | FS · kyles | https://freesound.org/people/kyles/sounds/454130/ | rain light on awning close drops on fabric.flac | CC0 | FLAC 48 kHz stereo · 1:22 · 8.6 MB | yes | Light rain tapping on a fabric awning, close. Soft, good for a 20–30 s loop |
| | A | FS · 1888software | https://freesound.org/people/1888software/sounds/554577/ | rain-on-canvas-awning.wav | CC0 | WAV 48 kHz stereo · 1:02 · 11.4 MB | yes | Rain on a lowered canvas awning, with individual drops audible |
| Phone buzz / message | P | FS · kyles | https://freesound.org/people/kyles/sounds/637346/ | cell phone smartphone vibrate single.flac | CC0 | FLAC 48 kHz mono · 4.8 s · 92 KB | yes | A single smartphone vibration. Short and quiet |
| | A | FS · semccab | https://freesound.org/people/semccab/sounds/154387/ | Phone vibrate soft.wav | CC0 | WAV 48 kHz stereo · 11 s · 3.0 MB | yes | A phone buzzing on a bed: muffled and soft, very cozy |
| Cat purr | P | FS · clareboots | https://freesound.org/people/clareboots/sounds/553962/ | Cat-purr.wav | CC0 | WAV 44.1 kHz mono · 1:35 · 12.0 MB | yes | A deep, steady domestic purr (with some head-scritching). Loopable after cutting |
| | A | FS · soundmary | https://freesound.org/people/soundmary/sounds/117612/ | Cat - Purr - Meow.wav | CC0 | WAV 44.1 kHz stereo · 35 s · 5.9 MB | yes | A close purr with small mews. Gives both sounds in one file |
| Cat meow (soft) | P | FS · sokworks | https://freesound.org/people/sokworks/sounds/866536/ | Meow (Gentle) | CC0 | WAV 48 kHz stereo · 1.0 s · 192 KB | yes | One small, gentle meow, already trimmed |
| Keys locking a door | P | FS · nickrandomsounds | https://freesound.org/people/nickrandomsounds/sounds/615287/ | Unlocking - Locking Door and Keys Rattling.wav | CC0 | WAV 48 kHz stereo · 6 s · 1.1 MB | yes | A key turning in the lock with the bunch jingling |
| | A | FS · treesound1951 | https://freesound.org/people/treesound1951/sounds/784468/ | door_lock, closing_the_door, lock, door, keys | CC0 | MP3 44.1 kHz stereo · 7.6 s · 297 KB | yes | Door close plus lock. MP3 source, so it's a little lossy |
| Footsteps on wet cobbles | P | FS · sean_sd2007 | https://freesound.org/people/sean_sd2007/sounds/354129/ | Wet Footsteps Sidewalk : Metro Pass .wav | CC0 | WAV 48 kHz stereo · 1:03 · 17.9 MB | yes | One walker on a wet sidewalk. Not cobbles, but the wet slap is right. Cut 4 steps |
| | A | FS · evsecrets | https://freesound.org/people/evsecrets/sounds/334457/ | Two people's footsteps walking on a wet cement sidewalk.mp3 | CC0 | MP3 44.1 kHz stereo · 38 s · 1.5 MB | yes | Two people after light rain. Pick moments where only one is heard |
| | A (layer) | Kenney · Impact Sounds | https://kenney.nl/assets/impact-sounds | `footstep_concrete_000..004.ogg` | CC0 | OGG · ~6 KB each | no | Dry hard steps. Could be layered with a little splash for "cobbles" |
| Pigeons flutter | P | FS · Kinoton | https://freesound.org/people/Kinoton/sounds/689998/ | Pigeon, Flies Away, Flapping Wings | CC0 | WAV 48 kHz stereo · 5.6 s · 1.6 MB | yes | One pigeon taking off close by: clear wing claps |
| | A | FS · csaszi | https://freesound.org/people/csaszi/sounds/528250/ | pigeons_fly away_ wing flaps_CsG | CC0 | WAV 48 kHz stereo · 4.2 s · 1.2 MB | yes | A few pigeons flying off (ORTF stereo) |
| Tram bell (Prague-style) | P | FS · 16HPanskaWillheimova_Sara (*Tramvaj* pack) | https://freesound.org/people/16HPanskaWillheimova_Sara/sounds/497561/ | 11-3 Tram bell | CC0 | WAV 48 kHz stereo · 2.4 s · 688 KB | yes | A Czech tram bell. The pack is tagged `czech`, `panska` (a Prague school's sound class) but the city isn't stated. Variants 11-1 and 11-2 are in the [same pack](https://freesound.org/people/16HPanskaWillheimova_Sara/packs/27965/) |
| | A | FS · AmazingRolo | https://freesound.org/people/AmazingRolo/sounds/448366/ | tram bell.wav | CC0 | WAV 44.1 kHz stereo · 31 s · 5.3 MB | yes | An Edinburgh tram bell. A different "ding", fallback only |
| Tram passing | P | FS · 16HPanskaWillheimova_Sara (*Tramvaj* pack) | https://freesound.org/people/16HPanskaWillheimova_Sara/sounds/497560/ | 15_Tram passing by.wav | CC0 | WAV 48 kHz stereo · 22.6 s · 6.2 MB | yes | A Czech tram passing by. Matches the bell above |
| | A | FS · ohenavoda | https://freesound.org/people/ohenavoda/sounds/702705/ | Street, steps, tram. Recorded in Prague | CC0 | WAV 48 kHz stereo · 4:47 · 78.9 MB | yes | A real Prague street with a passing tram, cars and voices. Heavy. Could also give wet-street steps |
| Camera shutter (polaroid) | P | FS · JamesMontour | https://freesound.org/people/JamesMontour/sounds/151800/ | Polaroid Spirit 600 "Shutter" (WAV) | CC0 | WAV 48 kHz stereo · 4.2 s · 1.1 MB | yes | The real Polaroid 600 shutter clack |
| Polaroid eject | P | FS · TRP | https://freesound.org/people/TRP/sounds/616967/ | 210506 Polaroid camera click whirr.wav | CC0 | WAV 48 kHz stereo · 21 s · 5.7 MB | yes | A click and then the motor whirr of the print coming out. Covers shutter and eject in one file |
| | A | FS · ExplosiveTamale | https://freesound.org/people/ExplosiveTamale/sounds/540889/ | SX-70 Polaroid Land Camera Operation | CC0 | WAV 48 kHz mono · 33 s · 3.0 MB | yes | A vintage SX-70: shutter and eject motor, a nice retro character |
| Blanket rustle | P | Kenney · RPG Audio | https://kenney.nl/assets/rpg-audio | `cloth1.ogg` … `cloth4.ogg` | CC0 | OGG · 11–17 KB each | no | Short fabric rustles. Four variants |
| | A | FS · IENBA | https://freesound.org/people/IENBA/sounds/850705/ | Soft Fabric Rustle | CC0 | WAV 44.1 kHz mono · 12.5 s · 1.1 MB | yes | Subtle fabric handling on a Sennheiser 416. Very soft |
| Wind / snowstorm (loop) | P | FS · DAVESTALKER | https://freesound.org/people/DAVESTALKER/sounds/331527/ | Wind Howling thru window crack.mp3 | CC0 | MP3 44.1 kHz mono · 40 s · 636 KB | yes | Wind whistling through a window, heard from *inside*: exactly the cozy "storm outside" feel. Smartphone recording, so it may need EQ |
| | A | FS · HECKFRICKER | https://freesound.org/people/HECKFRICKER/sounds/754256/ | Winter Snowstorm Ambience | CC0 | WAV 44.1 kHz stereo · 3:04 · 30.9 MB | yes | A real outdoor snowstorm, cleaned in RX. A rich loop source for the exterior |
| Page turn | P | Kenney · RPG Audio | https://kenney.nl/assets/rpg-audio | `bookFlip1..3.ogg` (+ `bookOpen`, `bookClose`, `bookPlace1..3`) | CC0 | OGG · 9–18 KB each | no | Book page flips. Three variants, plus open, close and place |
| | A | FS · LilMati | https://freesound.org/people/LilMati/sounds/397550/ | Page Turn 03 | CC0 | WAV 44.1 kHz stereo · 1.0 s · 173 KB | yes | A single magazine page turn. A little rougher paper |
| Chalk on board | P | FS · Breviceps | https://freesound.org/people/Breviceps/sounds/447925/ | Write/Draw with chalk on board | CC0 | WAV 44.1 kHz stereo · 0.48 s · 82 KB | yes | One short chalk stroke. No screech |
| | A | FS · Anthousai | https://freesound.org/people/Anthousai/sounds/398491/ | writing - chalk - center 01.wav | CC0 | WAV 96 kHz stereo · 6.1 s · 3.3 MB | yes | Continuous chalk writing. Cut several strokes |

## Recommended download list

Fewer sources first. All items are CC0.

### 1. Kenney — RPG Audio (no login)
- Page: https://kenney.nl/assets/rpg-audio. Licence: CC0 (stated on the page).
- 50 OGG files, about **0.7 MB** of audio (the zip size isn't shown on the page; expect ~1 MB).
- Covers **6 sounds**: blanket rustle (`cloth1-4`), page turn (`bookFlip1-3`), back door creak (`creak1-3`), door to outside (`doorOpen_1-2`, `doorClose_1-4`), lid clink (`metalPot1-2`), till coins (`handleCoins`, `handleCoins2`). It also gives alternatives for the light switch (`metalClick`) and a latch (`metalLatch`).

### 2. Kenney — Impact Sounds (no login, optional)
- Page: https://kenney.nl/assets/impact-sounds. Licence: CC0.
- 130 OGG files, about **1 MB**.
- Only fallbacks: bottle clink (`impactGlass_light_*`), knock (`impactWood_light_*`) and a dry step to layer for cobbles (`footstep_concrete_*`). Skip it if the Freesound files below are accepted.

### 3. Freesound — sokworks, *Morning Coffee* pack + one meow (login)
- Pack: https://freesound.org/people/sokworks/packs/46229/. Every file was checked as CC0.
- Download the single files, not the whole pack: Steaming Milk (Low) 1.5 MB, Steaming Milk (Finish) 0.3 MB, Espresso Machine Empty Shot Pull 1.4 MB, Knock Box 2 0.04 MB, Meow (Gentle) 0.2 MB.
- Total **≈ 3.4 MB**. Covers warm-up, sputter, knock and clean, and the meow.

### 4. Freesound — *Tramvaj* pack by 16HPanskaWillheimova_Sara (login)
- Pack: https://freesound.org/people/16HPanskaWillheimova_Sara/packs/27965/. Licence: CC0.
- Files: 11-3 Tram bell 0.7 MB and 15_Tram passing by 6.2 MB. The sizes of the bell variants 11-1 and 11-2 weren't checked.
- Total **≈ 6.9 MB**. Covers the tram bell and the tram passing, both recorded in Czechia.

### 5. Freesound — single files (login), all CC0

| Sound | File | Size |
|---|---|---|
| Purge | PROpolisher — steam wand purge (824466) | 16.9 MB |
| Soup loop | alienistcog — simmering-sauce.aif (124584) | 17.4 MB |
| Oven ding | dland — Kitchen Timer — Done! (149506) | 0.8 MB |
| Oven door | bolkmar — Oven door - open and close (503477) | 1.0 MB |
| Fridge door | The_Isot_is_Back — Fridge Door Open & Close (643537) | 0.7 MB |
| Fridge bottle | Luisa_Sanchez — clinking of bottles (816964) | 0.1 MB |
| Trash bag | semccab — trashbag.wav (154371) | 3.7 MB |
| Light switch | funkysandwich — light_switch10 (870289) | 0.1 MB |
| Rain on awning | kyles — rain light on awning… (454130) | 8.6 MB |
| Phone buzz | kyles — cell phone smartphone vibrate single (637346) | 0.1 MB |
| Cat purr | clareboots — Cat-purr.wav (553962) | 12.0 MB |
| Keys in lock | nickrandomsounds — Unlocking - Locking Door… (615287) | 1.1 MB |
| Wet footsteps | sean_sd2007 — Wet Footsteps Sidewalk (354129) | 17.9 MB |
| Pigeons | Kinoton — Pigeon, Flies Away (689998) | 1.6 MB |
| Polaroid shutter | JamesMontour — Polaroid Spirit 600 "Shutter" (151800) | 1.1 MB |
| Polaroid eject | TRP — Polaroid camera click whirr (616967) | 5.7 MB |
| Wind loop | DAVESTALKER — Wind Howling thru window crack (331527) | 0.6 MB |
| Chalk | Breviceps — Write/Draw with chalk on board (447925) | 0.1 MB |

Subtotal **≈ 89.5 MB**.

**Total download: ≈ 100 MB** (Kenney RPG Audio ~1 MB + Freesound ≈ 99.8 MB across 25 files; +1 MB if Impact Sounds is taken too).
Most of that is the raw field recordings. After cutting, mono downmix and OGG/WAV export, the shipped files should come to roughly 3–5 MB.

## Gaps / caveats

- **Footsteps on wet cobbles**: no CC0 recording of *cobblestones* specifically. The candidates are wet sidewalk or concrete. The Prague street recording (ohenavoda, 78.9 MB) may contain usable steps.
- **Prague tram bell**: the *Tramvaj* pack is Czech (tags `czech`, `panska`), but the page doesn't state the city. Listen before committing; the Edinburgh bell is only a fallback.
- **Loops**: none of the rain, soup, wind or purr files is a ready seamless loop. Each needs a loop cut with a crossfade.
- **Blizzard loop (m0ments)** was rejected because the audio was taken from a commercial SFX LP, despite its CC0 tag.
- Some sources are MP3 (wind, keys alternative, wet-steps alternative). They're usable, but prefer WAV/FLAC where both exist.
- After downloading, record each file in `assets/audio/CREDITS.txt` (title, author, URL, CC0) even though no credit is required.
