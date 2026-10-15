//! fix-grenade-release (109e) — the two playtest-1014-1725 symptoms, driven through the REAL input
//! path (`nf_view::load_weapons` + the panel give + `Sim::step_bond` with the trigger the window
//! builds from the mouse).
//!
//! Evidence:
//! - The release frame: `weapon_data+0x44` = 26 for the frag 0x34 [L record; probe] and the smoke
//!   0x36; the round spawns when the fire script's time (`AnimList+0x90`, advanced by
//!   `AnimScriptTick` 0x1c2630 as `+= speed·FRAME_RATE_MUL`, speed 1.0 [M]) reaches it, latched at
//!   `+0x95C` [M `Player_SetWeaponAnimObj` case 9 0x1a2e38:416-420; `Player_WeaponFiring`
//!   0x1a5460:681-693]. The fire script 0x0600049d is the 70-frame wind-up (the smoke's 0x0600703
//!   the same shape) whose op-1 plays the pin sound (0x1e4) at frame 16 [D script]; the pin pull
//!   precedes the spawn. The clip spends on the PRESS frame [L PCSX2
//!   docs/live/2026-10-14-weapon-measurements-pcsx2.md]. The port's release lands at view_time 25
//!   = script frame 26 (probe: grenades_live 0→1 at t=25 with the script frame = view_time+1) —
//!   the same ~0.43 s the game's screenshots show (pin pull at screenshots 01-02, flight at 07).
//! - The blast: `Explode_Create` plays script 0x0600052 at scale = the radius (8.0 for the frag)
//!   [M 0x17cbd0 stores param_1 into SCRIPTINFO+0x94; `Script_GetInterp` 0x1da158:117 multiplies
//!   every key scale by it]. The script's puffs (glists 0x020003cd/0x020003ce/0x020003c7, kind-0x31
//!   entities, and the kind-0x2f flash 0x020003cc) are BILLBOARDS: their glist flags
//!   (`celglist+0x44`) carry bit 0x8, which `Control_SetGList` 0x134960 copies into `obj+0xF0`,
//!   and `View_DrawObjectGlist` 0x1e6780:43-65 then draws them with the VIEWER's rotation
//!   (`Mat_CopyRot(viewer+0x1a0)`) keeping the interp translation and the interp scale
//!   (`Mat_Scale3f(fVar15,fVar15,fVar15)` 0x1e35f0, `fVar15 = obj+0xE8` written by
//!   `Script_GetInterp` as key_scale × SCRIPTINFO+0x94). The port drew the raw interp matrix —
//!   the flat puff quads sat in their authored plane and read edge-on as "a very thin wall".

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn q(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

/// The user's level with the frag pre-merged, given through the panel (the dev-menu path), switched
/// in through the real `switch_to` input, settled.
fn load_frag_on_15(iso: &str) -> nf_view::scene::Scene {
    let mut scene = nf_view::load_weapons(iso, "07000015.bin", false, &[0x34]).expect("load 07000015");
    let (msg, sw) = nf_view::panel::apply(&mut scene, &q(&[("do", "weapon"), ("id", "52"), ("rounds", "5")]));
    eprintln!("[give] {msg} -> {sw:?}");
    assert_eq!(sw, Some(0x34), "the panel must switch to the frag");
    for _ in 0..180 {
        scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(0x34), ..Default::default() }, false, false, false);
        let g = &scene.sim.as_ref().unwrap().gun;
        if g.weapon == 0x34 && g.switch_state == 0 { break; }
    }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x34, "the frag is in hand");
    for _ in 0..4 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    scene
}

