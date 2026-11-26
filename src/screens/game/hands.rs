//! Karolina behind the counter (design/handson.md §1, §2, §7.4): the logic half of the
//! player-first loop on the game screen. No `Ui` here, so the whole night can be driven
//! headless (tests, the autoplayer).
//!
//! Every way of playing goes through the same small set of [`Intent`]s: a mouse click in
//! the world, a hotkey, a ticket on the rail and the autoplayer all produce an intent, and
//! [`GameScreen::intent`] turns it into sim commands (`PlayerTakeOrder`, `PlayerQueue`,
//! `PlayerGreet`, …). The order card, the mini-game and the recipe book are overlays: while
//! one is open the sim runs at `overlay_scale` and at most ×1 (see [`GameScreen::sim_rate`]).

use std::collections::{HashMap, VecDeque};

use macroquad::prelude::{Vec2, vec2};

use super::world_view::sim;
use super::{GameScreen, Notice, Panel};
use crate::content::Content;
use crate::content::handson::{MatchTier, MiniGameKind as ContentGame, RecipeDef};
use crate::map::TILE;
use crate::minigame::{MiniGame, MiniGameKind, MiniGameParams, MiniGameText, Variation};
use crate::render::{CupGlyph, LatteArt, QueueIcon, TipCoin};
use crate::ui::handson::{BookState, BrewMode as UiBrew, DutiesState, RailState, RecordState};
use crate::ui::{Icon, ToastKind};
use sim::{
    BrewMode, Command, ItemState, OrderItem, Party, PartyState, Phase, PlayerAct, SimEvent,
    TicketStage, World,
};

/// Real seconds between two decisions of the autoplayer (a calm human).
const AUTOPLAY_REACT: f32 = 1.2;
/// Hold-to-wash progress is sent at least this often while the button is held (real s).
const WASH_SEND_EVERY: f32 = 0.25;
/// A mop swipe must travel this far (world px) across the blot.
pub const SWIPE_MIN: f32 = 22.0;

/// Something the player wants Karolina to do (mouse, keyboard, rail, autoplayer).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Intent {
    /// Clicked a guest: greet, take the order, open their order card or serve them.
    Guest { guest: u32, front: bool },
    /// Clicked a table: collect its tip coins / carry the cups to the sink.
    Table { table: u32, front: bool },
    /// Clicked the sink: go and wash (then hold to wash).
    Sink { sink: u32, front: bool },
    /// Swiped a dirt blot.
    Mop { tile: (i32, i32), front: bool },
    /// Clicked a tip coin.
    Tip { table: u32, front: bool },
    /// Clicked a window: the light in the window (the opening checklist item).
    Window,
    /// Clicked grandma's jukebox: the record picker.
    Jukebox,
    /// A ticket on the rail (click / Enter).
    Ticket { party: u32, guest: u32 },
    /// Right-click / Backspace: cancel what she is doing now.
    Cancel,
    /// A queue chip over her head (0 = the current act when there is one).
    Unqueue(usize),
    /// Held the sink (mouse or **E**) for `seconds` while she stands at it.
    WashHold { sink: u32, seconds: f32 },
    /// **E**: the nearest chore (a tip, a dirty table, the dishes, a dirt blot, the window).
    Chore { front: bool },
    /// **G**: wave at the newest guest who just came in.
    Greet,
}

/// An open order card.
#[derive(Debug, Clone, PartialEq)]
pub struct OrderOpen {
    /// The guest the card is about (leader for wishes, the item's guest for plain items).
    pub guest: u32,
    pub party: u32,
    pub mode: UiBrew,
    /// `GameScreen::time` when it opened.
    pub opened: f32,
}

/// A running mini-game.
pub struct MgOpen {
    pub game: MiniGame,
    pub guest: u32,
    pub recipe: String,
    /// "The usual?" (`PlayerUsual` when done).
    pub usual: bool,
    /// Title and prompts were set in the player's language (the first drawn frame).
    pub named: bool,
    /// `PlayerBrewStart` went through: Karolina is already at the station; the quality
    /// follows with `PlayerBrewQuality` when the game ends.
    pub started: bool,
}

/// The recipe book (K): `pick_for` = opened from an order card (pick mode).
#[derive(Debug, Clone, Default)]
pub struct BookOpen {
    pub pick_for: Option<u32>,
    pub mode: UiBrew,
    pub state: BookState,
    /// The last line on the pantry page (an experiment's result, "not for this guest").
    pub note: Option<BookNote>,
    /// Lida's latest margin note (the silhouette it revealed).
    pub margin: Option<String>,
    /// The napkins page beside the book is open.
    pub napkins: bool,
}

/// A line on the book's pantry page: an i18n key + args (`@key` values), its kind and the
/// recipe it is about.
#[derive(Debug, Clone, PartialEq)]
pub struct BookNote {
    pub kind: crate::ui::handson::ExperimentKind,
    pub key: String,
    pub args: Vec<(String, String)>,
    pub recipe: Option<String>,
}

/// A floating label in the world (tips, the match tier of a serve).
#[derive(Debug, Clone, PartialEq)]
pub struct Float {
    pub at: Vec2,
    pub text: String,
    pub born: f32,
    /// i18n key + args, translated when drawn (the logic half has no locale).
    pub args: Vec<(String, String)>,
}

/// A coin on a table as the player sees it (the sim's `TipCoin` id → a render coin).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoinVis {
    pub id: u32,
    pub table: u32,
    pub coin: TipCoin,
}

/// A mop swipe in progress.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Swipe {
    pub tile: (i32, i32),
    pub last: Vec2,
    pub travelled: f32,
}

/// Hold-to-wash bookkeeping.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WashHold {
    pub sink: u32,
    /// Real seconds held now (for the ring).
    pub held: f32,
    /// Held but not yet sent to the sim.
    pub unsent: f32,
}

/// The autoplayer (`SHIFTELLA_AUTOPLAY=1|keys`): plays the night through the same intents
/// a person would use, mini-games with their scripted input.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Autoplay {
    pub cooldown: f32,
    /// Mini-games by keyboard (`SHIFTELLA_AUTOPLAY=keys`).
    pub keyboard: bool,
    /// Every mini-game skipped at once (`SHIFTELLA_AUTOPLAY=skip`; QA: Relaxed + skip all).
    pub skip_minigames: bool,
}

/// Counters of things the coach marks wait for (this night).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TutTally {
    pub counts: HashMap<&'static str, u32>,
}

impl TutTally {
    pub fn bump(&mut self, what: &'static str) {
        *self.counts.entry(what).or_insert(0) += 1;
    }
    pub fn get(&self, what: &str) -> u32 {
        self.counts.get(what).copied().unwrap_or(0)
    }
}

