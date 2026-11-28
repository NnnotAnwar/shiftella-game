//! Mock view models for every hands-on widget (what the integration will fill from the
//! sim and content), localised through [`s`].

use macroquad::prelude::*;

use shiftella_game::ui::handson::*;

use crate::lang::{Lang, s};

pub fn hex(v: u32) -> Color {
    Color::from_hex(v)
}

pub fn taste_names(l: Lang) -> TasteNames {
    let n = |v: [&str; 4]| s(l, v);
    TasteNames {
        names: [
            n(["Warm", "Teplé", "Тепле", "Тёплое"]),
            n(["Cold", "Studené", "Холодне", "Холодное"]),
            n(["Sweet", "Sladké", "Солодке", "Сладкое"]),
            n(["Bitter", "Hořké", "Гірке", "Горькое"]),
            n(["Strong", "Silné", "Міцне", "Крепкое"]),
            n(["Mild", "Jemné", "М'яке", "Мягкое"]),
            n(["Comforting", "Útěšné", "Затишне", "Уютное"]),
            n(["Fancy", "Slavnostní", "Святкове", "Праздничное"]),
            n(["Quick", "Rychlé", "Швидке", "Быстрое"]),
            n(["Filling", "Sytý", "Ситне", "Сытное"]),
            n(["Spiced", "Kořeněné", "Пряне", "Пряное"]),
            n(["Fresh", "Svěží", "Свіже", "Свежее"]),
        ],
    }
}

// ---------------------------------------------------------------------------
// Dishes
// ---------------------------------------------------------------------------

pub fn black() -> Dish {
    Dish::new(DishGlyph::Espresso, hex(0x3B2416)).steam(true)
}
pub fn cappuccino() -> Dish {
    Dish::new(DishGlyph::Cup, hex(0xB07A4C)).steam(true)
}
pub fn lemon_tea() -> Dish {
    Dish::new(DishGlyph::TeaGlass, hex(0xC9772E)).steam(true)
}
pub fn hot_choc() -> Dish {
    Dish::new(DishGlyph::Mug, hex(0x6B3A24)).steam(true)
}
pub fn linden() -> Dish {
    Dish::new(DishGlyph::TeaGlass, hex(0xE2B655)).steam(true)
}
pub fn chai() -> Dish {
    Dish::new(DishGlyph::Mug, hex(0xC99263)).steam(true)
}
pub fn svarak() -> Dish {
    Dish::new(DishGlyph::MulledGlass, hex(0x8E1F35)).steam(true)
}
pub fn tonic() -> Dish {
    Dish::new(DishGlyph::ColdGlass, hex(0xD9A65A))
}
pub fn soup() -> Dish {
    Dish::new(DishGlyph::Bowl, hex(0xE6C278)).steam(true)
}
pub fn kulajda() -> Dish {
    Dish::new(DishGlyph::Bowl, hex(0xF1E3C2)).steam(true)
}
pub fn topinka() -> Dish {
    Dish::new(DishGlyph::Plate, hex(0xD9A04E))
}
pub fn kolac() -> Dish {
    Dish::new(DishGlyph::Pastry, hex(0x7A2E4E))
}
pub fn medovnik() -> Dish {
    Dish::new(DishGlyph::Cake, hex(0xC8864A))
}
pub fn varenyky() -> Dish {
    Dish::new(DishGlyph::Dumplings, hex(0x9A6A3A))
}

// ---------------------------------------------------------------------------
// Order card + rail
// ---------------------------------------------------------------------------

pub fn order_labels(l: Lang) -> OrderLabels {
    OrderLabels {
        by_hand: s(l, ["By hand", "Ručně", "Власноруч", "Вручную"]),
        quick: s(l, ["Quick", "Rychle", "Швидко", "Быстро"]),
        usual: s(l, ["The usual?", "Jako vždy?", "Як завжди?", "Как обычно?"]),
        hand_off: s(
            l,
            [
                "Hand off to a helper",
                "Předat pomocníkovi",
                "Передати помічнику",
                "Передать помощнику",
            ],
        ),
        book: s(
            l,
            [
                "Recipe book",
                "Kniha receptů",
                "Книга рецептів",
                "Книга рецептов",
            ],
        ),
        close: s(
            l,
            [
                "Close · Esc",
                "Zavřít · Esc",
                "Закрити · Esc",
                "Закрыть · Esc",
            ],
        ),
        plain: s(
            l,
            [
                "Plain order",
                "Běžná objednávka",
                "Звичайне замовлення",
                "Обычный заказ",
            ],
        ),
        practised: s(
            l,
            ["Practised", "Vyzkoušený", "Відпрацьований", "Отработан"],
        ),
        tastes: taste_names(l),
    }
}

pub fn recipe(name: String, dish: Dish, tags: &[TasteTag], practised: bool) -> RecipeChoice {
    RecipeChoice {
        name,
        dish,
        tags: tags.to_vec(),
        practised,
        highlight: false,
    }
}

pub fn mirek_order(l: Lang) -> OrderView {
    use TasteTag::*;
    OrderView {
        guest: "Mirek".into(),
        subtitle: s(
            l,
            [
                "Regular · taxi driver",
                "Štamgast · taxikář",
                "Завсідник · таксист",
                "Завсегдатай · таксист",
            ],
        ),
        regular: true,
        trust: Some(0.62),
        wait: 0.28,
        kind: OrderKind::Wish,
        text: s(
            l,
            [
                "Black, Karolina. The way Lida poured it.",
                "Černou, Karolíno. Tak, jak ji nalévala Lida.",
                "Чорну, Кароліно. Так, як наливала Ліда.",
                "Чёрный, Каролина. Так, как наливала Лида.",
            ],
        ),
        want: vec![Strong, Bitter, Warm],
        avoid: vec![Sweet],
        show_hints: true,
        recipes: vec![
            recipe(
                s(
                    l,
                    [
                        "Black, like Lida made it",
                        "Černá jako od Lidy",
                        "Чорна, як у Ліди",
                        "Чёрный, как у Лиды",
                    ],
                ),
                black(),
                &[Warm, Bitter, Strong, Quick],
                true,
            ),
            recipe(
                s(
                    l,
                    [
                        "Turek, taxi-rank style",
                        "Turek od taxikářů",
                        "Турек по-таксистськи",
                        "Турек по-таксистски",
                    ],
                ),
                Dish::new(DishGlyph::Espresso, hex(0x2A1A10)).steam(true),
                &[Strong, Bitter, Warm],
                false,
            ),
            recipe(
                s(l, ["Cappuccino", "Cappuccino", "Капучино", "Капучино"]),
                cappuccino(),
                &[Warm, Mild, Comforting],
                true,
            ),
            recipe(
                s(
                    l,
                    [
                        "Tea with lemon",
                        "Čaj s citronem",
                        "Чай з лимоном",
                        "Чай с лимоном",
                    ],
                ),
                lemon_tea(),
                &[Warm, Mild, Fresh, Quick],
                false,
            ),
        ],
        usual: Some(s(
            l,
            [
                "Black, like Lida made it",
                "Černá jako od Lidy",
                "Чорна, як у Ліди",
                "Чёрный, как у Лиды",
            ],
        )),
        can_hand_off: true,
        remember: Some(s(
            l,
            [
                "Handover note: “Mirek seemed tired.”",
                "Předávka: „Mirek vypadal unaveně.“",
                "Передача: «Мірек здавався втомленим».",
                "Передача: «Мирек выглядел уставшим».",
            ],
        )),
    }
}

pub fn plain_order(l: Lang) -> OrderView {
    use TasteTag::*;
    OrderView {
        guest: "Tereza Nováková".into(),
        subtitle: s(
            l,
            [
                "Traveller · EN 471",
                "Cestující · EN 471",
                "Мандрівниця · EN 471",
                "Путешественница · EN 471",
            ],
        ),
        regular: false,
        trust: None,
        wait: 0.64,
        kind: OrderKind::Plain,
        text: s(
            l,
            [
                "One cappuccino, please",
                "Jedno cappuccino, prosím",
                "Одне капучино, будь ласка",
                "Один капучино, пожалуйста",
            ],
        ),
        want: vec![],
        avoid: vec![],
        show_hints: true,
        recipes: vec![recipe(
            s(l, ["Cappuccino", "Cappuccino", "Капучино", "Капучино"]),
            cappuccino(),
            &[Warm, Mild, Comforting],
            true,
        )],
        usual: None,
        can_hand_off: false,
        remember: None,
    }
}

