//! The 0700000c guards' AI and walking (guards-0c): playtests 1012-1723 / 1012-2105 — "All of the AI
//! in this level is bugged as hell. The walking animation is kind of fucked too." Lists every 0c
//! drone at start and over 1800 ticks: class/mode, state transitions, route, the anim state/danim
//! and fractional frame per tick; per drone the walked distance, the anim's frame advance, and the
//! anim-rate-vs-speed slide metric (the walk danim's natural ground speed = its root-motion table
//! total over the script length — a mismatch is the foot slide). Opt-in: set `NF_ISO`.

use nf_formats::anim::Script;
use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, drone, level::Level, levelbin::LevelBin, weapon::WeaponData};
use nf_game::drone::{state, Data, Drone, Drones, Player, Rng};
use std::collections::HashMap;

struct Level0c {
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

impl Level0c {
    fn load() -> Option<Level0c> {
        let path = std::env::var_os("NF_ISO")?;
        let iso = IsoImage::open(&path).unwrap();
        let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
        let (entries, bin) = disc::files_bin_entries(&iso).unwrap();
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("0700000c.bin")).unwrap();
        let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
        let level = Level::parse(&data).unwrap();
        let lb = LevelBin::parse(&data).unwrap();
        let scripts = lb.entries.iter().filter(|e| e.kind == 5).filter_map(|e| Script::parse(lb.data(&data, e)).ok()).map(|s| (s.hashcode, s)).collect();
        let world = nf_game::world::CollisionWorld::build(&level, &level.instances(true).0);
        let skeletons = lb.entries.iter().filter(|e| e.kind == 6).filter_map(|e| nf_formats::character::parse_skeleton(lb.data(&data, e)).ok()).map(|k| (k.id as u8, k)).collect();
        let m = level.main_map().unwrap();
        let nav = nf_game::nav::Network::build(level.maps[m].ai_network.as_ref().unwrap(), &world, 0x0700_000c);
        let covers = nf_game::cover::build(&level, &nav, &world);
        let volumes = nf_game::aivolume::build(&level, 0x0700_000c);
        Some(Level0c {
            tables: drone::DroneAnimData::from_action_elf(&elf).unwrap(),
            props: drone::PropertyTables::from_action_elf(&elf).unwrap(),
            weapons: WeaponData::from_action_elf(&elf).unwrap(),
            sfx: nf_formats::sfx::SfxTable::from_action_elf(&elf).unwrap(),
            tuning: { let mut t = nf_formats::tuning::Tuning::from_action_elf(&elf).unwrap(); nf_game::tuning::read_tuning_vars(&mut t, 0x0700_000c); t },
            hitboxes: nf_formats::hitbox::CollisionBones::from_action_elf(&elf).unwrap(),
            sequences: disc::level_sequences(&iso, "0700000c.bin").unwrap(),
            models: nf_game::pickup::models(&level),
            hud: nf_formats::hud::HudTables::from_action_elf(&elf).unwrap(),
            surfaces: nf_formats::sfx::SurfaceSounds::from_action_elf(&elf).unwrap(),
            voices: nf_formats::sfx::voice_lines(&elf).unwrap(),
            level, scripts, world, skeletons, nav, covers, volumes,
        })
    }

    fn data(&self) -> Data<'_> {
        Data { tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world, level: 0x0700_000c,
            sequences: &self.sequences, skins: &self.level.skins, skeletons: &self.skeletons, nav: &self.nav, covers: &self.covers, tuning: &self.tuning, hitboxes: &self.hitboxes, volumes: &self.volumes,
            sfx: Some(&self.sfx), models: &self.models, surfaces: Some(&self.surfaces), voices: Some(&self.voices), difficulty: 1 }
    }
}

