//! Guard sight-to-first-shot timing (guard-reaction-delay): the user's "guards shoot way too soon upon spotting
//! you, at least give it half a second" and "his gun's firing before it lowers at me" (07000006 wine cellar, the
//! searchlights, 07000005). Measures the frames from first sight to the first bullet for the game's three sighting
//! paths, through the real sim path (positions only — no state injection):
//! 1. fresh idle guard, own sight — the sight reaction floor (`DroneFunc_ReactionTime` 0x14ac60: 59 ticks at
//!    alertness (+0x50C) 0, stat 0, and only on 07000002/03/05/09/0a — every other level gets
//!    FRAME_RATE_DIV × 10 ticks, and alertness ≥ 1 returns 0 [M 0x14ac60:9-26]; live notice 61, WORKLOG 0019) plus the state/aim transitions and the first combat
//!    move (a step move plays out its ~100-frame danim with the request held off, [M 0x1676a0:16-18]);
//! 2. a guard already at alertness 1 (the alarm/searchlight-raised guard — `HandleSoundAlerts` and the alarm pulse
//!    set 1.0) spotting Bond: no reaction floor;
//! 3. a mid-combat guard that lost and re-gained sight — the entry fire (`NDrone2_DSTATE_AimStandFire` 0x166328:59-61
//!    fires on entry when `+0x274 < 0x10`), the CombatNoRoute enter clears the request first [M 0x164c08:29-30].
//! Game anchors (docs/live/2026-10-14-weapon-measurements-pcsx2.md, "Guard sighting -> first shot", 07000006):
//! alert -> first bullet +97 frames (~1.6 s) for the stairs guard Bond walked into at 2.8 units; a courtyard guard
//! 16.7 units away +45 frames (attribution weak); the burst spacing 50-60 frames. Opt-in: set `NF_ISO`.

use nf_formats::anim::Script;
use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, drone, level::Level, levelbin::LevelBin, weapon::WeaponData};
use nf_game::drone::{Data, Drone, Drones, Player, Rng};
use std::collections::HashMap;

/// Everything `Data` borrows, owned (the ai_live harness, verbatim).
struct Level05 {
    tables: drone::DroneAnimData,
    props: drone::PropertyTables,
    weapons: WeaponData,
    level: Level,
    scripts: HashMap<u32, Script>,
    world: nf_game::world::CollisionWorld,
    sequences: HashMap<u32, nf_formats::anim::Sequence>,
    skeletons: HashMap<u8, nf_formats::character::Skeleton>,
    nav: nf_game::nav::Network,
    covers: Vec<nf_game::cover::CoverNode>,
    sfx: nf_formats::sfx::SfxTable,
    tuning: nf_formats::tuning::Tuning,
    hitboxes: nf_formats::hitbox::CollisionBones,
    volumes: Vec<nf_game::aivolume::AiVolume>,
    models: std::collections::HashMap<u32, nf_game::pickup::Model>,
    hud: nf_formats::hud::HudTables,
    surfaces: nf_formats::sfx::SurfaceSounds,
    voices: Vec<nf_formats::sfx::VoiceLine>,
}