pub fn tickets(l: Lang) -> Vec<Ticket> {
    let wish = s(l, ["wish", "přání", "побажання", "пожелание"]);
    let waiting = |m: u32| s(l, ["waiting", "čeká", "чекає", "ждёт"]) + &format!(" {m} min");
    let mk = |id: u64,
              guest: &str,
              label: &str,
              item: TicketItem,
              owner: TicketOwner,
              wait: f32,
              regular: bool,
              incoming: bool,
              mins: u32| {
        let what = match item {
            TicketItem::Wish => wish.clone(),
            TicketItem::Plain(_) => s(
                l,
                [
                    "plain order",
                    "běžná objednávka",
                    "звичайне",
                    "обычный заказ",
                ],
            ),
        };
        Ticket {
            id,
            guest: guest.into(),
            label: label.into(),
            tooltip: format!("{guest} · {what} · {}", waiting(mins)),
            item,
            owner,
            wait,
            regular,
            incoming,
        }
    };
    let to_go = s(l, ["to go", "s sebou", "з собою", "с собой"]);
    vec![
        mk(
            1,
            "Mirek",
            "Mirek",
            TicketItem::Wish,
            TicketOwner::Karolina,
            0.28,
            true,
            false,
            3,
        ),
        mk(
            2,
            "Tereza Nováková",
            "Tereza",
            TicketItem::Plain(cappuccino()),
            TicketOwner::Nobody,
            0.64,
            false,
            false,
            7,
        ),
        mk(
            3,
            "Jonas Weber",
            "Jonas",
            TicketItem::Plain(topinka()),
            TicketOwner::Helper("Kuba".into()),
            0.12,
            false,
            false,
            1,
        ),
        mk(
            4,
            "Vera",
            "Vera",
            TicketItem::Wish,
            TicketOwner::Nobody,
            0.86,
            true,
            false,
            11,
        ),
        mk(
            5,
            "Olena Bondar",
            "Olena",
            TicketItem::Plain(lemon_tea()),
            TicketOwner::Helper("Oleh".into()),
            0.4,
            false,
            false,
            4,
        ),
        mk(
            6,
            "Radek",
            &to_go,
            TicketItem::Plain(black()),
            TicketOwner::Nobody,
            0.0,
            true,
            true,
            0,
        ),
    ]
}

pub fn many_tickets(l: Lang) -> Vec<Ticket> {
    let mut v = tickets(l);
    for i in 0..6 {
        let mut t = v[1 + i % 4].clone();
        t.id = 100 + i as u64;
        t.wait = (0.15 * i as f32) % 1.0;
        t.regular = false;
        t.incoming = false;
        t.guest = ["Lukas", "Maria", "Pavel", "Sofia", "Andrej", "Ewa"][i].into();
        t.label = t.guest.clone();
        v.push(t);
    }
    v
}

// ---------------------------------------------------------------------------
// Moments
// ---------------------------------------------------------------------------

pub fn moment_cat(l: Lang) -> MomentView {
    use shiftella_game::ui::Icon;
    MomentView {
        icon: Icon::Heart,
        title: s(
            l,
            [
                "The station cat is at the door",
                "U dveří je nádražní kočka",
                "Біля дверей вокзальний кіт",
                "У двери вокзальный кот",
            ],
        ),
        text: Some(s(
            l,
            [
                "Wet whiskers, very polite. It looks at the counter, then at you.",
                "Mokré vousky, velmi zdvořilá. Dívá se na pult, pak na tebe.",
                "Мокрі вуса, дуже ввічливий. Дивиться на прилавок, потім на тебе.",
                "Мокрые усы, очень вежливый. Смотрит на прилавок, потом на тебя.",
            ],
        )),
        options: vec![
            MomentOption::new(s(
                l,
                [
                    "A bit of ham",
                    "Kousek šunky",
                    "Шматочок шинки",
                    "Кусочек ветчины",
                ],
            ))
            .cost("−15 Kč"),
            MomentOption::new(s(
                l,
                [
                    "A saucer of milk",
                    "Talířek mléka",
                    "Блюдечко молока",
                    "Блюдце молока",
                ],
            ))
            .cost("−10 Kč"),
            MomentOption::new(s(
                l,
                [
                    "Scratch its ears",
                    "Podrbat za ušima",
                    "Почухати за вушком",
                    "Почесать за ушком",
                ],
            ))
            .gain("+♥"),
        ],
        expires: Some(0.7),
    }
}

pub fn moment_window(l: Lang) -> MomentView {
    use shiftella_game::ui::Icon;
    MomentView {
        icon: Icon::Train,
        title: s(
            l,
            [
                "The window is fogged up",
                "Okno je zamlžené",
                "Вікно запітніло",
                "Окно запотело",
            ],
        ),
        text: None,
        options: vec![
            MomentOption::new(s(
                l,
                [
                    "Wipe the glass",
                    "Otřít sklo",
                    "Протерти скло",
                    "Протереть стекло",
                ],
            ))
            .gain("+3"),
            MomentOption::new(s(
                l,
                [
                    "Ask the band in",
                    "Pozvat kapelu dovnitř",
                    "Запросити гурт",
                    "Позвать музыкантов",
                ],
            ))
            .disabled(true)
            .tooltip(s(
                l,
                [
                    "Needs a stage",
                    "Chce to pódium",
                    "Потрібна сцена",
                    "Нужна сцена",
                ],
            )),
        ],
        expires: Some(0.25),
    }
}

// ---------------------------------------------------------------------------
// Duties
// ---------------------------------------------------------------------------

pub fn duties_labels(l: Lang) -> DutiesLabels {
    DutiesLabels {
        title: s(l, ["Duties", "Úkoly", "Обов'язки", "Обязанности"]),
        subtitle: s(
            l,
            [
                "Who helps with what tonight",
                "Kdo dnes s čím pomáhá",
                "Хто сьогодні з чим допомагає",
                "Кто сегодня с чем помогает",
            ],
        ),
        duty: [
            s(l, ["Dishes", "Nádobí", "Посуд", "Посуда"]),
            s(l, ["Floor", "Podlaha", "Підлога", "Пол"]),
            s(l, ["Register", "Pokladna", "Каса", "Касса"]),
            s(l, ["Bar", "Bar", "Бар", "Бар"]),
            s(l, ["Kitchen", "Kuchyně", "Кухня", "Кухня"]),
            s(l, ["Serving", "Obsluha", "Подача", "Подача"]),
        ],
        duty_desc: [
            s(
                l,
                [
                    "Clears tables, washes cups, drops left tips in the jar",
                    "Sklízí stoly, myje hrnky, dává dýška do sklenice",
                    "Прибирає столи, миє чашки, кладе чайові в банку",
                    "Убирает столы, моет чашки, кладёт чаевые в банку",
                ],
            ),
            s(
                l,
                [
                    "Mops, buckets, the cleaning checklist",
                    "Vytírá, kbelíky, úklidový checklist",
                    "Миє підлогу, відра, чекліст прибирання",
                    "Моет пол, вёдра, чек-лист уборки",
                ],
            ),
            s(
                l,
                [
                    "Takes plain orders and payments",
                    "Bere běžné objednávky a platby",
                    "Приймає звичайні замовлення й оплату",
                    "Принимает обычные заказы и оплату",
                ],
            ),
            s(
                l,
                [
                    "Makes plain drinks, base recipes only",
                    "Připravuje běžné nápoje, jen základní recepty",
                    "Готує звичайні напої, лише базові рецепти",
                    "Готовит обычные напитки, только базовые рецепты",
                ],
            ),
            s(
                l,
                [
                    "Makes plain food, base recipes only",
                    "Připravuje běžné jídlo, jen základní recepty",
                    "Готує звичайну їжу, лише базові рецепти",
                    "Готовит обычную еду, только базовые рецепты",
                ],
            ),
            s(
                l,
                [
                    "Carries what helpers made (never yours)",
                    "Roznáší, co připravili pomocníci (tvoje ne)",
                    "Розносить те, що зробили помічники (не твоє)",
                    "Разносит то, что сделали помощники (не твоё)",
                ],
            ),
        ],
        suggest: s(l, ["Suggest", "Navrhnout", "Запропонувати", "Предложить"]),
        karolina_takes: s(
            l,
            [
                "Left for you:",
                "Zbývá na tebe:",
                "Лишається тобі:",
                "Остаётся тебе:",
            ],
        ),
        all_covered: s(
            l,
            [
                "Every duty has a helper. You take the guests.",
                "Každý úkol má pomocníka. Ty máš hosty.",
                "Кожен обов'язок має помічника. Ти — гості.",
                "У каждой обязанности есть помощник. На тебе гости.",
            ],
        ),
        off_shift: s(
            l,
            [
                "Off tonight",
                "Dnes volno",
                "Сьогодні вихідний",
                "Сегодня выходной",
            ],
        ),
        full: s(
            l,
            [
                "Full — take a duty away first",
                "Plno — nejdřív nějaký úkol uber",
                "Повно — спершу зніми обов'язок",
                "Занято — сначала сними обязанность",
            ],
        ),
        capacity: s(
            l,
            [
                "{used} of {cap}",
                "{used} z {cap}",
                "{used} з {cap}",
                "{used} из {cap}",
            ],
        ),
        skill: s(l, ["skill", "dovednost", "навичка", "навык"]),
        likes: s(l, ["likes it", "má rád", "любить", "любит"]),
    }
}

