//! The world presentation FX the sim previously skipped — the placement object types from
//! docs/systems/levels.md §7 that only drew (or did not exist) before: 0x21 Ripples, 0xe3 Flicker,
//! 0xd4 Cloud drift, 0x4a Apocalypse and 0xf2 Emitter (particles).
//!
//! Game model ([M] = decompile, function + address):
//! - **Ripples** (`Ripples_Create` 0x1acc90 / `Ripples_Update` 0x1ac028): data+0 = param 0 (the s16
//!   x/z spread), data+2 = param 1 (the s16 period); both 0 → spread 1, period 0x1e [M 0x1acc90:20-30].
//!   Every tick: `Rand_Rand(period) == 0` → spawn one `Gas_Init(start, start·2, cel, pos+jitter, …,
//!   mesh 0x02000129, …, rgba 0x7f7f7fff, …)` puff: two `Rand_FRand(2·spread) − spread` draws move the
//!   x/z, one `Rand_FRand(0.05) + 0.05` sets the start scale [M 0x1ac028:34-52].
//! - **Flicker** (`Flicker_Create` 0x1acb00 / `Flicker_Update` 0x1ab960): the tint bytes obj+0x103..105
//!   fade 10 % per tick (min ±1) toward 255,255,255 while the found map light is active and toward 0
//!   when not; `Light_FindMapLight` 0x1cc3d0 is an **8-byte stub returning 0** on this build [M], so
//!   the game always takes the white path and stamps the signal channel (param 1) 0 every tick. The
//!   spark branch (gas puff + `Particles_Create` when the light is off) is unreachable behind the same
//!   stub, and `Particles_Update` 0x1cf528 self-kills through the stubbed `psiParticleUpdate` 0x111290
//!   anyway [M] — the port keeps the white fade + the channel stamp and skips the dead spark path.
//! - **Cloud** (`Cloud_Create` 0x18d620 / `cloud_update` 0x18d768): data = [param1·0.001, param2·0.001,
//!   phase_x, phase_z, param0 (height), param3 (wrap)]. Per tick the phases wrap by fmod and the object
//!   parks at `player + (phase_x, height, phase_z)` — a sky-slot object that drifts with the player
//!   [M 0x18d768:20-31]. No RNG, no channels.
//! - **Apocalypse** (`Apocalypse_Create` 0x18dcc0 / `Apocalypse_Update` 0x18d4f0): data[0] = data[1] =
//!   param1·60 (the remaining/total countdown in ticks), +8 u16 = param 0 (the latch channel), +0xa =
//!   param 2 (rumble), +0xc byte = param 3 (whiteout), +0xd byte = param 4 (flashbang), +0xe byte =
//!   the armed latch. Once `switch_channels[param0]` the latch sets for good; the countdown then runs,
//!   the viewer's whiteout value `glb_viewer+0x200 = remaining·5/total` rises, the rumble/flashbang
//!   params feed their (unported) systems, and at 0 the object self-destructs [M 0x18d4f0].
//! - **Emitter** (`Emitter_CreatePlist` 0x17b848 → `Emitter_Create` 0x17ae00 / `Emitter_Update`
//!   0x17b080 / `Emitter_Draw` 0x17b708): param 0 = the def hash (block 0x30, `Emitter_LoadDefs`
//!   0x17ab40 — `nf_formats::level::EmitterDef`), param 1 = the rate (0 → −1 = emit until disabled,
//!   1 → 0 = one-shot, n → n/60 seconds of emitting), params 2/3/4 = the on/off/out channels (out is
//!   driven 1 while any particle lives), param 5 = the blink period in ticks (ON half of it). Slots
//!   age at `age += dt`, `pos += vel·dt`, `vel.y += dt·gravity·−9.8`; the colour is `ColourGetValue`
//!   0x17a938 over `age/life`; dead slots respawn (batch-capped) while enabled, one-shots only fire
//!   the never-spawned slots once `expired` [M 0x17b080]. Death: 3 all-dead ticks set obj+0xFE bit 0.
//!
//! RNG discipline: the game draws from the shared stream (ripples 1 gate + 3, emitters 4 per spawn,
//! the flicker spark's `Rand_Rand(5)`) — the port consumes NONE of them (the bit-exact stream must not
//! gain draws, the `gas.rs`/searchlight discipline) and derives each value from a per-object counter
//! hash instead [S]. Same visible behaviour, deterministic; the deviation is listed in
//! docs/atlas/emitter.md and docs/models/misc.md.

use crate::triggers::Channels;
use crate::world::V3;
use nf_formats::level::{EmitterDef, Level};

const REC_FRAME_RATE: f32 = 1.0 / 60.0;

