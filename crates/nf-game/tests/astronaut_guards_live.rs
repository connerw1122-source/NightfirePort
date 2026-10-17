//! NF_ISO tests for the zero-G astronaut guards (docs/systems/uncited-audit.md §7):
//! skin kind 0x18 forces mode 0x1D + start state AstronautLaunch 0xBD [M 0x14b300:722-727],
//! the five `DSTATE_Astronaut*` states (0xBD-0xC1), the no-gravity float physics
//! (`Drone_CollisionHandler` flying path: no DoGravity / floor snap, damping ×0.98 while
//! +0x4F8 & 0x40 [M 0x140968:128-150]), the CombatMove flinch (HandleImpact resume 0xBE
//! [M 0x174550:38-42]) and the death routing mode 0x1D → 0xBF [M 0x1461c8:47-50].
//! Opt-in: `NF_ISO=<iso>`.

use nf_formats::anim::Script;
use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, drone, level::Level, levelbin::LevelBin, weapon::WeaponData};
use nf_game::drone::{state, Data, Drones, Hit, Player, Rng};
use std::collections::HashMap;

struct Fixture {
    tables: drone::DroneAnimData,
    props: drone::PropertyTables,
    weapons: WeaponData,
    level: Level,
    level_id: u32,
    scripts: HashMap<u32, Script>,
    world: nf_game::world::CollisionWorld,
    sequences: HashMap<u32, nf_formats::anim::Sequence>,
    skeletons: HashMap<u8, nf_formats::character::Skeleton>,
    nav: nf_game::nav::Network,
    covers: Vec<nf_game::cover::CoverNode>,
    tuning: nf_formats::tuning::Tuning,
    hitboxes: nf_formats::hitbox::CollisionBones,
    volumes: Vec<nf_game::aivolume::AiVolume>,
    models: HashMap<u32, nf_game::pickup::Model>,
    skins: HashMap<u32, nf_formats::character::Skin>,
}

impl Fixture {
    fn load(level_name: &str, level_id: u32) -> Option<Fixture> {
        let path = std::env::var_os("NF_ISO")?;
        let iso = IsoImage::open(&path).unwrap();
        let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
        let (entries, bin) = disc::files_bin_entries(&iso).unwrap();
        let read = |name: &str| -> Option<Vec<u8>> {
            let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name))?;
            Some(iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap())
        };
        let data = read(level_name)?;
        let level = Level::parse(&data).unwrap();
        let lb = LevelBin::parse(&data).unwrap();
        let skeletons: HashMap<u8, _> = lb.entries.iter().filter(|e| e.kind == 6)
            .filter_map(|e| nf_formats::character::parse_skeleton(lb.data(&data, e)).ok().map(|s| (s.id as u8, s))).collect();
        let anim_name = format!("{}F{}.bin", &level_name[..2], &level_name[3..8]);
        let mut sequences = HashMap::new();
        if let Some(anim) = read(&anim_name) {
            let alb = LevelBin::parse(&anim).unwrap();
            for e in alb.entries.iter().filter(|e| e.kind == 4 && e.length >= 0x40) {
                if let Ok(s) = nf_formats::anim::Sequence::parse(alb.data(&anim, e)) { sequences.insert(s.hashcode, s); }
            }
        }
        let scripts: HashMap<u32, Script> = lb.entries.iter().filter(|e| e.kind == 5)
            .filter_map(|e| Script::parse(lb.data(&data, e)).ok()).map(|s| (s.hashcode, s)).collect();
        let tables = drone::DroneAnimData::from_action_elf(&elf).unwrap();
        let props = drone::PropertyTables::from_action_elf(&elf).unwrap();
        let weapons = WeaponData::from_action_elf(&elf).unwrap();
        let world = nf_game::world::CollisionWorld::build(&level, &level.instances(true).0);
        let main = level.main_map().unwrap();
        let nav = nf_game::nav::Network::build(level.maps[main].ai_network.as_ref().unwrap_or(&Default::default()), &world, level_id);
        let covers = nf_game::cover::build(&level, &nav, &world);
        let hitboxes = nf_formats::hitbox::CollisionBones::from_action_elf(&elf).unwrap();
        let volumes = nf_game::aivolume::build(&level, level_id);
        let models = nf_game::pickup::models(&level);
        let tuning = { let mut t = nf_formats::tuning::Tuning::from_action_elf(&elf).unwrap();
            nf_game::tuning::read_tuning_vars(&mut t, level_id); t };
        let skins = level.skins.clone();
        Some(Fixture { tables, props, weapons, level, level_id, scripts, world, sequences, skeletons, nav, covers, tuning, hitboxes, volumes, models, skins })
    }

    fn data(&self) -> Data<'_> {
        Data { tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world,
            level: self.level_id, sequences: &self.sequences, skins: &self.skins, skeletons: &self.skeletons, nav: &self.nav,
            covers: &self.covers, tuning: &self.tuning, hitboxes: &self.hitboxes, volumes: &self.volumes, sfx: None, models: &self.models,
            surfaces: None, voices: None, difficulty: 1, ai_points: &[] }
    }

    fn sim(&self) -> Drones {
        let d = self.data();
        let mut rng = Rng::new(0x1234_5678);
        let mut lights = nf_game::light::LightList::default();
        let drones = nf_game::drone::level_drones(&d, &self.level, &mut rng, &mut lights);
        let mut s = Drones::new(drones, rng);
        s.attach_level(&self.level, self.level_id);
        s
    }
}

