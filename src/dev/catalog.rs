//! Every screen the dev panel's "Screens" section opens: a `SHIFTELLA_SCENE` scene of
//! `app::SceneBuilder::scene` plus its options (the same ones the screenshot runs set as env
//! vars — the module docs of `crate::app` list them all). Labels are English only: this is a
//! developer tool.

/// One entry of the "Screens" section.
#[derive(Debug, Clone, Copy)]
pub struct Opener {
    pub group: &'static str,
    pub label: &'static str,
    /// `SHIFTELLA_SCENE` name.
    pub scene: &'static str,
    /// `SHIFTELLA_*` options in force while the scene is built.
    pub opts: &'static [(&'static str, &'static str)],
}

const fn o(
    group: &'static str,
    label: &'static str,
    scene: &'static str,
    opts: &'static [(&'static str, &'static str)],
) -> Opener {
    Opener {
        group,
        label,
        scene,
        opts,
    }
}

const MENU: &str = "Menu & settings";
const START: &str = "Story start & tutorial";
const NIGHT: &str = "Night";
const PANELS: &str = "Panels & cards";
const APP: &str = "Shiftella app";
const JOURNAL: &str = "Journal & photos";
const RITUALS: &str = "Rituals & habits";
const MINI: &str = "Mini-games";
const MORNING: &str = "Morning & walk home";
const END: &str = "Endings & credits";
const SANDBOX: &str = "Sandbox";

/// The mini-games by `SHIFTELLA_MG` name (`app::SceneBuilder::scene`, `minigame`).
pub const MINIGAMES: &[&str] = &[
    "froth",
    "chocolate",
    "raf",
    "encore",
    "latte",
    "flat_white",
    "pour",
    "turek",
    "night_train",
    "tonic",
    "tea",
    "chai",
    "linden",
    "tatra",
    "uzvar",
    "rn333",
    "plate",
    "chlebicek",
    "kolac",
    "vera_kolac",
    "medovnik",
    "varenyky",
    "pirohy",
    "buchty",
    "soup",
    "kulajda",
    "borscht",
    "svarak",
];