fn bond_far(d: &Drone) -> Player {
    // Park Bond far from every guard so the patrols run undisturbed.
    Player { pos: [d.pos[0], d.pos[1] + 2.0, d.pos[2]], target: [d.pos[0], d.pos[1] + 2.5, d.pos[2]], eye: [d.pos[0], d.pos[1] + 3.0, d.pos[2]], aim: [0.0, 0.0, 1.0], ..Default::default() }
}

#[test]
fn patrol_trace() {
    let Some(l) = Level0c::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let mut rng = Rng::new(0x1234_5678);
    let mut lights = nf_game::light::LightList::default();
    let drones = nf_game::drone::level_drones(&d, &l.level, &mut rng, &mut lights);
    eprintln!("== drones at start: {}", drones.len());
    for (i, dr) in drones.iter().enumerate() {
        eprintln!("  #{i}: class {:#x} mode {:#x} present {} corpse {} pos ({:.1},{:.1},{:.1}) route {} croute {} sm_id {} anim.state {:#x} anim.danim {:#x}",
            dr.kind, dr.mode, dr.present, dr.corpse, dr.pos[0], dr.pos[1], dr.pos[2],
            dr.route.is_some(), dr.croute.is_some(), dr.sm_id, dr.anim.state, dr.anim.danim);
    }
    let mut sim = Drones::new(drones, rng);
    sim.attach_level(&l.level);
    sim.difficulty = 1;
    let bond = bond_far(sim.list.first().unwrap());
    // Per-drone accumulators.
    let n = sim.list.len();
    let mut last_pos: Vec<[f32; 3]> = sim.list.iter().map(|dr| dr.pos).collect();
    let mut dist: Vec<f32> = vec![0.0; n];
    let mut anim_adv: Vec<f32> = vec![0.0; n];
    let mut last_ftime: Vec<f32> = sim.list.iter().map(|dr| dr.anim.ftime).collect();
    let mut states: Vec<Vec<(u8, u32)>> = vec![Vec::new(); n]; // (state, first tick) transitions
    let mut anims: Vec<Vec<(u16, u32)>> = vec![Vec::new(); n]; // (danim, first tick)
    let mut last_state: Vec<u8> = sim.list.iter().map(|dr| dr.state).collect();
    let mut last_danim: Vec<u16> = sim.list.iter().map(|dr| dr.anim.danim).collect();
    let mut walk_ticks: Vec<u32> = vec![0; n];
    let mut move_ticks: Vec<u32> = vec![0; n];
    let mut gone: Vec<(usize, u32, u8)> = Vec::new();
    for t in 0..1800u32 {
        sim.tick(&d, &bond);
        for i in 0..n {
            let dr = &sim.list[i];
            if !dr.present && !gone.iter().any(|(g, _, _)| *g == i) { gone.push((i, t, dr.state)); }
            if !dr.present || dr.corpse { continue; }
            let dd = ((dr.pos[0] - last_pos[i][0]).powi(2) + (dr.pos[1] - last_pos[i][1]).powi(2) + (dr.pos[2] - last_pos[i][2]).powi(2)).sqrt();
            // The anim-rate-vs-speed metric accumulates DRAWN ticks only: the undrawn slide (the
            // game's own [M 0x149f88] no-gait 0.02 crawl) is not a walk-anim-rate question.
            let drawn = dr.drawn;
            if drawn { dist[i] += dd; }
            if dd > 0.001 && drawn { move_ticks[i] += 1; }
            last_pos[i] = dr.pos;
            // The anim's fractional frame advance (wrapping: new danim restarts at 0/1).
            if dr.anim.danim == last_danim[i] {
                let adv = dr.anim.ftime - last_ftime[i];
                if adv > -10.0 && adv < 10.0 && drawn { anim_adv[i] += adv; }
            }
            last_ftime[i] = dr.anim.ftime;
            last_danim[i] = dr.anim.danim;
            if dr.state != last_state[i] {
                states[i].push((dr.state, t));
                last_state[i] = dr.state;
            }
            if dr.anim.danim != anims[i].last().map_or(u16::MAX, |a| a.0) {
                anims[i].push((dr.anim.danim, t));
            }
            // A walking danim playing while the guard moves.
            if matches!(dr.anim.state, 0x37 | 0x38 | 0x39 | 0x35 | 0x0f) && dd > 0.001 { walk_ticks[i] += 1; }
            if i == 39 && (710..716).contains(&t) {
                eprintln!("    t{t} #{i}: drawn {} enabled {} ast {:#x} danim {:#x} ftime {:.2} time {} started {} len {:?} dd {dd:.4}",
                    dr.drawn, dr.enabled, dr.anim.state, dr.anim.danim, dr.anim.ftime, dr.anim.time, dr.anim.started,
                    dr.anim.script.and_then(|h| d.scripts.get(&h)).map(|s| (s.length, s.looping)));
            }
        }
    }
    eprintln!("== went not-present: {:?}", gone);
    eprintln!("== after 1800 ticks:");
    // The walk anim is tied to the speed for every guard that MOVED: while drawn the danim's
    // ftime advances at the script speed (1.0) and the per-tick displacement is the walk's root
    // motion (the feet match the ground); the 0.02 no-gait slide happens only for the guards the
    // game itself undraws (the frustum+room gate — [M NDrone2_DoAnimation 0x149f88, the danim's
    // Drone_AnimInfo+4 speed; [L work/switch_trace.tsv]). The drawn guards' anim must have
    // advanced with their motion: anim frames per unit walked within the walk's natural band.
    for i in 0..n {
        let dr = &sim.list[i];
        if !dr.present || dist[i] < 2.0 { continue; }
        let per_unit = anim_adv[i] / dist[i];
        eprintln!("  walk #{i}: {per_unit:.1} anim frames per unit walked (drawn walk's root band 60..130)");
        assert!(per_unit > 40.0, "guard #{i}: the walk anim must advance with the motion, got {per_unit:.1} frames/unit over {:.1} units", dist[i]);
        assert!(matches!(dr.state, state::PATROL | state::IDLE), "guard #{i} left its post state: {:#x}", dr.state);
    }
    for i in 0..n {
        let dr = &sim.list[i];
        if !dr.present { continue; }
        let natural = walk_natural_speed(&d, &l, dr);
        eprintln!("  #{i}: mode {:#x} state {:#x} anim.state {:#x} danim {:#x} route {} dist {:.1} move_ticks {} walk_ticks {} anim_adv {:.1} natural_speed {:.3}",
            dr.mode, dr.state, dr.anim.state, dr.anim.danim, dr.route.is_some(), dist[i], move_ticks[i], walk_ticks[i], anim_adv[i], natural);
        if !states[i].is_empty() {
            let hist: String = states[i].iter().map(|(s, t)| format!("{s:#x}@{t} ")).collect();
            eprintln!("      states: {hist}");
        }
        if anims[i].len() > 1 && anims[i].len() < 25 {
            let a: String = anims[i].iter().map(|(s, t)| format!("{s:#x}@{t} ")).collect();
            eprintln!("      danims: {a}");
        } else if anims[i].len() >= 25 {
            eprintln!("      danims: {} distinct", anims[i].len());
        }
    }
}

/// The walk danim's natural ground speed: its distance table total over the script length
/// (units per tick at speed 1.0) — the anim-rate-vs-move-speed slide reference.
fn walk_natural_speed(d: &Data, l: &Level0c, dr: &Drone) -> f32 {
    let Some(script) = dr.anim.script.and_then(|s| d.scripts.get(&s)) else { return 0.0 };
    let Some(seq) = script.sequences().find_map(|(_, _, h)| l.sequences.get(&h)) else { return 0.0 };
    let Some(dt) = nf_game::anim::DistanceTable::build(seq, 4, true) else { return 0.0 };
    if script.length <= 0 { return 0.0; }
    dt.total / script.length as f32
}
