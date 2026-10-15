//! Input record / replay for `nf-view` (B-5).
//!
//! Records every logic tick's input — keys, mouse buttons, wheel steps and mouse-look deltas — in the EXACT form
//! `Sim::step_bond` consumes (`nf_game::player::Input` + the three bool arguments trigger / reload / use_key), plus
//! a header with the level, scenario, nf-view git hash and the starting RNG state. Replay feeds the same inputs back
//! through the real `Sim` (no GPU, no window) so sessions are bit-for-bit reproducible in tests. Replay files hold
//! input only (never game bytes) and are committed under `tests/replays/`.
//!
//! Recording costs nothing when it is off: the `App` holds an `Option<Recorder>`; the `None` branch does no work.
//!
//! See `docs/tools/dev-mode.md` for the commands.

use crate::scene::Scene;
use anyhow::{bail, Context, Result};

// ── magic + version ───────────────────────────────────────────────────────────
const MAGIC: &[u8; 8] = b"NFREPLAY";
const VERSION: u16 = 1;

// ── per-tick input ─────────────────────────────────────────────────────────────

/// One logic tick's input, mirroring `(nf_game::player::Input, trigger, reload, use_key)` — the exact arguments
/// `Sim::step_bond` consumes. `switch_to` is `i16` (-1 = None, else the weapon id) so the record is fixed-size
/// with no `Option` tag.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TickInput {
    pub forward: f32,
    pub strafe: f32,
    pub turn: f32,
    pub look: f32,
    pub weapon_cycle: i8,
    pub gadget_cycle: i8,
    pub switch_to: i16,
    pub jump: bool,
    pub crouch: bool,
    pub aiming: bool,
    pub zoom: bool,
    pub trigger: bool,
    pub reload: bool,
    pub use_key: bool,
}

impl TickInput {
    pub fn from_parts(input: &nf_game::player::Input, trigger: bool, reload: bool, use_key: bool) -> Self {
        Self {
            forward: input.forward,
            strafe: input.strafe,
            turn: input.turn,
            look: input.look,
            weapon_cycle: input.weapon_cycle,
            gadget_cycle: input.gadget_cycle,
            switch_to: input.switch_to.map(|w| w as i16).unwrap_or(-1),
            jump: input.jump,
            crouch: input.crouch,
            aiming: input.aiming,
            zoom: input.zoom,
            trigger, reload, use_key,
        }
    }

    pub fn to_parts(&self) -> (nf_game::player::Input, bool, bool, bool) {
        let input = nf_game::player::Input {
            forward: self.forward, strafe: self.strafe, turn: self.turn, look: self.look,
            jump: self.jump, crouch: self.crouch, aiming: self.aiming,
            weapon_cycle: self.weapon_cycle, gadget_cycle: self.gadget_cycle,
            switch_to: if self.switch_to < 0 { None } else { Some(self.switch_to as u8) },
            zoom: self.zoom,
            // Replays record the mouse's right button as `zoom`; it also drives the aim bit (`scope`).
            scope: self.zoom,
            fire: self.trigger,
            // The mode press is not part of the replay format (replays predate it); no recorded
            // replay presses it (fix-weapon-modes).
            mode_switch: false,
        };
        (input, self.trigger, self.reload, self.use_key)
    }
}

// ── per-tick state snapshot (for determinism comparison) ───────────────────────

/// The Sim state at the end of a tick — the fields the determinism test compares across two replays. Captures
/// Bond's pose, the held weapon + its ammo, and the shared RNG state (`SEED_X` / `SEED_Y`, 0x30d0a0 / 0x30d0a4).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TickState {
    pub bond_pos: [f32; 3],
    pub bond_yaw: f32,
    pub bond_pitch: f32,
    pub weapon: u8,
    pub clip: u32,
    pub spare: u32,
    pub rng_x: u32,
    pub rng_y: u32,
    pub frame: u32,
}

// ── header ─────────────────────────────────────────────────────────────────────

