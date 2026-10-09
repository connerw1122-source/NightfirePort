//! Weather: snow / rain drops (`Env_Create` 0x17b968, `InitDrops` 0x18bf68, `Env_Update` 0x17bc88 → `UpdateDrops`
//! 0x18c580, `AddDrop` 0x18c278, `RainBox_Clip` 0x18d0d0). Spec: docs/systems/render.md "Snow".
//!
//! The drops use the shared `Rand_*` generator every tick (the wind, each respawn), and `Env_Update` draws one
//! `Rand_Rand(20000)` first; `Game_Run` calls it before the object handler, so `Drones::tick` runs it first.

use crate::drone::Rng;
use crate::world::V3;

/// A snow / rain volume (placement type 0x32 `Effect_Snow_*`, `RainBox_Create` 0x18d840): the glist bounds plus the
/// object position.
#[derive(Clone, Copy, Debug)]
pub struct RainBox { pub min: V3, pub max: V3 }

#[derive(Clone, Debug)]
pub struct Drops {
    /// `DropType`: 0 snow, 1 rain, 2 rising.
    pub kind: u8,
    /// `RandTable1`: 32 velocities per tick.
    pub vel: [V3; 32],
    /// `DropVector` (x, y, z, kill height w) and `DropState` (1 = falling).
    pub drops: Vec<[f32; 4]>,
    pub alive: Vec<bool>,
    /// `WindVector` (x, z).
    pub wind: [f32; 2],
    pub boxes: Vec<RainBox>,
    /// Clipped boxes of this tick and whether each is active.
    clipped: Vec<(RainBox, bool)>,
    /// `InitBodge` (07000005–06): the first update fills the cube.
    pub init_bodge: bool,
    /// `CubeCentre`.
    pub centre: V3,
}

impl Drops {
    /// `InitDrops(kind)`: the velocity table (x, z ∈ [−0.02, 0.02], y ∈ [−0.1, −0.025]; ±0.002 on 07000007–08;
    /// ×0.25 and upward for kind 2), 4096 drops for snow (2048 otherwise, rain 128 / 64). `rng`: the generator the
    /// table comes from — the game's runs before the level-start snapshot the port starts from, so the port takes it
    /// from a separate one (*deviation*: the table's values are not the game's).
    pub fn new(kind: u8, boxes: Vec<RainBox>, level: u32, rng: &mut Rng) -> Drops {
        let narrow = matches!(level, 0x0700_0007 | 0x0700_0008);
        let mut vel = [[0f32; 3]; 32];
        for v in &mut vel {
            let (x, y, z) = if narrow {
                (rng.frand(0.004) - 0.002, -(rng.frand(0.075) + 0.025), rng.frand(0.004) - 0.002)
            } else {
                (rng.frand(0.04) - 0.02, -(rng.frand(0.075) + 0.025), rng.frand(0.04) - 0.02)
            };
            *v = if kind == 2 { [x * 0.25, y * -0.25, z * 0.25] } else { [x, y, z] };
        }
        let max = match kind { 0 => 0x1000, 1 => if matches!(level.wrapping_sub(0x0700_0009), 0..=2) { 0x80 } else { 0x40 }, _ => 0x800 };
        Drops { kind, vel, drops: vec![[0.0, 0.0, -1024.0, 0.0]; max], alive: vec![false; max], wind: [0.0; 2], boxes, clipped: Vec::new(),
            init_bodge: matches!(level, 0x0700_0005 | 0x0700_0006), centre: [0.0; 3] }
    }