/// Everything the hands-on loop keeps between frames.
#[derive(Default)]
pub struct Hands {
    pub order: Option<OrderOpen>,
    /// An open night-moment card (`moments_ui`).
    pub moment: Option<super::moments_ui::MomentOpen>,
    pub minigame: Option<MgOpen>,
    pub book: Option<BookOpen>,
    /// The record picker is open.
    pub records_open: bool,
    /// The picker's toggle: one pass, then the house playlist (default: Repeat).
    pub play_once: bool,
    pub records: RecordState,
    /// Guests whose order was taken while another overlay was open (cards wait in line).
    pub pending_cards: VecDeque<u32>,
    pub coins: Vec<CoinVis>,
    pub floats: Vec<Float>,
    pub wash: Option<WashHold>,
    pub swipe: Option<Swipe>,
    /// Shown after a click (not a swipe) on a dirt blot: `(tile, since)`.
    pub swipe_hint: Option<((i32, i32), f32)>,
    pub swipe_hints_shown: u32,
    pub rail: RailState,
    pub duties: DutiesState,
    /// Regulars announced on the rail: (regular, party, until game minute).
    pub incoming: Vec<(String, u32, u32)>,
    /// "It's quiet — ×2?" until `GameScreen::time`.
    pub lull_until: f32,
    /// Cozy: a glass of water over a party (until `GameScreen::time`).
    pub water: HashMap<u32, f32>,
    /// A play-once record and the real seconds it has played.
    pub record_play: Option<(String, f32)>,
    pub tut: TutTally,
    pub autoplay: Option<Autoplay>,
    /// Real seconds the autoplayer has watched the current cutscene.
    pub scene_wait: f32,
    /// Screenshot runs (`SHIFTELLA_SKIP_SCENES=1`): story scenes finish at once.
    pub skip_scenes: bool,
    /// Karolina's act last frame (id, stage, target) — finished acts make their sound.
    pub prev_act: Option<(PlayerAct, u8)>,
    /// `GameScreen::time` of the last coin landing in the jar (the HUD chip pulses).
    pub jar_pulse: f32,
    /// Mouse went down on a world target this frame (so the release knows what it was).
    pub press_sink: Option<u32>,
    /// Right button pressed at this screen point (a click cancels, a drag pans).
    pub right_press: Option<Vec2>,
    /// A thank-you napkin just collected: (id, `GameScreen::time`).
    pub napkin: Option<(String, f32)>,
    /// The coziness level last announced tonight (level-up toasts only go up).
    pub atmo_level: u32,
    /// The "Count the jar" card is open.
    pub jar_open: bool,
    /// `GameScreen::time` of the last polaroid candidate (the HUD flash).
    pub flash_at: f32,
    /// Recipes discovered since the book was last opened on them (the amber dot).
    pub new_recipes: std::collections::BTreeSet<String>,
    /// The last experiment result (and Lida's last margin note) while the book was closed.
    pub last_experiment: Option<BookNote>,
    pub last_margin: Option<String>,
    /// The autoplayer's day: the sim's player-bot plans it (hire, rozpis, payroll, themes).
    pub day_bot: Option<sim::bot::PlayerBot>,
    /// Real seconds the autoplayer has looked at the day / the morning summary.
    pub day_wait: f32,
    /// The autoplayer tried tonight's experiment.
    pub tried_tonight: bool,
    /// Real seconds tonight with an overlay open (card, mini-game, book, moment, the phone)
    /// and in all — the night-length check (`game_autoplay`).
    pub overlay_real: f32,
    pub night_real: f32,
}

impl Hands {
    /// Order card, mini-game or the book is open (the sim slows down).
    pub fn overlay(&self) -> bool {
        self.order.is_some()
            || self.minigame.is_some()
            || self.book.is_some()
            || self.moment.is_some()
    }
}

/// Converts the UI brew toggle to the sim's.
pub fn sim_mode(m: UiBrew) -> BrewMode {
    match m {
        UiBrew::ByHand => BrewMode::ByHand,
        UiBrew::Quick => BrewMode::Quick,
    }
}

pub fn ui_mode(m: BrewMode) -> UiBrew {
    match m {
        BrewMode::ByHand => UiBrew::ByHand,
        BrewMode::Quick => UiBrew::Quick,
    }
}

/// The cup a menu item / recipe is served in (render glyphs, queue icons).
pub fn cup_of(content: &Content, recipe_or_menu: &str) -> CupGlyph {
    let menu = content
        .handson
        .recipe(recipe_or_menu)
        .map(|r| r.base_item.as_str())
        .unwrap_or(recipe_or_menu);
    if recipe_or_menu == "r_moon_latte" {
        return CupGlyph::Special;
    }
    match menu {
        "cappuccino" | "oat_flat_white" => CupGlyph::Latte,
        "lemon_tea" | "chai" => CupGlyph::Tea,
        "hot_chocolate" => CupGlyph::Chocolate,
        "svarak" => CupGlyph::MulledWine,
        "garlic_soup" | "grandma_kulajda" => CupGlyph::Soup,
        "kolac" | "medovnik" => CupGlyph::Pastry,
        "topinka" | "chlebicek" | "varenyky" | "bryndzove_pirohy" => CupGlyph::Plate,
        _ => CupGlyph::Espresso,
    }
}

/// The content mini-game of a recipe → the playable game and its variant id.
pub fn playable(recipe: &RecipeDef, seed: u64) -> (MiniGameKind, String) {
    let (kind, first) = recipe.playable_minigame(false);
    let kind = match kind {
        ContentGame::MilkFroth => MiniGameKind::MilkFroth,
        ContentGame::LatteArt => MiniGameKind::LatteArt,
        ContentGame::PourOver => MiniGameKind::PourOver,
        ContentGame::TeaSteep => MiniGameKind::TeaSteep,
        // the SHOULD games are not built yet: their recipes carry a MUST fallback
        ContentGame::Plate | ContentGame::SoupStir | ContentGame::MulledSpice => {
            MiniGameKind::Plate
        }
    };
    // the main game picks one of its variants by seed; a fallback uses its own variant —
    // except the soups, which plate their own bowl (česnečka, kulajda, borscht)
    if recipe.minigame == ContentGame::SoupStir && kind == MiniGameKind::Plate {
        let v = recipe
            .variants
            .first()
            .map(String::as_str)
            .unwrap_or("soup");
        let set = crate::minigame::PlateSet::from_id(v).unwrap_or(crate::minigame::PlateSet::Soup);
        return (kind, set.id().to_string());
    }
    let variant = if kind_matches(recipe.minigame, kind) && !recipe.variants.is_empty() {
        recipe.variants[(seed % recipe.variants.len() as u64) as usize].clone()
    } else {
        first.to_string()
    };
    (kind, variant)
}

fn kind_matches(c: ContentGame, k: MiniGameKind) -> bool {
    matches!(
        (c, k),
        (ContentGame::MilkFroth, MiniGameKind::MilkFroth)
            | (ContentGame::LatteArt, MiniGameKind::LatteArt)
            | (ContentGame::PourOver, MiniGameKind::PourOver)
            | (ContentGame::TeaSteep, MiniGameKind::TeaSteep)
            | (ContentGame::Plate, MiniGameKind::Plate)
    )
}

/// The icon of a queued act.
pub fn queue_icon(content: &Content, w: &World, act: &PlayerAct) -> QueueIcon {
    let item_cup = |id: u32| {
        w.order_items()
            .iter()
            .find(|i| i.id == id)
            .map(|i| match &i.origin {
                sim::ItemOrigin::Player { recipe, .. } => cup_of(content, recipe),
                _ => cup_of(content, &i.menu),
            })
            .unwrap_or_default()
    };
    match act {
        PlayerAct::TakeOrder { .. } => QueueIcon::TakeOrder,
        PlayerAct::Brew { item, .. } => QueueIcon::Brew(item_cup(*item)),
        PlayerAct::Serve { item, .. } => QueueIcon::Serve(item_cup(*item)),
        PlayerAct::ClearTable { .. } => QueueIcon::Clear,
        PlayerAct::Wash { .. } => QueueIcon::Wash,
        PlayerAct::Mop { .. } => QueueIcon::Mop,
        PlayerAct::CollectTip { .. } => QueueIcon::CollectTip,
        PlayerAct::Checklist { .. } => QueueIcon::Light,
        PlayerAct::PlayRecord { .. } => QueueIcon::Moment(crate::render::MomentMarkerKind::Jukebox),
        PlayerAct::GoTo { tile } => {
            // a moment's option or a night experiment waits at the end of the walk
            let m = &w.systems().moments;
            match m
                .walk_now
                .iter()
                .chain(m.walks.iter())
                .find(|j| j.tile == *tile)
                .map(|j| &j.kind)
            {
                Some(sim::WalkKind::Moment { id, .. }) => {
                    QueueIcon::Moment(crate::render::MomentMarkerKind::for_moment(id))
                }
                Some(sim::WalkKind::Experiment { .. }) => QueueIcon::Brew(CupGlyph::Special),
                None => QueueIcon::Walk,
            }
        }
    }
}

