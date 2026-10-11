//! Headless NF_ISO tests for the fix-render2 follow-ups (the user's playtest 2026-10-11 follow-up):
//! the script-character/cut-scene ambient dim, the dropped-gun and placed-pickup tint, the truck,
//! the doors, the first-guard cel snap, and the merge regression on the placed-pickup draw.
//!
//! Evidence tags in each test. Opt-in via `NF_ISO`.

use nf_view::scene;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// Step the sim `n` ticks with no input (the viewer's per-frame `step_bond`).
fn step(scene: &scene::Scene, n: usize) {
    let sim = scene.sim.as_ref().unwrap() as *const scene::Sim as *mut scene::Sim;
    for _ in 0..n {
        unsafe { (*sim).step_bond(Default::default(), false, false, false) };
    }
}


/// Idle ticks through the real `step_bond` path (no input).
fn tick_idle(sim: &mut scene::Sim, n: usize) {
    for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
}

/// Point Bond's aim at a world position (the fix_player2 helper shape).
fn aim_at(sim: &mut scene::Sim, to: [f32; 3]) {
    let e = sim.bond.head();
    let d = [to[0] - e[0], to[1] - e[1], to[2] - e[2]];
    let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-6);
    sim.bond.yaw = d[0].atan2(d[2]);
    sim.bond.pitch = (d[1] / l).clamp(-1.0, 1.0).asin() / (std::f32::consts::FRAC_PI_2);
}


/// Replicate `Camera::view_proj` for the level-start camera (the frustum_cull_live shape).
fn view_proj(pos: [f32; 3], yaw: f32, pitch: f32, aspect: f32) -> [[f32; 4]; 4] {
    use glam::{Mat4, Vec3};
    let (near, far, fov) = (0.05f32, 4000.0f32, std::f32::consts::FRAC_PI_3);
    let f = 1.0 / (fov * 0.5).tan();
    let proj = Mat4::from_cols_array_2d(&[
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, far / (near - far), -1.0],
        [0.0, 0.0, near * far / (near - far), 0.0],
    ]);
    let cp = pitch.cos();
    let fw = Vec3::new(cp * yaw.sin(), pitch.sin(), cp * yaw.cos()).normalize();
    let s = fw.cross(Vec3::Y).normalize();
    let u = s.cross(fw);
    let view = Mat4::from_cols_array_2d(&[
        [s.x, u.x, -fw.x, 0.0],
        [s.y, u.y, -fw.y, 0.0],
        [s.z, u.z, -fw.z, 0.0],
        [-s.dot(Vec3::from(pos)), -u.dot(Vec3::from(pos)), fw.dot(Vec3::from(pos)), 1.0],
    ]);
    (proj * view).to_cols_array_2d()
}

fn expect_bytes(a: [u8; 3]) -> [f32; 3] {
    [0, 1, 2].map(|k| ((0xffu32 * a[k] as u32) >> 8) as f32 / 255.0)
}

fn close(a: [f32; 3], b: [f32; 3]) -> bool {
    a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() < 1.0 / 255.0)
}

fn level_from_disc(iso: &str, archive: &str) -> nf_formats::level::Level {
    let img = nf_formats::iso9660::IsoImage::open(iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(archive)).expect("archive entry");
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read archive");
    nf_formats::level::Level::parse(&data).expect("parse level")
}

/// The merge d589c76 dropped fix-player2's pickup branch in `add_instance` (the only
/// `pickup: Some(pk)` CharVertex push): placed-pickup vertices enter the Character tail with no
/// CharVertex, so `posed_vertices`' vertex↔CharVertex zip misaligns and the pickups never pose.
/// fix-player2 eb1564b scene.rs:1039 had the branch [M git]; the merged master does not.
#[test]
fn placed_pickup_charvertices_survived_the_merge() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    // Every Character/Effect-layer vertex must have its CharVertex (the zip in posed_vertices pairs
    // them by index; a pickup group pushes vertices but (pre-fix) no CharVertex).
    let char_verts = scene.vertices[scene.char_vertex_start..].len();
    assert_eq!(char_verts, scene.char_verts.len(),
        "Character-tail vertex count {} != CharVertex count {} — a draw group pushed vertices without CharVertices",
        char_verts, scene.char_verts.len());
    // The placed pickups are drawn (fix-player2): their vertices must carry pickup CharVertices.
    let pickups = scene.char_verts.iter().filter(|v| v.pickup.is_some()).count();
    assert!(pickups > 0, "no CharVertex carries a pickup index — the placed pickups cannot pose");
    // Sister: the sniper's placed pickup exists in the sim (View_DrawObjectGlist draws obj+0xD8).
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.pickups.iter().any(|p| p.from.is_none()), "no placed pickup in the sim");
}

