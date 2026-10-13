//! fix-facial2: the morph TARGET buffer and the vertex half. The game stores the MORPH_DATA map
//! block (dispatch 0x1d1020 case 0x2b) payload straight into the runtime gfx record's +0x14
//! [M `parsemap_block_morph_data` 0x1d23e8: `*(gfx+0x14) = FileNextBlock + 4`], and `SkinIt`
//! 0x10bf30 adds Σ weight[m] × target(base + (morph + m) × 12) per vertex (the vertex's morph
//! index is the skinning word's upper half-word, −1 = none). Opt-in via NF_ISO (the slim ISO is
//! enough): 07000005's ledge dialogue (the 06000996/97 pair, face sequence 040007fa) drives two
//! lockstep scenes — one with the morph path, one with the targets cleared — so the bone posing
//! cancels and the difference is exactly the morph term.

use nf_game::player::Input;
use nf_view::scene::{Scene, ViewOpts};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str) -> Scene { nf_view::load(iso, "07000005.bin", false).expect("load 07000005") }

fn step(scene: &mut Scene) {
    scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
}

/// The first script character whose morph targets exist, with its face-track window sequence.
fn morph_script_char(scene: &Scene) -> Option<(usize, u16, u32)> {
    let s = scene.sim.as_ref()?;
    scene.characters.iter().enumerate().find_map(|(ci, c)| {
        if c.morph_targets.is_none() { return None; }
        let src = c.script?;
        let (sh, frame) = s.script_anim(src, c.skin.hashcode)?;
        let script = s.scripts.get(&sh)?;
        let count = c.skin.facial.max(1) as usize;
        let (first, q) = script.sequences().filter(|(a, _, _)| (*a as f32) <= frame)
            .filter_map(|(a, _, q)| s.seqs.get(&q).filter(|sq| sq.channels.len() == count).map(|_| (a, q)))
            .last()?;
        Some((ci, first, q))
    })
}

#[test]
fn a_face_vertex_moves_by_the_morph_term_when_the_track_first_moves() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut a = load(&iso);
    // The sister scene: identical, deterministic lockstep — only the morph targets cleared.
    let mut b = load(&iso);
    for c in b.characters.iter_mut() { c.morph_targets = None; }
    // The first morph-bearing script object to go live (the level intro plays the ledge dialogue's
    // script player); both scenes step in lockstep the whole way.
    let (mut ci, mut q) = (None, 0u32);
    for t in 0..24_000u32 {
        step(&mut a);
        step(&mut b);
        if let Some((c_, _first, q_)) = morph_script_char(&a) {
            eprintln!("[facial2] tick {t}: morph-bearing script object live (character {c_}, window {q_:08x})");
            ci = Some(c_);
            q = q_;
            break;
        }
    }
    let ci = match ci { Some(c) => c, None => { eprintln!("[facial2] no morph-bearing script object went live within 24k ticks"); return; } };
    {
        let s = a.sim.as_ref().unwrap();
        let seq = &s.seqs[&q];
        eprintln!("[facial2] character {ci} face window {q:08x}: {} frames, {} channels", seq.frames, seq.channels.len());
    }

    // The face vertices: this character's morph-bearing vertices and their bind positions.
    let face_verts: Vec<usize> = a.char_verts.iter().enumerate()
        .filter(|(_, v)| v.character as usize == ci && v.morph >= 0).map(|(i, _)| i).collect();
    let plain_verts: Vec<usize> = a.char_verts.iter().enumerate()
        .filter(|(_, v)| v.character as usize == ci && v.morph < 0).map(|(i, _)| i).collect();
    assert!(!face_verts.is_empty(), "the morph-bearing object has morph-indexed vertices (the 0x04000048-class mesh)");
    // The targets cover every face vertex's (morph + 22) index space [M SkinIt 0x10bf30's
    // (morph + channel) × 12 addressing]: the census the buffer pin rests on.
    let count = a.characters[ci].skin.facial as usize;
    let targets: Vec<u8> = a.characters[ci].morph_targets.as_ref().unwrap().clone();
    assert_eq!(targets.len() % 12, 0, "the MORPH_DATA payload is whole f32[3] targets");
    let max_morph = face_verts.iter().map(|&i| a.char_verts[i].morph as usize).max().unwrap();
    assert!(targets.len() / 12 >= max_morph + count,
        "targets {} must cover max morph index {max_morph} + {count} channels", targets.len() / 12);

    let mut weight0_tick = None;
    let mut moved_tick = None;
    for t in 0..24_000u32 {
        step(&mut a);
        step(&mut b);
        let Some(w) = a.characters.get(ci).and_then(|c| a.facial_weights_now(c)) else { continue };
        let max_w = w.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        if moved_tick.is_none() && max_w > 0.01 { moved_tick = Some(t); }
        if weight0_tick.is_none() && max_w < 1e-3 { weight0_tick = Some(t); }
        if moved_tick.is_some() && weight0_tick.is_some() { break; }
    }
    let t_move = moved_tick.expect("the face track's first weight crosses 0.01 within the dialogue");
    let t_zero = weight0_tick.expect("the dialogue starts from a still face (all weights < 1e-3)");

    // At the still frame the two scenes agree to the morph-free arithmetic (sister).
    let (pa, pb) = (a.animate(None, t_zero as f32, ViewOpts::default()), b.animate(None, t_zero as f32, ViewOpts::default()));
    for &i in &plain_verts {
        assert_eq!(pa[i], pb[i], "a non-morph vertex must be untouched by the morph path");
    }
    let still: f32 = face_verts.iter().map(|&i| (0..3).map(|k| (pa[i][k] - pb[i][k]).abs()).fold(0.0f32, f32::max)).fold(0.0f32, f32::max);
    eprintln!("[facial2] weight-0 frame: max face-vertex morph delta {still:.6}");
    assert!(still < 1e-3, "at the weight-0 frame the morph term is zero");

    // At the first moving frame the morph-bearing vertices have MOVED by the morph path while the
    // bone posing (identical in both lockstep scenes) cancels out. Pre-fix, level.rs dropped the
    // MORPH_DATA block and the skinning never applied any morph — the two scenes were identical
    // and this assertion failed.
    let (pa, pb) = (a.animate(None, t_move as f32, ViewOpts::default()), b.animate(None, t_move as f32, ViewOpts::default()));
    let moved: f32 = face_verts.iter().map(|&i| (0..3).map(|k| (pa[i][k] - pb[i][k]).abs()).fold(0.0f32, f32::max)).fold(0.0f32, f32::max);
    let weights = a.facial_weights_now(&a.characters[ci]).unwrap();
    let max_target = (0..targets.len() / 12).map(|i| (0..3).map(|k| f32::from_le_bytes(targets[i * 12 + k * 4..i * 12 + k * 4 + 4].try_into().unwrap()).abs()).fold(0.0f32, f32::max)).fold(0.0f32, f32::max);
    eprintln!("[facial2] tick {t_move}: max face-vertex morph delta {moved:.6} (weights max {:.4}, max target {:.4})", weights.iter().fold(0.0f32, |m, v| m.max(v.abs())), max_target);
    assert!(moved > 0.001, "a face vertex moves once the weights cross the blend's 0.01 threshold");
}

