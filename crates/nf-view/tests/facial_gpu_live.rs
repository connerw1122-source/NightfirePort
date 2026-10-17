//! fix-gpu-facial-morphs: the GPU skinning path must carry `SkinIt` 0x10bf30's morph term — the
//! per-character facial weights (`AnimFacialBlend` 0x1be648's array at animobj+0xb0) and the
//! MORPH_DATA targets (the gfx record's +0x14 base, `parsemap_block_morph_data` 0x1d23e8), added
//! to the source position BEFORE the palette blend, exactly as the CPU path (`Scene::animate` →
//! `morph_vertex`) does. Pre-fix the GPU path drew still faces: the palette/attrs carried no
//! morph data, and drone-backed characters were gated out of `animate` altogether (fix-facial3's
//! stand-in until "the GPU half lands").
//!
//! Disc-measured premises [D]: 07000006's driver (skin 0x05000035) is drone-backed with his face
//! window live from tick 0 (fix-facial3); 07000007 carries morph-bearing characters (the Kimiko
//! scene renders, fix-cutscene-faces). Opt-in via NF_ISO (the slim ISO is enough).

use nf_formats::placement::transform;
use nf_game::player::Input;
use nf_view::scene::{Scene, SkinAttr, ViewOpts};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn step(scene: &mut Scene) {
    scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
}

/// The shader's skinning step on the CPU: `w·(P[a]·p) + (1−w)·(P[b]·p)` (docs G-043 — the blend
/// weights sum to 1, so this equals the CPU path's `transform(m, skin_vertex(..))`).
fn apply(p: &[[[f32; 4]; 4]], a: &SkinAttr, pos: [f32; 3]) -> [f32; 3] {
    let pa = transform(&p[a.a as usize], pos);
    let pb = transform(&p[a.b as usize], pos);
    [0, 1, 2].map(|k| a.w * pa[k] + (1.0 - a.w) * pb[k])
}

/// `SkinIt` 0x10bf30's morph term [M 88-116]: `pos += Σ_m weight[m] × target(base + (morph + m)·12)`,
/// the 12-byte reads as three f32, before the skinning. Out-of-range channels contribute nothing
/// (the game's count-bounded loop; `morph_vertex` scene.rs).
fn morph_emul(mut pos: [f32; 3], morph: i16, weights: &[f32], targets: &[u8]) -> [f32; 3] {
    let f32_at = |i: usize| -> Option<[f32; 3]> {
        let o = i * 12;
        if o + 12 > targets.len() { return None; }
        Some([
            f32::from_le_bytes(targets[o..o + 4].try_into().unwrap()),
            f32::from_le_bytes(targets[o + 4..o + 8].try_into().unwrap()),
            f32::from_le_bytes(targets[o + 8..o + 12].try_into().unwrap()),
        ])
    };
    for (m, w) in weights.iter().enumerate() {
        if let Some(d) = f32_at(morph as usize + m) {
            for k in 0..3 { pos[k] += w * d[k]; }
        }
    }
    pos
}