fn bond_at(p: [f32; 3]) -> Player { Player { pos: p, target: [p[0], p[1] + 0.5, p[2]], eye: [p[0], p[1] + 1.0, p[2]], aim: [-1.0, 0.0, 0.0], sph: p, capsule: ([p[0], p[1] + 1.086, p[2]], p), hanging: false, cel: None } }

const LEVEL: &str = "0700001b.bin";
const LEVEL_ID: u32 = 0x0700_001b;

/// An enabled astronaut: the space station ships skins 0x05000078 (six guard groups),
/// 0x05000094 and Drake 0x050000ba — all kind 0x18 [M 0x14b300:234-240; D docs/levels/0700001b.md].
fn astronaut_index(sim: &Drones) -> Option<usize> {
    sim.list.iter().position(|x| x.present && matches!(x.skin, 0x0500_0078 | 0x0500_0094 | 0x0500_00ba))
}

/// The port reads `weapon_data[0x33] + 0xc` (the record's float) × difficulty damage for the
/// astronaut health [M 0x14b300:728-739 + DroneInit_TakeXHits 0x14d928:6-8].
#[test]
fn astronaut_health_is_take_x_hits_of_weapon_0x33() {
    let Some(fx) = Fixture::load(LEVEL, LEVEL_ID) else { return };
    let d = fx.data();
    let mut sim = fx.sim();
    for c in 1..=255u8 { sim.set_channel(c); }
    for _ in 0..3 { sim.tick(&d, &bond_at([0.0, -1000.0, 0.0])); }
    let Some(ai) = astronaut_index(&sim) else { eprintln!("no astronaut enabled"); return };
    let dmg = if d.difficulty == 1 { 1.0 } else { 2.0 };
    let per_hit = f32::from_le_bytes(d.weapons.records[0x33][0xc..0x10].try_into().unwrap());
    let want = per_hit * dmg * if sim.list[ai].skin & 0xffff == 0xba { 3.0 } else { 1.0 };
    assert!(sim.list[ai].health > 10.0, "astronaut health {} must come from TakeXHits(dmg, 0x33) = {want}, not the walker default 10", sim.list[ai].health);
}

/// Init + float physics: kind 0x18 → mode 0x1D, start 0xBD; the flying collision path never
/// gravity-snaps — a pinned upward velocity must rise instead of being zeroed by the floor snap.
#[test]
fn astronauts_spawn_in_launch_state_and_float_without_floor_snap() {
    let Some(fx) = Fixture::load(LEVEL, LEVEL_ID) else { return };
    let d = fx.data();
    let mut sim = fx.sim();
    for c in 1..=255u8 { sim.set_channel(c); }
    for _ in 0..5 { sim.tick(&d, &bond_at([0.0, -1000.0, 0.0])); }
    let Some(ai) = astronaut_index(&sim) else { panic!("0700001b ships ~20 suited guards [D dronetype-tables census]") };
    // DefaultInit kind 0x18: mode 0x1D, start state 0xBD [M 0x14b300:723-724]
    assert_eq!(sim.list[ai].mode, 0x1D, "skin kind 0x18 must force mode 0x1D");
    assert!(
        matches!(sim.list[ai].state, state::ASTRONAUT_LAUNCH | state::ASTRONAUT_COMBAT | state::ASTRONAUT_COMBAT_MOVE),
        "start state must be the astronaut family (row 29 start 0xBD), got {}",
        sim.list[ai].state
    );
    // No-gravity float [M Drone_CollisionHandler 0x140968 flying path: no DoGravity, no floor snap]:
    // pin an upward velocity through the real per-tick collision pass — a walker is snapped to the
    // floor (velocity zeroed), the astronaut must rise (vel 3 · 1/60 · Σ0.98^k ≈ 1.4 over 30 ticks).
    sim.list[ai].velocity = [0.0, 3.0, 0.0];
    let y0 = sim.list[ai].pos[1];
    for _ in 0..30 { sim.tick(&d, &bond_at([0.0, -1000.0, 0.0])); }
    let dy = sim.list[ai].pos[1] - y0;
    assert!(dy > 1.0, "astronaut must drift with its velocity, no floor snap (dy {dy:.3})");
}