impl Level05 {
    fn load() -> Option<Level05> {
        let path = std::env::var_os("NF_ISO")?;
        let iso = IsoImage::open(&path).unwrap();
        let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
        let (entries, bin) = disc::files_bin_entries(&iso).unwrap();
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000005.bin")).unwrap();
        let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
        let level = Level::parse(&data).unwrap();
        let lb = LevelBin::parse(&data).unwrap();
        let scripts = lb.entries.iter().filter(|e| e.kind == 5).filter_map(|e| Script::parse(lb.data(&data, e)).ok()).map(|s| (s.hashcode, s)).collect();
        let world = nf_game::world::CollisionWorld::build(&level, &level.instances(true).0);
        let skeletons = lb.entries.iter().filter(|e| e.kind == 6).filter_map(|e| nf_formats::character::parse_skeleton(lb.data(&data, e)).ok()).map(|k| (k.id as u8, k)).collect();
        let m = level.main_map().unwrap();
        let nav = nf_game::nav::Network::build(level.maps[m].ai_network.as_ref().unwrap(), &world, 0x0700_0005);
        let covers = nf_game::cover::build(&level, &nav, &world);
        let volumes = nf_game::aivolume::build(&level, 0x0700_0005);
        Some(Level05 {
            tables: drone::DroneAnimData::from_action_elf(&elf).unwrap(),
            props: drone::PropertyTables::from_action_elf(&elf).unwrap(),
            weapons: WeaponData::from_action_elf(&elf).unwrap(),
            sfx: nf_formats::sfx::SfxTable::from_action_elf(&elf).unwrap(),
            tuning: { let mut t = nf_formats::tuning::Tuning::from_action_elf(&elf).unwrap(); nf_game::tuning::read_tuning_vars(&mut t, 0x0700_0005); t },
            hitboxes: nf_formats::hitbox::CollisionBones::from_action_elf(&elf).unwrap(),
            sequences: disc::level_sequences(&iso, "07000005.bin").unwrap(),
            models: nf_game::pickup::models(&level),
            hud: nf_formats::hud::HudTables::from_action_elf(&elf).unwrap(),
            surfaces: nf_formats::sfx::SurfaceSounds::from_action_elf(&elf).unwrap(),
            voices: nf_formats::sfx::voice_lines(&elf).unwrap(),
            level, scripts, world, skeletons, nav, covers, volumes,
        })
    }

    fn sim(&self, drones: Vec<Drone>, rng: Rng) -> Drones {
        let mut s = Drones::new(drones, rng); s.attach_level_id(&self.level, 0x0700_0005);
        s.difficulty = 1; // the measured sessions ran at difficulty 1 (ai_live harness note)
        s
    }

    fn data(&self) -> Data<'_> {
        Data { tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world, level: 0x0700_0005,
            sequences: &self.sequences, skins: &self.level.skins, skeletons: &self.skeletons, nav: &self.nav, covers: &self.covers, tuning: &self.tuning, hitboxes: &self.hitboxes, volumes: &self.volumes,
            sfx: Some(&self.sfx), models: &self.models, surfaces: Some(&self.surfaces), voices: Some(&self.voices), difficulty: 1,
                ai_points: &[],
            }
    }

    fn drones_seeded(&self, alert: Option<f32>, x: f32, seed: u32) -> (Vec<Drone>, usize, Rng) {
        let seed = std::env::var("NF_TEST_SEED").ok().and_then(|v| v.parse().ok()).unwrap_or(seed);
        let d = self.data();
        let mut rng = Rng::new(seed);
        let mut lights = nf_game::light::LightList::default();
        let mut drones = nf_game::drone::level_drones(&d, &self.level, &mut rng, &mut lights);
        if let Some(a) = alert { for dr in &mut drones { dr.alert = a; dr.alert_floor = a; } }
        let found = drones.iter().position(|dr| (dr.pos[0] - x).abs() < 0.01).unwrap();
        (drones, found, rng)
    }
}

fn bond_at(p: [f32; 3]) -> Player { Player { pos: p, target: [p[0], p[1] + 0.5, p[2]], eye: [p[0], p[1] + 1.0, p[2]], aim: [-1.0, 0.0, 0.0], sph: p, capsule: ([p[0], p[1] + 1.086, p[2]], p), hanging: false, cel: None } }

/// Where the guard is standing (the ai_live notice-test spot) and where Bond appears.
const GUARD_X: f32 = 61.342;
const BOND: [f32; 3] = [67.31, 26.5, 99.47];
const FAR: [f32; 3] = [0.0, -500.0, 0.0];
const WARM: u32 = 10;
/// The window must cover a step-first combat move: the step danim plays ~100 frames with the fire request
/// held off ([M 0x1676a0:16-18] `CallAnim(0x5b→0x68)` + `+0x3b = 0`), the release rides its follow-on.
const RUN: u32 = 220;

fn attack_tick(g: &Drone, warm: u32) -> Option<u32> {
    g.log.iter().find(|(_, s)| *s == nf_game::drone::state::ATTACK).map(|(f, _)| f - warm)
}

