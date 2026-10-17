//! Switch channels and trigger objects (docs/systems/triggers.md): placements 0xEA TouchOnce, 0xEB Touch, 0xEC
//! MultiplexIn (AND), 0xED MultiplexSIn (ordered AND), 0xEE MultiplexOut, 0xEF MultiplexOrIn (OR), 0xDC Trigger (type
//! = p0). Volumes are the trigger model's collision triangles at the placement transform; contacts are capsules
//! (Bond: radius 0.55 from feet + 0.55 to feet + 1.86; drones: *suggested* radius 0.4 from the feet to the head).

use crate::world::V3;
use nf_formats::level::Level;
use nf_formats::placement::{object_matrix, transform};

pub mod kind {
    pub const TOUCH_ONCE: u8 = 1;
    pub const TOUCH: u8 = 2;
    pub const AND: u8 = 3;
    pub const SEQ_AND: u8 = 4;
    pub const OUT: u8 = 5;
    pub const LOAD_LEVEL: u8 = 10;
    pub const OR: u8 = 0xc;
}

/// `GetLevelToLoadFromMovieCode` (0x1d3560), disasm-checked against the decomp: the movie-code →
/// next-level table the load pipeline's phase 2 consults (`ResetMap_Load` 0x1d6360:173). The 0x72xxxx
/// codes are the story cut-scene movies that play BETWEEN levels (a level's script event 0x12 requests
/// them: 0700004a's pack script 0x060006d3 stream 9 fires `0x12 [0x0720004a]` — its own archive holds
/// no 0xE8 and no other chain); the 0x73xxxx codes are the DRIVING.ELF interludes, and the table sends
/// every one of them to the front-end map 0x07000048. Any other code (including the 0x71xxxx briefing
/// wrappers and plain 0x0700xxxx levels) falls through to `GameState+8 & 0xff00ffff` — the pending
/// load with its stream byte (bits 16-23) masked off [M 0x1d3560:97-98]; the caller passes it.
pub fn movie_code_next_level(code: u32, fallback: u32) -> u32 {
    match code {
        // The story cut-scenes [M 0x1d3560:52-96 — every arm of the binary search tree].
        0x0720_0001 => 0x0710_0002,
        0x0720_0002 => 0x0710_0003,
        0x0720_0003 => 0x0710_0004,
        0x0720_0005 => 0x0710_0006,
        0x0720_0006 => 0x0710_0007,
        0x0720_0007 => 0x0710_0008,
        0x0720_0009 => 0x0710_000a,
        0x0720_000a => 0x0710_000b,
        0x0720_000c => 0x0710_000d,
        0x0720_000e => 0x0710_000f,
        0x0720_0011 => 0x0710_0012,
        0x0720_0012 => 0x0710_004a,
        0x0720_0014 => 0x0710_0015,
        0x0720_0015 => 0x0710_0016,
        0x0720_004a => 0x0710_0013,
        // The driving interludes → the front end [M 0x1d3560:24-50].
        0x0730_0004 | 0x0730_0008 | 0x0730_000b | 0x0730_000d | 0x0730_000f | 0x0730_0010
        | 0x0730_0013 | 0x0730_0017 | 0x0730_001b => 0x0700_0048,
        _ => fallback,
    }
}

/// The level-exit request's resolve: the port's stand-in for `ResetMap_LevelToLoad` (0x1d3800) +
/// `ResetMap_Load`'s phase-0/phase-2 cycle compressed to the archive the port finally loads [M
/// 0x1d6360:29-36, 173-182]. A plain 0x0700xxxx code loads as-is. A 0x0720xxxx cut-scene code runs
/// phase 2 (`GetLevelToLoadFromMovieCode` → a 0x0710xxxx briefing code) and then the second cycle's
/// phase-2 fallback masks it to the playable level (0x1d3560:97-98: `GameState+8 & 0xff00ffff`).
/// A 0x0730xxxx driving code resolves straight to its front-end landing (the table has every driving
/// code → 0x07000048; the port cannot run DRIVING.ELF, so the interlude is skipped — the game's own
/// X-skip shortens it to nothing observable too [M docs/systems/movies.md §Skipping]). A 0x0710xxxx
/// briefing wrapper masks directly.
pub fn resolve_exit_code(code: u32) -> u32 {
    let masked = code & 0xff00_ffff;
    match (code >> 20) & 0xf {
        1 => masked, // 0x0710xxxx briefing wrapper → its level
        2 => movie_code_next_level(code, masked) & 0xff00_ffff, // 0x0720xxxx cut-scene movie
        3 => movie_code_next_level(code, masked),               // 0x0730xxxx driving interlude
        _ => code,   // a plain 0x0700xxxx hashcode loads as-is
    }
}

#[derive(Clone, Debug)]
pub struct Trigger {
    /// Position in the main map's placement list (objects update in reverse placement order).
    pub placement: usize,
    pub kind: u8,
    pub out: u8,
    /// Contact mask (8 player, 0x10 drone, 0x18 either with the filter list).
    pub mask: u32,
    /// Touch: filter list; muxes: input channels; Out: target channels.
    pub list: Vec<u32>,
    pub lock: u8,
    pub hide: bool,
    /// `obj+0xF0 & 0x10` — the glist flag `Trigger_Update` requires before it deletes a finished TouchOnce
    /// (0x1b1d78-0x1b1d90) or a Bond-moment AND (out 0x65-0x6E, 0x1b1da0). `Trigger_Create` sets `obj+0xF0` from
    /// the object's glist [M 0x1b1720]; the port does not read glist+0x44 yet — live data: every TouchOnce on
    /// 07000005 carries the bit [L, trigger-port-check #6] — so the flag is modelled as set.
    pub glist_0x10: bool,
    /// `ext+0x2E` (`mask & 1`'s base channel, `Trigger_Hit` 0x1b1fc8:63-68): the filter list's 5th entry — for the
    /// Touch families that is param 6 (the same param the lock reads; both live at ext+0x24/+0x2E from slot p6).
    pub filter_gate: u8,
    pub tris: Vec<[V3; 3]>,
    pub lo: V3,
    pub hi: V3,
    /// TouchOnce removed itself (its channel was on).
    pub gone: bool,
    /// Made by a script (`Script_CreateEntity` 0xEA / 0xEB): the script object's index; its volume follows the object.
    pub script: Option<usize>,
    /// `LOAD_LEVEL` (0xE8): the next level's hashcode (placement param 0). `Trigger_Activate` (0x1b2318:111)
    /// calls `ResetMap_LevelToLoad(next_level, 1, 0)` [M].
    pub next_level: u32,
    /// `LOAD_LEVEL` gate channel: `Trigger_Update` case 10 (0x1b1a78:163) fires `Trigger_Activate` only when
    /// `switch_channels[gate]` is non-zero. The gate is `DOORINFO+0x28` = original `param[2]` of the placement
    /// (verified against MIPS disassembly: `lhu v1,56(s1); sh v1,40(s0)`) [M].
    pub gate: u8,
    /// `LOAD_LEVEL` key gate (`Trigger_Activate` 0x1b2318:76-90): param 1 — if non-zero and the channel is off,
    /// the trigger is locked (the fallback channel is set, no level load). [M]
    pub key_gate: u8,
    /// `LOAD_LEVEL` success channel (`Trigger_Activate` 0x1b2318:100-105): param 4 — set on a successful load. [M]
    pub success: u8,
    /// `LOAD_LEVEL` fallback channel (`Trigger_Activate` 0x1b2318:92-98): param 3 — set when the key gate blocks. [M]
    pub fallback: u8,
}

