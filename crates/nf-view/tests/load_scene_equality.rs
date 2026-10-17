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
// Re-pinned at batch 1016c: the per-character light pick (characters key their draws per character —
// the game picks lights per object [M View_DrawObjList 0x1e69a0:73-81]) splits the shared character
// groups, so ONLY the material count and the hash move; vertices, indices, char verts, characters,
// attachments, colliders, cels, drones and pickups are unchanged on all 5 levels.
// Re-pinned at rocket-look: the swoosh ribbon pools (2 meshes x 128 slots) and Bond's projectile slots
// 8 -> 32 per projectile mesh add attachments (+496..+640) and their vertices, plus one material (the
// ribbons' additive batch); characters, colliders, cels, drones and pickups are unchanged on all 5.
// Re-pinned at batch 1017a: laser-scripts spawns the switch-script lasers' ON art at the load beat,
// which 07000014 carries: +2 attachments (+100 vertices, +168 indices, +5 materials); the other 4 unchanged.
// Re-pinned at drone-skins-g021: the captains draw their swapped skins (`NDrone2_DefaultInit`
// 0x14b300:563-565 — 07000002's two kind-1 captains and 07000001's now build the captain skin's
// meshes) and 07000014's 0x050000b1 session merge is gone (carried only by 07000015/16 [D] — the
// game's hashtable misses it, AnimObjectNew 0x1be910:44-46): 02 +259 tris/+2 materials, 14
// −127984 tris/−462 materials/−23 characters, 01 +57 verts; 05 and 0c unchanged byte-for-byte.
// (Merged onto 1017m: 07000014 also carries the laser delta above.)
// Re-pinned at the breakables draw (main-agent render check, 07000007): the 0x20 placements left the
// static World draw for the posed door/SS slot space (a broken one stops drawing — `Break_Kill`
// 0x12b970), so 01/02/14's World materials merge into per-slot Character groups: vertices and
// triangles unchanged, materials and the Character-vertex split move; 05 and 0c carry no 0x20.
const BASELINE: &[(&str, &str)] = &[
    ("07000002.bin", "d7e8f00ded57dd1f 4477259 9358977 5347 4339965 81 9682 205 28 25 8"),
    ("07000005.bin", "6a43ad2ce9dee894 4377695 9151611 3576 4293354 73 9731 106 19 16 6"),
    ("0700000c.bin", "15de2f0294792a8a 4871632 10240830 8177 4680808 96 10996 170 22 40 15"),
    ("07000014.bin", "b4011376b5d7991f 7208306 14968956 6767 7055181 111 13430 173 41 37 32"),
    ("07000001.bin", "d50702caf2606dc5 4389613 9198819 3484 4316311 76 9696 101 11 20 6"),
];

#[test]
fn the_cut_costs_build_the_same_scene() {
    let Some(iso) = iso() else { return };
    if cfg!(debug_assertions) { eprintln!("scene equality: release-only, skipped on a debug build"); return; }
    let mut drifted: Vec<String> = Vec::new();
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
        eprintln!("{level}: {got}");
        if &got != expected { drifted.push(format!("{level}: got {got}, pinned {expected}")); }
        assert!(scene.vertices.len() > 0, "{level}: the pin must not be vacuous");
    }
    assert!(drifted.is_empty(), "the built scenes changed (level-load-time cuts must be scene-preserving):\n{}", drifted.join("\n"));
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
