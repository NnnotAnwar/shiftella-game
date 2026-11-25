//! Внешность персонажа (`PersonLook`), позы и эмоции — чистые данные без рисования.
//!
//! `PersonLook::random(seed, staff)` детерминирован: один и тот же seed всегда
//! даёт одного и того же человека (удобно для сохранений и сети).

use macroquad::prelude::*;

/// Куда смотрит персонаж (вид 3/4 спереди: `Down` — лицом к камере).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Facing {
    #[default]
    Down,
    Up,
    Left,
    Right,
}

impl Facing {
    /// Направление по вектору движения (в мировых координатах, y вниз).
    pub fn from_dir(d: Vec2) -> Facing {
        if d.x.abs() > d.y.abs() {
            if d.x > 0.0 {
                Facing::Right
            } else {
                Facing::Left
            }
        } else if d.y < 0.0 {
            Facing::Up
        } else {
            Facing::Down
        }
    }
}

/// Что делает персонаж. `Carry` — идёт с подносом (ноги шагают).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pose {
    #[default]
    Idle,
    Walk,
    Sit,
    Carry,
    Work,
}

/// Где и как стоит персонаж в этом кадре.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PersonPose {
    /// Точка между ступнями, мировые пиксели. Для `Sit` — см. `sit_anchor`.
    pub feet: Vec2,
    pub facing: Facing,
    pub pose: Pose,
    /// Время анимации в секундах (может быть общим для всех — фазы разные).
    pub t: f32,
}

/// Значок-пузырь над головой.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Emote {
    Coffee,
    Heart,
    Zzz,
    Angry,
    Note,
    Music,
    Exclamation,
    Money,
}

impl Emote {
    pub const ALL: [Emote; 8] = [
        Emote::Coffee,
        Emote::Heart,
        Emote::Zzz,
        Emote::Angry,
        Emote::Note,
        Emote::Music,
        Emote::Exclamation,
        Emote::Money,
    ];
}

/// Выражение лица (для портретов в диалогах; те же имена у content-агента).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Expression {
    #[default]
    Neutral,
    Smile,
    Sad,
    Surprised,
    Thinking,
    Laugh,
    Tired,
}

impl Expression {
    pub const ALL: [Expression; 7] = [
        Expression::Neutral,
        Expression::Smile,
        Expression::Sad,
        Expression::Surprised,
        Expression::Thinking,
        Expression::Laugh,
        Expression::Tired,
    ];

