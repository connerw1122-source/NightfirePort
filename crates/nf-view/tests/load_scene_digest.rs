//! The level-load-time scene digest: a fingerprint of every built scene fact the optimization
//! must preserve. `nf_view::load` is load-side only, so before/after runs of this test over the
//! same levels must print IDENTICAL lines — the proof the cut costs did not change the built
//! scene (level-load-time step 3; the async-level-load live test's comparison generalised).
//!
//! The digest covers vertices (pos/uv/rgba/nrm bits), indices, materials (every field), character
//! vertices, characters, attachments, colliders, cels and the sim's drone/pickup/bond facts plus
//! the stats line. Run it before the optimization (baseline) and after; compare.
//!
//! Opt-in via `NF_ISO`; release-only.

use std::hash::Hasher;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn fnv64(h: &mut u64, bytes: &[u8]) {
    for &b in bytes {
        *h ^= b as u64;
        *h = h.wrapping_mul(0x100000001b3);
    }
}
fn fnv64_f32s(h: &mut u64, v: &[f32]) { for f in v { fnv64(h, &f.to_bits().to_le_bytes()); } }

fn digest(s: &nf_view::scene::Scene) -> (u64, String) {
    let mut h: u64 = 0xcbf29ce484222325;
    for v in &s.vertices { fnv64_f32s(&mut h, &v.pos); fnv64_f32s(&mut h, &v.uv); fnv64(&mut h, &v.rgba); fnv64(&mut h, &v.nrm.map(|x| x as u8)); }
    for i in &s.indices { fnv64(&mut h, &i.to_le_bytes()); }
    for m in &s.materials {
        fnv64(&mut h, &[m.layer as u8]);
        fnv64_f32s(&mut h, &m.centre); fnv64_f32s(&mut h, &[m.radius]);
        fnv64(&mut h, &m.cel.map(|c| c.to_le_bytes()).unwrap_or([0xff, 0xff]));
        fnv64(&mut h, &m.chain.to_le_bytes()); fnv64(&mut h, &m.node.to_le_bytes());
        if let Some((map, tex)) = m.key { fnv64(&mut h, &(map as u64 | ((tex as u64) << 32)).to_le_bytes()); } else { fnv64(&mut h, &[0xff]); }
        fnv64(&mut h, &m.lflags.to_le_bytes());
        fnv64(&mut h, &m.first_index.to_le_bytes()); fnv64(&mut h, &m.index_count.to_le_bytes());
    }
    for cv in &s.char_verts {
        fnv64(&mut h, &cv.character.to_le_bytes());
        fnv64_f32s(&mut h, &cv.pos); fnv64_f32s(&mut h, &[cv.weight]);
        fnv64(&mut h, &cv.slots[0].to_le_bytes()); fnv64(&mut h, &cv.slots[1].to_le_bytes());
        fnv64(&mut h, &cv.attachment.map(|a| a.to_le_bytes()).unwrap_or([0xff; 4]));
        fnv64(&mut h, &cv.searchlight.map(|a| a.to_le_bytes()).unwrap_or([0xff; 2]));
        fnv64(&mut h, &cv.door.map(|a| a.to_le_bytes()).unwrap_or([0xff; 2]));
        fnv64(&mut h, &cv.pickup.map(|a| a.to_le_bytes()).unwrap_or([0xff; 2]));
        fnv64(&mut h, &cv.morph.to_le_bytes());
    }
    for c in &s.characters {
        fnv64(&mut h, &c.skin.hashcode.to_le_bytes());
        for row in &c.matrix { fnv64_f32s(&mut h, row); }
        if let Some(f) = c.floor { fnv64_f32s(&mut h, &[f]); } else { fnv64(&mut h, &[0xff]); }
    }
    for a in &s.attachments {
        fnv64(&mut h, &a.character.to_le_bytes()); fnv64(&mut h, &a.bone.to_le_bytes());
        for row in &a.local { fnv64_f32s(&mut h, row); }
    }
    for (name, lo, hi) in &s.colliders {
        fnv64(&mut h, name.trim().as_bytes());
        fnv64_f32s(&mut h, lo); fnv64_f32s(&mut h, hi);
    }
    let sim = s.sim.as_ref().expect("a level scene carries a sim");
    fnv64(&mut h, &sim.level.to_le_bytes());
    fnv64_f32s(&mut h, &sim.bond.pos);
    fnv64(&mut h, &sim.drones.list.len().to_le_bytes());
    fnv64(&mut h, &sim.drones.pickups.len().to_le_bytes());
    fnv64(&mut h, &sim.drones.doors.list.len().to_le_bytes());
    fnv64(&mut h, s.stats.as_bytes());
    let summary = format!("h={h:016x} v={} i={} m={} cv={} ch={} at={} col={} cel={} dr={} pk={}",
        s.vertices.len(), s.indices.len(), s.materials.len(), s.char_verts.len(), s.characters.len(),
        s.attachments.len(), s.colliders.len(), s.cels.len(), sim.drones.list.len(), sim.drones.pickups.len());
    (h, summary)
}

#[test]
fn scene_digest_per_level() {
    let Some(iso) = iso() else { return };
    if cfg!(debug_assertions) { eprintln!("scene_digest: release-only, skipped on a debug build"); return; }
    // The equality-proof set: the archive-heavy story levels + the biggest ones (level-load-time).
    for level in ["07000002.bin", "07000005.bin", "0700000c.bin", "07000014.bin", "07000001.bin"] {
        let scene = nf_view::load(&iso, level, false).unwrap_or_else(|e| panic!("{level}: {e}"));
        let (h, summary) = digest(&scene);
        eprintln!("DIGEST {level} {summary}");
        assert!(h != 0, "{level}: the digest must not be vacuous");
    }
}