/// Karolina's acts, current first (what the chips over her head show).
pub fn karolina_acts(w: &World) -> Vec<PlayerAct> {
    let p = w.player();
    p.current
        .iter()
        .map(|c| c.act.clone())
        .chain(p.queue.iter().cloned())
        .collect()
}

/// The table top of a placed table (world px): where coins and cups lie.
pub fn table_top(w: &World, content: &Content, table: u32) -> Option<Vec2> {
    let p = w.furniture_by_uid(table)?;
    let def = content.furniture_def(&p.def)?;
    let (sw, sh) = sim::grid::rotated_size(def, p.rot);
    Some(vec2(
        (p.x as f32 + sw as f32 / 2.0) * TILE,
        (p.y as f32 + sh as f32 / 2.0) * TILE - 30.0,
    ))
}

/// The tip jar on the counter (world px of its mouth): next to the register.
pub fn jar_pos(w: &World, content: &Content) -> Vec2 {
    let reg = w.furniture().iter().find(|p| {
        content
            .furniture_def(&p.def)
            .is_some_and(|d| d.has_tag("register"))
    });
    match reg {
        Some(p) => vec2(p.x as f32 * TILE - 10.0, p.y as f32 * TILE + 14.0),
        None => vec2(20.0 * TILE, 10.0 * TILE),
    }
}

fn has_tag(content: &Content, w: &World, uid: u32, tag: &str) -> bool {
    w.furniture_by_uid(uid)
        .and_then(|p| content.furniture_def(&p.def))
        .is_some_and(|d| d.has_tag(tag))
}

/// The first pending plain item of a party nobody makes yet (Karolina may take it).
fn open_item<'a>(w: &'a World, p: &Party, guest: Option<u32>) -> Option<&'a OrderItem> {
    w.order_items()
        .iter()
        .filter(|i| {
            i.party == p.id
                && i.state == ItemState::Pending
                && i.by.is_none()
                && matches!(i.origin, sim::ItemOrigin::Unassigned)
        })
        .min_by_key(|i| (guest.is_some_and(|g| i.guest != g), i.id))
}

/// A ready item helpers made that nobody carries yet.
fn ready_item<'a>(w: &'a World, p: &Party) -> Option<&'a OrderItem> {
    w.order_items()
        .iter()
        .find(|i| i.party == p.id && i.state == ItemState::Ready && i.by.is_none())
}

/// Can she wave at this party now?
fn greetable(content: &Content, w: &World, p: &Party) -> bool {
    !p.greeted
        && p.state != PartyState::Leaving
        && w.minute().saturating_sub(p.arrived) <= content.handson.tuning.greeting.window_minutes
}

/// The closing ritual's last item is due: the light in the window goes off at six.
pub fn lights_off_due(g: &GameScreen) -> bool {
    g.world.minute() >= 466
        && g.world
            .app()
            .checklist
            .iter()
            .any(|c| c.id == "light_off_at_six" && !c.done && c.claim.is_none())
}

/// Is this act already in her hands (current or queued)?
fn in_hands(w: &World, act: &PlayerAct) -> bool {
    karolina_acts(w).iter().any(|a| a == act)
}

impl GameScreen {
    /// Sim time multiplier for this frame (speed × overlay scale, ×1 at most under an
    /// overlay) and the real seconds per tick.
    pub fn sim_rate(&self, content: &Content) -> (f32, f32) {
        let tick = self.world.real_seconds_per_tick(content);
        if self.overlay_open() {
            let speed = self.speed.min(1) as f32;
            (speed * self.world.overlay_scale(content), tick)
        } else {
            (self.speed as f32, tick)
        }
    }

    /// An overlay that slows the night is open (order card, mini-game, book, the phone).
    pub fn overlay_open(&self) -> bool {
        self.hands.overlay() || self.panel == Some(Panel::App)
    }

    fn toast_key(&mut self, kind: ToastKind, icon: Icon, key: &str, args: &[(&str, String)]) {
        self.notices.push(Notice::Key {
            kind,
            icon,
            key: key.to_string(),
            args: args
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        });
    }

    /// A command from the hands-on loop: a full queue says so gently, other refusals stay
    /// quiet (the click simply did nothing).
    pub fn hand(&mut self, content: &Content, cmd: Command) -> bool {
        match self.world.apply(content, cmd) {
            Ok(()) => true,
            Err(sim::CommandError::QueueFull) => {
                self.toast_key(ToastKind::Info, Icon::Info, "ui.queue.full", &[]);
                false
            }
            Err(_) => false,
        }
    }

    fn queue(&mut self, content: &Content, act: PlayerAct, front: bool) -> bool {
        if in_hands(&self.world, &act) {
            return false;
        }
        self.hand(content, Command::PlayerQueue { act, front })
    }

    /// Does what the player asked. Returns whether anything happened.
    pub fn intent(&mut self, content: &Content, i: Intent) -> bool {
        if self.world.phase() != Phase::Night && !matches!(i, Intent::Jukebox) {
            return false;
        }
        match i {
            Intent::Guest { guest, front } => self.guest_intent(content, guest, front),
            Intent::Ticket { party, guest } => {
                let target = self
                    .world
                    .parties()
                    .iter()
                    .find(|p| p.id == party)
                    .and_then(|p| p.leader())
                    .unwrap_or(guest);
                self.selected = Some(sim::AgentRef::Guest(guest));
                self.guest_intent(content, target, false)
                    || self.guest_intent(content, guest, false)
            }
            Intent::Table { table, front } => {
                let Some(p) = self.world.furniture_by_uid(table) else {
                    return false;
                };
                let tip = self.world.tips_on_tables().iter().any(|t| t.table == table);
                if p.dishes > 0 {
                    self.queue(content, PlayerAct::ClearTable { table }, front)
                } else if tip {
                    self.queue(content, PlayerAct::CollectTip { table }, front)
                } else {
                    false
                }
            }
            Intent::Tip { table, front } => {
                // clearing the table takes the coins too; otherwise just the coins
                let dishes = self
                    .world
                    .furniture_by_uid(table)
                    .is_some_and(|p| p.dishes > 0);
                if in_hands(&self.world, &PlayerAct::ClearTable { table }) {
                    return false;
                }
                if dishes {
                    self.queue(content, PlayerAct::ClearTable { table }, front)
                } else {
                    self.queue(content, PlayerAct::CollectTip { table }, front)
                }
            }
            Intent::Sink { sink, front } => {
                if self.world.sink_dishes() == 0 {
                    return false;
                }
                self.queue(content, PlayerAct::Wash { sink }, front)
            }
            Intent::WashHold { sink, seconds } => {
                self.hand(content, Command::PlayerWashProgress { sink, seconds })
            }
            Intent::Mop { tile, front } => {
                let busy = karolina_acts(&self.world)
                    .iter()
                    .any(|a| matches!(a, PlayerAct::Mop { tile: t } if *t == tile));
                if busy {
                    return false;
                }
                let ok = self.queue(content, PlayerAct::Mop { tile }, front);
                if ok {
                    self.scene.shine.push((tile, self.time + 0.6));
                }
                ok
            }
            Intent::Window => {
                // at closing (from 05:46) the window lamp goes off — the last ritual
                if lights_off_due(self) {
                    return self.hand(
                        content,
                        Command::TickChecklist {
                            id: "light_off_at_six".into(),
                        },
                    );
                }
                // (a lamp still on from last night — no light at six — counts once she
                // has checked it: the opening item is ticked either way)
                let open = self
                    .world
                    .app()
                    .checklist
                    .iter()
                    .any(|c| c.id == "light_window" && !c.done && c.claim.is_none());
                if !open && self.world.window_light_on(content) {
                    return false;
                }
                open && self.hand(
                    content,
                    Command::TickChecklist {
                        id: "light_window".into(),
                    },
                )
            }
            Intent::Jukebox => {
                self.hands.records_open = !self.hands.records_open;
                true
            }
            Intent::Cancel => self.hand(content, Command::PlayerCancel),
            Intent::Unqueue(i) => {
                if self.world.player().current.is_some() {
                    if i == 0 {
                        self.hand(content, Command::PlayerCancel)
                    } else {
                        self.hand(content, Command::PlayerUnqueue { index: i - 1 })
                    }
                } else {
                    self.hand(content, Command::PlayerUnqueue { index: i })
                }
            }
            Intent::Chore { front } => match self.next_chore(content) {
                Some(i) => {
                    let i = match i {
                        Intent::Table { table, .. } => Intent::Table { table, front },
                        Intent::Tip { table, .. } => Intent::Tip { table, front },
                        Intent::Sink { sink, .. } => Intent::Sink { sink, front },
                        Intent::Mop { tile, .. } => Intent::Mop { tile, front },
                        other => other,
                    };
                    self.intent(content, i)
                }
                None => false,
            },
            Intent::Greet => {
                let newest = self
                    .world
                    .parties()
                    .iter()
                    .filter(|p| greetable(content, &self.world, p))
                    .max_by_key(|p| p.arrived)
                    .and_then(|p| p.leader());
                match newest {
                    Some(g) => self.hand(content, Command::PlayerGreet { guest: g }),
                    None => false,
                }
            }
        }
    }