/// The morph delta is REAL on 07000007: at a live face tick, the CPU pose (`animate`, which runs
/// the morph term for script characters) leaves the GPU-path arithmetic (`apply` on the raw
/// bind-pose position — everything the pre-fix GPU path could express) behind by more than the
/// oracle tolerance. This is the symptom the renders showed byte-identical faces: the pre-fix GPU
/// path had nowhere to put the weights or the targets.
#[test]
fn the_kimiko_face_delta_is_beyond_the_gpu_oracle_tolerance() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load 07000007");
    // The Kimiko scene fires off TouchOnce #412's volume (fix_cutscenes2_live's spot): Bond stands
    // in it, the library scene runs, and its guest (the anim script 060004f1's script object)
    // plays the face track. Ride until the guest's live weights are clearly mid-blend.
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [94.2, 43.7, -45.9];
    }
    let mut found = None;
    for t in 0..1200u32 {
        step(&mut scene);
        for (ci, c) in scene.characters.iter().enumerate() {
            let Some(src) = c.script else { continue };
            if c.skin.facial == 0 || c.morph_targets.is_none() { continue; }
            if let Some(w) = scene.facial_weights_at(ci, c) {
                let m = w.iter().fold(0.0f32, |m, v| m.max(v.abs()));
                if found.as_ref().map_or(true, |(_, _, _, bm)| m > *bm) { found = Some((t, ci, w.clone(), m)); }
            }
        }
        if found.as_ref().is_some_and(|(_, _, _, m)| *m > 0.5) { break; }
    }
    let (t, ci, w, max_w) = found.expect("the Kimiko scene's guest went live within 1200 ticks");
    let t = t.max(1);
    eprintln!("[facial-gpu] character {ci} live weights at tick {t} (max |w| {max_w:.3}, {} channels)", w.len());
    // The two pose paths at this tick.
    let opts = ViewOpts::default();
    let cpu = scene.animate(None, t as f32, opts);
    let (layout, attrs) = scene.skin_layout();
    let (pal, _) = scene.skin_palette(&layout, None, t as f32, opts, None);
    let c = &scene.characters[ci];
    let targets = c.morph_targets.as_deref().unwrap();
    let face_verts: Vec<usize> = scene.char_verts.iter().enumerate()
        .filter(|(_, v)| v.character as usize == ci && v.morph >= 0).map(|(i, _)| i).collect();
    assert!(!face_verts.is_empty(), "the morph-bearing character has morph-indexed vertices");
    let w_now = scene.facial_weights_now(c).expect("the weights stay live at the captured tick");
    let (mut worst_frozen, mut worst_morph) = (0f32, 0f32);
    for &i in &face_verts {
        let cv = &scene.char_verts[i];
        let gpu_frozen = apply(&pal, &attrs[i], cv.pos);           // the pre-fix GPU path: no morph data
        let gpu_morphed = apply(&pal, &attrs[i], morph_emul(cv.pos, cv.morph, &w_now, targets));
        let e = |g: [f32; 3]| (0..3).map(|k| (g[k] - cpu[i][k]).abs()).fold(0f32, f32::max);
        worst_frozen = worst_frozen.max(e(gpu_frozen));
        worst_morph = worst_morph.max(e(gpu_morphed));
    }
    eprintln!("[facial-gpu] tick {t}: frozen-GPU vs CPU {worst_frozen:.6}, morphed-GPU vs CPU {worst_morph:.6} over {} face verts", face_verts.len());
    // The symptom: the pre-fix GPU path cannot express the face (well beyond the 1e-3 oracle).
    assert!(worst_frozen > 1e-3, "the frozen-face delta must exceed the oracle tolerance (got {worst_frozen})");
    // The morph term alone explains it: with the term applied, the CPU pose is reproduced.
    assert!(worst_morph < 1e-3, "the morph term reproduces the CPU pose (got {worst_morph})");
}

/// The play path (drone-backed characters, no script object): pre-fix `animate` gated them OUT
/// (fix-facial3's stand-in — the GPU half was missing), so the driver's face never moved in play.
/// The pose must carry the morph term: `animate` and the palette arithmetic on the raw bind-pose
/// position (what a morph-less GPU path draws) DISAGREE on his morph vertices at a live tick.
/// Pre-fix this fails the other way: the gate freezes him, the two agree.
#[test]
fn the_drivers_face_moves_in_play_not_only_in_scripts() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000006.bin", false).expect("load 07000006");
    let ci = scene.characters.iter().position(|c| c.skin.hashcode == 0x0500_0035)
        .expect("the driver's character exists");
    assert!(scene.characters[ci].script.is_none(), "the driver is drone-backed, not a script object");
    let face_verts: Vec<usize> = scene.char_verts.iter().enumerate()
        .filter(|(_, v)| v.character as usize == ci && v.morph >= 0).map(|(i, _)| i).collect();
    assert!(!face_verts.is_empty(), "the driver's mesh has morph-indexed vertices");
    for _ in 0..3000 { step(&mut scene); if !scene.sim.as_ref().unwrap().disabled { break; } }
    // The driver's face window is live from his first tick (fix-facial3); stop at the first
    // mid-blend crossing so the sim state still matches the recorded weights at evaluation time.
    let mut found = None;
    for t in 0..1200u32 {
        step(&mut scene);
        if let Some(w) = scene.facial_weights_at(ci, &scene.characters[ci]) {
            if w.iter().fold(0.0f32, |m, v| m.max(v.abs())) > 0.05 { found = Some((t, w.clone(), w.iter().fold(0.0f32, |m, v| m.max(v.abs())))); break; }
        }
    }
    let (t, w, mw) = found.expect("the driver's face weights went live (the fix-facial3 premise)");
    eprintln!("[facial-gpu] driver weights at tick {t} (max |w| {mw:.3}, {} channels)", w.len());
    let opts = ViewOpts::default();
    let cpu = scene.animate(None, t as f32, opts);
    let (layout, attrs) = scene.skin_layout();
    let (pal, _) = scene.skin_palette(&layout, None, t as f32, opts, None);
    let targets = scene.characters[ci].morph_targets.as_deref().expect("the driver carries targets");
    let (mut frozen, mut morphed) = (0f32, 0f32);
    for &i in &face_verts {
        let cv = &scene.char_verts[i];
        let e = |g: [f32; 3]| (0..3).map(|k| (g[k] - cpu[i][k]).abs()).fold(0f32, f32::max);
        frozen = frozen.max(e(apply(&pal, &attrs[i], cv.pos)));
        morphed = morphed.max(e(apply(&pal, &attrs[i], morph_emul(cv.pos, cv.morph, &w, targets))));
    }
    eprintln!("[facial-gpu] driver tick {t}: frozen {frozen:.6}, morphed {morphed:.6} over {} face verts", face_verts.len());
    assert!(frozen > 1e-3, "the driver's CPU pose carries the morph term in play (frozen delta {frozen})");
    assert!(morphed < 1e-3, "the morph term explains the play-path pose (got {morphed})");
}

