//! fix-lighting unit tests: the light selection and falloff values against the decompile
//! (`Lights_CalcClosestLights` 0x1cb2e0, `psiLight_SetLights` 0x10e588), `Light_Create`/
//! `Light_Update` lifetimes (0x1cc3d8 / 0x1cafb8), and the VU1 point-light formula read from
//! ACTION.ELF `.vutext` (the point-light program, I-mem ~832-940). Render-independent: the same
//! `shade_vertex` the renderer mirrors in WGSL.

use nf_game::light::{shade_vertex, Light, LightList};

/// A light `d` away with radius `r`: accepted when d ≤ sphere + r, falloff = d/(r·0.5) [M 0x1cb3f0].
#[test]
fn falloff_and_radius_gate_match_the_decompile() {
    let mut ll = LightList::default();
    let mut rng = nf_game::drone::Rng::new(1);
    ll.create(4.0, 1.0, [10.0, 0.0, 0.0], [255, 128, 0], 0, -1, 3, 0, &mut rng);
    // A 2-radius sphere at the origin: d = 10 > 2 + 4 → rejected.
    assert!(ll.resolve(&|_| [0.0; 3]).closest([0.0, 0.0, 0.0, 2.0], 0x80).is_empty(), "outside sphere + radius");
    // d = 5 ≤ 2 + 4 → accepted with falloff 5/(4·0.5) = 2.5.
    ll.lights[0].pos = [5.0, 0.0, 0.0];
    let got = ll.resolve(&|_| [0.0; 3]).closest([0.0, 0.0, 0.0, 2.0], 0x80);
    assert_eq!(got.len(), 1);
    assert!((got[0].1 - 2.5).abs() < 1e-5, "falloff = d / (radius · 0.5), got {}", got[0].1);
}

/// The kind gates: 0 needs flags & 0x80, 1 skips flags & 0x100, 2 needs 0x80 and d ≤ 4 (2D).
#[test]
fn kind_gates_match_the_decompile() {
    let mut ll = LightList::default();
    let mut rng = nf_game::drone::Rng::new(2);
    ll.create(4.0, 1.0, [1.0, 0.0, 0.0], [255, 255, 255], 0, -1, 0, 0, &mut rng);
    assert!(ll.resolve(&|_| [0.0; 3]).closest([0.0, 0.0, 0.0, 2.0], 0).is_empty(), "kind 0 without flags 0x80");
    assert_eq!(ll.resolve(&|_| [0.0; 3]).closest([0.0, 0.0, 0.0, 2.0], 0x80).len(), 1, "kind 0 with flags 0x80");
    ll.lights[0].kind = 1;
    assert_eq!(ll.resolve(&|_| [0.0; 3]).closest([0.0, 0.0, 0.0, 2.0], 0x100).len(), 0, "kind 1 skipped for 0x100");
    assert_eq!(ll.resolve(&|_| [0.0; 3]).closest([0.0, 0.0, 0.0, 2.0], 0x80).len(), 1, "kind 1 applies without 0x100");
    // Kind 2: the 2D (xz) distance against the tracked position, ≤ 4.0, intensity (1 − d·0.25)·50.
    ll.lights[0].kind = 2;
    ll.lights[0].track = Some(7);
    let view = ll.resolve(&|_| [3.0, 50.0, 0.0]); // y is ignored by the 2D distance
    let got = view.closest([0.0, 0.0, 0.0, 2.0], 0x80);
    assert_eq!(got.len(), 1, "kind 2 accepted at 2D distance 3 ≤ 4");
    let (_, lc) = view.pick2([0.0, 0.0, 0.0, 2.0], 0x80);
    assert!((lc[0][0] - 255.0 * (1.0 - 3.0 * 0.25) * 50.0).abs() < 1e-3, "kind 2 intensity (1 − d·0.25)·50 [M 0x1cb40c]");
    let view = ll.resolve(&|_| [4.1, 0.0, 0.0]);
    assert!(view.closest([0.0, 0.0, 0.0, 2.0], 0x80).is_empty(), "kind 2 rejected at 2D distance 4.1 > 4");
}