    /// A click on a guest: wave if they just came in, then whatever their order needs.
    fn guest_intent(&mut self, content: &Content, guest: u32, front: bool) -> bool {
        let Some(p) = self.world.party_of_guest(guest).cloned() else {
            return false;
        };
        let leader = p.leader().unwrap_or(guest);
        let mut acted = false;
        if greetable(content, &self.world, &p) {
            acted |= self.hand(content, Command::PlayerGreet { guest: leader });
        }
        match p.state {
            PartyState::Queue | PartyState::WaitOrder => {
                if p.order_claim.is_none() {
                    acted |= if front {
                        self.queue(content, PlayerAct::TakeOrder { party: p.id }, true)
                    } else {
                        self.hand(content, Command::PlayerTakeOrder { guest: leader })
                    };
                }
            }
            PartyState::WaitFood | PartyState::Eating => {
                if p.wish_open {
                    self.open_card(content, leader);
                    acted = true;
                } else if let Some(i) = open_item(&self.world, &p, Some(guest)) {
                    let g = i.guest;
                    self.open_card(content, g);
                    acted = true;
                } else if let Some(i) = ready_item(&self.world, &p) {
                    let item = i.id;
                    acted |= self.queue(
                        content,
                        PlayerAct::Serve {
                            item,
                            extra: Vec::new(),
                        },
                        front,
                    );
                }
            }
            _ => {}
        }
        acted
    }

    /// The nearest thing for **E** (and the autoplayer): coins, dirty tables, the sink, dirt,
    /// the window light — skipping what is already in her hands.
    pub fn next_chore(&self, content: &Content) -> Option<Intent> {
        let w = &self.world;
        let here = w
            .karolina()
            .map(|k| vec2(k.pos.0, k.pos.1))
            .unwrap_or_default();
        let acts = karolina_acts(w);
        let dist = |p: Vec2| p.distance(here);
        let mut best: Option<(f32, Intent)> = None;
        fn consider(best: &mut Option<(f32, Intent)>, d: f32, i: Intent) {
            if best.is_none_or(|(b, _)| d < b) {
                *best = Some((d, i));
            }
        }
        // tables with tip coins or cups
        for p in w.furniture() {
            if !has_tag(content, w, p.uid, "table") {
                continue;
            }
            let tip = w.tips_on_tables().iter().any(|t| t.table == p.uid);
            if (p.dishes == 0 && !tip)
                || p.claim
                    .is_some_and(|c| Some(c) != w.karolina().map(|k| k.id))
            {
                continue;
            }
            let queued = acts.iter().any(|a| {
                matches!(a, PlayerAct::ClearTable { table } | PlayerAct::CollectTip { table } if *table == p.uid)
            });
            if queued {
                continue;
            }
            let at = vec2(p.x as f32 + 0.5, p.y as f32 + 0.5) * TILE;
            consider(
                &mut best,
                dist(at),
                Intent::Table {
                    table: p.uid,
                    front: false,
                },
            );
        }
        if best.is_some() {
            return best.map(|b| b.1);
        }
        // the dishes
        if w.sink_dishes() > 0
            && let Some(s) = w
                .furniture()
                .iter()
                .find(|p| has_tag(content, w, p.uid, "dishes"))
            && !acts.iter().any(|a| matches!(a, PlayerAct::Wash { .. }))
        {
            return Some(Intent::Sink {
                sink: s.uid,
                front: false,
            });
        }
        // dirt blots
        for t in w.dirt_spots() {
            let queued = acts
                .iter()
                .any(|a| matches!(a, PlayerAct::Mop { tile } if *tile == t));
            if !queued {
                let at = vec2(t.0 as f32 + 0.5, t.1 as f32 + 0.5) * TILE;
                consider(
                    &mut best,
                    dist(at),
                    Intent::Mop {
                        tile: t,
                        front: false,
                    },
                );
            }
        }
        if best.is_some() {
            return best.map(|b| b.1);
        }
        let window_open = w
            .app()
            .checklist
            .iter()
            .any(|c| c.id == "light_window" && !c.done && c.claim.is_none());
        (window_open && !w.window_light_on(content)).then_some(Intent::Window)
    }

    // ───────────────────────── order card ─────────────────────────

    /// Opens the order card for `guest` (or queues it behind an open overlay).
    pub fn open_card(&mut self, content: &Content, guest: u32) {
        if self.hands.minigame.is_some() || self.hands.book.is_some() {
            if !self.hands.pending_cards.contains(&guest) {
                self.hands.pending_cards.push_back(guest);
            }
            return;
        }
        let Some(p) = self.world.party_of_guest(guest) else {
            return;
        };
        let wish = p.wish_open;
        let first = self
            .world
            .best_matches(content, guest)
            .first()
            .map(|r| r.0.clone())
            .unwrap_or_default();
        let mode = ui_mode(self.world.default_brew_mode(content, wish, &first));
        let party = p.id;
        if !wish {
            self.hands.tut.bump("plain_card");
        }
        self.hands.order = Some(OrderOpen {
            guest,
            party,
            mode,
            opened: self.time,
        });
        self.bubble = None;
    }

    /// The card's guest still needs something from Karolina.
    pub fn card_valid(&self, guest: u32) -> bool {
        let Some(p) = self.world.party_of_guest(guest) else {
            return false;
        };
        if !matches!(p.state, PartyState::WaitFood | PartyState::Eating) {
            return false;
        }
        p.wish_open && p.leader() == Some(guest) || open_item(&self.world, p, Some(guest)).is_some()
    }