/// The screens, in panel order (the mini-games follow, one per [`MINIGAMES`] entry: see
/// [`all`]).
pub const OPENERS: &[Opener] = &[
    // ── menu & settings ──
    o(MENU, "Main menu", "menu", &[]),
    o(MENU, "Language picker", "language", &[]),
    o(MENU, "New story (difficulty)", "newgame", &[]),
    o(MENU, "Sandbox setup", "sandbox_setup", &[]),
    o(
        MENU,
        "Sandbox setup · Challenge",
        "sandbox_setup",
        &[("SHIFTELLA_SB_MODE", "challenge")],
    ),
    o(
        MENU,
        "Settings · General",
        "settings",
        &[("SHIFTELLA_SETTINGS_TAB", "0")],
    ),
    o(
        MENU,
        "Settings · Accessibility",
        "settings",
        &[("SHIFTELLA_SETTINGS_TAB", "1")],
    ),
    o(
        MENU,
        "Settings · Controls",
        "settings",
        &[("SHIFTELLA_SETTINGS_TAB", "2")],
    ),
    // ── story start & tutorial ──
    o(START, "Opening scene", "opening", &[]),
    o(START, "Day 1", "story_day1", &[]),
    o(
        START,
        "Tutorial · day 1 coach marks",
        "story_day1",
        &[("SHIFTELLA_TUTORIAL", "0")],
    ),
    o(
        START,
        "Tutorial · step 4",
        "day",
        &[("SHIFTELLA_NIGHT", "1"), ("SHIFTELLA_TUTORIAL", "4")],
    ),
    o(START, "Night 1 · coach marks", "night1_handson", &[]),
    o(START, "Day 2", "day", &[("SHIFTELLA_NIGHT", "2")]),
    // ── night ──
    o(
        NIGHT,
        "Night 1 · 00:30",
        "night",
        &[("SHIFTELLA_NIGHT", "1"), ("SHIFTELLA_TIME", "00:30")],
    ),
    o(
        NIGHT,
        "Night 3 · 21:40 preparation",
        "night",
        &[("SHIFTELLA_NIGHT", "3"), ("SHIFTELLA_TIME", "21:40")],
    ),
    o(
        NIGHT,
        "Night 3 · 06:10 closing",
        "night",
        &[("SHIFTELLA_NIGHT", "3"), ("SHIFTELLA_TIME", "06:10")],
    ),
    o(NIGHT, "Pause menu", "pause", &[]),
    o(NIGHT, "Order card", "order_card", &[]),
    o(NIGHT, "Jukebox records", "jukebox", &[]),
    o(
        NIGHT,
        "Full house · to go",
        "to_go",
        &[("SHIFTELLA_NIGHT", "6")],
    ),
    o(
        NIGHT,
        "Strangers' names",
        "guest_names",
        &[("SHIFTELLA_NIGHT", "5")],
    ),
    o(NIGHT, "Anwar's visit (night 9)", "night9_anwar", &[]),
    o(NIGHT, "Moment card", "moments", &[]),
    o(NIGHT, "Garlands (night 21)", "garlands", &[]),
    o(NIGHT, "Station cat (night 12)", "cat", &[]),
    o(
        NIGHT,
        "Auto-pause · storm event",
        "auto_pause",
        &[("SHIFTELLA_EVENT", "storm")],
    ),
    o(
        NIGHT,
        "Stranger's story marker",
        "story_marker",
        &[("SHIFTELLA_NIGHT", "5")],
    ),
    o(
        NIGHT,
        "Stranger's story card",
        "story_card",
        &[("SHIFTELLA_NIGHT", "5")],
    ),
    o(NIGHT, "Storm night (29)", "storm", &[]),
    o(NIGHT, "Storm 03:00 talk", "storm_talk", &[]),
    o(NIGHT, "Night 14", "night14", &[]),
    o(NIGHT, "Night 22", "night22", &[]),
    o(NIGHT, "Night 27 · snow", "night27_snow", &[]),
    o(
        NIGHT,
        "Equipment · preparation",
        "equipment",
        &[("SHIFTELLA_EQUIP", "prep")],
    ),
    o(
        NIGHT,
        "Equipment · night",
        "equipment",
        &[("SHIFTELLA_EQUIP", "night")],
    ),
    o(
        NIGHT,
        "Equipment · worn",
        "equipment",
        &[("SHIFTELLA_EQUIP", "worn")],
    ),
    o(
        NIGHT,
        "Equipment · closing",
        "equipment",
        &[("SHIFTELLA_EQUIP", "closing")],
    ),
    o(NIGHT, "Talk in progress (Věra)", "talk", &[]),
    o(
        NIGHT,
        "Cutscene choice (Hana asleep)",
        "cutscene_choice",
        &[],
    ),
    // ── panels & cards ──
    o(PANELS, "Crew panel", "crew", &[]),
    o(PANELS, "Crew card · Karolina", "crew_karolina", &[]),
    o(PANELS, "Crew card · helper", "crew_helper", &[]),
    o(PANELS, "Crew card · Classic", "crew_classic", &[]),
    o(PANELS, "Crew card · by day", "crew_day", &[]),
    o(PANELS, "Crew card · hiring", "crew_hire", &[]),
    o(PANELS, "Duties board", "duties", &[]),
    o(PANELS, "Night log", "log", &[]),
    o(PANELS, "Build mode (day)", "build", &[]),
    o(PANELS, "Build mode · dragging a table", "build_drag", &[]),
    o(PANELS, "Decor themes", "themes", &[]),
    o(PANELS, "Grandma's attic", "attic", &[]),
    o(PANELS, "Back room", "backroom", &[]),
    o(PANELS, "Build requests", "requests", &[]),
    o(PANELS, "A corner", "corner", &[]),
    o(PANELS, "Before / after", "before_after", &[]),
    o(PANELS, "Recipe book", "book", &[]),
    o(PANELS, "Pantry experiment", "pantry", &[]),
    o(PANELS, "Grandma's napkins", "napkins", &[]),
    o(PANELS, "Payroll basis", "payroll", &[]),
    o(PANELS, "House drink · compose", "signature_compose", &[]),
    o(PANELS, "House drink · board", "signature_board", &[]),
    o(PANELS, "House drink · order", "signature_order", &[]),
    // ── the Shiftella app ──
    o(APP, "App · Schedule (rozpis)", "rozpis", &[]),
    o(
        APP,
        "App · Schedule auto-filled",
        "rozpis",
        &[("SHIFTELLA_AUTOFILL", "1")],
    ),
    o(APP, "App · Requests", "app", &[("SHIFTELLA_APP_TAB", "1")]),
    o(APP, "App · Bot", "app", &[("SHIFTELLA_APP_TAB", "2")]),
    o(APP, "App · Board", "app", &[("SHIFTELLA_APP_TAB", "3")]),
    o(
        APP,
        "App · Checklists",
        "app",
        &[("SHIFTELLA_APP_TAB", "4")],
    ),
    o(APP, "App · Digest", "app", &[("SHIFTELLA_APP_TAB", "5")]),
    o(APP, "App · Payroll", "app", &[("SHIFTELLA_APP_TAB", "6")]),
    // ── journal & photos ──
    o(
        JOURNAL,
        "Journal · page 1",
        "journal",
        &[("SHIFTELLA_PAGE", "0")],
    ),
    o(
        JOURNAL,
        "Journal · page 2",
        "journal",
        &[("SHIFTELLA_PAGE", "1")],
    ),
    o(
        JOURNAL,
        "Journal · page 3",
        "journal",
        &[("SHIFTELLA_PAGE", "2")],
    ),
    o(
        JOURNAL,
        "Journal · page 4",
        "journal",
        &[("SHIFTELLA_PAGE", "3")],
    ),
    o(
        JOURNAL,
        "Journal · page 5",
        "journal",
        &[("SHIFTELLA_PAGE", "4")],
    ),
    o(
        JOURNAL,
        "Journal · Lida's notes",
        "journal",
        &[("SHIFTELLA_PAGE", "5")],
    ),
    o(
        JOURNAL,
        "Journal · enlarged polaroid",
        "journal",
        &[("SHIFTELLA_PAGE", "0"), ("SHIFTELLA_OPEN", "0")],
    ),
    o(JOURNAL, "Journal · extra photos", "journal_photos", &[]),
    o(JOURNAL, "Morning polaroid", "polaroid", &[]),
    o(JOURNAL, "Photo mode", "photo_mode", &[]),
    o(JOURNAL, "Photo print", "photo_print", &[]),
    // ── rituals & habits ──
    o(
        RITUALS,
        "Checklist states",
        "hands_ux",
        &[("SHIFTELLA_HUX", "checklist")],
    ),
    o(
        RITUALS,
        "Rituals",
        "hands_ux",
        &[("SHIFTELLA_HUX", "rituals")],
    ),
    o(
        RITUALS,
        "Queue strip",
        "hands_ux",
        &[("SHIFTELLA_HUX", "queue")],
    ),
    o(
        RITUALS,
        "Follow chip",
        "hands_ux",
        &[("SHIFTELLA_HUX", "follow")],
    ),
    o(
        RITUALS,
        "Gesture · light the window",
        "hands_ux",
        &[
            ("SHIFTELLA_HUX", "gesture"),
            ("SHIFTELLA_GESTURE", "light_window"),
        ],
    ),
    o(
        RITUALS,
        "Gesture · count the float",
        "hands_ux",
        &[
            ("SHIFTELLA_HUX", "gesture"),
            ("SHIFTELLA_GESTURE", "count_float"),
        ],
    ),
    o(
        RITUALS,
        "Gesture · cash up",
        "hands_ux",
        &[
            ("SHIFTELLA_HUX", "gesture"),
            ("SHIFTELLA_GESTURE", "cash_up"),
            ("SHIFTELLA_TIME", "06:05"),
        ],
    ),
    o(
        RITUALS,
        "All by hand (app checklists)",
        "app",
        &[
            ("SHIFTELLA_APP_TAB", "4"),
            ("SHIFTELLA_BYHAND", "all"),
            ("SHIFTELLA_HABITS", "4"),
            ("SHIFTELLA_HABITS_SCROLL", "1"),
        ],
    ),
    o(
        RITUALS,
        "Grandma's habit card",
        "night",
        &[
            ("SHIFTELLA_NIGHT", "6"),
            ("SHIFTELLA_HABIT_CARD", "record_on"),
        ],
    ),
    // ── morning & walk home ──
    o(MORNING, "Morning summary", "summary", &[]),
    o(
        MORNING,
        "Morning summary · fridge full",
        "summary",
        &[("SHIFTELLA_NIGHT", "4"), ("SHIFTELLA_FRIDGE", "full")],
    ),
    o(
        MORNING,
        "Morning summary · bought for the café",
        "summary",
        &[("SHIFTELLA_NIGHT", "3"), ("SHIFTELLA_BOUGHT", "1200")],
    ),
    o(
        MORNING,
        "Day card · fridge worn",
        "day",
        &[("SHIFTELLA_NIGHT", "4"), ("SHIFTELLA_FRIDGE", "worn")],
    ),
    o(MORNING, "Walk home · pick a route", "walk_home", &[]),
    o(
        MORNING,
        "Walk home · embankment",
        "walk_home",
        &[("SHIFTELLA_ROUTE", "embankment"), ("SHIFTELLA_T", "4")],
    ),
    o(
        MORNING,
        "Walk home · park",
        "walk_home",
        &[("SHIFTELLA_ROUTE", "park"), ("SHIFTELLA_T", "4")],
    ),
    o(
        MORNING,
        "Walk home · bakery",
        "walk_home",
        &[("SHIFTELLA_ROUTE", "bakery"), ("SHIFTELLA_T", "4")],
    ),
    o(
        MORNING,
        "Walk home · tram",
        "walk_home",
        &[("SHIFTELLA_ROUTE", "tram"), ("SHIFTELLA_T", "4")],
    ),
    o(MORNING, "Walk home · rain", "walk_home_rain", &[]),
    o(MORNING, "Walk home · snow", "walk_home_snow", &[]),
    o(MORNING, "Back step", "backstep", &[]),
    o(MORNING, "Back step · snow", "backstep_snow", &[]),
    // ── endings & credits ──
    o(END, "Ending · the light", "ending_light", &[]),
    o(END, "Ending · the last train", "ending_last_train", &[]),
    o(END, "Credits", "credits", &[]),
    // ── sandbox ──
    o(SANDBOX, "Sandbox night", "sandbox", &[]),
    o(
        SANDBOX,
        "Challenge · morning with rivals",
        "challenge_summary",
        &[("SHIFTELLA_NIGHT", "12")],
    ),
    o(
        SANDBOX,
        "Challenge · final page (slow)",
        "challenge_final",
        &[],
    ),
];

