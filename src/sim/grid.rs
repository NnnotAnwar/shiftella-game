//! Tile grid: floors, walls, rooms, the street strip, placed furniture, walkability and A*.
//!
//! Coordinates: tile `(x, y)`, world px = tile × [`TILE`]. The café is the 30×16 `map::CAFE_LAYOUT`
//! frame (same tiles as the cutscene spots); rows 16–17 are the street/sidewalk in front of the
//! door where guests and late staff appear.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

use serde::{Deserialize, Serialize};

use super::types::Facing;
use crate::content::{BuildCategory, Content, FurnitureDef};

pub const TILE: f32 = 64.0;

/// Default café: `map::CAFE_LAYOUT` with furniture letters made concrete, plus the street.
///
/// `#` wall · `W` window · `D` door · `.` hall floor (wood) · `,` staff floor (tile) ·
/// `=` street · `T` table_round · `h` chair (faces the table) · `l` floor_lamp (the window
/// light) · `p` plant_tall · `g` grandma_photo · `b` bookshelf · `j` grandma Lida's jukebox
/// (handson.md §10.1). Every table also gets a `pendant_lamp` (tag `light`) hanging over it.
///
/// The bar (rows 10–14, x 16–28) is a small night café's: the **front counter** (row 10) faces
/// the hall — `C` bar_counter, `G` espresso_grinder beside `M` the coffee machine, `R` the
/// register (guests queue along row 9 to its left). Behind it a three-tile walkway; against the
/// front wall the **back line** (row 14): `S` stove (2×1), a free spot for the oven, `F` fridge,
/// `K` sink with `r` its drying rack, `t` bin, then the back-bar shelves (`B`). Staff come in
/// through the flap at the right end of the counter (row 10, x 27–28) or past its open left
/// end (x 16, rows 11–14 — a counter return there made every trip to the left tables longer
/// and cost the Classic bot its final payment). `c` = a return (guests to the west).
///
/// `s` = grandma's bar stools along the guests' ledge of the front counter (row 9, x 16–19,
/// facing the counter): a quick coffee at the bar when the tables are full. They stand left
/// of the register line (which runs along row 9 from the register to x 20, then up), never
/// in the staff's way past the counter's open end (x 15). They are placed after everything
/// else, so every other piece keeps the uid it had before 0.2.2.
pub const DEFAULT_LAYOUT: &str = "\
####WWWW######WWWW######WWWW##
#p....l......g...............#
#..hTh.....hTh.....hTh.......#
#............................#
#............................#
#..hTh.....hTh.....hTh......b#
#............................#
#............................#
#..hTh.....hTh..............j#
#...............ssss.........#
#...............CCCCCCGMCRC,,#
#...............,,,,,,,,,,,,,#
#...............,,,,,,,,,,,,,#
#...............,,,,,,,,,,,,,#
#.........p.....,SS,FKrtBBB,,#
#DD###########################
==============================
==============================";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TileKind {
    Floor,
    Wall,
    Window,
    Door,
    /// Outdoor sidewalk (walkable, never buildable).
    Street,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Floor {
    Wood,
    Tile,
    Carpet,
    Pavement,
}