    /// Разбор имени из данных (`"smile"`, `"Smile"`); неизвестное → `Neutral`.
    pub fn from_name(name: &str) -> Expression {
        match name.to_ascii_lowercase().as_str() {
            "smile" => Expression::Smile,
            "sad" => Expression::Sad,
            "surprised" => Expression::Surprised,
            "thinking" => Expression::Thinking,
            "laugh" => Expression::Laugh,
            "tired" => Expression::Tired,
            _ => Expression::Neutral,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HairStyle {
    Buzz,
    Short,
    Side,
    Bob,
    Long,
    Ponytail,
    Bun,
    Curly,
    Bald,
}

impl HairStyle {
    pub const ALL: [HairStyle; 9] = [
        HairStyle::Buzz,
        HairStyle::Short,
        HairStyle::Side,
        HairStyle::Bob,
        HairStyle::Long,
        HairStyle::Ponytail,
        HairStyle::Bun,
        HairStyle::Curly,
        HairStyle::Bald,
    ];
}

/// Фасон верха: меняет силуэт (пальто длиннее, у худи капюшон, у рубашки воротник).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopStyle {
    Tee,
    Sweater,
    Hoodie,
    Shirt,
    Cardigan,
    Jacket,
    Coat,
    Scrubs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accessory {
    None,
    Glasses,
    Scarf,
    Beanie,
    Headphones,
    Earrings,
    FlatCap,
    Hat,
    Hairclip,
    /// Маленький логотип Shiftella на груди (худи/свитер).
    ShiftellaLogo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacialHair {
    None,
    Stubble,
    Mustache,
    Beard,
}

/// Общий тон персонажа: обычный, «воспоминание» (тёплая сепия) или «блёклый» (гость 3:33).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Normal,
    Memory,
    Faded,
}

/// Внешность. Все цвета — базовые (до освещения).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PersonLook {
    pub skin: Color,
    pub hair: Color,
    pub hair_style: HairStyle,
    pub top: Color,
    pub top_style: TopStyle,
    pub bottom: Color,
    pub accessory: Accessory,
    /// Цвет аксессуара (шарф, шапка, очки).
    pub accent: Color,
    pub facial_hair: FacialHair,
    /// Индиговый фартук персонала со значком Shiftella.
    pub apron: bool,
    /// Масштаб роста, ~0.9…1.08 (1.0 ≈ 1.3 тайла).
    pub height: f32,
    /// Пожилой: морщинки у глаз, чуть сутулее.
    pub elder: bool,
    pub tone: Tone,
    /// Сдвиг фазы анимаций (моргание, дыхание), 0..1 — чтобы толпа не моргала хором.
    pub phase: f32,
    /// Не человек, а аватар @shiftella_bot (маленький индиговый робот).
    pub bot: bool,
    /// Чуть уставшие глаза (тени под глазами, тяжелее веки) при любом выражении.
    pub tired: bool,
}

const SKINS: [u32; 9] = [
    0xf7dcc8, 0xf0c6a6, 0xe3ad88, 0xd09872, 0xb67c58, 0x9a6444, 0x7d4c33, 0x5e3826, 0xf3d1b8,
];
const HAIRS: [u32; 9] = [
    0x2a1f22, 0x3e2a24, 0x5b3b2a, 0x8a5a36, 0xc79e68, 0xe2c99c, 0xa4462f, 0x8d8899, 0x4a3530,
];
const HAIRS_DYED: [u32; 3] = [0x8f7cf0, 0x3f9494, 0xd77fa6];
const TOPS: [u32; 14] = [
    0x3f8a8c, 0xc79a3e, 0xb5603f, 0x7f9a6a, 0xe8dcc6, 0x34406b, 0x7b4f8e, 0xd97a6a, 0x5b6f9a,
    0x9a5a6e, 0x4d6b57, 0xd6b8e0, 0x6d7891, 0xa8794f,
];
const STAFF_TOPS: [u32; 4] = [0xefe9f7, 0xe8dcc6, 0xd7d3ea, 0xf4efe6];
const BOTTOMS: [u32; 7] = [
    0x34406b, 0x2d2b3a, 0x5a4636, 0x4d5a3e, 0x3b4a6b, 0x6b5a4a, 0x24222e,
];
const ACCENTS: [u32; 8] = [
    0xc79a3e, 0xb5603f, 0x3f8a8c, 0x7b4f8e, 0xd97a6a, 0x6366f1, 0xe8dcc6, 0x9a5a6e,
];

/// Простой детерминированный генератор (splitmix64).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    fn f(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[(self.next() % items.len() as u64) as usize]
    }
    fn chance(&mut self, p: f32) -> bool {
        self.f() < p
    }
}

fn hex(c: u32) -> Color {
    Color::from_hex(c)
}

