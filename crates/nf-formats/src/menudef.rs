//! The front end's menu-def: the kind-8 LevelBin entry (08000002 in 07000048.bin) parsed into
//! pages/controls/scripts/keyframes/messages, plus the game's `Script_*` keyframe runtime as a
//! pure data/sim layer (nf-view draws from it; no GPU code here).
//!
//! Evidence (all [M] — ACTION.ELF decompile under `work/decomp/ACTION.ELF/`):
//! - The tag walk is `MenuManager_Create` 0x21b908 (BIN_GetDWord/Word/Byte order verified against
//!   the disassembly at 0x1158xx-0x1166xx): PAGE −0x10 {u32 id, u32 file hash, u32 aux, u8 kind,
//!   u32 size} (kind 0/3 + hash match → page, else skip `size` bytes); CONTROL −0x07 {u32 id, u8 b,
//!   u8 type, u16 x, y, w, h, u16 f1, u32 d1, u16 f2, u32 d2, u32 d3}; NAME −0x08 (32-byte string);
//!   SCRIPT −0x09 {u32 id, u32 flags → script+0x1c, disasm 0x115f84-0x115fa0}; MSG −0x0a {u8 id,
//!   u32 a, u32 b} (0x1a-0x1c with a == 0x7d6d59ff rewritten 0x645a49ff [0x115eac-0x115ec4]; id
//!   0x1d read and DISCARDED [0x115ed4-0x115ed8]); KFRAME −0x0b {u16 time, u16 x, y, w, h, u32
//!   flags} (disasm 0x115fbc-0x116004: first u16 → kf+0x18 time, trailing u32 → kf+0x1c flags);
//!   KMSG −0x0c {u32 target, u8 id, u32 a, u32 b} (disasm 0x116028-0x116058: the FIRST dword lands
//!   in M_MESSAGE+0x8 — the TARGET slot, not a time; keyframe messages fire when their keyframe's
//!   time passes so they carry no own time — the older "{u32 time}" reading in docs/systems/
//!   frontend.md and menu.rs's header was wrong and is corrected there); MGRMSG −0x0d {u8, u32,
//!   u32}; SET −0x03 (u16); COMPONENT −0x04 {u16 count, 3×u16, u32 hash}; SPRITE −0x05 {8×u16,
//!   u32, u32, f32, f32}; SKIP −0x02 (u32); END −0x0e. An unknown tag ENDS the walk (the switch's
//!   default sets iStack_a8 = 0 [0x21b908:196-197]).
//! - Control fixups at parse [M 0x21b908:305-314]: type 5 && x<1 && y<1 && w>0x27f && h>0x1df →
//!   w=0x200 h=0x1c0; id 0x10000224 → (340,198); type != 0xd → `FixupResolution` 0x21b610. The
//!   port does NOT bake that scale: it keeps the authored 640×480 space (the transcription's
//!   space) and the overlay maps ×512/640 ×448/480 at draw time. That is exact for this runtime:
//!   `FixupResolution` on a KEYFRAME (0x21b7a8) is a pure per-axis affine — x·0.8, y·0.9333334
//!   (the centre terms cancel because w' = 0.8·w, h' = 0.9333334·h; w/h clamp to ≥ 2 AFTER
//!   scaling) — and both interpolators are affine-invariant (the Catmull-Rom basis sums to 1).
//! - Keyframes attach to the current SCRIPT, sorted by time, dedup by time unless the existing
//!   frame's flags bit 0x20 is set [M `Script_AddKeyFrame` 0x2201c8:24-38].
//! - Runtime state is on the control [M `Script_PrepareScripting` 0x220ed8]: script list +0x28,
//!   current script +0x38, segment-end keyframe +0x3c, loop-return keyframe +0x40 / index +0x44,
//!   segment-start index +0x4c (init −1), last message result +0x50, frame timer +0x54, flags
//!   +0x58, delayed-process list +0x5c, animated rect +0x70..+0x77 (x, y, w, h shorts).
//! - `Script_RunFrame` 0x220580 (from `Page_Update` 0x21edc8:84 per control per frame):
//!   t = idx + (timer − prev.time)/(cur.time − prev.time) (t = idx+1 when the times tie
//!   [0x220580:27-41]); the interpolators clamp t to [0, count−1], take i = trunc(t),
//!   frac = t − i [M `Script_InterpolateLine` 0x220bc8 — disassembly 0x11b3c8 shows
//!   `Script_CalculateLine` 0x2214c8 = p1 + (p2−p1)·t, i.e. LINEAR between kf[i] and kf[i+1];
//!   `Script_CalculateSpline` 0x221410 is Catmull-Rom over kf[i−1], kf[i], kf[i+1], kf[i+2] with
//!   the i−1 slot wrapped to count−1 when i = 0 (the unsigned min [0x220bc8:53-56])]; then when
//!   the timer has reached the segment-end keyframe's time: a kf-flags bit 2 keyframe is recorded
//!   as the loop return and the script is armed (script flags |= 8) [0x220580:54-62]; a kf-flags
//!   bit 4 keyframe returns to the recorded point when armed (+0x58 |= 0x10) and otherwise plays
//!   on [0x220580:121-131]; each passed keyframe fires its messages [0x220580:73-120]; timer++.
//! - Messages [M 0x220580:83-116]: id < 0xe1 sent immediately (target −3 = the manager, −2 = the
//!   control itself, a == −4 in arg A = the control's last message result, else control-by-id via
//!   `__Menu_Send` 0x1f29c0); id ≥ 0xe1 become delayed processes (`Process_Create` 0x21f7a8) with
//!   delay = next keyframe's time − timer − 1 (16-bit wrapped, 15 when there is no next
//!   [0x220580:105-109]), attached to the TARGET control (resolved with msg 0x39, which returns
//!   the control pointer [M Label_SendMessage 0x218890:157-159]); id 0xe1 captures the target's
//!   current colour with msg 0x30 (the colour msg 0x1a wrote [M 0x218890:66, 148-150]) and
//!   `Process_RunFrame` 0x21f380 blends each byte channel start→target over the delay and sends
//!   msg 0x1a (disassembly 0x1195ec/0x119634: the message id 0x1a is hardcoded at BOTH call
//!   sites — every delayed process fires a colour set).
//! - Scripts play from `Script_Play` 0x212a8 (mode 0 = from the first keyframe: cur = first,
//!   loop-return = first, idx = −1, timer = 0) and `Script_PlayDefault` 0x220350: msg → default
//!   slot {0x4c/0x51→0, 0x4d/0x4e→1, 0x4b/0x63→2, 0x49→3, 0x4f→4, 0x5b→5}; msgs 0x4c/0x4d/0x63
//!   run the PAGE's table on the page, the rest the control's own table (type 0xd windows skip);
//!   the tables bind via msg 0x21 {slot, script id} in each widget's SendMessage
//!   [M Label 0x218890:89-96, Window 0x224190:19-26]. The page-open chain is Manager msg 0x44 →
//!   `Script_PlayDefault(page, 0x4c)` [M `Manager_SendMessage` 0x21c858:346].
//! - `Script_Select` 0x221250 sets the current script by id (no keyframe reset);
//!   `Script_ExitLoop` 0x2213d0 clears control flags bit 0x8; `Script_Stop` 0x221400.
//!
//! Coordinates are the authored 640×480 menu space throughout (the port's `menu.rs` maps them to
//! the 512×448 screen at draw time).