pub fn duties_view(l: Lang) -> DutiesView {
    let role = |v: [&str; 4]| s(l, v);
    DutiesView {
        helpers: vec![
            HelperRow {
                name: "Kuba".into(),
                role: role(["Porter", "Pomocník", "Носій", "Носильщик"]),
                capacity: 2,
                duties: vec![Duty::Dishes],
                on_shift: true,
                likes: Some(Duty::Dishes),
                skills: [7, 5, 2, 3, 4, 5],
                note: Some(role([
                    "Can take one more duty",
                    "Zvládne ještě jeden úkol",
                    "Може взяти ще один обов'язок",
                    "Может взять ещё одну обязанность",
                ])),
            },
            HelperRow {
                name: "Oleh".into(),
                role: role(["Sound tech", "Zvukař", "Звукорежисер", "Звукорежиссёр"]),
                capacity: 3,
                duties: vec![Duty::Bar, Duty::Register, Duty::Serving],
                on_shift: true,
                likes: Some(Duty::Bar),
                skills: [3, 3, 6, 8, 2, 6],
                note: None,
            },
            HelperRow {
                name: "Bára".into(),
                role: role(["Barista", "Baristka", "Бариста", "Бариста"]),
                capacity: 1,
                duties: vec![],
                on_shift: true,
                likes: Some(Duty::Bar),
                skills: [2, 2, 4, 9, 3, 5],
                note: Some(role([
                    "New: one duty for now",
                    "Nová: zatím jeden úkol",
                    "Нова: поки один обов'язок",
                    "Новенькая: пока одна обязанность",
                ])),
            },
            HelperRow {
                name: "Ema".into(),
                role: role(["Quiet one", "Tichá duše", "Тихоня", "Тихоня"]),
                capacity: 2,
                duties: vec![Duty::Floor],
                on_shift: false,
                likes: Some(Duty::Floor),
                skills: [5, 7, 3, 2, 3, 4],
                note: None,
            },
        ],
        karolina: karolina(l),
    }
}

pub fn karolina(l: Lang) -> String {
    s(
        l,
        [
            "Karolina (you)",
            "Karolína (ty)",
            "Кароліна (ти)",
            "Каролина (ты)",
        ],
    )
}

// ---------------------------------------------------------------------------
// Records
// ---------------------------------------------------------------------------

pub fn record_labels(l: Lang) -> RecordLabels {
    RecordLabels {
        title: s(
            l,
            [
                "Grandma's jukebox",
                "Babiččin jukebox",
                "Бабусин музичний автомат",
                "Бабушкин музыкальный автомат",
            ],
        ),
        repeat: s(l, ["Repeat", "Opakovat", "Повтор", "Повтор"]),
        play_once: s(l, ["Play once", "Jednou", "Один раз", "Один раз"]),
        now_playing: s(
            l,
            ["Now playing", "Právě hraje", "Зараз грає", "Сейчас играет"],
        ),
        close: s(
            l,
            [
                "Close · Esc",
                "Zavřít · Esc",
                "Закрити · Esc",
                "Закрыть · Esc",
            ],
        ),
        liked_by: s(l, ["Likes it:", "Líbí se:", "Подобається:", "Нравится:"]),
    }
}

pub fn records_view(l: Lang, playing: usize, repeat: bool) -> RecordsView {
    let rec = |title: String,
               artist: &str,
               mood: String,
               mc: u32,
               motif: SleeveMotif,
               color: u32,
               fans: &[&str]| RecordView {
        title,
        artist: artist.into(),
        mood,
        mood_color: hex(mc),
        motif,
        color: hex(color),
        locked: false,
        hint: None,
        fans: fans.iter().map(|f| f.to_string()).collect(),
        playing: false,
    };
    let mut v = vec![
        rec(
            s(
                l,
                [
                    "House playlist",
                    "Domácí playlist",
                    "Плейлист кав'ярні",
                    "Плейлист кафе",
                ],
            ),
            "Café Nocturne",
            s(
                l,
                [
                    "Follows the night",
                    "Podle noci",
                    "За настроєм ночі",
                    "По настроению ночи",
                ],
            ),
            0xA5B4FC,
            SleeveMotif::House,
            0x3A2F66,
            &[],
        ),
        rec(
            s(
                l,
                [
                    "Holešovice Swing, 1938",
                    "Holešovický swing, 1938",
                    "Голешовицький свінг, 1938",
                    "Голешовицкий свинг, 1938",
                ],
            ),
            "The Tram Twelve Band",
            s(l, ["Lively", "Živá", "Жвава", "Живая"]),
            0xFFC46B,
            SleeveMotif::Tram,
            0xC9A46B,
            &["Mirek", "Radek"],
        ),
        rec(
            s(
                l,
                [
                    "Last Tram Trio",
                    "Trio poslední tramvaje",
                    "Тріо останнього трамвая",
                    "Трио последнего трамвая",
                ],
            ),
            "Vltava Night Trio",
            s(l, ["Calm", "Klidná", "Спокійна", "Спокойная"]),
            0xA5B4FC,
            SleeveMotif::PianoKeys,
            0x2C3A6E,
            &["Radek"],
        ),
        rec(
            s(
                l,
                [
                    "Bossa for a Rainy Platform",
                    "Bossa pro deštivé nástupiště",
                    "Боса для дощового перону",
                    "Босса для дождливого перрона",
                ],
            ),
            "Lina Morais & friends",
            s(l, ["Dreamy", "Snivá", "Мрійлива", "Мечтательная"]),
            0xC4B5FD,
            SleeveMotif::Raindrops,
            0x2F6E7A,
            &["Lori"],
        ),
        rec(
            s(
                l,
                [
                    "Kolyskova — Carpathian Lullabies",
                    "Kolyskova — karpatské ukolébavky",
                    "Колискова — карпатські колискові",
                    "Колыбельная — карпатские колыбельные",
                ],
            ),
            "bandura, cimbalom & accordion",
            s(l, ["Tender", "Něžná", "Ніжна", "Нежная"]),
            0xF9A8A8,
            SleeveMotif::Sunflower,
            0xE0A96D,
            &[],
        ),
        rec(
            s(
                l,
                [
                    "Holešovice 2039",
                    "Holešovice 2039",
                    "Голешовиці 2039",
                    "Голешовице 2039",
                ],
            ),
            "SLEEPER CAR",
            s(l, ["Dreamy", "Snivá", "Мрійлива", "Мечтательная"]),
            0xC4B5FD,
            SleeveMotif::SynthGrid,
            0x3B1F5E,
            &[],
        ),
        rec(
            s(
                l,
                ["Lida's Waltz", "Lidin valčík", "Вальс Ліди", "Вальс Лиды"],
            ),
            "„Pro J.“",
            s(
                l,
                [
                    "Nostalgic",
                    "Nostalgická",
                    "Ностальгійна",
                    "Ностальгическая",
                ],
            ),
            0xFFC46B,
            SleeveMotif::Rose,
            0x5A2A3A,
            &[],
        ),
    ];
    v[5].locked = true;
    v[5].hint = Some(s(
        l,
        [
            "Filip might cut one himself",
            "Filip by možná nahrál vlastní",
            "Філіп, може, запише власну",
            "Филип, может, запишет свою",
        ],
    ));
    v[6].locked = true;
    v[6].hint = Some(s(
        l,
        [
            "An empty sleeve in the storeroom…",
            "Prázdný obal ve skladu…",
            "Порожній конверт у коморі…",
            "Пустой конверт в кладовке…",
        ],
    ));
    v[playing].playing = true;
    RecordsView { records: v, repeat }
}

