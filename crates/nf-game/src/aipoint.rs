//! AI points (placements 0xE9, `Drone_AIPoint` 0x137550, 162 placements): the run-to points the
//! guards and scared civilians route to. Each record embeds an `AIEmitter` (+0x50) that
//! `Drone_PostLoad_Init` 0x136938:78-86 turns into per-node walk costs over the point's
//! navigation path (`AINetwork_EmitPath` 0x158cf8 — the port's `Network::emit`); the drones then
//! ask `NDrone2_ClosestAIPoint` (0x151b18) for the cheapest point of a group on their own path.
//! Spec: docs/systems/script-census.md "AI waypoints (0xE9)".

use crate::nav::Network;
use crate::world::{CollisionWorld, V3};
use nf_formats::placement::Placement;

#[derive(Clone, Debug)]
pub struct AiPoint {
    /// `+0x06`: the group the point serves (parameter 0, u16). `NDrone2_FindRunToPoint` 0x153168
    /// asks 8 (prop 0x56) / 1 (prop 0x4e) / class 1→0, 2→9, 3→2; `NDrone2_FindAlarmPoint`
    /// 0x153008 and `NDrone2_AssignAIPath` 0x154190:55-66 ask 8 (alarm points).
    pub group: u16,
    /// `+0x0c`: parameter 2 — matched against the drone's +0x144 (placement parameter 0x11,
    /// `DefaultInit` 0x14b300:472) with first-match preference [M 0x151b18:151-181].
    pub owner: i32,
    /// `+0x14`: parameter 1 as a float — the goal accuracy handed to `AINetwork_SetupGoalPosition`
    /// 0x15acf8 (0 there means 2.0). `Drone_AIPoint` converts the word with `cvt.s.w` when ≥ 0;
    /// negative words take the `srl/andi/or` form ×2.0 (0x1375d4-0x137614) — none occur on the
    /// disc's 162 records [D-census].
    pub accuracy: f32,
    /// `+0x20`: the goal position — the placement translation [M 0x137550 disasm `lq a2,0(s1)` /
    /// `sq a2,0x20(v0)`].
    pub pos: V3,
    /// `+0x44`: the placement's y-rotation raw bits (`NDrone2_FindAlarmPoint` stores them at
    /// Drone+0x734 [M 0x153008:24]; the facing the game keys off the placement's rotation).
    pub yaw_bits: u32,
    /// The emitter's navigation path and its Dijkstra distance bytes per node (`+0x80`/`+0x88`,
    /// `AINetwork_InitEmitter` 0x15b1e8 binds `NavPathForPosition(pos)` at creation, the emit runs
    /// at `Drone_PostLoad_Init`).
    pub path: Option<usize>,
    pub dist: Vec<u8>,
}

impl AiPoint {
    /// From a placement whose flags word is 0xe9 (`parsemap_create_dynamic_objects` 0x1d04f0 case
    /// 0xe9 → `Drone_AIPoint`; the level_tag fields are the placement's parameters, the mapping
    /// `+0x2c+4k` ↔ parameter k that `Drone_AIVolume_Create` 0x1376c0 / `Trigger_Create` 0x1b1720
    /// pin: +0x2c = param 0, +0x30 = param 1, +0x34 = param 2).
    pub fn from_placement(p: &Placement) -> AiPoint {
        let param = |k: u32| p.params.iter().find(|(i, _)| *i == k).map(|&(_, v)| v).unwrap_or(0);
        let raw = param(1);
        // 0x1375d4-0x137614: bltz rec+0x30 → mtc1/cvt.s.w; else srl 1, andi 1, or, cvt.s.w, add.s ×2.
        let accuracy = if (raw as i32) < 0 {
            nf_formats::ps2float::mul(((raw >> 1) | (raw & 1)) as f32, 2.0)
        } else {
            raw as f32
        };
        AiPoint { group: param(0) as u16, owner: param(2) as i32, accuracy, pos: p.translation, yaw_bits: p.rotation[1].to_bits(), path: None, dist: Vec::new() }
    }
}

/// Every AI point of the level's main map, with its emitter (the navigation path for its position
/// and the distance bytes, exactly the cover-node build). `Drone_PreLoad_Init` 0x136888 resets the
/// game's table per level; `Drone_AIPoint` fills one record per 0xE9 placement.
pub fn build(placements: &[Placement], nav: &Network, world: &CollisionWorld) -> Vec<AiPoint> {
    placements.iter().filter(|p| p.flags == 0xe9).map(|p| {
        let mut n = AiPoint::from_placement(p);
        n.path = nav.nav_path_for(n.pos);
        if let Some(pi) = n.path { n.dist = nav.emit(world, pi, n.pos); }
        n
    }).collect()
}