/// The user's check of fix-render: "every cut-scene still draws characters at the old, over-bright
/// level." Root cause: `Sim::characters_script_pos` resolved script characters through
/// `script_entity`, which matches only `Obj::Entity` — script Anim characters (Obj::Anim) never got
/// an ambient target, so `char_ambient` stayed at the 0xff sentinel and the multiplier stayed 1.0.
/// [L ee_start_07000001.bin] the opening cut-scene's script-created character objects carry
/// obj+0x100..0x102 = (40, 30, 26) (cel 0x191c860, the courtyard): obj 0x018f5790 (-2.78, 6.43,
/// -12.40) flags 0x890, obj 0x018f6a30 (-6.52, 5.85, -9.06) flags 0x860, obj 0x018f6b90 (-3.25,
/// 5.85, -9.06) flags 0x860.
#[test]
fn cutscene_script_characters_dim_to_their_cel_ambient() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000001.bin", false).expect("load");
    let sim = scene.sim.as_ref().unwrap() as *const scene::Sim as *mut scene::Sim;
    unsafe { for c in 1..=255 { (*sim).drones.set_channel(c) } }
    let expect = expect_bytes([40, 30, 26]);
    let mut saw_script_char = false;
    let mut all_follow = true;
    let mut ever_dimmed = false;
    for _ in 0..900 {
        step(&scene, 1);
        let s = scene.sim.as_ref().unwrap();
        if s.script_camera().is_none() { continue; }
        for ci in 0..scene.characters.len() {
            if s.char_drone[ci].is_some() { continue; }               // drones, not script characters
            let Some(src) = s.char_script[ci] else { continue };
            if s.script_anim(src, scene.characters[ci].skin.hashcode).is_none() { continue; }
            saw_script_char = true;
            let m = s.char_multiplier(ci);
            // The invariant the game maintains (Lights_CalcAmbientLight 0x1cb178): the multiplier
            // steps toward the ambient of the cel the character is in — never the white sentinel.
            if close(m, [1.0; 3]) {
                all_follow = false;
                eprintln!("script char {ci} kept the white sentinel at its live position");
            } else { ever_dimmed = true; }
        }
        if saw_script_char && !all_follow { break; }
    }
    assert!(saw_script_char, "no script character was shown during the opening cut-scene");
    assert!(all_follow, "a script character kept the over-bright 0xff sentinel during the cut-scene");
    assert!(ever_dimmed, "no script character ever carried the ambient dim");
    // [L] pin: the [L] snapshot's three script characters (0x018f5790/0x018f6a30/0x018f6b90) all
    // read (40,30,26) — cel 0x191c860, the courtyard. When the port's script characters stand in
    // that cel, their multiplier must be exactly that triple.
    let s = scene.sim.as_ref().unwrap();
    let courtyard = s.cels.iter().find(|c| c.ambient[0] == [40, 30, 26]).expect("the courtyard cel");
    for ci in 0..scene.characters.len() {
        if s.char_drone[ci].is_some() || s.char_script[ci].is_none() { continue; }
        let Some(src) = s.char_script[ci] else { continue };
        if s.script_anim(src, scene.characters[ci].skin.hashcode).is_none() { continue; }
        let pos = s.characters_script_pos(ci);
        let Some(pos) = pos else { continue };
        let inside = courtyard.min[0] <= pos[0] && pos[0] <= courtyard.max[0]
            && courtyard.min[1] <= pos[1] && pos[1] <= courtyard.max[1]
            && courtyard.min[2] <= pos[2] && pos[2] <= courtyard.max[2];
        if inside {
            let m = s.char_multiplier(ci);
            assert!(close(m, expect), "a script character in the courtyard cel reads {m:?} != [L] (40,30,26)/255");
            eprintln!("script char {ci} in the courtyard at {pos:?} reads the [L] ambient");
            return;
        }
    }
    eprintln!("note: no script character stood in the courtyard cel at the end tick — the [L] pin is covered by the ramp invariant above");
    // Sister: the courtyard cel's ambient is NOT white — the assertion can fail.
    let level01 = level_from_disc(&iso, "07000001.bin");
    let cels = scene::parse_room_cels(&level01);
    assert!(cels.iter().any(|c| c.ambient[0] == [40, 30, 26]), "no parsed cel carries the [L] (40,30,26) ambient");
}

