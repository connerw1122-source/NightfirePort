//! The script-census pin (task script-census): re-runs the census of docs/systems/script-census.md
//! through the port's own parsers over every 07* archive in FILES.BIN and pins the handled sets
//! and the disc totals as numbers. A regression that stops handling something, or a newly
//! handled opcode/kind/type, changes a constant here — update it and the doc tables together.
//! Opt-in like the goldens: set `NF_ISO`; without it every test prints "skip" and passes.

use nf_formats::iso9660::IsoImage;
use nf_formats::{anim, disc, levelbin, map, nis, placement};
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// The NIS command opcodes `Script::run` (nf-game/src/nis.rs `Script_Run` 0x1d8548 port) dispatches.
const HANDLED_NIS_OPS: usize = 20;
/// The event kinds `Script::event` (`Script_EventHandler` 0x1d94c0 port) handles.
const HANDLED_EVENT_KINDS: usize = 17;
/// The trigger action codes nf-game/src/triggers.rs `kind` module handles (game dispatches 8).
const HANDLED_TRIGGER_ACTIONS: usize = 8;
/// Placement types with any handling: 53 simulated + 8 draw-only (game: 73 spawn cases).
/// cover-nodes: 0xE5/0xE6 joined the simulated set (nf-game/src/cover.rs `from_placement` +
/// `build` [M 0x1d4880 case 0/1/2/0xb → 0x1d04f0 case 0xe5/0xe6]; batch-1016c keeps `build`
/// main-map-only, U-050). ai-points: 0xE9 joined it (nf-game/src/aipoint.rs, `Drone_AIPoint` 0x137550).
/// sensor-volumes: 0x28 joined it (crate::sensor, `Sensor_Create` 0x1af010).
/// music-triggers: 0xFB joined it (nf-game/src/worldobj.rs `MusicTrigger` — `MusicTrigger_Create`
/// 0x1b30e0's gates and `MusicTrigger_Update` 0x1b31a0's countdown / `Music_Event(p2, 2)`).
/// (batch-1016c: s/hurt-volumes dropped at the merge — 0xE4 stays unhandled.)
const HANDLED_PLACEMENT_TYPES: usize = 62;

fn census() -> Option<&'static Census> {
    static C: OnceLock<Option<Census>> = OnceLock::new();
    let c = C.get_or_init(|| {
        let path = std::env::var_os("NF_ISO")?;
        Some(census_impl(&IsoImage::open(&path).unwrap_or_else(|e| panic!("NF_ISO={path:?}: {e}"))))
    });
    c.as_ref()
}

#[derive(Default)]
struct Census {
    entry_kinds: BTreeMap<u8, u64>,
    nis_scripts: u64,
    nis_streams: u64,
    nis_ops: BTreeMap<u8, u64>,
    event_kinds: BTreeMap<u8, u64>,
    anim_scripts: u64,
    anim_cmds: BTreeMap<u8, u64>,
    anim_op4_kinds: BTreeMap<u16, u64>,
    placements: u64,
    placement_types: BTreeMap<u8, u64>,
    trigger_actions: BTreeMap<u16, u64>,
    raw_dc_triggers: u64,
}