impl Floor {
    /// Render floor id for `render::draw_floor`.
    pub fn id(self) -> &'static str {
        match self {
            Floor::Wood => "floor_wood",
            Floor::Tile => "floor_tile",
            Floor::Carpet => "floor_carpet",
            Floor::Pavement => "pavement",
        }
    }
    pub fn from_id(id: &str) -> Option<Floor> {
        match id {
            "floor_wood" => Some(Floor::Wood),
            "floor_tile" => Some(Floor::Tile),
            "floor_carpet" => Some(Floor::Carpet),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Room {
    /// Guest area.
    Hall,
    /// Staff only: coffee / register side.
    Bar,
    /// Staff only: stove / fridge / sink side.
    Kitchen,
    Outside,
    /// Walls.
    None,
}

impl Room {
    pub fn staff_only(self) -> bool {
        matches!(self, Room::Bar | Room::Kitchen)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileInfo {
    pub kind: TileKind,
    pub floor: Floor,
    pub room: Room,
}

/// A placed furniture piece.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Placed {
    pub uid: u32,
    pub def: String,
    /// Top-left tile of the (rotated) footprint.
    pub x: i32,
    pub y: i32,
    /// 0 Down, 1 Left, 2 Up, 3 Right (see `Facing::from_rot`).
    pub rot: u8,
    /// Lamps / jukebox on, candle lit…
    pub on: bool,
    /// Table crumbs / dishes 0–1.
    pub dirt: f32,
    /// Dishes left on a table.
    pub dishes: u32,
    /// Appliance wear 0–1.
    pub wear: f32,
    /// Guest id per seat index.
    pub seat_by: Vec<Option<u32>>,
    /// Staff id currently using it (station / register / sink).
    pub user: Option<u32>,
    /// Claimed by a cleaner (tables).
    pub claim: Option<u32>,
    /// Price paid (refund basis).
    pub paid: u32,
}

impl Placed {
    pub fn facing(&self) -> Facing {
        Facing::from_rot(self.rot)
    }
}

/// Footprint size after rotation.
pub fn rotated_size(def: &FurnitureDef, rot: u8) -> (u32, u32) {
    if rot % 2 == 1 {
        (def.size.1, def.size.0)
    } else {
        def.size
    }
}

/// Lives on the rug layer (walkable, no seats): rugs, stage, pendant lamps.
pub fn is_rug_layer(def: &FurnitureDef) -> bool {
    def.walkable && def.seats == 0
}

/// Pure terrain + furniture occupancy. Derived walk masks are rebuilt by [`Grid::rebuild`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grid {
    pub width: i32,
    pub height: i32,
    pub tiles: Vec<TileInfo>,
    /// Non-rug furniture uid per tile.
    pub objects: Vec<Option<u32>>,
    /// Rug-layer furniture uid per tile.
    pub rugs: Vec<Option<u32>>,
    /// Tiles blocked by events (bucket under a leak).
    pub blocked: Vec<bool>,
    /// bit0 = walkable, bit1 = guests allowed, bit2 = reached from the door. Rebuilt from the
    /// above.
    #[serde(skip)]
    walk: Vec<u8>,
    /// Bumped on every change; path caches key on it.
    #[serde(skip)]
    pub version: u32,
    /// Seat tiles somebody sits on right now (`World::refresh_taken`): nobody else walks
    /// across or stands there.
    #[serde(skip)]
    taken: Vec<(i32, i32)>,
}

pub const WALK: u8 = 1;
pub const GUEST_OK: u8 = 2;
/// Reachable from the entrance (rebuilt with the masks).
pub const REACH: u8 = 16;
/// A seat of a walkable seat piece (chair, stool): people may cross it when there is no
/// other way, but paths and standing spots go around it (someone may be sitting there).
pub const SEAT: u8 = 4;
/// A seat someone sits on (see [`Grid::set_taken`]).
pub const TAKEN: u8 = 8;
/// Reached from the door without crossing a chair or a stool ([`SEAT`]): a standing spot
/// one can get to while every chair is taken (rebuilt with the masks).
pub const CLEAR: u8 = 32;
/// The same for guests: reached from the street over guests' floor without crossing a
/// chair (no staff-only floor).
pub const GUEST_CLEAR: u8 = 64;

impl Grid {
    pub fn idx(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && x < self.width && y < self.height)
            .then(|| (y * self.width + x) as usize)
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        self.idx(x, y).is_some()
    }

    pub fn tile(&self, x: i32, y: i32) -> Option<TileInfo> {
        self.idx(x, y).map(|i| self.tiles[i])
    }

    pub fn object_at(&self, x: i32, y: i32) -> Option<u32> {
        self.idx(x, y).and_then(|i| self.objects[i])
    }

    pub fn room(&self, x: i32, y: i32) -> Room {
        self.tile(x, y).map(|t| t.room).unwrap_or(Room::None)
    }

    pub fn walkable(&self, x: i32, y: i32) -> bool {
        self.idx(x, y)
            .is_some_and(|i| self.walk.get(i).copied().unwrap_or(0) & WALK != 0)
    }

    pub fn guest_ok(&self, x: i32, y: i32) -> bool {
        self.idx(x, y).is_some_and(|i| {
            let w = self.walk.get(i).copied().unwrap_or(0);
            w & WALK != 0 && w & GUEST_OK != 0
        })
    }

    /// The tile of a walkable seat (a chair, a stool): see [`SEAT`].
    pub fn is_seat(&self, x: i32, y: i32) -> bool {
        self.idx(x, y)
            .is_some_and(|i| self.walk.get(i).copied().unwrap_or(0) & SEAT != 0)
    }

    pub fn is_indoor_floor(&self, x: i32, y: i32) -> bool {
        self.tile(x, y).is_some_and(|t| t.kind == TileKind::Floor)
    }

    /// Recomputes walk masks. `defs` resolves a furniture uid to its walkability.
    pub fn rebuild(&mut self, furniture: &[Placed], content: &Content) {
        let mut walkable_obj: HashMap<u32, bool> = HashMap::new();
        let mut seat_obj: HashMap<u32, bool> = HashMap::new();
        for p in furniture {
            let d = content.furniture_def(&p.def);
            walkable_obj.insert(p.uid, d.is_some_and(|d| d.walkable));
            seat_obj.insert(p.uid, d.is_some_and(|d| d.seats > 0));
        }
        self.walk = vec![0; self.tiles.len()];
        for i in 0..self.tiles.len() {
            let t = self.tiles[i];
            let mut w = match t.kind {
                TileKind::Floor | TileKind::Door | TileKind::Street => WALK,
                TileKind::Wall | TileKind::Window => 0,
            };
            if let Some(uid) = self.objects[i]
                && !walkable_obj.get(&uid).copied().unwrap_or(false)
            {
                w = 0;
            }
            if self.blocked[i] {
                w = 0;
            }
            if w != 0 && !t.room.staff_only() {
                w |= GUEST_OK;
            }
            if w != 0
                && let Some(uid) = self.objects[i]
                && seat_obj.get(&uid).copied().unwrap_or(false)
            {
                w |= SEAT;
            }
            self.walk[i] = w;
        }
        // walkable tiles the door leads to (a walkable pocket boxed in by furniture is not)
        // (a bucket on the entrance tile: every walkable tile counts, as before)
        let start = self.entrance();
        let reach = if self.walkable(start.0, start.1) {
            reachable(self, start)
        } else {
            vec![true; self.tiles.len()]
        };
        for (i, r) in reach.into_iter().enumerate() {
            if r && self.walk[i] & WALK != 0 {
                self.walk[i] |= REACH;
            }
        }
        let way_in = self.way_in();
        let clear = if self.walkable(way_in.0, way_in.1) {
            reachable_off_seats(self, way_in)
        } else {
            vec![true; self.tiles.len()]
        };
        for (i, r) in clear.into_iter().enumerate() {
            if r && self.walk[i] & (WALK | SEAT) == WALK {
                self.walk[i] |= CLEAR;
            }
        }
        let guests = if self.walkable(way_in.0, way_in.1) {
            guest_reachable_off_seats(self, way_in)
        } else {
            vec![true; self.tiles.len()]
        };
        for (i, r) in guests.into_iter().enumerate() {
            if r && self.walk[i] & (WALK | GUEST_OK | SEAT) == WALK | GUEST_OK {
                self.walk[i] |= GUEST_CLEAR;
            }
        }
        for &(x, y) in &self.taken {
            if let Some(i) = self.idx(x, y) {
                self.walk[i] |= TAKEN;
            }
        }
        self.version = self.version.wrapping_add(1);
    }

    /// A walkable tile one can walk to from the door. A tile walled in by furniture (the
    /// placement check only keeps every *piece* reachable) is walkable but not reached.
    pub fn reached(&self, x: i32, y: i32) -> bool {
        self.idx(x, y)
            .is_some_and(|i| self.walk.get(i).copied().unwrap_or(0) & REACH != 0)
    }

    /// A floor tile (not a seat) reached from the door without crossing a chair: see
    /// [`CLEAR`].
    pub fn clear(&self, x: i32, y: i32) -> bool {
        self.idx(x, y)
            .is_some_and(|i| self.walk.get(i).copied().unwrap_or(0) & CLEAR != 0)
    }

    /// A guests' floor tile (not a seat) they reach from the street without crossing a
    /// chair: see [`GUEST_CLEAR`].
    pub fn guest_clear(&self, x: i32, y: i32) -> bool {
        self.idx(x, y)
            .is_some_and(|i| self.walk.get(i).copied().unwrap_or(0) & GUEST_CLEAR != 0)
    }

    /// The chairs one crosses to get into or out of the nook around `t` — a floor tile
    /// reached only over a chair (not [`CLEAR`]); empty for clear floor and for a seat. The
    /// nook's floor is followed through its chairs into a nook behind them, never from one
    /// chair to the next (a table's row of chairs isn't one way in).
    pub fn nook_gates(&self, t: (i32, i32)) -> Vec<(i32, i32)> {
        if !self.walkable(t.0, t.1) || self.clear(t.0, t.1) || self.is_seat(t.0, t.1) {
            return Vec::new();
        }
        let Some(start) = self.idx(t.0, t.1) else {
            return Vec::new();
        };
        let mut seen = vec![false; self.tiles.len()];
        seen[start] = true;
        let mut stack = vec![t];
        let mut gates = Vec::new();
        let mut steps = 0;
        while let Some((x, y)) = stack.pop() {
            steps += 1;
            if steps > 400 {
                break;
            }
            let from_seat = self.is_seat(x, y);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let (nx, ny) = (x + dx, y + dy);
                    let Some(i) = self.idx(nx, ny) else {
                        continue;
                    };
                    if seen[i] || !self.walkable(nx, ny) || self.clear(nx, ny) {
                        continue;
                    }
                    let seat = self.is_seat(nx, ny);
                    // floor ↔ chair only (4-way into the floor, any way onto a chair)
                    if from_seat && (seat || dx != 0 && dy != 0) {
                        continue;
                    }
                    if !from_seat && !seat && dx != 0 && dy != 0 {
                        continue;
                    }
                    seen[i] = true;
                    if seat {
                        gates.push((nx, ny));
                    }
                    stack.push((nx, ny));
                }
            }
        }
        gates.sort_unstable();
        gates
    }

    /// Marks the seats people sit on (path caches see a new version when it changed).
    pub fn set_taken(&mut self, mut tiles: Vec<(i32, i32)>) {
        tiles.sort_unstable();
        tiles.dedup();
        if tiles == self.taken || self.walk.len() != self.tiles.len() {
            self.taken = tiles;
            return;
        }
        for &(x, y) in &self.taken {
            if let Some(i) = self.idx(x, y) {
                self.walk[i] &= !TAKEN;
            }
        }
        for &(x, y) in &tiles {
            if let Some(i) = self.idx(x, y) {
                self.walk[i] |= TAKEN;
            }
        }
        self.taken = tiles;
        self.version = self.version.wrapping_add(1);
    }

    /// Somebody sits on this tile.
    pub fn is_taken(&self, x: i32, y: i32) -> bool {
        self.idx(x, y)
            .is_some_and(|i| self.walk.get(i).copied().unwrap_or(0) & TAKEN != 0)
    }

    pub fn ensure_masks(&mut self, furniture: &[Placed], content: &Content) {
        if self.walk.len() != self.tiles.len() {
            self.rebuild(furniture, content);
        }
    }

    /// The door tile(s), the inside tile in front of the first door and a street spawn point.
    pub fn door_tiles(&self) -> Vec<(i32, i32)> {
        let mut v = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                if self.tile(x, y).is_some_and(|t| t.kind == TileKind::Door) {
                    v.push((x, y));
                }
            }
        }
        v
    }

    /// Indoor tile just inside the main door.
    pub fn entrance(&self) -> (i32, i32) {
        for (x, y) in self.door_tiles() {
            for (dx, dy) in [(0, -1), (0, 1), (1, 0), (-1, 0)] {
                if self.is_indoor_floor(x + dx, y + dy) && self.object_at(x + dx, y + dy).is_none()
                {
                    return (x + dx, y + dy);
                }
            }
        }
        (1, 1)
    }

    /// Where people come in from for the "without crossing a chair" checks: the street
    /// outside the door (`entrance` skips a tile with a piece on it, so a chair right inside
    /// the door — everyone's only way in — was not seen as in the way).
    pub fn way_in(&self) -> (i32, i32) {
        let o = self.outside_door();
        if self.walkable(o.0, o.1) {
            o
        } else {
            self.entrance()
        }
    }

    /// Street tile outside the main door.
    pub fn outside_door(&self) -> (i32, i32) {
        for (x, y) in self.door_tiles() {
            for (dx, dy) in [(0, 1), (0, -1), (1, 0), (-1, 0)] {
                if self
                    .tile(x + dx, y + dy)
                    .is_some_and(|t| t.kind == TileKind::Street)
                {
                    return (x + dx, y + dy);
                }
            }
        }
        (0, self.height - 1)
    }

    /// Street edge tiles where people appear / vanish.
    pub fn street_edges(&self) -> [(i32, i32); 2] {
        let y = self.height - 1;
        [(0, y), (self.width - 1, y)]
    }
}

