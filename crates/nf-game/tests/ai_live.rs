//! Guard AI against live PCSX2 traces (`tools/trace/ai_trace.py`) in 07000005:
//! - WORKLOG 0019/0020: Bond appearing 6 units from the guard at (61.34, 99.50) is noticed after 61 ticks and fired at
//!   in 3-shot bursts, 4 ticks apart;
//! - WORKLOG 0021: the guard at (50.85, 28.27, 93.63) patrols PatrolPath 1 (work/patrol_trace.tsv).
//! Opt-in: set `NF_ISO` to the disc image.

use nf_formats::anim::Script;
use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, drone, level::Level, levelbin::LevelBin, weapon::WeaponData};
use nf_game::drone::{Data, Drone, Drones, Player, Rng};
use std::collections::HashMap;

/// Everything `Data` borrows, owned.
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
        let covers = nf_game::cover::build(&level.maps[m].placements, &nav, &world);
        Some(Level05 {
            tables: drone::DroneAnimData::from_action_elf(&elf).unwrap(),
            props: drone::PropertyTables::from_action_elf(&elf).unwrap(),
            weapons: WeaponData::from_action_elf(&elf).unwrap(),
            sequences: disc::level_sequences(&iso, "07000005.bin").unwrap(),
            level, scripts, world, skeletons, nav, covers,
        })
    }

    /// The simulation over these drones with the level's triggers.
    fn sim(&self, drones: Vec<Drone>, rng: Rng) -> Drones { let mut s = Drones::new(drones, rng); s.attach_level(&self.level); s }

    fn data(&self) -> Data<'_> {
        Data { tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world, level: 0x0700_0005,
            sequences: &self.sequences, skins: &self.level.skins, skeletons: &self.skeletons, nav: &self.nav, covers: &self.covers }
    }

    /// Every drone present at level start (line of sight is scheduled round robin over all of them, as in the game),
    /// optionally at a fixed alertness; and the index of the drone placed at `x`.
    fn drones(&self, alert: Option<f32>, x: f32) -> (Vec<Drone>, usize, Rng) { self.drones_seeded(alert, x, 1) }

    /// The same with another seed for the random stand-in (`Rand_*` is not modelled).
    fn drones_seeded(&self, alert: Option<f32>, x: f32, seed: u32) -> (Vec<Drone>, usize, Rng) {
        let d = self.data();
        let mut rng = Rng::new(seed);
        let mut drones = nf_game::drone::level_drones(&d, &self.level, &mut rng);
        if let Some(a) = alert { for dr in &mut drones { dr.alert = a; dr.alert_floor = a; } }
        let found = drones.iter().position(|dr| (dr.pos[0] - x).abs() < 0.01).unwrap();
        (drones, found, rng)
    }
}

fn bond_at(p: [f32; 3]) -> Player { Player { pos: p, target: [p[0], p[1] + 0.5, p[2]], eye: [p[0], p[1] + 1.0, p[2]], aim: [-1.0, 0.0, 0.0] } }

#[test]
fn guard_notices_and_fires_like_the_game() {
    let Some(l) = Level05::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    // Seed 2: the first combat-move roll picks the step (strafe or step are equally likely; the trace shows steps).
    let (drones, gi, rng) = l.drones_seeded(None, 61.342, 2);
    let mut sim = l.sim(drones, rng);
    // As in the live session, Bond appears (teleported) after the level has run: 10 ticks with him far away first, so
    // spawner 266's drone is already in the list.
    const WARM: u32 = 10;
    let away = bond_at([0.0, -500.0, 0.0]);
    for _ in 0..WARM { sim.tick(&d, &away); }
    assert_eq!(sim.list.iter().filter(|x| x.present).count(), 13, "live count at level start (spawner 266 brought one in)");
    let player = bond_at([67.31, 26.5, 99.47]);
    for _ in 0..400 { sim.tick(&d, &player); }
    let g = &sim.list[gi];
    let attack = g.log.iter().find(|(_, s)| *s == nf_game::drone::state::ATTACK).map(|(f, _)| *f - WARM);
    // Reaction time 59 (chopped arithmetic) plus the wait for the guard's first round-robin line-of-sight turn, which
    // depends on how many other drones see Bond: live 61 (WORKLOG 0019), here 59–63.
    assert!(attack.is_some_and(|f| (59..=63).contains(&f)), "noticed after {attack:?} ticks (live 61)");
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
    let (alert_drones, _, alert_rng) = l.drones_seeded(Some(1.0), 61.342, 2);
    let mut alert_sim = l.sim(alert_drones, alert_rng);
    for _ in 0..WARM { alert_sim.tick(&d, &away); }
    for _ in 0..100 { alert_sim.tick(&d, &player); }
    let early = alert_sim.list[gi].log.iter().find(|(_, s)| *s == nf_game::drone::state::ATTACK).map(|(f, _)| *f - WARM);
    assert!(early.is_some_and(|f| f < 59), "sister: alertness 1 must react earlier, got {early:?}");
}

