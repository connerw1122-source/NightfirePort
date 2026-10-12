//! Bond on foot: `Player_Update` (0x193600) with `Player_Move` (0x191dd0), `Player_HandleJump` (0x1916b0),
//! `Player_Collision` (0x194df0), `Player_CollisionHandler` (0x1949a8) and the eye of `Player_PositionCamera`.
//! Spec: docs/systems/player.md.
//!
//! Level carry-over and spawn (fix-level2): `Player_RamSave` (0x195670) / `Player_RamLoad` (0x1957a8),
//! the `player_start` table (`Player_AddNewStartPos` 0x195c00, `Player_Start` 0x190fd0) and the per-level
//! loadout switch of `Player_InitWeapon` (0x1a7ef8) with `Player_Weapon`'s party weapon lock (0x1a24c8).

use crate::ladder::Ladder;
use crate::triggers::Channels;
use crate::world::{CollisionWorld, V3};

/// `AnimObj+0xCC` [L]: the root height above the feet (07000005, skin 0x05000007).
pub const ROOT_HEIGHT: f32 = 1.0347183;
/// Capsule radius (`HITTEST+0x8C`).
pub const RADIUS: f32 = 0.55;
/// `BL+0x908` on land.
pub const EYE: f32 = 0.7;
const GRAVITY: f32 = -9.8;

/// `obj+0xF6` substates, from the `Player_Update` 0x193600 dispatch [M switch @0x193958; the jump table
/// reads obj+0xF6 and its cases call Player_SSWalk 0x192310 (0), Player_Climb 0x196310 (1),
/// Player_SSGrapple 0x192860 (2), the inline swim (3), Player_SSCrouch 0x1924c0 (4), Player_SSScanMode
/// 0x193098 (5), Player_Wire 0x197658 (6), Player_Creep 0x1969d0 (7), Player_ZeroG 0x192ab0 (8/9), the
/// scripted-cam countdown (10), Player_Zipline 0x197f50 (0xF)]. This session ports 1 and 3.
pub const SS_WALK: u8 = 0;
pub const SS_CLIMB: u8 = 1;
pub const SS_SWIM: u8 = 3;

/// The per-tick ladder/water context the Sim hands `Bond::tick_ctx` (the game reads the ladder through
/// `BLData+0x884` and the water surface through the player's cel, `obj+0x20 → cel+0x90`).
pub struct LadderCtx<'a> {
    /// The level's ladder objects (class 0x2D), in placement order.
    pub ladders: &'a [Ladder],
    /// `cel+0x90` for the cel Bond is in (`Player_InWater` 0x195e50 reads `*(obj+0x20)+0x90`;
    /// `build_alloc_cel` 0x1c93f8 defaults it to −500.0 = no water; `parseentity_fixup_entity` 0x1d00c0
    /// sets it to the entity's bbox max y for flag word 0xC023). `None` = no water cel / below −400.
    pub water: Option<f32>,
    /// Whether Bond holds the descend control this tick [S — the game's vertical swim input is not
    /// resolved in this pass; see docs/atlas/ladder.md "Not verified"].
    pub sink: bool,
}

impl Default for LadderCtx<'_> {
    fn default() -> Self { LadderCtx { ladders: &[], water: None, sink: false } }
}

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
    /// `obj+0xF6`: the substate id ([M Player_Update 0x193600 dispatch]; 0 walk, 1 climb, 3 swim here).
    pub substate: u8,
    /// `BL+0x884`: the ladder being climbed (an index into `LadderCtx::ladders`).
    pub ladder: Option<usize>,
    /// `BL+0x8A0` / `BL+0x8A4`: the climb's top/bottom dismount thresholds
    /// (`centre.y + radius + 0.85` / `centre.y − radius + 1.0` [M Player_CollLadder 0x196118,
    /// recomputed per tick in Player_Climb 0x196310]).
    pub climb_top: f32,
    pub climb_bottom: f32,
    /// `BL+0x951`: the regrab lockout in ticks (`Player_Climb` sets `(u8)FRAME_RATE` = 60 on every
    /// dismount; `Player_Update` 0x193840 decrements it; `Player_CollLadder` refuses grabs while set) [M].
    pub regrab: u8,
    /// `BL+0x94D`: the move-mode byte (1 = climbing) [M CollLadder sets 1 on grab].
    pub mode: u8,
    /// `BL+0x942`: the saved substate (the dispatch's case 10 restores it; the wire/creep/climb drops
    /// write it) [M]. The climb writes its own id, matching the game.
    pub saved_sub: u16,
    /// `BL+0x8C0`: the air meter, 0..100 (`Player_MonitorAir` 0x191a70: −5/s with the head below the
    /// surface, +20/s above, drown damage 0.5 (type 7) every 30 ticks below 1.0) [M].
    pub air: f32,
    /// Drown damage from this tick's `Player_MonitorAir`, for the caller to apply.
    pub drown_damage: Option<f32>,
    /// True while the head is below the water surface (`Player_MonitorAir`'s +0x8C8 bit 0x10 path).
    pub underwater: bool,
    /// `BL+0x8C4`: the underwater fade 0..1 (`Player_MonitorAir`: →1 under, →0 surfaced), for the renderer.
    pub water_fade: f32,
    /// The drown-damage tick accumulator (the game uses `GameState % 30` [M 0x191a70:136-140]).
    air_tick: u32,
    /// `BL+0x18` → the climb's one-tick local-forward push (`+0.6` top / `−0.05` action [M Player_Climb]),
    /// consumed by the next tick's movement (the game writes the wish `+0x18` after `Player_Move` and the
    /// next tick's move integrates it).
    push_fwd: f32,
}