/// Parses [`DEFAULT_LAYOUT`] into a grid + furniture list (uids from `next_uid`).
pub fn default_cafe(content: &Content, next_uid: &mut u32) -> (Grid, Vec<Placed>) {
    parse_layout(DEFAULT_LAYOUT, content, next_uid)
}

pub fn parse_layout(layout: &str, content: &Content, next_uid: &mut u32) -> (Grid, Vec<Placed>) {
    let rows: Vec<Vec<char>> = layout.lines().map(|l| l.chars().collect()).collect();
    let height = rows.len() as i32;
    let width = rows.iter().map(|r| r.len()).max().unwrap_or(0) as i32;
    let n = (width * height) as usize;
    let mut grid = Grid {
        width,
        height,
        tiles: Vec::with_capacity(n),
        objects: vec![None; n],
        rugs: vec![None; n],
        blocked: vec![false; n],
        walk: Vec::new(),
        version: 0,
        taken: Vec::new(),
    };
    // First pass: terrain + rooms.
    for (y, row) in rows.iter().enumerate() {
        for x in 0..width as usize {
            let c = row.get(x).copied().unwrap_or('#');
            let staff_floor = c == ',' || "CcBGRMSFKrt".contains(c) && y >= 10;
            let tile = match c {
                '#' => TileInfo {
                    kind: TileKind::Wall,
                    floor: Floor::Wood,
                    room: Room::None,
                },
                'W' => TileInfo {
                    kind: TileKind::Window,
                    floor: Floor::Wood,
                    room: Room::None,
                },
                'D' => TileInfo {
                    kind: TileKind::Door,
                    floor: Floor::Wood,
                    room: Room::Hall,
                },
                '=' => TileInfo {
                    kind: TileKind::Street,
                    floor: Floor::Pavement,
                    room: Room::Outside,
                },
                _ if staff_floor => TileInfo {
                    kind: TileKind::Floor,
                    floor: Floor::Tile,
                    room: if y >= 13 { Room::Kitchen } else { Room::Bar },
                },
                _ => TileInfo {
                    kind: TileKind::Floor,
                    floor: Floor::Wood,
                    room: Room::Hall,
                },
            };
            grid.tiles.push(tile);
        }
    }
    // Counter row tiles are staff side too (they're blocked anyway).
    let mut furniture: Vec<Placed> = Vec::new();
    let mut tables: Vec<(i32, i32)> = Vec::new();
    let mut stools: Vec<(i32, i32)> = Vec::new();
    let mut place = |grid: &mut Grid, def_id: &str, x: i32, y: i32, rot: u8| {
        let Some(def) = content.furniture_def(def_id) else {
            return;
        };
        let (w, h) = rotated_size(def, rot);
        let uid = *next_uid;
        *next_uid += 1;
        for dy in 0..h as i32 {
            for dx in 0..w as i32 {
                if let Some(i) = grid.idx(x + dx, y + dy) {
                    if is_rug_layer(def) {
                        grid.rugs[i] = Some(uid);
                    } else {
                        grid.objects[i] = Some(uid);
                    }
                }
            }
        }
        furniture.push(Placed {
            uid,
            def: def_id.to_string(),
            x,
            y,
            rot,
            on: def.has_tag("light") && !def.has_tag("music"),
            dirt: 0.0,
            dishes: 0,
            wear: 0.0,
            seat_by: vec![None; def.seats as usize],
            user: None,
            claim: None,
            paid: def.price,
        });
    };
    for (y, row) in rows.iter().enumerate() {
        let y = y as i32;
        let mut x = 0usize;
        while x < row.len() {
            let c = row[x];
            let xi = x as i32;
            match c {
                'T' => {
                    place(&mut grid, "table_round", xi, y, 0);
                    tables.push((xi, y));
                }
                // a bar stool on the counter's ledge, facing the counter (placed last)
                's' => stools.push((xi, y)),
                'h' => {
                    // Face the neighbouring table.
                    let right = row.get(x + 1) == Some(&'T');
                    place(&mut grid, "chair", xi, y, if right { 3 } else { 1 });
                }
                // a counter's `rot` = where its guests are: 0 the front counter (guests across
                // it, beyond the camera), 2 a back bar (no guests), 3 a return (guests west)
                'C' => place(&mut grid, "bar_counter", xi, y, 0),
                'c' => place(&mut grid, "bar_counter", xi, y, 3),
                'B' => place(&mut grid, "bar_counter", xi, y, 2),
                'R' => place(&mut grid, "register", xi, y, 0),
                'G' => place(&mut grid, "espresso_grinder", xi, y, 0),
                'M' => place(&mut grid, "coffee_machine", xi, y, 0),
                'S' => {
                    place(&mut grid, "stove", xi, y, 2);
                    x += 1; // 2×1
                }
                'F' => place(&mut grid, "fridge", xi, y, 2),
                'K' => place(&mut grid, "sink", xi, y, 2),
                'r' => place(&mut grid, "dish_rack", xi, y, 2),
                't' => place(&mut grid, "trash_bin", xi, y, 2),
                'l' => place(&mut grid, "floor_lamp", xi, y, 0),
                'p' => place(&mut grid, "plant_tall", xi, y, 0),
                'g' => place(&mut grid, "grandma_photo", xi, y, 0),
                'b' => place(&mut grid, "bookshelf", xi, y, 1),
                'j' => place(&mut grid, "jukebox", xi, y, 1),
                _ => {}
            }
            x += 1;
        }
    }
    // A pendant lamp over every table (rug layer: shares the table's tile). The warm pools
    // of light come from the sim's furniture, so the renderer just draws `pendant_lamp`s.
    for (x, y) in std::mem::take(&mut tables) {
        place(&mut grid, "pendant_lamp", x, y, 0);
    }
    // 0.2.2: the bar stools come last (the pieces before them keep their old uids)
    for (x, y) in std::mem::take(&mut stools) {
        place(&mut grid, "stool", x, y, 0);
    }
    // The window lamp starts off (the player lights it — `light_window` checklist).
    for p in furniture.iter_mut() {
        if p.def == "floor_lamp" {
            p.on = false;
        }
    }
    grid.rebuild(&furniture, content);
    (grid, furniture)
}

