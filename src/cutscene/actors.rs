//! Cutscene actors: walk along simple grid paths with the walking animation.

use std::collections::VecDeque;

use macroquad::prelude::*;

use crate::map::{Map, TILE};
use crate::render::{Facing, PersonLook, PersonPose, Pose};

/// Walking speed in world px / s (≈ 2.3 tiles/s — a cutscene walk, a bit brisk).
pub const WALK_SPEED: f32 = 150.0;
/// Long walks are sped up so no walk takes longer than this.
pub const MAX_WALK_SECONDS: f32 = 4.5;

/// One person on stage.
#[derive(Debug, Clone)]
pub struct Actor {
    pub id: String,
    pub look: PersonLook,
    pub pos: Vec2,
    pub facing: Facing,
    path: VecDeque<Vec2>,
    speed: f32,
    /// Disappear once the path is done (`ActorExit`).
    pub exit_on_arrive: bool,
    /// 0..1 — fades in on enter, out on exit.
    pub alpha: f32,
    pub gone: bool,
    /// Carries a suitcase (drawn beside the feet).
    pub suitcase: bool,
    /// Warm handheld light around the actor (dark café).
    pub hand_light: bool,
}

impl Actor {
    pub fn new(id: &str, look: PersonLook, pos: Vec2) -> Actor {
        Actor {
            id: id.to_string(),
            look,
            pos,
            facing: Facing::Down,
            path: VecDeque::new(),
            speed: WALK_SPEED,
            exit_on_arrive: false,
            alpha: 0.0,
            gone: false,
            suitcase: false,
            hand_light: false,
        }
    }

    pub fn walking(&self) -> bool {
        !self.path.is_empty()
    }

    /// Done with its walk (and fully faded out, if it was leaving).
    pub fn arrived(&self) -> bool {
        self.path.is_empty() && (!self.exit_on_arrive || self.gone)
    }

    pub fn walk(&mut self, path: Vec<Vec2>) {
        let mut len = 0.0;
        let mut prev = self.pos;
        for p in &path {
            len += prev.distance(*p);
            prev = *p;
        }
        self.speed = WALK_SPEED.max(len / MAX_WALK_SECONDS);
        self.path = path.into();
    }

    /// Jump to the end of the walk (skipping).
    pub fn finish(&mut self) {
        if let Some(last) = self.path.back() {
            self.pos = *last;
        }
        self.path.clear();
        if self.exit_on_arrive {
            self.gone = true;
            self.alpha = 0.0;
        } else {
            self.alpha = 1.0;
        }
    }

    pub fn update(&mut self, dt: f32) {
        let mut budget = self.speed * dt;
        while budget > 0.0 {
            let Some(target) = self.path.front().copied() else {
                break;
            };
            let d = target - self.pos;
            let len = d.length();
            if len <= budget {
                self.pos = target;
                budget -= len;
                self.path.pop_front();
            } else {
                self.pos += d / len * budget;
                budget = 0.0;
            }
            if len > 0.5 {
                self.facing = Facing::from_dir(d);
            }
        }
        let target_alpha = if self.exit_on_arrive && self.path.is_empty() {
            0.0
        } else {
            1.0
        };
        let step = dt / 0.35;
        self.alpha = if self.alpha < target_alpha {
            (self.alpha + step).min(target_alpha)
        } else {
            (self.alpha - step).max(target_alpha)
        };
        if self.exit_on_arrive && self.path.is_empty() && self.alpha <= 0.0 {
            self.gone = true;
        }
    }

    pub fn pose(&self, t: f32) -> PersonPose {
        PersonPose {
            feet: self.pos,
            facing: self.facing,
            pose: if self.walking() {
                Pose::Walk
            } else {
                Pose::Idle
            },
            t,
        }
    }
}

/// Centre of a tile in world px (feet stand a little below the centre).
pub fn tile_feet(x: i32, y: i32) -> Vec2 {
    vec2(x as f32 + 0.5, y as f32 + 0.62) * TILE
}

/// A walkable path between two world points. Inside the café it follows the floor grid
/// (BFS around tables and the counter); outside (y < 0) it is a straight line.
pub fn find_path(map: &Map, from: Vec2, to: Vec2) -> Vec<Vec2> {
    let tile_of = |p: Vec2| ((p.x / TILE).floor() as i32, (p.y / TILE).floor() as i32);
    let (fx, fy) = tile_of(from);
    let (tx, ty) = tile_of(to);
    let inside =
        |x: i32, y: i32| x >= 0 && y >= 0 && (x as usize) < map.width && (y as usize) < map.height;
    if !inside(fx, fy) || !inside(tx, ty) {
        return vec![to];
    }
    let w = map.width;
    let idx = |x: i32, y: i32| y as usize * w + x as usize;
    let walk = |x: i32, y: i32| {
        inside(x, y)
            && map
                .get(x as usize, y as usize)
                .is_some_and(|t| t.walkable())
    };
    let mut prev = vec![usize::MAX; w * map.height];
    let mut q = VecDeque::new();
    prev[idx(fx, fy)] = idx(fx, fy);
    q.push_back((fx, fy));
    while let Some((x, y)) = q.pop_front() {
        if (x, y) == (tx, ty) {
            break;
        }
        for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
            let (nx, ny) = (x + dx, y + dy);
            let target = (nx, ny) == (tx, ty);
            if inside(nx, ny) && (walk(nx, ny) || target) && prev[idx(nx, ny)] == usize::MAX {
                prev[idx(nx, ny)] = idx(x, y);
                q.push_back((nx, ny));
            }
        }
    }
    if prev[idx(tx, ty)] == usize::MAX {
        return vec![to];
    }
    // walk back, then keep only the corners
    let mut tiles = vec![(tx, ty)];
    let mut cur = idx(tx, ty);
    while cur != idx(fx, fy) {
        cur = prev[cur];
        tiles.push(((cur % w) as i32, (cur / w) as i32));
    }
    tiles.reverse();
    let mut pts = Vec::new();
    for i in 1..tiles.len() {
        let corner = i + 1 == tiles.len() || {
            let (a, b, c) = (tiles[i - 1], tiles[i], tiles[i + 1]);
            (b.0 - a.0, b.1 - a.1) != (c.0 - b.0, c.1 - b.1)
        };
        if corner {
            pts.push(tile_feet(tiles[i].0, tiles[i].1));
        }
    }
    if let Some(last) = pts.last_mut() {
        *last = to;
    } else {
        pts.push(to);
    }
    pts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::CAFE_LAYOUT;

    #[test]
    fn path_goes_around_tables() {
        let map = Map::from_layout(CAFE_LAYOUT);
        let from = tile_feet(1, 14);
        let to = tile_feet(14, 6);
        let path = find_path(&map, from, to);
        assert_eq!(*path.last().unwrap(), to);
        // every segment is axis-aligned and every corner is walkable
        let mut p = from;
        for q in &path {
            let d = *q - p;
            assert!(d.x.abs() < 1e-3 || d.y.abs() < 1e-3 || q == path.last().unwrap());
            p = *q;
        }
    }

    #[test]
    fn walking_reaches_the_target_and_exit_disappears() {
        let mut a = Actor::new("x", PersonLook::random(1, false), vec2(0.0, 0.0));
        a.walk(vec![vec2(100.0, 0.0), vec2(100.0, 100.0)]);
        a.exit_on_arrive = true;
        for _ in 0..600 {
            a.update(1.0 / 60.0);
        }
        assert_eq!(a.pos, vec2(100.0, 100.0));
        assert!(a.gone && a.arrived());
    }
}
