//! fix-dev-spawn-visible: the dev window's NPC spawns must produce a guard that shows up and fights,
//! through the REAL dev path (`dev::apply_menu_action_to_scene` + `Sim::step_bond`). Opt-in via `NF_ISO`.
//!
//! The main agent's dry run on master (`nf-view <iso> 07000005 --dev`, Bond at the spawn (105, 25, 109)
//! looking at the snow a few metres ahead): both "spawn at crosshair" (`MenuAction::Spawn` →
//! `Dev::spawn_guard`, the log's `[dev] guard #16: spawned (normal) at (101.2,24.3,106.4)`) and the skin
//! rows ("skin 05000030 (1/3)", `MenuAction::SpawnNpcSkin` → `panel::spawn_npc`, the pool dropping to
//! 0/3) put no guard anywhere near Bond. The candidates this test separates: under the floor, never
//! posed, portal-culled (no cel link — `build_LinkToRoom` 0x1c9680), missing from the frame's palette,
//! or the AI never engages.

use nf_view::dev::{Dev, MenuAction, SpawnPick};
use nf_view::scene::{Scene, ViewOpts};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// A hand-built palette layout covering every character (the GPU renderer builds the real one; the
/// layout is plain data — scene.rs `SkinLayout`). `skin_palette` breaks at the first character past
/// `char_base.len()` — the dev spawn's added character — which is exactly the "missing from the
/// frame's palette" candidate this helper detects.
fn full_layout(scene: &Scene) -> nf_view::scene::SkinLayout {
    let mut char_base = Vec::with_capacity(scene.characters.len());
    let mut b = 0u32;
    for c in &scene.characters { char_base.push(b); b += c.skeleton.bones as u32 + 1; }
    let fs = scene.frame_state(None, 0.0, ViewOpts::default());
    let attach_base = b;
    let door_base = attach_base + fs.attach.len() as u32;
    let (doors, picks) = scene.sim.as_ref().map(|s| (s.drones.doors.list.len() + s.drones.sss.list.len(), s.drones.pickups.len())).unwrap_or((0, 0));
    let pick_base = door_base + doors as u32;
    // The colour table: cols is sized col_one + 1; the writers reach col_att + attachments,
    // col_pick + pickups, col_sl + searchlights and the per-character mult indices.
    let parts = scene.sim.as_ref().map(|s| s.drones.searchlights.parts.len()).unwrap_or(0);
    let attaches = fs.attach.len();
    let col_one = (scene.characters.len()).max(attaches + picks + parts) as u32 + 1;
    nf_view::scene::SkinLayout {
        char_base,
        attach_base,
        sl_base: door_base,
        door_base,
        pick_base,
        n_mats: pick_base + picks as u32,
        col_att: 0,
        col_pick: attaches as u32,
        col_sl: (attaches + picks) as u32,
        col_one,
    }
}

/// The spawn dry run: Bond at the 07000005 start looking at the snow a few metres ahead (the main
/// agent's exact spot). Returns (scene, the spawned drone index, the bound character indices).
fn spawn_dry_run(action: MenuAction) -> (Scene, usize, Vec<usize>) {
    let mut scene = nf_view::load(&iso().as_deref().expect("NF_ISO"), "07000005.bin", false).expect("load 07000005");
    let mut dev = Dev::default();
    let eye = scene.sim.as_ref().unwrap().bond.head();
    let n0 = scene.sim.as_ref().unwrap().drones.list.len();
    let out = nf_view::dev::apply_menu_action_to_scene(&mut dev, &mut scene, action, eye, 0.0, false);
    eprintln!("[dev out] {}", out.status);
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.list.len() > n0, "the dev action added no drone: {}", out.status);
    }
    let (gi, chars) = {
        let sim = scene.sim.as_ref().unwrap();
        let gi = sim.drones.list.len() - 1;
        let chars: Vec<usize> = (0..scene.characters.len())
            .filter(|&ci| sim.char_drone.get(ci).copied().flatten() == Some(gi))
            .collect();
        (gi, chars)
    };
    (scene, gi, chars)
}

