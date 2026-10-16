//! fix-foliage-bullets: the playtest's "those trees are, for some reason, bulletproof, like the
//! leaves" (07000005, the small trees near the start). What the game's data and code actually say:
//!
//! - The foliage pass machinery is real [M]: `Collide_Intersect` 0x1ebaa8 ORs `HITDATA+0x51` bit 1
//!   from the glist node's flags bit 0x40 (cel+0x94 = the placement's `celglist+0x44` [M
//!   parsemap_block_map_data_static 0x1d1998:103]) or the surface byte's bit 0x40 (disasm
//!   0x1ebf78-0x1ebf80), `Collide_Update` 0x1ea398:216-229 walks the tracer past pass-flagged
//!   hits, and the bit-0x8 queries (HITTEST+0x98) drop them outright (disasm 0x1ebff0-0x1ec010).
//!   The bit-0x8 family is the AIM/LOS traces: the player's aim trace 0x20a/0x208
//!   (`Player_WeaponInitBullet` 0x1a4bf0:154-159) and `Bullet_homing`'s LOS 8 (0x12f358:92).
//! - The 07000005 tree objects (map 50, obj 96/98) carry glist flags 0x41 [D] — node-flagged —
//!   and the leaf cards (obj 21/109/117/144, flags 0x10) carry the 0x40 surface byte [D]. The
//!   separate wood core obj 97 (30 tris, mat 0x04, flags 0) is NOT flagged [D].
//! - The bullets' FLIGHT, though, never runs that walk: a bullet's HITTEST stays inactive
//!   (+0x94 = 0 [M 0x12c668 disasm]), so `Collide_Update`'s per-object walk never processes it,
//!   and the live-pinned guard cover trace (`ai_live::guard_takes_cover_like_the_game`,
//!   UnderCoverAim 37 ticks) pins the round stopping on the flagged tree's FIRST face — the pass
//!   does not lift the flight stop. The port's first-hit flight already matches.
//!
//! So the round stops on the leaf shell (surface byte 0x13, class 19: the EffectInfo record has
//! NO decal and NO gas — only debris chips [D EffectInfo class 19]), which reads as "bulletproof"
//! next to every wall that shows a bullet hole. These tests pin that trace behaviour and the
//! aim-trace family's drop, so the distinction survives.
//!
//! Run with: `NF_ISO=… cargo test -p nf-view --test fix_foliage_bullets_live`

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The tree cluster east of the spawn: inst 80 of obj 96's bbox (the port's level build).
const TREE_LO: [f32; 3] = [89.43, 23.61, 88.77];
const TREE_HI: [f32; 3] = [93.52, 30.72, 92.87];

fn in_tree_span_x(x: f32) -> bool { x >= TREE_LO[0] - 0.2 && x <= TREE_HI[0] + 0.2 }