/// `NDrone2_ClosestAIPoint` (0x151b18): the cheapest eligible point of `group`. A point qualifies
/// when its emitter path equals the drone's nav path (`+0x80` vs Drone+0x954, set at level load to
/// `NavPathForPosition(drone pos)` [M 0x14d890:18-22]) and its cost byte at the drone's nearest
/// node on that path (`NDrone2_NearestNodeDCV` 0x155958 → the port's `Network::node_from`) is not
/// 0xff [M 0x151b18:128-149]. Among the eligible ones the least cost wins, first on ties; once a
/// point whose `owner` (+0x0c) matches the drone's +0x144 is the incumbent, non-matching points
/// never replace it [M 0x151b18:151-181]. Returns the index into `points`.
pub fn closest(points: &[AiPoint], net: &Network, world: &CollisionWorld, drone_path: Option<usize>, drone_pos: V3, group: u16, owner: i32) -> Option<usize> {
    let node = drone_path.and_then(|p| net.node_from(world, p, drone_pos)).map(|n| n as usize);
    let mut best: Option<(f32, usize, bool)> = None; // (cost, index, owner matches the drone)
    for (i, p) in points.iter().enumerate() {
        if p.group != group { continue; } // [+6] == asked group [M 0x151b18:129]
        let Some(path) = p.path else { continue };
        if Some(path) != drone_path || p.dist.is_empty() { continue; } // [+0x80] vs +0x954, [+0x88] != 0
        let Some(node) = node else { continue };
        let c = p.dist[node];
        if c == 0xff { continue; } // [M 0x151b18:139-144]
        let c = c as f32;
        let matches = p.owner != 0 && p.owner == owner;
        match best {
            None => best = Some((c, i, matches)),
            Some((bc, _, have)) if c < bc => {
                if have && !matches { continue; } // a matching incumbent blocks generics [M 0x151b18:170-176]
                best = Some((c, i, matches));
            }
            _ => {}
        }
    }
    best.map(|(_, i, _)| i)
}

/// `NDrone2_FindRunToPoint` (0x153168) up to the point lookup: the asked group — property 0x56 → 8,
/// else property 0x4e → 1, else the class byte +0x44 (1 → 0, 2 → 9, 3 → 2, others find nothing).
pub fn run_to_point_group(prop56: bool, prop4e: bool, team: u8) -> Option<u16> {
    if prop56 { Some(8) } else if prop4e { Some(1) } else {
        match team { 1 => Some(0), 2 => Some(9), 3 => Some(2), _ => None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place(flags: u32, params: &[(u32, u32)], t: [f32; 3]) -> Placement {
        Placement { index: 0, hashcode: u32::MAX, flags, translation: t, rotation: [0.0, 1.25, 0.0], params: params.to_vec(), extra: [0; 9] }
    }

    #[test]
    fn record_reads_its_params_the_aivolume_way() {
        // Drone_AIPoint 0x137550 reads rec+0x2c (u16), +0x30 (word), +0x34 (word) — the placement
        // parameters 0/1/2 (the level_tag mapping Drone_AIVolume_Create 0x1376c0 pins).
        let p = place(0xe9, &[(0, 2), (1, 1), (2, 7)], [1.0, 2.0, 3.0]);
        let n = AiPoint::from_placement(&p);
        assert_eq!((n.group, n.owner, n.accuracy), (2, 7, 1.0));
        assert_eq!(n.pos, [1.0, 2.0, 3.0]);
        // +0x44 = the placement's y-rotation raw bits [M 0x153008:24].
        assert_eq!(n.yaw_bits, 1.25f32.to_bits());
        // Missing parameters default to 0 (the game's parsed level_tag slots) [S: the slot model].
        let n = AiPoint::from_placement(&place(0xe9, &[], [0.0; 3]));
        assert_eq!((n.group, n.owner, n.accuracy), (0, 0, 0.0));
        // The negative-word path: (float)((x >> 1) | (x & 1)) * 2.0 [M 0x1375f4-0x137614].
        let n = AiPoint::from_placement(&place(0xe9, &[(1, 0xffff_fffdu32)], [0.0; 3]));
        assert_eq!(n.accuracy, (((0xffff_fffdu32 >> 1) | 1) as f32) * 2.0);
    }

    #[test]
    fn run_to_point_group_by_prop_and_class() {
        // [M 0x153168:8-30]
        assert_eq!(run_to_point_group(true, false, 1), Some(8));
        assert_eq!(run_to_point_group(false, true, 1), Some(1));
        assert_eq!(run_to_point_group(false, false, 1), Some(0));
        assert_eq!(run_to_point_group(false, false, 2), Some(9));
        assert_eq!(run_to_point_group(false, false, 3), Some(2));
        assert_eq!(run_to_point_group(false, false, 0), None);
    }
}
