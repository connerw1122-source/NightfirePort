//! fix-ai-remainder: the corpse fade in the renderer (G-026). The game's `NDrone2_FadeOut` 0x172dd8
//! decays the drone object's alpha byte obj+0x106 (0.6 per tick in Fade 0x48 after Dead 0x47's
//! 300-tick timer, 8.0 in FadeFast 0x49) and the character draw multiplies the vertices' alpha by it,
//! disabling the object at 0. The port kept the sim-side alpha but never wired it into the posed
//! vertices — dead guards stayed solid until they vanished. Opt-in: NF_ISO.

use nf_game::player::Input;

fn max_char_alpha(scene: &nf_view::scene::Scene, ci: u32) -> u8 {
    scene.posed_vertices(None, 0.0, nf_view::scene::ViewOpts::default()).iter()
        .zip(scene.char_verts.iter())
        .filter(|(_, cv)| cv.character == ci && cv.attachment.is_none() && cv.pickup.is_none() && cv.searchlight.is_none() && cv.door.is_none())
        .map(|(v, _)| v.rgba[3]).max().unwrap_or(255)
}

/// fix-ai-remainder: the renderer's posed character vertices carry the drone's fade alpha — a guard
/// in Fade 0x48 (alpha decayed from 255, `NDrone2_FadeOut` 0.6/tick) draws faded, a guard at full
/// alpha draws at 255 (the sister). The alpha value is the sim's own Fade-state decay
/// (`Drones::tick` runs the same 0x48 machine; the end-to-end kill attempts are in the report).
#[test]
fn the_corpse_fade_alpha_reaches_the_posed_vertices() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    for _ in 0..10 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    // A living guard put into the Fade state the death chain ends in, with the alpha Dead 0x47's
    // timer hands to Fade 0x48 (~60 decayed ticks in). Sister: another guard untouched at 255.
    let (gi, ai) = {
        let sim = scene.sim.as_mut().unwrap();
        let guards: Vec<usize> = sim.drones.list.iter().enumerate()
            .filter(|(_, d)| d.present && d.enabled && !d.corpse && d.team == 1)
            .map(|(i, _)| i).collect();
        assert!(guards.len() >= 2, "two living guards at level start");
        (guards[0], guards[1])
    };
    {
        let sim = scene.sim.as_mut().unwrap();
        let g = &mut sim.drones.list[gi];
        g.corpse = true;                 // Dead 0x47 ENTER set obj+0xF0 |= 0x20
        g.state = 0x48;                  // Fade 0x48 (`NDrone2_DSTATE_FadeOut`)
        g.alpha = 120.0;                 // ~60 ticks of the 0.6/tick ramp
        sim.drones.list[ai].alpha = 255.0;
    }
    for _ in 0..3 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    let (faded_alpha, alive_alpha, fci, aci) = {
        let sim = scene.sim.as_ref().unwrap();
        (sim.drones.list[gi].alpha, sim.drones.list[ai].alpha,
         sim.char_drone.iter().position(|c| *c == Some(gi)).expect("the fading guard's scene character") as u32,
         sim.char_drone.iter().position(|c| *c == Some(ai)).expect("the alive guard's scene character") as u32)
    };
    assert!(faded_alpha < 255.0, "the Fade state decays the drone's alpha (got {faded_alpha})");
    assert_eq!(alive_alpha, 255.0);
    let faded = max_char_alpha(&scene, fci);
    let alive = max_char_alpha(&scene, aci);
    // The skins carry their own per-vertex alpha; the sister is "unchanged", the fade is "scaled".
    let raw = |ci: u32| scene.vertices[scene.char_vertex_start..].iter().zip(scene.char_verts.iter())
        .filter(|(_, cv)| cv.character == ci && cv.attachment.is_none() && cv.pickup.is_none() && cv.searchlight.is_none() && cv.door.is_none())
        .map(|(v, _)| v.rgba[3]).max().unwrap_or(255);
    assert_eq!(alive, raw(aci), "a full-alpha guard's vertices keep the skin's own alpha");
    assert_eq!(faded, (((raw(fci) as f32) * (faded_alpha / 255.0)).round() as u8).clamp(0, 255),
        "the fading guard's posed vertices carry the drone's alpha ({} on raw {})", faded, raw(fci));
    let _ = Input::default();
}