// ---------------------------------------------------------------------------
// Recipe book
// ---------------------------------------------------------------------------

pub fn book_labels(l: Lang) -> BookLabels {
    BookLabels {
        title: s(
            l,
            [
                "Recipe book",
                "Kniha receptů",
                "Книга рецептів",
                "Книга рецептов",
            ],
        ),
        tabs: [
            s(l, ["Drinks", "Nápoje", "Напої", "Напитки"]),
            s(l, ["Food", "Jídlo", "Їжа", "Еда"]),
            s(l, ["Specials", "Speciality", "Особливі", "Особые"]),
            s(l, ["Pantry", "Spíž", "Комора", "Кладовая"]),
        ],
        pick: s(
            l,
            ["Make this", "Udělat tohle", "Зробити це", "Сделать это"],
        ),
        ingredients: s(l, ["Ingredients", "Suroviny", "Інгредієнти", "Ингредиенты"]),
        tastes_head: s(l, ["Tastes", "Chutě", "Смаки", "Вкусы"]),
        minigame: s(l, ["By hand", "Ručně", "Власноруч", "Вручную"]),
        patterns: s(l, ["Latte art", "Latte art", "Лате-арт", "Латте-арт"]),
        unknown: s(
            l,
            [
                "Not discovered yet",
                "Zatím neobjeveno",
                "Ще не відкрито",
                "Ещё не открыто",
            ],
        ),
        where_to_learn: s(
            l,
            [
                "Where to learn it",
                "Kde se to naučit",
                "Де цьому навчитися",
                "Где этому научиться",
            ],
        ),
        practised: s(
            l,
            [
                "Practised — Quick brews come out better",
                "Vyzkoušené — rychlá příprava je lepší",
                "Відпрацьовано — швидке приготування краще",
                "Отработано — быстрое приготовление лучше",
            ],
        ),
        try_label: s(l, ["Try", "Zkusit", "Спробувати", "Попробовать"]),
        clear: s(l, ["Clear", "Vyprázdnit", "Очистити", "Очистить"]),
        slots_hint: s(
            l,
            [
                "Pick 2–3 ingredients and try a new recipe.",
                "Vyber 2–3 suroviny a zkus nový recept.",
                "Обери 2–3 інгредієнти й спробуй новий рецепт.",
                "Выбери 2–3 ингредиента и попробуй новый рецепт.",
            ],
        ),
        bar_shelf: s(
            l,
            ["Bar shelf", "Barová police", "Барна полиця", "Барная полка"],
        ),
        kitchen_shelf: s(
            l,
            [
                "Kitchen shelf",
                "Kuchyňská police",
                "Кухонна полиця",
                "Кухонная полка",
            ],
        ),
        locked_jar: s(
            l,
            [
                "Not in the pantry yet",
                "Ve spíži zatím není",
                "У коморі ще немає",
                "В кладовой пока нет",
            ],
        ),
        empty_section: s(
            l,
            [
                "Nothing here yet.",
                "Zatím tu nic není.",
                "Тут поки нічого.",
                "Здесь пока ничего.",
            ],
        ),
        close: s(
            l,
            [
                "Close · Esc",
                "Zavřít · Esc",
                "Закрити · Esc",
                "Закрыть · Esc",
            ],
        ),
        tastes: taste_names(l),
    }
}

#[allow(clippy::too_many_arguments)]
fn br(
    section: BookSection,
    name: String,
    known: bool,
    dish: Dish,
    tags: &[TasteTag],
    ingredients: Vec<String>,
    count: u8,
) -> BookRecipe {
    BookRecipe {
        section,
        name,
        known,
        practised: false,
        new: false,
        dish,
        tags: tags.to_vec(),
        ingredient_count: count,
        ingredients,
        desc: None,
        lore: None,
        minigame: None,
        base_line: None,
        stats_line: None,
        source_hint: None,
        patterns: vec![],
    }
}

pub fn ing_names(l: Lang) -> Vec<(String, Shelf, bool, u32, String)> {
    let f = |v: [&str; 4]| s(l, v);
    vec![
        (
            f(["Espresso", "Espresso", "Еспресо", "Эспрессо"]),
            Shelf::Bar,
            true,
            0x3B2416,
            f(["Coffee", "Káva", "Кава", "Кофе"]),
        ),
        (
            f(["Milk", "Mléko", "Молоко", "Молоко"]),
            Shelf::Bar,
            true,
            0xF4F0E6,
            f(["Milk", "Mléko", "Молоко", "Молоко"]),
        ),
        (
            f(["Oat milk", "Ovesné", "Вівсяне", "Овсяное"]),
            Shelf::Bar,
            true,
            0xE8DCC0,
            f(["Milk", "Mléko", "Молоко", "Молоко"]),
        ),
        (
            f(["Black tea", "Černý čaj", "Чорний чай", "Чёрный чай"]),
            Shelf::Bar,
            true,
            0x7A3E1C,
            f(["Tea", "Čaj", "Чай", "Чай"]),
        ),
        (
            f(["Linden", "Lipový květ", "Липа", "Липа"]),
            Shelf::Bar,
            true,
            0xD9C56A,
            f(["Tea", "Čaj", "Чай", "Чай"]),
        ),
        (
            f(["Lemon", "Citron", "Лимон", "Лимон"]),
            Shelf::Bar,
            true,
            0xF2D64B,
            f(["Fruit", "Ovoce", "Фрукти", "Фрукты"]),
        ),
        (
            f(["Honey", "Med", "Мед", "Мёд"]),
            Shelf::Bar,
            true,
            0xE3A33A,
            f(["Sweet", "Sladké", "Солодке", "Сладкое"]),
        ),
        (
            f(["Sugar", "Cukr", "Цукор", "Сахар"]),
            Shelf::Bar,
            true,
            0xF7F3EE,
            f(["Sweet", "Sladké", "Солодке", "Сладкое"]),
        ),
        (
            f(["Spice", "Koření", "Спеції", "Специи"]),
            Shelf::Bar,
            true,
            0xA0522D,
            f(["Spice", "Koření", "Спеції", "Специи"]),
        ),
        (
            f(["Cocoa", "Kakao", "Какао", "Какао"]),
            Shelf::Bar,
            true,
            0x5A3020,
            f(["Sweet", "Sladké", "Солодке", "Сладкое"]),
        ),
        (
            f(["Chili", "Chilli", "Чилі", "Чили"]),
            Shelf::Bar,
            false,
            0xC0392B,
            f(["Spice", "Koření", "Спеції", "Специи"]),
        ),
        (
            f(["Tonic", "Tonik", "Тонік", "Тоник"]),
            Shelf::Bar,
            false,
            0xDDEEF0,
            f(["Cold", "Studené", "Холодне", "Холодное"]),
        ),
        (
            f(["Bread", "Chléb", "Хліб", "Хлеб"]),
            Shelf::Kitchen,
            true,
            0xC98A43,
            f(["Dough", "Těsto", "Тісто", "Тесто"]),
        ),
        (
            f(["Garlic", "Česnek", "Часник", "Чеснок"]),
            Shelf::Kitchen,
            true,
            0xEFE6D2,
            f(["Savoury", "Slané", "Солоне", "Солёное"]),
        ),
        (
            f(["Potato", "Brambory", "Картопля", "Картофель"]),
            Shelf::Kitchen,
            true,
            0xD8B56A,
            f(["Savoury", "Slané", "Солоне", "Солёное"]),
        ),
        (
            f(["Tvaroh", "Tvaroh", "Сир", "Творог"]),
            Shelf::Kitchen,
            true,
            0xF5F1E8,
            f(["Dough", "Těsto", "Тісто", "Тесто"]),
        ),
        (
            f(["Plum jam", "Povidla", "Повидло", "Повидло"]),
            Shelf::Kitchen,
            true,
            0x5A1F3A,
            f(["Fruit", "Ovoce", "Фрукти", "Фрукты"]),
        ),
        (
            f(["Mushroom", "Houby", "Гриби", "Грибы"]),
            Shelf::Kitchen,
            false,
            0x8B6B4A,
            f(["Savoury", "Slané", "Солоне", "Солёное"]),
        ),
        (
            f(["Beetroot", "Řepa", "Буряк", "Свёкла"]),
            Shelf::Kitchen,
            false,
            0x8E1F45,
            f(["Savoury", "Slané", "Солоне", "Солёное"]),
        ),
    ]
}