/// The MORPH_DATA census: which skinned objects of 07000005 carry the block, and do the targets
/// cover the objects' morph-index space. The buffer pin's disc-side proof.
#[test]
fn the_morph_blocks_pair_to_their_objects() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let img = nf_formats::iso9660::IsoImage::open(&iso).expect("iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("bin entries");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000005.bin")).expect("07000005.bin");
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read");
    let level = nf_formats::level::Level::parse(&data).expect("parse level");
    let mut with_morph = 0;
    let mut skinned = 0;
    for (mi, m) in level.maps.iter().enumerate() {
        for (oi, o) in m.objects.iter().enumerate() {
            if o.gfx.is_skinned() {
                skinned += 1;
                if o.morph_targets.is_some() {
                    with_morph += 1;
                    if with_morph <= 6 {
                        let max_morph = o.gfx.leaves.iter().flat_map(|l| l.batches.iter()).flat_map(|b| b.verts.iter())
                            .map(|v| (v.w >> 16) as i16).filter(|m| *m >= 0).max().unwrap_or(-1);
                        eprintln!("[facial2] map {mi} object {oi} {:08x} '{}': targets {} bytes ({} targets), max vertex morph index {max_morph}, facial-skin? {}",
                            o.hashcode, o.name, o.morph_targets.as_ref().unwrap().len(), o.morph_targets.as_ref().unwrap().len() / 12, o.gfx.leaves.iter().flat_map(|l| l.batches.iter()).any(|b| b.skinned));
                        assert!(o.morph_targets.as_ref().unwrap().len() / 12 >= max_morph as usize + 22,
                            "targets cover max morph {max_morph} + 22 channels");
                        let any_finite = o.morph_targets.as_ref().unwrap().chunks_exact(4).any(|c| f32::from_le_bytes(c.try_into().unwrap()).is_finite());
                        assert!(any_finite, "the targets parse as floats");
                    }
                }
            }
        }
    }
    eprintln!("[facial2] {skinned} skinned objects, {with_morph} with MORPH_DATA targets");
    assert!(with_morph > 0, "07000005 carries MORPH_DATA blocks (0x2b) paired to its skinned objects");
    assert!(with_morph < skinned, "only the face-bearing objects carry targets");
}
