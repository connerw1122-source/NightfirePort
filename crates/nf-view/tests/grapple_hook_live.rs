//! fix-grapple-hook: the grapple point's pulley-with-hook mesh is drawn.
//!
//! The playtest (07000005 at `--bond 16.5,29.6,58.6,-2.01,0.24`, main agent 2026-10-13): the real
//! game shows a PULLEY WITH A HOOK hanging from the T-shaped bracket on the castle wall — the
//! grapple point the Q-Claw latches onto (placement #282 at (13.75, 32.29, 57.3), class 'Q' 0x51
//! per fix-qclaw). The port drew the bracket only: the hook/pulley object was missing.
//!
//! Why the game draws it (the fix's evidence; docs/reports/fix-grapple-hook.md has the walk):
//! - The dynamic-block parser resolves the placement's object and hands it to the creator as the
//!   glist argument [M parsemap_block_map_data_dynamic 0x1d1b68: hashtable_getitem(hashcode), or
//!   pCelList + index*0x50 when the hashcode is −1 — 05's placements are the −1/index branch [D]];
//!   case 0x3a passes it to `Grapple_Create` [M 0x1d04f0:237-239].
//! - `Grapple_Create` 0x198298:24-30 stores it at obj+0xD8 with obj+0xF0 = *(u32*)(glist+0x44), and
//!   the generic dynamic-object draw `View_DrawObjList` 0x1e69a0 → `View_DrawObjectGlist` 0x1e6780
//!   renders obj+0xD8 at obj+0x90 (the placement transform) via `psiDrawObjectMatrix` 0x124708.
//! - Live proof [L ee_start_05.bin]: both class-0x51 objects (0x16d80a0 at #282's spot, 0x16d81c0 at
//!   #281's) hold glist 0x170d6a0 (the `graple_pulley` entity), obj+0xF0 = 0x4000 (no 0x10 hide bit,
//!   no 0x8000 forced bit — no straddle), obj+0xF8 = 0x1f (display bit 0 passes
//!   `View_CaptureScene` 0x1e6df0's `object_display_mask = 1 << viewer_LOD`), and the psiDraw chain
//!   glist+4 → +8 → +4 lands on box tree 0x9b3260 — DRAWABLE. (The creep walls' 0x2B glists chain
//!   to boxes 0 — those stay undrawn, exactly as fix-gadgets2 concluded.)
//! - The pulley never animates: class 0x51 has no control_funcs row [L 0x26fa94..0x26fc68: rows 0x2b..0x50 span 0x26fa94..0x26fc5c; row 0x51 is 0x26fc5c..0x26fc68, all
//!   zero] and no grapple-path writer touches the object's gfx (Player_SSGrapple 0x192860,
//!   Player_SetGrapplePoint 0x195da0 and Bullet_Delete 0x12e660 only read obj+0x30/+0x80 and store
//!   the BLData hooks) — a static mesh at the placement transform is the game's draw, at rest and
//!   latched alike.

use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, level::Level};

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

/// The two 07000005 grapple placements [D nf-extract placements 07000005.bin 3a]: record index,
/// translation, and the resolved object's name. #282 is the playtest point (the Q-Claw's target).
const PLACEMENTS_05: &[(usize, [f32; 3], &str)] = &[
    (281, [-3.3962271, 42.940285, -71.76922], "graple_pulley"),
    (282, [13.752469, 32.291878, 57.303413], "graple_pulley"),
];

/// The pulley's local half-extents [D placements: gfx verts ±(0.169, 0.591, 0.183)] with a small
/// margin, and its total gfx vertex count [D parse: 212]. #282 sits at identity rotation, #281 at
/// yaw π — the world box is the same shape at both.
const PULLEY_HALF: [f32; 3] = [0.18, 0.62, 0.20];
const PULLEY_VERTS: usize = 212;

fn level_05() -> (Level, usize) {
    let path = std::env::var_os("NF_ISO").unwrap();
    let img = IsoImage::open(&path).unwrap();
    let (entries, bin) = disc::files_bin_entries(&img).unwrap();
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000005.bin")).unwrap();
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    let level = Level::parse(&data).unwrap();
    let m = level.main_map().unwrap();
    (level, m)
}

/// World-layer vertices inside the axis-aligned world box (identity/yaw-π placements keep the
/// pulley's local box axis-aligned).
fn verts_in_box(scene: &nf_view::scene::Scene, c: [f32; 3], h: [f32; 3]) -> usize {
    scene.vertices.iter().filter(|v| {
        v.pos[0] >= c[0] - h[0] && v.pos[0] <= c[0] + h[0]
            && v.pos[1] >= c[1] - h[1] && v.pos[1] <= c[1] + h[1]
            && v.pos[2] >= c[2] - h[2] && v.pos[2] <= c[2] + h[2]
    }).count()
}

