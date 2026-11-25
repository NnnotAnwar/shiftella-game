# Free candidates from the web

Nothing has been downloaded — this is a shortlist for the owner to decide on. Checked on 2026-11-25.
Licences were taken from the resources' pages. OpenGameArt blocks automated reading, so its data comes from search results. **Before downloading, open the page and re-check the licence.** The same author sometimes publishes different files under different licences.

Order of preference: **CC0** (no attribution needed) → **CC BY** (a credit in the credits screen is required). We don't take CC BY-SA, NC or "no licence stated".

None of these are used in v0.1.0; the shipped audio was generated with ElevenLabs (see `credits-log.md`).

## Recommended

| # | What | Author | Licence | Size / length | Why it fits |
|---|---|---|---|---|---|
| 1 | [Albi-Madeleine — Railroad crossing, southern-going train, 22:30](https://freesound.org/people/Sadiquecat/sounds/753071/) | Sadiquecat (Freesound) | **CC0** | WAV 96 kHz/32-bit, 4:16, 187.9 MB | A real night-time crossing at a small station, recorded at 22:30. 2–3 variants of `amb_train_pass_far` can be cut from the long recording, with real rail joints and the crossing bell. A heavy file: download once, cut it, and don't put the original in the repository. |
| 2 | [Coffee Shop, 6 People, low chatter, refrigerator, barista movement](https://freesound.org/s/403065/) | AwenAudio (Freesound) | **CC0** | WAV 48 kHz/24-bit, 2:21, 38.7 MB | A ready-made `amb_cafe_murmur_loop` and room tone: a quiet murmur with no intelligible words, a fridge, a barista, guests coming in. Exactly our scene. |
| 3 | [Interface Sounds](https://kenney.nl/assets/interface-sounds) | Kenney | **CC0** | 100 files (size not stated) | A full UI set (click, hover, back, confirm, error, switch) without spending credits. Pick the softest, most "wooden" ones. |
| 4 | [Rain (loopable)](https://opengameart.org/content/rain-loopable) | Ylmir (OpenGameArt) | **CC0** | 4 loops of 25–45 s | Ready seamless rain loops for `amb_rain_*`: several intensities for the weather layers. |
| 5 | [Rain on Window Loop](https://opengameart.org/content/rain-on-window-loop) | alxl (OpenGameArt) | **CC0** (+ others to choose from) | ~11 s | Rain specifically on glass, heard from inside. Short; good as a second layer over no. 4. |
| 6 | [Impact Sounds](https://kenney.nl/assets/impact-sounds) | Kenney | **CC0** | 130 files (size not stated) | Foley impacts: could cover footsteps, chairs, dishes (`sfx_footsteps_*`, `sfx_chair_move`, `sfx_plate_set`). The page doesn't list the contents — needs listening. |
| 7 | [Lofi again](https://opengameart.org/content/lofi-again) | omfgdude (OpenGameArt) | **CC0** | 4 tracks and an ambience (size not shown) | Lo-fi hip-hop, tagged loop/chill. A fallback for the calm night or the day, if the generated tracks don't work out. |
| 8 | [Dimly lit](https://opengameart.org/content/dimly-lit) | iamoneabe (OpenGameArt) | **CC0** | a loop (size not shown) | Lo-fi electro-blues that loops. Suits the deep night (3–5): a "dimly lit" hall. |
| 9 | [FREE Music Pack 14: Lo-fi / Chill](https://retroindiejosh.itch.io/free-music-pack-14-lo-fi-chill) | Retro Indie Josh (itch.io) | **CC BY 4.0** (credit required; the credit text is on the page) | 5 tracks; OGG 26 MB / WAV 172 MB | Already in OGG, which macroquad plays directly. Lo-fi/chill background music for the calm night and planning. |
| 10 | [FREE MUSIC: Ambient LoFi Jazz (MP3 & MIDI)](https://rustedstudio.itch.io/free-music-ambient-lofi-jazz-mp3-midi) | Rusted Music Studio (itch.io) | **CC BY 4.0** (on the page; search results also showed BY-SA — re-check) | 23 tracks, a 68 MB `.rar` (MP3 + MIDI) | Slow ambient lo-fi jazz "for restaurants and bars". Comes with MIDI, which could be used for variations or stems. The MP3s would need converting to OGG. |
| 11 | [FREE Music Pack (Lo-fi, Indie, Metal, Horror, Orchestral)](https://ailascott.itch.io/free-music-pack-lo-fi-indie-metal-orchestral) | Aila Scott (itch.io) | **CC BY 4.0** (credit and link; without attribution it costs $5) | ZIP 330 MB, WAV 48 kHz, every track loops | Every track loops out of the box. We'd only need the lo-fi part; the rest (metal, horror) doesn't fit the style. A heavy archive. |

## Not recommended (found, but there are problems)

| What | Why |
|---|---|
| [Quiet café](https://freesound.org/s/417682/) (arnaud coutancier) | CC BY-NC 3.0: a non-commercial licence, not allowed in a game that is sold. |
| [lofi world vol 1](https://kummel.itch.io/lofi-world-vol-1) (kummel), 7 tracks, 159 MB | No licence stated on the page. |
| [Cozy Game / Lofi Music Collection](https://cristiano-leone-ost.itch.io/freeloficozy) (Cristiano Leone / Redlight_Chill), 22 tracks, ~951 MB WAV | No formal licence. Permission is given only in the comments, and the credit requirements there contradict each other. |
| [lofi compilation](https://opengameart.org/content/lofi-compilation) (TAD) | According to the comments, the track "Morning rain" received an automatic copyright claim (a match with someone else's track). Risky. |

## If we use something
- Rename the files to the convention in `sound-design.md` (`amb_`, `sfx_`, `ui_`, `mus_`) and normalise them like the generated ones.
- For CC BY, create `assets/audio/CREDITS.txt` (title, author, URL, licence) and show the same lines in the game's credits.
- For CC0 a credit isn't required, but record it in CREDITS.txt anyway: it makes the file's origin easy to prove later.
- Unlike the ElevenLabs output, CC0 / CC BY files *may* be committed to the public repo (with their credits).
