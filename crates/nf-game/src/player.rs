//! Bond on foot: `Player_Update` (0x193600) with `Player_Move` (0x191dd0), `Player_HandleJump` (0x1916b0),
//! `Player_Collision` (0x194df0), `Player_CollisionHandler` (0x1949a8) and the eye of `Player_PositionCamera`.
//! Spec: docs/systems/player.md.

use crate::world::{CollisionWorld, V3};

/// `AnimObj+0xCC` [L]: the root height above the feet (07000005, skin 0x05000007).
pub const ROOT_HEIGHT: f32 = 1.0347183;
/// Capsule radius (`HITTEST+0x8C`).
pub const RADIUS: f32 = 0.55;
/// `BL+0x908` on land.
pub const EYE: f32 = 0.7;
const GRAVITY: f32 = -9.8;

/// One tick of player input as game actions: `forward` (action 2), `strafe` (action 1; + = left), `turn` (radians
/// to add to the yaw this tick, mouse or ramped pad), `look` (pitch delta in units of π/2), the jump (7) and crouch (8)
/// presses, aim mode (0x13).
///
/// Weapon switching [M Player_WeaponChange 0x1a6f80]: `weapon_cycle` = ±1 for next/prev weapon (mouse wheel / Q),
/// `gadget_cycle` = ±1 for next/prev gadget (G), `switch_to` = `Some(id)` for a direct selection (number keys).
/// The weapon/gadget split follows the game's own mode 0 (weapons 1..0x47) / mode 1 (gadgets 0x4A..0x5E).
#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    pub forward: f32,
    pub strafe: f32,
    pub turn: f32,
    pub look: f32,
    pub jump: bool,
    pub crouch: bool,
    pub aiming: bool,
    /// ±1: next/prev weapon (`Player_WeaponChange` mode=0, bounds 1..0x47).
    pub weapon_cycle: i8,
    /// ±1: next/prev gadget (`Player_WeaponChange` mode=1, bounds 0x4A..0x5E).
    pub gadget_cycle: i8,
    /// `Some(id)`: switch directly to weapon `id` (a number key).
    pub switch_to: Option<u8>,
    /// Right mouse held: zoom the scope (`Player_Zoom` 0x1a1a70, mouse-first like the game's aim button).
    pub zoom: bool,
}

#[derive(Clone, Debug)]
pub struct Bond {
    /// `obj+0x30`.
    pub pos: V3,
    /// `obj+0x54` (0 faces +Z; a larger yaw turns left).
    pub yaw: f32,
    /// `BL+0x8A8`: units of π/2, ±1, + = up.
    pub pitch: f32,
    /// `BL+0x20`: last tick's local velocity (x left, z forward) per tick.
    pub local: [f32; 2],
    /// `BL+0x50`: world velocity, units per second.
    pub vel: V3,
    /// AnimObj flag 8 and the floor's normal y (`BL+0x110`).
    pub grounded: bool,
    pub floor_ny: f32,
    /// `BL+0x940`: grounded in the last 4 ticks (shift register).
    pub grounded_history: u16,
    /// `BL+0x94C` (0 on foot, 1 rising, 2 bumped) and the landing delay `BL+0x952`.
    pub jump: u8,
    pub land_delay: u8,
    /// Substate 4 and the stand / crouch lockout `BL+0x960`.
    pub crouched: bool,
    pub lockout: u32,
    /// `BL+0x90C` (eye drop when crouched) and `BL+0x908`.
    pub crouch_drop: f32,
    pub eye: f32,
    /// `BL+0x928`: ticks falling.
    pub fall_timer: u32,
    /// AnimObj flag 0x20: something touched above.
    pub head_bump: bool,
    /// Damage from the last landing (`Player_HandlePain` type 6), for the caller to apply.
    pub fall_damage: Option<f32>,
}

impl Bond {
    /// Standing at `pos` (`Player_StandAtNewPosition`: the floor point + 1.1) facing `yaw`.
    pub fn new(pos: V3, yaw: f32) -> Bond {
        Bond { pos, yaw, pitch: 0.0, local: [0.0; 2], vel: [0.0; 3], grounded: true, floor_ny: 1.0, grounded_history: 8, jump: 0, land_delay: 0,
            crouched: false, lockout: 0, crouch_drop: 0.0, eye: EYE, fall_timer: 0, head_bump: false, fall_damage: None }
    }