/// Tiles covered by a piece.
pub fn footprint(def: &FurnitureDef, x: i32, y: i32, rot: u8) -> Vec<(i32, i32)> {
    let (w, h) = rotated_size(def, rot);
    let mut v = Vec::with_capacity((w * h) as usize);
    for dy in 0..h as i32 {
        for dx in 0..w as i32 {
            v.push((x + dx, y + dy));
        }
    }
    v
}

/// Walkable 4-neighbours of a footprint, the tile "in front" (facing) first.
pub fn access_tiles(grid: &Grid, def: &FurnitureDef, p: &Placed) -> Vec<(i32, i32)> {
    let fp = footprint(def, p.x, p.y, p.rot);
    let front = match p.facing() {
        Facing::Down => (0, 1),
        Facing::Up => (0, -1),
        Facing::Left => (-1, 0),
        Facing::Right => (1, 0),
    };
    let mut out: Vec<(i32, i32)> = Vec::new();
    let mut seats: Vec<(i32, i32)> = Vec::new();
    for dir in [front, (0, 1), (0, -1), (1, 0), (-1, 0)] {
        for &(x, y) in &fp {
            let t = (x + dir.0, y + dir.1);
            if !fp.contains(&t) && grid.walkable(t.0, t.1) && !out.contains(&t) {
                // a chair beside it is the last choice: someone may sit there
                if grid.is_seat(t.0, t.1) {
                    if !seats.contains(&t) {
                        seats.push(t);
                    }
                } else {
                    out.push(t);
                }
            }
        }
    }
    out.extend(seats);
    out
}

