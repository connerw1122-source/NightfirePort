//! Bond on foot: `Player_Update` (0x193600) with `Player_Move` (0x191dd0), `Player_HandleJump` (0x1916b0),
//! `Player_Collision` (0x194df0), `Player_CollisionHandler` (0x1949a8) and the eye of `Player_PositionCamera`.
//! Spec: docs/systems/player.md.
//!
//! Level carry-over and spawn (fix-level2): `Player_RamSave` (0x195670) / `Player_RamLoad` (0x1957a8),
//! the `player_start` table (`Player_AddNewStartPos` 0x195c00, `Player_Start` 0x190fd0) and the per-level
//! loadout switch of `Player_InitWeapon` (0x1a7ef8) with `Player_Weapon`'s party weapon lock (0x1a24c8).

use crate::ladder::Ladder;
use crate::triggers::Channels;
use crate::wire::{Wire, WireAnims};
use crate::world::{CollisionWorld, V3};

/// `AnimObj+0xCC` [L]: the root height above the feet (07000005, skin 0x05000007), the STANDING value.
pub const ROOT_HEIGHT: f32 = 1.0347183;
/// The CROUCHED stance's root height [L, derived from the PCSX2 trace 05-truck]: riding the truck bed
/// crouched, Bond parks with obj+0x30 = 37.887 over the bed floor 37.280 — pos−floor 0.607, i.e. cc
/// 0.556 after the standing rest offset (0.051: standing live 1.086 with cc 1.0347) — ≈ the capsule
/// radius 0.55 [M 0x194df0: 0x3f0ccccd]. The game's cc is **AnimObject+0xCC** (obj+0xDC+0xCC): the
/// anim/stance writes it and the feet-point math reads it there [M Player_Collision 0x194df0:86,
/// 222-236]. The every-tick copy at Player_Update 0x193600:449-453 / Player_CollisionHandler
/// 0x1949a8:179-183 is `obj+0xC0..0xCC = obj+0x30..0x3C` — the matrix ROW 3 mirroring the position
/// (gated on flags2 bit 4), NOT a root-height write (fields-player correction: the earlier
/// `obj+0xCC = obj+0x3C` reading mistook it). The port models the two stances as constants.
pub const CROUCH_ROOT_HEIGHT: f32 = 0.55;
/// Capsule radius (`HITTEST+0x8C`).
pub const RADIUS: f32 = 0.55;
/// `BL+0x908` on land.
pub const EYE: f32 = 0.7;
const GRAVITY: f32 = -9.8;

/// `obj+0xF6` substates, from the `Player_Update` 0x193600 dispatch [M switch @0x193958; the jump table
/// reads obj+0xF6 and its cases call Player_SSWalk 0x192310 (0), Player_Climb 0x196310 (1),
/// Player_SSGrapple 0x192860 (2), the inline swim (3), Player_SSCrouch 0x1924c0 (4), Player_SSScanMode
/// 0x193098 (5), Player_Wire 0x197658 (6), Player_Creep 0x1969d0 (7), Player_ZeroG 0x192ab0 (8/9), the
/// scripted-cam countdown (10), Player_Zipline 0x197f50 (0xF)]. This session ports 1, 3, 6 and 0xF.
pub const SS_WALK: u8 = 0;
pub const SS_CLIMB: u8 = 1;
pub const SS_GRAPPLE: u8 = 2;
pub const SS_SWIM: u8 = 3;
/// `Player_Wire` 0x197658 — the hand-over-hand wire crossing (wire type 0) [M dispatch case 6].
pub const SS_WIRE: u8 = 6;

/// The crawl substate (`Player_Creep` 0x1969d0; entered by `Player_CollCreepWall` 0x196758 through
/// `Player_Activate`'s class-'+' use scan).
pub const SS_CREEP: u8 = 7;
/// `Player_Zipline` 0x197f50 — the zipline ride (wire type 2) [M dispatch case 0xF].
pub const SS_ZIPLINE: u8 = 0xf;

/// The per-tick ladder/wire/water context the Sim hands `Bond::tick_ctx` (the game reads the ladder and
/// the wire through `BLData+0x884` and the water surface through the player's cel, `obj+0x20 → cel+0x90`).
pub struct LadderCtx<'a> {
    /// The level's ladder objects (class 0x2D), in placement order.
    pub ladders: &'a [Ladder],
    /// The level's wire objects (class 0x2C), in placement order (`Wire::all`).
    pub wires: &'a [Wire],
    /// The wire anims' root tracks (`wire::WireAnims`); `None` = no anim data (the anims hold their
    /// start pose — no root motion; the clamp still holds Bond on the wire).
    pub wire_anims: Option<&'a WireAnims>,
    /// `cel+0x90` for the cel Bond is in (`Player_InWater` 0x195e50 reads `*(obj+0x20)+0x90`;
    /// `build_alloc_cel` 0x1c93f8 defaults it to −500.0 = no water; `parseentity_fixup_entity` 0x1d00c0
    /// sets it to the entity's bbox max y for flag word 0xC023). `None` = no water cel / below −400.
    pub water: Option<f32>,
    /// Whether Bond holds the descend control this tick [S — the game's vertical swim input is not
    /// resolved in this pass; see docs/atlas/ladder.md "Not verified"].
    pub sink: bool,
    /// The level's creepwall objects (class 0x2B '+', placement 0x3F), in placement order.
    pub creepwalls: &'a [crate::creepwall::CreepWall],
}

impl Default for LadderCtx<'_> {
    fn default() -> Self { LadderCtx { ladders: &[], wires: &[], wire_anims: None, water: None, sink: false, creepwalls: &[] } }
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
    /// The fire action held this tick (action 9; the substates read it directly — the grapple's
    /// reel/hang/release [M Player_SSGrapple 0x192860's Input_Action(9, 1)]). The weapon fire path
    /// gets the same bit through its own parameter.
    pub fire: bool,
    /// The fire-mode button's newly-pressed edge (pad ○, action 12 mask 4) [M `Player_Weapon`
    /// 0x1a24c8: `Input_Action(slot, 0xc, 4)`]. Cycles the mode byte (`BL + w*0xC + 0x1BB`) on
    /// records without a twin link, or switches to the twin variant (weapon_data +0x05) when set.
    /// Keyboard: M (documented in the HUD help).
    pub mode_switch: bool,
}