/// At most the two closest upload (`psiLight_SetLights`), sorted by the falloff ascending
/// (the QuickSort comparator at 0x1d3350 compares light+0x38).
#[test]
fn only_the_two_closest_upload_sorted_by_falloff() {
    let mut ll = LightList::default();
    let mut rng = nf_game::drone::Rng::new(3);
    ll.create(4.0, 1.0, [20.0, 0.0, 0.0], [255, 0, 0], 0, -1, 3, 0, &mut rng);
    ll.create(4.0, 1.0, [3.0, 0.0, 0.0], [0, 255, 0], 0, -1, 3, 0, &mut rng);
    ll.create(4.0, 1.0, [5.5, 0.0, 0.0], [0, 0, 255], 0, -1, 3, 0, &mut rng);
    ll.create(4.0, 1.0, [30.0, 0.0, 0.0], [255, 255, 0], 0, -1, 3, 0, &mut rng);
    let (lp, lc) = ll.resolve(&|_| [0.0; 3]).pick2([0.0, 0.0, 0.0, 2.0], 0x80);
    assert!((lp[0][0] - 3.0).abs() < 1e-5 && (lp[1][0] - 5.5).abs() < 1e-5, "closest two by falloff");
    assert!((lc[0][1] - 255.0).abs() < 1e-3 && (lc[1][2] - 255.0).abs() < 1e-3, "their colours");
    assert!((lc[0][3] - 1.0 / 16.0).abs() < 1e-6, "1/r² in TColor.w [M psiLight_SetLights 0x10e588]");
}

/// `Light_Create`'s life mapping and `Light_Update`'s delete-at-zero [M 0x1cc3d8/0x1cafb8].
#[test]
fn life_and_delete_match_the_decompile() {
    let mut ll = LightList::default();
    let mut rng = nf_game::drone::Rng::new(4);
    ll.create(1.0, 1.0, [0.0; 3], [255, 255, 255], 0, 0, 1, 0, &mut rng);   // the muzzle: life arg 0
    ll.create(1.0, 1.0, [0.0; 3], [255, 255, 255], 0, -1, 0, 0, &mut rng);  // permanent
    assert_eq!(ll.lights[0].life, 1, "life arg 0 → +0x20 = 1");
    assert_eq!(ll.lights[1].life, 0, "life arg −1 → +0x20 = 0 (permanent)");
    ll.update(&|_| false, 0, &mut rng);
    assert_eq!(ll.lights.len(), 1, "the life-1 light is deleted after one update");
    assert_eq!(ll.lights[0].life, 0, "the permanent light stays");
}

/// The VU1 formula: atten = max(0, 1/d² − 1/r²), diffuse = max(0, atten·dot(N, L)) with the RAW L,
/// colour = base + TColor.rgb · diffuse [M .vutext 0x241c80, point-light program ~832-940].
#[test]
fn the_vu1_shading_formula_matches_the_microcode() {
    let lp = [[2.0, 0.0, 0.0, 1.0], [0.0; 4]];
    let r = 2.0f32;
    let lc = [[255.0, 255.0, 255.0, 1.0 / (r * r)], [0.0; 4]];
    // Vertex at the origin, normal +x: L = (2,0,0), d² = 4, 1/d² = 0.25, 1/r² = 0.25 → atten = 0.
    // d = r exactly → no light (the falloff reaches zero AT the radius).
    let got = shade_vertex([10.0, 20.0, 30.0], [1.0, 0.0, 0.0], [0.0, 0.0, 0.0], &lp, &lc);
    assert_eq!(got, [10.0, 20.0, 30.0], "atten = max(0, 1/d² − 1/r²) = 0 at d = r");
    // Vertex at (1,0,0): d = 1, 1/d² = 1, atten = 1 − 0.25 = 0.75, N·L = 1 → term = 0.75.
    let got = shade_vertex([10.0, 20.0, 30.0], [1.0, 0.0, 0.0], [1.0, 0.0, 0.0], &lp, &lc);
    assert!((got[0] - (10.0 + 255.0 * 0.75)).abs() < 1e-3, "base + colour·atten·N·L, got {}", got[0]);
    // Normal away from the light: no light.
    let got = shade_vertex([10.0, 20.0, 30.0], [-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], &lp, &lc);
    assert_eq!(got, [10.0, 20.0, 30.0], "max(0, N·L) clamps backfaces");
    // Zero normal: unlit (the sprites/snow/radar rule).
    let got = shade_vertex([10.0, 20.0, 30.0], [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], &lp, &lc);
    assert_eq!(got, [10.0, 20.0, 30.0], "zero normals stay unlit");
}