/// The three sighting paths, measured and printed as the report table. The assertions pin the
/// game-anchored floors so a future change that fires earlier fails here first.
#[test]
fn sight_to_first_shot_for_the_three_sighting_paths() {
    let Some(l) = Level05::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let away = bond_at(FAR);
    let player = bond_at(BOND);

    // ---- Path 1: fresh idle guard, own sight (the reaction floor) ------------------------------------------------
    let (drones, gi, rng) = l.drones_seeded(None, GUARD_X, 5);
    let mut sim = l.sim(drones, rng);
    for _ in 0..WARM { sim.tick(&d, &away); }
    let mut seen_at1 = None;
    for _ in 0..RUN {
        sim.tick(&d, &player);
        if seen_at1.is_none() && sim.list[gi].seen { seen_at1 = Some(sim.frame - WARM); }
    }
    let g = &sim.list[gi];
    let noticed = attack_tick(g, WARM);
    let first_shot = g.shots.first().map(|f| f - WARM);
    // The first combat move after ATTACK (the state history's second entry): a step move holds the
    // fire request through its danim; the game plays the same 100-frame script.
    let first_move = g.log.iter().filter(|(f, _)| *f > WARM).nth(1).map(|(_, s)| *s);
    eprintln!("[table] path1 fresh idle sight: noticed(ATTACK) {noticed:?}, first_move {first_move:?}, first_shot {first_shot:?}");
    // Live: notice 61 (WORKLOG 0019, 59 + the round-robin wait); the reaction floor holds.
    assert!(noticed.is_some_and(|f| (59..=63).contains(&f)), "path1 noticed {noticed:?} (live 61)");
    let shot1 = first_shot.expect("path1 fires within the window");
    // The game's reaction floor: the guard's own sight ack (seen) must precede the first bullet by at
    // least DroneFunc_ReactionTime's 59 ticks (0x14ac60; live notice 61 = 59 + the round-robin wait).
    // The game's +97 alert->bullet includes Bond walking into the guard's back (the turn); face-on the
    // port's floor is the reaction 59 + the state/aim transitions (a step-first draw ~160).
    let s1 = seen_at1.expect("path1 sees Bond");
    // The fresh guard's sight ack waits out the floor (live 61; round-robin slide ±2).
    assert!((55..=65).contains(&s1), "path1 sight ack at {s1} (live 61) — the reaction floor moved");
    assert!(shot1 >= s1 + 59, "path1 first shot {shot1} within the reaction floor of its sight tick {s1}");
    assert!(shot1 >= noticed.unwrap(), "path1 first shot {shot1} before notice {:?}", noticed);
    let aim1 = g.log.iter().filter(|(f, _)| *f > WARM).find(|(_, st)| *st == nf_game::drone::state::AIM_STAND_FIRE).map(|(f, _)| f - WARM);
    if let Some(a1) = aim1 {
        assert!(shot1 >= a1 && shot1 - a1 < 59, "path1 aim entry {a1} -> shot {shot1}: an extra pre-fire pause leaked in");
    }

    // ---- Path 2: the alarm-raised guard (alertness 1) spots Bond — no reaction floor ------------------------------
    let (alert_drones, gi2, alert_rng) = l.drones_seeded(Some(1.0), GUARD_X, 1);
    let mut sim2 = l.sim(alert_drones, alert_rng);
    for _ in 0..WARM { sim2.tick(&d, &away); }
    let mut seen_at2 = None;
    for _ in 0..RUN {
        sim2.tick(&d, &player);
        if seen_at2.is_none() && sim2.list[gi2].seen { seen_at2 = Some(sim2.frame - WARM); }
    }
    let g2 = &sim2.list[gi2];
    let noticed2 = attack_tick(g2, WARM);
    let first_shot2 = g2.shots.first().map(|f| f - WARM);
    let first_move2 = g2.log.iter().filter(|(f, _)| *f > WARM).nth(1).map(|(_, s)| *s);
    eprintln!("[table] path2 alert-1.0 sight: noticed(ATTACK) {noticed2:?}, first_move {first_move2:?}, first_shot {first_shot2:?}, seen_at {seen_at2:?}");
    eprintln!("[trace] path2 full log: {:?}", g2.log);
    // The alerted guard self-attacks while Bond is away (the alarm path routes to ATTACK without a
    // sighting — `DroneVision_AlertSound` 0x177800:67-71, the forced-attack bits), so its ATTACK entry
    // sits in the warm-up: no 59-tick reaction floor applies on the sight ([M 0x147c48/0x148010 routes]).
    assert!(noticed2.is_none_or(|f| f + WARM <= WARM), "path2 self-attacked only at {noticed2:?}");
    let shot2 = first_shot2.expect("path2 fires within the window");
    let s2 = seen_at2.expect("path2 sees Bond");
    // No reaction floor on the alerted path: the sight ack lands within the round-robin wait (the fresh
    // guard's ack waits out DroneFunc_ReactionTime's 59; this guard's is 0 at alertness 1).
    assert!(s2 < 30, "path2 sight ack at {s2} — the fresh guard's 59-tick floor leaked into the alerted path");
    assert!(shot2 >= s2, "path2 first shot {shot2} before its sight tick {s2}");
    // The 100-frame step the guard was already playing runs out first (steps don't abort on sight,
    // [M 0x1676a0 case 3]) — so sight->shot here is step-phase noise, not a floor; the leak check that
    // matters: once AimStandFire is entered the bullet follows inside the aim ease (< 59), no pause.
    let aim2 = g2.log.iter().find(|(f, st)| *f > WARM && *st == nf_game::drone::state::AIM_STAND_FIRE).map(|(f, _)| f - WARM);
    let a2 = aim2.expect("path2 aims");
    assert!(shot2 >= a2 && shot2 - a2 < 59, "path2 aim entry {a2} -> shot {shot2}: an extra pre-fire pause leaked in");

    // ---- Path 3: mid-combat re-sight — the entry fire --------------------------------------------------------------
    // Path 1's sim, continued: Bond walks away (the guard loses sight), then re-appears.
    let mut sim3 = sim;
    let gone = bond_at([67.31, 26.5, 160.0]); // far behind cover, out of the guard's cone
    for _ in 0..160 { sim3.tick(&d, &gone); }
    let g3_pre_pos = sim3.list[gi].pos;
    let was_fighting = sim3.list[gi].log.iter().any(|(_, s)| *s == nf_game::drone::state::AIM_STAND_FIRE || *s == nf_game::drone::state::COMBAT_NO_SIGHT);
    let shot_before = sim3.list[gi].shots.len();
    for _ in 0..60 { sim3.tick(&d, &player); }
    let g3 = &sim3.list[gi];
    // The re-sight tick: the first ATTACK-family state after the away phase.
    let re_sight = g3.log.iter().rev().find(|(f, s)| *f > WARM + RUN + 160 - 60 && matches!(*s,
        nf_game::drone::state::ATTACK | nf_game::drone::state::AIM_STAND_FIRE | nf_game::drone::state::COMBAT_NEW_SIGHTING | nf_game::drone::state::COMBAT_NO_SIGHT))
        .map(|(f, _)| f - WARM);
    let aim_entry3 = g3.log.iter().filter(|(f, _)| *f > WARM + RUN + 130)
        .find(|(_, st)| *st == nf_game::drone::state::AIM_STAND_FIRE).map(|(f, _)| f - WARM);
    let shot3 = g3.shots.get(shot_before).map(|f| f - WARM);
    // The re-sight chain (NewSighting -> AimStandFire) is the game's entry fire [M 0x166328:59-61] —
    // 1-2 ticks; at dist <= 4.0 CanBackoff routes to AimBackoff instead (0x166328:96-102), so this
    // spot's guard re-aims and backs off before any bullet (the search phase walked it to ~2 units).
    let backoff = g3.log.iter().any(|(f, st)| *f > WARM + RUN + 130 && *st == nf_game::drone::state::AIM_BACKOFF);
    eprintln!("[table] path3 mid-combat re-sight: was_fighting {was_fighting}, re_sight(ATTACK-family) {re_sight:?}, aim_entry {aim_entry3:?}, backoff {backoff}, next_shot {shot3:?}, pos_after_away {g3_pre_pos:?}");
    if let (Some(r), Some(a)) = (re_sight, aim_entry3) {
        assert!(a >= r && a - r <= 4, "path3 re-sight {r} -> AimStandFire {a}: the entry chain is 1-4 ticks in the game");
    }
}