/// Level-start snapshot (work/ee_start_05.bin, `drones_snapshot.py`): 8 drones wait for switch channels 0x22 / 0xA4 /
/// 0x43 (state 1, object flags 0x30), the others are Idle / Patrol; the first behaviour block (`+0x4DC`) holds the
/// placement words changed by `DoModeSettingsNEW` / `NDrone2_DefaultInit` / Idle.
#[test]
fn level_start_matches_the_game() {
    let Some(l) = Level05::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let (drones, _, rng) = l.drones(None, 61.342);
    let m = l.level.main_map().unwrap();
    let placement_words = |x: f32| {
        let p = l.level.maps[m].placements.iter().find(|p| p.flags & 0xff == 0x0f && (p.translation[0] - x).abs() < 0.01).unwrap();
        drone::behaviour_words(|k: u32| p.params.iter().find(|(i, _)| *i == k).map(|&(_, v)| v).unwrap_or(0)).0
    };
    let mut sim = l.sim(drones, rng);
    sim.tick(&d, &bond_at([104.95, 24.96, 108.83]));
    // (x, state, first block) from the snapshot; the drone at (62.20, 104.15) (state 0x29) is not compared.
    let live: [(f32, u8, [u32; 3]); 12] = [
        (61.341, 4, [0x8212_94c7, 0xecf3_77e5, 0x4029_1670]), (50.850, 7, [0xb012_95c7, 0xdcf3_77a5, 0x4029_14f2]),
        (150.216, 4, [0x8212_94c7, 0xecf3_77e5, 0x4029_1670]), (99.686, 4, [0x8212_9484, 0xecf3_77e5, 0x4029_1660]),
        (-31.296, 1, [0x3012_95c7, 0xdcf3_77a5, 0x4029_14f2]), (-33.532, 1, [0xa212_bcc7, 0x8cf3_7077, 0x4029_1670]),
        (-33.495, 1, [0xa212_bcc7, 0x8cf3_7077, 0x4029_1670]), (-32.767, 1, [0x3012_95c7, 0xdcf3_77a5, 0x4029_14f2]),
        (41.524, 1, [0xa212_94c7, 0xecf3_77e7, 0x4029_1670]), (46.407, 1, [0xa212_94c7, 0xecf3_77e7, 0x4029_1670]),
        (32.435, 1, [0xa212_94c7, 0xecf3_77e5, 0x4029_1670]), (27.578, 1, [0xa212_94c7, 0xecf3_77e5, 0x4029_1670]),
    ];
    let mut differs_raw = 0;
    for (x, state, words) in live {
        let dr = sim.list.iter().find(|dr| (dr.pos[0] - x).abs() < 0.01).unwrap_or_else(|| panic!("no drone at x {x}"));
        assert_eq!(dr.state, state, "state of the drone at x {x}");
        assert_eq!(dr.enabled, state != 1, "enabled at x {x}");
        assert_eq!(dr.props, words, "first behaviour block at x {x}: {:x?} vs {words:x?}", dr.props);
        differs_raw += (placement_words(x) != words) as u32;
    }
    // Sister: the placement words alone do not match, so the comparison can fail.
    assert!(differs_raw >= 10, "sister: only {differs_raw} drones differ from their placement words");
}