fn census_impl(iso: &IsoImage<std::fs::File>) -> Census {
    let (files, bin) = disc::files_bin_entries(iso).unwrap();
    let mut c = Census::default();
    for f in files.iter().filter(|f| {
        f.name.len() >= 10 && f.name[..2].eq_ignore_ascii_case("07") && f.name.to_ascii_lowercase().ends_with(".bin")
    }) {
        let data = iso
            .read_bytes(bin.lba as u64 * nf_formats::iso9660::SECTOR_SIZE as u64 + f.offset as u64, f.size as u64)
            .unwrap();
        let lb = levelbin::LevelBin::parse(&data).unwrap();
        for e in lb.entries.iter() {
            *c.entry_kinds.entry(e.kind).or_default() += 1;
            let bytes = lb.data(&data, e);
            match e.kind {
                7 => {
                    let nis = nis::Nis::parse(e.hash(), bytes).unwrap();
                    c.nis_scripts += 1;
                    c.nis_streams += nis.streams.len() as u64;
                    for s in &nis.streams {
                        for cmd in s.decode() {
                            let (op, event) = match &cmd {
                                nis::Cmd::End(_) => (4, None),
                                nis::Cmd::Wait(_) => (5, None),
                                nis::Cmd::Marker(_) => (6, None),
                                nis::Cmd::Entity { .. } => (7, None),
                                nis::Cmd::EntityEnd => (9, None),
                                nis::Cmd::Anim { .. } => (0x0a, None),
                                nis::Cmd::AnimEnd => (0x0c, None),
                                nis::Cmd::Camera { .. } => (0x0d, None),
                                nis::Cmd::CameraEnd => (0x0f, None),
                                nis::Cmd::Event { kind, .. } => (0x12, Some(*kind)),
                                nis::Cmd::Sound { .. } => (0x16, None),
                                nis::Cmd::SoundEnd => (0x17, None),
                                nis::Cmd::Fade(_) => (0x13, None),
                                nis::Cmd::Sprite { .. } => (0x14, None),
                                nis::Cmd::Skip { op } => (*op, None),
                                nis::Cmd::Light { .. } => (0x18, None),
                                nis::Cmd::SubScript { .. } => (0x1b, None),
                                nis::Cmd::Text { .. } => (0x1e, None),
                                nis::Cmd::Other(op) | nis::Cmd::Unread(op) => (*op, None),
                            };
                            *c.nis_ops.entry(op).or_default() += 1;
                            if let Some(kind) = event { *c.event_kinds.entry(kind).or_default() += 1; }
                        }
                    }
                }
                5 if bytes.len() >= 8 => {
                    let script = anim::Script::parse(bytes).unwrap();
                    c.anim_scripts += 1;
                    for cmd in &script.commands {
                        *c.anim_cmds.entry(cmd.op).or_default() += 1;
                        if cmd.op == 4 {
                            if let Some(&kind) = cmd.args.get(1) { *c.anim_op4_kinds.entry(kind).or_default() += 1; }
                        }
                    }
                }
                0 | 1 | 2 | 11 => {
                    let mf = map::MapFile::parse(bytes).unwrap();
                    for block in mf.all(map::id::MAP_DATA_STATIC) {
                        for p in placement::parse(block.payload(bytes)).unwrap() {
                            if p.flags & 0xa000 != 0 { continue; }
                            c.placements += 1;
                            *c.placement_types.entry((p.flags & 0xff) as u8).or_default() += 1;
                            // `Trigger_*Create` (0x1b1720/0x1b2950/0x1b2a08/0x1b2aa8/0x1b2b88/0x1b2c68/0x1b2d38/0x1b2e08/0x1b2ed8)
                            // prepend the action code; a raw 0xDC keeps its authored param 0.
                            let action = match p.flags & 0xff {
                                0xe8 => Some(10),
                                0xea => Some(1),
                                0xeb => Some(2),
                                0xec => Some(3),
                                0xed => Some(4),
                                0xee => Some(5),
                                0xef => Some(12),
                                0xf4 => Some(13),
                                0xdc => p.params.iter().find(|(i, _)| *i == 0).map(|(_, v)| *v as u16),
                                _ => None,
                            };
                            match (p.flags & 0xff, action) {
                                (0xdc, None) => c.raw_dc_triggers += 1,
                                (_, Some(a)) => *c.trigger_actions.entry(a).or_default() += 1,
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    c
}

fn bmap(v: &[(u64, u64)]) -> BTreeMap<u64, u64> { v.iter().copied().collect() }

#[test]
fn nis_opcode_census_matches_the_doc_and_the_handled_set() {
    let Some(c) = census() else { eprintln!("skip: NF_ISO not set"); return };
    assert_eq!(c.nis_scripts, 468, "NIS scripts (type-7 entries, level+pack archives)");
    assert_eq!(c.nis_streams, 5263, "NIS streams (nis.rs:120-125 census)");
    // All 20 opcodes that occur are the 20 the runtime handles; nothing unknown survives decode.
    assert_eq!(c.nis_ops.len(), HANDLED_NIS_OPS, "distinct opcodes on the disc");
    assert_eq!(c.nis_ops.keys().max().copied(), Some(0x1e));
    assert_eq!(*c.nis_ops.get(&4).unwrap(), 5263, "one End per stream");
    assert_eq!(*c.nis_ops.get(&5).unwrap(), 105661, "Wait");
    assert_eq!(*c.nis_ops.get(&6).unwrap(), 105661, "Marker (pairs Wait 1:1)");
    assert_eq!(*c.nis_ops.get(&7).unwrap(), 4985, "Entity");
    assert_eq!(*c.nis_ops.get(&0x12).unwrap(), 851, "Event");
    assert_eq!(*c.nis_ops.get(&0x13).unwrap(), 3, "Fade (rarest handled op)");
}

#[test]
fn event_kind_census_matches_the_doc_and_the_handled_set() {
    let Some(c) = census() else { eprintln!("skip: NF_ISO not set"); return };
    assert_eq!(c.event_kinds.values().sum::<u64>(), 851, "event commands on the disc");
    assert_eq!(bmap(&c.event_kinds.iter().map(|(&k, &v)| (k as u64, v)).collect::<Vec<_>>().as_slice()),
        bmap(&[(3, 303), (4, 36), (5, 35), (6, 31), (7, 29), (8, 1), (9, 1), (10, 1), (11, 23), (12, 65), (13, 195), (15, 119), (17, 10), (18, 2)]),
        "per-kind counts (docs/systems/script-census.md)");
    // Kinds 8 (Drone_CoderCreate 0x13a108) and 0xa (Break_DoBreak 0x12c168) are ported since
    // movie-trigger-41 — each occurs exactly once, both in script 0600005b of 07000001 (the
    // event-8 firing is the game's own no-op: its previous stream record creates no object, and
    // case 8 skips on a NULL object [M 0x1d94c0:75-96 + 0x1d96d4]). Kind 9 is `Player_Cam2Mode`
    // 0x195fc8 since nis-event9-cnis.
    for kind in [8u8, 10] { assert_eq!(*c.event_kinds.get(&kind).unwrap(), 1, "kind {kind:#x} (0600005b's lone firing)"); }
    assert_eq!(c.event_kinds.len(), 14, "14 distinct kinds seen; the port now handles all 14 (17 arms with 0x09/0x0a/0x11)");
    assert_eq!(HANDLED_EVENT_KINDS, 17, "Script_EventHandler arms (8 = CoderCreate, 0xa = DoBreak since movie-trigger-41; 9 = Cam2Mode)");
}

#[test]
fn trigger_action_census_matches_the_doc() {
    let Some(c) = census() else { eprintln!("skip: NF_ISO not set"); return };
    assert_eq!(bmap(&c.trigger_actions.iter().map(|(&k, &v)| (k as u64, v)).collect::<Vec<_>>().as_slice()),
        bmap(&[(1, 348), (2, 178), (3, 46), (4, 21), (5, 5), (10, 16), (12, 28), (13, 1)]),
        "action codes from placements (1/2/3/4/5/10/12/13 handled — 13 = the 07000041 movie trigger, \
         ported since movie-trigger-41; action 2 = 0xEB x175 + raw 0xDC x3)");
    assert_eq!(c.raw_dc_triggers, 33, "0xDC placements with no authored action (inert by design)");
    assert_eq!(c.trigger_actions.values().sum::<u64>() + c.raw_dc_triggers, 676, "trigger placements total");
    assert_eq!(HANDLED_TRIGGER_ACTIONS, 8, "triggers::kind arms (13 = movie since movie-trigger-41)");
}

#[test]
fn placement_type_census_matches_the_doc() {
    let Some(c) = census() else { eprintln!("skip: NF_ISO not set"); return };
    assert_eq!(c.placements, 7372, "dynamic-eligible placement records (flags bit 0xa000 clear)");
    // The 12 unhandled-or-ignored types, most-occurring first (docs/systems/script-census.md).
    // cover-nodes: 0xE5 (584) / 0xE6 (384) moved out; ai-points: 0xE9 (162);
    // sensor-volumes: 0x28 (62); music-triggers: 0xFB (15); movie-trigger-41: 0xF4 (1) —
    // the port now creates and runs the movie-player trigger.
    let want: &[(u8, u64)] = &[(0xe4, 64), (0x2d, 51),
        (0xff, 39), (0x27, 41), (0xdd, 33), (0xf6, 33), (0xf7, 33), (0x2c, 15), (0x39, 8),
        (0xf3, 8), (0x2b, 3), (0x38, 1)];
    for (t, n) in want { assert_eq!(c.placement_types.get(t), Some(n), "type 0x{t:02x}"); }
    let unhandled: u64 = want.iter().map(|(_, n)| n).sum();
    assert_eq!(unhandled, 329, "records on unhandled types (193 game-handled + 136 neither side touches)");
    assert_eq!(*c.placement_types.get(&0xe5).unwrap(), 584, "cover corner placements");
    assert_eq!(*c.placement_types.get(&0xe6).unwrap(), 384, "cover low placements");
    assert_eq!(*c.placement_types.get(&0xfb).unwrap(), 15, "music trigger placements");
    assert_eq!(c.placement_types.values().sum::<u64>(), 7372, "type histogram covers every record");
    assert_eq!(HANDLED_PLACEMENT_TYPES, 62, "54 sim + 8 draw-only (game: 73 spawn cases; 0xF4 since movie-trigger-41)");
}

#[test]
fn anim_script_census_matches_the_doc() {
    let Some(c) = census() else { eprintln!("skip: NF_ISO not set"); return };
    assert_eq!(c.anim_scripts, 18231, "kind-5 entries (anim::Script)");
    assert_eq!(bmap(&c.anim_cmds.iter().map(|(&k, &v)| (k as u64, v)).collect::<Vec<_>>().as_slice()),
        bmap(&[(0, 18296), (1, 3698), (4, 28285)]),
        "AnimProcessScriptCmds opcodes that occur (ops 2/3 never do)");
    assert_eq!(bmap(&c.anim_op4_kinds.iter().map(|(&k, &v)| (k as u64, v)).collect::<Vec<_>>().as_slice()),
        bmap(&[(0, 34), (1, 52), (2, 12947), (3, 53), (4, 6925), (5, 6976), (6, 1294), (7, 4)]),
        "op-4 kind sub-switch (kinds 0/4/5/6 replayed; 1/2/3/7 not)");
}

#[test]
fn archive_entry_kinds_match_the_doc() {
    let Some(c) = census() else { eprintln!("skip: NF_ISO not set"); return };
    assert_eq!(bmap(&c.entry_kinds.iter().map(|(&k, &v)| (k as u64, v)).collect::<Vec<_>>().as_slice()),
        bmap(&[(0, 978), (1, 35), (2, 551), (3, 1217), (4, 18252), (5, 18231), (6, 1485), (7, 468), (8, 34), (11, 666), (12, 26), (15, 1), (16, 33)]),
        "every entry kind LoaderProcess 0x1d4880 can see on the disc");
    // 13 distinct kinds seen; the port's src handles 11 of them (8 is test-only, 15 is absent).
    assert_eq!(c.entry_kinds.len(), 13);
}