/// The recording's header: enough to re-create the exact starting state and to identify the build.
#[derive(Clone, Debug)]
pub struct Header {
    pub flags: u16,           // bit 0 = had --dev, bit 1 = had --scenario
    pub level: String,         // e.g. "07000005.bin"
    pub scenario: Option<String>, // present iff bit 1 set
    pub git_hash: String,     // nf-view's git rev (or "unknown")
    pub rng_x: u32,            // starting RNG state, before tick 0
    pub rng_y: u32,
    pub bond_pos: [f32; 3],    // Bond's starting pos (verification)
    pub bond_yaw: f32,
    pub bond_pitch: f32,
    pub start_weapon: u8,
    pub start_clip: u32,
    pub start_spare: u32,
}

// ── recording (the parsed file) ────────────────────────────────────────────────

/// A parsed replay file: its header plus the per-tick inputs.
#[derive(Clone, Debug)]
pub struct Replay {
    pub header: Header,
    pub ticks: Vec<TickInput>,
}

impl Replay {
    pub fn tick_count(&self) -> u32 { self.ticks.len() as u32 }

    /// Read + parse a replay file from disk.
    pub fn load(path: &str) -> Result<Replay> {
        let buf = std::fs::read(path).with_context(|| format!("read replay {path}"))?;
        Self::from_bytes(&buf)
    }

    pub fn from_bytes(buf: &[u8]) -> Result<Replay> {
        let mut r = Reader::new(buf);
        if r.take(8) != MAGIC { bail!("not an NFREPLAY file (bad magic)"); }
        let version = r.u16();
        if version != VERSION { bail!("unsupported replay version {version} (expected {VERSION})"); }
        let flags = r.u16();
        let level = r.string();
        let scenario = (flags & 2 != 0).then(|| r.string());
        let git_hash = r.string();
        let rng_x = r.u32(); let rng_y = r.u32();
        let bond_pos = [r.f32(), r.f32(), r.f32()];
        let bond_yaw = r.f32(); let bond_pitch = r.f32();
        let start_weapon = r.u8();
        let start_clip = r.u32(); let start_spare = r.u32();
        let tick_count = r.u32() as usize;
        let mut ticks = Vec::with_capacity(tick_count);
        for _ in 0..tick_count {
            let forward = r.f32(); let strafe = r.f32(); let turn = r.f32(); let look = r.f32();
            let weapon_cycle = r.i8(); let gadget_cycle = r.i8(); let switch_to = r.i16();
            let bits = r.u8();
            ticks.push(TickInput {
                forward, strafe, turn, look, weapon_cycle, gadget_cycle, switch_to,
                jump: bits & 1 != 0, crouch: bits & 2 != 0, aiming: bits & 4 != 0,
                zoom: bits & 8 != 0, trigger: bits & 16 != 0, reload: bits & 32 != 0, use_key: bits & 64 != 0,
            });
        }
        Ok(Replay { header: Header { flags, level, scenario, git_hash, rng_x, rng_y, bond_pos, bond_yaw, bond_pitch, start_weapon, start_clip, start_spare }, ticks })
    }

    /// Serialise to bytes (the inverse of `from_bytes`).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.put(MAGIC);
        w.u16(VERSION);
        w.u16(self.header.flags);
        w.string(&self.header.level);
        // The scenario string follows iff bit 1 of flags is set (consistent with `from_bytes`).
        if let Some(s) = &self.header.scenario { w.string(s); }
        w.string(&self.header.git_hash);
        w.u32(self.header.rng_x); w.u32(self.header.rng_y);
        w.f32(self.header.bond_pos[0]); w.f32(self.header.bond_pos[1]); w.f32(self.header.bond_pos[2]);
        w.f32(self.header.bond_yaw); w.f32(self.header.bond_pitch);
        w.u8(self.header.start_weapon);
        w.u32(self.header.start_clip); w.u32(self.header.start_spare);
        w.u32(self.ticks.len() as u32);
        for t in &self.ticks {
            w.f32(t.forward); w.f32(t.strafe); w.f32(t.turn); w.f32(t.look);
            w.i8(t.weapon_cycle); w.i8(t.gadget_cycle); w.i16(t.switch_to);
            let bits = t.jump as u8 | (t.crouch as u8) << 1 | (t.aiming as u8) << 2 | (t.zoom as u8) << 3
                | (t.trigger as u8) << 4 | (t.reload as u8) << 5 | (t.use_key as u8) << 6;
            w.u8(bits);
        }
        w.buf
    }

    /// Write to disk.
    pub fn save(&self, path: &str) -> Result<()> {
        std::fs::write(path, self.to_bytes()).with_context(|| format!("write replay {path}"))
    }

    /// Build a `Replay` from a scene's CURRENT state (the header captures the starting RNG / Bond pose / weapon) and a
    /// fixed list of tick inputs. Used by tests to synthesise a recording without a window (B-5 determinism test).
    pub fn from_scene_and_ticks(scene: &Scene, level: &str, scenario: Option<&str>, dev: bool, ticks: Vec<TickInput>) -> Result<Replay> {
        let r = Recorder::start(scene, ":memory:", level, scenario, dev)?;
        let mut replay = r.replay;
        replay.ticks = ticks;
        Ok(replay)
    }
}