/// The flight stops on the flagged tree's first face (the 0x13 shell) — the same surface the flat
/// trace reports — while the aim/LOS family's trace (bit 0x8 [M 0x1ebff0-0x1ec010]) reports the
/// unflagged wood core (obj 97, 0x04 [D]) behind the skipped shell. The fired round's impact lands
/// on the shell, with the shell's class-19 record: no decal, no gas puff [D EffectInfo] — the
/// playtest's "bulletproof" look.
#[test]
fn flight_stops_on_the_flagged_shell_while_the_aim_trace_skips_it() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Default::default(), false, false, false);
        // Stand 6 units east of the tree, aim at its crown line (the playtest's shot).
        sim.bond.pos = [97.0, 24.30, 90.80];
        let eye = [sim.bond.pos[0], sim.bond.pos[1] + 0.70, sim.bond.pos[2]];
        let target = [91.0f32, 25.5f32, 90.8f32];
        let d = [target[0] - eye[0], target[1] - eye[1], target[2] - eye[2]];
        let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        let dir = [d[0] / l, d[1] / l, d[2] / l];
        sim.bond.yaw = dir[0].atan2(dir[2]);
        sim.bond.pitch = dir[1].clamp(-1.0, 1.0).asin() * 2.0 / std::f32::consts::PI;
        let w = &sim.world;
        let far = [eye[0] + dir[0] * 60.0, eye[1] + dir[1] * 60.0, eye[2] + dir[2] * 60.0];
        // The flight/flat trace: the first hit is the shell's 0x13 face inside the tree span.
        let (tf, _, mf) = w.segment_surface(eye, far, false).expect("the flight trace stops at the shell");
        let pf = [eye[0] + (far[0] - eye[0]) * tf, eye[1] + (far[1] - eye[1]) * tf, eye[2] + (far[2] - eye[2]) * tf];
        assert!(in_tree_span_x(pf[0]), "flat: the first hit is inside the tree span ({pf:?})");
        assert_eq!(mf & 0x3f, 0x13, "flat: the hit surface byte is the shell's 0x13, got 0x{mf:02x}");
        // The aim-trace family (bit 0x8): the shell drops, the first report is the wood core.
        let (tb, _, mb) = w.segment_surface_bullet(eye, far).expect("the aim trace reaches the core");
        let pb = [eye[0] + (far[0] - eye[0]) * tb, eye[1] + (far[1] - eye[1]) * tb, eye[2] + (far[2] - eye[2]) * tb];
        assert!(in_tree_span_x(pb[0]), "the core sits inside the tree span ({pb:?})");
        assert_eq!(mb & 0x3f, 0x04, "aim: the reported class is the unflagged core's 0x04 [D obj 97], got 0x{:02x}", mb & 0x3f);
        assert!(pb[0] < pf[0], "the core is deeper along the ray than the dropped shell (core {} < shell {})", pb[0], pf[0]);
        eprintln!("[pre] flight/flat stop 0x{mf:02x} at {pf:?}; aim-trace stop 0x{mb:02x} at {pb:?}");
        // Fire one P2K round through the real input path.
        sim.gun.reload_state = 0;
        sim.gun.fire_state = false;
        sim.gun.clip = 12;
        sim.gun.pending = 0;
    }
    let mut input = Input::default();
    input.fire = true;
    let mut impacts: Vec<[f32; 3]> = Vec::new();
    for t in 0..40u32 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(if t == 0 { input } else { Input::default() }, t == 0, false, false);
        let sim = scene.sim.as_mut().unwrap();
        for im in &sim.drones.player_impacts {
            if let nf_game::drone::PlayerImpact::World(p) = im { impacts.push(*p); }
        }
        if sim.drones.player_bullets.is_empty() && t > 0 { break; }
    }
    assert!(!impacts.is_empty(), "the round records a world impact (on the tree)");
    // The impact lands ON the tree (the spread-perturbed ray meets one of its faces) and never
    // reaches the terrain behind (the 0x0a wall at x≈61.8 the aim trace reports).
    let sim = scene.sim.as_ref().unwrap();
    for p in &impacts {
        assert!(in_tree_span_x(p[0]), "the impact is the tree stop, inside the tree span ({p:?})");
        let (_, _, m) = sim.world.segment_surface([p[0] + 1.5, p[1], p[2]], [p[0] - 1.5, p[1], p[2]], false)
            .expect("the impact surface is re-queryable");
        assert!(matches!(m & 0x3f, 0x13 | 0x03 | 0x04), "the impact's surface is the tree's 0x13/0x03/0x04, got 0x{m:02x}");
    }
    eprintln!("[fire] world impacts on the tree: {impacts:?}");
}