/// Ticks the sim `n` steps through the real path.
fn tick(scene: &mut Scene, n: usize) {
    for _ in 0..n { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
}

/// "spawn at crosshair": the spawned guard stands on the floor in front of Bond, in his own (visible)
/// cel, posed, and — normal mode — notices and shoots at him.
#[test]
fn spawn_at_crosshair_guards_on_the_floor_in_front_and_fighting() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let (mut scene, gi, chars) = spawn_dry_run(MenuAction::Spawn(SpawnPick::Normal));
    let bond_pos;
    {
        let sim = scene.sim.as_ref().unwrap();
        bond_pos = sim.bond.pos;
        let dr = &sim.drones.list[gi];
        eprintln!("spawned: pos {:?} present {} enabled {} drawn {} state {:#x} cel {:?} (bond cel {:?})",
            dr.pos, dr.present, dr.enabled, dr.drawn, dr.state, sim.cel_at(dr.pos), sim.cel_at(bond_pos));
    }
    // Settle: 2 s of sim — gravity, the AI start, the first sight checks.
    tick(&mut scene, 120);
    {
        let sim = scene.sim.as_ref().unwrap();
        let dr = &sim.drones.list[gi];
        eprintln!("t120: pos {:?} drawn {} state {:#x} health {}", dr.pos, dr.drawn, dr.state, dr.health);
        // ON THE FLOOR: the drone's root sits within a body height of the floor under him (the placed
        // guards stand ~1.0 above it; the crosshair y can sit above the snow — gravity settles).
        let floor = sim.world.floor_below([dr.pos[0], dr.pos[1] + 1.0, dr.pos[2]], 12.0).expect("a floor under the spawned guard");
        assert!((dr.pos[1] - floor - 1.0).abs() < 1.2,
            "the spawned guard is not on the floor: root {} vs floor {} (the under-the-floor/inside-the-wall candidate)", dr.pos[1], floor);
        // IN FRONT: within 8 m of Bond.
        let dx = dr.pos[0] - bond_pos[0];
        let dz = dr.pos[2] - bond_pos[2];
        assert!((dx * dx + dz * dz).sqrt() < 8.0, "the spawned guard is not in front of Bond ({} m)", (dx * dx + dz * dz).sqrt());
        // IN A VISIBLE ROOM: the guard stands in SOME room cel (not a void — the palette's per-position
        // lookup has an answer; the pool guard may slide into the adjacent room on the spawn slope,
        // and the live-drone palette exemption covers the walk's dropped rooms).
        assert!(sim.cel_at(dr.pos).is_some(), "the spawned guard is in no room cel (a void)");
    }
    // POSED + IN THE PALETTE: the bound character has a pose and is not hidden; with a layout that
    // covers every character the palette carries the guard's object matrix, not the FAR collapse.
    assert!(!chars.is_empty(), "no character is bound to the spawned drone (char_drone) — the renderer has nothing to draw");
    // The bound character's MESH must be in the renderer's vertex buffer: the character layer's
    // vertices were uploaded at build time (the Character-layer draw groups → `Scene::char_verts`,
    // the `v.character` tag). A Character pushed after the build (the old spawn_guard) has a
    // palette but no vertices — the GPU draws nothing (fix-dev-spawn-visible's crosshair-row bug).
    for &ci in &chars {
        assert!(scene.char_verts.iter().any(|v| v.character as usize == ci),
            "the bound character {ci} has no vertices in the renderer's buffer — nothing draws");
    }
    {
        let fs = scene.frame_state(None, 0.0, ViewOpts::default());
        for &ci in &chars {
            assert!(!fs.hidden[ci], "the spawned guard's character {ci} is hidden (the present/enabled gate)");
            assert!(fs.posed[ci].is_some(), "the spawned guard's character {ci} has no pose");
        }
    }
    {
        let l = full_layout(&scene);
        let (pal, _) = scene.skin_palette(&l, None, 0.0, ViewOpts::default(), None);
        for &ci in &chars {
            let m = pal[l.char_base[ci] as usize];
            assert!(m[3][1] > -9999.0,
                "the spawned guard's palette slot is the FAR collapse — nothing draws (the palette candidate)");
            eprintln!("palette[char {ci}] object at ({:.1},{:.1},{:.1})", m[3][0], m[3][1], m[3][2]);
        }
    }
    // HE FIGHTS: the playtest's own flow — Bond walks up to him. Inside 2 m the game's visibility
    // score is the guaranteed 1.0 (`DroneVision_OpponentVisibility` 0x175af0's dist < 2 arm [M]),
    // which bypasses the 0.25 shadow volume Bond starts in [L WORKLOG 0032], so a normal-mode guard
    // sees him and opens fire. No warm-up shot: the level's own placement HP for these guards is 10
    // [L probe: drone health 10] and one P2K round kills him — a fading corpse cannot fight.
    {
        let sim = scene.sim.as_mut().unwrap();
        let (b, g) = (sim.bond.pos, sim.drones.list[gi].pos);
        sim.bond.yaw = (g[0] - b[0]).atan2(g[2] - b[2]);
        sim.bond.pitch = 0.0;
    }
    let (bond_health_0, mut fired) = (scene.sim.as_ref().unwrap().drones.health.health, false);
    for t in 0..2400 {
        let dx = scene.sim.as_ref().unwrap().drones.list[gi].pos[0] - scene.sim.as_ref().unwrap().bond.pos[0];
        let dz = scene.sim.as_ref().unwrap().drones.list[gi].pos[2] - scene.sim.as_ref().unwrap().bond.pos[2];
        let close = dx * dx + dz * dz < 1.8 * 1.8;
        let input = nf_game::player::Input { forward: if close { 0.0 } else { 1.0 }, ..Default::default() };
        scene.sim.as_mut().unwrap().step_bond(input, false, false, false);
        let live = scene.sim.as_ref().unwrap().drones.bullets.iter().any(|b| b.state == 1);
        if live { fired = true; eprintln!("[fight] a guard bullet is live at tick {t}"); break; }
    }
    {
        let sim = scene.sim.as_ref().unwrap();
        let dr = &sim.drones.list[gi];
        eprintln!("fight end: state {:#x} drawn {} bullets {} bond health {} → {}",
            dr.state, dr.drawn, sim.drones.bullets.len(), bond_health_0, sim.drones.health.health);
        assert!(fired || sim.drones.health.health < bond_health_0 || matches!(dr.state, 0x56 | 0x58 | 0x59 | 0x5a | 0x5b | 0x67 | 0x68),
            "the spawned normal guard never noticed Bond (state {:#x}, drawn {}, no bullet, no damage) — the AI candidate", dr.state, dr.drawn);
    }
}