// ── recorder (live, used by the windowed main loop) ────────────────────────────

/// Live recorder: appends each tick's input to an in-memory `Replay` (header set at creation), written to disk on
/// `finish`. Held as `Option<Recorder>` in the `App`; when it is `None` recording costs nothing.
pub struct Recorder {
    pub replay: Replay,
    pub path: String,
}

impl Recorder {
    /// Begin a recording. Captures the starting state (RNG, Bond pos/weapon) from the scene NOW — call after the
    /// scene is loaded and the scenario applied, before the first tick.
    pub fn start(scene: &Scene, path: &str, level: &str, scenario: Option<&str>, dev: bool) -> Result<Recorder> {
        let sim = scene.sim.as_ref().ok_or_else(|| anyhow::anyhow!("cannot record without a Sim (no level loaded)"))?;
        let header = Header {
            flags: (dev as u16) | (scenario.is_some() as u16) << 1,
            level: level.to_string(),
            scenario: scenario.map(|s| s.to_string()),
            git_hash: git_hash(),
            rng_x: sim.drones.rng.x, rng_y: sim.drones.rng.y,
            bond_pos: sim.bond.pos, bond_yaw: sim.bond.yaw, bond_pitch: sim.bond.pitch,
            start_weapon: sim.gun.weapon, start_clip: sim.gun.clip, start_spare: sim.gun.spare,
        };
        Ok(Recorder { replay: Replay { header, ticks: Vec::new() }, path: path.to_string() })
    }

    /// Append one tick's input. Called in the main loop right before `sim.step_bond`.
    pub fn push(&mut self, t: TickInput) { self.replay.ticks.push(t); }

    /// Finalise: write the recording to disk.
    pub fn finish(&self) -> Result<()> {
        self.replay.save(&self.path)?;
        eprintln!("[replay] wrote {} ({} ticks) to {}", self.replay.header.level, self.replay.ticks.len(), self.path);
        Ok(())
    }
}

// ── headless replay (no GPU, no window) — for tests ────────────────────────────