pub fn book_view(
    l: Lang,
    pick_mode: bool,
    result: Option<ExperimentResult>,
    margin: bool,
) -> BookView {
    use BookSection::*;
    use TasteTag::*;
    let f = |v: [&str; 4]| s(l, v);
    let mut black_r = br(
        Drinks,
        f([
            "Black, like Lida made it",
            "Černá jako od Lidy",
            "Чорна, як у Ліди",
            "Чёрный, как у Лиды",
        ]),
        true,
        black(),
        &[Warm, Bitter, Strong, Quick],
        vec![f(["Espresso", "Espresso", "Еспресо", "Эспрессо"])],
        1,
    );
    black_r.practised = true;
    black_r.lore = Some(f([
        "Pour slowly, Karolinko. Mirek can tell when you rush.",
        "Nalévej pomalu, Karolínko. Mirek pozná, když spěcháš.",
        "Наливай повільно, Каролінко. Мірек відчує, коли поспішаєш.",
        "Наливай медленно, Каролинка. Мирек заметит, если спешишь.",
    ]));
    black_r.minigame = Some(f(["Pour-over", "Překapávání", "Пуровер", "Пуровер"]));
    black_r.base_line = Some(f([
        "Black coffee · 45 Kč",
        "Černá káva · 45 Kč",
        "Чорна кава · 45 Kč",
        "Чёрный кофе · 45 Kč",
    ]));
    black_r.stats_line = Some(f([
        "Served 12× · best: Beautiful",
        "Podáno 12× · nejlépe: Krásná",
        "Подано 12× · найкраще: Чудово",
        "Подано 12× · лучшее: Прекрасно",
    ]));
    let mut capp = br(
        Drinks,
        f(["Cappuccino", "Cappuccino", "Капучино", "Капучино"]),
        true,
        cappuccino(),
        &[Warm, Mild, Comforting],
        vec![
            f(["Espresso", "Espresso", "Еспресо", "Эспрессо"]),
            f(["Milk", "Mléko", "Молоко", "Молоко"]),
        ],
        2,
    );
    capp.minigame = Some(f([
        "Milk froth",
        "Napěnění mléka",
        "Збивання молока",
        "Взбивание молока",
    ]));
    capp.patterns = vec![
        f(["Heart", "Srdce", "Серце", "Сердце"]),
        f(["Tulip", "Tulipán", "Тюльпан", "Тюльпан"]),
    ];
    capp.lore = Some(f([
        "The foam should hold a sugar cube for a second.",
        "Pěna má udržet kostku cukru aspoň vteřinu.",
        "Піна має втримати шматочок цукру хоч секунду.",
        "Пена должна удержать кусочек сахара хоть секунду.",
    ]));
    let mut linden_r = br(
        Drinks,
        f([
            "Linden tea with honey",
            "Lipový čaj s medem",
            "Липовий чай з медом",
            "Липовый чай с мёдом",
        ]),
        true,
        linden(),
        &[Warm, Mild, Comforting, Sweet],
        vec![],
        2,
    );
    linden_r.new = true;
    let mut moon = br(
        Drinks,
        String::new(),
        false,
        Dish::new(DishGlyph::Cup, hex(0x6A7BD8)),
        &[Warm, Fancy, Mild, Sweet],
        vec![f(["Honey", "Med", "Мед", "Мёд"])],
        3,
    );
    moon.source_hint = Some(f([
        "A blue flower from a lost parcel, perhaps…",
        "Modrý květ ze ztraceného balíčku, možná…",
        "Синя квітка із загубленої посилки, може…",
        "Синий цветок из потерянной посылки, может…",
    ]));
    let mut turek = br(
        Drinks,
        String::new(),
        false,
        Dish::new(DishGlyph::Espresso, hex(0x2A1A10)),
        &[Strong, Bitter, Warm],
        vec![],
        2,
    );
    turek.source_hint = Some(f([
        "Someone at the taxi rank might know.",
        "Někdo na stanovišti taxi to bude vědět.",
        "Хтось на стоянці таксі має знати.",
        "Кто-то на стоянке такси знает.",
    ]));
    let recipes = vec![
        black_r,
        capp,
        br(
            Drinks,
            f([
                "Tea with lemon",
                "Čaj s citronem",
                "Чай з лимоном",
                "Чай с лимоном",
            ]),
            true,
            lemon_tea(),
            &[Warm, Mild, Fresh, Quick],
            vec![],
            2,
        ),
        linden_r,
        br(
            Drinks,
            f(["Chai", "Chai", "Чай масала", "Чай масала"]),
            true,
            chai(),
            &[Warm, Sweet, Spiced, Comforting],
            vec![],
            3,
        ),
        br(
            Drinks,
            f([
                "Hot chocolate",
                "Horká čokoláda",
                "Гарячий шоколад",
                "Горячий шоколад",
            ]),
            true,
            hot_choc(),
            &[Warm, Sweet, Comforting],
            vec![],
            2,
        ),
        turek,
        moon,
        br(
            Drinks,
            f(["Svařák", "Svařák", "Глінтвейн", "Глинтвейн"]),
            true,
            svarak(),
            &[Warm, Sweet, Spiced, Fancy],
            vec![],
            3,
        ),
        br(
            Drinks,
            String::new(),
            false,
            tonic(),
            &[Cold, Fresh, Bitter, Fancy],
            vec![],
            3,
        ),
        br(
            Food,
            f(["Topinka", "Topinka", "Топінка", "Топинка"]),
            true,
            topinka(),
            &[Warm, Filling, Quick],
            vec![],
            2,
        ),
        br(
            Food,
            f(["Česnečka", "Česnečka", "Часникова юшка", "Чесночный суп"]),
            true,
            soup(),
            &[Warm, Filling, Comforting],
            vec![],
            3,
        ),
        br(
            Food,
            f(["Koláč", "Koláč", "Колач", "Колач"]),
            true,
            kolac(),
            &[Sweet, Quick, Comforting],
            vec![],
            2,
        ),
        br(
            Food,
            f(["Medovník", "Medovník", "Медівник", "Медовик"]),
            true,
            medovnik(),
            &[Sweet, Fancy, Cold],
            vec![],
            2,
        ),
        br(
            Food,
            f(["Varenyky", "Vareniky", "Вареники", "Вареники"]),
            true,
            varenyky(),
            &[Warm, Filling, Comforting],
            vec![],
            2,
        ),
        br(
            Specials,
            f([
                "Grandma's kulajda",
                "Babiččina kulajda",
                "Бабусина кулайда",
                "Бабушкина кулайда",
            ]),
            false,
            kulajda(),
            &[Warm, Filling, Comforting, Fancy],
            vec![],
            3,
        ),
    ];
    let pantry = ing_names(l)
        .into_iter()
        .map(|(name, shelf, unlocked, c, family)| Ingredient {
            name,
            family,
            shelf,
            unlocked,
            color: hex(c),
        })
        .collect();
    BookView {
        recipes,
        pantry,
        result,
        pick_mode,
        try_cost: Some(f([
            "free tonight",
            "dnes zdarma",
            "сьогодні безкоштовно",
            "сегодня бесплатно",
        ])),
        margin_note: margin.then(|| {
            f([
                "Lida's margin note: …honey, always honey.",
                "Lidina poznámka na okraji: …med, vždycky med.",
                "Нотатка Ліди на полях: …мед, завжди мед.",
                "Заметка Лиды на полях: …мёд, всегда мёд.",
            ])
        }),
    }
}