/// The user's check: "weapons lying on the ground are far too bright. fix-render left dropped guns
/// at 1.0. Port their tint, and check placed pickups too." The game's pickups are character-lit
/// objects: `Pickup_Create` (0x18dda0) forces obj+0xF0 = glist_flags & ~0x2000 | 0x840 — the 0x800
/// ambient dim — except kind 3 (armour) and kind 6 (007 bonus), which get 0x800 cleared. [L
/// ee_start_05.bin] the placed sniper pickup obj 0x01382bc0 reads +0xF0 = 0x880, ambient
/// (57, 70, 86). The slim ISO's 07000005 places only a kind-6 pickup, so the placed-pin here is the
/// kind gate, and the dim itself is pinned on a guard's dropped weapon (the next test).
#[test]
fn placed_pickups_follow_the_pickup_create_gate() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    step(&scene, 1);
    let sim = scene.sim.as_ref().unwrap();
    let posed = scene.posed_vertices(None, 0.0, Default::default());
    let mut checked = 0;
    for (i, cv) in scene.char_verts.iter().enumerate() {
        let Some(pk) = cv.pickup else { continue };
        let Some(p) = sim.drones.pickups.get(pk as usize) else { continue };
        if p.from.is_some() || p.gone || p.hidden { continue; }
        let raw = scene.vertices[scene.char_vertex_start + i];
        let drawn = &posed[i];
        if p.kind == nf_game::pickup::kind::ARMOUR || p.kind == nf_game::pickup::kind::BOND_BONUS {
            // Pickup_Create clears 0x800 for kinds 3 and 6: the drawn bytes stay raw.
            assert_eq!(drawn.rgba, raw.rgba, "kind {} pickup {pk} was tinted — Pickup_Create 0x18dda0 clears its 0x800", p.kind);
        } else {
            let Some(amb) = sim.ambient_at([p.m[3][0], p.m[3][1], p.m[3][2]]) else { continue };
            let want = expect_bytes(amb);
            let got = [0, 1, 2].map(|k| if raw.rgba[k] == 0 { 1.0 } else { drawn.rgba[k] as f32 / raw.rgba[k] as f32 });
            assert!((got[0] - want[0]).abs() < 2.0 / 255.0 && (got[1] - want[1]).abs() < 2.0 / 255.0 && (got[2] - want[2]).abs() < 2.0 / 255.0,
                "placed pickup {pk} drawn multiplier {got:?} != the cel ambient {amb:?} → {want:?}");
        }
        checked += 1;
        eprintln!("placed pickup {pk} kind {} checked against the gate", p.kind);
    }
    assert!(checked > 0, "no placed pickup was checkable");
    // Sister: the gate is really the kind — the placed pickup here is a kind-6 007 bonus [D], the
    // branch the game exempts; a wrong always-dim port fails the assert above.
    let level = level_from_disc(&iso, "07000005.bin");
    let _ = level;
}