/// The skin rows: a pool NPC of the level's skin bound to a fresh drone — same assertions (the pool
/// characters are the hidden copies far below the world; the drone must pull them in front of Bond).
#[test]
fn spawn_npc_skin_binds_a_pool_guard_in_front_of_bond_and_fighting() {
    let Some(_) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // The skin rows come from the level's skins; take the first active guard's skin — the same one
    // "spawn at crosshair" picks (dev.rs's own skin choice).
    let skin = {
        let mut scene = nf_view::load(&iso().as_deref().expect("NF_ISO"), "07000005.bin", false).expect("load 07000005");
        let sim = scene.sim.take().unwrap();
        sim.drones.list.iter()
            .find(|x| x.present && x.enabled && !x.corpse && x.health > 0.0 && x.drawn)
            .map(|x| x.skin)
            .expect("07000005 has an active guard to copy the skin from")
    };
    let (mut scene, gi, chars) = spawn_dry_run(MenuAction::SpawnNpcSkin(skin));
    let bond_pos;
    {
        let sim = scene.sim.as_ref().unwrap();
        bond_pos = sim.bond.pos;
        let dr = &sim.drones.list[gi];
        eprintln!("skin {skin:08x}: drone pos {:?} present {} enabled {} state {:#x}, chars {chars:?}",
            dr.pos, dr.present, dr.enabled, dr.state);
    }
    assert!(!chars.is_empty(), "the skin spawn bound no pool character (the pool candidate)");
    tick(&mut scene, 120);
    {
        let sim = scene.sim.as_ref().unwrap();
        let dr = &sim.drones.list[gi];
        let floor = sim.world.floor_below([dr.pos[0], dr.pos[1] + 1.0, dr.pos[2]], 12.0).expect("a floor under the pool guard");
        assert!((dr.pos[1] - floor - 1.0).abs() < 1.2,
            "the pool guard is not on the floor: root {} vs floor {} (the y −1e6 pool copy must move to the drone)", dr.pos[1], floor);
        let dx = dr.pos[0] - bond_pos[0];
        let dz = dr.pos[2] - bond_pos[2];
        assert!((dx * dx + dz * dz).sqrt() < 8.0, "the pool guard is not in front of Bond");
        assert!(sim.cel_at(dr.pos).is_some(), "the pool guard is in no room cel (a void)");
    }
    {
        let fs = scene.frame_state(None, 0.0, ViewOpts::default());
        for &ci in &chars {
            assert!(!fs.hidden[ci], "the pool guard's character {ci} is hidden");
            assert!(fs.posed[ci].is_some(), "the pool guard's character {ci} has no pose");
        }
    }
    {
        let l = full_layout(&scene);
        let (pal, _) = scene.skin_palette(&l, None, 0.0, ViewOpts::default(), None);
        for &ci in &chars {
            let m = pal[l.char_base[ci] as usize];
            assert!(m[3][1] > -9999.0, "the pool guard's palette slot is the FAR collapse (the y −1e6 copy never moved into the drone's pose)");
            eprintln!("palette[pool char {ci}] object at ({:.1},{:.1},{:.1})", m[3][0], m[3][1], m[3][2]);
        }
    }
    // The fight: the same game-natural flow — walk up (the sub-2 m guaranteed visibility bypasses
    // the shadow volume; no warm-up shot, the level's placement HP is 10 and one round kills).
    {
        let sim = scene.sim.as_mut().unwrap();
        let (b, g) = (sim.bond.pos, sim.drones.list[gi].pos);
        sim.bond.yaw = (g[0] - b[0]).atan2(g[2] - b[2]);
        sim.bond.pitch = 0.0;
    }
    let mut fired = false;
    for t in 0..2400 {
        let (gp, bp) = (scene.sim.as_ref().unwrap().drones.list[gi].pos, scene.sim.as_ref().unwrap().bond.pos);
        let dx = gp[0] - bp[0];
        let dz = gp[2] - bp[2];
        let close = dx * dx + dz * dz < 1.8 * 1.8;
        let input = nf_game::player::Input { forward: if close { 0.0 } else { 1.0 }, ..Default::default() };
        scene.sim.as_mut().unwrap().step_bond(input, false, false, false);
        let live = scene.sim.as_ref().unwrap().drones.bullets.iter().any(|b| b.state == 1);
        if live { fired = true; eprintln!("[fight] a pool-guard bullet is live at tick {t}"); break; }
    }
    {
        let sim = scene.sim.as_ref().unwrap();
        let dr = &sim.drones.list[gi];
        eprintln!("fight end: state {:#x} drawn {}", dr.state, dr.drawn);
        assert!(fired || matches!(dr.state, 0x56 | 0x58 | 0x59 | 0x5a | 0x5b | 0x67 | 0x68),
            "the pool guard never noticed Bond (state {:#x})", dr.state);
    }
}