#[derive(Clone, Debug)]
pub struct Bond {
    /// `obj+0x30`.
    pub pos: V3,
    /// `obj+0x40` — the previous position the movement pass copies after the cel change
    /// (`control_movement_object_handler` 0x1336f8:212), so the next `control_handle_cel_change`
    /// 0x133288 relinks on the last tick's move segment. fix-swim-entry.
    pub prev_pos: V3,
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
    /// `obj+0xCC` (= `obj+0x3C` [M Player_Update 0x193600:449]): the current stance's root height
    /// above the feet. The capsule ends and the floor ray hang off this, so a crouched Bond is a
    /// shorter capsule resting closer to his feet (the truck ride parks him at floor + 0.607).
    pub cc: f32,
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
    /// `BL+0x884`: the carried creepwall object while the crawl substate (7) runs (`Player_CollCreepWall`
    /// 0x196758 stores the wall; `Player_Creep` 0x1969d0 reads it back every tick).
    pub creep: Option<usize>,
    /// The runtime cel the queries start from (obj+0x20, the movement pass's relink — fix-safe08b):
    /// the `pCelList` cell index in the world's `ScopeGraph`. `None` = the flat (unscoped) world.
    pub cel: Option<usize>,
    /// The drown-damage tick accumulator (the game uses `GameState % 30` [M 0x191a70:136-140]).
    air_tick: u32,
    /// `BL+0x18` → the climb's one-tick local-forward push (`+0.6` top / `−0.05` action [M Player_Climb]),
    /// consumed by the next tick's movement (the game writes the wish `+0x18` after `Player_Move` and the
    /// next tick's move integrates it).
    push_fwd: f32,
    /// `BLData+0x884` narrowed to the wires: the wire being ridden (an index into `LadderCtx::wires`).
    /// The game stores the object pointer; the port indexes the ctx slice like `ladder` [M CollWire
    /// 0x197628 writes +0x884 = the wire object].
    pub wire: Option<usize>,
    /// `BLData+0x8CC`: the wire auto-drop timer — `+= 100/(FRAME_RATE·30)` per tick (paused in the grab
    /// pose, mode 0xC); past 100 the wire drops him [M Player_Wire 0x197748-0x1977a8].
    pub auto_drop: f32,
    /// `oldhash` — the hashcode of the wire anim playing on Bond's AnimObject (the grab pose 0x060005e5,
    /// the idle pair 0x060005ec/ed, the turn pair 0x060005ee/ea, the move pair 0x060000b4/b5, the
    /// zipline 0x06000095/96/94) [M Player_Wire's oldhash tree]. 0 = none.
    pub wire_anim: u32,
    /// The anim's script time in frames (`AnimScriptAdd` starts at 1; `AnimScriptIsStopped` = past the
    /// script's length for these non-looping scripts [D all seven scripts carry looping = false]).
    pub wire_anim_time: f32,
    /// The playing wire anim's rate (`AnimScript+0x98`): 1.0 for the grab pose, ±1.2 for everything
    /// `Player_Wire` itself adds (the tail at 0x197ec4 sets ±1.2; a fresh script's rate is 0 → +1.2 [S —
    /// the negative branch's trigger is not resolved]).
    pub wire_anim_rate: f32,
    /// The camera mode the grab/drop paths set (`Player_SetCamMode`: 0xB on the wire, 0 on foot) [M
    /// CollWire 0x1975f8, Player_Wire 0x1978a8]. The renderer can adopt it; the headless sim only
    /// records it.
    pub cam_mode: u16,
    /// `BLData+0x18`: the fall-speed accumulator — the zipline ride adds FRAME_RATE_MUL·0.15 per tick
    /// and the creep drop 0.05 [M Player_Zipline 0x198054, Player_Creep; the game's `Player_Move`
    /// applies it [S — the atlas's BLData table]].
    pub fall_speed: f32,
    /// The Q-Claw grapple's runtime state (`BLData+0x140..0x160` narrowed; fix-gadgets). `Some`
    /// while the substate 2 runs [M Player_SetGrapplePoint 0x195da0 → ChangeSubState(2)].
    pub grapple: Option<crate::gadget::GrappleState>,
    /// `BLData+0x95F`: the third-icon interaction context — 0xFF = none. `Player_CollisionHandler`
    /// 0x1949a8:22 resets it every tick and `ThirdIcon_Update` 0x196e80 writes the enabled icon's
    /// type (param 0) while the player is in its room; `Player_HandleJump` 0x1916b0:118-147 grabs the
    /// nearest wire when the jump is pressed with 3 (the cable context). The Sim (scene.rs) owns the
    /// reset + the icon writes.
    pub icon_ctx: u8,
}

impl Bond {
    /// Standing at `pos` (`Player_StandAtNewPosition`: the floor point + 1.1) facing `yaw`.
    pub fn new(pos: V3, yaw: f32) -> Bond {
        Bond { pos, yaw, pitch: 0.0, local: [0.0; 2], vel: [0.0; 3], grounded: true, floor_ny: 1.0, grounded_history: 8, jump: 0, land_delay: 0,
            crouched: false, lockout: 0, cc: ROOT_HEIGHT, crouch_drop: 0.0, eye: EYE, fall_timer: 0, head_bump: false, fall_damage: None,
            substate: SS_WALK, ladder: None, climb_top: 0.0, climb_bottom: 0.0, regrab: 0, mode: 0, saved_sub: 0, air: 100.0, drown_damage: None, underwater: false,
            water_fade: 0.0, air_tick: 0, push_fwd: 0.0, wire: None, auto_drop: 0.0, wire_anim: 0, wire_anim_time: 0.0, wire_anim_rate: 1.0, cam_mode: 0, fall_speed: 0.0, grapple: None,
            icon_ctx: 0xff,
            cel: None,
            prev_pos: pos,
            creep: None,
        }
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

    /// The capsule ends (top A, bottom B) for the current stance, off the dynamic root height `cc`
    /// (`Player_Collision` 0x194df0: standing `0.275` / `0.55 − cc`; crouched `0.65 − cc` / `0.45 − cc`).
    /// The swim substate (3) builds its own pair [M 0x194df0 case 3:228-253]: spheres at
    /// `0.1 − (cc − 0.55)` = `0.65 − cc` and `0.45 − cc` with `+0x7cc` (the radius slot) 0.55 — a
    /// low-slung 0.2-long spine hanging under the root, so at the swim equilibrium (root = surface − 0.3)
    /// the capsule's top stays ~0.13 under the water plane and never touches a pond floor that sits at
    /// the surface (the 07000002 garden pond [D probe_vent_swim]).
    fn capsule(&self) -> (V3, V3) {
        let (a, b) = if self.substate == SS_SWIM {
            (0.65 - self.cc, 0.45 - self.cc)
        } else if self.crouched {
            (0.65 - self.cc, 0.45 - self.cc)
        } else {
            (0.275, 0.55 - self.cc)
        };
        ([self.pos[0], self.pos[1] + a, self.pos[2]], [self.pos[0], self.pos[1] + b, self.pos[2]])
    }

    /// The capsule ends for the current stance, public for the Sim's wire grab scan (the game's
    /// `Player_Collision` 0x194df0 builds the same segment as the HITTEST).
    pub fn capsule_seg(&self) -> (V3, V3) { self.capsule() }

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
            SS_GRAPPLE => self.grapple_tick(&input),
            SS_SWIM => self.swim_tick(world, ctx, input),
            SS_WIRE => self.wire_tick(world, ctx, input),
            SS_CREEP => self.creep_tick(world, ctx, input),
            SS_ZIPLINE => self.zipline_tick(world, ctx, input),
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
        self.physics_tick(world, ctx, input, false);
        self.ladder_grab(ctx);
    }

