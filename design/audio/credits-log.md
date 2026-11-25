# Журнал генерации и лицензии ElevenLabs

## Статус на 2026-11-25

**Генерация не запускалась: API-ключ не найден.** Переменной `ELEVENLABS_API_KEY` нет в окружении, файла `night-shift/.env` тоже нет.

Что нужно сделать владельцу:
1. Создать `night-shift/.env` со строкой `ELEVENLABS_API_KEY=...`. `.env` уже добавлен в `night-shift/.gitignore`.
2. Запустить из корня `night-shift`: `python3 -I design/audio/generate.py`.
3. Скрипт сам допишет машинный лог в `design/audio/generation-log.jsonl`: файл, длительность, параметры, промпт и расход кредитов по счётчику подписки. Сводку перенести в таблицу ниже.

Использовано кредитов: **0**.

## Запланированная первая партия

Промпты полностью лежат в `prompts.json`. Исходник везде: `output_format=mp3_44100_128`. Затем ffmpeg кодирует в OGG Vorbis q6 стерео или WAV 16-бит, 44.1 кГц.

### Музыка: `POST /v1/music`, `model_id=music_v2_5`, `force_instrumental=true`
| id | Файл | Длина (запрос) | Кратко о промпте | Фактическая длина | Кредиты |
|---|---|---|---|---|---|
| menu_theme | `assets/audio/music/mus_menu_theme.ogg` | 75 с | фетровое пианино + Rhodes, винил, щётки, ~75 BPM, D minor, мягкий конец | — | — |
| night_calm_loop | `assets/audio/music/mus_night_calm_loop.ogg` | 105 с | lo-fi hip-hop, Rhodes/нейлон, место для дождя, ровная энергия, ~72 BPM | — | — |
| rush_loop | `assets/audio/music/mus_rush_loop.ogg` | 75 с | jazz-hop, свинг, шагающий бас, вибрафон, труба, ~96 BPM | — | — |

### SFX: `POST /v1/sound-generation`, модель по умолчанию (`eleven_text_to_sound_v2`)
| id | Файл | Длина | Луп | Кратко | Факт | Кредиты |
|---|---|---|---|---|---|---|
| door_bell | `assets/audio/sfx/sfx_door_bell.wav` | 1.5 с | — | латунный колокольчик над дверью | — | — |
| coffee_machine | `assets/audio/sfx/sfx_coffee_machine.wav` | 6 с | — | эспрессо: помпа, пролив, пар | — | — |
| cash_register | `assets/audio/sfx/sfx_cash_register.wav` | 1.5 с | — | механическая касса «ка-чинг» и ящик | — | — |
| cup_saucer | `assets/audio/sfx/sfx_cup_saucer.wav` | 1 с | — | чашка на блюдце, один «клинк» | — | — |
| footsteps_tile | `assets/audio/sfx/sfx_footsteps_tile.wav` | 3 с | — | ~6 шагов по плитке | — | — |
| rain_window_loop | `assets/audio/sfx/amb_rain_window_loop.ogg` | 30 с | да | дождь по стеклу ночью, изнутри | — | — |
| train_pass_far | `assets/audio/sfx/amb_train_pass_far.ogg` | 12 с | — | поезд у станции, слышно из кафе | — | — |
| power_outage | `assets/audio/sfx/sfx_power_outage.wav` | 3 с | — | гул холодильника и неона обрывается, реле | — | — |
| ui_click | `assets/audio/sfx/ui_click.wav` | 0.5 с | — | мягкий деревянный UI-тик | — | — |
| event_alert | `assets/audio/sfx/ui_alert_event.wav` | 1.5 с | — | двухнотный стеклянный колокольчик | — | — |

**Оценка расхода:**
- SFX по документации стоят 40 кредитов за секунду при заданной длительности. Всего 60.5 с ≈ **2 420 кредитов**.
- Цену музыки (3 трека, 4 мин 15 с) документация не называет. Скрипт измерит её по счётчику подписки.
- Чтобы сэкономить, начните с `--kind sfx` или `--only menu_theme` и прослушайте результат до генерации остального.

## Лицензии ElevenLabs (проверено 2026-11-25)

Источники:
- [Eleven Music — model specific terms](https://elevenlabs.io/eleven-music-model-specific-terms) (обновлено 09.10.2026);
- [Help center: Can I publish the content I generate?](https://elevenlabs.io/docs/help-center/legal/can-i-publish-the-content-i-generate-on-the-platform);
- [Music capability](https://elevenlabs.io/docs/overview/capabilities/music);
- [Sound effects capability](https://elevenlabs.io/docs/overview/capabilities/sound-effects).

### Общее (вся платформа, включая SFX)
- **Free:** коммерческой лицензии нет. При публикации нужна атрибуция («elevenlabs.io» или «11.ai» в названии).
- **Любой платный тариф (Starter и выше):** коммерческая лицензия включена. Исключение — функции в статусе Beta: их результат нельзя использовать коммерчески и в продакшене.
- Отдельных условий для Sound Effects на страницах help center не нашлось. Действуют общие условия плюс Service-Specific Terms. Перед релизом стоит перечитать их.

### Музыка (Eleven Music, условия для аккаунтов с 21.11.2025)
| Тариф | API | Генерация / скачивание в месяц | Атрибуция | Права |
|---|---|---|---|---|
| Free | **нет** | 11 мин / скачивание запрещено | нужна («Eleven Music») | личное использование |
| Starter | да | 17 / 30 мин | не нужна | коммерция, кроме кино/ТВ/радио и «Studio Games». Стриминг-дистрибуция запрещена |
| Creator | да | 62 / 250 мин | не нужна | то же, стриминг разрешён |
| Pro | да | 304 / 500 мин | не нужна | то же. Lossless WAV и mp3 192 кбит/с — с Pro |
| Scale / Business | да | 1 100–4 800 мин | не нужна | то же. Ограничения по числу сотрудников (<10 / <50) |
| Enterprise Music | да | custom | — | все виды коммерческого использования |

Важно для игры:
- **Music API работает только на платных тарифах.** На Free скрипт получит ошибку.
- В игре можно использовать музыку на тарифах от Starter до Business, пока игра считается **«Indie Game»**: выручка самой игры < $500 000 и годовая выручка разработчика < $1 000 000. Если порог превышен, есть 60 дней, чтобы перейти на тариф, разрешающий Studio Games (Enterprise Music). Иначе использование надо прекратить. Копии, распространённые до этого срока, использовать можно.
- **Запрещено на всех тарифах, кроме Enterprise:** создавать или распространять «музыкальную библиотеку», то есть выкладывать сгенерированные треки как отдельный пак для других, и перепродавать модель. Поэтому треки поставляются только внутри игры. Отдельный «саундтрек-пак» на itch.io или Steam без разрешения ElevenLabs не выпускать.
- Выпуск OST на стриминги (Spotify и т. п.) разрешён с Creator и выше.
- Модель `music_v1` объявлена устаревшей и будет удалена 31.01.2027. Скрипт использует `music_v2_5`.

## История запусков

| Дата | Что | Кредиты | Примечание |
|---|---|---|---|
| 2026-11-25 | — | 0 | ключ не найден, подготовлены промпты и скрипт |