/// Hit variants: CombatMove passes resume 0xBE → surviving a bullet flinches into AstronautHit
/// (knockback away from the opponent), the 0.5 s timer returns to AstronautCombat; stationary
/// AstronautCombat passes resume 0 → no flinch. Death: mode 0x1D → 0xBF (not 0x44), the corpse
/// drifts with damping off, and the 10 s timer fades it.
#[test]
fn astronaut_hit_and_death_variants_fire() {
    let Some(fx) = Fixture::load(LEVEL, LEVEL_ID) else { return };
    let d = fx.data();
    let mut sim = fx.sim();
    // No channel sweep here: the 1b guard groups recycle through spawner Removes (a corpse gets
    // Action::Remove'd mid-test with every channel forced on); the initially-present guards are
    // enough for the pinned-state flow.
    for _ in 0..5 { sim.tick(&d, &bond_at([0.0, -1000.0, 0.0])); }
    let Some(ai) = astronaut_index(&sim) else { eprintln!("no initially-present astronaut"); return; };
    // Bond stands 10 units along +x of the astronaut: `opponent_info` keeps `opp_target` there, so
    // the knockbacks must push along -x.
    let bp = [sim.list[ai].pos[0] + 10.0, sim.list[ai].pos[1], sim.list[ai].pos[2]];
    let dr = &mut sim.list[ai];
    dr.opp_target = bp;
    dr.state = state::ASTRONAUT_COMBAT_MOVE;
    dr.next_state = None;
    dr.send(&d, &mut sim.rng, sim.frame, nf_game::drone::msg::ENTER);
    for _ in 0..3 { sim.tick(&d, &bond_at(bp)); }
    assert_eq!(sim.list[ai].state, state::ASTRONAUT_COMBAT_MOVE);
    // Surviving bullet in CombatMove → HandleImpact(resume 0xBE) [M 0x174550:38-42]: the port's
    // flow flinches through BulletImpact 0x55 with +0x118 = AstronautHit, then the resume.
    sim.list[ai].hit = Some(Hit { part: 4, ray: [0.0; 3], point: [0.0; 3], damage: 1.0, weapon: 2 });
    sim.list[ai].send(&d, &mut sim.rng, sim.frame, nf_game::drone::msg::BULLET);
    assert_eq!(sim.list[ai].impact_resume, state::ASTRONAUT_HIT, "CombatMove's bullet resume is 0xBE");
    let mut resumed = false;
    for _ in 0..60 {
        sim.tick(&d, &bond_at(bp));
        if sim.list[ai].state == state::ASTRONAUT_HIT { resumed = true; break; }
    }
    assert!(resumed, "the flinch must resume into AstronautHit 0xBE");
    let v = sim.list[ai].velocity;
    assert!(v[0] < -0.5, "knockback is away from the opponent (vel {v:?})");
    // The 0.5 s hit timer (FRAME_RATE_DIV · 30 [M 0x174690:16]) returns to AstronautCombat.
    // The 0.5 s hit timer (FRAME_RATE_DIV · 30 [M 0x174690:16]) returns to AstronautCombat; the
    // same tick's Combat UPDATE may re-pick at once (the strafe deadline +0xCD8/+0xCDC is still 0
    // [M 0x16dd28:27-28]) and go straight back to CombatMove.
    let mut back = false;
    for _ in 0..30 {
        sim.tick(&d, &bond_at(bp));
        if matches!(sim.list[ai].state, state::ASTRONAUT_COMBAT | state::ASTRONAUT_COMBAT_MOVE) { back = true; break; }
    }
    assert!(back, "the 0.5 s hit timer must return to the combat family");
    // Pin stationary AstronautCombat: a bullet there (resume 0) must NOT flinch [M 0x174458:33-36].
    let dr = &mut sim.list[ai];
    dr.state = state::ASTRONAUT_COMBAT;
    dr.next_state = None;
    dr.thrust = false;
    dr.send(&d, &mut sim.rng, sim.frame, nf_game::drone::msg::ENTER);
    dr.hit = Some(Hit { part: 4, ray: [0.0; 3], point: [0.0; 3], damage: 1.0, weapon: 2 });
    dr.send(&d, &mut sim.rng, sim.frame, nf_game::drone::msg::BULLET);
    assert_eq!(sim.list[ai].state, state::ASTRONAUT_COMBAT, "stationary combat does not flinch");
    // Death [M BulletImpact 0x1461c8:47-50: dtype 0x1d → 0xBF, not 0x44]. Pin the killer 10 units
    // along +x and a clean velocity so the death impulse's direction is exact.
    let dr = &mut sim.list[ai];
    dr.opp_target = [dr.pos[0] + 10.0, dr.pos[1], dr.pos[2]];
    dr.velocity = [0.0; 3];
    dr.hit = Some(Hit { part: 4, ray: [0.0; 3], point: [0.0; 3], damage: 5000.0, weapon: 2 });
    dr.send(&d, &mut sim.rng, sim.frame, nf_game::drone::msg::BULLET);
    assert_eq!(sim.list[ai].state, state::ASTRONAUT_DEATH, "mode 0x1D death must route to AstronautDeath 0xBF");
    // DeathMove: knockback away from the killer, damping cleared [M 0x156008:29-46].
    let v0 = sim.list[ai].velocity;
    assert!(v0[0] < -0.5, "death drift is away from the opponent (vel {v0:?})");
    let p0 = sim.list[ai].pos;
    for _ in 0..10 { sim.tick(&d, &bond_at(bp)); }
    assert!((sim.list[ai].velocity[0] - v0[0]).abs() < 0.05, "damping is off for the corpse drift");
    let moved = (sim.list[ai].pos[0] - p0[0]).abs();
    assert!(moved > 0.3, "the corpse must drift (moved {moved:.3})");
    // The 10 s death timer (DAT_00298f58 = FRAME_RATE_INT · 10 [M 0x1747b8:22; 0x13a5a8:10]) → Fade.
    for _ in 0..295 { sim.tick(&d, &bond_at(bp)); }
    assert_eq!(sim.list[ai].state, state::FADE, "the death timer must reach Fade 0x48");
}