/// One mini-game opener (`minigame` with `SHIFTELLA_MG`, played half a second in).
fn minigame(name: &'static str) -> Opener {
    // the option slices must be 'static: one per game, leaked once
    let opts: &'static [(&'static str, &'static str)] = Box::leak(Box::new([
        ("SHIFTELLA_MG", name),
        ("SHIFTELLA_MG_T", "0.5"),
    ]));
    Opener {
        group: MINI,
        label: name,
        scene: "minigame",
        opts,
    }
}

/// Every opener: [`OPENERS`] with the mini-games inserted after the rituals.
pub fn all() -> &'static [Opener] {
    static ALL: std::sync::OnceLock<Vec<Opener>> = std::sync::OnceLock::new();
    ALL.get_or_init(|| {
        let at = OPENERS
            .iter()
            .position(|o| o.group == MORNING)
            .unwrap_or(OPENERS.len());
        let mut v: Vec<Opener> = OPENERS[..at].to_vec();
        v.extend(MINIGAMES.iter().map(|m| minigame(m)));
        v.extend_from_slice(&OPENERS[at..]);
        v
    })
}

/// The groups in panel order.
pub fn groups() -> Vec<&'static str> {
    let mut g: Vec<&'static str> = Vec::new();
    for o in all() {
        if !g.contains(&o.group) {
            g.push(o.group);
        }
    }
    g
}
