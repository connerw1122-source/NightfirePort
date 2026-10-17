//! flesh-hit-effects live tests: the guard hit's blood puff, end to end.
//!
//! Playtest 1015-1738: bullet hits on people show "No red puffs".
//!
//! The game's flesh visual [M]: a bullet hit on a person carries `HITDATA+0x50 = 0` ([M
//! `Collide_Jointy` 0x1ec180:125/173 — the anim-coll write; +0x52 = the bone id]), so
//! `Effect_Create` 0x1ce688:63's `material & 0x3f != 0` gate early-outs and NO `Effect_Bullet`
//! runs on a person (`Drone_CollisionHandler` 0x140968 has no effect calls either) — the blood
//! puff is the VICTIM's own: `NDrone2_DealWithObjHit` 0x13fff0's default arm calls
//! `Gas_Init(3.0, 6.0, obj+0x20, HITDATA+0x40, 0, 0, 0x20008bc, 5, 0xffffffff, 0xffffff00, life
//! 0x3c)` and sets the gas obj's `+0xf0 |= 8` draw flag [M disasm 0x14047c site: 0x140470
//! `ori $t0, $t0, 0x8bc`, 0x140474 kind 5, 0x140444/0x140450 the 3.0/6.0 floats, 0x14043c the
//! 0x3c life; the 0x32/0x33 bolt arm 0x140414 = 0x200026e at 1.0/16.0]. The port already spawns
//! that puff into the gas pool (fix-flesh-impacts' `victim_puff`); the leak was RENDER-side: the
//! draw-slot list (scene.rs) only knew the `EffectInfo` rows' gas meshes + SMOKE_QUAD, so the
//! blood mesh had no slot and the puff sat in the pool invisible. The fix adds the pool's own
//! spawn meshes (`gas::POOL_MESHES`) to the slot list.
//!
//! Assertions are numeric: the pool holds the puff with the game's data (mesh 0x0200_08bc, kind 5,
//! scale 3.0, life 60, white rgba) at the guard; the scene has gas DRAW SLOTS for the mesh (slots
//! are only created for hashes the level archive resolves, so a slot proves the mesh draws); the
//! sister world hit spawns no blood puff. Opt-in via `NF_ISO`.

use nf_game::player::Input;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn load() -> nf_view::scene::Scene {
    unsafe { std::env::set_var("NF_SILENT", "1"); }
    nf_view::load(&iso(), "07000005.bin", false).expect("load 07000005")
}

fn settle(scene: &mut nf_view::scene::Scene, ticks: u32) {
    for _ in 0..3000 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); if !scene.sim.as_ref().unwrap().disabled { break; } }
    for _ in 0..ticks { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
}

fn blood_puffs(scene: &nf_view::scene::Scene) -> Vec<&nf_game::gas::Gas> {
    scene.sim.as_ref().unwrap().drones.gas.puffs.iter().filter(|p| p.mesh == 0x0200_08bc).collect()
}