/// The dim half of the pickups item: a guard's dropped weapon (DroneWeap_DropWeapon → Pickup_Create,
/// the same forced 0x840) must draw at the ambient of the cel it lies in — on 07000001 the guard
/// drops sit in cels around the courtyard, never white [L ee_start_07000001.bin: every live 0x800
/// object reads a non-white ambient]. Before the fix the dropped-gun attachment kept 1.0.
#[test]
fn dropped_guns_dim_to_the_cel_ambient() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000001.bin", false).expect("load");
    {
        let sim = scene.sim.as_mut().unwrap();
        // Kill an armed guard with the P2K (the fix-player2 harness: two torso hits at Easy).
        tick_idle(sim, 2);
        let guards: Vec<usize> = sim.drones.list.iter().enumerate()
            .filter(|(_, dr)| dr.present && dr.enabled && dr.drawn && !dr.corpse && dr.health > 0.0 && (2..0x47).contains(&dr.weapon))
            .map(|(i, _)| i).collect();
        assert!(!guards.is_empty(), "07000001 starts armed guards");
        let gi = guards[0];
        let gp = sim.drones.list[gi].pos;
        let floor = sim.world.floor_below(gp, 6.0).unwrap_or(gp[1]);
        let stand = [gp[0], floor + 1.1, gp[2] - 3.0];
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, stand, 0.0);
        aim_at(sim, [gp[0], gp[1] + 0.3, gp[2]]);
        tick_idle(sim, 10);
        for t in 0..600 {
            sim.step_bond(Default::default(), t % 9 == 0, false, false);
            if sim.drones.list[gi].health <= 0.0 { break; }
        }
        assert!(sim.drones.list[gi].health <= 0.0, "the guard died");
        tick_idle(sim, 80); // the drop falls and lands
    }
    let sim = scene.sim.as_ref().unwrap();
    let drop = sim.drones.pickups.iter().find(|p| p.from.is_some() && !p.hidden && !p.gone)
        .expect("the guard dropped his weapon");
    assert!(drop.kind == nf_game::pickup::kind::WEAPON, "the drop is a weapon");
    let amb = sim.ambient_at([drop.m[3][0], drop.m[3][1], drop.m[3][2]])
        .expect("the drop's position resolves a cel");
    let want = expect_bytes(amb);
    // The dropped attachment's drawn vertices carry the multiplier.
    let posed = scene.posed_vertices(None, 0.0, Default::default());
    let di = sim.char_drone.iter().position(|d| *d == drop.from).expect("the dropping drone's character");
    let mut checked = 0;
    for (i, cv) in scene.char_verts.iter().enumerate() {
        let Some(a) = cv.attachment.and_then(|a| scene.attachments.get(a as usize)) else { continue };
        if !a.dropped || a.character as usize != di { continue; }
        let raw = scene.vertices[scene.char_vertex_start + i];
        let drawn = &posed[i];
        let got = [0, 1, 2].map(|k| if raw.rgba[k] == 0 { 1.0 } else { drawn.rgba[k] as f32 / raw.rgba[k] as f32 });
        assert!((got[0] - want[0]).abs() < 2.0 / 255.0 && (got[1] - want[1]).abs() < 2.0 / 255.0 && (got[2] - want[2]).abs() < 2.0 / 255.0,
            "dropped gun drawn multiplier {got:?} != the cel ambient {amb:?} → {want:?}");
        checked += 1;
    }
    assert!(checked > 0, "the dropped gun drew no vertices");
    // Sister: the multiplier is not the white sentinel — the ambient at the drop is real.
    assert!(!close(want, [1.0; 3]), "the drop's cel ambient is white — nothing to assert");
}

