//! hatch-02-intro (task 715): "the hatch still doesn't open" (playtest 1014-1725, [intro02]).
//!
//! What the disc says — every claim measured here through the real loaders:
//! - 07000002's level-start NIS (`SP` placement #400, mode 1, script 0x06000084 of pack
//!   07800002.bin) animates **no hatch**. Its streams: the camera (s0), two anim actors (s1
//!   skin 0x0500004f Mayhew / s2 skin 0x05000009), the player/drones disable-enable event pair
//!   (s3/s4), and ONE animated entity — the 8 cm `OBJECT_DrogonPearl` prop (s8, entity 0x19,
//!   1500 keys rolling through the secret bunker). The only channel write is event 0xD
//!   [0x47, 1] at tick 1539 — the objective announce gate ("Destroy Mayhew's computer" and
//!   "Rescue geisha girls" both gate their announce on en 0x47, MissionData).
//! - The level's hatches are the four 0xE2 SimpleScripts (grates-02): #440 `grill1` is the
//!   bunker-bed grate above the bed in the room Bond starts in. The game opens it with the use
//!   key (`SS_Activate` 0x1a9ea8, mask 4, ch word 0) — no script, no channel, no cut-scene
//!   drives it; the ported state machine (`nf_game::ss`) sweeps it open in 150 ticks and holds
//!   the open pose (the wrap disarms the one-shot without re-posing).
//! - The second cut-scene 0x0600007f (the polySurface2 hatch panel, skin 0x05000034, 73 bones)
//!   is dormant on the disc: its chain Touch #495 (out 0x3c) → MultiplexSIn #496 (out 0x35,
//!   needs ch 0x3b && 0x3c sequential) → SP #399 waits on ch 0x35, and ch 0x3b is
//!   MultiplexIn #397's AND of ch 0x29/0x2a/0x88/0x49 — no placement on any of the 52 maps
//!   writes any of those channels (scan), so ch 0x35 can never rise.
//!
//! So current master already behaves like the game (grates-02, merged): E opens the bed grate
//! and it holds the open pose; the playtest predates that merge. These tests PIN the behaviour
//! — they fail on the pre-grates-02 channel-seek tick (a ch-0 grate's playhead never moved, so
//! the matrix stayed at key 0 forever) — plus the intro-side premises above.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The intro plays to its end through the real path; the only thing it animates is the pearl
/// prop; no hatch object ever appears; the four SS grates never move during it; the ch-0x47
/// objective gate rises exactly at the end (event 0xD, tick 1539).
#[test]
fn the_intro_animates_no_hatch_and_ends_on_the_objective_gate() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000002.bin", false).expect("load 07000002");
    let ss_idx: Vec<usize> = [272usize, 308, 309, 440].iter()
        .map(|pl| scene.sim.as_ref().unwrap().drones.sss.of_placement(*pl).expect("the four grates exist [D]"))
        .collect();
    let closed: Vec<nf_game::ss::Mat4> = ss_idx.iter()
        .map(|&i| scene.sim.as_ref().unwrap().drones.sss.list[i].matrix).collect();
    let mut pearl_mid: Option<[f32; 3]> = None;
    for t in 0..1560u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if t == 100 {
            // every object the intro created: the camera, the two anim actors, the pearl — nothing else
            let objs: Vec<String> = sim.drones.scripts.objects.list.iter().flatten().map(|o| match o {
                nf_game::nis::Obj::Anim { skin, .. } => format!("anim {skin:08x}"),
                nf_game::nis::Obj::Entity { hash, .. } => format!("entity {hash:08x}"),
                nf_game::nis::Obj::Camera { .. } => "camera".into(),
                other => format!("{other:?}"),
            }).collect();
            assert_eq!(objs.len(), 4, "the intro created unexpected objects: {objs:?}");
            assert!(objs.contains(&"anim 0500004f".into()) && objs.contains(&"anim 05000009".into())
                && objs.contains(&"entity 020005db".into()) && objs.contains(&"camera".into()),
                "the intro's object set is not the game's (no hatch among them) [D]: {objs:?}");
            pearl_mid = sim.drones.scripts.objects.list.iter().flatten().find_map(|o| match o {
                nf_game::nis::Obj::Entity { matrix, src: (h, s), .. } if *h == 0x0600_0084 && *s == 8 => {
                    Some([matrix[3][0], matrix[3][1], matrix[3][2]])
                }
                _ => None,
            });
        }
        if t == 1500 {
            assert!(!sim.drones.channels.on(0x47), "ch 0x47 rose before the intro's final event");
        }
        // no grate ever moves during the intro (the script has no hatch stream [D])
        if t % 300 == 0 {
            for (&i, c) in ss_idx.iter().zip(&closed) {
                assert_eq!(sim.drones.sss.list[i].matrix, *c, "SS (idx {i}) moved during the intro");
            }
        }
    }
    let sim = scene.sim.as_ref().unwrap();
    let player = sim.drones.scripts.players.iter().find(|p| p.scripts.contains(&0x0600_0084)).expect("the intro SP");
    assert_eq!(player.state, 5, "the intro (0x06000084) never ran to its end (state 5)");
    assert!(sim.drones.channels.on(0x47), "the intro's final event 0xD [0x47, 1] (tick 1539) never landed");
    // the pearl tracked its key path: the final entity position ≈ key 1719 (the last key)
    let nis = sim.drones.scripts.lib.get(&0x0600_0084).unwrap();
    let last = nis.keys[1719].clone();
    let mid = pearl_mid.expect("the pearl entity was alive mid-intro");
    let moved = ((mid[0] - last.pos[0]).powi(2) + (mid[1] - last.pos[1]).powi(2) + (mid[2] - last.pos[2]).powi(2)).sqrt();
    assert!(moved > 0.5, "the pearl never left its first key (moved {moved:.3}) — the entity stream is dead");
}