/// The GPU-side contract, through the fix's own plumbing: the weight blocks (`skin_morph_weights`,
/// exactly what `update_skin`/`upload_morph_weights` write) plus the concatenated targets
/// (`morph_target_blob`, the mtg buffer) plus the attrs' morph fields reproduce the CPU pose —
/// including the drone-backed characters (the un-gate; the driver is one) — on the Kimiko scene's
/// live script guest AND the 07000006 driver, mid-blend. Sister: zeroing the weight upload (the
/// frozen-face bug) is caught by the same oracle.
#[test]
fn the_gpu_morph_buffers_reproduce_the_cpu_pose_mid_blend() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for (lvl, place_bond) in [("07000007.bin", true), ("07000006.bin", false)] {
        let mut scene = nf_view::load(&iso, lvl, false).expect("load");
        if place_bond {
            // The Kimiko scene's trigger (fix_cutscenes2_live's TouchOnce #412 volume).
            scene.sim.as_mut().unwrap().bond.pos = [94.2, 43.7, -45.9];
        }
        // The character with the strongest live blend over the ride.
        let mut found = None;
        for t in 0..1200u32 {
            step(&mut scene);
            for (ci, c) in scene.characters.iter().enumerate() {
                if c.skin.facial == 0 || c.morph_targets.is_none() { continue; }
                if let Some(w) = scene.facial_weights_at(ci, c) {
                    let m = w.iter().fold(0.0f32, |m, v| m.max(v.abs()));
                    if found.as_ref().map_or(true, |(_, _, _, bm)| m > *bm) { found = Some((t.max(1), ci, w.clone(), m)); }
                }
            }
            if found.as_ref().is_some_and(|(_, _, _, m)| *m > 0.5) { break; }
        }
        let (t, ci, _, mw) = found.expect("a morph-bearing character went live mid-blend");
        let opts = ViewOpts::default();
        let cpu = scene.animate(None, t as f32, opts);
        let (layout, attrs) = scene.skin_layout();
        let (pal, _) = scene.skin_palette(&layout, None, t as f32, opts, None);
        let wts = scene.skin_morph_weights(&layout);
        let blob = scene.morph_target_blob();
        let tgt_f: Vec<f32> = blob.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect();
        assert_eq!(layout.n_tgt as usize, tgt_f.len(), "the layout's target count matches the blob");
        let face_verts: Vec<usize> = scene.char_verts.iter().enumerate()
            .filter(|(i, v)| v.character as usize == ci && v.morph >= 0 && attrs[*i].cnt > 0).map(|(i, _)| i).collect();
        assert!(!face_verts.is_empty(), "the live character's mesh carries morph attrs (character {ci}, max |w| {mw:.3})");
        let err = |zero_wts: bool| {
            let mut worst = 0f32;
            for &i in &face_verts {
                let w2 = if zero_wts { &vec![0.0f32; wts.len()][..] } else { &wts[..] };
                let g = apply_new(&pal, &attrs[i], scene.char_verts[i].pos, w2, &tgt_f);
                worst = worst.max((0..3).map(|k| (g[k] - cpu[i][k]).abs()).fold(0f32, f32::max));
            }
            worst
        };
        let (full, frozen) = (err(false), err(true));
        eprintln!("[facial-gpu] {lvl} char {ci} tick {t}: GPU+weights vs CPU {full:.8}, zeroed weights {frozen:.6}");
        assert!(full < 1e-3, "the GPU morph buffers reproduce the CPU pose (error {full})");
        // Sister: a broken/zeroed weight upload re-freezes the face — the oracle must catch it.
        assert!(frozen > 1e-3, "zeroed weights must be caught (the frozen-face regression)");
    }
}

/// The GPU path's arithmetic (the shader's `morph_pos` + blend) on the CPU.
fn apply_new(p: &[[[f32; 4]; 4]], a: &SkinAttr, pos0: [f32; 3], wts: &[f32], tgt: &[f32]) -> [f32; 3] {
    let mut pos = pos0;
    if a.mtb >= 0 {
        let (mwb, cnt, tend) = (a.mwb as usize, a.cnt as usize, a.tend as usize);
        for m in 0..cnt {
            let w = wts[(mwb + m / 4) * 4 + m % 4];
            if w != 0.0 {
                let e = a.mtb as usize + m * 3;
                if e + 2 < tend { for k in 0..3 { pos[k] += w * tgt[e + k]; } }
            }
        }
    }
    let pa = transform(&p[a.a as usize], pos);
    let pb = transform(&p[a.b as usize], pos);
    [0, 1, 2].map(|k| a.w * pa[k] + (1.0 - a.w) * pb[k])
}