    /// Opens the next waiting card once nothing else is open.
    pub fn next_card(&mut self, content: &Content) {
        if self.hands.overlay() {
            return;
        }
        while let Some(g) = self.hands.pending_cards.pop_front() {
            if self.card_valid(g) {
                self.open_card(content, g);
                return;
            }
        }
    }

    /// Pick recipe `recipe` for the card's guest in `mode`.
    pub fn brew_chosen(&mut self, content: &Content, guest: u32, recipe: &str, mode: UiBrew) {
        if !self.can_brew(content, guest, recipe) {
            return;
        }
        self.hands.order = None;
        self.hands.book = None;
        self.hands.tut.bump("brew_chosen");
        match mode {
            UiBrew::Quick => {
                self.hands.tut.bump("quick_chosen");
                let ok = self.hand(
                    content,
                    Command::PlayerBrew {
                        guest,
                        recipe: recipe.to_string(),
                        quality: 0.0,
                        mode: BrewMode::Quick,
                    },
                );
                if ok {
                    self.follow_up(guest);
                } else {
                    // her hands are full: the card waits on the rail
                    self.toast_key(ToastKind::Info, Icon::Info, "ui.hands.brew_later", &[]);
                }
                self.next_card(content);
            }
            UiBrew::ByHand => self.start_minigame(content, guest, recipe, false),
        }
    }

    /// The recipe answers this card: any known, available recipe for an open wish; for a
    /// plain item one of the recipes riding on it (the card's own suggestions).
    pub fn can_brew(&self, content: &Content, guest: u32, recipe: &str) -> bool {
        let Some(p) = self.world.party_of_guest(guest) else {
            return false;
        };
        if p.wish_open && p.leader() == Some(guest) {
            return self.world.recipe_available(content, recipe);
        }
        self.world
            .best_matches(content, guest)
            .iter()
            .any(|m| m.0 == recipe)
    }

    /// After a brew: the same party's other open items get their card right away (a party
    /// of three is one round of cards, not three hunts on the rail).
    fn follow_up(&mut self, guest: u32) {
        let Some(p) = self.world.party_of_guest(guest).cloned() else {
            return;
        };
        if let Some(i) = open_item(&self.world, &p, None) {
            let g = i.guest;
            if !self.hands.pending_cards.contains(&g) {
                self.hands.pending_cards.push_front(g);
            }
        }
    }

    /// "The usual?": the regular's favourite, by hand.
    pub fn usual_chosen(&mut self, content: &Content, guest: u32) {
        let Some(recipe) = self.world.usual_available(content, guest) else {
            return;
        };
        self.hands.order = None;
        self.hands.tut.bump("brew_chosen");
        self.hands.tut.bump("usual_chosen");
        self.start_minigame(content, guest, &recipe, true);
    }

    pub fn hand_off(&mut self, content: &Content, guest: u32) {
        self.hands.order = None;
        self.hand(content, Command::PlayerHandOff { guest });
        self.next_card(content);
    }

    /// Starts the mini-game of `recipe` (the text comes in when the frame has a locale).
    pub fn start_minigame(&mut self, content: &Content, guest: u32, recipe: &str, usual: bool) {
        let Some(def) = content.handson.recipe(recipe) else {
            return;
        };
        let seed = self.world.seed() ^ (self.world.tick_count() as u64) << 8 ^ guest as u64;
        let (kind, variant) = playable(def, seed);
        let regular = self
            .world
            .party_of_guest(guest)
            .and_then(|p| p.regular.clone());
        let practised = self
            .world
            .knowledge()
            .recipes
            .get(recipe)
            .is_some_and(|s| s.practised);
        let params = MiniGameParams::new(seed)
            .variant(Variation::from_variant_id(kind, &variant))
            .relaxed(self.relaxed)
            .regular(regular.as_deref())
            .practised(practised);
        let game = MiniGame::start(kind, params).with_text(MiniGameText::default());
        // she walks to the station and works while the mini-game plays (the cup waits
        // there for its quality)
        let target = if usual {
            self.world
                .party_of_guest(guest)
                .and_then(|p| p.leader())
                .unwrap_or(guest)
        } else {
            guest
        };
        let started = self
            .world
            .apply(
                content,
                Command::PlayerBrewStart {
                    guest: target,
                    recipe: recipe.to_string(),
                    mode: BrewMode::ByHand,
                },
            )
            .is_ok();
        self.hands.minigame = Some(MgOpen {
            game,
            guest: target,
            recipe: recipe.to_string(),
            usual,
            named: false,
            started,
        });
        self.hands.wash = None;
    }

    /// The mini-game reported its quality: brew it.
    pub fn minigame_done(&mut self, content: &Content, quality: f32) {
        let Some(mg) = self.hands.minigame.take() else {
            return;
        };
        if mg.game.kind() == MiniGameKind::LatteArt
            && let Some(shape) = mg.game.variation().shape
        {
            self.latte_for.insert(mg.guest, shape.id().to_string());
        }
        self.hands.tut.bump("minigame_done");
        if mg.started {
            // the brew is on its way; a late quality (she already served) is simply dropped
            let _ = self.world.apply(
                content,
                Command::PlayerBrewQuality {
                    guest: mg.guest,
                    quality,
                },
            );
            self.follow_up(mg.guest);
            self.next_card(content);
            return;
        }
        let ok = if mg.usual {
            self.hand(
                content,
                Command::PlayerUsual {
                    guest: mg.guest,
                    quality,
                },
            )
        } else {
            self.hand(
                content,
                Command::PlayerBrew {
                    guest: mg.guest,
                    recipe: mg.recipe.clone(),
                    quality,
                    mode: BrewMode::ByHand,
                },
            )
        };
        if ok {
            self.follow_up(mg.guest);
        } else {
            // the guest left meanwhile, or her hands are full: nothing is lost on Cozy
            self.toast_key(ToastKind::Info, Icon::Info, "ui.hands.brew_later", &[]);
        }
        self.next_card(content);
    }

    // ───────────────────────── events ─────────────────────────