/// Validates that a footprint can hold a new piece. `ignore` = uid being moved.
pub fn can_place(
    grid: &Grid,
    def: &FurnitureDef,
    x: i32,
    y: i32,
    rot: u8,
    ignore: Option<u32>,
) -> Result<(), super::types::CommandError> {
    use super::types::CommandError;
    match def.category {
        BuildCategory::Floors => {
            return if grid.is_indoor_floor(x, y) {
                Ok(())
            } else {
                Err(CommandError::OutOfBounds)
            };
        }
        BuildCategory::Structure => {
            return Err(CommandError::Invalid(
                "structure editing is not supported".into(),
            ));
        }
        _ => {}
    }
    for (tx, ty) in footprint(def, x, y, rot) {
        let Some(i) = grid.idx(tx, ty) else {
            return Err(CommandError::OutOfBounds);
        };
        if grid.tiles[i].kind != TileKind::Floor || grid.blocked[i] {
            return Err(CommandError::OutOfBounds);
        }
        let layer = if is_rug_layer(def) {
            grid.rugs[i]
        } else {
            grid.objects[i]
        };
        if let Some(uid) = layer
            && Some(uid) != ignore
        {
            return Err(CommandError::Occupied);
        }
    }
    Ok(())
}

/// Flood fill from `start` over walkable tiles.
pub fn reachable(grid: &Grid, start: (i32, i32)) -> Vec<bool> {
    reachable_by(grid, start, false, false)
}

