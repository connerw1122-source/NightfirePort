//! Guard AI against a live PCSX2 trace (WORKLOG 0019, `tools/trace/ai_trace.py`): in 07000005, Bond appearing 6 units
//! from the guard at (61.34, 99.50) is noticed after 61 ticks and fired at in 3-shot bursts, 4 ticks apart.
//! Opt-in: set `NF_ISO` to the disc image.

use nf_formats::anim::Script;
use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, drone, level::Level, levelbin::LevelBin, weapon::WeaponData};
use nf_game::drone::{Data, Drone, Drones, Player, Rng};
use std::collections::HashMap;

#[test]
fn guard_notices_and_fires_like_the_game() {
    let Some(path) = std::env::var_os("NF_ISO") else { eprintln!("skip: NF_ISO not set"); return };
    let iso = IsoImage::open(&path).unwrap();
    let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
    let (tables, props, weapons) = (drone::DroneAnimData::from_action_elf(&elf).unwrap(), drone::PropertyTables::from_action_elf(&elf).unwrap(), WeaponData::from_action_elf(&elf).unwrap());
    let (entries, bin) = disc::files_bin_entries(&iso).unwrap();
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000005.bin")).unwrap();
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    let level = Level::parse(&data).unwrap();
    let lb = LevelBin::parse(&data).unwrap();
    let scripts: HashMap<u32, Script> = lb.entries.iter().filter(|e| e.kind == 5).filter_map(|e| Script::parse(lb.data(&data, e)).ok()).map(|s| (s.hashcode, s)).collect();
    let world = nf_game::world::CollisionWorld::build(&level, &level.instances(true).0);
    let sequences = disc::level_sequences(&iso, "07000005.bin").unwrap();
    let skeletons: HashMap<u8, nf_formats::character::Skeleton> = lb.entries.iter().filter(|e| e.kind == 6).filter_map(|e| nf_formats::character::parse_skeleton(lb.data(&data, e)).ok()).map(|k| (k.id as u8, k)).collect();
    let d = Data { tables: &tables, props: &props, scripts: &scripts, weapons: &weapons, world: &world, level: 0x0700_0005, sequences: &sequences, skins: &level.skins, skeletons: &skeletons };
    // Every drone present at level start (line of sight is scheduled round robin over all of them, as in the game).
    let m = level.main_map().unwrap();
    let list: Vec<(u32, Vec<(u32, u32)>)> = level.maps[m].placements.iter().map(|p| (p.flags & 0xff, p.params.clone())).collect();
    let build = |alert: Option<f32>| {
        let mut rng = Rng::new(1);
        let (mut drones, mut guard) = (Vec::new(), None);
        for (p, role) in level.maps[m].placements.iter().filter(|p| p.flags & 0xff == 0x0f).zip(drone::spawn_roles(&list)) {
            if matches!(role, drone::SpawnRole::Reinforcement { .. }) { continue; }
            let param = |k: u32| p.params.iter().find(|(i, _)| *i == k).map(|&(_, v)| v).unwrap_or(0);
            let floor = world.floor_below(p.translation, 3.0).unwrap_or(p.translation[1] - 1.0);
            if (p.translation[0] - 61.342).abs() < 0.01 { guard = Some(drones.len()); }
            let mut x = Drone::new(&d, [p.translation[0], floor + 1.0, p.translation[2]], p.rotation[1], param, &mut rng);
            if let Some(a) = alert { x.alert = a; x.alert_floor = a; }
            drones.push(x);
        }
        (drones, guard.unwrap(), rng)
    };
    let (drones, gi, rng) = build(None);
    assert_eq!(drones.len(), 13, "live count at level start");
    let mut sim = Drones::new(drones, rng);
    let bond = [67.31, 26.5, 99.47];
    let player = Player { pos: bond, target: [bond[0], bond[1] + 0.5, bond[2]], eye: [bond[0], bond[1] + 1.0, bond[2]], aim: [-1.0, 0.0, 0.0] };
    for _ in 0..400 { sim.tick(&d, &player); }
    let g = &sim.list[gi];
    let attack = g.log.iter().find(|(_, s)| *s == nf_game::drone::state::ATTACK).map(|(f, _)| *f);
    assert_eq!(attack, Some(61), "noticed after the game's 61 ticks (reaction time 0.3 × 200 at alertness 0)");
    assert!(g.shots.len() >= 6);
    assert_eq!((g.shots[1] - g.shots[0], g.shots[2] - g.shots[1]), (4, 4), "SG5: weapon_data +0x40 = 4 ticks");
    assert!(g.shots[3] - g.shots[2] >= 4 + 15, "burst delay ≥ 15 + interval");
    assert_eq!(g.clip, 64 - g.shots.len() as i32);
    // Combat moves (trace work/ai_trace_b.tsv): with Bond's crosshair on him the guard's only move is a step
    // (prop 0x42): Attack → StepAimLeft/Right, which lasts its animation (live: 101 and 113 ticks), then AimStandFire.
    let after_attack = g.log.iter().skip_while(|(_, s)| *s != nf_game::drone::state::ATTACK).nth(1).copied();
    let (step_at, step) = after_attack.unwrap();
    assert!(matches!(step, 0x73 | 0x74), "first move is a step, got {step:#x}");
    let step_end = g.log.iter().find(|(f, s)| *f > step_at && *s == nf_game::drone::state::AIM_STAND_FIRE).map(|(f, _)| *f).unwrap();
    assert!((100..=115).contains(&(step_end - step_at)), "step lasted {} ticks", step_end - step_at);
    // Damage (live health column): an SG5 hit on Bond costs 2.75 × 0.6 = 1.65 at difficulty 1.
    let guard_hits: Vec<f32> = sim.player_hits.iter().filter(|(_, o, _)| *o == gi).map(|(_, _, h)| *h).collect();
    assert!(!guard_hits.is_empty() && guard_hits.iter().all(|h| (h - 1.65).abs() < 1e-3), "SG5 hits {guard_hits:?}");
    // The sniper (L96A1, 30 × 0.6) hits for 18, as in the live trace.
    assert!(sim.player_hits.iter().filter(|(_, o, _)| *o != gi).all(|(_, _, h)| (h - 18.0).abs() < 1e-3 || (h - 1.65).abs() < 1e-3 || *h < 18.0));
    // Sister: the same guard already at alertness 1 reacts without the delay, so the comparison above can fail.
    let (alert_drones, _, alert_rng) = build(Some(1.0));
    let mut alert_sim = Drones::new(alert_drones, alert_rng);
    for _ in 0..100 { alert_sim.tick(&d, &player); }
    let early = alert_sim.list[gi].log.iter().find(|(_, s)| *s == nf_game::drone::state::ATTACK).map(|(f, _)| *f);
    assert!(early.is_some_and(|f| f < 61), "sister: alertness 1 must react earlier, got {early:?}");
}
