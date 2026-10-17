//! The scene-equality proof for the level-load-time cuts (level-load-time step 3).
//!
//! Every cost cut must build the SAME scene. The digests below were captured from the
//! pre-optimization tree (probe session 2026-10-16, `tests/load_scene_digest.rs` — FNV-1a over
//! vertices/indices/materials/CharVertices/characters/attachments/colliders/cels + the sim's
//! drone/pickup/door/bond facts and the stats line, plus the counts) and pinned here. The test
//! rebuilds each level through the CURRENT load path and asserts the digest is unchanged, so a
//! future edit that shifts one vertex, index or material field fails here.
//!
//! Levels: the two heaviest archives (07000014, 0700000c), the archive-heavy mid-game pair
//! (07000005, 07000002) and the smallest (07000001) — 5 of the 27 on-foot levels, spanning the
//! pool-heavy (projectile/tracer/gas/decal/casing/debris) and merge-heavy (session mesh merge)
//! shapes. Opt-in via `NF_ISO`; release-only.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// `(hash, vertices, indices, materials, char_verts, characters, attachments, colliders, cels, drones, pickups)`
/// from `load_scene_digest.rs` on the pre-optimization tree. The full FNV hash pins the contents;
/// the counts pin the shape.
// Re-pinned at merge batch 1016b: master gained scene content since the branch cut (e.g. the
// quiver bolt pool); the digests were re-captured on origin/master bee8c129 (no cuts) and the
// merged tree (cuts) builds them identically on all 5 levels.
const BASELINE: &[(&str, &str)] = &[
    ("07000002.bin", "d2d94af0cf17010d 4303432 8995716 4540 4160642 81 9186 255 28 25 8"),
    ("07000005.bin", "7099e4ac4aae4104 4204127 8788635 2680 4119786 73 9235 106 19 16 6"),
    ("0700000c.bin", "a4a2155d96a2f706 4687168 9851358 7026 4496344 96 10452 170 22 40 15"),
    ("07000014.bin", "826b23eb71554128 7060774 14654658 5293 6907637 134 12845 176 41 37 32"),
    ("07000001.bin", "d053e1db8a46c7a3 4215988 8835732 2786 4133583 76 9200 156 11 20 6"),
];

#[test]
fn the_cut_costs_build_the_same_scene() {
    let Some(iso) = iso() else { return };
    if cfg!(debug_assertions) { eprintln!("scene equality: release-only, skipped on a debug build"); return; }
    for (level, expected) in BASELINE {
        let scene = nf_view::load(&iso, level, false).unwrap_or_else(|e| panic!("{level}: {e}"));
        let sim = scene.sim.as_ref().expect("a level scene carries a sim");
        let got = format!("{} {} {} {} {} {} {} {} {} {} {}",
            // The digest hash itself (tests/load_scene_digest.rs computes it; recompute here over
            // the same fields so this test is self-contained).
            digest_hash(&scene, sim),
            scene.vertices.len(), scene.indices.len(), scene.materials.len(), scene.char_verts.len(),
            scene.characters.len(), scene.attachments.len(), scene.colliders.len(), scene.cels.len(),
            sim.drones.list.len(), sim.drones.pickups.len());
        // [S port engineering] scene-equality of the load-time cuts (digests re-captured on master bee8c129), no game claim
        assert_eq!(&got, expected, "{level}: the built scene changed (level-load-time cuts must be scene-preserving)");
        assert!(scene.vertices.len() > 0, "{level}: the pin must not be vacuous");
    }
}

/// The same FNV-1a field walk `tests/load_scene_digest.rs` runs (kept in sync; see that file).
fn digest_hash(s: &nf_view::scene::Scene, sim: &nf_view::scene::Sim) -> String {
    use std::hash::Hasher;
    let mut h: u64 = 0xcbf29ce484222325;
    let mut f = |h: &mut u64, b: &[u8]| { for &x in b { *h ^= x as u64; *h = h.wrapping_mul(0x100000001b3); } };
    for v in &s.vertices {
        for p in v.pos { f(&mut h, &p.to_bits().to_le_bytes()); }
        for p in v.uv { f(&mut h, &p.to_bits().to_le_bytes()); }
        f(&mut h, &v.rgba); f(&mut h, &v.nrm.map(|x| x as u8));
    }
    for i in &s.indices { f(&mut h, &i.to_le_bytes()); }
    for m in &s.materials {
        f(&mut h, &[m.layer as u8]);
        for p in m.centre { f(&mut h, &p.to_bits().to_le_bytes()); }
        f(&mut h, &m.radius.to_bits().to_le_bytes());
        f(&mut h, &m.cel.map(|c| c.to_le_bytes()).unwrap_or([0xff, 0xff]));
        f(&mut h, &m.chain.to_le_bytes()); f(&mut h, &m.node.to_le_bytes());
        if let Some((map, tex)) = m.key { f(&mut h, &(map as u64 | ((tex as u64) << 32)).to_le_bytes()); } else { f(&mut h, &[0xff]); }
        f(&mut h, &m.lflags.to_le_bytes());
        f(&mut h, &m.first_index.to_le_bytes()); f(&mut h, &m.index_count.to_le_bytes());
    }
    for cv in &s.char_verts {
        f(&mut h, &cv.character.to_le_bytes());
        for p in cv.pos { f(&mut h, &p.to_bits().to_le_bytes()); }
        f(&mut h, &cv.weight.to_bits().to_le_bytes());
        f(&mut h, &cv.slots[0].to_le_bytes()); f(&mut h, &cv.slots[1].to_le_bytes());
        f(&mut h, &cv.attachment.map(|a| a.to_le_bytes()).unwrap_or([0xff; 4]));
        f(&mut h, &cv.searchlight.map(|a| a.to_le_bytes()).unwrap_or([0xff; 2]));
        f(&mut h, &cv.door.map(|a| a.to_le_bytes()).unwrap_or([0xff; 2]));
        f(&mut h, &cv.pickup.map(|a| a.to_le_bytes()).unwrap_or([0xff; 2]));
        f(&mut h, &cv.morph.to_le_bytes());
    }
    for c in &s.characters {
        f(&mut h, &c.skin.hashcode.to_le_bytes());
        for row in &c.matrix { for p in *row { f(&mut h, &p.to_bits().to_le_bytes()); } }
        if let Some(fl) = c.floor { f(&mut h, &fl.to_bits().to_le_bytes()); } else { f(&mut h, &[0xff]); }
    }
    for a in &s.attachments {
        f(&mut h, &a.character.to_le_bytes()); f(&mut h, &a.bone.to_le_bytes());
        for row in &a.local { for p in *row { f(&mut h, &p.to_bits().to_le_bytes()); } }
    }
    for (name, lo, hi) in &s.colliders {
        f(&mut h, name.trim().as_bytes());
        for p in *lo { f(&mut h, &p.to_bits().to_le_bytes()); }
        for p in *hi { f(&mut h, &p.to_bits().to_le_bytes()); }
    }
    f(&mut h, &sim.level.to_le_bytes());
    for p in sim.bond.pos { f(&mut h, &p.to_bits().to_le_bytes()); }
    f(&mut h, &sim.drones.list.len().to_le_bytes());
    f(&mut h, &sim.drones.pickups.len().to_le_bytes());
    f(&mut h, &sim.drones.doors.list.len().to_le_bytes());
    f(&mut h, s.stats.as_bytes());
    format!("{h:016x}")
}