/// THE repro: shoot a guard on 07000005 through the real fire path — the pool gains the game's
/// blood puff with its exact `Gas_Init` data, at the guard — and the scene carries the gas draw
/// slots for the blood mesh (the fix; a slot only exists when the level archive resolves the hash,
/// so the slot doubles as the "the mesh draws" proof). The sister mesh checks (the Samurai bolt's
/// 0x0200_026e and the trail smoke 0x0200_001f — the same hardcoded-site family) ride along.
#[test]
fn a_guard_hit_spawns_the_blood_puff_and_the_mesh_has_draw_slots() {
    let mut scene = load();
    settle(&mut scene, 60);
    // The guard: the first live one, moved 5 units ahead of Bond (same floor), then a couple of
    // ticks so his pose (and the hit boxes) follow.
    let (gi, guard_pos) = {
        let sim = scene.sim.as_ref().unwrap();
        let yaw = sim.bond.yaw;
        let fwd = [yaw.sin(), 0.0, yaw.cos()];
        let spot = [sim.bond.pos[0] + fwd[0] * 5.0, sim.bond.pos[1], sim.bond.pos[2] + fwd[2] * 5.0];
        let gi = sim.drones.list.iter().position(|d| d.present && d.enabled && !d.corpse && d.health > 0.0)
            .expect("07000005 has a live guard");
        (gi, spot)
    };
    scene.sim.as_mut().unwrap().drones.list[gi].pos = guard_pos;
    for _ in 0..3 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    // Aim at the live torso box centre (the pose settled above), fire from the hip.
    let (chest, hp0) = {
        let sim = scene.sim.as_ref().unwrap();
        let data = nf_game::drone::Data {
            tables: &sim.tables, props: &sim.props, scripts: &sim.scripts, weapons: &sim.weapons,
            world: &sim.world, level: sim.level, sequences: &sim.seqs, skins: &sim.skins,
            skeletons: &sim.skeletons, nav: &sim.nav, covers: &sim.covers, tuning: &sim.tuning,
            hitboxes: &sim.hitboxes, volumes: &sim.volumes, sfx: sim.sfx.as_ref(), models: &sim.models,
            surfaces: sim.surfaces.as_ref(), voices: sim.voices.as_deref(), difficulty: sim.drones.difficulty,
            ai_points: &sim.ai_points,
        };
        let boxes = sim.drones.list[gi].hit_boxes(&data);
        let torso = boxes.iter().find(|b| b.part == 4).expect("the torso box");
        ([torso.m[3][0], torso.m[3][1], torso.m[3][2]], sim.drones.list[gi].health)
    };
    {
        let sim = scene.sim.as_mut().unwrap();
        let d = [chest[0] - sim.bond.pos[0], chest[1] - sim.bond.head()[1], chest[2] - sim.bond.pos[2]];
        let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-6);
        // `Bond::look`'s convention: forward = (sin yaw, 0, cos yaw), pitch in ±1 = ±π/2 units.
        sim.bond.yaw = d[0].atan2(d[2]);
        sim.bond.pitch = (d[1] / l).asin() / std::f32::consts::FRAC_PI_2;
    }
    let mut hit_at = None;
    for _ in 0..40 {
        scene.sim.as_mut().unwrap().step_bond(Input { fire: true, ..Input::default() }, true, false, false);
        if !blood_puffs(&scene).is_empty() { hit_at = Some(scene.sim.as_ref().unwrap().drones.frame); break; }
        if scene.sim.as_ref().unwrap().drones.list[gi].health < hp0 { break; }
    }
    let puffs = blood_puffs(&scene);
    assert!(!puffs.is_empty(), "no 0x0200_08bc puff in the gas pool after firing at a guard (the DealWithObjHit flesh arm never ran or spawned wrong)");
    assert!(scene.sim.as_ref().unwrap().drones.list[gi].health < hp0, "the guard took no damage — the shot never landed, the puff proves nothing");
    let p = puffs[0];
    // The game's exact `Gas_Init` args [M 0x13fff0 disasm 0x14047c site].
    assert_eq!(p.mesh, 0x0200_08bc, "the blood puff's mesh");
    assert_eq!(p.kind, 5, "the blood puff's behaviour type");
    assert!((p.scale - 3.0).abs() < 1e-4, "scale start 3.0, got {}", p.scale);
    assert!((p.life - 60.0).abs() < 1e-4, "life 0x3c = 60 ticks, got {}", p.life);
    assert!(p.rgba.iter().all(|&c| (c - 255.0).abs() < 1e-3), "rgba starts 0xffffffff (the mesh carries the red), got {:?}", p.rgba);
    let gpos = scene.sim.as_ref().unwrap().drones.list[gi].pos;
    let near = (p.pos[0] - gpos[0]).powi(2) + (p.pos[1] - gpos[1]).powi(2) + (p.pos[2] - gpos[2]).powi(2);
    assert!(near < 9.0, "the blood puff spawned {:.1} units from the guard", near.sqrt());
    assert!(hit_at.is_some(), "the puff appeared only after the shot tick");
    // THE fix: the blood mesh now has gas draw slots — and since scene.rs only creates slots for
    // hashes `level.by_hash` resolves, a slot is the "this mesh draws on this level" proof.
    let slots = |mesh: u32| scene.attachments.iter()
        .filter(|a| matches!(a.gas, Some((_, m)) if m == mesh)).count();
    assert!(slots(0x0200_08bc) > 0, "no gas draw slot for the blood puff 0x0200_08bc — the pool puff would still be invisible (the playtest's 'no red puffs')");
    assert!(slots(0x0200_026e) > 0, "no gas draw slot for the Samurai bolt puff 0x0200_026e (the same hardcoded-site family)");
    assert!(slots(0x0200_001f) > 0, "no gas draw slot for the projectile trail smoke 0x0200_001f");
}

/// The sister: a world hit spawns the material row's own puffs and NEVER the blood mesh — the flesh
/// puff belongs to the victim's reaction only.
#[test]
fn a_world_hit_spawns_no_blood_puff() {
    let mut scene = load();
    settle(&mut scene, 60);
    // Straight down at the floor: the ray hits the ground within ~2 units of the head.
    scene.sim.as_mut().unwrap().bond.pitch = -1.0;
    let mut saw_impact = false;
    for _ in 0..20 {
        let before = scene.sim.as_ref().unwrap().drones.gas.puffs.len();
        scene.sim.as_mut().unwrap().step_bond(Input { fire: true, ..Input::default() }, true, false, false);
        for _ in 0..4 {
            scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        }
        let after = &scene.sim.as_ref().unwrap().drones.gas.puffs;
        if after.len() > before { saw_impact = true; }
        assert!(blood_puffs(&scene).is_empty(), "a world hit spawned the blood puff 0x0200_08bc — flesh effects must not run on static hits");
    }
    assert!(saw_impact, "the floor shots spawned no gas at all — the fire path never ran (aim/gun setup broke)");
}
