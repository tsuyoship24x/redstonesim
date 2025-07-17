// src/lib.rs

// redstonesim ‑ L0 core
// Basic tick‑diff simulation with internal timers (button / repeater)
// ================================================
// Cargo.toml (minimal)
// [package]
// name = "redstonesim"
// version = "0.1.0"
// edition = "2021"
//
// [dependencies]
// serde = { version = "1.0", features = ["derive"] }
// serde_json = "1.0"
//
// =================================================

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

// -------------------------------------------------
// Position
// -------------------------------------------------
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    North,
    East,
    South,
    West,
    Up,
    Down,
}

impl Direction {
    fn offset(self) -> (i32, i32, i32) {
        match self {
            Direction::East => (1, 0, 0),
            Direction::West => (-1, 0, 0),
            Direction::Up => (0, 1, 0),
            Direction::Down => (0, -1, 0),
            Direction::South => (0, 0, 1),
            Direction::North => (0, 0, -1),
        }
    }

    fn opposite(self) -> Self {
        match self {
            Direction::East => Direction::West,
            Direction::West => Direction::East,
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::South => Direction::North,
            Direction::North => Direction::South,
        }
    }

    fn all() -> [Direction; 6] {
        [
            Direction::East,
            Direction::West,
            Direction::Up,
            Direction::Down,
            Direction::South,
            Direction::North,
        ]
    }
}

/// Calculate the `Direction` from one block to an adjacent block.
fn dir_from_to(from: Pos, to: Pos) -> Direction {
    for d in Direction::all() {
        let (dx, dy, dz) = d.offset();
        if from.x + dx == to.x && from.y + dy == to.y && from.z + dz == to.z {
            return d;
        }
    }
    panic!("positions are not adjacent: {:?} -> {:?}", from, to);
}

/// Trait for blocks that know where they accept input from and send output to.
pub trait Connectable {
    fn input_positions(&self, pos: Pos) -> Vec<Pos>;
    fn output_positions(&self, pos: Pos) -> Vec<Pos>;
}

// -------------------------------------------------
// Block kinds & internal state
// -------------------------------------------------
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum BlockKind {
    Lever {
        on: bool,
        facing: Direction,
    },
    Button {
        ticks_remaining: u8,
        facing: Direction,
    }, // keeps signal while > 0
    Dust {
        power: u8,
    }, // 0 = off, 1‑15 = power level
    Lamp {
        on: bool,
    },
    Repeater {
        delay: u8,           // configured delay (1‑4)
        ticks_remaining: u8, // countdown until output
        powered: bool,       // current output state
        facing: Direction,
    },
    Comparator {
        output: u8, // current output power
        facing: Direction,
    },
    Torch {
        lit: bool,
        facing: Direction,
    },
    Piston {
        extended: bool,
        facing: Direction,
    },
    Hopper {
        enabled: bool,
        facing: Direction,
    },
}

impl Connectable for BlockKind {
    fn input_positions(&self, pos: Pos) -> Vec<Pos> {
        match self {
            BlockKind::Lever { .. } | BlockKind::Button { .. } => Vec::new(),
            BlockKind::Dust { .. }
            | BlockKind::Lamp { .. }
            | BlockKind::Piston { .. }
            | BlockKind::Hopper { .. }
            | BlockKind::Comparator { .. } => Direction::all()
                .iter()
                .map(|d| {
                    let (dx, dy, dz) = d.offset();
                    Pos { x: pos.x + dx, y: pos.y + dy, z: pos.z + dz }
                })
                .collect(),
            BlockKind::Repeater { facing, .. } => {
                let back = facing.opposite();
                let (dx, dy, dz) = back.offset();
                vec![Pos { x: pos.x + dx, y: pos.y + dy, z: pos.z + dz }]
            }
            BlockKind::Torch { facing, .. } => {
                let (dx, dy, dz) = facing.offset();
                vec![Pos { x: pos.x + dx, y: pos.y + dy, z: pos.z + dz }]
            }
        }
    }