impl PersonLook {
    /// Случайная, но всегда одинаковая для `seed` внешность.
    /// `staff` — в фартуке, светлый верх, аккуратнее причёска.
    pub fn random(seed: u64, staff: bool) -> PersonLook {
        let mut r = Rng(seed ^ 0x5eed_cafe_u64.wrapping_mul(0x1000_0001));
        r.next();
        let elder = !staff && r.chance(0.14);
        let skin = hex(r.pick(&SKINS));
        let hair = if elder {
            hex(r.pick(&[0x8d8899, 0xc9c4cf, 0xe4e0e8, 0x6e6773]))
        } else if r.chance(0.1) {
            hex(r.pick(&HAIRS_DYED))
        } else {
            hex(r.pick(&HAIRS))
        };
        let hair_style = if elder {
            r.pick(&[
                HairStyle::Short,
                HairStyle::Bun,
                HairStyle::Curly,
                HairStyle::Bald,
                HairStyle::Bob,
            ])
        } else {
            r.pick(&HairStyle::ALL[..8])
        };
        let top_style = if staff {
            r.pick(&[TopStyle::Shirt, TopStyle::Tee, TopStyle::Sweater])
        } else {
            r.pick(&[
                TopStyle::Tee,
                TopStyle::Sweater,
                TopStyle::Hoodie,
                TopStyle::Shirt,
                TopStyle::Cardigan,
                TopStyle::Jacket,
                TopStyle::Coat,
            ])
        };
        let top = if staff {
            hex(r.pick(&STAFF_TOPS))
        } else {
            hex(r.pick(&TOPS))
        };
        let accessory = if staff {
            r.pick(&[
                Accessory::None,
                Accessory::None,
                Accessory::Glasses,
                Accessory::Hairclip,
                Accessory::Earrings,
            ])
        } else {
            r.pick(&[
                Accessory::None,
                Accessory::None,
                Accessory::Glasses,
                Accessory::Scarf,
                Accessory::Beanie,
                Accessory::Headphones,
                Accessory::Earrings,
                Accessory::FlatCap,
                Accessory::Hairclip,
            ])
        };
        let facial_hair = if matches!(
            hair_style,
            HairStyle::Buzz | HairStyle::Short | HairStyle::Bald
        ) && r.chance(0.35)
        {
            r.pick(&[FacialHair::Stubble, FacialHair::Mustache, FacialHair::Beard])
        } else {
            FacialHair::None
        };
        PersonLook {
            skin,
            hair,
            hair_style,
            top,
            top_style,
            bottom: hex(r.pick(&BOTTOMS)),
            accessory,
            accent: hex(r.pick(&ACCENTS)),
            facial_hair,
            apron: staff,
            height: 0.92 + r.f() * 0.15 - if elder { 0.03 } else { 0.0 },
            elder,
            tone: Tone::Normal,
            phase: r.f(),
            bot: false,
            tired: false,
        }
    }