/// The same without walking over chairs and stools ([`SEAT`] tiles).
pub fn reachable_off_seats(grid: &Grid, start: (i32, i32)) -> Vec<bool> {
    reachable_by(grid, start, true, false)
}

/// Where a guest walks without crossing a chair (no staff-only floor, no [`SEAT`] tiles).
pub fn guest_reachable_off_seats(grid: &Grid, start: (i32, i32)) -> Vec<bool> {
    reachable_by(grid, start, true, true)
}

fn reachable_by(grid: &Grid, start: (i32, i32), off_seats: bool, guest: bool) -> Vec<bool> {
    let mut seen = vec![false; grid.tiles.len()];
    let mut stack = vec![start];
    if let Some(i) = grid.idx(start.0, start.1) {
        seen[i] = true;
    }
    while let Some((x, y)) = stack.pop() {
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            if let Some(i) = grid.idx(nx, ny)
                && !seen[i]
                && grid.walkable(nx, ny)
                && !(guest && !grid.guest_ok(nx, ny))
                && !(off_seats && grid.is_seat(nx, ny))
            {
                seen[i] = true;
                stack.push((nx, ny));
            }
        }
    }
    seen
}

// ---------------------------------------------------------------------------------------------
// A*
// ---------------------------------------------------------------------------------------------