    /// `Player_SSGrapple` 0x192860 through `gadget::grapple_tick`: the reel toward the grapple
    /// point, the hang, and the fire's release (the substate 0; the swing momentum's restore is
    /// [S] — the release just drops Bond for now). The reel ignores gravity [M the substate body
    /// writes obj+0x30 directly].
    fn grapple_tick(&mut self, input: &Input) {
        let Some(st) = self.grapple.as_mut() else { self.substate = SS_WALK; return };
        let mut pos = self.pos;
        let fire = input.fire;
        let released = crate::gadget::grapple_tick(&mut pos, st, fire);
        self.pos = pos;
        self.vel = [0.0; 3];
        self.grounded = false;
        if released {
            self.grapple = None;
            self.substate = SS_WALK;
        }
    }

    /// `Player_Update` case 3 [M 0x193958 case 3]: `Player_Move(1.0, …)` — the walk physics with the
    /// multiplier 1.0 — then the inline swim tail: with a cel, Bond's y-velocity is clamped so he cannot
    /// rise above `cel+0x90 − 0.3`, and he leaves the water when grounded and `cel+0x90 <= pos.y − 0.3`
    /// (`BLData+0x130 = 2`, `ChangeSubState(0)`). `Player_Collision` 0x194df0 applies **no gravity** while
    /// Bond's y is below the water surface [M the default case jumps past `WldGravity` to the substate
    /// capsule switch when `cel+0x90 > obj+0x34`].
    fn swim_tick(&mut self, world: &CollisionWorld, ctx: &LadderCtx, input: &Input) {
        self.physics_tick(world, ctx, input, true);
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
            _ => { self.substate = SS_WALK; self.ladder = None; self.physics_tick(world, ctx, input, false); return; }
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
        // The pass is the actor's own cel's object lists (fix-08-collision): `Player_Collision`
        // 0x194df0 stores obj+0x20 (the relinked cel) into the BLData+0x740 HITTEST's start-cel
        // field (`*(BLData+0x7c0) = *(obj+0x20)`, the function's first store), and `Collide_Pick`
        // 0x1eb320 walks `Collide_StraddleCels` 0x1eac98's result, testing each straddled cel's
        // OWN list (`*(cel + 0x18)`) — walls hanging on other rooms' cels never block. `self.cel`
        // is that relink (obj+0x20; the Sim refreshes it off the position each tick, the
        // `build_LinkToRoom` 0x1c9680 order); `None` falls back to the flat world.
        let (push, walkable) = world.capsule_push_scoped(self.cel, a, b, RADIUS);
        self.head_bump = push[1] < -1e-4;
        for k in 0..3 { self.pos[k] += push[k]; }
        let ray = self.cc + 0.6;
        // The floor test reads the same HITTEST (`Player_CollisionHandler` 0x1949a8 →
        // `Player_FeetOnPoint(param_1, BLData+0x740, …)`), so it shares the per-cel scope.
        let floor = world.segment_hit_scoped(self.cel, self.pos, [self.pos[0], self.pos[1] - ray, self.pos[2]]);
        self.grounded = walkable || floor.is_some_and(|(t, _)| t * ray - 0.1 <= self.cc);
        if let Some((_, n)) = floor { self.floor_ny = n[1]; } else if walkable { self.floor_ny = 1.0; }
    }

    /// `Player_ClearInertia` 0x195e08 [M]: the four velocity triples (`BLData+0x10/+0x20/+0x30/+0x40`)
    /// zero. The port models +0x10 as `vel` and +0x14's push as `push_fwd`; `local` is +0x28's pair.
    fn clear_inertia(&mut self) {
        self.vel = [0.0; 3];
        self.local = [0.0; 2];
        self.push_fwd = 0.0;
    }