/// Live trace (work/patrol_trace.tsv, Bond standing at his start (104.95, 24.96, 108.83)): the guard walks
/// PatrolPath 1 from node 20 without stopping (anim state 0x37 throughout); its positions every 300 samples (the trace
/// starts ~11 ticks after the level).
#[test]
fn guard_patrols_like_the_game() {
    let Some(l) = Level05::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let (drones, gi, rng) = l.drones(None, 50.849);
    let mut sim = l.sim(drones, rng);
    let player = bond_at([104.95, 24.96, 108.83]);
    let route = |s: &Drones| s.list[gi].route.as_ref().map(|r| r.nodes.clone());
    sim.tick(&d, &player);
    assert_eq!(sim.list[gi].state, nf_game::drone::state::PATROL);
    // `InitAIPath` order from node 20 (the first link of a patrol path's start node is skipped).
    let expect: Vec<u16> = [20, 26, 21, 22, 23, 24, 25, 0].into_iter().chain(1..=19).collect();
    assert_eq!(route(&sim), Some(expect));
    let live = [(300, [53.3023, 93.4941]), (600, [56.0890, 93.9130]), (900, [58.8470, 94.1405]), (1200, [61.6508, 94.3452]),
        (1500, [64.4133, 93.7609]), (1800, [63.7538, 94.2367]), (2100, [61.0325, 94.5154])];
    let mut at = HashMap::new();
    for t in 2..=2200 {
        sim.tick(&d, &player);
        at.insert(t, sim.list[gi].pos);
        assert_eq!(sim.list[gi].state, nf_game::drone::state::PATROL, "tick {t}");
    }
    for (sample, xz) in live {
        let p = at[&(sample + 11)];
        let err = ((p[0] - xz[0]).powi(2) + (p[2] - xz[1]).powi(2)).sqrt();
        assert!(err < 0.35, "sample {sample}: port ({:.2}, {:.2}) vs live ({:.2}, {:.2}), {err:.2} apart", p[0], p[2], xz[0], xz[1]);
        // Sister: 60 ticks earlier the guard is measurably elsewhere, so the tolerance can fail.
        let q = at[&(sample + 11 - 60)];
        assert!(((q[0] - xz[0]).powi(2) + (q[2] - xz[1]).powi(2)).sqrt() > 0.35, "sister at sample {sample}");
    }
}

/// Live trace (work/chase_trace.tsv: Bond teleported to (79.62, 22.6, 95.96), settling at (79.86, 22.92, 96.0)): the
/// gate guard notices him, goes Combat → OutOfRange → RunForCover (0x8C) → UnderCoverInit (31 ticks) → UnderCoverIdle →
/// UnderCoverAim (37 ticks, lean-out 176 at speed 1.5) → UnderCoverFire behind corner node 1 (59.11, 98.69), firing from
/// (58.83, 99.32). Seed 3 makes the port's rolls pick no first-sight move and the lean cover type, as the game did here.
#[test]
fn guard_takes_cover_like_the_game() {
    let Some(l) = Level05::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let (drones, gi, rng) = l.drones_seeded(None, 61.342, 3);
    let mut sim = l.sim(drones, rng);
    let player = bond_at([79.86, 22.92, 96.0]);
    for _ in 0..700 { sim.tick(&d, &player); }
    let g = &sim.list[gi];
    let states: Vec<u8> = g.log.iter().map(|(_, s)| *s).filter(|s| *s >= 0x58).collect();
    assert_eq!(&states[..7], &[0x58, 0x5a, 0x8c, 0x93, 0x94, 0x95, 0x96], "{:x?}", g.log);
    let at = |s: u8| g.log.iter().find(|(_, x)| *x == s).map(|(f, _)| *f).unwrap();
    assert!((28..=34).contains(&(at(0x94) - at(0x93))), "UnderCoverInit lasted {} (live 31)", at(0x94) - at(0x93));
    assert!((32..=40).contains(&(at(0x96) - at(0x95))), "UnderCoverAim lasted {} (live 37)", at(0x96) - at(0x95));
    assert_eq!(g.cover, Some(1), "corner node 1");
    // Live in UnderCoverFire (samples 431–900): (58.83, 99.32).
    let err = ((g.pos[0] - 58.83).powi(2) + (g.pos[2] - 99.32).powi(2)).sqrt();
    assert!(err < 0.4, "in cover at ({:.2}, {:.2}), {err:.2} from the live position", g.pos[0], g.pos[2]);
    // Sister: with Bond 6 units away (the first test's setup) the same guard never runs for cover.
    let (drones, gi, rng) = l.drones_seeded(None, 61.342, 3);
    let mut near = l.sim(drones, rng);
    for _ in 0..300 { near.tick(&d, &bond_at([67.31, 26.5, 99.47])); }
    assert!(near.list[gi].log.iter().all(|(_, s)| *s != 0x8c), "sister: cover must depend on the situation");
}