    /// `UpdateDrops` for the viewer at `eye` with the player's `yaw` (the cube centre: the eye 7.5 ahead).
    /// `fill_rng`: the generator for the first-frame fill (*deviation*: the game's fill precedes the snapshot).
    pub fn update(&mut self, rng: &mut Rng, fill_rng: &mut Rng, eye: V3, yaw: f32) {
        if self.init_bodge {
            // The whole cube at once, kill height eye − 7.5.
            self.init_bodge = false;
            for i in 0..self.drops.len() {
                self.alive[i] = true;
                let x = eye[0] + (fill_rng.frand(30.0) - 15.0);
                let z = eye[2] + (fill_rng.frand(30.0) - 15.0);
                let y = eye[1] + (fill_rng.frand(30.0) - 15.0);
                self.drops[i] = [x, y, z, eye[1] - 7.5];
            }
            return;
        }
        let mut added = 0;
        if self.kind != 1 {
            self.wind[0] = (self.wind[0] + (rng.frand(0.0004) - 0.0002)).clamp(-0.01, 0.01);
            self.wind[1] = (self.wind[1] + (rng.frand(0.0004) - 0.0002)).clamp(-0.01, 0.01);
        }
        self.centre = [eye[0] + crate::trig::sinf(yaw) * 7.5, eye[1], eye[2] + crate::trig::cosf(yaw) * 7.5];
        // `RainBox_Clip`: each box clipped to the cube (± 15); active when its x and z ranges are not empty.
        let c = self.centre;
        self.clipped = self.boxes.iter().map(|b| {
            let lo = [b.min[0].max(c[0] - 15.0), b.min[1].max(c[1] - 15.0), b.min[2].max(c[2] - 15.0)];
            let hi = [b.max[0].min(c[0] + 15.0), b.max[1].min(c[1] + 15.0), b.max[2].min(c[2] + 15.0)];
            (RainBox { min: lo, max: hi }, hi[0] > lo[0] && hi[2] > lo[2])
        }).collect();
        if !self.clipped.iter().any(|b| b.1) { return; }
        // `RandTable2` = velocity + wind (y 0 for snow).
        let step: Vec<V3> = self.vel.iter().map(|v| [v[0] + self.wind[0], v[1], v[2] + self.wind[1]]).collect();
        for i in 0..self.drops.len() {
            if self.alive[i] {
                let s = step[i & 31];
                let d = &mut self.drops[i];
                for k in 0..3 { d[k] += s[k]; }
                let dead = if self.kind == 2 { d[3] < d[1] } else { d[1] < d[3] };
                if dead { self.alive[i] = false; d[1] -= 1024.0; }
            } else if added < 0x15 {
                added += 1;
                self.add(rng, i);
            }
        }
    }

    /// `AddDrop`: x, z within the cube, y above the centre (7.5 + rand 7.5; rising drops anywhere in ±15); accepted only
    /// inside an active box (above its floor), the kill height max(box floor, centre − 7.5); else it stays dead.
    fn add(&mut self, rng: &mut Rng, i: usize) {
        let c = self.centre;
        let mut p = [c[0], c[1], c[2], 0.0];
        p[0] += rng.frand(30.0) - 15.0;
        p[2] += rng.frand(30.0) - 15.0;
        p[1] += if self.kind == 2 { rng.frand(30.0) - 15.0 } else { rng.frand(7.5) + 7.5 };
        for (b, active) in &self.clipped {
            if !*active || !(b.min[0] < p[0] && p[0] < b.max[0] && b.min[2] < p[2] && p[2] < b.max[2]) { continue; }
            if self.kind == 2 {
                p[1] = p[1].max(b.min[1]);
                p[3] = b.max[1].min(c[1] + 7.5);
            } else {
                if !(b.min[1] < p[1]) { continue; }
                p[3] = b.min[1].max(c[1] - 7.5);
            }
            self.drops[i] = p;
            self.alive[i] = true;
            return;
        }
        p[1] -= 1054.0;
        self.drops[i] = p;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snow_falls_inside_its_box_and_respawns_above_the_viewer() {
        let mut rng = Rng::new(1);
        let mut fill = Rng::new(2);
        let boxes = vec![RainBox { min: [-100.0, -20.0, -100.0], max: [100.0, 80.0, 100.0] }];
        let mut d = Drops::new(0, boxes, 0x0700_0005, &mut rng);
        assert_eq!(d.drops.len(), 4096);
        let eye = [0.0, 10.0, 0.0];
        d.update(&mut rng, &mut fill, eye, 0.0);
        assert!(d.alive.iter().all(|&a| a), "first frame: the cube is full");
        for _ in 0..600 { d.update(&mut rng, &mut fill, eye, 0.0); }
        // Every live flake is near the cube around the centre (eye + 7.5 ahead; flakes drift up to 0.02 + wind per tick
        // and are only killed by height) and above its kill height.
        for (p, &a) in d.drops.iter().zip(&d.alive) {
            if !a { continue; }
            assert!((p[0] - d.centre[0]).abs() < 40.0 && (p[2] - d.centre[2]).abs() < 40.0, "{p:?}");
            assert!(p[1] >= p[3] - 0.2, "{p:?}");
        }
        assert!(d.alive.iter().filter(|&&a| a).count() > 1000);
        // Sister: no box → nothing moves or respawns.
        let mut e = Drops::new(0, Vec::new(), 0x0700_0001, &mut rng);
        e.update(&mut rng, &mut fill, eye, 0.0);
        assert!(e.alive.iter().all(|&a| !a));
    }
}