/// The ripples' spawn mesh (`Gas_Init` arg 7 in `Ripples_Update` 0x1ac028:50; the 0x0200xxxx mesh
/// namespace, same as gas.rs's SMOKE_QUAD 0x02000039).
pub const RIPPLE_MESH: u32 = 0x0200_0129;

/// The counter-hash the RNG-discipline note describes (the same bit-mix gas.rs uses).
fn jitter(counter: u32, salt: u32) -> f32 {
    let mut h = counter ^ salt;
    h = h.wrapping_mul(0x9e37_79b9);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    (h & 0xffff) as f32 / 65536.0
}

/// One 0xe3 placement: the tint ramp and the (always-0) signal channel.
#[derive(Clone, Debug, PartialEq)]
pub struct FlickerFx {
    pub placement: usize,
    pub pos: V3,
    /// param 1: the channel `Flicker_Update` stamps 0 (1 only on the unreachable light-off path).
    pub signal_channel: u16,
    /// obj+0x103..105, started black (the object memory zeroes) and fading white.
    pub rgb: [u8; 3],
}

/// One 0x21 placement.
#[derive(Clone, Debug, PartialEq)]
pub struct RippleFx {
    pub placement: usize,
    pub pos: V3,
    /// param 0 (s16): the x/z jitter half-spread.
    pub spread: f32,
    /// param 1 (s16): the spawn period in ticks (0 with a non-0 spread keeps it).
    pub period: i32,
    counter: u32,
}

/// One 0xd4 placement.
#[derive(Clone, Debug, PartialEq)]
pub struct CloudFx {
    pub placement: usize,
    /// param 4: the `View_AddSkyObj` slot the sky pass orders by (levels.md sky_instances).
    pub slot: u32,
    /// param 1/2 × 0.001: the drift per tick.
    pub drift: [f32; 2],
    /// param 0: the height over the player.
    pub height: f32,
    /// param 3: the fmod wrap modulus of both phases.
    pub wrap: f32,
    /// data[2]/data[3]: the drift phases.
    pub phase: [f32; 2],
}

/// One 0x4a placement.
#[derive(Clone, Debug, PartialEq)]
pub struct ApocalypseFx {
    pub placement: usize,
    /// param 0: the latch channel.
    pub channel: u16,
    /// param 2: the rumble strength (the port has no rumble; carried for completeness).
    pub rumble: u16,
    /// param 3 / param 4: the whiteout / flashbang flags.
    pub whiteout: bool,
    pub flashbang: bool,
    /// data[0]/data[1]: the remaining/total countdown in ticks.
    pub remaining: f32,
    pub total: f32,
    /// data+0xe: latched once the channel fires.
    pub armed: bool,
    /// The viewer whiteout value `remaining·5/total` [M 0x18d4f0] the renderer would apply.
    pub fade: f32,
}

/// One emitter particle slot (the game's four parallel arrays collapsed).
#[derive(Clone, Debug, PartialEq)]
pub struct EmitterParticle {
    pub pos: V3,
    pub vel: V3,
    pub age: f32,
    pub life: f32,
    /// The 4-byte ARGB the draw reads (rgb at half range from the keys).
    pub rgba: [u8; 4],
}

/// One 0xf2 placement.
#[derive(Clone, Debug, PartialEq)]
pub struct EmitterFx {
    pub placement: usize,
    pub pos: V3,
    /// The placement rotation (the spawn direction's basis; the game rotates by obj+0x90).
    pub rotation: [f32; 3],
    /// param 0: the def hash (resolved against the level's block-0x30 defs).
    pub def: u32,
    /// param 1: 0 → −1 (emit while enabled), 1 → 0 (one-shot), n → n/60 s countdown.
    pub rate: f32,
    /// params 2/3/4: the on / off / out channels.
    pub ch_on: u16,
    pub ch_off: u16,
    pub ch_out: u16,
    /// param 5: the blink period in ticks (0 = steady).
    pub toggle_period: f32,
    toggle_timer: f32,
    toggle_state: bool,
    /// data+0x4b: the one-shot latch (rate counted out).
    pub expired: bool,
    /// data+0x49: the all-dead tick counter (3 → the object dies).
    dead_ticks: u8,
    pub parts: Vec<EmitterParticle>,
    counter: u32,
    /// G-041 (fix-impact-remainder): the `Effect_Create` 0x1ce688 impact arm's emitters consume
    /// the SHARED stream (`Emitter_Update` 0x17b080's 4 × `Rand_FRandHalf(1.0)` per spawn) —
    /// unlike the 0xf2 placements, whose counter-hash stand-in predates this task and stays.
    pub shared_rng: bool,
    /// The object basis `Mat_SetDir/SetNorm/SetUp` 0x1e3418/0x1e3458/0x1e3438 built from the hit
    /// normal (`Emitter_Create` 0x17ae00): rows [Norm = v, Up = normal, Dir = u] — the spawn's
    /// local +y rides the normal. `None` → the placement euler matrix (the 0xf2 path).
    pub basis: Option<[[f32; 4]; 4]>,
}