/// Live trace (work/switch_trace.tsv): right after the level loads, switch channel 0x22 is set by PINE; the Dr_17 guard
/// at (−31.30, 37.58, −37.05) leaves WaitSwitch and patrols PatrolPath (Dr_17) from node 10. Bond stands at his start
/// facing yaw −2.147; the guard is not drawn there (`obj+0xFC` = 0) so it does not animate: it slides 0.02 per frame
/// (walk 392's speed). The trace starts ~10 ticks after the port's tick 0.
#[test]
fn switched_guard_patrols_like_the_game() {
    let Some(l) = Level05::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let (drones, gi, rng) = l.drones(None, -31.296);
    let mut sim = l.sim(drones, rng);
    sim.set_channel(0x22);
    let y = -2.1467f32;
    let p = [104.95, 24.96, 108.83];
    let player = Player { pos: p, target: [p[0], p[1] + 0.5, p[2]], eye: [p[0], p[1] + 1.0, p[2]], aim: [y.sin(), 0.0, y.cos()] };
    let mut at = Vec::new();
    for _ in 0..1500 { sim.tick(&d, &player); at.push(sim.list[gi].pos); }
    assert_eq!(sim.list[gi].state, nf_game::drone::state::PATROL);
    assert!(!sim.list[gi].drawn);
    let live = [(150, [-31.55, -35.04]), (300, [-31.62, -32.04]), (450, [-31.57, -29.04]), (600, [-31.49, -26.04]),
        (750, [-32.01, -24.80]), (900, [-32.49, -27.75]), (1050, [-32.64, -30.74]), (1200, [-32.70, -33.74])];
    for (sample, xz) in live {
        let q = at[sample + 10 - 1];
        let err = ((q[0] - xz[0]).powi(2) + (q[2] - xz[1]).powi(2)).sqrt();
        assert!(err < 0.3, "sample {sample}: port ({:.2}, {:.2}) vs live ({:.2}, {:.2})", q[0], q[2], xz[0], xz[1]);
    }
    // Sister: drawn (animated with the walk's root motion) it would be about half as far along.
    let (drones, gi, rng) = l.drones(None, -31.296);
    let mut drawn = l.sim(drones, rng);
    drawn.set_channel(0x22);
    for _ in 0..610 {
        // An observer right at the guard: always drawn (within 2 units).
        let g = drawn.list[gi].pos;
        drawn.tick(&d, &Player { eye: [g[0], g[1] + 1.0, g[2]], ..player });
        assert!(drawn.list[gi].drawn);
    }
    let q = drawn.list[gi].pos;
    assert!(((q[0] + 31.49).powi(2) + (q[2] + 26.04).powi(2)).sqrt() > 2.0, "sister: a drawn guard walks slower ({:.2}, {:.2})", q[0], q[2]);
}

/// Live (WORKLOG 0027): Bond walked through TouchOnce box 21 (x 60.1, z 91 → 101): channels 0x23 and (mux 25) 0x22
/// latched, the two Dr_17 guards (wait channel 0x22) left WaitSwitch for Patrol, and spawner 268 (start 0x22) brought
/// in both gate guards (placements 3 and 4). (Standing still after a teleport into the box set nothing: the game only
/// takes contacts from movement; the port tests positions.)
#[test]
fn gate_box_wakes_the_guards_like_the_game() {
    let Some(l) = Level05::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let (drones, _, rng) = l.drones(None, 61.342);
    let mut sim = l.sim(drones, rng);
    for _ in 0..10 { sim.tick(&d, &bond_at([104.95, 24.96, 108.83])); }
    let state_of = |s: &Drones, placement: usize| s.list.iter().find(|x| x.placement == placement).map(|x| (x.present, x.state)).unwrap();
    assert_eq!(state_of(&sim, 65), (true, nf_game::drone::state::WAIT_SWITCH));
    assert!(!state_of(&sim, 3).0 && !sim.channels.on(0x22), "sister: nothing before Bond enters the box");
    for i in 0..40 { sim.tick(&d, &bond_at([60.1, 26.6, 91.0 + i as f32 * 0.25])); }
    for _ in 0..30 { sim.tick(&d, &bond_at([60.1, 26.6, 101.0])); }
    assert!(sim.channels.on(0x23) && sim.channels.on(0x22));
    assert_eq!(state_of(&sim, 65), (true, nf_game::drone::state::PATROL));
    assert_eq!(state_of(&sim, 68), (true, nf_game::drone::state::PATROL));
    assert!(state_of(&sim, 3).0 && state_of(&sim, 4).0, "spawner 268 brought in the gate guards");
}