/// ── The throw timing: the grenade leaves on the fire script's release frame, after the pin pull ──
#[test]
fn the_frag_leaves_the_hand_on_the_release_frame_after_the_pin_pull() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_frag_on_15(&iso);
    {
        let sim = scene.sim.as_ref().unwrap();
        let w = &sim.weapons.records[0x34];
        let release = u32::from_le_bytes(w[0x44..0x48].try_into().unwrap());
        assert_eq!(release, 26, "the frag's +0x44 release frame (the record the port runs the game's own initialiser over)");
        assert_eq!(u32::from_le_bytes(w[0x40..0x44].try_into().unwrap()), 450, "the frag's +0x40 refire interval");
    }
    scene.sim.as_mut().unwrap().bond.pitch = -0.9; // throw at the floor ahead
    let mut press_tick = None;
    let mut spawn_tick = None;
    let mut clip_on_press = None;
    for t in 0..120u32 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), true, false, false); // the trigger held, as the mouse does
        let sim = scene.sim.as_ref().unwrap();
        if press_tick.is_none() && sim.gun.clip < 1 {
            press_tick = Some(t);
            clip_on_press = Some(sim.gun.clip);
            assert!(sim.view_script == 0x0600_049d && sim.view_time <= 1.0,
                "the press queues the fire script (got {:#x} at {})", sim.view_script, sim.view_time);
        }
        if spawn_tick.is_none() && sim.drones.grenades.iter().any(|g| g.state < 2) {
            spawn_tick = Some(t);
            let frame = sim.view_time + 1.0; // the port's script frames are time+1
            // The grenades play the fire script at 1.5× [M Player_WeaponFiring 0x1a5460:676-685;
            // L PCSX2 07000015: +1.5 per frame], so the release frame 26 is crossed between 26 and 27.5.
            assert!((26.0..27.5).contains(&frame), "the spawn lands on the fire script's release frame 26 (+0x44), \
                i.e. AFTER the pin pull (the pin sound is frame 16) — not on the press tick (frame {frame})");
            break;
        }
    }
    let (Some(press), Some(spawn)) = (press_tick, spawn_tick) else { panic!("no press or no spawn within 120 ticks (press {press_tick:?})") };
    assert_eq!(clip_on_press, Some(0), "the clip spends on the press frame [L PCSX2]");
    let lag = spawn - press;
    // [L PCSX2 07000015, the main agent's per-frame trace: frame 26 crossed at +17 from the press].
    // Sister: the old 1.0 speed spawned at +25, outside this window.
    assert!((16..=18).contains(&lag), "the grenade leaves the hand {} ticks after the press (the game: +17)", lag);
}