    /// Hands-on events (called from `handle_event`).
    pub fn hands_event(&mut self, content: &Content, e: &SimEvent) {
        self.moment_event(content, e);
        match e {
            SimEvent::NightStarted { .. } => {
                self.hands.tried_tonight = false;
                self.hands.overlay_real = 0.0;
                self.hands.night_real = 0.0;
                self.hands.atmo_level = self.world.coziness().level;
                self.hands.tut = TutTally::default();
                self.hands.coins.clear();
                self.hands.incoming.clear();
                self.hands.record_play = None;
            }
            SimEvent::NightEnded { .. } => {
                self.hands.moment = None;
                self.hands.order = None;
                self.hands.minigame = None;
                self.hands.book = None;
                self.hands.pending_cards.clear();
                self.hands.records_open = false;
                self.hands.wash = None;
            }
            SimEvent::GuestArrived { regular, .. } => {
                self.hands.tut.bump("guest_arrived");
                if regular.as_deref() == Some("mirek") {
                    self.hands.tut.bump("mirek_arrived");
                }
            }
            SimEvent::PlayerGreeted { .. } => self.hands.tut.bump("greeted"),
            SimEvent::PlayerOrderTaken { guest, .. } => {
                self.hands.tut.bump("order_taken");
                if self.hands.order.is_some() || !self.hands.pending_cards.is_empty() {
                    if !self.hands.pending_cards.contains(guest) {
                        self.hands.pending_cards.push_back(*guest);
                    }
                } else {
                    self.open_card(content, *guest);
                }
            }
            SimEvent::PlayerServed {
                guest,
                recipe,
                match_score,
                quality,
            } => {
                self.hands.tut.bump("served");
                let art = self
                    .latte_for
                    .remove(guest)
                    .and_then(|id| LatteArt::from_id(&id));
                self.scene
                    .served
                    .insert(*guest, (cup_of(content, recipe), art));
                if let Some(head) = self.scene.head_of(sim::AgentRef::Guest(*guest), self.time) {
                    let tier = MatchTier::of(*match_score, &content.handson.tuning.match_tiers);
                    let wish = self
                        .world
                        .party_of_guest(*guest)
                        .is_some_and(|p| p.order_kind.is_wish());
                    let tier_key = if wish {
                        format!("ui.served.{}", tier_id(tier))
                    } else {
                        "ui.served.plain".to_string()
                    };
                    let q = content.handson.minigames.quality_tiers.label(*quality);
                    self.hands.floats.push(Float {
                        at: head - vec2(0.0, 18.0),
                        text: tier_key,
                        born: self.time,
                        args: vec![("quality".into(), format!("@minigame.quality.{q}"))],
                    });
                }
            }
            SimEvent::GuestReaction { guest, key, .. } => {
                self.show_line(sim::AgentRef::Guest(*guest), key.clone(), 6.0);
            }
            SimEvent::GuestForgiven { party, key } => {
                if let Some(l) = self
                    .world
                    .parties()
                    .iter()
                    .find(|p| p.id == *party)
                    .and_then(|p| p.leader())
                {
                    self.show_line(sim::AgentRef::Guest(l), key.clone(), 5.0);
                }
            }
            SimEvent::GuestPatienceRefilled { party } => {
                self.hands.water.insert(*party, self.time + 8.0);
            }
            SimEvent::TipLeft { .. } => self.hands.tut.bump("tip_left"),
            SimEvent::TipCollected { amount, .. } => {
                self.hands.tut.bump("tip_collected");
                self.hands.jar_pulse = self.time;
                self.hands.floats.push(Float {
                    at: jar_pos(&self.world, content) - vec2(0.0, 18.0),
                    text: "ui.hands.tip_float".into(),
                    born: self.time + crate::render::TIP_COLLECT_SECS,
                    args: vec![("amount".into(), format!("money:{amount}"))],
                });
            }
            SimEvent::RegularIncoming {
                regular,
                party,
                minutes,
            } => {
                let until = self.world.minute() + minutes + 1;
                self.hands.incoming.push((regular.clone(), *party, until));
            }
            SimEvent::RegularArrived { party, .. } => {
                self.hands.incoming.retain(|i| i.1 != *party);
            }
            SimEvent::LullSuggestion { hint } => match hint {
                sim::LullHint::Speed => {
                    self.hands.lull_until = self.time + 20.0;
                    self.hands.tut.bump("lull_chip");
                }
                sim::LullHint::Talk { guest } => {
                    if let Some(id) = self.world.talk_available(content, *guest)
                        && self.bubble.as_ref().is_none_or(|b| !b.pinned)
                    {
                        let text = self
                            .world
                            .ambient_line_for(content, *guest)
                            .unwrap_or_default();
                        self.bubble = Some(super::Bubble {
                            agent: sim::AgentRef::Guest(*guest),
                            text: format!("@key:{text}"),
                            talk: Some(id),
                            pinned: false,
                            until: self.time + 12.0,
                        });
                    }
                }
                _ => {}
            },
            SimEvent::RecordChanged { record, repeat } => {
                self.hands.tut.bump("record_changed");
                self.hands.record_play = match record {
                    Some(id) if !repeat => Some((id.clone(), 0.0)),
                    _ => None,
                };
            }
            SimEvent::RecordReaction {
                guest,
                record,
                liked,
            } => {
                let regular = self
                    .world
                    .party_of_guest(*guest)
                    .and_then(|p| p.regular.clone());
                if let (Some(r), Some(def)) = (regular, content.handson.record(record)) {
                    let key = if *liked {
                        def.liked_key(&r)
                    } else {
                        def.disliked_key(&r)
                    };
                    self.show_line(sim::AgentRef::Guest(*guest), key, 6.0);
                }
            }
            SimEvent::RecordUnlocked { id } => {
                if let Some(def) = content.handson.record(id) {
                    let title = def.title_key();
                    self.toast_key(
                        ToastKind::Success,
                        Icon::Vinyl,
                        "ui.jukebox.unlocked",
                        &[("title", format!("@{title}"))],
                    );
                }
            }
            SimEvent::RecipeLearned { recipe, source } if source != "starter" => {
                if let Some(def) = content.handson.recipe(recipe) {
                    self.toast_key(
                        ToastKind::Success,
                        Icon::Book,
                        "book.learned",
                        &[("recipe", format!("@{}", def.name_key()))],
                    );
                }
            }
            SimEvent::ExperimentDone { result } => {
                use crate::ui::handson::ExperimentKind as K;
                let (kind, key, args, recipe): (K, String, Vec<(String, String)>, Option<String>) =
                    match result {
                        sim::ExperimentResult::Known { recipe } => (
                            K::Known,
                            "experiment.known".into(),
                            vec![("recipe".into(), format!("@recipe.{recipe}.name"))],
                            Some(recipe.clone()),
                        ),
                        sim::ExperimentResult::Discovered { recipe } => {
                            self.hands.new_recipes.insert(recipe.clone());
                            self.hands.tut.bump("recipe_discovered");
                            (
                                K::Discovered,
                                "experiment.discovered".into(),
                                vec![("recipe".into(), format!("@recipe.{recipe}.name"))],
                                Some(recipe.clone()),
                            )
                        }
                        sim::ExperimentResult::Gated { recipe, key } => {
                            (K::Gated, key.clone(), Vec::new(), Some(recipe.clone()))
                        }
                        sim::ExperimentResult::Close { key, family } => (
                            K::Close,
                            key.clone(),
                            family
                                .iter()
                                .map(|f| ("family".to_string(), format!("@{f}")))
                                .collect(),
                            None,
                        ),
                        sim::ExperimentResult::Nothing { key, cat } => {
                            let mut a = Vec::new();
                            if *cat {
                                a.push(("cat".to_string(), "@experiment.cat_sniffs".to_string()));
                            }
                            (K::Nothing, key.clone(), a, None)
                        }
                    };
                self.hands.tut.bump("experiment_done");
                let note = BookNote {
                    kind,
                    key,
                    args,
                    recipe,
                };
                match self.hands.book.as_mut() {
                    Some(b) => b.note = Some(note),
                    None => {
                        // a night experiment: she tried it at the counter while the book was shut
                        self.notices.push(Notice::Key {
                            kind: ToastKind::Info,
                            icon: Icon::Flask,
                            key: note.key.clone(),
                            args: note.args.clone(),
                        });
                        self.hands.last_experiment = Some(note);
                    }
                }
            }
            SimEvent::MarginNote { recipe, .. } => {
                self.hands.last_margin = Some(recipe.clone());
                if let Some(b) = self.hands.book.as_mut() {
                    b.margin = Some(recipe.clone());
                }
            }
            SimEvent::PractisedRecipe { recipe } => {
                if let Some(def) = content.handson.recipe(recipe) {
                    self.toast_key(
                        ToastKind::Success,
                        Icon::Star,
                        "ui.hands.practised",
                        &[("recipe", format!("@{}", def.name_key()))],
                    );
                }
            }
            _ => {}
        }
    }