    /// `Player_CollWire` 0x197130 — the grab. Guards [M 0x19716c-0x1971a8]: the regrab lockout
    /// `+0x951 == 0`, health `+0x894 > 0` (the caller checks — the port keeps health in the Sim, like
    /// the ladder grab), wire type ≠ 1, and the substate ≠ the type's target (6 for type 0, 0xF for
    /// type 2). Builds the grab segment (`radius − 2.0` at the object's pitch), the closest point to
    /// Bond; a zipline refuses grabs within half the segment of the LOWER end. Then `Player_ClearInertia`,
    /// `ChangeSubState(6 or 0xF)`, mode `+0x94D` = 6 (type 0 → 0xC, the grab pose, with anim
    /// 0x060005e5; type 2 → anim 0x06000095), `+0x130 = 2` (the port does not model +0x130), the
    /// auto-drop timer zeroed, the yaw = the nearer of `wire.yaw ± π/2` to Bond's old yaw (type 2 picks
    /// by the segment's slope instead), the hang offset 1.0207 (type 0) / 1.17363 (type 2) below the
    /// closest point, the velocity slots zeroed, `Player_WeaponNone` (the Sim holsters — the caller),
    /// `SetCamMode(0xB)`, and the hand-reach correction: `pos −= R(yaw)·(root(0x060005e5 @last) −
    /// root(@0))` with x −= 0.082 [M 0x1974b8-0x197538, disasm 0x1975b0-0x197618; the track values come
    /// from `WireAnims`]. The caller has already resolved `wi` (the collision dispatch, the jump grab
    /// or the use key).
    pub fn grab_wire(&mut self, wi: usize, ctx: &LadderCtx) {
        let Some(w) = ctx.wires.get(wi) else { return };
        if self.regrab != 0 { return; }
        if w.wire_type == 1 { return; }
        let target = if w.wire_type == 2 { SS_ZIPLINE } else { SS_WIRE };
        if self.substate == target { return; }
        let (a, b) = w.grab_segment();
        if w.wire_type == 2 {
            // A zipline refuses grabs within half the segment of the lower end [M 0x197260-0x19728c:
            // `if (Dist3D(pos, lower) < Dist3D(A, B) · 0.5) return`].
            let low = if a[1] < b[1] { a } else { b };
            if dist3_sq(self.pos, low) < 0.25 * dist3_sq(a, b) { return; }
        }
        let closest = Wire::closest_point(a, b, self.pos);
        self.clear_inertia();
        self.substate = target;
        self.wire = Some(wi);
        self.auto_drop = 0.0;
        // The yaw: the nearer of wire.yaw ± π/2 to Bond's old yaw [M 0x1973f8-0x197430; type 2 flips by
        // the slope — both live wires are type 0 [D placements probe]].
        let old = self.yaw;
        let mut new_yaw = w.yaw - std::f32::consts::FRAC_PI_2;
        if wrap_pi(new_yaw - old).abs() > std::f32::consts::FRAC_PI_2 { new_yaw += std::f32::consts::PI; }
        self.yaw = new_yaw;
        self.pos = [closest[0], closest[1], closest[2]];
        if w.wire_type == 2 {
            self.mode = 6;
            self.pos[1] -= 1.17363;
            self.wire_anim = 0x0600_0095;
        } else {
            self.mode = 0xC;
            self.pos[1] -= 1.0207;
            self.wire_anim = 0x0600_05e5;
        }
        self.wire_anim_time = 1.0;
        self.wire_anim_rate = 1.0;
        self.cam_mode = 0xB;
        // The hand-reach correction [M 0x1974b8-0x197538]: delta = root(last) − root(0) of the grab
        // anim's sequence, x −= 0.082, rotated into world by the NEW yaw and subtracted. The rotation's
        // exact MMI signs are reconstructed [S — disasm 0x1975b0-0x197618]; the magnitude ~0.46 along
        // the facing (the reach), erased by the grab anim's own root motion as it plays.
        if let Some(t) = ctx.wire_anims.and_then(|wa| wa.tracks.get(&0x0600_05e5)) {
            let e = t.at(t.frames as f32);
            let s = t.at(1.0);
            let (dx, dy, dz) = (e[0] - s[0] - 0.082, e[1] - s[1], e[2] - s[2]);
            let (sin, cos) = self.yaw.sin_cos();
            // world = R(yaw)·local (the game's row-vector convention), negated (pos −= …).
            self.pos[0] -= dx * cos + dz * sin;
            self.pos[1] -= dy;
            self.pos[2] -= -dx * sin + dz * cos;
        }
    }

    /// The wire anim state machine helper: advances the playing anim by `rate` frames, applies its root
    /// motion rotated by Bond's yaw (`AnimSeqTick`/`AnimFrameResolve`; the same convention as the
    /// drones' `root_motion`: world = l.x·(cos y, 0, −sin y) + l.z·(sin y, 0, cos y)), and reports
    /// whether the anim has reached its end (`AnimScriptIsStopped` — these scripts never loop [D]).
    /// Missing track or no anim data = instantly stopped with no motion [S fallback].
    fn wire_anim_advance(&mut self, ctx: &LadderCtx) -> bool {
        if self.wire_anim == 0 { return true; }
        let Some(track) = ctx.wire_anims.and_then(|wa| wa.tracks.get(&self.wire_anim)) else {
            return true;
        };
        let len = track.frames as f32;
        if self.wire_anim_time >= len { return true; }
        let prev = self.wire_anim_time.max(1.0);
        let next = (self.wire_anim_time + self.wire_anim_rate).min(len);
        let (a, b) = (track.at(prev), track.at(next));
        let (dx, dy, dz) = (b[0] - a[0], b[1] - a[1], b[2] - a[2]);
        let (sin, cos) = self.yaw.sin_cos();
        self.pos[0] += dx * cos + dz * sin;
        self.pos[1] += dy;
        self.pos[2] += -dx * sin + dz * cos;
        self.wire_anim_time = next;
        self.wire_anim_time >= len
    }

    /// Adds a wire anim (`AnimScriptAdd` + the sway the decompile applies per add): snaps Bond onto
    /// `closest` at hang height with the ±0.0542 hand sway (perpendicular to the facing: the shift is
    /// `±0.0542·(cos yaw, −sin yaw)` [M the PS2Sinf2(−yaw+π/2, −yaw) pairs; the per-add signs follow
    /// the decompile's tree]) and sets `oldhash`/time/rate.
    fn wire_anim_add(&mut self, hash: u32, rate: f32, sway_sign: f32, closest: V3, hang: f32) {
        let (sin, cos) = self.yaw.sin_cos();
        self.pos[0] = closest[0] + sway_sign * 0.0542 * cos;
        self.pos[1] = closest[1] - hang;
        self.pos[2] = closest[2] - sway_sign * 0.0542 * sin;
        self.wire_anim = hash;
        self.wire_anim_time = 1.0;
        self.wire_anim_rate = rate;
    }