/// A body that can touch triggers: capsule axis from `a` to `b`, radius, drone (true) or Bond, drone skin hash.
#[derive(Clone, Copy, Debug)]
/// `contain`: whether the volume-containment contact (see `Trigger::touched_by`) applies to this toucher.
pub struct Toucher { pub a: V3, pub b: V3, pub r: f32, pub drone: bool, pub skin: u32, pub contain: bool }

impl Toucher {
    /// Bond in the normal stance (`Player_Collision` 0x194DF0, hip height 1.035 read live).
    pub fn player(feet: V3) -> Toucher { Toucher { a: [feet[0], feet[1] + 0.55, feet[2]], b: [feet[0], feet[1] + 1.86, feet[2]], r: 0.55, drone: false, skin: 0, contain: true } }
}

/// The channel table (`switch_channels`) with the frame each channel was last set (`switch_channels_time`).
#[derive(Clone, Debug)]
pub struct Channels { pub v: Vec<u8>, pub time: Vec<u32> }

impl Default for Channels { fn default() -> Channels { Channels { v: vec![0; 256], time: vec![0; 256] } } }

impl Channels {
    /// Channel 0 is a real channel: the game's readers index `switch_channels[ch]` directly —
    /// `SS_Update` 0x1a9948:20-23 reads AND writes `switch_channels[ss.ch]` for any channel word
    /// including 0 (the 0c pulley winches #351/#364/#377 listen on ch 0 [D]), and the trigger/AND
    /// writers copy Out into the array unconditionally [M 0x1b1a78]. The old port skipped ch 0 in
    /// both `set` and `on`, so a ch-0 SS could never run.
    pub fn on(&self, c: u32) -> bool { self.v.get(c as usize).is_some_and(|&x| x != 0) }
    pub fn set(&mut self, c: u32, value: u8, frame: u32) {
        if c > 255 { return; }
        // The game stamps `switch_channels_time` on EVERY nonzero write in its trigger and script paths
        // [M 0x1b1a78:87,107,188-189 — the type-2 OR, the AND write, the Out copy; Script_EventHandler 0xD],
        // not only on the rising edge (trigger-port-check #4): a re-set channel refreshes its SEQ_AND window.
        if value != 0 { self.time[c as usize] = frame; }
        // NF_LEVELLOG=1: every switch-channel write that turns a channel on (rising edge), with the frame
        // and the new value. Lets the user trace e.g. ch 0x1D being set by the EndLevel script's event 0x11.
        if std::env::var("NF_LEVELLOG").is_ok_and(|v| v == "1") && value != 0 && self.v[c as usize] == 0 {
            eprintln!("[levellog] frame {} channel 0x{:02x} = {}", frame, c, value);
        }
        self.v[c as usize] = value;
    }
}

fn sub(a: V3, b: V3) -> V3 { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
fn dot(a: V3, b: V3) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
fn cross(a: V3, b: V3) -> V3 { [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]] }
fn lerp(a: V3, b: V3, t: f32) -> V3 { [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t] }

/// Closest distance between point p and triangle t.
fn point_triangle(p: V3, t: &[V3; 3]) -> f32 {
    let (e0, e1) = (sub(t[1], t[0]), sub(t[2], t[0]));
    let n = cross(e0, e1);
    let nn = dot(n, n);
    if nn > 1e-12 {
        let d = dot(sub(p, t[0]), n) / nn;
        let q = sub(p, [n[0] * d, n[1] * d, n[2] * d]);
        let inside = (0..3).all(|k| { let (u, v) = (t[k], t[(k + 1) % 3]); dot(cross(sub(v, u), sub(q, u)), n) >= 0.0 });
        if inside { return (d * d * nn).sqrt(); }
    }
    (0..3).map(|k| segment_segment(p, p, t[k], t[(k + 1) % 3])).fold(f32::INFINITY, f32::min)
}

/// Closest distance between segments p1–q1 and p2–q2.
fn segment_segment(p1: V3, q1: V3, p2: V3, q2: V3) -> f32 {
    let (d1, d2, r) = (sub(q1, p1), sub(q2, p2), sub(p1, p2));
    let (a, e, f) = (dot(d1, d1), dot(d2, d2), dot(d2, r));
    let (s, t) = if a <= 1e-12 && e <= 1e-12 { (0.0, 0.0) } else if a <= 1e-12 { (0.0, (f / e).clamp(0.0, 1.0)) } else {
        let c = dot(d1, r);
        if e <= 1e-12 { ((-c / a).clamp(0.0, 1.0), 0.0) } else {
            let b = dot(d1, d2);
            let den = a * e - b * b;
            let mut s = if den > 1e-12 { ((b * f - c * e) / den).clamp(0.0, 1.0) } else { 0.0 };
            let mut t = (b * s + f) / e;
            if t < 0.0 { t = 0.0; s = (-c / a).clamp(0.0, 1.0); } else if t > 1.0 { t = 1.0; s = ((b - c) / a).clamp(0.0, 1.0); }
            (s, t)
        }
    };
    let (c1, c2) = (lerp(p1, q1, s), lerp(p2, q2, t));
    dot(sub(c1, c2), sub(c1, c2)).sqrt()
}