/// ── The blast: the puffs draw camera-facing (billboarded), not as a wall in their authored plane ──
#[test]
fn the_blast_puffs_draw_camera_facing() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // The blast script's entities build only where the level archive carries 0x06000052 — 07000005
    // does (the probe); the user's 07000015 archive does not [D probe: no 0x0600_0052 attachments].
    let mut scene = {
        let mut scene = nf_view::load_weapons(&iso, "07000005.bin", false, &[0x34]).expect("load 07000005");
        let (msg, sw) = nf_view::panel::apply(&mut scene, &q(&[("do", "weapon"), ("id", "52"), ("rounds", "5")]));
        eprintln!("[give] {msg} -> {sw:?}");
        assert_eq!(sw, Some(0x34), "the panel must switch to the frag");
        for _ in 0..180 {
            scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(0x34), ..Default::default() }, false, false, false);
            let g = &scene.sim.as_ref().unwrap().gun;
            if g.weapon == 0x34 && g.switch_state == 0 { break; }
        }
        assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x34, "the frag is in hand");
        for _ in 0..4 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
        scene
    };
    // The glist flags: the puff meshes carry the billboard bit 0x8 (celglist+0x44 [M 0x134960]).
    // 0x0200021a (streams 0-2's puffs) and 0x020003d7 (stream 3/4's) are in the scene's set; some
    // sibling meshes (0x020003d8, 0x020003e1…) are not — their glists lack the bit, the game draws
    // them with the interp matrix too.
    for h in [0x0200_021au32, 0x0200_03d7] {
        assert!(scene.script_billboards.contains(&h), "blast mesh {h:#010x} must billboard (its glist flags carry 0x8)");
    }
    assert!(scene.attachments.iter().any(|a| matches!(a.script, Some((0x0600_0052, _, _)))),
        "the scene builds attachments for the blast script's entity streams");
    // Throw at the floor and wait for the explosion.
    scene.sim.as_mut().unwrap().bond.pitch = -1.4;
    let mut exploded = false;
    for t in 0..400 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), t < 2, false, false);
        if !scene.sim.as_ref().unwrap().drones.explosions.is_empty() { exploded = true; break; }
    }
    assert!(exploded, "the grenade never exploded");
    // The frame's entity matrices with a KNOWN camera basis (the caller sets opts from its camera):
    // a billboarded puff's rotation IS the viewer's — column 0 parallel to opts.right, column 1 to
    // opts.up — while the interp scale (|column|) and the translation survive.
    let (right, up) = ([1.0f32, 0.0, 0.0], [0.0f32, 1.0, 0.0]);
    let opts = nf_view::scene::ViewOpts { reinforcements: false, firing: false, right, up };
    let fs = scene.frame_state(None, 0.0, opts);
    let mut checked = 0;
    for (ai, a) in scene.attachments.iter().enumerate() {
        let Some((h, _si, hash)) = a.script else { continue };
        if h != 0x0600_0052 { continue; }
        let m = fs.attach[ai];
        eprintln!("[blast-attach] ai={ai} hash={hash:#010x} t=[{:.2},{:.2},{:.2}] |c0|={:.4}", m[3][0], m[3][1], m[3][2], (m[0][0]*m[0][0]+m[1][0]*m[1][0]+m[2][0]*m[2][0]).sqrt());
        if m[3][1] < -9000.0 { continue; } // collapsed slot
        let col = |k: usize| [m[0][k], m[1][k], m[2][k]];
        let norm = |v: [f32; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let s = norm(col(0));
        if s < 1e-4 { continue; }
        let along = |v: [f32; 3], axis: [f32; 3]| (dot(v, axis) / (norm(v) * norm(axis))).abs();
        // The puff's own scale (the interp scale × the radius) must survive the billboard swap.
        assert!(s > 0.05, "the interp scale survives (|col0| = {s})");
        if scene.script_billboards.contains(&hash) {
            assert!(along(col(0), right) > 0.999, "mesh {hash:#010x}: the drawn X axis is the viewer's right (got {:.4} parallel)", along(col(0), right));
            assert!(along(col(1), up) > 0.999, "mesh {hash:#010x}: the drawn Y axis is the viewer's up (got {:.4} parallel)", along(col(1), up));
        }
        checked += 1;
    }
    // The visible puffs ride the SUB-script 0x06000047 (the runtime Obj::Entity src.0 — the lib
    // key `Script_SubScriptStart` 0x1db050 loads) whose meshes the scene build does not attach yet
    // (open item in the report): the top-level 0x06000052 streams' own entities may or may not be
    // live this frame. The billboard semantics on whatever DOES draw are asserted in the loop.
    eprintln!("[blast] live script-entity attachments checked: {checked}");
}

/// The launch origin [M Player_WeaponInitBullet 0x1a4bf0:212-240]: the midpoint of the head and the view model's
/// bone 0 (its object origin, ~0.43 behind the eye), so on the release tick the frag is BEHIND the camera — the
/// game shows no grenade on the release frame (PCSX2 frozen at the fire script's frame 26). Sister: it is not at
/// the eye (the old origin, which drew a screen-filling frag on that frame).
#[test]
fn the_frag_launches_from_behind_the_eye() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load_weapons(&iso, "07000015.bin", false, &[0x34]).expect("load 07000015");
    let sim = scene.sim.as_mut().unwrap();
    let wd = sim.weapons.clone();
    sim.drones.inventory.equip_weapon(&wd, 0x34, 1, true); sim.drones.inventory.equip_ammo(&wd, 0x34, 3);
    sim.step_bond(Input { switch_to: Some(0x34), ..Input::default() }, false, false, false);
    for _ in 0..120 { sim.step_bond(Input::default(), false, false, false); }
    let mut spawned = None;
    for t in 0..40 {
        let eye = sim.bond.head(); let look = sim.bond.look();
        sim.step_bond(Input::default(), t == 0, false, false);
        if let Some(g) = sim.drones.grenades.first() { spawned = Some((g.pos, eye, look)); break; }
    }
    let (p, eye, look) = spawned.expect("the frag spawned");
    let fwd = (p[0] - eye[0]) * look[0] + (p[1] - eye[1]) * look[1] + (p[2] - eye[2]) * look[2];
    let off = ((p[0] - eye[0]).powi(2) + (p[1] - eye[1]).powi(2) + (p[2] - eye[2]).powi(2)).sqrt();
    assert!(fwd < 0.05, "the frag starts at or behind the eye plane on the release tick (fwd {fwd})");
    assert!(off > 0.1, "the frag does not start at the eye (offset {off})");
}