    /// On the floor below `p` (within 10 units), as a level start places him.
    pub fn stand_at(world: &CollisionWorld, p: V3, yaw: f32) -> Bond {
        let y = world.floor_below([p[0], p[1] + 1.0, p[2]], 10.0).map_or(p[1], |f| f + 1.1);
        Bond::new([p[0], y, p[2]], yaw)
    }

    /// `Player_GetHeadPos`: the eye.
    pub fn head(&self) -> V3 { [self.pos[0], self.pos[1] + self.eye - self.crouch_drop, self.pos[2]] }

    /// The player's world-sphere centre (`obj+0x80`, `Control_BuildWorldSph` 0x133070): the local sphere
    /// centre at `M_CONTROL+0x80` through the object matrix — `world = x·row0 + y·row1 + z·row2 + row3`,
    /// i.e. the yaw-rotated local offset added to the position [M 0x133070:33-44; the player takes the
    /// `obj+0xFA & 8` branch which reads M_CONTROL (obj+0xDC), radius `M_CONTROL+0x8C · |obj+0xE8|`].
    /// `M_CONTROL+0x80 = (−0.1347, −0.1267, 0.2402)` local, radius 1.0576 at the 07000005 start `[L]
    /// ee_start_05.bin M_CONTROL 0x16cb8a0+0x80/0x8C`; the computed centre (104.826, 24.837, 108.582)
    /// matches the player object's `+0x80` there `[L] 0x16cefe0+0x80`.
    pub fn sphere_centre(&self) -> V3 {
        // [L] M_CONTROL+0x80 (ee_start_05.bin, player 0 standing).
        const LOCAL: V3 = [-0.1347, -0.1267, 0.2402];
        let (s, c) = self.yaw.sin_cos();
        // The object matrix rows [cos,0,−sin / 0,1,0 / sin,0,cos] (+ translation = pos), as read live
        // [L 0x16cefe0+0x90..0xc8] and matching `Bond::look`'s convention.
        [self.pos[0] + LOCAL[0] * c + LOCAL[2] * s, self.pos[1] + LOCAL[1], self.pos[2] - LOCAL[0] * s + LOCAL[2] * c]
    }

    /// The view direction (pitch applied to the forward axis).
    pub fn look(&self) -> V3 {
        let p = self.pitch * std::f32::consts::FRAC_PI_2;
        [p.cos() * self.yaw.sin(), p.sin(), p.cos() * self.yaw.cos()]
    }

    /// The capsule ends (top A, bottom B) for the current stance.
    fn capsule(&self) -> (V3, V3) {
        let (a, b) = if self.crouched { (0.65 - ROOT_HEIGHT, 0.45 - ROOT_HEIGHT) } else { (0.275, 0.55 - ROOT_HEIGHT) };
        ([self.pos[0], self.pos[1] + a, self.pos[2]], [self.pos[0], self.pos[1] + b, self.pos[2]])
    }