/// A character next to Bond's muzzle light is brighter that frame; the sister asserts the same
/// vertex without the light is not. This is the render-independent form of the user's symptom
/// (lights fall on characters).
#[test]
fn a_character_next_to_the_muzzle_light_gets_brighter_that_frame() {
    let mut ll = LightList::default();
    let mut rng = nf_game::drone::Rng::new(5);
    // The muzzle light: P2K radius 2.0 [L weapon_data+0x58 of 0x33], rgb (0x54,0x55,0x56) — the
    // flash colour — intensity 2.0 [M 0x1a4370-0x1a43a8].
    ll.create(2.0, 2.0, [0.5, 1.0, 0.0], [0x54, 0x55, 0x56], 0, 0, 1, 0, &mut rng);
    let view = ll.resolve(&|_| [0.0; 3]);
    assert_eq!(view.closest([0.0, 1.0, 0.0, 1.0], 0x80).len(), 1, "the guard's sphere is inside the muzzle radius");
    let (lp, lc) = view.pick2([0.0, 1.0, 0.0, 1.0], 0x80);
    let base = [120.0, 120.0, 120.0];
    // A surface facing the muzzle (the light sits at +x of the vertex).
    let lit = shade_vertex(base, [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], &lp, &lc);
    let unlit = shade_vertex(base, [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], &[[0.0; 4]; 2], &[[0.0; 4]; 2]);
    assert!(lit[0] > unlit[0] && lit[1] > unlit[1] && lit[2] > unlit[2], "the muzzle light brightens the character that frame");
    // The falloff shape: twice as far, much dimmer (1/d²).
    let (lp2, lc2) = view.pick2([0.0, 1.0, 0.0, 1.0], 0x80);
    let far = shade_vertex(base, [1.0, 0.0, 0.0], [0.0, 1.0, -1.0], &lp2, &lc2);
    assert!(far[0] < lit[0], "the falloff dims with distance");
}

/// `Light_Create` always draws from the shared RNG, arg 0 included — the state advances [M 0x1cc3d8].
#[test]
fn the_create_draw_advances_the_shared_rng_even_for_zero() {
    let a = nf_game::drone::Rng::new(7);
    let mut b = nf_game::drone::Rng { x: a.x, y: a.y, ..Default::default() };
    let mut ll = LightList::default();
    ll.create(1.0, 1.0, [0.0; 3], [255, 255, 255], 0, -1, 0, 0, &mut b);
    { let a = &a; assert_ne!((a.x, a.y), (b.x, b.y), "Rand_Rand(param_11) drew (the muzzle light fires on every shot)"); }
    // The value with n = 0 is 0, so the flicker counter stays 0 [M ((y & 0xffff) × 0) >> 16].
    assert_eq!(ll.lights[0].flicker_count, 0);
}

/// A `Light` record's defaults match `Light_Create`'s stores.
#[test]
fn create_fills_the_record_like_the_game() {
    let mut ll = LightList::default();
    let mut rng = nf_game::drone::Rng::new(8);
    let i = ll.create(3.5, 1.25, [1.0, 2.0, 3.0], [10, 20, 30], 0x12, -1, 0, 0x40, &mut rng);
    let l = &ll.lights[i];
    assert_eq!((l.radius, l.intensity), (3.5, 1.25));
    assert_eq!(l.pos, [1.0, 2.0, 3.0]);
    assert_eq!(l.rgb, [10, 20, 30]);
    assert_eq!(l.kind, 0);
    assert!(l.active);
    assert_eq!((l.flicker_period, l.flicker_base), (0x40, 0x20), "+0x24 = flicker, +0x26 = flicker >> 1");
}

/// The searchlight's kind-2 record as `Searchlight_Create` 0x1cb4f8 builds it inline.
/// fix-searchlight-remainder re-premised: `psiLight_SetLights` 0x10e588:34-38 uploads the node's
/// STATIC +0x10 (the housing — written once before the −10000 move [M 0x1cb4f8:154-167]) while the
/// intensity/acceptance measure the tracked spot's LIVE position [M 0x1cb2e0:41-44, store 0x1cb400].
/// The old assertion ("kind 2 rides the tracked position") pinned the pre-fix reading that lit a
/// halo at the ground circle instead of the beam from the housing.
#[test]
fn the_searchlight_light_record_matches_the_inline_create() {
    let l = Light {
        pos: [10.0, 40.0, 10.0], life: 0, channel: 0, flicker_period: 0, flicker_base: 0, flicker_count: 0,
        flicker_mask: 0, sound: 0, track: Some(3), radius: 500.0, intensity: 1.0, falloff: 0.0,
        rgb: [255, 255, 255], kind: 2, active: true,
    };
    let mut ll = LightList::default();
    ll.lights.push(l);
    let view = ll.resolve(&|i| { assert_eq!(i, 3, "the tracked part's position is read per selection"); [1.0, 2.0, 3.0] });
    assert_eq!(view.lights[0].pos, [10.0, 40.0, 10.0, 1.0], "the upload stays at the record's (housing) position");
    assert_eq!(view.lights[0].track_pos, [1.0, 2.0, 3.0, 1.0], "the live spot rides track_pos");
    let (lp, _) = view.pick2([1.0, 2.0, 4.0, 1.0], 0x80);
    assert_eq!(lp[0], [10.0, 40.0, 10.0, 1.0], "psiLight_SetLights uploads the housing [M 0x10e588:34-38]");
    let _ = Light { pos: [0.0; 3], life: 0, channel: 0, flicker_period: 0, flicker_base: 0, flicker_count: 0, flicker_mask: 0, sound: 0, track: None, radius: 1.0, intensity: 1.0, falloff: 0.0, rgb: [0, 0, 0], kind: 3, active: false };
}