/// The capsule-vs-mesh contact the trigger `contains` uses, shared with the hurt volumes
/// (the game's object collision picks a class-0x27 hurt volume's glist cels with the player's
/// capsule [M Player_DealWithObjHit 0x194458:95-103; the exact BL pick rule is the port's
/// trigger-contact approximation]).
pub fn capsule_hits_tris(a: V3, b: V3, r: f32, tris: &[[V3; 3]]) -> bool {
    tris.iter().any(|t| {
        segment_hits_triangle(a, b, t) || point_triangle(a, t) <= r || point_triangle(b, t) <= r
            || (0..3).any(|k| segment_segment(a, b, t[k], t[(k + 1) % 3]) <= r)
    })
}

/// Whether the segment a–b crosses triangle t.
fn segment_hits_triangle(a: V3, b: V3, t: &[V3; 3]) -> bool {
    let d = sub(b, a);
    let (e1, e2) = (sub(t[1], t[0]), sub(t[2], t[0]));
    let p = cross(d, e2);
    let det = dot(e1, p);
    if det.abs() < 1e-12 { return false; }
    let s = sub(a, t[0]);
    let u = dot(s, p) / det;
    if !(0.0..=1.0).contains(&u) { return false; }
    let q = cross(s, e1);
    let v = dot(d, q) / det;
    if v < 0.0 || u + v > 1.0 { return false; }
    (0.0..=1.0).contains(&(dot(e2, q) / det))
}

impl Trigger {
    /// A trigger of `kind` from its parameters (placement or script entity: parameter k at `level_tag+0x2C+4k`) and its
    /// world triangles; `None` for a Touch on channel 0x7D (only live on 0x07001500). The wrapper families shift the
    /// slots differently [M 0x1d04f0 dispatch; core-loop port map §3.1]: the Touch wrappers keep slots 2-7 in place
    /// (0x1b2aa8/0x1b2b88), the mux wrappers shift every param down by one (out p0 → slot 1, inputs = raw p1..p7,
    /// mask = raw p8, lock = raw p9, hide = raw p10 — 0x1b2c68/0x1b2e08/0x1b2d38 loops), and `Trigger_LoadLevelCreate`
    /// (0x1b2950, disasm) forces mask = 0xE and maps key/gate/fallback/success into the input slots.
    pub fn from_params(placement: usize, kind: u8, q: impl Fn(u32) -> u32, tris: Vec<[V3; 3]>) -> Option<Trigger> {
        let (out_ch, mask, list, lock, hide, next_level, gate, key_gate, success, fallback): (u32, u32, Vec<u32>, u32, bool, u32, u8, u8, u8, u8) = match kind {
            kind::TOUCH_ONCE => (q(0), q(1), (2..=8).map(&q).collect(), q(7), q(8) != 0, 0, 0, 0, 0, 0),
            kind::TOUCH => (q(0), q(1), (2..=8).map(&q).collect(), q(6), q(7) != 0, 0, 0, 0, 0, 0),
            // `LOAD_LEVEL` (0xE8): `Trigger_Update` case 10 fires when `switch_channels[gate]` is non-zero —
            // NO touch check. `next_level` = param 0. `gate` = param 2 (the gate channel; set by the EndLevel
            // script's event 0x11 on 07000005). `key_gate` = param 1, `success` = param 4, `fallback` = param 3.
            // The wrapper hard-sets mask 0xE (`addiu t0,zero,14` @0x1b299c → tag+0x50) [M]. Only the input slots
            // p1 (key) and p2 (gate) are read back by `Trigger_Activate` (+0x26/+0x28) — the list keeps p1..p4.
            // [M Trigger_Update 0x1b1a78:162-166, Trigger_Activate 0x1b2318:64-111, 0x1b2950].
            kind::LOAD_LEVEL => (0, 0xE, (1..=4).map(&q).collect(), q(10), q(11) != 0, q(0), q(2) as u8, q(1) as u8, q(4) as u8, q(3) as u8),
            // The mux family (0xEC AND / 0xED SEQ_AND / 0xEE Out / 0xEF OR): the wrapper backs the params up and
            // writes slot[k] = p(k-1) for k = 2..7 ONLY — the loop bound is `while (uVar3 < 8)` [M 0x1b2c68], so
            // slot 8 keeps raw p8 and slots 9-11 are never touched. `Trigger_Create` copies 7 input ushorts from
            // slots 2..8 into ext+0x26..0x32 (ext+0x34 stays 0) and reads mask/lock/hide from slots 9/10/11
            // [M 0x1b1720:80-91 — the audit's correction of the port-map's "p1..p7 / p8 / p9 / p10" table].
            _ => (q(0), q(9), (1..=6).map(&q).chain(std::iter::once(q(8))).collect(), q(10), q(11) != 0, 0, 0, 0, 0, 0),
        };
        // `Trigger_Create` defaults a zero mask to 2 (class-5 contacts) [M 0x1b1720:80-84].
        let mask = if mask == 0 { 2 } else { mask };
        if matches!(kind, kind::TOUCH | kind::TOUCH_ONCE) && out_ch == 0x7d { return None; }
        let filter_gate = if matches!(kind, kind::TOUCH | kind::TOUCH_ONCE) { list.get(4).copied().unwrap_or(0) as u8 } else { 0 };
        let mut t = Trigger { placement, kind, out: out_ch as u8, mask, list, lock: lock as u8, hide, glist_0x10: true, filter_gate, tris: Vec::new(), lo: [0.0; 3], hi: [0.0; 3], gone: false, script: None, next_level, gate, key_gate, success, fallback };
        t.set_tris(tris);
        Some(t)
    }

    /// New world triangles (a moving trigger).
    pub fn set_tris(&mut self, tris: Vec<[V3; 3]>) {
        let (mut lo, mut hi) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
        for t in &tris { for v in t { for k in 0..3 { lo[k] = lo[k].min(v[k]); hi[k] = hi[k].max(v[k]); } } }
        (self.tris, self.lo, self.hi) = (tris, lo, hi);
    }

    /// The trigger's bounding-box centre (for scenario placement — put Bond at a TouchOnce trigger's centre to
    /// fire it). B-11: pub so nf-view's cutscene-ai-pause scenario + its regression test can find the gatehouse.
    pub fn centre(&self) -> V3 {
        if self.tris.is_empty() { return [0.0; 3]; }
        [(self.lo[0] + self.hi[0]) * 0.5, (self.lo[1] + self.hi[1]) * 0.5, (self.lo[2] + self.hi[2]) * 0.5]
    }