    /// A bubble line (i18n key) over someone, unless the player pinned a bubble.
    fn show_line(&mut self, agent: sim::AgentRef, key: String, seconds: f32) {
        if self.bubble.as_ref().is_some_and(|b| b.pinned) {
            return;
        }
        self.bubble = Some(super::Bubble {
            agent,
            text: format!("@key:{key}"),
            talk: None,
            pinned: false,
            until: self.time + seconds,
        });
    }

    /// Per frame (logic): tip coins follow the sim, play-once records end, finished acts
    /// make their sound. Returns the sounds to play.
    pub fn hands_frame(&mut self, content: &Content, dt: f32) -> Vec<(crate::audio::Sfx, f32)> {
        let mut sounds = Vec::new();
        let now = self.time;
        // coins
        let jar = jar_pos(&self.world, content);
        for c in self.world.tips_on_tables() {
            if !self.hands.coins.iter().any(|v| v.id == c.id)
                && let Some(top) = table_top(&self.world, content, c.table)
            {
                let j = (c.id % 5) as f32 - 2.0;
                let pos = top + vec2(j * 3.0 + 8.0, j.abs() * 1.5);
                self.hands.coins.push(CoinVis {
                    id: c.id,
                    table: c.table,
                    coin: TipCoin::new(pos, now, jar, c.id),
                });
                sounds.push((crate::audio::Sfx::UiClick, 0.25));
            }
        }
        let live: Vec<u32> = self.world.tips_on_tables().iter().map(|c| c.id).collect();
        for v in self.hands.coins.iter_mut() {
            if v.coin.collected.is_none() && !live.contains(&v.id) {
                v.coin.collected = Some(now);
                sounds.push((crate::audio::Sfx::CashRegister, 0.25));
            }
        }
        self.hands.coins.retain(|v| v.coin.alive(now));
        self.hands.floats.retain(|f| now - f.born < 1.6);
        self.hands.water.retain(|_, until| *until > now);
        self.scene.shine.retain(|s| now - s.1 < 6.0);
        let m = self.world.minute();
        self.hands.incoming.retain(|i| i.2 > m);
        // served cups stay while their guest is in the café
        let guests: Vec<u32> = self.world.guests().iter().map(|g| g.id).collect();
        self.scene.served.retain(|g, _| guests.contains(g));
        // a play-once record: the integration measures its length (no end-of-sound callback)
        if let Some((id, played)) = &mut self.hands.record_play {
            *played += dt;
            let len = content.handson.record(id).map_or(95.0, |r| r.seconds);
            if *played >= len {
                let id = id.clone();
                self.hands.record_play = None;
                let _ = self.world.apply(content, Command::RecordFinished { id });
            }
        }
        // finished acts: clink, swish
        let cur = self
            .world
            .player()
            .current
            .as_ref()
            .map(|c| (c.act.clone(), c.stage));
        if cur != self.hands.prev_act {
            if let Some((act, stage)) = &self.hands.prev_act {
                let still = cur.as_ref().is_some_and(|c| &c.0 == act);
                match act {
                    PlayerAct::ClearTable { .. }
                        if cur.as_ref().is_some_and(|c| &c.0 == act && c.1 == 2) =>
                    {
                        sounds.push((crate::audio::Sfx::CupSaucer, 0.5));
                        self.hands.tut.bump("table_cleared");
                    }
                    PlayerAct::ClearTable { .. } if !still && *stage < 2 => {
                        // picked up and put down in one step (a coin only)
                        self.hands.tut.bump("table_cleared");
                    }
                    PlayerAct::Mop { .. } if !still => {
                        sounds.push((crate::audio::Sfx::Footsteps, 0.4));
                        self.hands.tut.bump("mopped");
                    }
                    PlayerAct::Wash { .. } if !still => {
                        sounds.push((crate::audio::Sfx::CupSaucer, 0.45));
                        self.hands.tut.bump("washed");
                    }
                    _ => {}
                }
            }
            self.hands.prev_act = cur;
        }
        // a card whose guest no longer needs it closes (they left, a helper took it)
        if let Some(o) = &self.hands.order
            && now - o.opened > 0.3
            && !self.card_valid(o.guest)
        {
            self.hands.order = None;
            self.next_card(content);
        }
        sounds
    }

    /// Hold-to-wash: `holding` = the button (mouse on the sink, or **E**) is down now.
    pub fn wash_input(&mut self, content: &Content, holding: bool, dt: f32) {
        let at_sink = self
            .world
            .player()
            .current
            .as_ref()
            .and_then(|c| match c.act {
                PlayerAct::Wash { sink } if c.stage == 1 => Some(sink),
                _ => None,
            });
        match (at_sink, holding) {
            (Some(sink), true) => {
                let h = self.hands.wash.get_or_insert(WashHold {
                    sink,
                    ..Default::default()
                });
                h.sink = sink;
                h.held += dt;
                h.unsent += dt;
                if h.unsent >= WASH_SEND_EVERY {
                    let s = std::mem::take(&mut h.unsent);
                    self.intent(content, Intent::WashHold { sink, seconds: s });
                }
            }
            _ => {
                if let Some(h) = self.hands.wash.take()
                    && h.unsent > 0.0
                    && at_sink == Some(h.sink)
                {
                    self.intent(
                        content,
                        Intent::WashHold {
                            sink: h.sink,
                            seconds: h.unsent,
                        },
                    );
                }
            }
        }
    }

    // ───────────────────────── autoplay ─────────────────────────