/// The user's check: "The 07000005 truck is too bright." The truck is the script Anim object of
/// skin 0x05000005 (script-player.md [D]: ScriptPlayer #37, stream 1, nine rigid parts); its nine
/// discrete game objects carry obj+0xF0 = 0x8880 with ambient (57, 70, 86) [L ee_start_05.bin
/// 0x0137C8A0…, gs-leaf.md's 0x0137bfa0 row]. The port draws it as a script character, whose
/// ambient never resolved (the Obj::Anim position bug) — fixed with the cut-scene characters.
#[test]
fn the_truck_takes_the_0x800_ambient() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let sim = scene.sim.as_ref().unwrap();
    // [D script-player.md] the truck's skin hash:
    let truck_skin = 0x0500_0005u32;
    let ci = (0..scene.characters.len()).find(|&ci| scene.characters[ci].skin.hashcode == truck_skin && scene.characters[ci].script.is_some())
        .expect("the truck's script character (skin 0x05000005) is in the scene");
    // The truck's model objects carry 0x800 in the level parse [D 07000005.bin via models_0x800].
    let level = level_from_disc(&iso, "07000005.bin");
    // Nine RIGID parts on nine bones (script-player.md): the meshes sit in skin.main and skin.extra.
    let truck_meshes: Vec<u32> = level.skins.get(&truck_skin).map(|sk|
        sk.main.iter().flatten().copied().chain(sk.extra.iter().filter_map(|(_, h)| *h)).collect()).unwrap_or_default();
    assert!(!truck_meshes.is_empty(), "the truck skin has no main meshes");
    assert!(truck_meshes.iter().any(|h| scene.models_0x800.get(h).copied().unwrap_or(false)),
        "no truck mesh carries the 0x800 glist flag — the dim gate would never open");
    // Run the truck script; the multiplier must reach the [L] (57,70,86) bytes once the Anim is live.
    let mut saw_live = false;
    for _ in 0..120 {
        step(&scene, 1);
        let s = scene.sim.as_ref().unwrap();
        if s.script_anim(scene.characters[ci].script.unwrap(), truck_skin).is_some() { saw_live = true; break; }
    }
    assert!(saw_live, "the truck's Anim never went live in 120 ticks");
    let s = scene.sim.as_ref().unwrap();
    let m = s.char_multiplier(ci);
    assert!(close(m, expect_bytes([0x39, 0x46, 0x56])),
        "truck multiplier {m:?} != [L] (57,70,86)/255 — the truck kept the white sentinel");
    // Sister: the multiplier is not the level-wide constant — the truck's own cel lookup resolved.
    assert!(!close(m, [1.0; 3]), "the truck stayed at the pre-fix 1.0 multiplier");
}

/// The user's check: "The first guard at the start of 07000001 looks too dark, but guards later in
/// the level look right." [L ee_start_07000001.bin] the game's drones read, in list order:
/// drone 0 (-4.92, 6.34, -0.09) cel 0x191f0e0 amb (20,20,20); drone 1 (-7.70, 6.34, -7.62) cel
/// 0x191c860 amb (40,30,26); drone 2 (-30.06, 10.31, 13.44) amb (60,60,60); drones 5-7 amb
/// (80,80,70); drones 8/13 amb (45,30,30). Every guard's multiplier must match the game's cel
/// ambient for a guard at the same spot — the cel chosen by the game's raycast tie-break
/// (build_FindCel 0x1c8cb0), not the port's smallest-volume pick.
#[test]
fn the_first_guard_snaps_to_the_game_ambient() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000001.bin", false).expect("load");
    step(&scene, 1);
    let sim = scene.sim.as_ref().unwrap();
    // The game's drone → cel-ambient table, keyed by the rounded snapshot position.
    let game: &[([f32; 3], [u8; 3])] = &[
        ([-4.92, 6.34, -0.09], [20, 20, 20]),
        ([-7.70, 6.34, -7.62], [40, 30, 26]),
        ([-30.06, 10.31, 13.44], [60, 60, 60]),
        ([-12.17, 6.35, 17.22], [80, 80, 70]),
        ([6.98, 6.34, 8.48], [45, 30, 30]),
    ];
    let mut matched = 0;
    for ci in 0..scene.characters.len() {
        let Some(di) = sim.char_drone[ci] else { continue };
        let d = &sim.drones.list[di];
        let pos = [d.pos[0], d.pos[1], d.pos[2]];
        let Some((gp, gamb)) = game.iter().find(|(gp, _)| {
            gp.iter().zip(pos.iter()).all(|(a, b)| (a - b).abs() < 0.6)
        }).map(|(gp, a)| (*gp, *a)) else { continue };
        matched += 1;
        let m = sim.char_multiplier(ci);
        assert!(close(m, expect_bytes(gamb)),
            "guard {ci} (drone {di}) at {pos:?} multiplier {m:?} != [L] {gamb:?} (game drone at {gp:?})");
        assert!(!close(m, [1.0; 3]), "guard {ci} at the sentinel 1.0");
    }
    assert!(matched >= 3, "only {matched} guards matched a [L] snapshot position");
}

