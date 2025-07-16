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

// -------------------------------------------------
// Block kinds & internal state
// -------------------------------------------------
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum BlockKind {
    Lever {
        on: bool,
    },
    Button {
        ticks_remaining: u8,
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
    },
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

    // Pre‑compute 6‑direction offsets (Manhattan adjacency)
    const DIRS: [(i32, i32, i32); 6] = [
        (1, 0, 0),
        (-1, 0, 0),
        (0, 1, 0),
        (0, -1, 0),
        (0, 0, 1),
        (0, 0, -1),
    ];

    for tick in 1..=request.ticks {
        let mut changes: Vec<BlockChange> = Vec::new();

        // 1️⃣ update internal timers
        for (pos, block) in world.iter_mut() {
            match block {
                BlockKind::Button { ticks_remaining } => {
                    if *ticks_remaining > 0 {
                        *ticks_remaining -= 1;
                        changes.push(BlockChange {
                            pos: *pos,
                            kind: block.clone(),
                        });
                    }
                }
                BlockKind::Repeater {
                    delay: _,
                    ticks_remaining,
                    powered,
                } => {
                    if *ticks_remaining > 0 {
                        *ticks_remaining -= 1;
                        if *ticks_remaining == 0 {
                            *powered = true; // output now powered
                        }
                        changes.push(BlockChange {
                            pos: *pos,
                            kind: block.clone(),
                        });
                    }
                }
                _ => {}
            }
        }

        // 2️⃣ gather currently powered outputs
        let mut powered_positions: HashSet<Pos> = HashSet::new();
        for (pos, block) in &world {
            match block {
                BlockKind::Lever { on: true } => {
                    powered_positions.insert(*pos);
                }
                BlockKind::Button { ticks_remaining } if *ticks_remaining > 0 => {
                    powered_positions.insert(*pos);
                }
                BlockKind::Repeater { powered: true, .. } => {
                    powered_positions.insert(*pos);
                }
                _ => {}
            }
        }

        // 3️⃣ propagate power to adjacent dust & lamp
        for src in powered_positions.iter() {
            for (dx, dy, dz) in DIRS {
                let p = Pos {
                    x: src.x + dx,
                    y: src.y + dy,
                    z: src.z + dz,
                };
                if let Some(block) = world.get_mut(&p) {
                    match block {
                        BlockKind::Dust { power } if *power == 0 => {
                            *power = 15;
                            changes.push(BlockChange {
                                pos: p,
                                kind: block.clone(),
                            });
                        }
                        BlockKind::Lamp { on } if !*on => {
                            *on = true;
                            changes.push(BlockChange {
                                pos: p,
                                kind: block.clone(),
                            });
                        }
                        _ => {}
                    }
                }
            }
        }

        // 4️⃣ diff collection & termination check
        if !changes.is_empty() {
            diffs.push(TickDiff { tick, changes });
        } else if request.early_exit {
            // no visible changes —> check if any timer still running
            let timers_active = world.values().any(|b| match b {
                BlockKind::Button { ticks_remaining } if *ticks_remaining > 0 => true,
                BlockKind::Repeater {
                    ticks_remaining, ..
                } if *ticks_remaining > 0 => true,
                _ => false,
            });
            if !timers_active {
                return SimResponse {
                    diffs,
                    terminated: Termination::Stable,
                };
            }
        }
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
                    kind: BlockKind::Lever { on: true },
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
}

pub mod py;