    /// `Player_Wire` 0x197658 (substate 6) — the hand-over-hand crossing, ported in the decompile's
    /// order. `Music_Event(0xD, 1)` every tick and the switch-channel writes are the Sim's (they cannot
    /// reach `Bond`), keyed off the substate.
    fn wire_tick(&mut self, world: &CollisionWorld, ctx: &LadderCtx, input: &Input) {
        let wi = match self.wire {
            Some(wi) if ctx.wires.get(wi).is_some() => wi,
            _ => { self.substate = SS_WALK; self.wire = None; self.physics_tick(world, ctx, input, false); return; }
        };
        let wire = ctx.wires[wi].clone();
        let hang = 1.0207f32;
        // 1. `Vec_Zero(BLData+0x50)` — the world velocity zeroes every tick; the anim root motion moves
        //    Bond instead [M 0x197668].
        self.vel = [0.0; 3];
        // 2. The travel segment (radius − 0.5, pitch forced 0) [M 0x1976dc-0x197730]. Bond's x/z snaps
        //    to an end when he is beyond it, then the closest point recomputes.
        let (a, b) = wire.travel_segment();
        let seg2 = dist2_sq(a, b);
        let d_a_pre = dist2_sq(a, self.pos);
        let d_b_pre = dist2_sq(b, self.pos);
        if seg2 < d_a_pre { self.pos[0] = b[0]; self.pos[2] = b[2]; }
        if seg2 < d_b_pre { self.pos[0] = a[0]; self.pos[2] = a[2]; }
        let closest = Wire::closest_point(a, b, self.pos);
        let d_a = dist2_sq(a, self.pos);
        let d_b = dist2_sq(b, self.pos);
        // 3. The auto-drop timer [M 0x197748-0x197754]: paused while the grab pose (mode 0xC) plays;
        //    100/(FRAME_RATE·30) per tick at the fixed 60 Hz = 1800 ticks to the drop.
        if self.mode != 0xC { self.auto_drop += 100.0 / (60.0 * 30.0); }
        // 4. The drop paths [M 0x197758-0x197834]: the action press (jump, action 7) or the timer past
        //    100 — `+0x942 = substate`, `ChangeSubState(0)`, `SetCamMode(0)`, the timer zeroed,
        //    `+0x951 = 0x1E` (the 30-frame regrab lockout), `AnimObj[99] = AnimObj[100]` (the Sim's
        //    re-equip carry) and the channel 0 (the Sim).
        if input.jump || self.auto_drop > 100.0 {
            self.saved_sub = SS_WIRE as u16;
            self.substate = SS_WALK;
            self.cam_mode = 0;
            self.auto_drop = 0.0;
            self.regrab = 0x1e;
            self.wire = None;
            self.wire_anim = 0;
            return;
        }
        // 5. The strafe axis (`Input_Actionf(slot, 1, 1)`) [M 0x1977c0-0x197834]: mode 7 for positive
        //    (toward B), 8 for negative (toward A), 6 idle — zeroed within 0.5² of the travelled-to end
        //    or past the far one.
        let mut strafe = input.strafe.clamp(-1.0, 1.0);
        if (strafe < 0.0 && (d_a <= 0.5 || seg2 < d_b_pre)) || (strafe > 0.0 && (d_b <= 0.5 || seg2 < d_a_pre)) {
            strafe = 0.0;
        }
        let next_mode = if strafe > 0.0 { 7u8 } else if strafe < 0.0 { 8 } else { 6 };
        // 6. The anim machine. The playback (root motion) runs first: while the current anim plays,
        //    the branches below early-return exactly like the decompile (`AnimScriptIsStopped` gates).
        let stopped = self.wire_anim_advance(ctx);
        if self.mode == 0xC && !stopped {
            // The grab pose still playing: wait (the timer is paused above) [M 0x197838-0x197860].
            return;
        }
        if self.mode == 6 && next_mode == 6 {
            if !stopped { return; }
            // The idle pair alternates [M 0x197870-0x1978d0]: after the grab pose / 5ec / b4 / 5ee the
            // next idle is 5ec, otherwise 5ed. The idle adds return early — no rate change (1.0).
            let hash = if matches!(self.wire_anim, 0x0600_05e5 | 0x0600_05ec | 0x0600_00b4 | 0x0600_05ee) {
                0x0600_05ec
            } else {
                0x0600_05ed
            };
            let sign = if hash == 0x0600_05ec { 1.0 } else { -1.0 };
            self.wire_anim_add(hash, 1.0, sign, closest, hang);
            return;
        }
        // The move/turn tree [M 0x1978e0-0x1979d0]. The decompile gates on `AnimScriptIsStopped`
        // ONLY when the mode byte is NOT 6 — an idle (mode 6) with move input falls straight into the
        // move tree (0x197838's `if (+0x94D == 6)` only wraps the idle branch), so a strafe press
        // during the idle starts the move immediately instead of waiting out the idle cycle.
        if self.mode != 6 && !stopped { return; }
        if next_mode >= 7 {
            let target = if next_mode == 7 { wire.yaw + 3.0 * std::f32::consts::FRAC_PI_2 } else { wire.yaw + std::f32::consts::FRAC_PI_2 };
            let diff = wrap_pi(target - self.yaw);
            if diff.abs() <= 0.0002 || matches!(self.wire_anim, 0x0600_05ee | 0x0600_05ea) {
                // Facing along the wire: the move pair alternates (b4 after b5/5ed, else b5).
                self.yaw = target;
                let (hash, sign) = if self.wire_anim == 0x0600_00b5 || self.wire_anim == 0x0600_05ed {
                    (0x0600_00b4, -1.0)
                } else {
                    (0x0600_00b5, 1.0)
                };
                self.wire_anim_add(hash, 1.2, sign, closest, hang);
            } else if self.wire_anim == 0x0600_00b5 || self.wire_anim == 0x0600_05ed {
                self.wire_anim_add(0x0600_05ea, 1.2, -1.0, closest, hang);
            } else {
                self.wire_anim_add(0x0600_05ee, 1.2, 1.0, closest, hang);
            }
        } else if matches!(self.mode, 7 | 8) && matches!(self.wire_anim, 0x0600_05ee | 0x0600_05ea) {
            // A turn just finished and the input is gone: the yaw fix-up lands on the opposite facing
            // and the mode byte takes 0xA ("turn") before settling to idle next tick [M 0x1979d8-0x197a30].
            let d = wrap_pi(self.yaw - (wire.yaw + std::f32::consts::FRAC_PI_2));
            let base = if d.abs() < 0.0002 { wire.yaw + std::f32::consts::PI } else { wire.yaw };
            self.yaw = base + std::f32::consts::FRAC_PI_2;
            self.mode = 0xA;
            return;
        }
        self.mode = next_mode;
    }