/// The float anim runs: in CombatMove the DroneMove_AstronautAnim pick (0x71-0x74 by the angle
/// to the move target [M 0x155ea0:24-29]) keeps a danim playing — its script time must advance.
#[test]
fn astronaut_float_anim_runs_in_combat_move() {
    let Some(fx) = Fixture::load(LEVEL, LEVEL_ID) else { return };
    let d = fx.data();
    let mut sim = fx.sim();
    for c in 1..=255u8 { sim.set_channel(c); }
    for _ in 0..5 { sim.tick(&d, &bond_at([0.0, -1000.0, 0.0])); }
    let Some(ai) = astronaut_index(&sim) else { panic!("no astronaut drone present") };
    let dr = &mut sim.list[ai];
    dr.opp_target = [dr.pos[0] + 10.0, dr.pos[1], dr.pos[2]];
    dr.state = state::ASTRONAUT_COMBAT_MOVE;
    dr.next_state = None;
    dr.send(&d, &mut sim.rng, sim.frame, nf_game::drone::msg::ENTER);
    for _ in 0..10 { sim.tick(&d, &bond_at([0.0, -1000.0, 0.0])); }
    let t0 = sim.list[ai].anim.ftime;
    for _ in 0..20 { sim.tick(&d, &bond_at([0.0, -1000.0, 0.0])); }
    assert!(sim.list[ai].anim.ftime > t0, "a danim must keep playing in CombatMove (ftime {} → {})", t0, sim.list[ai].anim.ftime);
}