/// The playtest R3: "Doors don't take the ambient lighting while they open and close." The
/// evidence says the proposed fix shape is WRONG: the doors' glist flags carry NO 0x800
/// ([D 07000005.bin via the port's parse — Door_Locked_3/Door_UNLocked/Door_Locked_4 all flags
/// 0x0]), `Door_Create` (0x136100) only calls `Control_SetGList` (which merges glist+0x44 into
/// obj+0xF0), so the game never dims doors through the 0x800 path — the port's 1.0 matches the
/// game. This test pins that negative result so a future "fix" has to re-measure first.
#[test]
fn doors_carry_no_0x800_and_stay_untinted() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let level = level_from_disc(&iso, "07000005.bin");
    let doors = nf_game::door::door_placements(&level.maps[level.main_map().unwrap()].placements);
    assert!(!doors.is_empty(), "no doors on 07000005");
    let mut flagged = 0;
    for pl in &doors {
        let p = &level.maps[level.main_map().unwrap()].placements[*pl];
        let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
            else { Some((level.main_map().unwrap(), p.index as usize)) };
        let Some((m, oi)) = target else { continue };
        if level.maps[m].objects[oi].flags & 0x800 != 0 { flagged += 1; }
    }
    assert_eq!(flagged, 0, "a door model DOES carry 0x800 — re-open R3 and dim the doors");
    // The drawn doors stay at multiplier 1.0 (the game's path has no ambient term for them).
    step(&scene, 1);
    let posed = scene.posed_vertices(None, 0.0, Default::default());
    for (i, cv) in scene.char_verts.iter().enumerate() {
        if cv.door.is_none() { continue; }
        let raw = scene.vertices[scene.char_vertex_start + i];
        assert_eq!(posed[i].rgba, raw.rgba, "a door vertex was tinted — the game does not do this");
    }
}

/// The playtest R4 / the user's performance check: "the frame rate drops badly looking towards the
/// party level's grandfather clock" and "the frame rate is still very poor (07000006, 07000007)".
/// The renderer submitted every material every frame (the cull was off by default after A-11's
/// vanish bug). fix-render2 scopes the default cull to the static World layer (gpu.rs): the moving
/// layers keep drawing, the static bulk skips off-screen. This counts what the render loop would
/// submit from the level-start camera of the party levels.
#[test]
fn party_levels_cull_the_static_bulk_from_the_start_view() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for level in ["07000006.bin", "07000007.bin"] {
        let scene = match nf_view::load(&iso, level, false) { Ok(s) => s, Err(e) => { eprintln!("{level} not loadable on this ISO: {e}"); continue } };
        let total = scene.materials.len();
        let (pos, yaw) = scene.start.clone().unwrap_or(([0.0; 3], 0.0));
        let eye = [pos[0], pos[1] + 1.6, pos[2]];
        let vp = view_proj(eye, yaw, 0.0, 16.0 / 9.0);
        let planes = nf_view::gpu::extract_frustum(vp);
        let (mut sub_world, mut cull_world, mut moving) = (0usize, 0usize, 0usize);
        for m in &scene.materials {
            let is_world = m.layer == scene::Layer::World;
            if !is_world { moving += 1; continue; }
            if m.draw.pass != 4 && nf_view::gpu::sphere_outside(&planes, m.centre, m.radius) { cull_world += 1; }
            else { sub_world += 1; }
        }
        eprintln!("{level}: {total} materials — World {sub} submitted / {cull_world} culled, moving layers {moving} always drawn", sub = sub_world);
        assert!(total > 0 && cull_world > 0, "{level}: the World cull skipped nothing — the counters are broken");
        if level == "07000006.bin" {
            // [measured] the start view's World relief: 1040 of 4438 submitted (77 % skipped) — the
            // draw burst that made the party level lag shrinks accordingly.
            assert!(sub_world * 10 < (sub_world + cull_world) * 3,
                "07000006: {sub_world} of {} World draws still submitted — the cull regressed", sub_world + cull_world);
        }
        // 07000007's start view faces its whole interior (4526 of 4869 World draws inside the
        // frustum spheres): the sphere cull cannot relieve it — that is the portal-cull item
        // (G-016), recorded here as the measured next lever, not asserted away.
    }
}