    /// Готовые сюжетные персонажи. Неизвестный id → детерминированный случайный гость.
    ///
    /// Ids: `grandma_lida`, `mirek`, `hana`, `guest_333`, `vera`, `filip`, `radek`,
    /// `lori`, `player`, `shiftella_bot`. Префикс `guest_` можно опускать/добавлять.
    pub fn preset(id: &str) -> PersonLook {
        let base = |seed: u64| PersonLook::random(seed, false);
        let id = id.strip_prefix("guest_").unwrap_or(id);
        match id {
            "grandma_lida" | "lida" => PersonLook {
                skin: hex(0xf0c6a6),
                hair: hex(0xe8e3ec),
                hair_style: HairStyle::Bun,
                top: hex(0x7b4f8e),
                top_style: TopStyle::Cardigan,
                bottom: hex(0x4a3550),
                accessory: Accessory::Glasses,
                accent: hex(0xc79a3e),
                facial_hair: FacialHair::None,
                apron: false,
                height: 0.93,
                elder: true,
                tone: Tone::Memory,
                phase: 0.21,
                bot: false,
                tired: false,
            },
            "mirek" => PersonLook {
                skin: hex(0xe3ad88),
                hair: hex(0xb9b4c0),
                hair_style: HairStyle::Short,
                top: hex(0x6b5a3e),
                top_style: TopStyle::Jacket,
                bottom: hex(0x2d2b3a),
                accessory: Accessory::FlatCap,
                accent: hex(0x5a4636),
                facial_hair: FacialHair::Mustache,
                apron: false,
                height: 0.97,
                elder: true,
                tone: Tone::Normal,
                phase: 0.63,
                bot: false,
                tired: false,
            },
            "hana" => PersonLook {
                skin: hex(0xd09872),
                hair: hex(0x3e2a24),
                hair_style: HairStyle::Ponytail,
                top: hex(0x5aa6a8),
                top_style: TopStyle::Scrubs,
                bottom: hex(0x4b8f93),
                accessory: Accessory::None,
                accent: hex(0xe8dcc6),
                facial_hair: FacialHair::None,
                apron: false,
                height: 0.99,
                elder: false,
                tone: Tone::Normal,
                phase: 0.37,
                bot: false,
                tired: false,
            },
            "333" => PersonLook {
                skin: hex(0xe6d2c2),
                hair: hex(0x4a4650),
                hair_style: HairStyle::Short,
                top: hex(0x6f6e78),
                top_style: TopStyle::Coat,
                bottom: hex(0x3a3942),
                accessory: Accessory::Hat,
                accent: hex(0x3c3b45),
                facial_hair: FacialHair::None,
                apron: false,
                height: 1.04,
                elder: false,
                tone: Tone::Faded,
                phase: 0.333,
                bot: false,
                tired: false,
            },
            "vera" => PersonLook {
                skin: hex(0xf3d1b8),
                hair: hex(0xcfc9d6),
                hair_style: HairStyle::Curly,
                top: hex(0xb86b7f),
                top_style: TopStyle::Cardigan,
                bottom: hex(0x4a3550),
                accessory: Accessory::Glasses,
                accent: hex(0x9a7a3e),
                facial_hair: FacialHair::None,
                apron: false,
                height: 0.9,
                elder: true,
                tone: Tone::Normal,
                phase: 0.81,
                bot: false,
                tired: false,
            },
            "filip" => PersonLook {
                skin: hex(0xf0c6a6),
                hair: hex(0x5b3b2a),
                hair_style: HairStyle::Curly,
                top: hex(0x3f5f8a),
                top_style: TopStyle::Hoodie,
                bottom: hex(0x2d2b3a),
                accessory: Accessory::Headphones,
                accent: hex(0xc79a3e),
                facial_hair: FacialHair::None,
                apron: false,
                height: 1.05,
                elder: false,
                tone: Tone::Normal,
                phase: 0.12,
                bot: false,
                tired: false,
            },
            "radek" => PersonLook {
                skin: hex(0xb67c58),
                hair: hex(0x5a5058),
                hair_style: HairStyle::Buzz,
                top: hex(0x2f3b5f),
                top_style: TopStyle::Jacket,
                bottom: hex(0x24222e),
                accessory: Accessory::None,
                accent: hex(0xe0a53a),
                facial_hair: FacialHair::Beard,
                apron: false,
                height: 1.03,
                elder: false,
                tone: Tone::Normal,
                phase: 0.55,
                bot: false,
                tired: false,
            },
            "lori" => PersonLook {
                skin: hex(0xe3ad88),
                hair: hex(0xa4462f),
                hair_style: HairStyle::Long,
                top: hex(0xb5603f),
                top_style: TopStyle::Jacket,
                bottom: hex(0x34406b),
                accessory: Accessory::Beanie,
                accent: hex(0xc79a3e),
                facial_hair: FacialHair::None,
                apron: false,
                height: 0.96,
                elder: false,
                tone: Tone::Normal,
                phase: 0.47,
                bot: false,
                tired: false,
            },
            "player" => PersonLook {
                skin: hex(0xe3ad88),
                hair: hex(0x5b3b2a),
                hair_style: HairStyle::Side,
                top: hex(0xe8dcc6),
                top_style: TopStyle::Sweater,
                bottom: hex(0x34406b),
                accessory: Accessory::None,
                accent: hex(0x6366f1),
                facial_hair: FacialHair::None,
                apron: true,
                height: 1.0,
                elder: false,
                tone: Tone::Normal,
                phase: 0.0,
                bot: false,
                tired: false,
            },
            // Анвар — постоянный гость, разработчик приложения Shiftella (под конец 20-х)
            "anwar" => PersonLook {
                skin: hex(0xd09872),
                hair: hex(0x251c22),
                hair_style: HairStyle::Short,
                top: hex(0x2f2c72),
                top_style: TopStyle::Hoodie,
                bottom: hex(0x2d2b3a),
                accessory: Accessory::ShiftellaLogo,
                accent: hex(0x6366f1),
                facial_hair: FacialHair::Stubble,
                apron: false,
                height: 1.02,
                elder: false,
                tone: Tone::Normal,
                phase: 0.71,
                bot: false,
                tired: true,
            },
            "shiftella_bot" | "bot" => PersonLook {
                bot: true,
                height: 0.8,
                phase: 0.5,
                ..base(0x0b07)
            },
            other => base(other.bytes().fold(1469598103934665603u64, |h, b| {
                (h ^ b as u64).wrapping_mul(1099511628211)
            })),
        }
    }