impl EmitterFx {
    fn enabled(&self, ch: &Channels) -> bool {
        if self.ch_on != 0 && !ch.on(self.ch_on as u32) { return false; }
        if self.ch_off != 0 && ch.on(self.ch_off as u32) { return false; }
        true
    }
}

/// The frame's particle billboards (`Emitter_Draw` path A: a size-`def.size` rgba sprite per alive
/// slot) — the render handoff, the way `Sim::snow_quads` exposes the snow.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleSprite {
    pub pos: V3,
    pub size: f32,
    pub rgba: [u8; 4],
    /// def+0x3c: 0 = normal alpha, else additive [M 0x17b708:15].
    pub additive: bool,
}

/// The whole FX set for one level (`WorldFx::attach_level`).
#[derive(Default)]
pub struct WorldFx {
    pub flickers: Vec<FlickerFx>,
    pub ripples: Vec<RippleFx>,
    pub clouds: Vec<CloudFx>,
    pub apocalypses: Vec<ApocalypseFx>,
    pub emitters: Vec<EmitterFx>,
    defs: std::collections::HashMap<u32, EmitterDef>,
}

impl WorldFx {
    /// Every FX placement of the main map, in placement order (the game creates the dynamic objects
    /// in the same `parsemap_create_dynamic_objects` 0x1d04f0 walk). The emitter defs merge from
    /// every map's block 0x30 (the game's hashtable is global).
    pub fn attach_level(level: &Level) -> WorldFx {
        let mut defs = std::collections::HashMap::new();
        for map in &level.maps {
            for d in &map.emitter_defs { defs.insert(d.hash, d.clone()); }
        }
        let mut fx = WorldFx { defs, ..Default::default() };
        let Some(m) = level.main_map() else { return fx };
        for (i, p) in level.maps[m].placements.iter().enumerate() {
            let param = |k: u32| p.params.iter().find(|(j, _)| *j == k).map(|&(_, v)| v).unwrap_or(0);
            let kind = p.flags & 0xff;
            let pos = p.translation;
            let _ = &mut fx;
            match kind {
                0x21 => {
                    // `Ripples_Create` 0x1acc90:20-30 — the s16 params, both-0 defaults.
                    let (spread, period) = (param(0) as u16 as i16, param(1) as u16 as i16);
                    let (spread, period) = if period == 0 && spread == 0 { (1.0, 0x1e) } else { (spread as f32, period as i32) };
                    fx.ripples.push(RippleFx { placement: i, pos, spread, period, counter: i as u32 });
                }
                0xe3 => {
                    // `Flicker_Create` 0x1acb00: data+4 = param 0 (the u16 spark gate — unreachable),
                    // data+0x20 = param 1 (the signal channel). The tint starts black.
                    fx.flickers.push(FlickerFx { placement: i, pos, signal_channel: param(1) as u16, rgb: [0; 3] });
                }
                0xd4 => {
                    // `Cloud_Create` 0x18d620:20-28 (the port's drift rates; the slot is param 4 —
                    // levels.rs sky_instances already draws the static placement).
                    fx.clouds.push(CloudFx {
                        placement: i, slot: param(4),
                        drift: [param(1) as f32 * 0.001, param(2) as f32 * 0.001],
                        height: param(0) as i32 as f32,
                        wrap: param(3) as i32 as f32,
                        phase: [0.0; 2],
                    });
                }
                0x4a => {
                    // `Apocalypse_Create` 0x18dcc0: data[0] = data[1] = param1·60, +8 = param 0,
                    // +0xa = param 2, +0xc = param 3, +0xd = param 4.
                    let total = param(1) as i32 as f32 * 60.0;
                    fx.apocalypses.push(ApocalypseFx {
                        placement: i, channel: param(0) as u16, rumble: param(2) as u16,
                        whiteout: param(3) != 0, flashbang: param(4) != 0,
                        remaining: total, total, armed: false, fade: 0.0,
                    });
                }
                0xf2 => {
                    // `Emitter_CreatePlist` 0x17b848: the rate from param 1 (0 → −1, 1 → 0, else /60);
                    // `Emitter_Create` 0x17ae00 inits every slot dead (age 100000, life 0), the
                    // expired byte = (rate == 0), the toggle state = 1, the channels from params 2-4.
                    // An unresolvable def means NO object (`Emitter_Create` 0x17ae00: hashtable_getitem
                    // 0 → returns 0) — the placement is skipped exactly like the game.
                    let Some(d) = fx.defs.get(&param(0)) else { continue; };
                    let rate = match param(1) as i32 { 0 => -1.0, 1 => 0.0, n => n as f32 * REC_FRAME_RATE };
                    fx.emitters.push(EmitterFx {
                        placement: i, pos, rotation: p.rotation,
                        def: param(0), rate,
                        ch_on: param(2) as u16, ch_off: param(3) as u16, ch_out: param(4) as u16,
                        toggle_period: param(5) as i32 as f32,
                        toggle_timer: 0.0, toggle_state: true,
                        expired: rate == 0.0, dead_ticks: 0,
                        parts: (0..d.max).map(|_| EmitterParticle {
                            pos: [0.0; 3], vel: [0.0; 3], age: 99999.0, life: 0.0, rgba: [0; 4],
                        }).collect(), counter: i as u32,
                        shared_rng: false, basis: None,
                    });
                }
                _ => {}
            }
        }
        fx
    }