    /// Whether a capsule touches the trigger's triangles.
    /// `Trigger_Hit` 0x1b1fc8 itself does NO geometry: it classifies the object's CONTACT LIST
    /// (obj+0xD0, built by the collision pass — a toucher whose sphere overlaps the trigger's glist
    /// bounds counts, including one standing INSIDE a room-shaped volume where no triangle is
    /// within reach). The port's triangle tests cover the plate/quad cases; the containment branch
    /// here covers the volume case (a toucher capsule fully inside the trigger's bounds — e.g.
    /// 07000006's LoadPoint #746, an open room-shaped glist [D measured tris]).
    pub fn touched_by(&self, c: &Toucher) -> bool {
        let grow = c.r + 0.1;
        let lo = [c.a[0].min(c.b[0]) - grow, c.a[1].min(c.b[1]) - grow, c.a[2].min(c.b[2]) - grow];
        let hi = [c.a[0].max(c.b[0]) + grow, c.a[1].max(c.b[1]) + grow, c.a[2].max(c.b[2]) + grow];
        if (0..3).any(|k| hi[k] < self.lo[k] || lo[k] > self.hi[k]) { return false; }
        // Containment: both capsule ends inside the trigger's own bounds (the bounding-sphere
        // contact the game's collision pass registers).
        // Not for Bond hanging on a wire / zipline: there the bounds of the chasm's plunge volume enclose him, and
        // the containment contact fired the falling-to-death cut-scene the moment he grabbed the 07000005 cable
        // (ch 0xBE, merge of map-mayhew over fix-wire). In the game the crossing has no fall or death path at all
        // [M Player_Wire 0x197658]. [S]: the game's exact contact rule for a hanging Bond isn't read yet.
        // fix-objects2: containment is a PLATE/ROOM contact, not a valley-shell one. The game's contact is the
        // capsule against the volume's own mesh (the shells fire on crossing; standing strictly inside a hollow
        // volume touches nothing — 07000008's boarding platform sits INSIDE plunge #54's AABB and the game's player
        // does not die there [D #54 tris: a floor quad at y 17.5 + a wall at x 136.6 over the valley; the platform
        // at y ≈ 24–27 touches neither]). The containment approximation stays only for volumes whose triangles are
        // small (the room-exit shells and walk-over plates — 07000006's LoadPoint #746, edges ≤ 10.4); the plunge
        // family's valley-sized quads (edges 25.8–46.5 [D]) fall back to the mesh test alone.
        let contain_ok = self.tris.iter().all(|t| (0..3).all(|k| {
            let a = t[k];
            let b = t[(k + 1) % 3];
            ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)) < 24.0 * 24.0
        }));
        if c.contain && contain_ok
            && (0..3).all(|k| c.a[k] >= self.lo[k] && c.a[k] <= self.hi[k])
            && (0..3).all(|k| c.b[k] >= self.lo[k] && c.b[k] <= self.hi[k]) {
            return true;
        }
        capsule_hits_tris(c.a, c.b, c.r, &self.tris)
    }

    /// `Trigger_Hit` (0x1B1FC8) for a Touch / TouchOnce trigger. The 0x18 branch is an AND-chain [M 0x1b1fc8:56-123]:
    /// the base is 1, or `ch[ext+0x2E]` when `mask & 1`; every nonzero filter entry must then pass (0xB needs a drone
    /// contact, 0xE the player, anything else a drone whose skin hash matches the entry's low 16 bits). An empty
    /// filter list returns the base alone.
    fn hit(&self, ch: &Channels, touchers: &[Toucher]) -> bool {
        let player = touchers.iter().any(|c| !c.drone && self.touched_by(c));
        let drone_touch = touchers.iter().any(|c| c.drone && self.touched_by(c));
        let m = self.mask;
        // The filter branch runs only for types 1/2 with mask & 0x18 and a player/drone contact [M 0x1b1fc8:56].
        if m & 0x18 != 0 && (player || drone_touch) {
            if m == 8 { return player; }
            if m == 0x10 && !drone_touch { return false; }
            // Base: 1, or the ext+0x2E channel when mask bit 0 is set [M 0x1b1fc8:63-68].
            let mut acc = if m & 1 != 0 { ch.on(self.filter_gate as u32) } else { true };
            for &f in self.list.iter().take_while(|&&f| f != 0) {
                let ok = match f & 0xffff {
                    0xb => drone_touch,
                    0xe => player,
                    h => touchers.iter().any(|c| c.drone && self.touched_by(c) && c.skin & 0xffff == h),
                };
                acc = acc && ok;
            }
            return acc;
        }
        // mask & 2 (class-5 object contacts, 0x1b1fc8:124-159): the port has no object touchers — contacts bit 1
        // never sets (documented stand-in, trigger-port-check #11).
        (m & 8 != 0 && player) || (m & 0x10 != 0 && drone_touch)
    }

    /// `Trigger_Activate` (0x1b2318): fires the trigger immediately from `Player_Activate`'s E-key path (routes
    /// b and c on 07000005 — the on-foot gate door and the zip-wire exit). Returns the channel writes to apply
    /// and `Some(next_level)` if a level load is requested. Does NOT mutate — the caller applies the writes
    /// (avoids the `&mut self.triggers[i]` + `&mut self.channels` aliasing). [M 0x1b2318:34-111].
    pub fn activate_request(&self, ch: &Channels) -> (Vec<(u8, u8)>, Option<u32>) {
        match self.kind {
            kind::LOAD_LEVEL => {
                // The game latches `ext+0x38 = 99` (type 99 = dead) after the load request [M
                // 0x1b2318:113], so the E-path can never re-fire a spent exit either (`gone`).
                if self.gone { return (Vec::new(), None); }
                // Cancel channels 0x60–0x63 (99, 0x62, 0x61, 0x60) [M 0x1b2318:64-75]: if any is on, abort (an
                // alarm blocks the exit).
                if (0x60..=0x63).any(|c| ch.on(c)) { return (Vec::new(), None); }
                // Key gate [M 0x1b2318:76-90]: if non-zero and off, the trigger is locked — locked message (label
                // 0x2000021) + ext+0x3C cooldown (no text system in the port), NO channel write, NO load.
                if self.key_gate != 0 && !ch.on(self.key_gate as u32) { return (Vec::new(), None); }
                // Auto channel [M 0x1b2318:96-108]: gate (+0x28) on → the success write (+0x2C); gate off →
                // the fallback write (+0x2A) — and BOTH branches fall through to the level load @0x1b24b8.
                if self.gate != 0 && ch.on(self.gate as u32) {
                    let sets = if self.success != 0 { vec![(self.success, 1)] } else { Vec::new() };
                    (sets, Some(self.next_level))
                } else {
                    let sets = if self.fallback != 0 { vec![(self.fallback, 1)] } else { Vec::new() };
                    (sets, Some(self.next_level))
                }
            }
            kind::TOUCH_ONCE => {
                // `Trigger_Activate` (0x1b2318:38-49) type-1 arm: begins with the `mask & 4` gate — a
                // trigger whose mask lacks bit 4 is NOT player-activatable (07000005's plunge/death
                // triggers are mask 8: pressing E near one must never fire it — playtest F5). The port
                // used to fire them from the use-key path. Then the out write (skipped when out = 0,
                // :41-44) — the glist-0x10 self-delete latch is the caller's (`gone`, set by
                // player_use_triggers like the game's `obj+0xFE |= 1`).
                if self.mask & 4 == 0 { return (Vec::new(), None); }
                if !self.gone && self.out != 0 { (vec![(self.out, 1)], None) } else { (Vec::new(), None) }
            }
            kind::TOUCH => {
                // `Trigger_Activate` (0x1b2318:23-33) type-2 arm: the same `mask & 4` gate, then the out
                // write with out != 0 required.
                if self.mask & 4 == 0 || self.out == 0 { return (Vec::new(), None); }
                (vec![(self.out, 1)], None)
            }
            _ => (Vec::new(), None),
        }
    }

    /// `Trigger_Update` (0x1B1A78) for every type except Touch (run in `calc`). The mux input scan walks the slots
    /// as a PREFIX — a zero slot ends the list (a mid-list 0 hides everything after it) [M 0x1b1a78 case 3/4/0xC
    /// loops: `while input[k] != 0`].
    pub fn update(&mut self, ch: &mut Channels, touchers: &[Toucher], frame: u32) {
        if self.gone || (self.lock != 0 && ch.on(self.lock as u32)) { return; }
        match self.kind {
            kind::TOUCH_ONCE => {
                // Case 1: channel already on → delete only when the glist carries 0x10 [M 0x1b1d78-0x1b1d90];
                // otherwise it FALLS THROUGH into case 2 every frame while the channel is off.
                if ch.on(self.out as u32) {
                    if self.glist_0x10 { self.gone = true; }
                    return;
                }
                if self.hit(ch, touchers) { ch.set(self.out as u32, 1, frame); }
            }
            kind::AND | kind::OR => {
                // Case 3 / 0xC: the accumulator starts at 1 for the AND (an empty/zero prefix writes 1 —
                // 0x1b1cbc `addiu a3,zero,1`) and at 0 for the OR; a zero input slot ends the scan.
                let mut acc = self.kind == kind::AND;
                for &c in self.list.iter().take_while(|&&c| c != 0) {
                    let on = ch.on(c);
                    acc = if self.kind == kind::AND { acc && on } else { acc || on };
                }
                ch.set(self.out as u32, acc as u8, frame);
                // The Bond-moment self-delete: out 0x65-0x6E, result on, glist 0x10 [M 0x1b1da0-0x1b1da8].
                if acc && (0x65..=0x6e).contains(&self.out) && self.glist_0x10 { self.gone = true; }
            }
            kind::SEQ_AND => {
                // Case 4: first input read directly (an empty list reads ch[0] = off → writes 0); every later
                // input must be on AND satisfy `time[prev] − 4 <= time[cur]` — the current stamp may be up to 4
                // frames OLDER than the previous one's, with no upper bound [M 0x1b1e04-0x1b1e1c].
                let mut it = self.list.iter().take_while(|&&c| c != 0);
                let mut acc = it.next().is_some_and(|&c| ch.on(c));
                let mut prev = self.list.first().copied().unwrap_or(0);
                for &c in it {
                    if !ch.on(c) { acc = false; break; }
                    let (a, b) = (ch.time[prev as usize] as i64, ch.time[c as usize] as i64);
                    if a - 4 > b { acc = false; break; }
                    prev = c;
                }
                ch.set(self.out as u32, acc as u8, frame);
                if acc && (0x65..=0x6e).contains(&self.out) && self.glist_0x10 { self.gone = true; }
            }
            kind::OUT => {
                let v = ch.v.get(self.out as usize).copied().unwrap_or(0);
                for &t in &self.list { if t != 0 { ch.set(t, v, frame); } }
            }
            // `LOAD_LEVEL` (0xE8): `Trigger_Update` case 10 fires when `switch_channels[gate]` is non-zero —
            // NO touch check [M 0x1b1a78:162-166] — and then runs the FULL `Trigger_Activate` [M 0x1b2318]:
            // the cancel channels, the key gate (locked → no write, no load), the success write, and the
            // type←99 latch (`gone`). The level-load side effects (Music_Event(3,1), `GS_PauseGame`,
            // `ResetMap_LevelToLoad`) are carried by `gone` — `drone.rs`'s LOAD_LEVEL arm posts them.
            kind::LOAD_LEVEL => {
                if self.gone || self.gate == 0 { return; }
                if !ch.on(self.gate as u32) { return; }
                if (0x60..=0x63).any(|c| ch.on(c)) { return; }
                if self.key_gate != 0 && !ch.on(self.key_gate as u32) { return; }
                if self.success != 0 { ch.set(self.success as u32, 1, frame); }
                self.gone = true;
            }
            _ => {}
        }
    }
}