/// The pulley mesh is in the drawn set at both 07000005 grapple points. Before the fix the scene
/// had ZERO vertices in the pulley's box at #282 (the probe on the pre-fix tree failed at exactly
/// that); after the fix the box holds the pulley's own 212 gfx vertices — and nothing else: the
/// bracket/wall statics stay outside the box, so the exact count is also a no-double-draw guard.
#[test]
fn the_pulley_draws_at_both_07000005_grapple_points() {
    let iso = iso();
    let scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");

    // The disc-side truth: the 0x3A placements resolve to the pulley object, which has gfx.
    let (level, m) = level_05();
    for &(i, t, name) in PLACEMENTS_05 {
        let p = &level.maps[m].placements[i];
        assert_eq!(p.flags & 0xff, 0x3a, "placement {i} is a grapple point");
        assert!(!p.by_hash(), "placement {i} resolves by index (hashcode −1) — the game's pCelList branch [M 0x1d1b68]");
        let obj = &level.maps[m].objects[p.index as usize];
        assert_eq!(obj.name.trim(), name, "placement {i}'s object");
        assert_eq!(obj.flags, 0x4000, "placement {i} object flags — the live obj+0xF0 [L ee_start_05.bin]");
        let nv: usize = obj.gfx.leaves.iter().map(|l| l.batches.iter().map(|b| b.verts.len()).sum::<usize>()).sum();
        assert_eq!(nv, PULLEY_VERTS, "placement {i} pulley gfx verts");
    }

    // The draw: one instance per placement (the counter the sweep keeps, like ladders_drawn).
    assert_eq!(scene.grapple_hooks_drawn, 2, "both 07000005 grapple points draw their pulley (0 = the user's symptom)");

    // And the drawn geometry really is the pulley at each spot: the pulley's full vertex set lands
    // inside its own world box, exactly (nothing else intrudes — checked on the fixed tree).
    for &(_, t, _) in PLACEMENTS_05 {
        let got = verts_in_box(&scene, t, PULLEY_HALF);
        assert_eq!(got, PULLEY_VERTS, "the pulley's {} verts must sit in its box at {:?} (0 = invisible)", PULLEY_VERTS, t);
    }
}

/// One hook per grapple point on every level that has them. The placement counts are disc-derived
/// [D nf-extract placements <level> 3a] over every FILES.BIN archive: the story levels (05, 0a, 0c,
/// 0d, 11, 12, 14, 41), the multiplayer arena 4b, and the driving-mission archives (23–29). Each
/// drawn count must equal its placement count — a miss is a level whose hooks stay invisible (the
/// user's symptom on that map) or a double draw.
#[test]
fn one_hook_per_grapple_point_on_every_level() {
    let iso = iso();
    // (level, 0x3A placements [D])
    const LEVELS: &[(&str, usize)] = &[
        ("07000005.bin", 2), ("0700000a.bin", 1), ("0700000c.bin", 5), ("0700000d.bin", 3),
        ("07000011.bin", 1), ("07000012.bin", 1), ("07000014.bin", 3), ("07000023.bin", 2),
        ("07000024.bin", 7), ("07000025.bin", 4), ("07000026.bin", 4), ("07000027.bin", 8),
        ("07000028.bin", 4), ("07000029.bin", 3), ("07000041.bin", 1), ("0700004b.bin", 8),
    ];
    for (name, placements) in LEVELS {
        let scene = nf_view::load(&iso, name, false).unwrap_or_else(|e| panic!("load {name}: {e}"));
        assert_eq!(scene.grapple_hooks_drawn, *placements,
            "{name}: the hooks drawn must equal the 0x3A placements on the disc");
    }
}

/// Sister (the negative fix-gadgets2 got right): the creep walls stay undrawn — the game's own
/// 0x2B glists chain to boxes 0 [L ee_start_05.bin: obj 0x16d69a0/0x16e8d90, psiDraw chain EMPTY],
/// and the port's parse agrees (their objects carry no batches), so there is nothing to draw and
/// the sweep must not have added them (psiDrawObjectMatrix 0x124708's empty-chain no-op).
#[test]
fn the_creep_walls_stay_undrawn() {
    let iso = iso();
    let scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let (level, m) = level_05();
    let mut creep_walls = 0;
    for p in level.maps[m].placements.iter().filter(|p| p.flags & 0xff == 0x3f) {
        let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
            else if (p.index as usize) < level.maps[m].objects.len() { Some((m, p.index as usize)) } else { None };
        let Some((om, oi)) = target else { continue };
        let obj = &level.maps[om].objects[oi];
        let batches: usize = obj.gfx.leaves.iter().map(|l| l.batches.len()).sum();
        assert_eq!(batches, 0, "creep wall object {} carries no batches [L: the live 0x2B glist chains to boxes 0]", obj.name.trim());
        creep_walls += 1;
    }
    assert!(creep_walls > 0, "07000005 has creep-wall placements to check");
    assert!(scene.misc_drawn.get(&0x3f).is_none(), "no creep-wall instance may be drawn");
    // …while the grapple sweep next to it still runs: the two sweeps share the cels_only block.
    assert_eq!(scene.grapple_hooks_drawn, 2, "the grapple sweep is unaffected by the creep-wall gate");
}
