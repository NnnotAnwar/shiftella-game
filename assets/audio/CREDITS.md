# Audio credits

The files in `assets/audio/cc0/` are cut from recordings released under
[CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) (public domain dedication).
Credit is not required; it is given here anyway, with thanks. Every licence was checked on the
sound's own page (Freesound) or in the pack's `License.txt` (Kenney) on 2026-11-26.

The rest of `assets/audio/` (music and the original sound effects) is generated with ElevenLabs.
It is not in this repository and ships only inside release builds.

## How the files were made

All files were cut with ffmpeg (`atrim`, `afade`, `acrossfade`, `highpass` / `lowpass`,
`alimiter`, `ebur128`). Unless a line says otherwise, each one was:

- **trimmed** to the moment the game needs, with short fades in and out so it starts and stops
  without a click;
- **mixed down to mono** and resampled to 44.1 kHz;
- **softened** with a gentle low-pass (9 kHz, or 6.5 kHz for brighter recordings) and a
  high-pass against rumble;
- **normalised** to −24 LUFS integrated (the loudness of the game's other sound effects), with
  a gentle limiter keeping the true peak under −1.5 dBTP (−3 dBTP for the `.ogg` files).

One-shots are 16-bit WAV. Loops (`amb_*`) and the long tram pass are Ogg Vorbis (ffmpeg's
built-in encoder, quality 3, stereo). Each loop is made seamless: the last second crossfades
into the moment that comes right before the loop's first sample in the original recording.

## Files

Freesound sounds are listed as *title* by *author* (Freesound id).

### The coffee machine

| File | Source | Licence | What was changed |
|---|---|---|---|
| `sfx_machine_hiss.wav` | *Steaming Milk (Low)* by sokworks (868483), https://freesound.org/people/sokworks/sounds/868483/ | CC0 1.0 | 1.4 s from 4.0 s, mono, low-passed, normalised |
| `sfx_machine_purge.wav` | *Coffee Machine, cup rinse, boiler hum, steam wand purge* by PROpolisher (824466), https://freesound.org/people/PROpolisher/sounds/824466/ | CC0 1.0 | the purge burst (1.9 s from 51.5 s), mono, low-passed, normalised |
| `sfx_machine_sputter.wav` | *Steaming Milk (Finish)* by sokworks (868481), https://freesound.org/people/sokworks/sounds/868481/ | CC0 1.0 | first 1.3 s, mono, low-passed, normalised |
| `sfx_shot_pull.wav` | *Espresso Machine Empty Shot Pull* by sokworks (868497), https://freesound.org/people/sokworks/sounds/868497/ | CC0 1.0 | first 1.8 s (the click and the pump starting), mono, normalised |
| `sfx_group_clean.wav` | *Knock Box 2* by sokworks (868472), https://freesound.org/people/sokworks/sounds/868472/ + *Espresso Machine Empty Shot Pull* by sokworks (868497), https://freesound.org/people/sokworks/sounds/868497/ | CC0 1.0 | the knock, then 1.25 s of the blank shot's end (from 4.4 s, the pump stopping) mixed in after 0.28 s; mono, normalised |
| `sfx_knock_box.wav` | *Knock Box 2* by sokworks (868472), https://freesound.org/people/sokworks/sounds/868472/ | CC0 1.0 | whole file (0.19 s), mono, normalised |

### The stove, the oven, the fridge

| File | Source | Licence | What was changed |
|---|---|---|---|
| `amb_soup_simmer.ogg` | *simmering-sauce.aif* by alienistcog (124584), https://freesound.org/people/alienistcog/sounds/124584/ | CC0 1.0 | a 14 s loop from 60 s (1 s crossfade), kept stereo, low-passed, normalised |
| `sfx_pot_lid.wav` | `metalPot1.ogg` from *RPG Audio* by Kenney (kenney.nl), https://kenney.nl/assets/rpg-audio | CC0 1.0 | first 1.2 s, mono, low-passed, normalised (turned down) |
| `sfx_pot_stir.wav` | `metalPot2.ogg` from *RPG Audio* by Kenney, https://kenney.nl/assets/rpg-audio | CC0 1.0 | first 0.9 s, mono, low-passed, normalised (turned down) |
| `sfx_oven_ding.wav` | *Kitchen Timer - Done!* by dland (149506), https://freesound.org/people/dland/sounds/149506/ | CC0 1.0 | only the chime (2.1 s from 2.06 s, without the ticking), long fade-out, normalised |
| `sfx_oven_door.wav` | *Oven door - open and close* by bolkmar (503477), https://freesound.org/people/bolkmar/sounds/503477/ | CC0 1.0 | the opening (0.05–0.8 s) joined to the closing (2.95–3.65 s), mono, normalised |
| `sfx_fridge_door.wav` | *Fridge Door Open & Close* by The_Isot_is_Back (643537), https://freesound.org/people/The_Isot_is_Back/sounds/643537/ | CC0 1.0 | 1.77 s from 0.08 s, mono, normalised |
| `sfx_bottle_clink.wav` | *clinking of bottles* by Luisa_Sanchez (816964), https://freesound.org/people/Luisa_Sanchez/sounds/816964/ | CC0 1.0 | first 0.6 s, a stronger low-pass (5 kHz: a piezo recording), normalised |

### Closing time

| File | Source | Licence | What was changed |
|---|---|---|---|
| `sfx_till_coins.wav` | `handleCoins.ogg` from *RPG Audio* by Kenney, https://kenney.nl/assets/rpg-audio | CC0 1.0 | whole file, mono, normalised |
| `sfx_trash_bag.wav` | *trashbag.wav* by semccab (154371), https://freesound.org/people/semccab/sounds/154371/ | CC0 1.0 | 1.5 s from 4.6 s, high-passed at 250 Hz (room rumble), mono, normalised |
| `sfx_door_outside.wav` | `doorOpen_1.ogg` + `doorClose_2.ogg` from *RPG Audio* by Kenney, https://kenney.nl/assets/rpg-audio | CC0 1.0 | the opening, then the closing 0.7 s later; mono, normalised |
| `sfx_lamp_switch.wav` | *light_switch10* by funkysandwich (870289), https://freesound.org/people/funkysandwich/sounds/870289/ | CC0 1.0 | whole file (0.15 s), 96 → 44.1 kHz, mono, normalised |

### The walk home

| File | Source | Licence | What was changed |
|---|---|---|---|
| `sfx_keys_lock.wav` | *Unlocking - Locking Door and Keys Rattling.wav* by nickrandomsounds (615287), https://freesound.org/people/nickrandomsounds/sounds/615287/ | CC0 1.0 | the locking half (2.2 s from 2.45 s), mono, low-passed, normalised |
| `amb_wet_steps.ogg` | `footstep00.ogg` … `footstep09.ogg` from *RPG Audio* by Kenney, https://kenney.nl/assets/rpg-audio | CC0 1.0 | 16 recorded shoe steps at a slow stroll (0.56 s apart, the takes shuffled, every other one 2 dB softer) into an 8.96 s loop; mono, normalised |
| `sfx_pigeons.wav` | *Pigeon, Flies Away, Flapping Wings* by Kinoton (689998), https://freesound.org/people/Kinoton/sounds/689998/ | CC0 1.0 | 2.3 s from 0.35 s, mono, normalised |
| `sfx_tram_bell.wav` | *11-3 Tram bell* by 16HPanskaWillheimova_Sara (497561), https://freesound.org/people/16HPanskaWillheimova_Sara/sounds/497561/ | CC0 1.0 | 1.6 s from 0.22 s, mono, low-passed, normalised |
| `sfx_tram_pass.ogg` | *15_Tram passing by.wav* by 16HPanskaWillheimova_Sara (497560), https://freesound.org/people/16HPanskaWillheimova_Sara/sounds/497560/ | CC0 1.0 | 7 s from 8 s (the tram coming close and going by), fade in 1.2 s / out 2.5 s, mono, normalised |

### The back step

| File | Source | Licence | What was changed |
|---|---|---|---|
| `sfx_door_creak.wav` | `creak1.ogg` from *RPG Audio* by Kenney, https://kenney.nl/assets/rpg-audio | CC0 1.0 | whole file, mono, normalised |
| `amb_rain_awning.ogg` | *rain light on awning close drops on fabric.flac* by kyles (454130), https://freesound.org/people/kyles/sounds/454130/ | CC0 1.0 | a 15 s loop from 20 s (1 s crossfade), kept stereo, low-passed, normalised |
| `sfx_phone_buzz.wav` | *cell phone smartphone vibrate single.flac* by kyles (637346), https://freesound.org/people/kyles/sounds/637346/ | CC0 1.0 | two buzzes (1.7 s from 1.15 s), low-passed at 6 kHz, normalised |
| `amb_cat_purr.ogg` | *Cat-purr.wav* by clareboots (553962), https://freesound.org/people/clareboots/sounds/553962/ | CC0 1.0 | a 10.5 s loop from 62.5 s (a steady stretch without scratching, 1 s crossfade), low-passed at 5 kHz, normalised |
| `sfx_cat_meow.wav` | *Meow (Gentle)* by sokworks (866536), https://freesound.org/people/sokworks/sounds/866536/ | CC0 1.0 | whole file (1 s), mono, normalised |

### The storm

| File | Source | Licence | What was changed |
|---|---|---|---|
| `amb_storm.ogg` | *Wind Howling thru window crack.mp3* by DAVESTALKER (331527), https://freesound.org/people/DAVESTALKER/sounds/331527/ | CC0 1.0 | a 16 s loop from 6.5 s (1.5 s crossfade), band-limited 70 Hz–7 kHz (a phone recording), normalised |

### Books, blankets, chalk, the polaroid

| File | Source | Licence | What was changed |
|---|---|---|---|
| `sfx_book_open.wav` | `bookOpen.ogg` from *RPG Audio* by Kenney, https://kenney.nl/assets/rpg-audio | CC0 1.0 | whole file, mono, normalised (turned up) |
| `sfx_page_flip.wav` | `bookFlip1.ogg` from *RPG Audio* by Kenney, https://kenney.nl/assets/rpg-audio | CC0 1.0 | whole file, mono, normalised |
| `sfx_blanket.wav` | `cloth1.ogg` from *RPG Audio* by Kenney, https://kenney.nl/assets/rpg-audio | CC0 1.0 | whole file, mono, low-passed, normalised |
| `sfx_chalk.wav` | *Write/Draw with chalk on board* by Breviceps (447925), https://freesound.org/people/Breviceps/sounds/447925/ | CC0 1.0 | whole file (0.48 s), mono, low-passed, normalised |
| `photo_shutter.wav` | *Polaroid Spirit 600 "Shutter" (WAV)* by JamesMontour (151800), https://freesound.org/people/JamesMontour/sounds/151800/ | CC0 1.0 | the click and the clack (0.7 s from 0.36 s), mono, normalised |
| `photo_eject.wav` | *210506 Polaroid camera click whirr.wav* by TRP (616967), https://freesound.org/people/TRP/sounds/616967/ | CC0 1.0 | the motor starting (2.05–2.9 s) joined to the motor stopping (7.4–8.15 s), so the print comes out in 1.5 s; mono, normalised |

## Kenney packs

- *RPG Audio* by Kenney Vleugels (kenney.nl), https://kenney.nl/assets/rpg-audio — CC0 1.0.
- *Impact Sounds* by Kenney Vleugels (kenney.nl), https://kenney.nl/assets/impact-sounds — CC0 1.0.