/// 8-directional A* with no corner cutting. The goal tile may be non-walkable (sofa seat,
/// counter) — the path then ends on it. `guest` = stay out of staff-only rooms.
/// Returns the tiles after `from` up to and including `to`.
pub fn find_path(
    grid: &Grid,
    from: (i32, i32),
    to: (i32, i32),
    guest: bool,
) -> Option<Vec<(i32, i32)>> {
    // around the chairs (someone may sit there) when there is a way; across a free chair
    // otherwise; over someone's chair only when nothing else leads there (an old layout
    // where a chair is reached through another — new builds can't do that)
    find_path_in(grid, from, to, guest, 2)
        .or_else(|| find_path_in(grid, from, to, guest, 1))
        .or_else(|| find_path_in(grid, from, to, guest, 0))
}

fn find_path_in(
    grid: &Grid,
    from: (i32, i32),
    to: (i32, i32),
    guest: bool,
    // 2 = around every chair, 1 = around the taken ones, 0 = anywhere walkable
    avoid: u8,
) -> Option<Vec<(i32, i32)>> {
    if from == to {
        return Some(Vec::new());
    }
    let (Some(start), Some(goal)) = (grid.idx(from.0, from.1), grid.idx(to.0, to.1)) else {
        return None;
    };
    let pass = |x: i32, y: i32| -> bool {
        if (x, y) == to {
            return true;
        }
        if avoid >= 1 && grid.is_taken(x, y) || avoid >= 2 && grid.is_seat(x, y) {
            return false;
        }
        if guest {
            grid.guest_ok(x, y)
        } else {
            grid.walkable(x, y)
        }
    };
    let n = grid.tiles.len();
    let mut g = vec![u32::MAX; n];
    let mut came = vec![u32::MAX; n];
    let mut heap = BinaryHeap::new();
    let h = |x: i32, y: i32| -> u32 {
        let dx = (x - to.0).unsigned_abs();
        let dy = (y - to.1).unsigned_abs();
        10 * dx.max(dy) + 4 * dx.min(dy)
    };
    g[start] = 0;
    heap.push(Reverse((h(from.0, from.1), 0u32, start as u32)));
    while let Some(Reverse((_, cost, cur))) = heap.pop() {
        let cur = cur as usize;
        if cur == goal {
            break;
        }
        if cost > g[cur] {
            continue;
        }
        let cx = cur as i32 % grid.width;
        let cy = cur as i32 / grid.width;
        for (dx, dy) in [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ] {
            let (nx, ny) = (cx + dx, cy + dy);
            let Some(ni) = grid.idx(nx, ny) else {
                continue;
            };
            if !pass(nx, ny) {
                continue;
            }
            let diag = dx != 0 && dy != 0;
            // no corner cutting
            if diag && (!pass(cx + dx, cy) || !pass(cx, cy + dy)) {
                continue;
            }
            let step = if diag { 14 } else { 10 };
            let ng = cost + step;
            if ng < g[ni] {
                g[ni] = ng;
                came[ni] = cur as u32;
                heap.push(Reverse((ng + h(nx, ny), ng, ni as u32)));
            }
        }
    }
    if g[goal] == u32::MAX {
        return None;
    }
    let mut path = Vec::new();
    let mut cur = goal;
    while cur != start {
        path.push((cur as i32 % grid.width, cur as i32 / grid.width));
        cur = came[cur] as usize;
        if cur == u32::MAX as usize {
            return None;
        }
    }
    path.reverse();
    Some(path)
}