/// Sister (the guard side): a drone-style `Bullet` flying the same ray dies on the shell (the
/// flight keeps the first hit), and the flat sight trace stops on the same face (the guards'
/// sight: flags 0x721, no bit 0x8 [M 0x18e148:214] — the foliage stays solid to sight).
#[test]
fn guard_style_bullets_and_sight_stop_on_the_shell() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let (eye, dir) = {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Default::default(), false, false, false);
        sim.bond.pos = [97.0, 24.30, 90.80];
        let eye = [sim.bond.pos[0], sim.bond.pos[1] + 0.70, sim.bond.pos[2]];
        let target = [91.0f32, 25.5f32, 90.8f32];
        let d = [target[0] - eye[0], target[1] - eye[1], target[2] - eye[2]];
        let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        let dir = [d[0] / l, d[1] / l, d[2] / l];
        (eye, dir)
    };
    let sim = scene.sim.as_ref().unwrap();
    let vol = nf_game::bullet::PlayerVolume { feet: [1e4, 0.0, 1e4], height: 0.1, radius: 0.1 };
    // A guard's rifle round: no fly-by, range past the tree.
    let mut b = nf_game::bullet::Bullet { pos: eye, dir, speed: 8.0, travelled: 0.0, range: 120.0, damage: 1.0, owner: 1, state: 0, weapon: 0x16, flyby: false, bounces: 0 };
    let mut rng = nf_game::drone::Rng::new(7);
    let mut died_at: Option<[f32; 3]> = None;
    for _ in 0..60 {
        match b.update(&sim.world, &vol, [1e4, 0.0, 1e4], &mut rng) {
            Some(nf_game::bullet::BulletEvent::World(p)) => { died_at = Some(p); break; }
            _ => {}
        }
        if b.state >= 2 { break; }
    }
    let dead = died_at.unwrap_or_else(|| { assert!(b.state >= 2 || b.travelled >= b.range, "the round ends this tick"); b.pos });
    assert!(in_tree_span_x(dead[0]), "the guard round stops on the shell, inside the tree span ({dead:?})");
    assert!(dead[0] >= 92.0, "the guard round stops on the FIRST face (x >= 92): {dead:?}");
    // Sight: the flat trace stops on the same shell face.
    let far = [eye[0] + dir[0] * 60.0, eye[1] + dir[1] * 60.0, eye[2] + dir[2] * 60.0];
    let t = sim.world.segment(eye, far).expect("sight still stops at the tree");
    let p = [eye[0] + (far[0] - eye[0]) * t, eye[1] + (far[1] - eye[1]) * t, eye[2] + (far[2] - eye[2]) * t];
    assert!(in_tree_span_x(p[0]) && p[0] >= 92.0, "sight's first hit is the shell's front face ({p:?})");
    let (_, _, m) = sim.world.segment_surface(eye, far, false).expect("sight surface re-query");
    assert_eq!(m & 0x3f, 0x13, "sight stops on the shell's 0x13 surface, got 0x{m:02x}");
}

/// Sister (the leaf cards): the collision-only leaf planes by the spawn (obj 117, glist flags 0x10,
/// surface byte 0x40 [D]) block the flat trace and vanish from the aim/LOS-family trace — the
/// material's pass bit (Collide_Intersect 0x1ebaa8 disasm 0x1ebf78-0x1ebf80) with the bit-0x8 drop.
#[test]
fn the_0x40_leaf_cards_leave_the_aim_trace_only() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let sim = scene.sim.as_mut().unwrap();
    sim.step_bond(Default::default(), false, false, false);
    // The card plane spans (101.4-114.5, 24.75-25.75, 93.4-114.9) [D inst 125 obj 117]; the ray
    // crosses it at (111.89, 25.25, 101.89) with nothing non-pass behind on this line.
    let from = [106.0f32, 25.25f32, 96.0f32];
    let to = [112.0f32, 25.25f32, 102.0f32];
    let (t, _, m) = sim.world.segment_surface(from, to, false).expect("the flat trace stops on the leaf card");
    assert!(m & 0x40 != 0 && m & 0x3f == 0, "the leaf card's surface byte carries the pass bit 0x40 over class 0 (got 0x{m:02x})");
    let p = [from[0] + (to[0] - from[0]) * t, from[1] + (to[1] - from[1]) * t, from[2] + (to[2] - from[2]) * t];
    assert!(p[0] >= 101.3 && p[0] <= 114.6 && p[2] >= 93.3 && p[2] <= 115.0, "the flat hit is on the card plane ({p:?})");
    assert!(sim.world.segment_surface_bullet(from, to).is_none(), "the aim/LOS trace drops the pass-flagged card entirely");
}