    /// The `Effect_Create` 0x1ce688 impact arm's `Emitter_Create(0, 0, EffectInfo+0x84, hit_pos,
    /// normal, 0, 0, 0)` 0x17ae00: a def-hash lookup (a miss → NO object, the placement path's own
    /// rule), a ONE-SHOT emitter (the rate argument is 0 → the +0x4b latch), no channels (+0x4a =
    /// 1 from create, params 0 don't touch it), the basis from the hit normal, every slot dead.
    /// Consumes no RNG here — the burst's draws run on the object's own update ticks.
    pub fn spawn_impact(&mut self, def_hash: u32, pos: V3, normal: V3) {
        let Some(d) = self.defs.get(&def_hash) else { return };
        // `Vec_MakeOrthonormalBasis(normal)` → (u, v); Mat_SetDir(u) → row +0x20 (the spawn's
        // local z), Mat_SetNorm(v) → row +0x00 (local x), Mat_SetUp(normal) → row +0x10 (local
        // y) [M 0x1e3418/0x1e3458/0x1e3438]. The rows the velocity transform reads:
        // vel = Norm_row·x + Up_row·y + Dir_row·z [M 0x17b080 _vmulabc/_vmaddabc/_vmaddbc].
        let (u, v) = crate::effect::orthonormal_basis_pair(normal);
        let basis = [
            [v[0], v[1], v[2], 0.0],
            [normal[0], normal[1], normal[2], 0.0],
            [u[0], u[1], u[2], 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        self.emitters.push(EmitterFx {
            placement: usize::MAX, pos, rotation: [0.0; 3],
            def: def_hash, rate: 0.0, ch_on: 0, ch_off: 0, ch_out: 0,
            toggle_period: 0.0, toggle_timer: 0.0, toggle_state: true,
            expired: true, dead_ticks: 0,
            parts: (0..d.max).map(|_| EmitterParticle {
                pos: [0.0; 3], vel: [0.0; 3], age: 99999.0, life: 0.0, rgba: [0; 4],
            }).collect(),
            counter: 0, shared_rng: true, basis: Some(basis),
        });
    }

    /// The def table (the test's count predictor: the impact burst draws 4 × min(max, batch)).
    pub fn defs(&self) -> &std::collections::HashMap<u32, EmitterDef> { &self.defs }

    /// One 60 Hz tick (`Emitter_Update`/`Ripples_Update`/`cloud_update`/`Apocalypse_Update` run in
    /// the game's object handler; `Flicker_Update` likewise). `frame` = `GameState+0x52`. `rng` is
    /// the SHARED stream — consumed ONLY by the impact emitters (`shared_rng`); the 0xf2
    /// placements keep the counter-hash stand-in (the G-033/fix-world-fx discipline).
    pub fn tick(&mut self, ch: &mut Channels, player_pos: V3, frame: u32, gas: &mut crate::gas::GasPool, rng: &mut crate::drone::Rng) {
        // Flicker [M 0x1ab960]: the stubbed FindMapLight keeps the white target and the 0 stamp.
        for f in &mut self.flickers {
            for k in 0..3 {
                let gap = 255i32 - f.rgb[k] as i32;
                if gap != 0 {
                    let d = gap as f32 * 0.1;
                    let d = if d.abs() < 1.0 { if d < 0.0 { -1.0 } else { 1.0 } } else { d };
                    f.rgb[k] = (f.rgb[k] as i32 + d.round() as i32).clamp(0, 255) as u8;
                }
            }
            if f.signal_channel != 0 { ch.set(f.signal_channel as u32, 0, frame); }
        }
        // Ripples [M 0x1ac028]: the gate (counter-hash for Rand_Rand(period)), then 3 counter-hash
        // draws for the x/z jitter and the start scale.
        for r in &mut self.ripples {
            if r.period <= 0 { continue; }
            r.counter = r.counter.wrapping_add(0x9e37_79b9);
            if jitter(r.counter, 7) * r.period as f32 >= 1.0 { continue; }
            let jx = jitter(r.counter, 1) * (r.spread + r.spread) - r.spread;
            let jz = jitter(r.counter, 2) * (r.spread + r.spread) - r.spread;
            let s = jitter(r.counter, 3) * 0.05 + 0.05;
            gas.init([r.pos[0] + jx, r.pos[1], r.pos[2] + jz], 0.0, [0.0; 3], s, s + s, 6, [0x7f, 0x7f, 0x7f, 0xff], r.period as f32);
        }
        // Clouds [M 0x18d768]: fmod-wrapped phases, position = player + (phase_x, height, phase_z).
        for c in &mut self.clouds {
            c.phase[0] = (c.phase[0] + c.drift[0]).rem_euclid(c.wrap.max(1e-6));
            c.phase[1] = (c.phase[1] + c.drift[1]).rem_euclid(c.wrap.max(1e-6));
            let _ = player_pos; // the parked position rides the sky pass's own follow (levels.rs)
        }
        // Apocalypse [M 0x18d4f0]: the latch, the countdown, the whiteout value.
        for a in &mut self.apocalypses {
            if !a.armed && ch.on(a.channel as u32) { a.armed = true; }
            if a.armed {
                a.remaining -= 1.0;
                if a.remaining <= 0.0 { a.remaining = 0.0; a.fade = 0.0; continue; }
                if a.whiteout { a.fade = a.remaining * 5.0 / a.total; }
            }
        }
        // Emitters [M 0x17b080]: the rate countdown, the channels, the blink, the spawn, the motion.
        for e in &mut self.emitters {
            let Some(def) = self.defs.get(&e.def).cloned() else { continue; };
            // Creation-tick timing (fix-impact-remainder2): the game's object list is NEWEST-FIRST
            // (`control_add_object_to_list` 0x134698 prepends to the 0x2705a0 head; the pass-1 walk
            // in `control_movement_object_handler` 0x1336f8 follows head+0x14 first) and the walk
            // cursor never revisits the head, so an object created mid-walk (the impact emitter,
            // during the bullet's collide phase) is NOT updated its creation frame [M Collide_Update
            // 0x1ea398 vs Control_Update's class walk] and is the FIRST object updated the next
            // frame — ahead of the older drones. The port's `Drones::tick` runs this fx pass BEFORE
            // the bullet collide phase of the same frame, so an emitter created at tick N is first
            // seen HERE at tick N+1 — which IS the game's first `Emitter_Update` (the burst draws
            // then, not one tick later; the creation-frame skip is already implicit in the call
            // order, and gating it again delayed every burst by a tick and shifted every AI roll
            // after a wall hit).
            // The out channel resets every tick and re-sets while any particle lives [M 0x17b080].
            if e.ch_out != 0 { ch.set(e.ch_out as u32, 0, frame); }
            // The blink (param 5): ON half the period, OFF the whole period [M 0x17b080:105-120].
            if e.toggle_period > 0.0 {
                e.toggle_timer += REC_FRAME_RATE;
                let phase = if e.toggle_state { e.toggle_period * 0.5 } else { e.toggle_period };
                if e.toggle_timer > phase { e.toggle_timer = 0.0; e.toggle_state = !e.toggle_state; }
            }
            // The rate countdown [M 0x17b080:121-131].
            if e.rate > 0.0 {
                e.rate -= REC_FRAME_RATE;
                if e.rate < 0.0 { e.expired = true; }
            }
            let enabled = e.enabled(ch) && e.toggle_state;
            // Motion + colour over life; the out channel rides any live particle.
            let mut alive = 0usize;
            for p in &mut e.parts {
                if p.age < p.life {
                    alive += 1;
                    let c = def.colour_at(p.age * (1.0 / p.life));
                    p.rgba = [c[0].clamp(0.0, 255.0) as u8, c[1].clamp(0.0, 255.0) as u8, c[2].clamp(0.0, 255.0) as u8, c[3].clamp(0.0, 255.0) as u8];
                    p.vel[1] += REC_FRAME_RATE * def.gravity * -9.8;
                    for k in 0..3 { p.pos[k] += p.vel[k] * REC_FRAME_RATE; }
                    p.age += REC_FRAME_RATE;
                }
            }
            if alive > 0 && e.ch_out != 0 { ch.set(e.ch_out as u32, 1, frame); }
            // The spawn pass: batch-capped respawns of dead slots while enabled [M 0x17b080:191-260].
            let mut batch = def.batch;
            if def.max > 0 && def.batch > 0 && e.dead_ticks == 0 && enabled {
                for p in e.parts.iter_mut() {
                    if batch == 0 { break; }
                    let dead = p.life < p.age || p.age >= 99999.0;
                    if !dead { continue; }
                    if e.expired && p.age < 99999.0 { continue; } // one-shot: never-spawned slots only
                    p.age = 0.0;
                    batch -= 1;
                    // The spawn draws (yaw, pitch, speed, life) — the game's order [M 0x17b080:
                    // 216-247]. The impact emitters draw the SHARED stream (`Rand_FRandHalf(1.0)`
                    // × 4); the 0xf2 placements keep the counter-hash (no shared-stream draws).
                    let (yaw, pitch, speed, life);
                    if e.shared_rng {
                        yaw = def.yaw_base + def.yaw_var * crate::effect::frand_half(rng, 1.0);
                        pitch = def.pitch_base + def.pitch_var * crate::effect::frand_half(rng, 1.0);
                        speed = def.speed_base + def.speed_var * crate::effect::frand_half(rng, 1.0);
                        life = def.life_base + def.life_var * crate::effect::frand_half(rng, 1.0);
                        if std::env::var_os("NF_RNGLOG").is_some() {
                            eprintln!("[rnglog] emitter_burst +4 (draw #{})", rng.draws);
                        }
                    } else {
                        e.counter = e.counter.wrapping_add(0x9e37_79b9);
                        yaw = def.yaw_base + def.yaw_var * jitter(e.counter, 1);
                        pitch = def.pitch_base + def.pitch_var * jitter(e.counter, 2);
                        speed = def.speed_base + def.speed_var * jitter(e.counter, 3);
                        life = def.life_base + def.life_var * jitter(e.counter, 4);
                    }
                    // G-046: the game's cone is two PS2Sinf2 pairs [M Emitter_Update 0x17b080, disasm
                    // 0x17b504-0x17b53c]: PS2Sinf2(pitch + 1.5707964, pitch) → (cos p, sin p) and
                    // PS2Sinf2(yaw + 1.5707964, yaw) → (cos t, sin t); the stored dir vector is
                    // (sin p·cos t, cos p, sin p·sin t) — polar p from +Y, azimuth t from +X about +Y
                    // — then Vec_MulR32(speed). The port's old form (elevation p, azimuth from +Z)
                    // pointed the cone differently.
                    let (cp, sp) = (crate::trig::cosf(pitch), crate::trig::sinf(pitch));
                    let (ct, st) = (crate::trig::cosf(yaw), crate::trig::sinf(yaw));
                    let dir = [sp * ct * speed, cp * speed, sp * st * speed];
                    // The object basis (obj+0x90): the impact emitters carry the Mat_SetDir/
                    // SetNorm/SetUp rows built from the hit normal; the placements the euler matrix.
                    let m = e.basis.unwrap_or_else(|| nf_formats::placement::object_matrix(e.rotation, [0.0; 3]));
                    p.vel = [0, 1, 2].map(|k| m[k][0] * dir[0] + m[k][1] * dir[1] + m[k][2] * dir[2]);
                    p.pos = e.pos;
                    p.life = life;
                    let c0 = def.keys.first().copied().unwrap_or([0.0; 5]);
                    p.rgba = [c0[0].clamp(0.0, 255.0) as u8, c0[1].clamp(0.0, 255.0) as u8, c0[2].clamp(0.0, 255.0) as u8, c0[3].clamp(0.0, 255.0) as u8];
                }
            }
            // The all-dead latch: 3 ticks after everything died, a spent emitter dies [M 0x17b080:300-320].
            let all_dead = e.parts.iter().all(|p| p.age >= p.life);
            if (e.dead_ticks > 0 || e.expired) && all_dead { e.dead_ticks += 1; }
        }
        self.emitters.retain(|e| e.dead_ticks < 3);
        // (no gas.update() here — Drones::tick already runs the pool once per tick at its head)
    }

    /// The frame's particle sprites (`Emitter_Draw` path A inputs: position, def size, the slot rgba).
    pub fn particle_sprites(&self) -> Vec<ParticleSprite> {
        let mut out = Vec::new();
        for e in &self.emitters {
            let Some(def) = self.defs.get(&e.def) else { continue };
            for p in &e.parts {
                if p.age >= p.life || p.rgba[3] == 0 { continue; }
                out.push(ParticleSprite { pos: p.pos, size: def.size, rgba: p.rgba, additive: def.blend != 0 });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colour_at_lerps_between_keys() {
        let d = EmitterDef {
            hash: 1, texture: 0, model: 0, blend: 0, max: 1, batch: 1, size: 1.0,
            life_base: 1.0, life_var: 0.0, gravity: 0.0, speed_base: 0.0, speed_var: 0.0,
            pitch_base: 0.0, pitch_var: 0.0, yaw_base: 0.0, yaw_var: 0.0,
            keys: vec![[100.0, 0.0, 0.0, 255.0, 0.0], [0.0, 0.0, 0.0, 0.0, 1.0]],
        };
        assert_eq!(d.colour_at(0.0)[0] as i32, 100, "key0 at t=0");
        assert_eq!(d.colour_at(0.5)[0].round() as i32, 50, "the midpoint lerps");
        assert_eq!(d.colour_at(2.0)[3], 0.0, "past the last key = the last key");
    }

    /// `Flicker_Update` 0x1ab960 with the stubbed `Light_FindMapLight` 0x1cc3d0: the tint ramps
    /// white (10%/tick, min ±1) and the signal channel stamps 0 every tick, never 1.
    #[test]
    fn flicker_fades_white_and_the_signal_channel_stays_clear() {
        let mut fx = WorldFx::default();
        fx.flickers.push(FlickerFx { placement: 0, pos: [0.0; 3], signal_channel: 0x55, rgb: [0; 3] });
        let mut ch = Channels::default();
        ch.set(0x55, 1, 0);
        let mut rng = crate::drone::Rng::new(1);
        for _ in 0..600 { fx.tick(&mut ch, [0.0; 3], 0, &mut crate::gas::GasPool::default(), &mut rng); }
        assert_eq!(fx.flickers[0].rgb, [255; 3], "the tint reaches white");
        assert_eq!(ch.v[0x55], 0, "the stubbed FindMapLight keeps the light-on path");
    }

    /// `Ripples_Update` 0x1ac028: with the default period 30 the gate fires (counter-hash) roughly
    /// once per period; the puff lands inside the spread and takes the ripple mesh + white rgba.
    #[test]
    fn ripples_spawn_periodically_inside_the_spread() {
        let mut fx = WorldFx::default();
        fx.ripples.push(RippleFx { placement: 0, pos: [10.0, 5.0, 20.0], spread: 2.0, period: 30, counter: 0 });
        let mut gas = crate::gas::GasPool::default();
        let mut ch = Channels::default();
        let mut rng = crate::drone::Rng::new(1);
        let mut occupied = 0;
        let mut seen = None;
        for f in 0..600 {
            fx.tick(&mut ch, [0.0; 3], f, &mut gas, &mut rng);
            if !gas.puffs.is_empty() { occupied += 1; seen = Some(gas.puffs[0].clone()); }
        }
        // A ring lives its whole period (life = period), so with ~1 spawn per period the pool holds
        // a puff the overwhelming majority of ticks.
        assert!(occupied > 400, "600 ticks at period 30 → a ring alive most ticks, {occupied}");
        let g = seen.unwrap();
        assert!((g.pos[0] - 10.0).abs() <= 2.0 && (g.pos[2] - 20.0).abs() <= 2.0, "the jitter stays inside the spread");
        assert!((g.scale - 0.05).abs() < 0.051 || (g.scale - 0.10).abs() < 0.051, "the start scale is 0.05..0.10");
    }

    /// `Emitter_Update` 0x17b080: a continuous emitter (rate −1, no channels) fills its slots at the
    /// batch rate, integrates them (pos += vel·dt, gravity), colours them over life, drives the out
    /// channel while anything lives, and a one-shot (rate 0 → expired) dies 3 all-dead ticks later.
    #[test]
    fn emitter_spawns_integrates_and_a_one_shot_dies() {
        let def = EmitterDef {
            hash: 0x1234, texture: 0, model: 0, blend: 1, max: 8, batch: 2, size: 0.5,
            life_base: 0.5, life_var: 0.0, gravity: 1.0, speed_base: 1.0, speed_var: 0.0,
            pitch_base: 0.0, pitch_var: 0.0, yaw_base: 0.0, yaw_var: 0.0,
            keys: vec![[128.0, 64.0, 0.0, 255.0, 0.0], [0.0, 0.0, 0.0, 0.0, 1.0]],
        };
        let mut fx = WorldFx { defs: std::iter::once((0x1234, def.clone())).collect(), ..Default::default() };
        fx.emitters.push(EmitterFx {
            placement: 0, pos: [1.0, 2.0, 3.0], rotation: [0.0; 3], def: 0x1234, rate: -1.0,
            ch_on: 0, ch_off: 0, ch_out: 0x66, toggle_period: 0.0, toggle_timer: 0.0,
            toggle_state: true, expired: false, dead_ticks: 0, parts: (0..8).map(|_| EmitterParticle {
                pos: [0.0; 3], vel: [0.0; 3], age: 99999.0, life: 0.0, rgba: [0; 4],
            }).collect(), counter: 0,
            shared_rng: false, basis: None,
        });
        let mut ch = Channels::default();
        let mut rng = crate::drone::Rng::new(1);
        for f in 0..60 { fx.tick(&mut ch, [0.0; 3], f, &mut crate::gas::GasPool::default(), &mut rng); }
        assert_eq!(fx.emitters.len(), 1, "a continuous emitter never dies");
        assert!(fx.emitters[0].parts.iter().filter(|p| p.age < p.life).count() >= 6, "the slots cycle at the batch rate (batch 2/tick, life 0.5 s)");
        assert!(fx.particle_sprites().len() >= 6, "the draw handoff exposes the alive slots");
        assert_eq!(ch.v[0x66], 1, "the out channel is driven while particles live");
        let p0 = &fx.emitters[0].parts[0];
        assert!(p0.pos[1] < 2.0 && p0.vel[1] < 0.0, "gravity pulls the velocity down (speed 1 up, g 1)");
        assert!(p0.rgba != [0; 4], "the colour keys feed the slot rgba");
        // The one-shot: rate 0 → expired at create; the never-spawned slots fire once more
        // [M 0x17b080 the `age < 99999` skip], live their life, and 3 all-dead ticks later the
        // object self-destructs (obj+0xFE bit 0).
        fx.emitters[0].rate = 0.0;
        fx.emitters[0].expired = true;
        let mut died = None;
        for f in 60..200 {
            fx.tick(&mut ch, [0.0; 3], f, &mut crate::gas::GasPool::default(), &mut rng);
            if fx.emitters.is_empty() { died = Some(f); break; }
        }
        assert!(died.is_some(), "the spent one-shot self-destructs (obj+0xFE bit 0)");
        assert_eq!(ch.v[0x66], 0, "the out channel clears when the emitter dies");
    }

    /// `Apocalypse_Update` 0x18d4f0: the channel latches, the countdown runs, the whiteout value
    /// rises, and the object dies at 0.
    #[test]
    fn apocalypse_latches_counts_down_and_fades() {
        let mut fx = WorldFx::default();
        fx.apocalypses.push(ApocalypseFx {
            placement: 0, channel: 0x10, rumble: 1, whiteout: true, flashbang: false,
            remaining: 120.0, total: 120.0, armed: false, fade: 0.0,
        });
        let mut ch = Channels::default();
        let mut rng = crate::drone::Rng::new(1);
        fx.tick(&mut ch, [0.0; 3], 0, &mut crate::gas::GasPool::default(), &mut rng);
        assert!(!fx.apocalypses[0].armed && fx.apocalypses[0].fade == 0.0, "unlatched: no countdown");
        ch.set(0x10, 1, 1);
        for f in 1..61 { fx.tick(&mut ch, [0.0; 3], f, &mut crate::gas::GasPool::default(), &mut rng); }
        assert!(fx.apocalypses[0].armed, "the latch sets once the channel fires");
        assert!((fx.apocalypses[0].remaining - 60.0).abs() < 1e-4, "60 of 120 ticks remain");
        assert!((fx.apocalypses[0].fade - 60.0 * 5.0 / 120.0).abs() < 1e-4, "fade = remaining·5/total [M 0x18d4f0]");
        for f in 61..130 { fx.tick(&mut ch, [0.0; 3], f, &mut crate::gas::GasPool::default(), &mut rng); }
        assert!(fx.apocalypses[0].remaining == 0.0, "the countdown floors at 0");
    }

    /// `cloud_update` 0x18d768: the phases accumulate the drift and wrap by fmod.
    #[test]
    fn cloud_phases_drift_and_wrap() {
        let mut fx = WorldFx::default();
        fx.clouds.push(CloudFx { placement: 0, slot: 3, drift: [0.002, 0.0], height: 50.0, wrap: 0.1, phase: [0.0; 2] });
        let mut ch = Channels::default();
        let mut rng = crate::drone::Rng::new(1);
        for f in 0..100 { fx.tick(&mut ch, [0.0; 3], f, &mut crate::gas::GasPool::default(), &mut rng); }
        let c = &fx.clouds[0];
        assert!(c.phase[0] >= 0.0 && c.phase[0] < 0.1, "the phase wraps inside the modulus");
        assert!(c.phase[0] > 0.0, "the drift accumulated (100 ticks × 0.002 = 0.2 → wrapped)");
    }
}