    fn output_positions(&self, pos: Pos) -> Vec<Pos> {
        match self {
            BlockKind::Lever { facing, .. }
            | BlockKind::Button { facing, .. }
            | BlockKind::Repeater { facing, .. }
            | BlockKind::Comparator { facing, .. } => {
                let (dx, dy, dz) = facing.offset();
                vec![Pos { x: pos.x + dx, y: pos.y + dy, z: pos.z + dz }]
            }
            BlockKind::Torch { facing, .. } => Direction::all()
                .iter()
                .filter_map(|d| {
                    if *d == *facing {
                        None
                    } else {
                        let (dx, dy, dz) = d.offset();
                        Some(Pos { x: pos.x + dx, y: pos.y + dy, z: pos.z + dz })
                    }
                })
                .collect(),
            BlockKind::Dust { .. } => Direction::all()
                .iter()
                .map(|d| {
                    let (dx, dy, dz) = d.offset();
                    Pos { x: pos.x + dx, y: pos.y + dy, z: pos.z + dz }
                })
                .collect(),
            BlockKind::Lamp { .. }
            | BlockKind::Piston { .. }
            | BlockKind::Hopper { .. } => Vec::new(),
        }
    }
}

// -------------------------------------------------
// A block placed in the world
// -------------------------------------------------
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlacedBlock {
    #[serde(flatten)]
    pub pos: Pos,
    #[serde(flatten)]
    pub kind: BlockKind,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct World {
    pub blocks: Vec<PlacedBlock>,
}

impl World {
    fn into_map(self) -> HashMap<Pos, BlockKind> {
        self.blocks.into_iter().map(|b| (b.pos, b.kind)).collect()
    }
}

// -------------------------------------------------
// Simulation request / response
// -------------------------------------------------
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimRequest {
    pub ticks: u32,   // maximum ticks to simulate
    pub world: World, // t = 0 state (raw user input)
    #[serde(default = "default_true")]
    pub early_exit: bool, // stop when stable & no timers running
}
fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockChange {
    #[serde(flatten)]
    pub pos: Pos,
    #[serde(flatten)]
    pub kind: BlockKind,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TickDiff {
    pub tick: u32,
    pub changes: Vec<BlockChange>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Termination {
    Stable,          // reached stable state (no external or internal changes)
    MaxTicksReached, // hit user‑specified limit
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimResponse {
    pub diffs: Vec<TickDiff>,
    pub terminated: Termination,
}

// -------------------------------------------------
// Public entry point
// -------------------------------------------------
/// Simulate the world for `request.ticks` or until it becomes stable.
/// Returns per‑tick diffs only for blocks that actually changed.
pub fn simulate(request: SimRequest) -> SimResponse {
    let mut world = request.world.into_map();
    let mut diffs: Vec<TickDiff> = Vec::new();

    // helper to query output from a block toward a direction
    fn output_towards(block: &BlockKind, dir: Direction) -> u8 {
        match block {
            BlockKind::Lever { on: true, facing } if *facing == dir => 15,
            BlockKind::Button { ticks_remaining, facing }
                if *ticks_remaining > 0 && *facing == dir => 15,
            BlockKind::Repeater { powered: true, facing, .. } if *facing == dir => 15,
            BlockKind::Comparator { output, facing } if *output > 0 && *facing == dir => *output,
            BlockKind::Torch { lit: true, facing } if dir != *facing => 15,
            BlockKind::Dust { power } => *power,
            _ => 0,
        }
    }

    fn mark_outputs(block: &BlockKind, pos: Pos, set: &mut HashSet<Pos>) {
        for n in block.output_positions(pos) {
            set.insert(n);
        }
    }

    let mut dirty: HashSet<Pos> = world.keys().cloned().collect();

    for tick in 1..=request.ticks {
        let mut changes: Vec<BlockChange> = Vec::new();
        let snapshot = world.clone();
        let mut next_dirty: HashSet<Pos> = HashSet::new();

        for pos in dirty.iter() {
            if let Some(block) = world.get_mut(pos) {
                let mut changed = false;
                let mut mark_out = false;
                match block {
                    BlockKind::Button { ticks_remaining, .. } => {
                        if *ticks_remaining > 0 {
                            let prev_output = 15;
                            *ticks_remaining -= 1;
                            let new_output = if *ticks_remaining > 0 { 15 } else { 0 };
                            changed = true;
                            if prev_output != new_output {
                                mark_out = true;
                            }
                            if *ticks_remaining > 0 {
                                next_dirty.insert(*pos);
                            }
                        }
                    }
                    BlockKind::Repeater { delay, ticks_remaining, powered, facing } => {
                        let back = facing.opposite();
                        let (dx, dy, dz) = back.offset();
                        let n = Pos { x: pos.x + dx, y: pos.y + dy, z: pos.z + dz };
                        let mut input = 0;
                        if let Some(nb) = snapshot.get(&n) {
                            input = output_towards(nb, *facing);
                        }

                        let prev_output = if *powered { 15 } else { 0 };

                        if input > 0 {
                            if !*powered && *ticks_remaining == 0 {
                                *ticks_remaining = *delay;
                            }
                        } else {
                            *powered = false;
                            *ticks_remaining = 0;
                        }

                        if *ticks_remaining > 0 {
                            *ticks_remaining -= 1;
                            if *ticks_remaining == 0 && input > 0 {
                                *powered = true;
                            }
                        }

                        let new_output = if *powered { 15 } else { 0 };

                        if prev_output != new_output || *ticks_remaining != 0 {
                            changed = true;
                        }

                        if prev_output != new_output {
                            mark_out = true;
                        }

                        if *ticks_remaining > 0 {
                            next_dirty.insert(*pos);
                        }
                    }
                    BlockKind::Comparator { output, .. } => {
                        let mut new_out = 0;
                        for n in block.input_positions(*pos) {
                            if let Some(nb) = snapshot.get(&n) {
                                let dir = dir_from_to(n, *pos);
                                new_out = new_out.max(output_towards(nb, dir));
                            }
                        }
                        if *output != new_out {
                            *output = new_out;
                            changed = true;
                            mark_out = true;
                        }
                    }
                    BlockKind::Dust { power } => {
                        let mut new_power = 0;
                        for n in block.input_positions(*pos) {
                            if let Some(nb) = snapshot.get(&n) {
                                let dir = dir_from_to(n, *pos);
                                let pw = output_towards(nb, dir);
                                let candidate = match nb {
                                    BlockKind::Dust { power: p, .. } => p.saturating_sub(1),
                                    _ => pw,
                                };
                                new_power = new_power.max(candidate);
                            }
                        }
                        if *power != new_power {
                            *power = new_power;
                            changed = true;
                            mark_out = true;
                        }
                    }
                    BlockKind::Lamp { on } => {
                        let mut powered = false;
                        for n in block.input_positions(*pos) {
                            if let Some(nb) = snapshot.get(&n) {
                                let dir = dir_from_to(n, *pos);
                                if output_towards(nb, dir) > 0 {
                                    powered = true;
                                    break;
                                }
                            }
                        }
                        if *on != powered {
                            *on = powered;
                            changed = true;
                        }
                    }
                    BlockKind::Torch { lit, facing } => {
                        let mut powered = false;
                        let (dx, dy, dz) = facing.offset();
                        let n = Pos { x: pos.x + dx, y: pos.y + dy, z: pos.z + dz };
                        if let Some(nb) = snapshot.get(&n) {
                            if output_towards(nb, facing.opposite()) > 0 {
                                powered = true;
                            }
                        }
                        let new_lit = !powered;
                        if *lit != new_lit {
                            *lit = new_lit;
                            changed = true;
                            mark_out = true;
                        }
                    }
                    BlockKind::Piston { extended, .. } => {
                        let mut powered = false;
                        for n in block.input_positions(*pos) {
                            if let Some(nb) = snapshot.get(&n) {
                                let dir = dir_from_to(n, *pos);
                                if output_towards(nb, dir) > 0 {
                                    powered = true;
                                    break;
                                }
                            }
                        }
                        if *extended != powered {
                            *extended = powered;
                            changed = true;
                            mark_out = true;
                        }
                    }
                    BlockKind::Hopper { enabled, .. } => {
                        let mut powered = false;
                        for n in block.input_positions(*pos) {
                            if let Some(nb) = snapshot.get(&n) {
                                let dir = dir_from_to(n, *pos);
                                if output_towards(nb, dir) > 0 {
                                    powered = true;
                                    break;
                                }
                            }
                        }
                        let new_enabled = !powered;
                        if *enabled != new_enabled {
                            *enabled = new_enabled;
                            changed = true;
                        }
                    }
                    _ => {}
                }

                if changed {
                    changes.push(BlockChange { pos: *pos, kind: block.clone() });
                }
                if mark_out {
                    mark_outputs(block, *pos, &mut next_dirty);
                }
            }
        }

        if !changes.is_empty() {
            diffs.push(TickDiff { tick, changes });
        } else if request.early_exit {
            let timers_active = world.values().any(|b| match b {
                BlockKind::Button { ticks_remaining, .. } if *ticks_remaining > 0 => true,
                BlockKind::Repeater { ticks_remaining, .. } if *ticks_remaining > 0 => true,
                _ => false,
            });
            if !timers_active {
                return SimResponse {
                    diffs,
                    terminated: Termination::Stable,
                };
            }
        }

        dirty = next_dirty;
    }

    SimResponse {
        diffs,
        terminated: Termination::MaxTicksReached,
    }
}

// -------------------------------------------------
// Unit tests
// -------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lever_to_lamp_one_tick() {
        let world = World {
            blocks: vec![
                PlacedBlock {
                    pos: Pos { x: 0, y: 0, z: 0 },
                    kind: BlockKind::Lever { on: true, facing: Direction::East },
                },
                PlacedBlock {
                    pos: Pos { x: 1, y: 0, z: 0 },
                    kind: BlockKind::Dust { power: 0 },
                },
                PlacedBlock {
                    pos: Pos { x: 2, y: 0, z: 0 },
                    kind: BlockKind::Lamp { on: false },
                },
            ],
        };
        let req = SimRequest {
            ticks: 5,
            world,
            early_exit: true,
        };
        let res = simulate(req);
        assert!(matches!(res.terminated, Termination::Stable));
        // lamp should turn on at tick = 1
        assert!(res.diffs.iter().any(|d| d.tick == 1
            && d.changes
                .iter()
                .any(|c| matches!(c.kind, BlockKind::Lamp { on: true }))));
    }

    #[test]
    fn dust_attenuation() {
        let world = World {
            blocks: vec![
                PlacedBlock {
                    pos: Pos { x: 0, y: 0, z: 0 },
                    kind: BlockKind::Lever { on: true, facing: Direction::East },
                },
                PlacedBlock {
                    pos: Pos { x: 1, y: 0, z: 0 },
                    kind: BlockKind::Dust { power: 0 },
                },
                PlacedBlock {
                    pos: Pos { x: 2, y: 0, z: 0 },
                    kind: BlockKind::Dust { power: 0 },
                },
            ],
        };
        let req = SimRequest { ticks: 3, world, early_exit: true };
        let res = simulate(req);
        assert!(res.diffs.iter().any(|d| d.changes.iter().any(|c| matches!(c.kind, BlockKind::Dust { power: 14 }))));
    }

    #[test]
    fn torch_turns_off_when_powered() {
        let world = World {
            blocks: vec![
                PlacedBlock {
                    pos: Pos { x: 0, y: 0, z: 0 },
                    kind: BlockKind::Lever { on: true, facing: Direction::East },
                },
                PlacedBlock {
                    pos: Pos { x: 1, y: 0, z: 0 },
                    kind: BlockKind::Torch { lit: true, facing: Direction::West },
                },
            ],
        };
        let req = SimRequest { ticks: 2, world, early_exit: true };
        let res = simulate(req);
        assert!(res.diffs.iter().any(|d| d.changes.iter().any(|c| matches!(c.kind, BlockKind::Torch { lit: false }))));
    }

    #[test]
    fn repeater_requires_back_input() {
        let world = World {
            blocks: vec![
                PlacedBlock {
                    pos: Pos { x: 1, y: 0, z: 1 },
                    kind: BlockKind::Lever { on: true, facing: Direction::North },
                },
                PlacedBlock {
                    pos: Pos { x: 1, y: 0, z: 0 },
                    kind: BlockKind::Repeater {
                        delay: 1,
                        ticks_remaining: 0,
                        powered: false,
                        facing: Direction::East,
                    },
                },
                PlacedBlock {
                    pos: Pos { x: 2, y: 0, z: 0 },
                    kind: BlockKind::Dust { power: 0 },
                },
                PlacedBlock {
                    pos: Pos { x: 3, y: 0, z: 0 },
                    kind: BlockKind::Lamp { on: false },
                },
            ],
        };
        let req = SimRequest { ticks: 3, world, early_exit: true };
        let res = simulate(req);
        assert!(!res.diffs.iter().any(|d| d.changes.iter().any(|c| matches!(c.kind, BlockKind::Lamp { on: true }))));
    }
}

pub mod py;
