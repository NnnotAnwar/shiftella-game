//! # Hands-on widgets (design `handson.md` §15 T6)
//!
//! Glass widgets for Karolina's work behind the counter. They draw from plain **view
//! models** (data the integration fills from the sim / content each frame) and take every
//! player-visible string from the caller (a labels struct), so nothing here is hardcoded
//! text. They return small action enums instead of mutating game state.
//!
//! | widget | fn | view model → action |
//! |---|---|---|
//! | Order card | [`order_card`] | [`OrderView`] + [`OrderLabels`] → [`OrderAction`] |
//! | Ticket rail | [`ticket_rail`] | `&[Ticket]` → [`RailAction`] |
//! | Moment card | [`moment_card`] | [`MomentView`] → [`MomentAction`] |
//! | Duties board | [`duties_board`] | [`DutiesView`] → [`DutiesAction`] |
//! | Record picker | [`record_picker`] | [`RecordsView`] → [`RecordAction`] |
//! | Recipe book | [`recipe_book`] | [`BookView`] → [`BookAction`] |
//! | Atmosphere meter | [`atmosphere_meter`] | [`AtmosphereView`] |
//! | Rozpis puzzle | [`rozpis_board`], [`pref_chip`], [`wellbeing_ring`], [`fairness_badge`] | [`RozpisView`] → [`RozpisAction`] |
//! | Payroll | [`payroll_screen`] | [`PayrollView`] → [`PayrollAction`] |
//! | Journal | [`journal`], [`polaroid`] | [`JournalView`] / [`PolaroidView`] → [`JournalAction`] |
//! | Coach mark | [`coach_mark`] | [`CoachMark`] → [`CoachAction`] |
//!
//! Shared: [`TasteTag`] (12 tastes with icons and colours), [`Duty`], [`Dish`] /
//! [`draw_dish`] (procedural cups, bowls, plates), [`with_transform`] (rotated / slanted
//! drawing), [`draw_handwritten`] (faux-italic captions).
//!
//! See `examples/ui_handson_demo` for every widget on mock data.

pub mod atmosphere;
pub mod book;
pub mod coach;
pub mod common;
pub mod duties;
pub mod journal;
pub mod moment;
pub mod order;
pub mod payroll;
pub mod records;
pub mod rozpis;

pub use atmosphere::{ATMO_SIZE, AtmosphereView, atmosphere_meter, candle};
pub use book::{
    BOOK_SIZE, BookAction, BookLabels, BookRecipe, BookSection, BookState, BookView,
    ExperimentKind, ExperimentResult, Ingredient, Shelf, draw_jar, recipe_book,
};
pub use coach::{CoachAction, CoachMark, coach_mark};
pub use common::{
    Affine, Dish, DishGlyph, Duty, TasteMark, TasteNames, TasteTag, dashed_rrect, draw_dish,
    draw_handwritten, draw_handwritten_block, mini_chip, ring, taste_dot, taste_row,
    with_transform,
};
pub use duties::{
    DutiesAction, DutiesLabels, DutiesState, DutiesView, HelperRow, duties_board, duties_board_size,
};
pub use journal::{
    JOURNAL_SIZE, JournalAction, JournalEntry, JournalLabels, JournalState, JournalView,
    PolaroidView, journal, polaroid, polaroid_size,
};
pub use moment::{
    MomentAction, MomentOption, MomentView, moment_card, moment_card_rect, moment_card_size,
};
pub use order::{
    BrewMode, OrderAction, OrderKind, OrderLabels, OrderOut, OrderView, RailAction, RailState,
    RecipeChoice, Ticket, TicketItem, TicketOwner, order_card, order_card_size, ticket_rail,
    ticket_rail_rect, ticket_rail_size,
};
pub use payroll::{
    Insight, PAYROLL_SIZE, PayRow, PayrollAction, PayrollLabels, PayrollView, SplitOption,
    TipSplit, payroll_screen,
};
pub use records::{
    RecordAction, RecordLabels, RecordState, RecordView, RecordsView, SleeveMotif, draw_sleeve,
    record_picker, record_picker_size,
};
pub use rozpis::{
    CoverageNight, FairnessView, PrefChip, PrefStatus, RozpisAction, RozpisLabels, RozpisRow,
    RozpisState, RozpisView, ShiftCell, WeekWeather, fairness_badge, pref_chip, pref_chip_width,
    rozpis_board, rozpis_board_size, wellbeing_color, wellbeing_ring,
};