    /// Все ids сюжетных пресетов.
    pub const PRESETS: [&'static str; 11] = [
        "grandma_lida",
        "mirek",
        "hana",
        "guest_333",
        "vera",
        "filip",
        "radek",
        "lori",
        "player",
        "anwar",
        "shiftella_bot",
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_look_is_deterministic() {
        for seed in 0..50 {
            assert_eq!(
                PersonLook::random(seed, false),
                PersonLook::random(seed, false)
            );
            assert_eq!(
                PersonLook::random(seed, true),
                PersonLook::random(seed, true)
            );
        }
    }

    #[test]
    fn random_looks_are_diverse() {
        let looks: Vec<_> = (0..60).map(|s| PersonLook::random(s, false)).collect();
        let styles: std::collections::HashSet<_> =
            looks.iter().map(|l| l.hair_style as u8).collect();
        assert!(styles.len() >= 6, "hair styles: {}", styles.len());
        let mut skins: Vec<u32> = looks.iter().map(|l| (l.skin.r * 255.0) as u32).collect();
        skins.sort();
        skins.dedup();
        assert!(skins.len() >= 5);
        assert!(looks.iter().all(|l| !l.apron));
        assert!(looks.iter().all(|l| (0.85..1.1).contains(&l.height)));
    }

    #[test]
    fn staff_wear_aprons() {
        assert!((0..20).all(|s| PersonLook::random(s, true).apron));
    }

    #[test]
    fn presets_resolve() {
        for id in PersonLook::PRESETS {
            let _ = PersonLook::preset(id);
        }
        assert_eq!(PersonLook::preset("guest_333").tone, Tone::Faded);
        assert_eq!(
            PersonLook::preset("guest_mirek"),
            PersonLook::preset("mirek")
        );
        assert!(PersonLook::preset("shiftella_bot").bot);
        let a = PersonLook::preset("anwar");
        assert!(a.tired && a.top_style == TopStyle::Hoodie && !a.apron);
        assert_eq!(a.accessory, Accessory::ShiftellaLogo);
        assert_eq!(
            PersonLook::preset("someone_new"),
            PersonLook::preset("someone_new")
        );
    }

    #[test]
    fn facing_from_dir() {
        assert_eq!(Facing::from_dir(vec2(1.0, 0.2)), Facing::Right);
        assert_eq!(Facing::from_dir(vec2(-1.0, 0.2)), Facing::Left);
        assert_eq!(Facing::from_dir(vec2(0.0, -1.0)), Facing::Up);
        assert_eq!(Facing::from_dir(vec2(0.1, 1.0)), Facing::Down);
    }

    #[test]
    fn expression_names() {
        assert_eq!(Expression::from_name("Laugh"), Expression::Laugh);
        assert_eq!(Expression::from_name("???"), Expression::Neutral);
    }
}