    /// One decision of the autoplayer (mini-games are stepped by the frame). Returns the
    /// intent it used, for logs.
    pub fn autoplay_step(&mut self, content: &Content, dt: f32) -> Option<&'static str> {
        let mut ap = self.hands.autoplay?;
        if self.world.phase() != Phase::Night || self.hands.minigame.is_some() {
            return None;
        }
        ap.cooldown -= dt;
        if ap.cooldown > 0.0 {
            self.hands.autoplay = Some(ap);
            return None;
        }
        ap.cooldown = AUTOPLAY_REACT;
        self.hands.autoplay = Some(ap);
        // an open card: the usual if offered, else the first suggestion in its default mode
        if let Some(o) = self.hands.order.clone() {
            if self.world.usual_available(content, o.guest).is_some() {
                self.usual_chosen(content, o.guest);
                return Some("usual");
            }
            match self.world.best_matches(content, o.guest).first() {
                Some((r, _)) => {
                    let r = r.clone();
                    self.brew_chosen(content, o.guest, &r, o.mode);
                    return Some("pick");
                }
                None => {
                    self.hands.order = None;
                    return Some("close");
                }
            }
        }
        if self.hands.book.is_some() {
            self.hands.book = None;
        }
        if let Some(m) = self.hands.moment.clone() {
            let opts = self.world.moment_options(content, &m.id);
            match opts.first() {
                Some(o) => {
                    let o = o.clone();
                    self.moment_option(content, &m.id, &o);
                }
                None => self.hands.moment = None,
            }
            return Some("moment");
        }
        // standing at the sink: the frame holds the button
        if self
            .world
            .player()
            .current
            .as_ref()
            .is_some_and(|c| matches!(c.act, PlayerAct::Wash { .. }) && c.stage == 1)
        {
            return Some("wash");
        }
        // closing time: the handover note and the light at six go first in her hands (from
        // 05:30, so a busy last half hour can't push them past six)
        if self.world.minute() >= 450 {
            for id in ["handover_note", "light_off_at_six"] {
                let open = self
                    .world
                    .app()
                    .checklist
                    .iter()
                    .any(|c| c.id == id && !c.done && c.claim.is_none());
                if !open {
                    continue;
                }
                // a full queue at closing time: the last thing waits, the light goes first
                if self.world.player().queue.len() >= 5 {
                    let last = self.world.player().queue.len() - 1;
                    self.hand(content, Command::PlayerUnqueue { index: last });
                }
                if self.queue(content, PlayerAct::Checklist { id: id.to_string() }, true) {
                    return Some("closing");
                }
            }
        }
        // light the window first, like grandma said
        if self.world.minute() < 420 && self.intent(content, Intent::Window) {
            return Some("window");
        }
        // a record for the opening ritual
        if self.world.minute() < 30
            && self
                .world
                .app()
                .checklist
                .iter()
                .any(|c| c.id == "record_on" && !c.done && c.claim.is_none())
            && self.hand(
                content,
                Command::TickChecklist {
                    id: "record_on".into(),
                },
            )
        {
            return Some("record");
        }
        if self.intent(content, Intent::Greet) {
            return Some("greet");
        }
        // a moment on the map: answer it (they are hers alone)
        if self.hands.moment.is_none()
            && self.world.player().queue.len() < 3
            && self.autoplay_moment(content)
        {
            return Some("moment");
        }
        // the rituals: warm up the machine, count the float, cash up…
        if self.world.player().queue.len() < 2
            && let Some((i, _)) = super::hands_view::open_rituals(self, content)
                .first()
                .copied()
            && let Some(id) = content.app.checklists.get(i).map(|c| c.id.clone())
            && self.hand(content, Command::TickChecklist { id })
        {
            return Some("ritual");
        }
        // closing: two notes for tomorrow
        if self.world.minute() >= 7 * 60 + 30 && self.world.handover_picks().is_empty() {
            let picks: Vec<String> = self
                .world
                .handover_suggestions(content)
                .iter()
                .take(2)
                .map(|s| s.id.clone())
                .collect();
            if !picks.is_empty() && self.hand(content, Command::PickHandover { picks }) {
                return Some("handover");
            }
        }
        // a lull: one experiment a night (grandma's margin notes first)
        if !self.hands.tried_tonight
            && self.world.player().queue.is_empty()
            && self.world.player().current.is_none()
            && self.world.minute() > 60
        {
            self.hands.tried_tonight = true;
            let target = self.world.margin_note_target(content);
            let mut ings: Vec<String> = target
                .map(|r| self.world.revealed_ingredients(content, &r))
                .unwrap_or_default();
            let pantry: Vec<String> = self.world.pantry().iter().cloned().collect();
            let mut k = self.world.night() as usize;
            while ings.len() < 2 && !pantry.is_empty() {
                let c = pantry[k % pantry.len()].clone();
                if !ings.contains(&c) {
                    ings.push(c);
                }
                k += 7;
                if k > 400 {
                    break;
                }
            }
            ings.retain(|i| pantry.contains(i));
            ings.truncate(3);
            if ings.len() >= 2 && self.hand(content, Command::TryExperiment { ingredients: ings }) {
                return Some("experiment");
            }
        }
        if self.world.player().queue.len() >= 2 {
            return None;
        }
        // the rail: regulars and wishes first, then the rest (only what nobody else holds)
        let mut tickets = self.world.ticket_rail(content);
        tickets.sort_by_key(|t| {
            (
                t.regular.is_none(),
                !matches!(t.stage, TicketStage::Wish { .. }),
                (1000.0 - t.wait_frac * 1000.0) as i32,
            )
        });
        for t in tickets {
            let mine = match t.owner {
                sim::TicketOwner::Nobody => true,
                sim::TicketOwner::Karolina => true,
                sim::TicketOwner::Helper(_) => false,
            };
            if !mine {
                continue;
            }
            let busy = match &t.stage {
                TicketStage::Order => self
                    .world
                    .parties()
                    .iter()
                    .any(|p| p.id == t.party && p.order_claim.is_some()),
                TicketStage::Make { item, .. } | TicketStage::Serve { item, .. } => self
                    .world
                    .order_items()
                    .iter()
                    .any(|i| i.id == *item && i.by.is_some()),
                TicketStage::Wish { .. } => false,
            };
            if busy {
                continue;
            }
            if self.intent(
                content,
                Intent::Ticket {
                    party: t.party,
                    guest: t.guest,
                },
            ) {
                return Some("ticket");
            }
        }
        if self.intent(content, Intent::Chore { front: false }) {
            return Some("chore");
        }
        None
    }
}

impl GameScreen {
    /// The autoplayer's morning and day: keep the polaroid, let the sim's player-bot plan
    /// the day (hire on nights 3/5/9, the rozpis, payroll, themes), then open the café.
    pub fn autoplay_day(&mut self, content: &std::rc::Rc<Content>, dt: f32, wait: f32) {
        if self.hands.autoplay.is_none()
            || self.world.phase() != Phase::Day
            || self.cutscene_playing()
            || self.world.playing_cutscene().is_some()
            || self.world.ending().is_some()
        {
            self.hands.day_wait = 0.0;
            return;
        }
        self.hands.day_wait += dt;
        if self.hands.day_wait < wait {
            return;
        }
        self.hands.day_wait = 0.0;
        if self.summary.is_some() {
            super::journal_ui::keep_morning(self, content);
            self.summary = None;
            return;
        }
        let seed = self.world.seed();
        let bot = self.hands.day_bot.get_or_insert_with(|| {
            sim::bot::PlayerBot::new(sim::bot::BotConfig {
                seed,
                ..Default::default()
            })
        });
        bot.day(&mut self.world, content);
        self.drain_and_handle(content, None);
        self.start_night(content);
    }

    /// Plays the current night without a window through the autoplayer (scenes, tests):
    /// the sim, the intents, the mini-games with their scripted input and hold-to-wash, at
    /// `dt` real seconds a step, until `stop` says so, the morning comes or `max_s` passes.
    pub fn play_headless(
        &mut self,
        content: &std::rc::Rc<Content>,
        loc: &crate::settings::Locale,
        max_s: f32,
        stop: &dyn Fn(&GameScreen) -> bool,
    ) {
        let before = self.hands.autoplay;
        let ap = Autoplay {
            cooldown: 0.0,
            keyboard: false,
            skip_minigames: before.is_some_and(|a| a.skip_minigames),
        };
        self.hands.autoplay = Some(ap);
        let dt = 1.0 / 30.0;
        let mut t = 0.0;
        while t < max_s && self.world.phase() == Phase::Night && !stop(self) {
            if self.cutscene_playing() {
                self.finish_cutscene_now(content, loc);
            }
            self.time += dt;
            self.hands.night_real += dt;
            if self.overlay_open() {
                self.hands.overlay_real += dt;
            }
            self.advance(content, dt, Some(loc));
            let _ = self.hands_frame(content, dt);
            self.autoplay_step(content, dt);
            if let Some(mg) = self.hands.minigame.as_mut() {
                let input = if ap.skip_minigames {
                    crate::minigame::MgInput {
                        skip: true,
                        ..Default::default()
                    }
                } else {
                    mg.game.autoplay_input()
                };
                if let crate::minigame::MiniGameStatus::Done { quality } = mg.game.step(&input, dt)
                {
                    self.minigame_done(content, quality);
                }
            }
            self.wash_input(content, true, dt);
            t += dt;
        }
        self.hands.autoplay = before;
    }
}

/// `perfect` / `good` / `okay` / `not_quite`.
pub fn tier_id(t: MatchTier) -> &'static str {
    match t {
        MatchTier::Perfect => "perfect",
        MatchTier::Good => "good",
        MatchTier::Okay => "okay",
        MatchTier::NotQuite => "not_quite",
    }
}