    /// The crawl entry, `Player_CollCreepWall` 0x196758 (the caller resolved the wall — the game's
    /// `Player_Activate` 0x1a5170:66-97 picks the nearest class-'+' object within 1000 when the
    /// third-icon context is 6). Guards [M 0x196758:73-74]: regrab `+0x951 == 0`, health `+0x894 > 0`
    /// (the port's dead Bond gets no input — the caller), and the substate check `1 < (u16)(f6 − 6)`
    /// — the wire (6) and the crawl (7) refuse, everything else passes. Entry [M 0x196758:76-128]:
    /// Bond's xz snaps to the wall's axis (the closest point of pos ± fwd(yaw+π/2)·len), the velocity
    /// slots zero, the floor ray (the game's request 0x62d, from closest.y + 5 down to − 5) puts
    /// `pos.y = hit + 1.0`, `Player_WeaponNone` (the Sim holsters — the caller watches the substate),
    /// `Player_ClearInertia`, `ChangeSubState(7)`, `+0x94D = 0` (the crawl mode byte), cam mode 0xB,
    /// the crawl anim script 0x6000476 (the port renders no body anims here) and `BLData+0x884 = wall`.
    pub fn start_creep(&mut self, wi: usize, world: &CollisionWorld, ctx: &LadderCtx) {
        let Some(w) = ctx.creepwalls.get(wi) else { return };
        if self.regrab != 0 { return; }
        if self.substate == SS_WIRE || self.substate == SS_CREEP { return; }
        let closest = w.closest_point(self.pos);
        self.vel = [0.0; 3];
        self.pos = [closest[0], closest[1], closest[2]];
        // The floor under the wall line: the game's vertical ray from closest.y + 5 [M 0x196758:94-112].
        if let Some(floor) = world.floor_below([closest[0], closest[1] + 4.5, closest[2]], 10.0) {
            self.pos[1] = floor + 1.0;
        }
        self.yaw = w.yaw + std::f32::consts::PI; // [M 0x196758:80 + 102: wall.yaw + π/2 + π/2]
        self.clear_inertia();
        self.substate = SS_CREEP;
        self.mode = 0;
        self.cam_mode = 0xB;
        self.creep = Some(wi);
    }

    /// `Player_Creep` 0x1969d0 (substate 7) [M], with [S] the crawl motion: the game drives Bond along
    /// the wall through the crawl anim scripts (0x600046f/0x6000471/0x6000475 — the port runs no body
    /// anim scripts), so the port moves him along the axis at 0.02/tick (1.2 u/s) with the input.
    /// Every tick: the xz snaps to the closest axis point + fwd(yaw)·0.275 [M 0x1969d0:99-125], the
    /// yaw = wall.yaw + π; the 2D distances to the two ends pick the near end (iVar10) and the exits:
    /// within 1.0 of an end WITH the input toward it → `+0x942 = 7`, `ChangeSubState(0)`, cam mode 0,
    /// the exit yaw = crawl yaw ∓ π/2 (toward the open side of that end), regrab = 0x78 (120),
    /// `AnimObj[99] = [100]` (the Sim's weapon carry, as the ladder dismount), `+0x18 += 0.05` (a
    /// forward push out) and the channel cleared [M 0x1969d0:134-156]. The music event (0xD,1) and the
    /// channel set ride the Sim's tick (the port keeps `switch_channels` in the drones).
    fn creep_tick(&mut self, world: &CollisionWorld, ctx: &LadderCtx, input: &Input) {
        let Some(wi) = self.creep else { self.substate = SS_WALK; return };
        let Some(w) = ctx.creepwalls.get(wi) else { self.substate = SS_WALK; self.creep = None; return };
        let crawl_yaw = w.yaw + std::f32::consts::PI;
        self.yaw = crawl_yaw;
        let closest = w.closest_point(self.pos);
        let (s, c) = crawl_yaw.sin_cos();
        self.pos = [closest[0] + s * 0.275, self.pos[1], closest[2] + c * 0.275];
        // The ends: A = pos + fwd(yaw+π/2)·len, B = pos − fwd(yaw+π/2)·len [M 0x1969d0:91-105].
        let (a, b) = w.endpoints();
        let d2 = |p: V3, q: V3| (p[0] - q[0]) * (p[0] - q[0]) + (p[2] - q[2]) * (p[2] - q[2]);
        let (d_a, d_b) = (d2(self.pos, a).sqrt(), d2(self.pos, b).sqrt());
        let fwd = input.forward.clamp(-1.0, 1.0);
        let pushing = if fwd.abs() < 0.5 { 0u8 } else if fwd > 0.0 { 2 } else { 1 };
        let at_a = d_a < 1.0;
        let at_b = d_b < 1.0;
        let (exit, nearer_a) = if at_a && pushing == 2 {
            (true, d_a < d_b)
        } else if at_b && pushing == 1 {
            (true, d_a < d_b)
        } else {
            (false, true)
        };
        if exit {
            // [M 0x1969d0:134-156] the shared exit tail; the yaw turn: nearer the A end → −π/2.
            self.saved_sub = SS_CREEP as u16;
            self.substate = SS_WALK;
            self.creep = None;
            self.cam_mode = 0;
            self.yaw = crawl_yaw + if nearer_a { -std::f32::consts::FRAC_PI_2 } else { std::f32::consts::FRAC_PI_2 };
            self.regrab = 0x78;
            self.push_fwd += 0.05;
            return;
        }
        // [S] the crawl motion along the axis (the game's anim scripts carry the displacement).
        if pushing != 0 {
            let dir = if pushing == 2 { 1.0 } else { -1.0 };
            let (ax_s, ax_c) = (w.yaw + std::f32::consts::FRAC_PI_2).sin_cos();
            self.pos[0] += ax_s * 0.02 * dir;
            self.pos[2] += ax_c * 0.02 * dir;
            self.mode = pushing;
        } else {
            self.mode = 0;
        }
        let _ = world;
    }