pub fn discovered(l: Lang) -> ExperimentResult {
    ExperimentResult {
        kind: ExperimentKind::Discovered,
        text: s(
            l,
            [
                "Discovered! Linden tea with honey — it smells like grandma's hallway.",
                "Objeveno! Lipový čaj s medem — voní jako babiččina chodba.",
                "Відкрито! Липовий чай з медом — пахне, як бабусин коридор.",
                "Открыто! Липовый чай с мёдом — пахнет, как бабушкин коридор.",
            ],
        ),
        recipe: Some(3),
    }
}

pub fn close_result(l: Lang) -> ExperimentResult {
    ExperimentResult {
        kind: ExperimentKind::Close,
        text: s(
            l,
            [
                "So close — swap the Fruit for something Sweet.",
                "Skoro — vyměň ovoce za něco sladkého.",
                "Майже — заміни фрукт на щось солодке.",
                "Почти — замени фрукт на что-то сладкое.",
            ],
        ),
        recipe: None,
    }
}

// ---------------------------------------------------------------------------
// Atmosphere
// ---------------------------------------------------------------------------

pub fn atmosphere(l: Lang, value: f32) -> AtmosphereView {
    let levels = [
        s(l, ["Bare", "Holé", "Голо", "Пусто"]),
        s(l, ["Plain", "Obyčejné", "Звичайно", "Обычно"]),
        s(l, ["Pleasant", "Příjemné", "Приємно", "Приятно"]),
        s(l, ["Cosy", "Útulné", "Затишно", "Уютно"]),
        s(l, ["Like home", "Jako doma", "Як удома", "Как дома"]),
    ];
    let level = ((value / 20.0) as usize).min(4);
    AtmosphereView {
        value,
        level,
        level_name: levels[level].clone(),
        title: s(l, ["Atmosphere", "Atmosféra", "Атмосфера", "Атмосфера"]),
        components: vec![
            (s(l, ["Light", "Světlo", "Світло", "Свет"]), 0.55),
            (s(l, ["Beauty", "Krása", "Краса", "Красота"]), 0.7),
            (s(l, ["Comfort", "Pohodlí", "Комфорт", "Комфорт"]), 0.62),
            (s(l, ["Clean", "Čistota", "Чистота", "Чистота"]), 0.38),
            (s(l, ["Music", "Hudba", "Музика", "Музыка"]), 1.0),
            (s(l, ["Theme", "Styl", "Стиль", "Стиль"]), 0.6),
        ],
        tip: Some(s(
            l,
            [
                "Dishes are piling up.",
                "Hromadí se nádobí.",
                "Посуд накопичується.",
                "Посуда копится.",
            ],
        )),
        tiers: Vec::new(),
        bounds: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Rozpis
// ---------------------------------------------------------------------------

pub fn rozpis_labels(l: Lang) -> RozpisLabels {
    RozpisLabels {
        shift: [
            "—".into(),
            "22–06".into(),
            "22–02".into(),
            "02–06".into(),
            s(l, ["Leave", "Volno", "Відпустка", "Отпуск"]),
        ],
        staff: s(l, ["Crew", "Tým", "Команда", "Команда"]),
        wellbeing: s(l, ["Mood", "Pohoda", "Настрій", "Настрой"]),
        coverage: s(l, ["Helpers", "Pomocníci", "Помічники", "Помощники"]),
        autofill: s(l, ["Auto-fill", "Vyplnit", "Заповнити", "Заполнить"]),
        clear: s(l, ["Clear", "Vymazat", "Очистити", "Очистить"]),
        publish: s(l, ["Publish", "Zveřejnit", "Опублікувати", "Опубликовать"]),
        cell_tip: s(
            l,
            [
                "Click: next shift · right-click: back · drag to paint",
                "Klik: další směna · pravé tlačítko: zpět · táhni pro malování",
                "Клік: наступна зміна · правий клік: назад · тягни, щоб малювати",
                "Клик: следующая смена · правый клик: назад · тяни, чтобы рисовать",
            ],
        ),
        locked_tip: s(
            l,
            [
                "You're here every night.",
                "Jsi tu každou noc.",
                "Ти тут щоночі.",
                "Ты здесь каждую ночь.",
            ],
        ),
        coverage_tip: s(
            l,
            [
                "{on} on · {suggested} suggested",
                "{on} na směně · {suggested} doporučeno",
                "{on} на зміні · {suggested} радимо",
                "{on} на смене · {suggested} советуем",
            ],
        ),
        publish_locked_tip: String::new(),
    }
}

fn pref(
    icon: shiftella_game::ui::Icon,
    label: String,
    quote: String,
    status: PrefStatus,
    need: bool,
    portrait: Option<&str>,
) -> PrefChip {
    PrefChip {
        icon,
        label,
        quote,
        status,
        need,
        portrait: portrait.map(str::to_string),
    }
}

pub fn rozpis_view(l: Lang, cells: &[[ShiftCell; 7]]) -> RozpisView {
    use ShiftCell::*;
    use shiftella_game::ui::Icon;
    let f = |v: [&str; 4]| s(l, v);
    let days = match l {
        Lang::En => ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"],
        Lang::Cs => ["Po", "Út", "St", "Čt", "Pá", "So", "Ne"],
        Lang::Uk => ["Пн", "Вт", "Ср", "Чт", "Пт", "Сб", "Нд"],
        Lang::Ru => ["Пн", "Вт", "Ср", "Чт", "Пт", "Сб", "Вс"],
    }
    .map(str::to_string);
    let ema_tue = cells[3][1] != Off;
    let rows = vec![
        RozpisRow {
            name: karolina(l),
            prefs: vec![],
            cells: cells[0],
            conflicts: [false; 7],
            wellbeing: 92.0,
            locked: true,
        },
        RozpisRow {
            name: "Bára".into(),
            prefs: vec![
                pref(
                    Icon::Moon,
                    f(["late", "pozdě", "пізно", "поздно"]),
                    f([
                        "Night owl — late shift, please.",
                        "Noční sova — pozdní směnu, prosím.",
                        "Нічна сова — пізню зміну, будь ласка.",
                        "Сова — позднюю смену, пожалуйста.",
                    ]),
                    PrefStatus::Honoured,
                    false,
                    None,
                ),
                pref(
                    Icon::Calendar,
                    f(["Sun off", "Ne volno", "Нд вихідний", "Вс выходной"]),
                    f([
                        "Sunday lunch with my parents. They think I'm still at uni.",
                        "Nedělní oběd s rodiči. Myslí si, že pořád studuju.",
                        "Недільний обід з батьками. Вони думають, що я ще вчуся.",
                        "Воскресный обед с родителями. Они думают, что я ещё учусь.",
                    ]),
                    PrefStatus::Honoured,
                    false,
                    None,
                ),
            ],
            cells: cells[1],
            conflicts: [false; 7],
            wellbeing: 88.0,
            locked: false,
        },
        RozpisRow {
            name: "Kuba".into(),
            prefs: vec![pref(
                Icon::Heart,
                "Zdeněk".into(),
                f([
                    "Mr Holub lets me watch him bake.",
                    "Pan Holub mě nechá koukat, jak peče.",
                    "Пан Голуб дозволяє дивитися, як він пече.",
                    "Пан Голуб разрешает смотреть, как он печёт.",
                ]),
                PrefStatus::Pending,
                false,
                Some("Zdeněk"),
            )],
            cells: cells[2],
            conflicts: [false; 7],
            wellbeing: 74.0,
            locked: false,
        },
        RozpisRow {
            name: "Ema".into(),
            prefs: vec![
                pref(
                    Icon::Calendar,
                    f(["Tue off", "Út volno", "Вт вихідний", "Вт выходной"]),
                    f([
                        "Lectures on Tuesdays.",
                        "V úterý mám přednášky.",
                        "У вівторок лекції.",
                        "По вторникам лекции.",
                    ]),
                    if ema_tue {
                        PrefStatus::Violated
                    } else {
                        PrefStatus::Honoured
                    },
                    true,
                    None,
                ),
                pref(
                    Icon::Home,
                    f(["weekend", "víkend", "вихідні", "выходные"]),
                    f(["Crowds.", "Davy.", "Натовпи.", "Толпы."]),
                    PrefStatus::Violated,
                    false,
                    None,
                ),
            ],
            cells: cells[3],
            conflicts: [false, ema_tue, false, false, false, true, false],
            wellbeing: if ema_tue { 51.0 } else { 63.0 },
            locked: false,
        },
        RozpisRow {
            name: "Zdeněk".into(),
            prefs: vec![
                pref(
                    Icon::LinkBroken,
                    f([
                        "not 2 in a row",
                        "ne 2 za sebou",
                        "не 2 поспіль",
                        "не 2 подряд",
                    ]),
                    f([
                        "Not two nights in a row — my back.",
                        "Ne dvě noci za sebou — moje záda.",
                        "Не дві ночі поспіль — моя спина.",
                        "Не две ночи подряд — моя спина.",
                    ]),
                    PrefStatus::Honoured,
                    true,
                    None,
                ),
                pref(
                    Icon::Clock,
                    f(["full", "celá", "повна", "полная"]),
                    f([
                        "Full nights suit me.",
                        "Celé noci mi vyhovují.",
                        "Повні ночі мені пасують.",
                        "Полные ночи мне подходят.",
                    ]),
                    PrefStatus::Honoured,
                    false,
                    None,
                ),
            ],
            cells: cells[4],
            conflicts: [false; 7],
            wellbeing: 96.0,
            locked: false,
        },
    ];
    let mut coverage = [CoverageNight::default(); 7];
    let suggested = [1, 1, 2, 2, 3, 3, 2];
    let cups = [1, 1, 2, 2, 3, 3, 2];
    for d in 0..7 {
        coverage[d] = CoverageNight {
            on: cells[1..]
                .iter()
                .filter(|r| !matches!(r[d], Off | Leave))
                .count() as u8,
            suggested: suggested[d],
            cups: cups[d],
        };
    }
    let score = if ema_tue { 66.0 } else { 74.0 };
    let weather = WeekWeather::from_score(score);
    let wl = match weather {
        WeekWeather::Sunny => f([
            "Sunny week",
            "Slunečný týden",
            "Сонячний тиждень",
            "Солнечная неделя",
        ]),
        WeekWeather::MostlySunny => f([
            "Mostly sunny week",
            "Spíš slunečný týden",
            "Переважно сонячно",
            "В основном солнечно",
        ]),
        WeekWeather::Cloudy => f([
            "Cloudy week",
            "Zatažený týden",
            "Хмарний тиждень",
            "Облачная неделя",
        ]),
        WeekWeather::CosyRain => f([
            "Rainy, but cosy",
            "Deštivo, ale útulně",
            "Дощить, але затишно",
            "Дождливо, но уютно",
        ]),
    };
    RozpisView {
        days,
        today: Some(0),
        rows,
        coverage,
        fairness: FairnessView {
            score,
            weather,
            label: wl,
            detail: f([
                "Fairness: everyone's mood, minus how uneven it is.",
                "Férovost: pohoda všech, minus jak je nerovná.",
                "Справедливість: настрій усіх мінус нерівність.",
                "Справедливость: настрой всех минус неравенство.",
            ]),
        },
        autofill_line: Some(f([
            "Auto-fill kept 6 of 7 wishes. Ema's 'weekend off' didn't fit Saturday.",
            "Vyplnění splnilo 6 ze 7 přání. Emin „volný víkend“ se do soboty nevešel.",
            "Автозаповнення зберегло 6 із 7 побажань. «Вихідні» Еми не вмістилися в суботу.",
            "Автозаполнение сохранило 6 из 7 пожеланий. «Выходные» Эмы не влезли в субботу.",
        ])),
        dirty: true,
        publish_locked: false,
    }
}

pub fn rozpis_cells() -> Vec<[ShiftCell; 7]> {
    use ShiftCell::*;
    vec![
        [Full; 7],
        [Off, Late, Late, Off, Late, Late, Off],
        [Full, Off, Early, Full, Off, Full, Full],
        [Early, Off, Early, Off, Off, Full, Off],
        [Off, Full, Off, Full, Off, Full, Leave],
    ]
}

// ---------------------------------------------------------------------------
// Payroll
// ---------------------------------------------------------------------------

pub fn payroll_labels(l: Lang) -> PayrollLabels {
    PayrollLabels {
        title: s(
            l,
            [
                "Payroll basis",
                "Podklady pro mzdy",
                "Дані для зарплати",
                "Данные для зарплаты",
            ],
        ),
        cols: [
            s(l, ["Crew", "Tým", "Команда", "Команда"]),
            s(l, ["Hours", "Hodiny", "Години", "Часы"]),
            s(l, ["Wage", "Mzda", "Зарплата", "Зарплата"]),
            s(l, ["Tips", "Dýška", "Чайові", "Чаевые"]),
        ],
        total: s(l, ["Total", "Celkem", "Разом", "Итого"]),
        split_head: s(
            l,
            [
                "This week's tips",
                "Dýška za týden",
                "Чайові за тиждень",
                "Чаевые за неделю",
            ],
        ),
        star_head: s(
            l,
            [
                "Star of the week",
                "Hvězda týdne",
                "Зірка тижня",
                "Звезда недели",
            ],
        ),
        star_tip: s(
            l,
            [
                "Star of the week: +200 Kč bonus and a thank-you card",
                "Hvězda týdne: +200 Kč a děkovná kartička",
                "Зірка тижня: +200 Kč і листівка-подяка",
                "Звезда недели: +200 Kč и открытка-спасибо",
            ],
        ),
        grant: s(
            l,
            [
                "Grant a night off",
                "Dát volnou noc",
                "Дати вихідну ніч",
                "Дать выходную ночь",
            ],
        ),
        granted: s(
            l,
            [
                "Night off next week",
                "Volno příští týden",
                "Вихідний наступного тижня",
                "Выходной на след. неделе",
            ],
        ),
        insights_head: s(
            l,
            [
                "From the night digest",
                "Z nočního přehledu",
                "З нічного підсумку",
                "Из ночной сводки",
            ],
        ),
        send: s(
            l,
            [
                "Send to the accountant",
                "Poslat účetní",
                "Надіслати бухгалтерці",
                "Отправить бухгалтеру",
            ],
        ),
        sent: s(
            l,
            [
                "Sent — thank you!",
                "Odesláno — díky!",
                "Надіслано — дякую!",
                "Отправлено — спасибо!",
            ],
        ),
    }
}

pub fn payroll_view(
    l: Lang,
    split: TipSplit,
    star: Option<usize>,
    granted: bool,
    sent: bool,
) -> PayrollView {
    use shiftella_game::ui::Icon;
    let f = |v: [&str; 4]| s(l, v);
    let tips = match split {
        TipSplit::Cafe => ["0 Kč", "0 Kč", "0 Kč", "0 Kč"],
        TipSplit::Half => ["212 Kč", "164 Kč", "141 Kč", "103 Kč"],
        TipSplit::Crew => ["424 Kč", "328 Kč", "282 Kč", "206 Kč"],
    };
    let row =
        |name: &str, role: String, h: &str, wage: &str, tip: &str, toil: Option<String>| PayRow {
            name: name.into(),
            role,
            hours: h.into(),
            wage: wage.into(),
            tips: tip.into(),
            toil,
            toil_granted: granted,
        };
    PayrollView {
        week: f([
            "Week 2 · 14–20 Nov 2039",
            "Týden 2 · 14.–20. 11. 2039",
            "Тиждень 2 · 14–20.11.2039",
            "Неделя 2 · 14–20.11.2039",
        ]),
        rows: vec![
            row(
                "Bára",
                f(["Barista", "Baristka", "Бариста", "Бариста"]),
                "32 h",
                "5 120 Kč",
                tips[0],
                Some(f(["6 h owed", "dluh 6 h", "борг 6 год", "долг 6 ч"])),
            ),
            row(
                "Kuba",
                f(["Porter", "Pomocník", "Носій", "Носильщик"]),
                "28 h",
                "3 920 Kč",
                tips[1],
                None,
            ),
            row(
                "Zdeněk",
                f(["Cook", "Kuchař", "Кухар", "Повар"]),
                "24 h",
                "4 320 Kč",
                tips[2],
                None,
            ),
            row(
                "Ema",
                f(["Quiet one", "Tichá duše", "Тихоня", "Тихоня"]),
                "16 h",
                "2 240 Kč",
                tips[3],
                None,
            ),
        ],
        wages_total: "15 600 Kč".into(),
        tips_total: match split {
            TipSplit::Cafe => "0 Kč",
            TipSplit::Half => "620 Kč",
            TipSplit::Crew => "1 240 Kč",
        }
        .into(),
        split,
        options: [
            SplitOption {
                title: f(["All to the café", "Vše kavárně", "Усе кав'ярні", "Всё кафе"]),
                detail: f([
                    "1 240 Kč towards grandma's debt",
                    "1 240 Kč na babiččin dluh",
                    "1 240 Kč на бабусин борг",
                    "1 240 Kč на бабушкин долг",
                ]),
                effect: f([
                    "the crew understands",
                    "tým to chápe",
                    "команда розуміє",
                    "команда понимает",
                ]),
            },
            SplitOption {
                title: f([
                    "Half to the crew",
                    "Polovina týmu",
                    "Половина команді",
                    "Половина команде",
                ]),
                detail: f([
                    "620 Kč to the café · 620 Kč shared",
                    "620 Kč kavárně · 620 Kč týmu",
                    "620 Kč кав'ярні · 620 Kč команді",
                    "620 Kč кафе · 620 Kč команде",
                ]),
                effect: f([
                    "crew mood +4 next week",
                    "nálada týmu +4 příští týden",
                    "настрій команди +4",
                    "настрой команды +4",
                ]),
            },
            SplitOption {
                title: f(["All to the crew", "Vše týmu", "Усе команді", "Всё команде"]),
                detail: f([
                    "1 240 Kč shared by hours",
                    "1 240 Kč podle hodin",
                    "1 240 Kč за годинами",
                    "1 240 Kč по часам",
                ]),
                effect: f([
                    "mood +8 · wellbeing +5",
                    "nálada +8 · pohoda +5",
                    "настрій +8 · добробут +5",
                    "настрой +8 · самочувствие +5",
                ]),
            },
        ],
        star,
        insights: vec![
            Insight {
                icon: Icon::Sparkle,
                text: f([
                    "8 wishes, 5 perfect. Vera loved the linden tea.",
                    "8 přání, 5 dokonalých. Vera milovala lipový čaj.",
                    "8 побажань, 5 ідеальних. Вірі сподобався липовий чай.",
                    "8 пожеланий, 5 идеальных. Вере понравился липовый чай.",
                ]),
                link: f(["Book", "Kniha", "Книга", "Книга"]),
            },
            Insight {
                icon: Icon::Snowflake,
                text: f([
                    "Three guests wanted something Cold.",
                    "Tři hosté chtěli něco studeného.",
                    "Троє гостей хотіли щось холодне.",
                    "Трое гостей хотели холодного.",
                ]),
                link: f(["Pantry", "Spíž", "Комора", "Кладовая"]),
            },
            Insight {
                icon: Icon::Calendar,
                text: f([
                    "Friday will be busy again.",
                    "Pátek bude zase rušný.",
                    "У п'ятницю знову буде людно.",
                    "В пятницу снова будет людно.",
                ]),
                link: f(["Rozpis", "Rozpis", "Розклад", "Расписание"]),
            },
        ],
        sent,
    }
}

// ---------------------------------------------------------------------------
// Journal
// ---------------------------------------------------------------------------

pub fn journal_labels(l: Lang) -> JournalLabels {
    JournalLabels {
        title: s(l, ["Journal", "Deník", "Щоденник", "Дневник"]),
        page: s(
            l,
            [
                "Page {n} of {m}",
                "Strana {n} z {m}",
                "Сторінка {n} з {m}",
                "Страница {n} из {m}",
            ],
        ),
        night: s(l, ["Night {n}", "Noc {n}", "Ніч {n}", "Ночь {n}"]),
        close: s(
            l,
            [
                "Close · Esc",
                "Zavřít · Esc",
                "Закрити · Esc",
                "Закрыть · Esc",
            ],
        ),
        prev: s(
            l,
            [
                "Previous page · ←",
                "Předchozí · ←",
                "Попередня · ←",
                "Предыдущая · ←",
            ],
        ),
        next: s(
            l,
            [
                "Next page · →",
                "Další · →",
                "Наступна · →",
                "Следующая · →",
            ],
        ),
        notes: s(
            l,
            [
                "Lida's notes",
                "Lidiny vzkazy",
                "Записки Ліди",
                "Записки Лиды",
            ],
        ),
        not_found: s(
            l,
            [
                "Not found yet",
                "Zatím nenalezeno",
                "Ще не знайдено",
                "Пока не найдено",
            ],
        ),
    }
}

pub fn captions(l: Lang) -> Vec<(String, String)> {
    let f = |v: [&str; 4]| s(l, v);
    vec![
        (
            f([
                "A quiet night. Rain on the window.",
                "Tichá noc. Déšť na okně.",
                "Тиха ніч. Дощ на вікні.",
                "Тихая ночь. Дождь на окне.",
            ]),
            f([
                "Rain and the first regular",
                "Déšť a první štamgast",
                "Дощ і перший завсідник",
                "Дождь и первый завсегдатай",
            ]),
        ),
        (
            f([
                "Mirek said it was exactly right.",
                "Mirek řekl, že to bylo přesně ono.",
                "Мірек сказав, що саме те.",
                "Мирек сказал, что в точку.",
            ]),
            f([
                "Black, like Lida's",
                "Černá jako od Lidy",
                "Чорна, як у Ліди",
                "Чёрный, как у Лиды",
            ]),
        ),
        (
            f([
                "The cat stayed. I think it lives here now.",
                "Kočka zůstala. Myslím, že tu teď bydlí.",
                "Кіт лишився. Здається, тепер живе тут.",
                "Кот остался. Кажется, теперь живёт тут.",
            ]),
            f(["Whiskers", "Vousky", "Вуса", "Усы"]),
        ),
        (
            f([
                "Every chair taken. Grandma would have loved it.",
                "Každá židle obsazená. Babička by to milovala.",
                "Усі стільці зайняті. Бабуся б це любила.",
                "Все стулья заняты. Бабушке бы понравилось.",
            ]),
            f(["Full house", "Plno", "Повна зала", "Полный зал"]),
        ),
        (
            f([
                "Vera laughed over her tea.",
                "Vera se smála nad čajem.",
                "Віра сміялася над чаєм.",
                "Вера смеялась над чаем.",
            ]),
            f([
                "Tea, properly made",
                "Čaj, jak se patří",
                "Чай як слід",
                "Чай как надо",
            ]),
        ),
    ]
}