    /// One 60 Hz tick.
    pub fn tick(&mut self, world: &CollisionWorld, input: &Input) {
        let prev = self.pos;
        self.fall_damage = None;
        // `Player_SSWalk` / `Player_SSCrouch`: the lockout runs down (*deviation*: the transition animation that may
        // end it early is not played).
        self.lockout = self.lockout.saturating_sub(1);
        let k = if self.crouched { 0.75 } else { 1.0 };
        // `Player_Move`: aim mode zeroes forward (and the pad's turn action, which the caller leaves out of `turn`:
        // with a mouse the view turns past the aim box's edge, D-002).
        let (fwd, turn) = if input.aiming { (0.0, input.turn) } else { (input.forward.clamp(-1.0, 1.0), input.turn) };
        let strafe = input.strafe.clamp(-1.0, 1.0);
        let target_f = if fwd > 0.0 { 0.1 * fwd * fwd + 0.005 } else if fwd < 0.0 { -0.075 * fwd * fwd - 0.005 } else { 0.0 };
        let target_s = 0.075 * strafe;
        let axis = |last: f32, target: f32| -> f32 {
            let mut v = last + (target - last) * 0.2;
            if (target - last) * (target - v) < 0.0 { v = target; }
            if target.abs() <= 0.0002 && v.abs() < 0.01 { v = 0.0; }
            v.clamp(-0.1, 0.1)
        };
        let (vs, vf) = (axis(self.local[0], target_s) * k, axis(self.local[1], target_f) * k);
        // `Player_HandleJump`: crouch (toggle) and jump.
        let headroom = |w: &CollisionWorld| w.line_of_sight(self.pos, [prev[0], prev[1] + 0.95, prev[2]]);
        if !self.crouched && self.jump == 0 && input.crouch && self.lockout == 0 {
            self.crouched = true;
            self.lockout = 60;
        } else if self.crouched && (input.crouch || input.jump) && self.lockout == 0 && !self.head_bump && headroom(world) {
            self.crouched = false;
            self.lockout = 60;
        } else if !self.crouched && self.jump == 0 && input.jump && self.grounded_history != 0 && !self.head_bump && headroom(world) {
            self.vel = [0.0, -GRAVITY * 0.4, 0.0];
            self.jump = 1;
            self.land_delay = 4;
            self.grounded = false;
        }
        // The matrix turns first, then the position moves along the new axes.
        self.yaw += turn;
        self.pitch = (self.pitch + input.look).clamp(-1.0, 1.0);
        let (s, c) = (self.yaw.sin(), self.yaw.cos());
        self.pos[0] += c * vs + s * vf;
        self.pos[2] += -s * vs + c * vf;
        self.local = [vs / k, vf / k];
        // `Player_Collision`: gravity unless standing on a gentle floor (jumping keeps the velocity).
        if !self.grounded || self.floor_ny < 0.5 {
            self.vel[1] += GRAVITY / 60.0;
        } else if self.jump != 1 {
            self.vel = [0.0; 3];
        }
        self.vel[1] = self.vel[1].clamp(-45.0, 45.0);
        for k in 0..3 { self.pos[k] += self.vel[k] / 60.0; }
        // `Player_CollisionHandler`: the floor ray first, then the capsule's push.
        let (a, b) = self.capsule();
        let (push, walkable) = world.capsule_push(a, b, RADIUS);
        let ray = ROOT_HEIGHT + 0.6;
        let floor = world.segment_hit(self.pos, [self.pos[0], self.pos[1] - ray, self.pos[2]]);
        self.grounded = walkable || floor.is_some_and(|(t, _)| t * ray - 0.1 <= ROOT_HEIGHT);
        if let Some((_, n)) = floor { self.floor_ny = n[1]; } else if walkable { self.floor_ny = 1.0; }
        self.head_bump = push[1] < -1e-4;
        for k in 0..3 { self.pos[k] += push[k]; }
        self.grounded_history = (self.grounded_history >> 1) | if self.grounded { 8 } else { 0 };
        // Fall damage: ticks falling with no ground for 4 ticks; on landing, over 60 → (timer − 60) if ≥ 10.
        if self.grounded_history == 0 && self.vel[1] < 0.0 { self.fall_timer += 1; }
        else if self.grounded {
            if self.fall_timer > 60 && self.fall_timer - 60 >= 10 { self.fall_damage = Some((self.fall_timer - 60) as f32); }
            self.fall_timer = 0;
        }
        // Jump: a head bump ends the rise; landing after the delay.
        if self.jump == 1 && self.head_bump { self.vel = [0.0; 3]; self.jump = 2; }
        self.land_delay = self.land_delay.saturating_sub(1);
        if self.jump != 0 && self.land_delay == 0 && self.grounded && self.vel[1] <= 0.0 { self.vel = [0.0; 3]; self.jump = 0; }
        if self.grounded && self.jump == 0 { self.vel[1] = 0.0; }
        // `Player_PositionCamera`: the crouch drop and the eye ease in.
        self.crouch_drop = if self.crouched { (self.crouch_drop + 0.0225).min(0.45) } else { (self.crouch_drop - 0.0225).max(0.0) };
        self.eye += (EYE - self.eye) * 0.1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Bond::sphere_centre` vs the live 07000005 start [L ee_start_05.bin]: player obj 0x16cefe0, pos
    /// (104.954, 24.963, 108.825), yaw from the matrix (cos −0.5446, sin 0.8387 → forward (0.8387, 0,
    /// −0.5446)), `obj+0x80` = (104.826, 24.837, 108.582). `Control_BuildWorldSph` 0x133070 takes the
    /// centre from `M_CONTROL+0x80` through the object matrix [M].
    #[test]
    fn sphere_centre_matches_the_game_snapshot() {
        // yaw: sin = −0.8387, cos = −0.5446 — forward = Bond::look() = (sin, 0, cos) = matrix row 2 [L]
        // (rows [cos,0,−sin / 0,1,0 / sin,0,cos] read at 0x16cefe0+0x90..0xc8).
        let yaw = (-0.8387f32).atan2(-0.5446);
        let b = Bond::new([104.954, 24.963, 108.825], yaw);
        assert!((b.look()[0] - (-0.8387)).abs() < 1e-3 && (b.look()[2] - (-0.5446)).abs() < 1e-3, "forward matches the matrix row 2");
        let c = b.sphere_centre();
        for (k, want) in [(0, 104.826), (1, 24.837), (2, 108.582)] {
            assert!((c[k] - want).abs() < 2e-3, "axis {k}: got {} want {want}", c[k]);
        }
        // Sister: a different yaw moves the centre (the offset rotates with the body).
        let b2 = Bond::new([104.954, 24.963, 108.825], yaw + std::f32::consts::FRAC_PI_2);
        assert!((b2.sphere_centre()[0] - c[0]).abs() > 0.05, "the centre rotates with yaw");
    }

    fn room() -> CollisionWorld {
        let q = |a: V3, b: V3, c: V3, d: V3| [[a, b, c], [a, c, d]];
        let mut t = Vec::new();
        t.extend(q([-20.0, 0.0, -20.0], [20.0, 0.0, -20.0], [20.0, 0.0, 20.0], [-20.0, 0.0, 20.0]));
        // A wall at z = 5 and a 0.3 step at x ≥ 3 (top at y 0.3).
        t.extend(q([-20.0, 0.0, 5.0], [20.0, 0.0, 5.0], [20.0, 4.0, 5.0], [-20.0, 4.0, 5.0]));
        t.extend(q([3.0, 0.3, -20.0], [20.0, 0.3, -20.0], [20.0, 0.3, 4.0], [3.0, 0.3, 4.0]));
        t.extend(q([3.0, 0.0, -20.0], [3.0, 0.3, -20.0], [3.0, 0.3, 4.0], [3.0, 0.0, 4.0]));
        CollisionWorld::from_triangles(t)
    }

    #[test]
    fn walks_at_the_game_speed_stops_at_walls_and_climbs_steps() {
        let w = room();
        let mut b = Bond::stand_at(&w, [0.0, 0.0, 0.0], 0.0);
        let fwd = Input { forward: 1.0, ..Default::default() };
        for _ in 0..30 { b.tick(&w, &fwd); }
        // 0.1 a² + 0.005 = 0.105, clamped to 0.1 per tick once accelerated.
        assert!((b.local[1] - 0.1).abs() < 1e-6, "{:?}", b.local);
        for _ in 0..120 { b.tick(&w, &fwd); }
        assert!(b.pos[2] < 5.0 - RADIUS + 0.01 && b.pos[2] > 4.0, "held by the wall: {:?}", b.pos);
        assert!(b.grounded && (b.pos[1] - (ROOT_HEIGHT + 0.05)).abs() < 0.07, "standing: {:?}", b.pos);
        // Left (+x at yaw 0) onto the step: the capsule rides up.
        let left = Input { strafe: 1.0, ..Default::default() };
        let mut b = Bond::stand_at(&w, [0.0, 0.0, 0.0], 0.0);
        for _ in 0..120 { b.tick(&w, &left); }
        assert!(b.pos[0] > 4.0 && b.pos[1] > 0.3 + ROOT_HEIGHT - 0.05, "on the step: {:?}", b.pos);
        // Sister: stopping takes ~11 ticks, not one.
        for _ in 0..5 { b.tick(&w, &Input::default()); }
        assert!(b.local[0] != 0.0);
    }

    #[test]
    fn jumps_and_falls_with_gravity() {
        let w = room();
        let mut b = Bond::stand_at(&w, [0.0, 0.0, 0.0], 0.0);
        let y0 = b.pos[1];
        b.tick(&w, &Input { jump: true, ..Default::default() });
        let mut top = y0;
        for _ in 0..60 { b.tick(&w, &Input::default()); top = top.max(b.pos[1]); }
        // 3.92 m/s up against 9.8: ~0.78 high.
        assert!((top - y0 - 0.78).abs() < 0.08, "apex {}", top - y0);
        assert!(b.jump == 0 && (b.pos[1] - y0).abs() < 0.1);
        // Off a ledge: no floor → falls.
        let mut b = Bond::new([0.0, 30.0, 0.0], 0.0);
        for _ in 0..30 { b.tick(&w, &Input::default()); }
        assert!(b.pos[1] < 30.0 - 1.0 && !b.grounded);
    }
}
