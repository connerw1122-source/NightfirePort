//! fix-cutscene-brightness (102g): the 07000002 intro's cut-scene actors lit near-white — the
//! playtest's "the models in the game during the cutscene are extremely bright" (results
//! 1013-1825, the screenshot at `--bond -82.30,5.30,32.05` frame 170 shows the port's Bond and
//! the man at the computer near-white where PCSX2's 02 intro lights them darkly to match the
//! room).
//!
//! The game's rule: every object is linked to its cel by position
//! (`control_link_object_to_cel` 0x1346b8 — the None-cel path calls `build_LinkToRoom`, the
//! position relink), and `Lights_CalcAmbientLight` 0x1cb178 ramps `obj+0x100..0x102` toward that
//! cel's ambient bytes (`cel+0x9a/0x9c/0x9e` shifted by `switch_channels[cel+0xa2]`), snapping
//! on the first link while the triple is the 0xff,ff,ff sentinel of `control_init_object`
//! 0x134630. The port's ramp (`Sim::ramp_char_ambient`) resolves a script character's position
//! through `characters_script_pos` — whose sequence filter was the strict
//! `nf_formats::anim::fits`: the intro's actors play shared human tracks whose record byte is
//! skeleton 0 on skeleton-N skins, every candidate sequence was rejected, the position was None,
//! and the 0xff sentinel never snapped. The fix adopts `script_pose`'s relaxed filter (the
//! skeleton-id match dropped, the channel-count guard kept).
//!
//! Opt-in via NF_ISO (the slim ISO is enough).

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load_02_intro() -> nf_view::scene::Scene {
    let mut scene = nf_view::load(iso().as_deref().expect("NF_ISO"), "07000002.bin", false).expect("load 07000002");
    let sim = scene.sim.as_mut().unwrap();
    let idle = Input::default();
    // Into the intro (the user's screenshot is frame ~170): no skip — the cut-scene plays.
    for _ in 0..200 { sim.step_bond(idle, false, false, false); }
    scene
}

/// The intro's script actors — characters linked to a running NIS stream (`char_script` Some) —
/// ride their animation's root translation in the ambient ramp: their `obj+0x100..0x102` snapped
/// to the bunker cel's ambient on the first link [M control_link_object_to_cel 0x1346b8's
/// 0xffffff sentinel snap; Lights_CalcAmbientLight 0x1cb178's ClrStep ramp], instead of staying
/// the full-bright 0xff,ff,ff sentinel.
#[test]
fn the_02_intro_actors_take_the_rooms_ambient_not_full_bright() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = load_02_intro();
    let sim = scene.sim.as_ref().unwrap();
    // The intro cast: script-linked characters whose animation is live this tick.
    let mut cast = Vec::new();
    for ci in 0..sim.char_ambient.len() {
        let Some(src) = sim.char_script.get(ci).copied().flatten() else { continue };
        let Some(skin) = sim.char_script_skin.get(ci).copied().flatten() else { continue };
        if sim.script_anim(src, skin).is_none() { continue; }
        cast.push(ci);
    }
    assert!(!cast.is_empty(), "the intro's script actors have live animations at frame ~200 [D the playtest's screenshot moment]");
    // The bunker room's ambient — the target every actor in the set ramps to. Dark: every channel
    // well under the full-bright sentinel [D the playtest's screenshot].
    let bunker = sim.ambient_at(sim.bond.pos).expect("the bunker cel resolves at Bond's intro spot");
    eprintln!("the bunker's ambient: {:?}; live script actors: {:?}", bunker, cast);
    assert!(bunker.iter().all(|&b| b < 0x40), "the bunker cel's ambient is dark (got {:?})", bunker);
    for ci in &cast {
        let pos = sim.characters_script_pos(*ci);
        eprintln!("char[{ci}] amb {:?} pos {:?}", sim.char_ambient[*ci], pos);
        let Some(p) = pos else {
            panic!("char[{ci}]'s live animation resolves a position — the relaxed sequence filter \
                    keeps the shared human track [M script_pose's guard] (the strict fits rejected \
                    it and the ambient stayed 0xff)");
        };
        assert_eq!(sim.char_ambient[*ci], bunker,
            "intro actor char[{ci}] carries the room's ambient (the first link snapped \
             obj+0x100..0x102 to it [M 0x1346b8]) — not the 0xff,ff,ff sentinel (got {:?})",
            sim.char_ambient[*ci]);
        // The render-facing number: the VU1 TAmultiplier floats of `View_SetupRenderModes_Tweak`
        // 0x1e5888 — the ambient bytes over 255, not 1.0.
        let mul = sim.char_multiplier(*ci);
        assert!(mul.iter().all(|&f| f < 0.5), "the actor's draw multiplier is the dark ambient, not full bright (got {:?})", mul);
        let _ = p;
    }
}

/// Sister: the drone-backed characters keep their own ramped ambients (the relaxed filter touches
/// only the script-character position resolution — the drone path is untouched).
#[test]
fn the_drone_characters_ambient_ramp_is_untouched() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = load_02_intro();
    let sim = scene.sim.as_ref().unwrap();
    let mut checked = 0;
    for ci in 0..sim.char_ambient.len() {
        let Some(di) = sim.char_drone.get(ci).copied().flatten() else { continue };
        let d = &sim.drones.list[di];
        let target = sim.ambient_at([d.pos[0], d.pos[1], d.pos[2]]);
        if let Some(t) = target {
            assert_ne!(sim.char_ambient[ci], [0xffu8; 3],
                "drone char[{ci}] ramped off the sentinel (got 0xff,ff,ff)");
            assert!(sim.char_ambient[ci].iter().zip(t.iter()).all(|(a, b)| (*a as i32 - *b as i32).abs() <= 8),
                "drone char[{ci}] rides its cel's ambient ramp (got {:?} vs target {:?})", sim.char_ambient[ci], t);
            checked += 1;
        }
        if checked >= 8 { break; }
    }
    assert!(checked >= 4, "the level's drone characters resolve ambients (checked {checked})");
}

/// Sister: the first-person view-model characters keep riding Bond's position (their ambient is
/// the bunker's — the ramp's view-model branch, untouched).
#[test]
fn the_view_model_characters_keep_bonds_ambient() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = load_02_intro();
    let sim = scene.sim.as_ref().unwrap();
    if sim.view_model_chars.is_empty() { eprintln!("no view models live yet — skipping"); return; }
    let bunker = sim.ambient_at(sim.bond.sphere_centre()).expect("Bond's cel resolves");
    for &ci in &sim.view_model_chars {
        assert_eq!(sim.char_ambient[ci], bunker,
            "view-model char[{ci}] rides Bond's ambient (got {:?} vs {:?})", sim.char_ambient[ci], bunker);
    }
}
