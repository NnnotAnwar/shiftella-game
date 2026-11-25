# Generation log and ElevenLabs licences

> **The generated audio is not in the public repository.** ElevenLabs' terms let us use the tracks and
> SFX inside the game but not redistribute them on their own (see "Licences" below), so `assets/audio/` and
> `design/audio/raw/` ship only inside release builds. The game runs without them. To generate your own
> copies, put your own key into `.env` in the repository root (`ELEVENLABS_API_KEY=...`, git-ignored) and run
> `python3 -I design/audio/generate.py` — details in [`sound-design.md` §8](sound-design.md#8-how-to-generate).
> Never commit a key.

## Status on 2026-11-25

**The first batch was generated on 2026-11-25** (3 music tracks + 10 SFX, 13 files, all successful). The machine
log with file, duration, parameters, prompt and credits per item is in `generation-log.jsonl`.

The credits counter read from the subscription endpoint lagged behind during the run, so the per-item credit
numbers in the log are unreliable (most show 0, one shows the whole batch). Check the real usage in the
ElevenLabs dashboard.

## The first batch

The full prompts are in `prompts.json` (they were written while the game still had a pixel-art working
title; the wording only steers the mood). Source format everywhere: `output_format=mp3_44100_128`. Then
ffmpeg encodes to OGG Vorbis q6 stereo or 16-bit WAV, 44.1 kHz.

### Music: `POST /v1/music`, `model_id=music_v2_5`, `force_instrumental=true`
| id | File | Length (requested) | The prompt in brief | Actual length |
|---|---|---|---|---|
| menu_theme | `assets/audio/music/mus_menu_theme.ogg` | 75 s | felt piano + Rhodes, vinyl, brushes, ~75 BPM, D minor, soft ending | 75.05 s |
| night_calm_loop | `assets/audio/music/mus_night_calm_loop.ogg` | 105 s | lo-fi hip-hop, Rhodes/nylon guitar, room for the rain, steady energy, ~72 BPM | 105.04 s |
| rush_loop | `assets/audio/music/mus_rush_loop.ogg` | 75 s | jazz-hop, swing, walking bass, vibraphone, trumpet, ~96 BPM | 75.05 s |

### SFX: `POST /v1/sound-generation`, the default model (`eleven_text_to_sound_v2`)
| id | File | Length | Loop | In brief | Actual |
|---|---|---|---|---|---|
| door_bell | `assets/audio/sfx/sfx_door_bell.wav` | 1.5 s | — | a brass bell above the door | 1.48 s |
| coffee_machine | `assets/audio/sfx/sfx_coffee_machine.wav` | 6 s | — | espresso: pump, pour, steam | 6.0 s |
| cash_register | `assets/audio/sfx/sfx_cash_register.wav` | 1.5 s | — | a mechanical register "ka-ching" and the drawer | 1.48 s |
| cup_saucer | `assets/audio/sfx/sfx_cup_saucer.wav` | 1 s | — | a cup on a saucer, one "clink" | 1.0 s |
| footsteps_tile | `assets/audio/sfx/sfx_footsteps_tile.wav` | 3 s | — | ~6 footsteps on tiles | 3.0 s |
| rain_window_loop | `assets/audio/sfx/amb_rain_window_loop.ogg` | 30 s | yes | rain on the glass at night, heard from inside | 30.0 s |
| train_pass_far | `assets/audio/sfx/amb_train_pass_far.ogg` | 12 s | — | a train by the station, heard from the café | 12.0 s |
| power_outage | `assets/audio/sfx/sfx_power_outage.wav` | 3 s | — | the hum of the fridge and the sign cuts out, a relay | 3.0 s |
| ui_click | `assets/audio/sfx/ui_click.wav` | 0.5 s | — | a soft wooden UI tick | 0.48 s |
| event_alert | `assets/audio/sfx/ui_alert_event.wav` | 1.5 s | — | a two-note glass bell | 1.48 s |

**Cost estimate (from before the run):**
- According to the documentation, SFX cost 40 credits per second when a duration is set. 60.5 s in total ≈ **2 420 credits**.
- The documentation doesn't state the price of music (3 tracks, 4 min 15 s); the script tries to measure it from the subscription counter.
- To save credits when you regenerate, start with `--kind sfx` or `--only menu_theme` and listen before generating the rest.

## ElevenLabs licences (checked on 2026-11-25)

Sources:
- [Eleven Music — model specific terms](https://elevenlabs.io/eleven-music-model-specific-terms) (updated 2026-10-09);
- [Help center: Can I publish the content I generate?](https://elevenlabs.io/docs/help-center/legal/can-i-publish-the-content-i-generate-on-the-platform);
- [Music capability](https://elevenlabs.io/docs/overview/capabilities/music);
- [Sound effects capability](https://elevenlabs.io/docs/overview/capabilities/sound-effects).

*This is a summary for our own planning, not legal advice — read the terms yourself before you publish anything you generate.*

### General (the whole platform, including SFX)
- **Free:** no commercial licence. Publishing requires attribution ("elevenlabs.io" or "11.ai" in the title).
- **Any paid plan (Starter and up):** a commercial licence is included. The exception is features in Beta: their output may not be used commercially or in production.
- The help center has no separate terms for Sound Effects. The general terms plus the Service-Specific Terms apply. Re-read them before a release.

### Music (Eleven Music, terms for accounts from 2025-11-21)
| Plan | API | Generation / download per month | Attribution | Rights |
|---|---|---|---|---|
| Free | **no** | 11 min / downloads not allowed | required ("Eleven Music") | personal use |
| Starter | yes | 17 / 30 min | not required | commercial, except film/TV/radio and "Studio Games". No streaming distribution |
| Creator | yes | 62 / 250 min | not required | the same, streaming allowed |
| Pro | yes | 304 / 500 min | not required | the same. Lossless WAV and 192 kbps mp3 from Pro |
| Scale / Business | yes | 1 100–4 800 min | not required | the same. Limits on team size (<10 / <50) |
| Enterprise Music | yes | custom | — | every kind of commercial use |

What matters for the game:
- **The Music API works only on paid plans.** On Free the script gets an error.
- Music can be used in the game on plans from Starter to Business while the game counts as an **"Indie Game"**: the game's own revenue < $500 000 and the developer's annual revenue < $1 000 000. If the threshold is exceeded, there are 60 days to move to a plan that allows Studio Games (Enterprise Music); otherwise the use must stop. Copies distributed before that deadline may still be used.
- **Not allowed on any plan except Enterprise:** creating or distributing a "music library" — publishing the generated tracks as a stand-alone pack for others — and reselling the model. **That is why the tracks ship only inside the game and are not in the public repository.** Don't release a separate "soundtrack pack" on itch.io or Steam without ElevenLabs' permission.
- Releasing an OST on streaming services (Spotify etc.) is allowed from Creator up.
- The `music_v1` model is deprecated and will be removed on 2027-01-31. The script uses `music_v2_5`.

## Run history

| Date | What | Credits | Note |
|---|---|---|---|
| 2026-11-25 | — | 0 | no key found yet; prompts and script prepared |
| 2026-11-25 | first batch: 3 music + 10 SFX | see the dashboard | all 13 OK; the in-script counter lagged (see "Status") |