    /// `Player_Zipline` 0x197f50 (substate 0xF, wire type 2) [M]. Mode 9 = the arrival pose: when its
    /// anim stops → `+0x942 = substate`, `ChangeSubState(0)`, `SetCamMode(0)`, regrab = (u8)FRAME_RATE
    /// (60), `AnimObj[99] = [100]` (the Sim's carry) and the channel 0. Otherwise: the ride anim
    /// 0x06000096 re-added whenever stopped, the FULL-radius pitch segment, Bond's y lifted 1.17363 for
    /// the lower-end test — within 1.0 of it → mode 9 + anim 0x06000094 — then pos = the closest point
    /// at `closest.y − 1.17363` and the fall-speed accumulator `BLData+0x18 += FRAME_RATE_MUL·0.15`
    /// (the port integrates it as a downward velocity: the game's `Player_Move` applies +0x18 [S — the
    /// atlas's BLData table]; the clamp projects the fall onto the line, so Bond slides down a sloped
    /// zipline and accelerates).
    fn zipline_tick(&mut self, world: &CollisionWorld, ctx: &LadderCtx, _input: &Input) {
        let wi = match self.wire {
            Some(wi) if ctx.wires.get(wi).is_some() => wi,
            _ => { self.substate = SS_WALK; self.wire = None; self.physics_tick(world, ctx, _input, false); return; }
        };
        let wire = ctx.wires[wi].clone();
        if self.mode == 9 {
            if self.wire_anim_advance(ctx) {
                self.saved_sub = SS_ZIPLINE as u16;
                self.substate = SS_WALK;
                self.cam_mode = 0;
                self.regrab = 60;
                self.wire = None;
                self.wire_anim = 0;
            }
            return;
        }
        if self.wire_anim_advance(ctx) && self.wire_anim != 0x0600_0094 {
            self.wire_anim_add(0x0600_0096, 1.0, 0.0, self.pos, 0.0);
        }
        let (a, b) = wire.zipline_segment();
        // The lower-end test runs on the LIFTED position (the decompile lifts Bond's y onto the wire
        // first [M 0x197ff8: `pos.y += 1.17363`], checks `Dist3D(pos, lower) < 1.0`, and only then
        // re-projects and hangs him back 1.17363 below the line).
        let lifted = [self.pos[0], self.pos[1] + 1.17363, self.pos[2]];
        let low = if a[1] < b[1] { a } else { b };
        if dist3_sq(lifted, low) < 1.0 {
            self.mode = 9;
            self.wire_anim_add(0x0600_0094, 1.0, 0.0, self.pos, 0.0);
            return;
        }
        // The fall-speed accumulator `BLData+0x18` [M 0x19804c-0x198054: += FRAME_RATE_MUL·0.15 per
        // tick; the game's `Player_Move` applies it [S — the atlas's BLData table]]. The port applies it
        // as this tick's downward velocity; the clamp below projects the fall onto the line, so Bond
        // slides down a sloped zipline and accelerates.
        self.fall_speed += 0.15;
        self.pos[1] -= self.fall_speed / 60.0;
        let lifted = [self.pos[0], self.pos[1] + 1.17363, self.pos[2]];
        let closest = Wire::closest_point(a, b, lifted);
        self.pos = [closest[0], closest[1] - 1.17363, closest[2]];
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
    fn physics_tick(&mut self, world: &CollisionWorld, ctx: &LadderCtx, input: &Input, swimming: bool) {
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
            self.cc = CROUCH_ROOT_HEIGHT;
            self.lockout = 60;
        } else if self.crouched && (input.crouch || input.jump) && self.lockout == 0 && !self.head_bump && headroom(world) {
            self.crouched = false;
            self.cc = ROOT_HEIGHT;
            self.lockout = 60;
        } else if !self.crouched && self.jump == 0 && input.jump && self.grounded_history != 0 && !self.head_bump && headroom(world) {
            // `Player_HandleJump` 0x1916b0:118-147 — the jump grab: with the third-icon context 3 (the
            // wire icon, `BLData+0x95F`), the press scans the level's wires for the NEAREST class-0x2C
            // within 1000 units (no capsule test) and grabs it instead of jumping [M — the game's scan
            // is `control_first_object` over class ',' with `Vec_Dist3D`]. A refused grab (the regrab
            // lockout inside `grab_wire`) still eats the jump — the game returns from HandleJump
            // regardless [M 0x1916b0:146-149]. No context (0xFF) or other types → the plain jump.
            if self.icon_ctx == 3 {
                if let Some(wi) = ctx.wires.iter().enumerate()
                    .filter(|(_, w)| dist3_sq(w.centre, self.pos) < 1000.0 * 1000.0)
                    .min_by(|(_, a), (_, b)| dist3_sq(a.centre, self.pos).partial_cmp(&dist3_sq(b.centre, self.pos)).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(i, _)| i) {
                    self.grab_wire(wi, ctx);
                    return;
                }
            }
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
        // `Player_CollisionHandler`: the floor ray first, then the capsule's push. Both read the
        // one BLData+0x740 HITTEST, whose start cel is the actor's own obj+0x20 [M
        // `Player_Collision` 0x194df0's first store `*(BLData+0x7c0) = *(obj+0x20)`;
        // `Collide_Pick` 0x1eb320 tests each `Collide_StraddleCels` cel's own +0x18 list] — the
        // per-cel attribution (fix-08-collision): walls hanging on other rooms' cels never block.
        // `self.cel` is the Sim's per-tick relink (obj+0x20); `None` falls back to the flat world.
        let (a, b) = self.capsule();
        let (mut push, mut walkable) = world.capsule_push_scoped(self.cel, a, b, RADIUS);
        let ray = self.cc + 0.6;
        let mut floor = world.segment_hit_scoped(self.cel, self.pos, [self.pos[0], self.pos[1] - ray, self.pos[2]]);
        // The game's pass never lands over a void: `build_FindCel` 0x1c8cb0 relinks obj+0x20 by
        // tie-break rays against each candidate's own chain geometry, so the actor's cel always
        // carries the geometry under him. The port's buckets are coarser than the game's cels [S]:
        // over 07000002's ladder head the scoped pass sees no floor at all (the garden-floor rim
        // sits in a bucket the straddle walk doesn't keep) and Bond fell down the shaft he had
        // just climbed (fix_ladder_live); on 07000009 the car-park stairwell steps hung outside
        // the straddle set too (walkreal_07000009_live) — a wall push in the set does not put a
        // floor under Bond, so the redo fires on the missing floor alone. On the ground the
        // scoped pass always finds the room's own floor, so the redo never fires there: across
        // the fix-08-collision slab band the floor stays in the room's bucket while the exterior
        // wall stays out of it.
        if floor.is_none() {
            let (fp, fw) = world.capsule_push(a, b, RADIUS);
            push = fp;
            walkable = fw;
            floor = world.segment_hit(self.pos, [self.pos[0], self.pos[1] - ray, self.pos[2]]);
        }
        self.grounded = walkable || floor.is_some_and(|(t, _)| t * ray - 0.1 <= self.cc);
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

/// `Vec_SqDist2D` 0x1e2ef8 [M]: the x/z squared distance (the wire clamp is 2D).
fn dist2_sq(a: V3, b: V3) -> f32 {
    let (dx, dz) = (a[0] - b[0], a[2] - b[2]);
    dx * dx + dz * dz
}

/// `Vec_Dist3D` squared [M 0x1e2e58's pattern].
fn dist3_sq(a: V3, b: V3) -> f32 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    d[0] * d[0] + d[1] * d[1] + d[2] * d[2]
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
    /// The mission layer's globals (`MissionState` 0x30cb9c, `BaseMap` 0x30cba0, `ThisOrderNum`
    /// 0x30cba4, `FailLabel` 0x30cba8 and the per-objective states). The game keeps them in `.data`
    /// across `ResetMap_LevelToLoad` — only `Mission_Init` 0x182270 touches them, and it resets the
    /// current and future parts IN PLACE, leaving previous parts alone [M 0x182270:68-129]. The port
    /// re-creates `Mission` per load, so the carry rides the RamSave (the payload shape is
    /// `LS_MakeMission`/`LS_LoadMission` 0x1cd720/0x1cd880; map-tower2).
    pub mission: Option<crate::mission::MissionSave>,
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
            mission: None,
        }
    }

