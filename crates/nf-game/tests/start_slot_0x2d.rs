//! start-slot-0x2d: the second start-position set's registration kind and `Player_Start`'s pick,
//! headless. The registration maps the main map's placements in parse order — type 0x24 → kind 1,
//! type 0x2d → kind 0 [M `parsemap_create_dynamic_objects` 0x1d04f0 case 0x24 → disasm 0x1d0a00
//! `addiu a2,$zero,1`; case 0x2d → disasm 0x1d0e98 `move a2,$zero`] — and `Player_Start` 0x190fd0
//! picks only kind-1 entries (disasm 0x191028/0x1910b0: both passes `bne v0, 1` on entry+0x22).
//! The disc-side pins (per-level counts, the 0700000b/07000007 layouts, the real-path loads) are
//! start_slot_0x2d_live; this file pins the rule with synthetic data so it runs without NF_ISO.

use nf_formats::placement::Placement;
use nf_game::player::{pick_start, starts_from_placements, LevelStart};
use nf_game::triggers::Channels;

fn start(kind: u16, pos: [f32; 3], gate: u16) -> LevelStart {
    LevelStart { pos, yaw: 0.0, gate, p3: 0, kind }
}

/// A synthetic placement with the given type byte (+ optional gate param 0 / p3 param 3).
fn placement(ty: u32, params: &[(u32, u32)]) -> Placement {
    Placement { flags: ty, translation: [1.0, 2.0, 3.0], rotation: [0.0, 1.5, 0.0], params: params.to_vec(), ..Default::default() }
}

/// ── The registration: 0x24 → kind 1, 0x2d → kind 0, parse order, other types and the 0xa000-
/// ineligible records dropped [M 0x1d1b68:93 — only `(flags & 0xa000) == 0` reaches the dispatch]. ──
#[test]
fn registration_maps_0x24_to_kind_1_and_0x2d_to_kind_0() {
    let placements = vec![
        placement(0x24, &[(0, 0x1d), (3, 2)]),
        placement(0x2d, &[]),
        placement(0x25, &[]),            // the MP team spawn: never a player start
        placement(0x2d | 0xa000, &[]),   // flags bit set → the dynamic parser skips it entirely
        placement(0x24, &[]),
    ];
    let starts = starts_from_placements(&placements);
    assert_eq!(starts.len(), 3, "two 0x24 + one eligible 0x2d register [M 0x1d04f0 case 0x24/0x2d]");
    assert_eq!(starts[0].kind, 1, "type 0x24 → kind 1 [M disasm 0x1d0a00]");
    assert_eq!(starts[1].kind, 0, "type 0x2d → kind 0 [M disasm 0x1d0e98]");
    assert_eq!(starts[2].kind, 1, "parse order preserved (the second 0x24)");
    assert_eq!(starts[0].gate, 0x1d, "gate = placement param 0 [M level_starts' pinned parse]");
    assert_eq!(starts[0].p3, 2, "p3 = placement param 3 (level_tag+0x38) [M Player_Init 0x1910f8]");
    assert_eq!(starts[0].pos, [1.0, 2.0, 3.0], "pos = the placement's translation");
    assert_eq!(starts[0].yaw, 1.5, "yaw = the placement's rotation[1] (the pinned convention)");
}

/// ── THE sister: a kind-0 entry EARLIER in parse order must not win the pick — the game's
/// `Player_Start` skips it in both passes [M 0x190fd0 disasm 0x191028/0x1910b0]. 0700000b ships
/// exactly this layout on its disc data (start_slot_0x2d_live pins the real bytes). ──────────────
#[test]
fn pick_kind0_first_still_loses_to_the_kind1_entry() {
    let starts = vec![
        start(0, [9.92, 48.53, 21.48], 0),    // the 0x2d set, registered first
        start(1, [28.92, 13.28, 28.67], 0),   // the level's only kind-1 entry
        start(0, [-0.42, 58.53, 27.08], 0),
    ];
    let picked = pick_start(&starts, &Channels::default()).expect("the kind-1 entry is picked");
    assert_eq!(picked, 1, "the kind-0 entry ahead of it must not win [M 0x190fd0 both passes ==1]");
    assert_eq!(starts[picked].pos, [28.92, 13.28, 28.67]);
}

/// ── Pass 1: the first kind-1 entry whose gate is 0 or whose channel is on [M 0x191030-0x191040]. ──
#[test]
fn pick_takes_the_first_open_gate_kind1_entry() {
    let starts = vec![
        start(1, [1.0, 0.0, 0.0], 0x1d), // gate off
        start(1, [2.0, 0.0, 0.0], 0),    // gate 0 → wins
        start(1, [3.0, 0.0, 0.0], 0x1e),
    ];
    assert_eq!(pick_start(&starts, &Channels::default()), Some(1), "the first gate-0 kind-1 entry");
    let mut ch = Channels::default();
    ch.set(0x1d, 1, 0);
    assert_eq!(pick_start(&starts, &ch), Some(0), "with ch 0x1d on, the first entry's gate is open");
}

/// ── Pass 2 (fallback): every kind-1 gate closed → the first kind-1 entry; kind-0 entries are
/// skipped there too [M 0x190fd0 disasm 0x1910b0 — the second loop has the same ==1 check]. ───────
#[test]
fn pick_fallback_takes_the_first_kind1_entry_skipping_kind0() {
    let starts = vec![
        start(0, [9.0, 0.0, 0.0], 0),
        start(1, [1.0, 0.0, 0.0], 0x1d), // gate off
        start(1, [2.0, 0.0, 0.0], 0x1e), // gate off
        start(0, [8.0, 0.0, 0.0], 0),
    ];
    assert_eq!(pick_start(&starts, &Channels::default()), Some(1),
        "fallback = the first kind-1 entry regardless of its gate [M 0x190fd0 pass 2]");
}

/// ── No kind-1 entry → no pick: the game's `Player_Start` returns without `Player_Init`
/// (both passes require kind 1; the MP maps' human spawns come from `MP_Start` 0x183a50's
/// `MP_GetSpawnPoint`, not `player_start` [M 0x183a50:118-128]). ─────────────────────────────────
#[test]
fn pick_with_only_kind0_entries_is_none() {
    let starts = vec![start(0, [1.0, 0.0, 0.0], 0), start(0, [2.0, 0.0, 0.0], 0)];
    assert_eq!(pick_start(&starts, &Channels::default()), None, "kind-0 entries are unpickable");
    assert_eq!(pick_start(&[], &Channels::default()), None, "no entries → no pick");
}