/// Tiles to walk, start excluded.
pub type Path = Vec<(i32, i32)>;

/// Path cache keyed by (from, to, guest) for one grid version.
#[derive(Debug, Clone, Default)]
pub struct PathCache {
    version: u32,
    map: HashMap<(u32, u32, bool), Option<Path>>,
    pub hits: u64,
    pub misses: u64,
}

impl PathCache {
    pub fn get(
        &mut self,
        grid: &Grid,
        from: (i32, i32),
        to: (i32, i32),
        guest: bool,
    ) -> Option<Vec<(i32, i32)>> {
        if self.version != grid.version {
            self.map.clear();
            self.version = grid.version;
        }
        let key = (
            (from.0 as u32) << 16 | from.1 as u32 & 0xffff,
            (to.0 as u32) << 16 | to.1 as u32 & 0xffff,
            guest,
        );
        if let Some(p) = self.map.get(&key) {
            self.hits += 1;
            return p.clone();
        }
        self.misses += 1;
        let p = find_path(grid, from, to, guest);
        if self.map.len() > 8192 {
            self.map.clear();
        }
        self.map.insert(key, p.clone());
        p
    }
}

/// World px of a tile centre.
pub fn tile_center(t: (i32, i32)) -> (f32, f32) {
    ((t.0 as f32 + 0.5) * TILE, (t.1 as f32 + 0.5) * TILE)
}

pub fn px_to_tile(p: (f32, f32)) -> (i32, i32) {
    ((p.0 / TILE).floor() as i32, (p.1 / TILE).floor() as i32)
}

pub fn chebyshev(a: (i32, i32), b: (i32, i32)) -> i32 {
    (a.0 - b.0).abs().max((a.1 - b.1).abs())
}

pub fn manhattan(a: (i32, i32), b: (i32, i32)) -> i32 {
    (a.0 - b.0).abs() + (a.1 - b.1).abs()
}