    /// Attaches the mission globals snapshot — the game persists them in `.data` across a level load
    /// while the port re-creates `Mission` (map-tower2; see the `mission` field).
    pub fn with_mission(mut self, m: &crate::mission::Mission) -> RamSave {
        self.mission = Some(m.snapshot());
        self
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
        // [M 0x1a7ef8 case 0x7000009]: the Nightfire loadout — `Upgrade_Weapon(6)` (the handgun,
        // 48), `Upgrade_Weapon(0x4a)` (the Stunner, 999), `Upgrade_Weapon(0x56)` (the Decryptor, 0),
        // **0x58 (the Q-Worm, 0)**, 0x50 (the Q-Claw, 0), `Upgrade_Weapon(0x43)` (the dart gun, 999)
        // and 0x5b (the Shaver, 999), selecting the dart gun. The upgrade level is the boot `Upgrades`
        // (0), so every `Upgrade_Weapon` is its base record here.
        0x0700_0009 => {
            init_ammo_weapons(inv, w, loaded);
            let up = |id: u16| crate::pickup::Inventory::upgrade_weapon(id, 0, 0);
            equip(inv, w, up(6), 0x30, loaded);
            equip(inv, w, up(0x4a), 999, loaded);
            equip(inv, w, up(0x56), 0, loaded);
            equip(inv, w, 0x58, 0, loaded);
            equip(inv, w, 0x50, 0, loaded);
            equip(inv, w, up(0x43), 999, loaded);
            equip(inv, w, 0x5b, 999, loaded);
            req = up(0x43) as u8;
        }
        // [M 0x1a7ef8 cases 0x700000a/0x700000b]: `Player_RamLoad` keeps the carry and re-selects the
        // saved weapon; when the handgun record is unowned the 0x0700009 loadout re-runs (the island's
        // Q-Worm arrives here — the user's "no Q-Worm" symptom had no grant at all).
        0x0700_000a | 0x0700_000b => {
            req = saved_weapon;
            if !p2k_owned(inv) {
                init_ammo_weapons(inv, w, loaded);
                let up = |id: u16| crate::pickup::Inventory::upgrade_weapon(id, 0, 0);
                equip(inv, w, up(6), 0x30, loaded);
                equip(inv, w, up(0x4a), 999, loaded);
                equip(inv, w, up(0x56), 0, loaded);
                equip(inv, w, 0x58, 0, loaded);
                equip(inv, w, 0x50, 0, loaded);
                equip(inv, w, up(0x43), 999, loaded);
                equip(inv, w, 0x5b, 999, loaded);
                req = up(0x43) as u8;
            }
        }
        0x0700_0008 => {
            req = saved_weapon;
            if !p2k_owned(inv) { party(inv, health); req = 1; }
        }
        // The power station [M 0x1a7ef8 cases 0x700000c/0x700000d]: a cold 0x070000c runs
        // `Player_InitAmmoWeapons` then equips the handgun (2, 48), `Upgrade_Weapon(0x1e)` (the
        // sniper, 999), `Upgrade_Weapon(0x4a)` (the Q-Claw, 999) and the gadgets 0x50/0x54/0x58 at 0,
        // selecting the sniper (`+0x63 = uVar3`). 0x070000d is the 06/08 shape: `Player_RamLoad`
        // keeps the carry and re-selects the saved weapon; when the sniper is missing the 0c
        // loadout re-runs.
        0x0700_000c => {
            init_ammo_weapons(inv, w, loaded);
            equip(inv, w, HANDGUN, 0x30, loaded);
            equip(inv, w, crate::pickup::Inventory::upgrade_weapon(0x1e, 0, 0), 999, loaded);
            equip(inv, w, crate::pickup::Inventory::upgrade_weapon(0x4a, 0, 0), 999, loaded);
            equip(inv, w, 0x50, 0, loaded);
            equip(inv, w, 0x54, 0, loaded);
            equip(inv, w, 0x58, 0, loaded);
            req = crate::pickup::Inventory::upgrade_weapon(0x1e, 0, 0) as u8;
        }
        0x0700_000d => {
            req = saved_weapon;
            let sniper = crate::pickup::Inventory::upgrade_weapon(0x1e, 0, 0);
            if !inv.owned.get(sniper as usize).copied().unwrap_or(false) {
                init_ammo_weapons(inv, w, loaded);
                equip(inv, w, HANDGUN, 0x30, loaded);
                equip(inv, w, sniper, 999, loaded);
                equip(inv, w, crate::pickup::Inventory::upgrade_weapon(0x4a, 0, 0), 999, loaded);
                equip(inv, w, 0x50, 0, loaded);
                equip(inv, w, 0x54, 0, loaded);
                equip(inv, w, 0x58, 0, loaded);
                req = sniper as u8;
            }
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
        let ctx = LadderCtx { ladders: &[lad.clone()], wires: &[], wire_anims: None, water: None, sink: false, creepwalls: &[] };
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
        // All quads wound with the stored plane normal facing the occupant (floors UP — the disc's
        // convention, 7,246 up / 0 down; the wall −z at the walker; `Intersect_RayGeom` 0x10a8c8
        // skips dot(n,dir) >= 0).
        t.extend(q([-20.0, 0.0, -20.0], [-20.0, 0.0, 20.0], [20.0, 0.0, 20.0], [20.0, 0.0, -20.0]));
        // A wall at z = 5 and a 0.3 step at x ≥ 3 (top at y 0.3).
        t.extend(q([-20.0, 0.0, 5.0], [-20.0, 4.0, 5.0], [20.0, 4.0, 5.0], [20.0, 0.0, 5.0]));
        t.extend(q([3.0, 0.3, -20.0], [3.0, 0.3, 4.0], [20.0, 0.3, 4.0], [20.0, 0.3, -20.0]));
        t.extend(q([3.0, 0.0, -20.0], [3.0, 0.0, 4.0], [3.0, 0.3, 4.0], [3.0, 0.3, -20.0]));
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