impl Bond {
    /// Standing at `pos` (`Player_StandAtNewPosition`: the floor point + 1.1) facing `yaw`.
    pub fn new(pos: V3, yaw: f32) -> Bond {
        Bond { pos, yaw, pitch: 0.0, local: [0.0; 2], vel: [0.0; 3], grounded: true, floor_ny: 1.0, grounded_history: 8, jump: 0, land_delay: 0,
            crouched: false, lockout: 0, crouch_drop: 0.0, eye: EYE, fall_timer: 0, head_bump: false, fall_damage: None,
            substate: SS_WALK, ladder: None, climb_top: 0.0, climb_bottom: 0.0, regrab: 0, mode: 0, saved_sub: 0, air: 100.0, drown_damage: None, underwater: false,
            water_fade: 0.0, air_tick: 0, push_fwd: 0.0 }
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

    /// One 60 Hz tick (no ladders/water context — the pre-ladder signature, still used by the unit tests).
    pub fn tick(&mut self, world: &CollisionWorld, input: &Input) {
        self.tick_ctx(world, &LadderCtx::default(), input);
    }

    /// One 60 Hz tick with the ladder/water context: `Player_Update` 0x193600 — the regrab decay
    /// ([M 0x193840-0x19384c], the `obj+0xF4 == 1` branch that also zeroes BLData+0x10/+0x30 every tick),
    /// the substate dispatch ([M switch @0x193958]), then `Player_MonitorAir` 0x191a70 [M — it runs after
    /// every substate, before the collision pass].
    pub fn tick_ctx(&mut self, world: &CollisionWorld, ctx: &LadderCtx, input: &Input) {
        self.regrab = self.regrab.saturating_sub(1);
        self.fall_damage = None;
        self.drown_damage = None;
        match self.substate {
            SS_CLIMB => self.climb_tick(world, ctx, input),
            SS_SWIM => self.swim_tick(world, ctx, input),
            _ => self.walk_tick(world, ctx, input),
        }
        self.monitor_air(ctx);
    }

    /// `Player_SSWalk` 0x192310 / `Player_SSCrouch` 0x1924c0 (the walk/crouch substates share this body in
    /// the port; the game splits them). `Player_SSWalk`'s FIRST statement is the water check
    /// [M 0x192310:29-32 `if (Player_InWater()) return;`] — walking into water enters the swim substate
    /// instead of the walk tick. The ladder grab runs last: the game's `Player_CollisionHandler`
    /// 0x1949a8 follows the substate update and `Player_DealWithObjHit` 0x194458 dispatches hit class
    /// 0x2D to `Player_CollLadder` 0x196118.
    fn walk_tick(&mut self, world: &CollisionWorld, ctx: &LadderCtx, input: &Input) {
        if self.enter_water(ctx) { return; }
        self.physics_tick(world, input, false);
        self.ladder_grab(ctx);
    }

    /// `Player_Update` case 3 [M 0x193958 case 3]: `Player_Move(1.0, …)` — the walk physics with the
    /// multiplier 1.0 — then the inline swim tail: with a cel, Bond's y-velocity is clamped so he cannot
    /// rise above `cel+0x90 − 0.3`, and he leaves the water when grounded and `cel+0x90 <= pos.y − 0.3`
    /// (`BLData+0x130 = 2`, `ChangeSubState(0)`). `Player_Collision` 0x194df0 applies **no gravity** while
    /// Bond's y is below the water surface [M the default case jumps past `WldGravity` to the substate
    /// capsule switch when `cel+0x90 > obj+0x34`].
    fn swim_tick(&mut self, world: &CollisionWorld, ctx: &LadderCtx, input: &Input) {
        self.physics_tick(world, input, true);
        let Some(w) = ctx.water else { self.substate = SS_WALK; return };
        if self.grounded && w <= self.pos[1] - 0.3 {
            // Leave the water [M case 3: `(AnimObj+0x60 & 8) && cel+0x90 <= pos.y − 0.3` → substate 0].
            self.substate = SS_WALK;
            self.ladder = None;
            return;
        }
        // Vertical swim [S — the game's vertical swim input is not resolved in this pass; enter/leave,
        // the surface clamp and drowning are [M]]. Jump rises, crouch sinks. This is the ONLY vertical
        // integration this tick (the physics pass skipped the y velocity for swimming).
        self.vel[1] = if input.jump { 2.4 } else if input.crouch || ctx.sink { -2.4 } else { 0.0 };
        // The surface clamp [M]: `pos.y < surface − 0.3` → `vel.y = min(vel.y, surface − 0.3 − pos.y)`
        // (world-up); else any upward velocity is zeroed.
        if self.pos[1] < w - 0.3 {
            self.vel[1] = self.vel[1].min((w - 0.3 - self.pos[1]) * 60.0);
        } else if self.vel[1] > 0.0 {
            self.vel[1] = 0.0;
        }
        self.pos[1] += self.vel[1] / 60.0;
        // The grab scan runs from the swim substate too — `Player_CollLadder` excludes only substates 1
        // and 2, and `Player_CollisionHandler`'s grab dispatch follows every substate update [M 0x196118].
        self.ladder_grab(ctx);
    }

    /// `Player_CollLadder` 0x196118, fed by the `Player_DealWithObjHit` 0x194458 class-0x2D dispatch over
    /// the level's ladder objects. Guards [M 0x196118:16-17]: substate ∉ {1, 2}, regrab `+0x951 == 0`,
    /// health `+0x894 > 0` (the port keeps health in the Sim's `drones.health` — a dead Bond gets no input,
    /// so the health guard is enforced by the caller). Conditions [M]: `|AngleDiff(ladder.yaw, yaw)| > π/2`,
    /// then the y window: `bottom = cy − r + 1.0 < pos.y ≤ clamped ≤ top = cy + r + 0.85` with `pos.y`
    /// raised to `bottom + 0.02` first. On grab: snap to `centre + fwd(yaw)·0.5`, zero the horizontal
    /// velocity (`obj+0x50/+0x58 = 0`), mode `+0x94D = 1`, `+0x130 = 2`, `ChangeSubState(1)`,
    /// `Player_WeaponNone` (the Sim holsters to fists), `BLData+0x884 = ladder`.
    fn ladder_grab(&mut self, ctx: &LadderCtx) {
        if self.substate != SS_WALK || self.regrab != 0 { return; }
        for (i, lad) in ctx.ladders.iter().enumerate() {
            if !lad.capsule_hits(self.pos, RADIUS) { continue; }
            let diff = wrap_pi(lad.yaw - self.yaw); // Vec_AngleDifference(player, ladder) [M 0x1e2b78]
            if diff.abs() <= std::f32::consts::FRAC_PI_2 { continue; }
            let (top, bottom) = (lad.top(), lad.bottom());
            self.climb_top = top;
            self.climb_bottom = bottom;
            self.pos[1] = self.pos[1].max(bottom + 0.02);
            if !(bottom < self.pos[1] && self.pos[1] < top) { continue; }
            let (s, c) = lad.yaw.sin_cos();
            self.pos[0] = lad.centre[0] + s * 0.5;
            self.pos[2] = lad.centre[2] + c * 0.5;
            self.vel[0] = 0.0;
            self.vel[2] = 0.0;
            self.mode = 1;
            self.substate = SS_CLIMB;
            self.ladder = Some(i);
            break;
        }
    }

    /// `Player_Climb` 0x196310 (substate 1) [M]. Order: the yaw clamp+ease toward `ladder.yaw + π`;
    /// `Player_Move(0, …)` (no input movement — the local axes decay); `Vec_Zero(BLData+0x50)` (the world
    /// velocity zeroes every climb tick); the ladder carry `pos += ladder(+0xC0) − ladder(+0x40)` (zero for
    /// the port's static ladders); the input/height logic:
    /// - action (7) pressed with `forward < 0.05` → dismount, `+0x18 −= 0.05` (a slight backward push);
    /// - `pos.y > top` → `+0x18 += 0.6` (a forward lunge onto the platform), dismount;
    /// - `pos.y >= bottom` → climb: `fwd = forward`; a hit above (`AnimObj 0x20`) zeroes an up-push;
    ///   `!(hit below && fwd < 0)` → `+0x14 += fwd·0.02` (0.02 units/tick = 1.2 u/s at full push), stay;
    ///   else dismount;
    /// - `pos.y < bottom` → dismount.
    /// Every dismount: `regrab = (u8)FRAME_RATE` (60), `+0x942 = substate`, the anim carry cleared, and
    /// `ChangeSubState(0)` through the shared tail.
    fn climb_tick(&mut self, world: &CollisionWorld, ctx: &LadderCtx, input: &Input) {
        let li = match self.ladder {
            Some(li) if ctx.ladders.get(li).is_some() => li,
            _ => { self.substate = SS_WALK; self.ladder = None; self.physics_tick(world, input, false); return; }
        };
        let lad = &ctx.ladders[li];
        // The yaw clamp+ease [M 0x196310:18-33]: target = ladder.yaw + π (facing the ladder). The ease
        // uses the PRE-clamp diff (the game overshoots the clamp on the clamp tick — audit #4).
        let target = lad.yaw + std::f32::consts::PI;
        let diff0 = wrap_pi(target - self.yaw);
        let mut diff = diff0;
        if diff > std::f32::consts::FRAC_PI_2 { self.yaw = target - std::f32::consts::FRAC_PI_2; diff = std::f32::consts::FRAC_PI_2; }
        if diff < -std::f32::consts::FRAC_PI_2 { self.yaw = target + std::f32::consts::FRAC_PI_2; diff = -std::f32::consts::FRAC_PI_2; }
        // The ease runs while the AnimObj flag bit 0 is clear — bit 0 is the AIMING flag, managed by
        // `Player_Weapon`/`Player_Aiming` and cleared by `Player_WeaponNone` on the grab [M 0x1a9858:18-19
        // `&= 0xfffe`; 0x1a24c8:45-62] — so the ease pauses while the aim key is held (fix-ladder2).
        if !input.aiming { self.yaw += diff0 * 0.2; }
        // The look during the climb (fix-ladder2) [M 0x196310 + 0x193600 + 0x1a1d28/0x1a92e8]: the turn
        // channel (`bd+0x34`) still integrates into the yaw after `Player_Climb` — `Player_Update`'s
        // `bd+0x30..0x3F` → `obj+0x50..0x5F` integration — and the next tick's clamp above pulls it back
        // into the ±90° window; the pitch offset (`BL+0x8A8`) takes the stick like on foot:
        // `Player_Aiming` runs for every substate ≤ 7 (climb = 1 passes its early-out [M 0x1a1d28:21-30])
        // and `Player_ViewClamping` clamps ±1.0 [M 0x1a92e8:9-16].
        self.yaw += input.turn;
        self.pitch = (self.pitch + input.look).clamp(-1.0, 1.0);
        // No walk movement; the local axes decay toward 0 (Player_Move's smoothing with target 0) [M].
        self.local[0] *= 0.8;
        self.local[1] *= 0.8;
        self.vel = [0.0; 3]; // Vec_Zero(BLData+0x50) [M]
        // The ladder carry [M the +0xC0/−0x40 pair]: static ladders → 0 (no moving-ladder code found).
        // Input: action pressed (jump) with no forward → the action dismount [M 0x196310:45-48, 88-96].
        let fwd_raw = if input.aiming { 0.0 } else { input.forward.clamp(-1.0, 1.0) };
        let action = input.jump;
        let mut dismount_push = 0.0f32; // the +0x18 (local forward) one-tick push
        if action && fwd_raw < 0.05 {
            self.push_fwd = -0.05;
            dismount_push = -0.05;
            self.dismount();
        } else {
            // Recompute the thresholds every tick [M 0x196310:49-53].
            self.climb_top = lad.top();
            self.climb_bottom = lad.bottom();
            if self.climb_top < self.pos[1] {
                // Above the top: the +0.6 forward lunge [M 0x196310:54-62], consumed next tick.
                self.push_fwd = 0.6;
                dismount_push = 0.6;
                self.dismount();
            } else if self.climb_bottom <= self.pos[1] {
                // Mid-ladder: climb [M 0x196310:63-78].
                let mut fwd = fwd_raw;
                // A hit above (AnimObj flag 0x20 — the port's head_bump) blocks an up-push [M].
                if self.head_bump && fwd > 0.0 { fwd = 0.0; }
                // A hit below (flag 8) with a down-push dismounts [M]; the port reads the floor under
                // Bond's feet within 1.2 units as "hit below" [S — the game's hit comes from the
                // CollisionHandler's climb scan; see docs/atlas/ladder.md].
                let floor_below = world.floor_below(self.pos, 1.2).is_some();
                if !floor_below || fwd >= 0.0 {
                    self.vel[1] = fwd * 0.02 * 60.0; // +0x14 += fwd·0.02 per tick → 1.2·fwd u/s
                    self.physics_tick_climb(world);
                    return;
                }
                self.dismount();
            } else {
                // Below the bottom: dismount [M 0x196310:79-86].
                self.dismount();
            }
        }
        // The climb tick still runs the collision pass [M — Player_Collision/Handler run for substate 1;
        // the CollisionHandler's climb branch scans the hit list for the ladder and sets the 0x20/8 flags].
        self.physics_tick_climb(world);
        let _ = dismount_push; // the push is stored at the dismount sites below
    }

    /// The climb dismount common path [M 0x196310 dismount branches]: `regrab = (u8)FRAME_RATE`,
    /// `+0x942 = substate`, `+0x14 = 0`, the anim carry (`AnimObj+0xDC = 0`), `ChangeSubState(0)` (the
    /// port stores the substate directly; the game's dispatch case 10 path is the scripted-cam restore,
    /// not this). `+0x94D` (the mode byte) is NOT touched by the game's dismount — the next grab sets it.
    fn dismount(&mut self) {
        self.regrab = 60;
        self.saved_sub = SS_CLIMB as u16;
        self.substate = SS_WALK;
        self.ladder = None;
        self.local[1] = 0.0; // the dismounts write BLData+0x14 = 0 [M]
    }

    /// The collision pass for the climb substate: the capsule push and the floor ray (the game runs
    /// `Player_Collision` 0x194df0 + `Player_CollisionHandler` 0x1949a8 for every substate), without the
    /// walk-only bookkeeping (jump/land/fall timers).
    fn physics_tick_climb(&mut self, world: &CollisionWorld) {
        // The climb velocity integrates first (the game's wish plumbing lands it in the position before
        // the collision pass).
        for k in 0..3 { self.pos[k] += self.vel[k] / 60.0; }
        let (a, b) = self.capsule();
        let (push, walkable) = world.capsule_push(a, b, RADIUS);
        self.head_bump = push[1] < -1e-4;
        for k in 0..3 { self.pos[k] += push[k]; }
        let ray = ROOT_HEIGHT + 0.6;
        let floor = world.segment_hit(self.pos, [self.pos[0], self.pos[1] - ray, self.pos[2]]);
        self.grounded = walkable || floor.is_some_and(|(t, _)| t * ray - 0.1 <= ROOT_HEIGHT);
        if let Some((_, n)) = floor { self.floor_ny = n[1]; } else if walkable { self.floor_ny = 1.0; }
    }

    /// `Player_InWater` 0x195e50 [M]: with a water cel and `pos.y − 0.3 < surface`, enter the swim
    /// substate — `ChangeSubState(3)`, `+0x94C = 0`, `+0x130 = 2`, `+0x14 = 0`, `regrab = (u8)(DIV·5) = 5`,
    /// `obj+0x50/+0x58 = 0`, the per-tick deltas cleared. Returns whether Bond is swimming (substate 3).
    fn enter_water(&mut self, ctx: &LadderCtx) -> bool {
        let Some(w) = ctx.water else { return false };
        if self.pos[1] - 0.3 >= w { return false; }
        if self.substate != SS_SWIM {
            self.substate = SS_SWIM;
            self.jump = 0;        // +0x94C = 0 [M]
            self.local[1] = 0.0;  // +0x14 = 0 [M]
            self.regrab = 5;      // (u8)(FRAME_RATE_DIV·5) [M]
            self.vel[0] = 0.0;    // obj+0x50 = 0 [M]
            self.vel[2] = 0.0;    // obj+0x58 = 0 [M]
            self.mode = 0;
            self.ladder = None;
        }
        true
    }

    /// `Player_MonitorAir` 0x191a70 [M], run after every substate: the head (the port uses the eye [S])
    /// against `cel+0x90`: below the surface the air `+0x8C0` drains at 5/s from 100 and the underwater
    /// fade `+0x8C4` rises; above it (any cel — the −500 default is far below every head) the air refills
    /// at 20/s. At `air < 1.0` every 30 ticks (`GameState % (FRAME_RATE·0.5) == 0`):
    /// `Player_HandlePain(0x40000000 = 2.0, type 7)` [M 0x191a70:136-143, disasm 0x191d9c] — the drown
    /// damage the caller applies. The breath loop sounds (0x60c surfaced / the underwater loop on
    /// `+0x89C`) and the HUD air pane are not ported this pass.
    fn monitor_air(&mut self, ctx: &LadderCtx) {
        // The game's refill branch runs for ANY cel (including the −500 default) when the head is above
        // `cel+0x90` [M 0x191a70:98-111] — air refills on dry land too.
        let surface = ctx.water.unwrap_or(-500.0);
        let head = self.head()[1];
        self.underwater = head < surface && surface > -400.0;
        self.water_fade = if self.underwater { (self.water_fade + (1.0 - self.water_fade) / 60.0).min(1.0) }
            else { (self.water_fade - self.water_fade / 120.0).max(0.0) };
        if self.underwater {
            self.air -= 5.0 / 60.0;
            if self.air < 1.0 {
                self.air_tick += 1;
                if self.air_tick >= 30 { self.air_tick = 0; self.drown_damage = Some(2.0); }
            }
        } else {
            self.air = (self.air + 20.0 / 60.0).min(100.0);
            self.air_tick = 0;
        }
        self.air = self.air.clamp(0.0, 100.0);
    }

    /// One 60 Hz physics tick (the port's `Player_Move` + `Player_Collision` walk body). `swimming`
    /// selects the swim-substate differences: no `Player_HandleJump` (case 3 doesn't call it [M]) and no
    /// gravity (`Player_Collision`'s water path skips `WldGravity` [M]).
    fn physics_tick(&mut self, world: &CollisionWorld, input: &Input, swimming: bool) {
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
        let (vs, vf0) = (axis(self.local[0], target_s) * k, axis(self.local[1], target_f) * k);
        // `Player_HandleJump`: crouch (toggle) and jump. Case 3 (swim) doesn't call it [M].
        let headroom = |w: &CollisionWorld| w.line_of_sight(self.pos, [prev[0], prev[1] + 0.95, prev[2]]);
        if swimming {
        } else if !self.crouched && self.jump == 0 && input.crouch && self.lockout == 0 {
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
        // The matrix turns first, then the position moves along the new axes. The climb's one-tick
        // local-forward push (Player_Climb's ±0x18 write) rides along once.
        let vf = vf0 + self.push_fwd;
        self.push_fwd = 0.0;
        self.yaw += turn;
        self.pitch = (self.pitch + input.look).clamp(-1.0, 1.0);
        let (s, c) = (self.yaw.sin(), self.yaw.cos());
        self.pos[0] += c * vs + s * vf;
        self.pos[2] += -s * vs + c * vf;
        self.local = [vs / k, vf / k];
        // `Player_Collision`: gravity unless standing on a gentle floor (jumping keeps the velocity).
        // The water path skips `WldGravity` entirely [M 0x194df0 default case]. The vertical velocity
        // integration is also the swim tail's job alone (it clamps first) — skip it here when swimming.
        if swimming {
        } else if !self.grounded || self.floor_ny < 0.5 {
            self.vel[1] += GRAVITY / 60.0;
        } else if self.jump != 1 {
            self.vel = [0.0; 3];
        }
        self.vel[1] = self.vel[1].clamp(-45.0, 45.0);
        for k in 0..3 { if !(swimming && k == 1) { self.pos[k] += self.vel[k] / 60.0; } }
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
        if swimming {
        } else if self.grounded_history == 0 && self.vel[1] < 0.0 { self.fall_timer += 1; }
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

/// `Vec_AngleDifference` 0x1e2b78 [M]: `wrap(b − a)` into (−π, π].
fn wrap_pi(d: f32) -> f32 {
    let mut d = d % std::f32::consts::TAU;
    if d >= std::f32::consts::PI { d -= std::f32::consts::TAU; }
    if d < -std::f32::consts::PI { d += std::f32::consts::TAU; }
    d
}

// ── Level carry-over and spawn (fix-level2) ────────────────────────────────────────────────────────

/// `Player_RamSave` (0x195670) → the global `RamSave` (0x2b9440, 0xD50 bytes) [M]. The port carries the
/// fields the port models; the game also saves `PlrMissionStats` (0x38), `SSysItems` (0x28) and
/// `BondMoments` (0x14) — none of which the port has (WORKLOG fix-level2).
#[derive(Clone, Debug, Default)]
pub struct RamSave {
    /// `RamSave+0x5FC ← BLData+0x894` [M 0x195670:16].
    pub health: f32,
    /// `RamSave+0x600 ← BLData+0x8B0` [M 0x195670:17].
    pub armour: f32,
    /// `RamSave+0xA18 ← M_CONTROL+0x63` (the switch-request byte) [M 0x195670:18-19].
    pub weapon: u8,
    /// `RamSave+0x038 ← BLData+0x170`, 0x42 = 33 s16 spare-ammo rounds [M 0x195670:6].
    pub carried: Vec<i16>,
    /// `RamSave+0x07C ← BLData+0x1B4`, 114 weapon entries × 12 B: clip s16 at +4, owned u8 at +6
    /// [M 0x195670:7; atlas player.md].
    pub clip: Vec<i16>,
    pub owned: Vec<bool>,
    /// `RamSave+0xA1A ← BLData+0x70C`, 0x33 key-ring bytes [M 0x195670:8].
    pub keys: Vec<bool>,
    /// `RamSave+0xA4D ← switch_channels`, 0x100 [M 0x195670:13].
    pub channels: Vec<u8>,
    /// `RamSave+0x604 ← switch_channels_time`, 0x400 [M 0x195670:12]. The game also saves `_hold` and
    /// `_prev` (0x2b9f8d/0x2ba08d); the port models no hold array and keeps `_prev` per-consumer
    /// (trigger-port-check #10) [M gap noted in the WORKLOG].
    pub ch_time: Vec<u32>,
}

impl RamSave {
    /// The save `Trigger_Activate` makes on the way out of a level, after the success/fallback channel
    /// write [M 0x1b2318:97-116].
    pub fn capture(health: &crate::bullet::PlayerHealth, weapon: u8, inv: &crate::pickup::Inventory, ch: &Channels) -> RamSave {
        RamSave {
            health: health.health,
            armour: health.armour,
            weapon,
            carried: inv.carried.clone(),
            clip: inv.clip.clone(),
            owned: inv.owned.clone(),
            keys: inv.keys.clone(),
            channels: ch.v.clone(),
            ch_time: ch.time.clone(),
        }
    }

    /// `Player_RamLoad` (0x1957a8), `IsWarmReset != 0` arm [M]: the whole body is gated on the warm flag,
    /// so a cold load (`save = None`) restores nothing. Restores health (`Player_SetHealth`), armour
    /// (+0x8B0), the saved weapon id (+0x934) and the four channel arrays. The `boost` argument (the death
    /// retry's health floor, `P_ENDMISSION_Handler` 0x216618) is not ported — the port has no death-retry
    /// reload path (WORKLOG fix-level2).
    pub fn restore(&self, health: &mut crate::bullet::PlayerHealth, inv: &mut crate::pickup::Inventory, ch: &mut Channels) -> u8 {
        health.health = self.health;
        health.armour = self.armour;
        let fit = |dst: &mut Vec<i16>, src: &[i16]| { dst.resize(src.len(), 0); dst.copy_from_slice(src); };
        fit(&mut inv.carried, &self.carried);
        fit(&mut inv.clip, &self.clip);
        inv.owned.resize(self.owned.len(), false);
        for (d, &s) in inv.owned.iter_mut().zip(&self.owned) { *d = s; }
        inv.keys.resize(self.keys.len(), false);
        for (d, &s) in inv.keys.iter_mut().zip(&self.keys) { *d = s; }
        ch.v.resize(256, 0);
        ch.v.copy_from_slice(&self.channels);
        ch.time.resize(256, 0);
        ch.time.copy_from_slice(&self.ch_time);
        self.weapon
    }
}

/// One registered player start (`player_start` entry, 0xE0 bytes): position, yaw, and the gate switch
/// channel — placement type 0x24's parameter 0 [M `parsemap_create_dynamic_objects` 0x1d04f0 case 0x24 →
/// `Player_AddNewStartPos` 0x195c00: param block +0x2C lands at entry+0x50].
#[derive(Clone, Copy, Debug)]
pub struct LevelStart { pub pos: V3, pub yaw: f32, pub gate: u16 }

/// `Player_Start` (0x190fd0) [M]: pass 1 — the first entry with kind 1 whose gate is 0 or whose channel is
/// on; pass 2 (fallback) — the first kind-1 entry (the game zeroes its gate so a later `Player_Start` in
/// the same level cannot re-pick it; the port runs one `Player_Start` per load). Index order = placement
/// order [M 0x195c00 appends]. Returns the entry index, or None with no entries.
pub fn pick_start(starts: &[LevelStart], ch: &Channels) -> Option<usize> {
    if let Some(i) = starts.iter().position(|s| s.gate == 0 || ch.on(s.gate as u32)) { return Some(i); }
    starts.iter().position(|_| true)
}

/// `Player_InitAmmoWeapons` (0x1a17f8) with the boot flag `DAT_002a373c == 0`: carried = 0, clip = 0,
/// owned = false for every weapon [M :20-45]. The [L] boot snapshot proves this arm ran: owned after
/// `Player_InitWeapon` = exactly the equip set {1, 2, 0x4A, 0x4E, 0x50, 0x54} — the +0x04 byte is 1 on
/// every probed weapon, so the non-zero arm (owned = +0x04) would have owned 23 (ee_start_05.bin;
/// WORKLOG fix-level2). `Player_CheckWeaponsLoaded` (0x1a97c8) can only revoke owned — a no-op from
/// all-false. The equip calls that follow carry the `loaded` refusal of `Player_EquipWeapon`
/// [M 0x1a8a58:20].
pub fn init_ammo_weapons(inv: &mut crate::pickup::Inventory, w: &nf_formats::weapon::WeaponData, loaded: impl Fn(u8) -> bool) {
    for c in inv.carried.iter_mut() { *c = 0; }
    for id in 0..inv.clip.len().min(0x72) {
        inv.clip[id] = 0;
        inv.owned[id] = false;
    }
    let _ = (w, loaded);
}

/// `Player_EquipWeapon` is `Inventory::equip_weapon` [M 0x1a8a58]; the game looks the model up in the
/// hashtable and refuses when missing — the port's `loaded` closure stands in.
fn equip(inv: &mut crate::pickup::Inventory, w: &nf_formats::weapon::WeaponData, id: u16, rounds: i16, loaded: impl Fn(u8) -> bool) {
    inv.equip_weapon(w, id as usize, rounds, loaded(id as u8));
}

/// `Player_InitWeapon` (0x1a7ef8) — the per-level loadout switch on `GameState+0x0C`, the part the
/// campaign's first mission walks through. `save = Some(…)` = a warm transition (`IsWarmReset != 0`, the
/// only way a `LOAD_LEVEL`/script-event-0x12 transition loads [M 0x1d3800, 0x1b24cc]); `None` = cold
/// (`Player_RamLoad`'s whole body is skipped [M 0x1957a8:9]).
///
/// Returns the switch request the game writes to `M_CONTROL+0x63` (the draw machine then holsters and
/// draws it). Cases [M 0x1a7ef8:148-317]:
/// - 0x0700005 (fresh, and the fallback of 06): P2K 48 rounds, taser 999, laser 0, watch 0, + the common
///   tail; request = `Upgrade_Weapon(6)` = 2 [L UpgradedHandguns 0x2bee08 = [2,4,6,8], Upgrades 0].
/// - 0x0700006: `Player_RamLoad`; request = the saved weapon; if the P2K is not owned → the fresh 05 case.
/// - 0x0700007: `Player_RamLoad`, `Player_InitAmmoWeapons`, armour = 0, P2K 48 / taser 999 / laser 0 /
///   watch 0, request = **1 (fists)** — the party disguise.
/// - 0x0700008: `Player_RamLoad`; request = the saved weapon; if the P2K is not owned → the 07 case.
/// - default (levels outside the switch, e.g. 07000001): the [L] 07000005 start loadout
///   (`Inventory::level_start`), request = 2.
///
/// Common tail [M 0x1a7ef8:318-325]: equip laser 0x4E 999, equip fists 1 with 0 rounds, re-save. The
/// "current weapon's ammo type unowned → request 1" check (:321-324) and the final `Player_RamSave` are
/// the caller's (the port re-captures at the next exit).
pub fn init_level_loadout(
    level: u32,
    save: Option<&RamSave>,
    health: &mut crate::bullet::PlayerHealth,
    inv: &mut crate::pickup::Inventory,
    ch: &mut Channels,
    w: &nf_formats::weapon::WeaponData,
    loaded: impl Fn(u8) -> bool + Copy,
) -> u8 {
    // `Upgrade_Weapon(6)` (0x1a8f30 case 6 → `UpgradedHandguns[Upgrades]`, slot 0 = 2 [L]).
    const HANDGUN: u16 = 2;
    let mut req = HANDGUN as u8;
    // A cold load (`save = None`, `IsWarmReset == 0`) skips `Player_RamLoad`'s whole body [M
    // 0x1957a8:9], so the inventory `Player_InitWeapon` reads behind it is the game's boot state —
    // the one `Player_InitAmmoWeapons` (0x1a17f8) zeroed (the [L] boot snapshot proves the zero arm
    // ran: owned = exactly the equip set, WORKLOG fix-level2). fix-perf-level: the port's Sim used to
    // carry the viewer's legacy default (P2K 7/41, `Gun::new(2,7,41)` + the sim's built-in owned
    // flags), so case 0x7000006/8's `p2k_owned` read TRUE on a cold load and the request became the
    // uninitialized saved weapon 0 — the draw machine holstered into weapon 0 and Bond stood unarmed
    // (nf-view 07000006 --frame 30: "switch request 0", the user's no-weapon report). Clear first on
    // a cold load; cases 5/7 and the default re-clear or replace anyway, so only 6/8 change.
    if save.is_none() { init_ammo_weapons(inv, w, loaded); }
    // `Player_RamLoad` (0x1957a8): the saved health/armour/weapon/inventory/channels.
    let saved_weapon = save.map(|s| s.restore(health, inv, ch)).unwrap_or(0);
    let p2k_owned = |inv: &crate::pickup::Inventory| inv.owned.get(HANDGUN as usize).copied().unwrap_or(false);
    let fresh_05 = |inv: &mut crate::pickup::Inventory, health: &mut crate::bullet::PlayerHealth| {
        init_ammo_weapons(inv, w, loaded);
        equip(inv, w, HANDGUN, 0x30, loaded);
        equip(inv, w, 0x4a, 999, loaded);
        equip(inv, w, 0x50, 0, loaded);
        equip(inv, w, 0x54, 0, loaded);
        let _ = health;
    };
    let party = |inv: &mut crate::pickup::Inventory, health: &mut crate::bullet::PlayerHealth| {
        init_ammo_weapons(inv, w, loaded);
        *armour_in(health) = 0.0;
        equip(inv, w, HANDGUN, 0x30, loaded);
        equip(inv, w, 0x4a, 999, loaded);
        equip(inv, w, 0x50, 0, loaded);
        equip(inv, w, 0x54, 0, loaded);
    };
    match level {
        0x0700_0005 => fresh_05(inv, health),
        0x0700_0006 => {
            req = saved_weapon;
            if !p2k_owned(inv) { fresh_05(inv, health); req = HANDGUN as u8; }
        }
        0x0700_0007 => { party(inv, health); req = 1; }
        0x0700_0008 => {
            req = saved_weapon;
            if !p2k_owned(inv) { party(inv, health); req = 1; }
        }
        _ => { *inv = crate::pickup::Inventory::level_start(inv.clip.len(), inv.carried.len()); req = HANDGUN as u8; }
    }
    // Common tail [M 0x1a7ef8:318-319].
    equip(inv, w, 0x4e, 999, loaded);
    equip(inv, w, 1, 0, loaded);
    req
}

/// The armour lives in `PlayerHealth` next to health; a tiny accessor keeps `party`'s closure signature
/// flat (the game writes `BLData+0x8B0` [M 0x1a7ef8:180]).
fn armour_in(h: &mut crate::bullet::PlayerHealth) -> &mut f32 { &mut h.armour }

/// The party weapon lock (`Player_Weapon` 0x1a24c8:242-275) [M]: on 0x07000007 with switch channel 0x48
/// OFF (`if (switch_channels[0x48] == '\0') bVar4 = false`), the weapon-cycle actions route to
/// `Player_WeaponChange(±1, mode 2)` — the fists-only cycle. The port zeroes the cycle request (the
/// mode-2 cycle from fists re-picks fists and plays the empty-switch click 0x1DB — no switch, sound
/// aside) and, as the only way a player could otherwise reach a gun, refuses direct selections that are
/// not the fists [S: the number-key path's lock is not verified in the decompile; the observable game
/// behaviour is "no weapons at the party"].
pub const WEAPON_LOCK_LEVEL: u32 = 0x0700_0007;
/// The re-arm channel: TouchOnce placements 411/412 on 07000007 write it [D nf-extract triggers];
/// `Trigger_Activate` case 1 latches it [M 0x1b2318:41-49].
pub const WEAPON_LOCK_CHANNEL: u32 = 0x48;

pub fn weapon_lock_input(level: u32, ch48_on: bool, input: &mut Input) {
    // [M 0x1a24c8:243-246]: `if (GameState == 0x7000007) { if (switch_channels[0x48] == 0) bVar4 = false; }`
    // — the lock is active while the channel is OFF.
    if level == WEAPON_LOCK_LEVEL && !ch48_on {
        input.weapon_cycle = 0;
        if input.switch_to.is_some_and(|w| w != 1) { input.switch_to = None; }
    }
}

/// `Player_Weapon`'s rising-edge re-arm [M 0x1a24c8:247-250]: channel 0x48 on while `switch_channels_prev`
/// still reads off → the switch request becomes `Upgrade_Weapon(6)` = 2 (the P2K auto-draws).
pub fn weapon_lock_auto_draw(level: u32, on: bool, prev: bool) -> bool {
    level == WEAPON_LOCK_LEVEL && on && !prev
}

#[cfg(test)]
mod tests {
    use super::*;

    /// fix-ladder2: the climb look [M Player_Climb 0x196310 + Player_Update 0x193600 + ViewClamping
    /// 0x1a92e8]. The turn input moves the yaw inside the eased ±90° window around `ladder.yaw + π`,
    /// and the pitch offset takes the stick (free, clamped ±1). Before the fix neither input reached
    /// the climb substate.
    #[test]
    fn climb_look_turns_within_the_ladder_window_and_pitch_is_free() {
        let world = CollisionWorld::from_triangles(Vec::new());
        let lad = Ladder { centre: [10.0, 3.0, 20.0], radius: 2.0, yaw: 0.0, channel: 0 };
        let ctx = LadderCtx { ladders: &[lad.clone()], water: None, sink: false };
        let mut b = Bond::new([10.0, 3.0, 20.0 + 0.5], lad.yaw + std::f32::consts::PI);
        b.substate = SS_CLIMB;
        b.ladder = Some(0);
        b.climb_top = lad.top();
        b.climb_bottom = lad.bottom();
        // Turn left 0.05 rad/tick for a second: the yaw leaves the facing but stays within ±90°.
        let turn = Input { turn: 0.05, ..Default::default() };
        for _ in 0..60 { b.tick_ctx(&world, &ctx, &turn); }
        let target = lad.yaw + std::f32::consts::PI;
        let dev = wrap_pi(target - b.yaw).abs();
        // The equilibrium under a held turn is turn/0.2 (the ease opposes the turn [M 0x196310]) — 0.25.
        assert!(dev > 0.1, "the turn input never moved the yaw during the climb (deviation {dev}) — the camera is stuck");
        assert!(dev <= std::f32::consts::FRAC_PI_2 + 1e-3, "the yaw left the ±90° ladder window: {dev}");
        // Pitch: free, clamped ±1 [M ViewClamping 0x1a92e8].
        let look = Input { look: 0.05, ..Default::default() };
        for _ in 0..60 { b.tick_ctx(&world, &ctx, &look); }
        assert!(b.pitch > 0.3, "the pitch offset did not move during the climb ({})", b.pitch);
        for _ in 0..240 { b.tick_ctx(&world, &ctx, &look); }
        assert!(b.pitch <= 1.0 + 1e-4 && b.pitch >= -1.0 - 1e-4, "the pitch offset passed the ViewClamping clamp: {}", b.pitch);
        // Sister: without turn input the eased yaw converges to the ladder facing (the pre-existing ease).
        let mut b2 = Bond::new([10.0, 3.0, 20.0 + 0.5], lad.yaw + std::f32::consts::PI + 0.8);
        b2.substate = SS_CLIMB; b2.ladder = Some(0);
        b2.climb_top = lad.top(); b2.climb_bottom = lad.bottom();
        let idle = Input::default();
        for _ in 0..120 { b2.tick_ctx(&world, &ctx, &idle); }
        assert!(wrap_pi(target - b2.yaw).abs() < 0.1, "the ease no longer converges to the ladder facing");
    }

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