/// One parsed menu-def tag stream.
#[derive(Debug, Default, Clone)]
pub struct MenuDef {
    pub pages: Vec<PageDef>,
    /// Every PAGE tag seen, including ones skipped by hash/kind (the game's 56-page count
    /// counts these [D the 2026-10-13 census]; `pages` holds only the registered ones).
    pub pages_seen: usize,
}

impl MenuDef {
    /// Parse a kind-8 menu-def entry (the whole LevelBin entry body).
    /// `file_hash` is the hash `MenuManager_Create` was called with (0x08000002 for the front
    /// end [M bootup_bootup 0x1c7e78:24 — GameState+0x20]); PAGE records whose hash differs are
    /// skipped by `size` bytes exactly as the game does.
    pub fn parse(data: &[u8], file_hash: u32) -> Result<MenuDef, String> {
        let mut p = Parser { data, pos: 0 };
        // [M 0x21b908 disasm 0x11598c-0x11599c + the switch entry 0x1159b8-0x1159e0]: the walk
        // consumes ONE leading dword before the tag switch — a format word (0xfffffffa on the
        // disc's 08000002). The parse skips it the same way.
        if data.len() < 4 {
            return Err("menu-def shorter than its format word".into());
        }
        p.pos = 4;
        let mut def = MenuDef::default();
        let mut page: Option<usize> = None;
        let mut ctrl: Option<usize> = None;
        let mut script: Option<usize> = None;
        let mut kf: Option<usize> = None;
        // bVar3 in 0x21b908: true while NO control is open — the game still consumes the tags'
        // bytes but drops the registrations.
        loop {
            if p.pos + 4 > data.len() {
                return Err(format!("menu-def truncated at {}", p.pos));
            }
            let tag = p.dword() as i32;
            match tag {
                -0x10 => {
                    // PAGE
                    let id = p.dword();
                    let fh = p.dword();
                    let aux = p.dword();
                    let kind = p.byte();
                    let size = p.dword() as usize;
                    def.pages_seen += 1;
                    if fh == file_hash && (kind == 0 || kind == 3) {
                        def.pages.push(PageDef { id, file_hash: fh, aux, kind: kind as u8, ..Default::default() });
                        page = Some(def.pages.len() - 1);
                        ctrl = None;
                        script = None;
                        kf = None;
                    } else if p.pos + size > data.len() {
                        return Err(format!("PAGE 0x{id:08x} size {size} overruns the def"));
                    } else {
                        p.pos += size;
                    }
                }
                -0x07 => {
                    // CONTROL — finalize of the previous control is the game's msg 0x51 (runtime
                    // side only); parse-side we just open the new one.
                    let id = p.dword();
                    let b = p.byte();
                    let ctype = p.byte();
                    let mut x = p.word() as i16;
                    let mut y = p.word() as i16;
                    let mut w = p.word() as i16;
                    let mut h = p.word() as i16;
                    let f1 = p.word();
                    let d1 = p.dword();
                    let f2 = p.word();
                    let d2 = p.dword();
                    let d3 = p.dword();
                    // The game's fixups [M 0x21b908:305-314] — the two id-shaped ones are kept
                    // (they move authored data), the resolution scale is NOT (see module docs).
                    if ctype == 5 && x < 1 && y < 1 && w > 0x27f && h > 0x1df {
                        w = 0x200;
                        h = 0x1c0;
                    } else if id == 0x1000_0224 {
                        x = 340;
                        y = 198;
                    }
                    let pg = match page {
                        Some(pg) => pg,
                        None => continue, // control outside any matching page: dropped
                    };
                    def.pages[pg].controls.push(ControlDef {
                        id, b: b as u8, ctype: ctype as u8, x, y, w, h, f1, d1, f2, d2, d3,
                        ..Default::default()
                    });
                    let registered = ctype == 0 || ctype == 3;
                    if registered {
                        ctrl = Some(def.pages[pg].controls.len() - 1);
                        script = None;
                        kf = None;
                    } else {
                        ctrl = None;
                        script = None;
                        kf = None;
                    }
                }
                -0x08 => {
                    // NAME: 32-byte string → msg 0x23 (the wide→narrow rewrite applies only when
                    // it equals the game's DAT_0030d610 cursor string — not modelled).
                    if p.pos + 32 > data.len() {
                        return Err("NAME overruns".into());
                    }
                    let mut end = 32;
                    for (i, c) in data[p.pos..p.pos + 32].iter().enumerate() {
                        if *c == 0 { end = i; break; }
                    }
                    let name = String::from_utf8_lossy(&data[p.pos..p.pos + end]).into_owned();
                    p.pos += 32;
                    if let (Some(pg), Some(ct)) = (page, ctrl) {
                        def.pages[pg].controls[ct].name = Some(name);
                    }
                }
                -0x09 => {
                    // SCRIPT {id, flags}
                    let id = p.dword();
                    let flags = p.dword();
                    if let (Some(pg), Some(ct)) = (page, ctrl) {
                        let c = &mut def.pages[pg].controls[ct];
                        c.scripts.push(ScriptDef { id, flags, keyframes: Vec::new() });
                        script = Some(c.scripts.len() - 1);
                        kf = None;
                    } else {
                        script = None;
                        kf = None;
                    }
                }
                -0x0a => {
                    // MSG {id, a, b}
                    let id = p.byte();
                    let mut a = p.dword();
                    let b = p.dword();
                    if (0x1a..=0x1c).contains(&id) && a == 0x7d6d_59ff {
                        a = 0x645a_49ff;
                    }
                    if let (Some(pg), Some(ct)) = (page, ctrl) {
                        let c = &mut def.pages[pg].controls[ct];
                        if id == 0x1d {
                            // read and discarded [M 0x115ed4-0x115ed8]
                            continue;
                        }
                        c.msgs.push((id as u8, a, b));
                        match id {
                            // [M Label 0x218890:42-64] msg 0x18: a == 0 → the label is
                            // Txt_BindLabel(b)'s id in B; a != 0 → a raw string pointer.
                            0x18 => c.label = Some(if a == 0 { b } else { a }),
                            0x24 => c.texture = Some(a),
                            0x1a => c.colour = Some(a),
                            0x21 => {
                                // bind: defaults[slot] = the script with id b, if it exists yet
                                // [M Script_Select 0x221250 via the widgets' case 0x21]
                                let slot = (a & 0xff) as usize;
                                if slot < 6 {
                                    c.defaults[slot] =
                                        c.scripts.iter().position(|s| s.id == b).map(|ix| ix as u32);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                -0x0b => {
                    // KFRAME {time, x, y, w, h (u16 each), flags (u32)}
                    let time = p.word();
                    let x = p.word() as i16;
                    let y = p.word() as i16;
                    let w = p.word() as i16;
                    let h = p.word() as i16;
                    let flags = p.dword();
                    if let (Some(pg), Some(ct), Some(sc)) = (page, ctrl, script) {
                        let s = &mut def.pages[pg].controls[ct].scripts[sc];
                        // dedup by time unless the existing frame's 0x20 flag [M 0x2201c8:24-38]
                        // [M Script_AddKeyFrame 0x2201c8:24-38]: same time + existing 0x20 →
                        // the NEW frame is DROPPED (the game's parser current-kf pointer becomes
                        // NULL, so a following KMSG attaches nowhere); same time otherwise → the
                        // existing frame is updated in place.
                        let mut done: Option<Option<usize>> = None;
                        for (ki, k) in s.keyframes.iter_mut().enumerate() {
                            if k.time == time {
                                if (k.flags & 0x20) != 0 {
                                    done = Some(None);
                                } else {
                                    k.flags = flags;
                                    k.x = x; k.y = y; k.w = w; k.h = h;
                                    done = Some(Some(ki));
                                }
                                break;
                            }
                        }
                        match done {
                            Some(ki) => { kf = ki; }
                            None => {
                                s.keyframes.push(KeyframeDef { time, x, y, w, h, flags, msgs: Vec::new() });
                                kf = Some(s.keyframes.len() - 1);
                            }
                        }
                    }
                }
                -0x0c => {
                    // KMSG {target, id, a, b} — attaches to the CURRENT keyframe
                    let target = p.dword() as i32;
                    let id = p.byte();
                    let a = p.dword();
                    let b = p.dword();
                    if let (Some(pg), Some(ct), Some(sc), Some(kfi)) = (page, ctrl, script, kf) {
                        def.pages[pg].controls[ct].scripts[sc].keyframes[kfi]
                            .msgs.push(KfMsg { target, id, a, b });
                    }
                }
                -0x0d => {
                    // MGRMSG {msg, a, b} — manager-level message
                    let msg = p.byte();
                    let a = p.dword();
                    let b = p.dword();
                    if let Some(pg) = page {
                        def.pages[pg].mgr_msgs.push((msg as u8, a, b));
                    }
                }
                -0x03 => {
                    // SET (u16) — a sprite set begins; the game's Component_AddSet
                    let _n = p.word();
                    if let Some(pg) = page {
                        def.pages[pg].sets.push(SpriteSet::default());
                    }
                }
                -0x04 => {
                    // COMPONENT — [D the disc's own bytes: {u16 count, u32 hash, 4×u16}, 14 B
                    // (08000002 opens SET 1 → COMPONENT count 9 hash 0x03000042 — the
                    // button/arrow atlas — then 8 zero bytes). The hash comes right after the
                    // count, matching the C's dword-second read.]
                    let count = p.word();
                    let hash = p.dword();
                    let a = p.word();
                    let b = p.word();
                    let c = p.word();
                    let d = p.word();
                    if let Some(pg) = page {
                        def.pages[pg].sets.last_mut().map(|s| s.components.push(ComponentDef { count, hash, a, b, c, d }));
                    }
                }
                -0x05 => {
                    // SPRITE {8×u16, u32, u32, f32, f32}
                    let mut w = [0u16; 8];
                    for v in w.iter_mut() { *v = p.word(); }
                    let d1 = p.dword();
                    let d2 = p.dword();
                    let f1 = f32::from_bits(p.dword());
                    let f2 = f32::from_bits(p.dword());
                    if let Some(pg) = page {
                        def.pages[pg].sets.last_mut().map(|s| s.sprites.push(SpriteDef { w, d1, d2, f1, f2 }));
                    }
                }
                -0x02 => {
                    // SKIP (u32)
                    let _ = p.dword();
                }
                -0x0e => {
                    // END — finalize and stop
                    break;
                }
                _ => {
                    // the game's default case stops the walk [M 0x21b908:195-197]
                    let at = p.pos - 4;
                    let ctx: Vec<String> = data[at..(at + 16).min(data.len())]
                        .iter().map(|b| format!("{b:02x}")).collect();
                    return Err(format!("unknown menu-def tag {tag:#x} at {at}: {}", ctx.join(" ")));
                }
            }
        }
        Ok(def)
    }

    pub fn page(&self, id: u32) -> Option<&PageDef> {
        self.pages.iter().find(|p| p.id == id)
    }
}

#[derive(Debug, Default, Clone)]
pub struct PageDef {
    pub id: u32,
    pub file_hash: u32,
    pub aux: u32,
    pub kind: u8,
    pub controls: Vec<ControlDef>,
    pub mgr_msgs: Vec<(u8, u32, u32)>,
    pub sets: Vec<SpriteSet>,
}

#[derive(Debug, Default, Clone)]
pub struct ControlDef {
    pub id: u32,
    pub b: u8,
    pub ctype: u8,
    pub x: i16,
    pub y: i16,
    pub w: i16,
    pub h: i16,
    pub f1: u16,
    pub d1: u32,
    pub f2: u16,
    pub d2: u32,
    pub d3: u32,
    pub name: Option<String>,
    pub label: Option<u32>,
    pub texture: Option<u32>,
    pub colour: Option<u32>,
    pub msgs: Vec<(u8, u32, u32)>,
    pub scripts: Vec<ScriptDef>,
    /// The six default-script slots, as script indices into `scripts`
    /// (the game stores the pointer `Script_Select` returned [M 0x218890:96]; 0 = unbound).
    pub defaults: [Option<u32>; 6],
}

#[derive(Debug, Default, Clone)]
pub struct ScriptDef {
    pub id: u32,
    pub flags: u32,
    pub keyframes: Vec<KeyframeDef>,
}

#[derive(Debug, Default, Clone)]
pub struct KeyframeDef {
    pub time: u16,
    pub x: i16,
    pub y: i16,
    pub w: i16,
    pub h: i16,
    pub flags: u32,
    pub msgs: Vec<KfMsg>,
}

/// A keyframe message [M M_MESSAGE layout from `Script_RunFrame` 0x220580's reads:
/// +0x8 target, +0xc id, +0x10 a, +0x14 b].
#[derive(Debug, Default, Clone)]
pub struct KfMsg {
    pub target: i32,
    pub id: u32,
    pub a: u32,
    pub b: u32,
}

#[derive(Debug, Default, Clone)]
pub struct SpriteSet {
    pub components: Vec<ComponentDef>,
    pub sprites: Vec<SpriteDef>,
}

#[derive(Debug, Default, Clone)]
pub struct ComponentDef {
    pub count: u16,
    pub hash: u32,
    pub a: u16,
    pub b: u16,
    pub c: u16,
    pub d: u16,
}

#[derive(Debug, Default, Clone)]
pub struct SpriteDef {
    pub w: [u16; 8],
    pub d1: u32,
    pub d2: u32,
    pub f1: f32,
    pub f2: f32,
}

struct Parser<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn byte(&mut self) -> u32 {
        let v = self.data[self.pos] as u32;
        self.pos += 1;
        v
    }
    fn word(&mut self) -> u16 {
        let v = u16::from_le_bytes(self.data[self.pos..self.pos + 2].try_into().unwrap());
        self.pos += 2;
        v
    }
    fn dword(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.data[self.pos..self.pos + 4].try_into().unwrap());
        self.pos += 4;
        v
    }
}

// ---------------------------------------------------------------------------
// The keyframe runtime [M Script_RunFrame 0x220580 and friends]
// ---------------------------------------------------------------------------

/// What one control would draw at the current tick [M the game's control fields the renderer
/// reads: the animated rect +0x70..+0x77 (falls back to the authored rect while no script is
/// active), the current colour (msg 0x1a/0x30), the bound label (msg 0x18) and texture (0x24)].
#[derive(Debug, Clone, PartialEq)]
pub struct DrawItem {
    pub control_id: u32,
    pub ctype: u8,
    /// (x, y, w, h) in the authored 640×480 space.
    pub rect: (i16, i16, i16, i16),
    /// 0xRRGGBBAA, the colour msg 0x1a last set (the parse-time msg 0x1a value when none did).
    pub colour: u32,
    pub label: Option<u32>,
    pub texture: Option<u32>,
    /// True while a script's timer still runs or a delayed process is pending.
    pub animating: bool,
}

/// One delayed process [M `Process_Create` 0x21f7a8: the 0x24-byte record].
#[derive(Debug, Clone)]
struct Proc {
    tick: i32,
    id: u32,
    a: u32,
    b: u32,
    start_colour: u32,
    delay: i32,
}

/// Per-control runtime state [M the offsets in `Script_PrepareScripting` 0x220ed8].
#[derive(Debug, Clone)]
struct CtrlState {
    ctrl_ix: usize,
    script: Option<usize>,
    /// +0x3c — the segment's END keyframe index; None = no script running.
    cur: Option<usize>,
    /// +0x40/+0x44 — loop-return keyframe + index.
    loop_kf: Option<usize>,
    loop_idx: i32,
    /// +0x4c — the segment's START keyframe index, init −1.
    idx: i32,
    /// +0x50 — last message result (a == −4 substitutes it).
    last_result: i32,
    /// +0x54 — frames since the script started.
    timer: i32,
    /// +0x58 — control flags (0x10 set on a loop step; bit 0x8 cleared by ExitLoop).
    flags: u32,
    /// +0x5c — delayed processes.
    procs: Vec<Proc>,
    /// +0x70..+0x77 — the animated rect.
    rect: (i16, i16, i16, i16),
    /// The colour msg 0x1a last set (what msg 0x30 returns [M 0x218890:148-150]).
    colour: u32,
}

/// The per-page keyframe runtime. Build one per page, drive it with [`MenuRuntime::page_shown`],
/// [`MenuRuntime::focus_msg`] and [`MenuRuntime::tick`], read [`MenuRuntime::draw_list`].
pub struct MenuRuntime<'a> {
    page: &'a PageDef,
    state: Vec<CtrlState>,
    /// Per-control, per-script "loop armed" bit — the game keeps it on script+0x1c |= 8
    /// [M 0x220580:60].
    armed: Vec<Vec<bool>>,
    /// The PAGE's own default-script table (script indices; the page is a control whose table
    /// lives at page+0x7c [M 0x220350:46]).
    page_default: [Option<usize>; 6],
}

impl<'a> MenuRuntime<'a> {
    /// `Script_PrepareScripting` 0x220ed8 for every control (the page itself included — the page
    /// is a type-8 control [M `Page_Init` 0x21f1c0:23]) + the parse-time colour defaults.
    pub fn new(page: &'a PageDef) -> MenuRuntime<'a> {
        let mut state = Vec::new();
        // The page itself first; children follow in tag order.
        state.push(CtrlState {
            ctrl_ix: usize::MAX,
            script: None, cur: None, loop_kf: None, loop_idx: 0, idx: -1,
            last_result: 0, timer: 0, flags: 0, procs: Vec::new(),
            rect: (0, 0, 0x280, 0x1e0), // Page_Init 0x21f1c0:25-26
            colour: 0xffff_ffff,
        });
        for (ix, c) in page.controls.iter().enumerate() {
            state.push(CtrlState {
                ctrl_ix: ix,
                script: None, cur: None, loop_kf: None, loop_idx: 0, idx: -1,
                last_result: 0, timer: 0, flags: 0, procs: Vec::new(),
                rect: (c.x, c.y, c.w, c.h),
                colour: c.colour.unwrap_or(0xffff_ffff),
            });
        }
        let armed = page.controls.iter().map(|c| vec![false; c.scripts.len()]).collect();
        MenuRuntime { page, state, armed, page_default: [None; 6] }
    }

    /// Bind the PAGE's default-script table (script indices into `PageDef::controls[i]`'s
    /// scripts — built by the caller from the page's own msg-0x21 records).
    pub fn bind_page_defaults(&mut self, table: [Option<usize>; 6]) {
        self.page_default = table;
    }

    /// `Script_PlayDefault` 0x220350 on the page (msg 0x4c = page shown [M 0x21c858:346],
    /// 0x4d/0x63 = the page's slots 1/2).
    pub fn page_shown(&mut self, msg: u8) {
        self.play_default(0, msg);
    }

    /// `Script_PlayDefault` 0x220350 on a control (focus changes: 0x4e gained / 0x4f lost
    /// [M `Page_Update` 0x21edc8:45-46]; 0x5b etc.). `ctrl_ix` is the index into
    /// `PageDef::controls`. Unknown msgs are ignored (the game's default arm returns 0).
    pub fn focus_msg(&mut self, ctrl_ix: usize, msg: u8) {
        self.play_default(ctrl_ix + 1, msg);
    }

    /// `Script_Play` 0x212a8 mode 0 on a control's script by id.
    pub fn play_script(&mut self, ctrl_ix: usize, script_id: u32) -> bool {
        let st_ix = ctrl_ix + 1;
        let Some(s_ix) = self.page.controls[ctrl_ix].scripts.iter().position(|s| s.id == script_id) else {
            return false;
        };
        self.start_script(st_ix, s_ix);
        true
    }

    /// `Script_ExitLoop` 0x2213d0 on a control: clears control-flags bit 0x8. (The decompile
    /// links this bit to the loop gate only as separate facts — the gate itself reads the
    /// SCRIPT's armed bit 8 [0x220580:122] — so no cycle-ending claim is made here.)
    pub fn exit_loop(&mut self, ctrl_ix: usize) {
        self.state[ctrl_ix + 1].flags &= !0x8;
    }

    /// `Script_Stop` 0x221400: control+0x3c = 0 — the script ends this frame; the rect freezes.
    pub fn stop_script(&mut self, ctrl_ix: usize) {
        self.state[ctrl_ix + 1].cur = None;
    }

    /// `Script_Select` 0x221250: select the script by id WITHOUT resetting the keyframes.
    pub fn select_script(&mut self, ctrl_ix: usize, script_id: u32) -> bool {
        let st_ix = ctrl_ix + 1;
        let Some(s_ix) = self.page.controls[ctrl_ix].scripts.iter().position(|s| s.id == script_id) else {
            return false;
        };
        self.state[st_ix].script = Some(s_ix);
        true
    }

    /// `Script_PlayDefault` 0x220350: msg → slot {0x4c/0x51→0, 0x4d/0x4e→1, 0x4b/0x63→2,
    /// 0x49→3, 0x4f→4, 0x5b→5}; 0x4c/0x4d/0x63 run the PAGE's table on the page, the rest the
    /// control's own (type 0xd windows skip).
    fn play_default(&mut self, st_ix: usize, msg: u8) {
        let slot = match msg {
            0x4c | 0x51 => 0,
            0x4d | 0x4e => 1,
            0x4b | 0x63 => 2,
            0x49 => 3,
            0x4f => 4,
            0x5b => 5,
            _ => return,
        };
        let page_path = msg == 0x4c || msg == 0x4d || msg == 0x63;
        if page_path {
            // the PAGE's table, run on the page [M 0x220350:42-46 Window_Get]
            let s_ix = self.page_default[slot];
            if let Some(s_ix) = s_ix {
                self.start_page_script(s_ix);
            }
        } else {
            let c_ix = self.state[st_ix].ctrl_ix;
            if c_ix == usize::MAX || self.page.controls[c_ix].ctype == 0xd {
                return; // 0x220350:98-100
            }
            if let Some(s_ix) = self.page.controls[c_ix].defaults[slot] {
                let s_ix = s_ix as usize;
                if s_ix < self.page.controls[c_ix].scripts.len() {
                    self.start_script(st_ix, s_ix);
                }
            }
        }
    }

    /// One `Script_RunFrame` 0x220580 over every control (the page included).
    pub fn tick(&mut self) {
        for st_ix in 0..self.state.len() {
            self.run_frame(st_ix);
        }
    }

    fn run_frame(&mut self, st_ix: usize) {
        // Process_RunFrame 0x21f380 first: delayed processes tick on their owner.
        self.run_procs(st_ix);
        let Some(cur) = self.state[st_ix].cur else { return };
        let c_ix = self.state[st_ix].ctrl_ix;
        let (kfs, sflags) = match self.script_of(st_ix) {
            Some(x) => x,
            None => return,
        };
        let _ = c_ix;
        if cur >= kfs.len() {
            return;
        }
        // t = idx + (timer − prev.time)/(cur.time − prev.time); t = idx+1 when the times tie
        // [M 0x220580:27-41 — prev = the current keyframe's list-predecessor, itself when none].
        let prev_ix = if cur == 0 { cur } else { cur - 1 };
        let pt = kfs[prev_ix].time as i32;
        let ct = kfs[cur].time as i32;
        let timer0 = self.state[st_ix].timer;
        let t = if ct == pt {
            self.state[st_ix].idx as f32 + 1.0
        } else {
            self.state[st_ix].idx as f32 + (timer0 - pt) as f32 / (ct - pt) as f32
        };
        // The interpolators' clamp [M 0x220bc8:18-44]: t ∈ [0, count−1], i = trunc, frac = t − i.
        let count = kfs.len() as i32;
        let tc = t.clamp(0.0, (count - 1) as f32);
        let i = tc as i32;
        let frac = tc - i as f32;
        let iu = i as usize;
        // Line [M 0x2214c8 disasm 0x11b3c8: p1 + (p2−p1)·t — kf[i] → kf[i+1]]; spline
        // [M 0x221410: Catmull-Rom, p0 at min(count−1, i−1 as u32) — the i=0 wrap to the LAST
        // keyframe is the game's own unsigned min [0x220bc8:53-56]].
        let fields: [fn(&KeyframeDef) -> i16; 4] = [|k| k.x, |k| k.y, |k| k.w, |k| k.h];
        let at = |k: usize, f: &fn(&KeyframeDef) -> i16| -> f32 {
            f(&kfs[k.min(kfs.len() - 1)]) as f32
        };
        let mut out = [0i32; 4];
        for (ci, field) in fields.iter().enumerate() {
            let p1 = at(iu, field);
            let p2 = at(iu + 1, field);
            let v = if sflags & 1 != 0 {
                let p0 = at((i as u32).wrapping_sub(1) as usize, field);
                let p3 = at(iu + 2, field);
                0.5 * (p1 + p1 + frac * (p2 - p0)
                    + frac * frac * ((p0 + p0 - p1 * 5.0 + p2 * 4.0) - p3)
                    + frac * frac * frac * ((p1 * 3.0 - p2 * 3.0) + p3 - p0))
            } else {
                p1 + (p2 - p1) * frac
            };
            // stored as a short via a C float→int cast = trunc toward zero [M 0x220bc8:74]
            out[ci] = v.trunc() as i32;
        }
        self.state[st_ix].rect = (out[0] as i16, out[1] as i16, out[2] as i16, out[3] as i16);
        // Timer vs the segment-end keyframe's time [M 0x220580:50-52].
        if timer0 < ct {
            self.state[st_ix].timer = timer0 + 1;
            return;
        }
        // The keyframe whose time passed: kf-flags bit 2 arms the loop return
        // [M 0x220580:54-62], bit 4 stops there when armed [0x220580:121-131].
        let kf_flags = kfs[cur].flags;
        if kf_flags & 2 != 0 {
            self.state[st_ix].loop_kf = Some(cur);
            self.state[st_ix].loop_idx = self.state[st_ix].idx;
            if let Some(s_ix) = self.state[st_ix].script {
                if c_ix != usize::MAX {
                    if let Some(v) = self.armed.get_mut(c_ix) {
                        if s_ix < v.len() { v[s_ix] = true; }
                    }
                }
            }
        }
        if kf_flags & 4 == 0 || !self.armed_now(st_ix) {
            // advance: cur = next; when none, the script ends (cur = None → idx = 0)
            // [M 0x220580:63-72]; the passed keyframe's messages fire [0x220580:73-120].
            let next = cur + 1;
            let had_next = next < kfs.len();
            self.fire_msgs(st_ix, cur);
            self.state[st_ix].cur = if had_next { Some(next) } else { None };
            self.state[st_ix].idx = if had_next { self.state[st_ix].idx + 1 } else { 0 };
        } else {
            // loop back to the recorded point [M 0x220580:121-131]
            self.state[st_ix].flags |= 0x10;
            self.state[st_ix].idx = self.state[st_ix].loop_idx;
            let lk = self.state[st_ix].loop_kf;
            self.state[st_ix].cur = lk;
            if let Some(lk) = lk {
                self.state[st_ix].timer = kfs[lk].time as i32;
            }
        }
        self.state[st_ix].timer += 1;
    }

    fn armed_now(&self, st_ix: usize) -> bool {
        let c_ix = self.state[st_ix].ctrl_ix;
        let Some(s_ix) = self.state[st_ix].script else { return false };
        if c_ix == usize::MAX { return false; }
        self.armed.get(c_ix).and_then(|v| v.get(s_ix)).copied().unwrap_or(false)
    }

    fn script_of(&self, st_ix: usize) -> Option<(&'a [KeyframeDef], u32)> {
        let c_ix = self.state[st_ix].ctrl_ix;
        let s_ix = self.state[st_ix].script?;
        if c_ix == usize::MAX {
            return None; // the page itself carries no parsed keyframe scripts in this model
        }
        let c: &'a ControlDef = self.page.controls.get(c_ix)?;
        let s: &'a ScriptDef = c.scripts.get(s_ix)?;
        Some((&s.keyframes[..], s.flags))
    }

    fn start_page_script(&mut self, s_ix: usize) {
        // The page's own script — the parsed data binds none in this model; kept for parity.
        let st = &mut self.state[0];
        st.script = None;
        st.cur = None;
        st.idx = -1;
        st.timer = 0;
        let _ = s_ix;
    }

    /// Fire the keyframe's messages [M 0x220580:73-120].
    fn fire_msgs(&mut self, st_ix: usize, kf_ix: usize) {
        let c_ix = self.state[st_ix].ctrl_ix;
        if c_ix == usize::MAX { return; }
        let msgs: Vec<KfMsg> = {
            let (kfs, _) = match self.script_of(st_ix) {
                Some(x) => x,
                None => return,
            };
            if kf_ix >= kfs.len() { return; }
            kfs[kf_ix].msgs.clone()
        };
        let next_time = {
            let (kfs, _) = match self.script_of(st_ix) {
                Some(x) => x,
                None => return,
            };
            kfs.get(kf_ix + 1).map(|k| k.time as i32)
        };
        for m in msgs {
            if m.id < 0xe1 {
                // immediate: a == −4 → the control's last message result [M 0x220580:84-87]
                let a = if m.a as i32 == -4 { self.state[st_ix].last_result } else { m.a as i32 };
                let res = self.deliver(st_ix, m.target, m.id, a, m.b as i32);
                self.state[st_ix].last_result = res;
            } else {
                // delayed process [M 0x220580:104-116]: delay = next kf time − timer − 1
                // (16-bit wrapped) or 15 when no next; owner = the target control (msg 0x39
                // resolves the id), the control itself for target −2.
                let timer = self.state[st_ix].timer;
                let delay = match next_time {
                    Some(nt) => ((nt as u16).wrapping_sub(timer as u16).wrapping_sub(1)) as i16 as i32,
                    None => 0xf,
                };
                let owner = if m.target == -2 {
                    Some(st_ix)
                } else {
                    self.find_control(m.target as u32).map(|ix| ix + 1)
                };
                if let Some(own) = owner {
                    let start = if m.id == 0xe1 { self.state[own].colour } else { 0 };
                    self.state[own].procs.push(Proc {
                        tick: 0, id: m.id, a: m.a, b: m.b, start_colour: start, delay,
                    });
                }
            }
        }
    }

    /// Deliver a message the way `Script_RunFrame` routes it [M 0x220580:88-101]:
    /// target −3 = the manager (no handler modelled → 0), −2 = the control itself, else the
    /// control with that id. Returns the handler result (msg 0x30 = the colour, 0x39 = the
    /// control's index+1 as a stand-in pointer, others 1 on delivery).
    fn deliver(&mut self, st_ix: usize, target: i32, id: u32, a: i32, b: i32) -> i32 {
        match target {
            -3 => 0, // Manager_SendMessage — not modelled (no manager-level handler reads it)
            -2 => self.control_msg(st_ix, id, a, b),
            t => {
                let Some(ix) = self.find_control(t as u32) else { return 0 };
                self.control_msg(ix + 1, id, a, b)
            }
        }
    }

    fn control_msg(&mut self, st_ix: usize, id: u32, a: i32, _b: i32) -> i32 {
        match id {
            0x1a => { self.state[st_ix].colour = a as u32; 1 }
            0x30 => self.state[st_ix].colour as i32,
            0x39 => st_ix as i32, // the game returns the control pointer [M 0x218890:157-159]
            0x14 => { self.state[st_ix].flags &= !0x8; 1 } // Script_ExitLoop [M 0x2213d0]
            _ => 1,
        }
    }

    fn find_control(&self, id: u32) -> Option<usize> {
        self.page.controls.iter().position(|c| c.id == id)
    }

    fn start_script(&mut self, st_ix: usize, s_ix: usize) {
        // Script_Play 0x212a8 mode 0: cur = first kf, loop-return = first, idx = −1, timer = 0.
        self.state[st_ix].script = Some(s_ix);
        let c_ix = self.state[st_ix].ctrl_ix;
        let has_kf = c_ix != usize::MAX
            && !self.page.controls[c_ix].scripts[s_ix].keyframes.is_empty();
        if has_kf {
            self.state[st_ix].cur = Some(0);
            self.state[st_ix].loop_kf = Some(0);
        } else {
            self.state[st_ix].cur = None;
            self.state[st_ix].loop_kf = None;
        }
        self.state[st_ix].loop_idx = 0;
        self.state[st_ix].idx = -1;
        self.state[st_ix].timer = 0;
    }

    /// Process_RunFrame 0x21f380: blend id-0xe1 colours while ticking; on expiry send the exact
    /// target colour (the disassembly hardcodes msg 0x1a at BOTH call sites [0x1195ec/0x119634]).
    fn run_procs(&mut self, st_ix: usize) {
        let st = &mut self.state[st_ix];
        if st.procs.is_empty() {
            return;
        }
        let mut done = Vec::new();
        for (n, p) in st.procs.iter_mut().enumerate() {
            if p.id == 0xe1 {
                // per-channel start + (target − start)·tick/delay [M 0x21f380:88-103]
                st.colour = blend_colour(p.start_colour, p.a, p.tick, p.delay);
            }
            p.tick += 1;
            if p.delay as u32 <= p.tick as u32 {
                st.colour = p.a; // the exact target [M 0x21f380:109-110]
                done.push(n);
            }
        }
        for n in done.into_iter().rev() {
            st.procs.remove(n);
        }
    }

    /// The draw list at the current tick, in control-tag order (page control first).
    pub fn draw_list(&self) -> Vec<DrawItem> {
        let mut out = Vec::new();
        for st in &self.state {
            let (cid, ctype, label, texture) = if st.ctrl_ix == usize::MAX {
                (self.page.id, 8u8, None, None)
            } else {
                let c = &self.page.controls[st.ctrl_ix];
                (c.id, c.ctype, c.label, c.texture)
            };
            let animating = st.cur.is_some() || !st.procs.is_empty();
            out.push(DrawItem {
                control_id: cid,
                ctype,
                rect: st.rect,
                colour: st.colour,
                label,
                texture,
                animating,
            });
        }
        out
    }
}

/// 0xRRGGBBAA byte-channel blend [M 0x21f380:88-103 — integer bytes through f32 math].
fn blend_colour(start: u32, target: u32, tick: i32, delay: i32) -> u32 {
    let mix = |sh: u32| -> u32 {
        let s = ((start >> sh) & 0xff) as f32;
        let t = ((target >> sh) & 0xff) as f32;
        (s + (t - s) / delay as f32 * tick as f32) as i64 as u32 & 0xff
    };
    (mix(24) << 24) | (mix(16) << 16) | (mix(8) << 8) | mix(0)
}