/// A snapshot cursor over a `Replay`: yields the next tick's input, or `None` when the recording ends.
pub struct ReplayCursor<'a> { ticks: &'a [TickInput], idx: usize }
impl<'a> ReplayCursor<'a> {
    pub fn new(r: &'a Replay) -> Self { Self { ticks: &r.ticks, idx: 0 } }
    pub fn next(&mut self) -> Option<TickInput> {
        let t = self.ticks.get(self.idx).copied();
        if t.is_some() { self.idx += 1; }
        t
    }
    pub fn done(&self) -> bool { self.idx >= self.ticks.len() }
}

/// Replay a recording through the REAL `Sim` (no GPU, no window). The scene must already be loaded with the
/// recording's level + scenario (so the starting state matches the header). Verifies the starting RNG / Bond
/// pose against the header, then feeds each tick's input to `step_bond`. Returns the Sim state snapshot at each
/// tick (including tick 0 = the starting state).
///
/// This is the function the determinism test calls twice on the same recording and compares.
pub fn replay_headless(scene: &mut Scene, replay: &Replay) -> Result<Vec<TickState>> {
    let sim = scene.sim.as_mut().context("no Sim in scene")?;
    // Verify the starting state matches the header (load determinism): RNG, Bond pose, weapon. If any differ, the
    // replay is invalid for this build — a real divergence we want surfaced, not hidden.
    let h = &replay.header;
    if sim.drones.rng.x != h.rng_x || sim.drones.rng.y != h.rng_y {
        bail!("replay start mismatch: rng ({:#x},{:#x}) vs header ({:#x},{:#x}) — non-deterministic load",
            sim.drones.rng.x, sim.drones.rng.y, h.rng_x, h.rng_y);
    }
    if (sim.bond.pos[0] - h.bond_pos[0]).abs() > 1e-3 || (sim.bond.pos[1] - h.bond_pos[1]).abs() > 1e-3
        || (sim.bond.pos[2] - h.bond_pos[2]).abs() > 1e-3 {
        bail!("replay start mismatch: bond {:?} vs header {:?}", sim.bond.pos, h.bond_pos);
    }
    let mut out = Vec::with_capacity(replay.ticks.len() + 1);
    out.push(snapshot(sim));
    for t in &replay.ticks {
        let (input, trigger, reload, use_key) = t.to_parts();
        sim.step_bond(input, trigger, reload, use_key);
        out.push(snapshot(sim));
    }
    Ok(out)
}

fn snapshot(sim: &crate::scene::Sim) -> TickState {
    TickState {
        bond_pos: sim.bond.pos, bond_yaw: sim.bond.yaw, bond_pitch: sim.bond.pitch,
        weapon: sim.gun.weapon, clip: sim.gun.clip, spare: sim.gun.spare,
        rng_x: sim.drones.rng.x, rng_y: sim.drones.rng.y, frame: sim.drones.frame,
    }
}

// ── little-endian reader/writer ────────────────────────────────────────────────

struct Reader<'a> { buf: &'a [u8], pos: usize }
impl<'a> Reader<'a> {
    fn new(buf: &'a [u8]) -> Self { Self { buf, pos: 0 } }
    fn take(&mut self, n: usize) -> &[u8] { let s = &self.buf[self.pos..self.pos + n]; self.pos += n; s }
    fn u8(&mut self) -> u8 { let v = self.buf[self.pos]; self.pos += 1; v }
    fn i8(&mut self) -> i8 { self.u8() as i8 }
    fn u16(&mut self) -> u16 { let v = u16::from_le_bytes(self.take(2).try_into().unwrap()); v }
    fn i16(&mut self) -> i16 { let v = i16::from_le_bytes(self.take(2).try_into().unwrap()); v }
    fn u32(&mut self) -> u32 { u32::from_le_bytes(self.take(4).try_into().unwrap()) }
    fn f32(&mut self) -> f32 { f32::from_bits(self.u32()) }
    fn string(&mut self) -> String {
        let n = self.u8() as usize;
        String::from_utf8_lossy(self.take(n)).into_owned()
    }
}

struct Writer { buf: Vec<u8> }
impl Writer {
    fn new() -> Self { Self { buf: Vec::new() } }
    fn put(&mut self, b: &[u8]) { self.buf.extend_from_slice(b); }
    fn u8(&mut self, v: u8) { self.buf.push(v); }
    fn i8(&mut self, v: i8) { self.buf.push(v as u8); }
    fn u16(&mut self, v: u16) { self.buf.extend_from_slice(&v.to_le_bytes()); }
    fn i16(&mut self, v: i16) { self.buf.extend_from_slice(&v.to_le_bytes()); }
    fn u32(&mut self, v: u32) { self.buf.extend_from_slice(&v.to_le_bytes()); }
    fn f32(&mut self, v: f32) { self.u32(v.to_bits()); }
    fn string(&mut self, s: &str) {
        let b = s.as_bytes();
        let n = b.len().min(255) as u8;
        self.u8(n);
        self.buf.extend_from_slice(&b[..n as usize]);
    }
}

/// nf-view's git revision at build time (or "unknown" if git is unavailable). Captured once when a recording starts.
fn git_hash() -> String {
    match std::process::Command::new("git").args(["rev-parse", "HEAD"]).output() {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => "unknown".to_string(),
    }
}