/// Every trigger of the level's main map.
pub fn build(level: &Level) -> Vec<Trigger> {
    let Some(m) = level.main_map() else { return Vec::new() };
    let map = &level.maps[m];
    let mut out = Vec::new();
    for (i, p) in map.placements.iter().enumerate() {
        let param = |k: u32| p.params.iter().find(|(j, _)| *j == k).map(|&(_, v)| v).unwrap_or(0);
        let kind = match p.flags { 0xea => kind::TOUCH_ONCE, 0xeb => kind::TOUCH, 0xec => kind::AND, 0xed => kind::SEQ_AND, 0xee => kind::OUT,
            0xef => kind::OR, 0xe8 => kind::LOAD_LEVEL, 0xdc => param(0) as u8, _ => continue };
        // Every trigger family — typed 0xEA-0xEF wrappers and the raw 0xDC alike — shares one param
        // layout: the type byte at p0 (the typed wrappers overwrite it), out = p1, inputs = p2..p8,
        // mask = p9, lock = p10, hide = p11 [M Trigger_Create 0x1b1720:61-91; the 0xEA-0xEF wrappers
        // all tail-call it with the same param_3]. No shift for the raw type.
        let q = |k: u32| param(k);
        let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() } else if (p.index as usize) < map.objects.len() { Some((m, p.index as usize)) } else { None };
        let mx = object_matrix(p.rotation, p.translation);
        let tris: Vec<[V3; 3]> = target.and_then(|(om, oi)| level.maps[om].objects[oi].collision.as_ref())
            .map(|c| c.triangles().into_iter().map(|t| t.map(|v| transform(&mx, v))).collect()).unwrap_or_default();
        // Touch triggers on channel 0x7D only work on level 0x07001500.
        if let Some(t) = Trigger::from_params(i, kind, q, tris) { out.push(t); }
    }
    out
}