/// The game's own answer to the playtest: from the bunker (the intro area), Bond walks to the
/// bed under `grill1` #440 and presses E — the hatch's transform changes by the animation's
/// delta (the last key's pose) and holds. Sister: before the key it is closed (key 0).
/// Fails on the pre-grates-02 channel-seek tick (the ch-0 grate never moved).
#[test]
fn the_bunker_bed_hatch_opens_on_e_and_holds_the_open_pose() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000002.bin", false).expect("load 07000002");
    let ss440 = scene.sim.as_ref().unwrap().drones.sss.of_placement(440).expect("grill1 SS exists [D #440]");
    let (closed, open, centre) = {
        let sim = scene.sim.as_ref().unwrap();
        let s = &sim.drones.sss.list[ss440];
        let last = (s.keys.len() - 1) as u16;
        (s.matrix,
         nf_game::door::interp(&s.keys, 0, last, last as f32, s.spline).unwrap(),
         s.pos())
    };
    {
        let sim = scene.sim.as_mut().unwrap();
        // Bond in the bunker at the intro area, onto the bed under the grate (the object's world
        // bounds [D]: centre (−22.40, 0.34, 32.68), top 0.75), then settle.
        sim.bond.pos = [-22.35, 1.2, 32.95];
        sim.bond.yaw = 0.2;
        sim.bond.pitch = 0.0;
        for _ in 0..20 { sim.step_bond(Input::default(), false, false, false); }
        // aim head→grate: the game's use pick is a head+forward ray vs the mesh [M 0x1a5170]
        let head = sim.bond.head();
        let (dx, dy, dz) = (centre[0] - head[0], centre[1] - head[1], centre[2] - head[2]);
        let horiz = (dx * dx + dz * dz).sqrt();
        sim.bond.yaw = dz.atan2(dx);
        sim.bond.pitch = (dy.atan2(horiz) * 2.0 / std::f32::consts::PI).clamp(-1.0, 1.0);
        // Sister (closed first): the posed matrix is still key 0 — the hatch is shut.
        assert_eq!(sim.drones.sss.list[ss440].matrix, closed, "the hatch is open before any use key");
    }
    // The use key through the real input path (the E edge every other tick), then the sweep
    // (150 ticks to the wrap) — the same shape grates-02 pinned from the teleported stance.
    for t in 0..200u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, t % 2 == 0);
    }
    let sim = scene.sim.as_ref().unwrap();
    let s = &sim.drones.sss.list[ss440];
    eprintln!("after E: state {} t {:.2} matrix trans ({:.2}, {:.2}, {:.2}) open trans ({:.2}, {:.2}, {:.2})",
        s.state, s.t, s.matrix[3][0], s.matrix[3][1], s.matrix[3][2], open[3][0], open[3][1], open[3][2]);
    assert_eq!(s.state, 0, "the wrap disarmed the one-shot grate (p1=1) [M 0x1a9948:121-126]");
    assert_eq!(s.matrix, open, "the hatch did not reach the open key's pose — the E-open is broken");
}