/// `Trigger_Calc` (0x1B1900): level latches, then the Touch triggers (their channels cleared, then OR-ed).
pub fn calc(level: u32, triggers: &mut [Trigger], ch: &mut Channels, touchers: &[Toucher], frame: u32) {
    if level == 0x0700_0005 && ch.on(0xa4) { ch.set(0x6f, 1, frame); }
    if level == 0x0700_000b && ch.on(0x34) { ch.set(0x2d, 1, frame); }
    for t in triggers.iter().filter(|t| t.kind == kind::TOUCH) { ch.set(t.out as u32, 0, frame); }
    let hits: Vec<u8> = triggers.iter().filter(|t| t.kind == kind::TOUCH && !(t.lock != 0 && ch.on(t.lock as u32)) && t.hit(ch, touchers)).map(|t| t.out).collect();
    for out in hits { ch.set(out as u32, 1, frame); }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plate(kind: u8, out: u8, mask: u32, list: Vec<u32>) -> Trigger {
        // A 2 × 2 horizontal quad at y = 0 around the origin.
        let (a, b, c, d) = ([-1.0, 0.0, -1.0], [1.0, 0.0, -1.0], [1.0, 0.0, 1.0], [-1.0, 0.0, 1.0]);
        Trigger { placement: 0, kind, out, mask, list, lock: 0, hide: false, glist_0x10: true, filter_gate: 0, tris: vec![[a, b, c], [a, c, d]], lo: [-1.0, 0.0, -1.0], hi: [1.0, 0.0, 1.0], gone: false, script: None, next_level: 0, gate: 0, key_gate: 0, success: 0, fallback: 0 }
    }

    #[test]
    fn touch_once_latches_and_muxes_follow_inputs() {
        let mut ch = Channels::default();
        let mut once = plate(kind::TOUCH_ONCE, 0x23, 8, vec![]);
        let mut or = Trigger { kind: kind::OR, out: 0x22, list: vec![0x23, 0x24], ..plate(kind::OR, 0, 0, vec![]) };
        let away = [Toucher::player([5.0, 0.0, 5.0])];
        let on = [Toucher::player([0.5, -0.3, 0.0])];
        once.update(&mut ch, &away, 1);
        or.update(&mut ch, &away, 1);
        assert!(!ch.on(0x22));
        once.update(&mut ch, &on, 2);
        or.update(&mut ch, &on, 2);
        assert!(ch.on(0x23) && ch.on(0x22));
        // Latched: stays on after Bond leaves; the TouchOnce removes itself.
        once.update(&mut ch, &away, 3);
        or.update(&mut ch, &away, 3);
        assert!(ch.on(0x22) && once.gone);
        // A drone-only plate ignores Bond.
        let drone_plate = plate(kind::TOUCH_ONCE, 0x30, 0x10, vec![]);
        assert!(!drone_plate.hit(&ch, &on));
        assert!(drone_plate.hit(&ch, &[Toucher { drone: true, ..on[0] }]));
    }

    /// `LOAD_LEVEL` (0xE8): `Trigger_Update` case 10 fires when `switch_channels[gate]` is non-zero — no touch check.
    /// The gate is set by a script event. When the gate is on, the trigger latches `gone` and the caller reads
    /// `next_level`. Sister: gate off → never fires.
    #[test]
    fn load_level_trigger_fires_on_gate_channel() {
        let mut ch = Channels::default();
        let mut exit = Trigger { kind: kind::LOAD_LEVEL, gate: 0x1D, next_level: 0x0700_0006, ..plate(kind::LOAD_LEVEL, 0, 8, vec![]) };
        let touchers: [Toucher; 0] = [];
        // Sister: gate off → nothing.
        exit.update(&mut ch, &touchers, 1);
        assert!(!exit.gone);
        // Gate on → fires, latches.
        ch.set(0x1D, 1, 1);
        exit.update(&mut ch, &touchers, 2);
        assert!(exit.gone);
        assert_eq!(exit.next_level, 0x0700_0006, "carries the next level hashcode");
        // Stays gone.
        exit.update(&mut ch, &touchers, 3);
        assert!(exit.gone);
    }

    /// `Trigger_Activate` (the E-key path, routes b/c on 07000005): pressing E on a LOAD_LEVEL trigger fires it
    /// immediately — no gate channel needed. Gate on → the success channel; gate OFF → the fallback channel AND
    /// the level still loads (both branches fall through to the load @0x1b24b8) [M 0x1b2318:96-111]. Sister: a
    /// key-gated trigger with the key off → NO write and NO load (locked message + cooldown, 0x1b2318:76-90).
    #[test]
    fn trigger_activate_fires_on_e_key_path() {
        let ch = Channels::default();
        let exit = Trigger { kind: kind::LOAD_LEVEL, next_level: 0x0700_0066, success: 0x1F, fallback: 0x1E, key_gate: 0, gate: 0, ..plate(kind::LOAD_LEVEL, 0, 8, vec![]) };
        // Gate 0 = the auto channel is off → the fallback write AND the load [M 0x1b2318:96-108].
        let (sets, next) = exit.activate_request(&ch);
        assert_eq!(next, Some(0x0700_0066), "requests the next level");
        assert_eq!(sets, vec![(0x1E, 1)], "the gate-off fallback write");
        // Gate off (and key gate clear) → the fallback write AND the load still happen [M 0x1b2318:105-108].
        let gated = Trigger { gate: 0x10, ..exit.clone() };
        let (sets, next) = gated.activate_request(&ch);
        assert_eq!(next, Some(0x0700_0066), "the gate-off fallback branch STILL loads");
        assert!(sets.contains(&(0x1E, 1)), "sets the fallback channel");
        // Gate on → the success channel.
        let mut ch_on = Channels::default();
        ch_on.set(0x10, 1, 1);
        let (sets, next) = gated.activate_request(&ch_on);
        assert_eq!(next, Some(0x0700_0066));
        assert!(sets.contains(&(0x1F, 1)), "sets the success channel");
        // Sister: key gate on and off → no write, no load (the game shows the locked message instead).
        let ch2 = Channels::default();
        let locked = Trigger { kind: kind::LOAD_LEVEL, next_level: 0x0700_0066, success: 0x1F, fallback: 0x1E, key_gate: 0x10, gate: 0, ..plate(kind::LOAD_LEVEL, 0, 8, vec![]) };
        let (sets2, next2) = locked.activate_request(&ch2);
        assert_eq!(next2, None, "locked trigger does not load");
        assert!(sets2.is_empty(), "the locked path writes NO channel (0x1b2318 returns at :90)");
        // Sister: an alarm channel (0x60-0x63) blocks the exit entirely.
        let mut ch3 = Channels::default();
        ch3.set(0x61, 1, 1);
        let (sets3, next3) = locked.activate_request(&ch3);
        assert_eq!(next3, None, "an alarm blocks the level load");
        assert!(sets3.is_empty());
    }

    /// The mux wrappers write slot[k] = p(k-1) for k = 2..7 only [M 0x1b2c68 loop bound `while (uVar3 < 8)`],
    /// and `Trigger_Create` copies 7 inputs from slots 2..8 [M 0x1b1720]: mux inputs = raw p1..p6 + raw p8 (p7 is
    /// overwritten and lost), mask = p9 with the default-2 fallback [M 0x1b1720:80-84], lock = p10, hide = p11.
    #[test]
    fn mux_params_read_the_shifted_slots() {
        let q = |k: u32| 0x10 + k; // param k reads as 0x10+k so each slot is distinguishable
        let t = Trigger::from_params(0, kind::AND, q, Vec::new()).unwrap();
        assert_eq!(t.out, 0x10, "out = p0");
        assert_eq!(t.list, vec![0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x18], "inputs = p1..p6, p8 (p7 lost to the wrapper)");
        assert_eq!(t.mask, 0x19, "mask = p9");
        assert_eq!(t.lock, 0x1a, "lock = p10");
        assert!(t.hide, "hide = p11 (0x1b != 0)");
        // Sister: a zero mask reads as the game's default 2 [M 0x1b1720:80-84].
        let t = Trigger::from_params(0, kind::AND, |_| 0, Vec::new()).unwrap();
        assert_eq!(t.mask, 2, "Trigger_Create defaults a zero mask to 2");
        // LOAD_LEVEL keeps its own mapping and the hard 0xE mask [M 0x1b2950: addiu t0,zero,14 → tag+0x50].
        let t = Trigger::from_params(0, kind::LOAD_LEVEL, q, Vec::new()).unwrap();
        assert_eq!(t.mask, 0xE, "Trigger_LoadLevelCreate forces mask 0xE");
        assert_eq!(t.next_level, 0x10, "next level = p0");
    }

    /// Case 3 [M 0x1b1a78:91-103]: the AND accumulator starts at 1 — an empty input list writes 1 — and the scan
    /// is a prefix: a zero input slot ends it (a deliberate 0 mid-list hides the inputs after it). Sister: the OR
    /// starts at 0.
    #[test]
    fn and_accumulator_starts_1_and_scans_the_prefix() {
        let mut ch = Channels::default();
        ch.set(0x20, 1, 1);
        // Empty list → AND writes 1 (the port used to write 0).
        let mut empty = plate(kind::AND, 0x21, 0, vec![]);
        empty.update(&mut ch, &[], 2);
        assert!(ch.on(0x21), "an empty MuxIn latches its output ON (0x1b1cbc)");
        // Mid-list zero: inputs after it are invisible to the scan.
        ch.set(0x22, 1, 3);
        let mut mid = Trigger { kind: kind::AND, out: 0x23, list: vec![0x20, 0, 0x22], ..plate(kind::AND, 0, 0, vec![]) };
        mid.update(&mut ch, &[], 4);
        assert!(ch.on(0x23), "AND stops at the zero slot: only the prefix matters");
        // The same list on an OR: the zero slot also ends the scan (0x22 ignored).
        let mut orm = Trigger { kind: kind::OR, out: 0x24, list: vec![0, 0x22], ..plate(kind::OR, 0, 0, vec![]) };
        orm.update(&mut ch, &[], 5);
        assert!(!ch.on(0x24), "OR over an all-zero prefix writes 0");
    }

    /// Case 4's window predicate is `time[prev] − 4 <= time[cur]` [M 0x1b1e04-0x1b1e1c] — the current input may be
    /// up to 4 frames OLDER than the previous one and there is NO upper bound. The port's old test
    /// (`b >= a && b - a <= 4`) rejected the >4-frame gaps the game accepts.
    #[test]
    fn seq_and_window_is_lower_bound_only() {
        let mut ch = Channels::default();
        // A 100-frame gap between input 1 and input 2: the game fires, the old port never did.
        ch.set(0x30, 1, 10);
        ch.set(0x31, 1, 110);
        let mut seq = Trigger { kind: kind::SEQ_AND, out: 0x32, list: vec![0x30, 0x31], ..plate(kind::SEQ_AND, 0, 0, vec![]) };
        seq.update(&mut ch, &[], 111);
        assert!(ch.on(0x32), "no upper bound on the SEQ_AND window");
        // Up to 4 frames of skew the other way is also accepted (cur older than prev).
        let mut ch = Channels::default();
        ch.set(0x30, 1, 50);
        ch.set(0x31, 1, 47); // 4 frames older: 50-4 <= 47
        let mut seq = Trigger { kind: kind::SEQ_AND, out: 0x32, list: vec![0x30, 0x31], ..plate(kind::SEQ_AND, 0, 0, vec![]) };
        seq.update(&mut ch, &[], 51);
        assert!(ch.on(0x32), "the current input may be up to 4 frames older");
        // Sister: more than 4 frames of skew the other way fails.
        let mut ch = Channels::default();
        ch.set(0x30, 1, 50);
        ch.set(0x31, 1, 45); // 5 frames older: 50-4 > 45
        let mut seq = Trigger { kind: kind::SEQ_AND, out: 0x32, list: vec![0x30, 0x31], ..plate(kind::SEQ_AND, 0, 0, vec![]) };
        seq.update(&mut ch, &[], 51);
        assert!(!ch.on(0x32));
    }

    /// `Channels::set` stamps `time` on EVERY nonzero write (the game's trigger and script paths re-stamp;
    /// 0x1b1a78:87,107, trigger-port-check #4) — a re-set channel refreshes its SEQ_AND window.
    #[test]
    fn channels_restamp_on_every_nonzero_write() {
        let mut ch = Channels::default();
        ch.set(0x40, 1, 10);
        assert_eq!(ch.time[0x40], 10);
        ch.set(0x40, 1, 77); // same value, still nonzero: the stamp refreshes
        assert_eq!(ch.time[0x40], 77, "every nonzero write re-stamps");
        ch.set(0x40, 0, 90);
        assert_eq!(ch.time[0x40], 77, "a zero write does not stamp");
    }

    /// Case 1 deletes the TouchOnce only when the glist flags carry 0x10 [M 0x1b1d78-0x1b1d90]; without the bit
    /// the trigger stays live (and case 1 keeps falling through into case 2 while the channel is off).
    #[test]
    fn touch_once_delete_needs_the_glist_flag() {
        let mut ch = Channels::default();
        let on = [Toucher::player([0.5, -0.3, 0.0])];
        let away = [Toucher::player([5.0, 0.0, 5.0])];
        // Latch via touch, leave, then re-arm the channel externally: with the flag the plate deletes itself.
        let mut t = plate(kind::TOUCH_ONCE, 0x50, 8, vec![]);
        t.update(&mut ch, &on, 1);
        assert!(ch.on(0x50));
        t.update(&mut ch, &away, 2);
        assert!(t.gone, "glist 0x10 set: the TouchOnce deletes itself");
        // Without the glist bit the delete is refused (the port models every TouchOnce with the bit — live data).
        let mut ch2 = Channels::default();
        ch2.set(0x50, 1, 1);
        let mut keep = Trigger { glist_0x10: false, ..plate(kind::TOUCH_ONCE, 0x50, 8, vec![]) };
        keep.update(&mut ch2, &away, 2);
        assert!(!keep.gone, "no glist 0x10: the trigger survives");
    }

    /// `GetLevelToLoadFromMovieCode` (0x1d3560): the story cut-scene codes map to the NEXT level's
    /// briefing code, the driving interludes map to the front end, and everything else falls back to
    /// `GameState+8 & 0xff00ffff` [M the decomp's every arm]. The numbers are the task's chain table:
    /// 06→07, 09→0a, 0a→0b, 11→12, 12→4a, 4a→13 ride exactly these pairs.
    #[test]
    fn movie_code_table_matches_the_decomp() {
        // The story cut-scene pairs (the movie AFTER level N loads level N+1) [M 0x1d3560:52-96].
        for (code, want) in [
            (0x0720_0001u32, 0x0710_0002), (0x0720_0002, 0x0710_0003), (0x0720_0003, 0x0710_0004),
            (0x0720_0005, 0x0710_0006), (0x0720_0006, 0x0710_0007), (0x0720_0007, 0x0710_0008),
            (0x0720_0009, 0x0710_000a), (0x0720_000a, 0x0710_000b), (0x0720_000c, 0x0710_000d),
            (0x0720_000e, 0x0710_000f), (0x0720_0011, 0x0710_0012), (0x0720_0012, 0x0710_004a),
            (0x0720_0014, 0x0710_0015), (0x0720_0015, 0x0710_0016), (0x0720_004a, 0x0710_0013),
        ] {
            assert_eq!(movie_code_next_level(code, 0x0bad_f00d), want, "movie {code:08x}");
        }
        // The driving interludes all land on the front end [M 0x1d3560:24-50].
        for code in [0x0730_0004u32, 0x0730_0008, 0x0730_000b, 0x0730_000d, 0x0730_000f, 0x0730_0010, 0x0730_0013, 0x0730_0017, 0x0730_001b] {
            assert_eq!(movie_code_next_level(code, 0x0bad_f00d), 0x0700_0048, "driving {code:08x}");
        }
        // Sisters: an unknown code and a plain level code fall back to the masked pending load.
        assert_eq!(movie_code_next_level(0x0720_0099, 0x0700_0004), 0x0700_0004, "unknown 0x72 → the masked fallback");
        assert_eq!(movie_code_next_level(0x0710_0006, 0), 0, "0x71 codes have no arm — the caller's fallback");
    }

    /// `resolve_exit_code`: the port's compression of `ResetMap_LevelToLoad` + `ResetMap_Load`'s
    /// phase-0/phase-2 cycle [M 0x1d3800/0x1d6360/0x1d3560]. The numbers are the task's chains:
    /// 4a's cut-scene request lands on 13, a driving exit lands on the front end, and a plain
    /// 0xE8 hashcode passes through untouched.
    #[test]
    fn resolve_exit_code_walks_the_game_pipeline() {
        assert_eq!(resolve_exit_code(0x0720_004a), 0x0700_0013, "4a's cut-scene movie → 13 (0x720004a → 0x7100013 → masked)");
        assert_eq!(resolve_exit_code(0x0720_0006), 0x0700_0007, "06's cut-scene movie would land on 07");
        assert_eq!(resolve_exit_code(0x0730_0013), 0x0700_0048, "13's driving interlude → the front end");
        assert_eq!(resolve_exit_code(0x0710_0006), 0x0700_0006, "a briefing wrapper masks to its level");
        assert_eq!(resolve_exit_code(0x0700_0007), 0x0700_0007, "a plain exit hashcode passes through");
        // Sister: the 0x720004a request the 4a SP actually fires is resolvable end to end.
        assert_eq!(resolve_exit_code(0x0720_004a), 0x0700_0013);
    }
}
