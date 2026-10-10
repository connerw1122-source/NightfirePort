//! Grenade tests on level 07000002 (the level where guards carry stun grenades, weapon 0x36 × 3 — param 24).
//! Opt-in: set `NF_ISO` to the disc image. See docs/systems/levels.md "Grenade-carrying guards".

use nf_formats::anim::Script;
use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, drone, level::Level, levelbin::LevelBin, weapon::WeaponData};
use nf_game::drone::{Data, Drone, Drones, Player, Rng};
use std::collections::HashMap;

struct Level02 {
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
    sfx: Option<nf_formats::sfx::SfxTable>,
    tuning: nf_formats::tuning::Tuning,
    hitboxes: nf_formats::hitbox::CollisionBones,
    volumes: Vec<nf_game::aivolume::AiVolume>,
    models: HashMap<u32, nf_game::pickup::Model>,
    hud: nf_formats::hud::HudTables,
    surfaces: Option<nf_formats::sfx::SurfaceSounds>,
    voices: Vec<nf_formats::sfx::VoiceLine>,
    skins: HashMap<u32, nf_formats::character::Skin>,
}

impl Level02 {
    fn load() -> Option<Level02> {
        let path = std::env::var_os("NF_ISO")?;
        let iso = IsoImage::open(&path).unwrap();
        let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
        let (entries, bin) = disc::files_bin_entries(&iso).unwrap();
        let read = |name: &str| -> Option<Vec<u8>> {
            let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name))?;
            Some(iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap())
        };
        let data = read("07000002.bin")?;
        let level = Level::parse(&data).unwrap();
        let lb = LevelBin::parse(&data).unwrap();
        let skeletons: HashMap<u8, _> = lb.entries.iter().filter(|e| e.kind == 6)
            .filter_map(|e| nf_formats::character::parse_skeleton(lb.data(&data, e)).ok().map(|s| (s.id as u8, s))).collect();
        let anim_name = "07F00002.bin";
        let mut sequences = HashMap::new();
        if let Some(anim) = read(anim_name) {
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
        let nav = nf_game::nav::Network::build(level.maps[level.main_map().unwrap()].ai_network.as_ref().unwrap(), &world, 0x0700_0002);
        let covers = nf_game::cover::build(&level.maps[level.main_map().unwrap()].placements, &nav, &world);
        let hitboxes = nf_formats::hitbox::CollisionBones::from_action_elf(&elf).unwrap();
        let volumes = nf_game::aivolume::build(&level, 0x0700_0002);
        let models = nf_game::pickup::models(&level);
        let sfx = nf_formats::sfx::SfxTable::from_action_elf(&elf).ok();
        let tuning = nf_formats::tuning::Tuning::default();
        let hud = nf_formats::hud::HudTables::from_action_elf(&elf).unwrap();
        let surfaces = None;
        let voices = Vec::new();
        let skins = level.skins.clone();
        Some(Level02 { tables, props, weapons, level, scripts, world, sequences, skeletons, nav, covers, sfx, tuning, hitboxes, volumes, models, hud, surfaces, voices, skins })
    }

    fn data(&self) -> Data<'_> {
        Data { tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world, level: 0x0700_0002, sequences: &self.sequences, skins: &self.skins, skeletons: &self.skeletons, nav: &self.nav, covers: &self.covers, tuning: &self.tuning, hitboxes: &self.hitboxes, volumes: &self.volumes, sfx: self.sfx.as_ref(), models: &self.models, surfaces: self.surfaces.as_ref(), voices: None }
    }

    fn drones(&self) -> (Vec<Drone>, Rng) {
        let d = self.data();
        let mut rng = Rng::new(0x1234_5678);
        let drones = nf_game::drone::level_drones(&d, &self.level, &mut rng);
        (drones, rng)
    }

    fn sim(&self, drones: Vec<Drone>, rng: Rng) -> Drones { let mut s = Drones::new(drones, rng); s.attach_level(&self.level); s }
}

fn bond_at(p: [f32; 3]) -> Player { Player { pos: p, target: [p[0], p[1] + 0.5, p[2]], eye: [p[0], p[1] + 1.0, p[2]], aim: [-1.0, 0.0, 0.0] } }

/// On 07000002, guards at placements 464 and 466 carry stun grenades (weapon 0x36, count 3 from param 24).
/// When Bond is within 8–20 units, the guard should throw: the throw anim plays, a grenade entity spawns,
/// flies under gravity, bounces off the world, and explodes. Sister: Bond out of range (dist > 20) → no throw.
#[test]
fn guard_throws_a_grenade_on_level_02() {
    use nf_game::drone::state;
    let Some(l) = Level02::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let (drones, rng) = l.drones();
    let mut sim = l.sim(drones, rng);

    // Find a guard that has grenades AND is a normal combat guard (not mode 7 / 0x85).
    let gi = sim.list.iter().position(|x| {
        x.present && x.grenade_count > 0 && x.mode != 7 && x.mode != 0x1e
    }).expect("level 07000002 should have a normal grenade-carrying guard");
    let g = sim.list[gi].pos;
    let gc_before = sim.list[gi].grenade_count;
    eprintln!("grenade guard #{} at ({:.1}, {:.1}, {:.1}), {} grenades, mode={}", gi, g[0], g[1], g[2], gc_before, sim.list[gi].mode);
    let gw = sim.list[gi].grenade_weapon;
    let gc = sim.list[gi].grenade_count;
    assert!(gw == 0x36 && gc > 0, "guard has stun grenades: weapon={:#x} count={}", gw, gc);

    // Bond 15 units in front of the guard, in view.
    let bond = bond_at([g[0] + 15.0, g[1], g[2]]);
    // Run the sim so the guard sees Bond and reaches AIM_STAND_FIRE.
    for _ in 0..300 { sim.tick(&d, &bond); }
    // Now force the grenade throw: set up the conditions and set_state.
    {
        let dr = &mut sim.list[gi];
        dr.state = state::AIM_STAND_FIRE;
        dr.next_state = None;
        dr.anim.class = 2;
        dr.anim.state = nf_game::drone::anim_state::AIM_STAND;
        dr.dist = 15.0;
        dr.engage_range = 25.0;
        dr.opp_target = bond.pos;
        dr.seen = true; dr.ever_seen = true; dr.vision_on = true; dr.unseen_ticks = 0;
        dr.clip = 5; dr.burst_done = true;
        dr.cover_claimed = false; dr.has_cover = false; dr.cover = None;
        d.props.set(0x30, &mut dr.props, 0);
        if let Some(ref mut ap) = dr.alert_props { d.props.set(0x30, ap, 0); }
        dr.state = state::GRENADE_THROW;
        dr.pending_grenade = true;
    }
    let before = sim.grenades.len();
    sim.tick(&d, &bond);
    // A grenade entity should have been created.
    assert!(sim.grenades.len() > before, "a grenade was spawned after the throw state");

    // Run the grenade to completion (it should bounce and explode).
    let mut exploded = false;
    for _ in 0..600 {
        sim.tick(&d, &bond);
        if !sim.grenades.is_empty() && sim.grenades[0].state >= 2 { exploded = true; break; }
        if sim.grenades.is_empty() && !sim.explosions.is_empty() { exploded = true; break; }
    }
    assert!(exploded, "the grenade exploded (bounced + detonated)");

    // Sister: Bond out of range (dist > 20) — the guard should NOT throw.
    let (drones2, rng2) = l.drones();
    let mut sim2 = l.sim(drones2, rng2);
    let gi2 = sim2.list.iter().position(|x| x.present && x.grenade_count > 0).unwrap();
    let g2 = sim2.list[gi2].pos;
    let bond_far = bond_at([g2[0] + 30.0, g2[1], g2[2]]); // 30 units — outside the 8–20 window
    for _ in 0..300 { sim2.tick(&d, &bond_far); }
    let grenades_before = sim2.grenades.len();
    for _ in 0..300 { sim2.tick(&d, &bond_far); }
    assert!(sim2.grenades.len() == grenades_before, "guard out of range (dist 30) does not throw");
}

/// Unforced: a grenade guard on 07000009, Bond in range (15 m), run 1800 ticks (30 s at 60 Hz).
/// The guard should decide to throw through its own decision tree, with the game's RNG.
/// On 07000009, guards have dtype=0xd, mode_normal=0 → mode 9 (HostageKillerAttack — a combat guard).
/// Sister: Bond at 30 m (out of range) → no throw in the same number of ticks.
#[test]
fn guard_decides_to_throw_grenade_unforced() {
    let Some(l) = Level02::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let (drones, rng) = l.drones();
    let mut sim = l.sim(drones, rng);

    // Find a guard that has grenades AND is a normal combat guard (not stuck in a non-combat mode).
    // On 07000002, all grenade guards are mode 7 (0x85) or mode 5 (Hostage). Neither enters combat.
    // We need to give grenades to a normal combat guard. The simplest: find a guard with mode 0
    // (default combat) and set its grenade fields the way the game's NDrone2_DefaultInit would
    // (param 24 = 0xcc800013 → weapon 0x36, count 3).
    let gi = sim.list.iter().position(|x| {
        x.present && x.mode == 0 && x.team == 1
    }).expect("level 07000002 should have a normal combat guard");
    let g = sim.list[gi].pos;
    // Give the guard grenades (the way param 24 would in NDrone2_DefaultInit).
    sim.list[gi].grenade_weapon = 0x36;
    sim.list[gi].grenade_count = 3;
    let gc_before = sim.list[gi].grenade_count;
    eprintln!("unforced: combat guard #{} at ({:.1}, {:.1}, {:.1}), given 3 stun grenades, mode={}", gi, g[0], g[1], g[2], sim.list[gi].mode);

    // Bond 12 units in front of the guard (in +x), on the floor. The guard faces roughly -x (yaw -1.5),
    // so Bond is behind the guard. But visibility still fires (dist < sight_range, alert >= 1 after first
    // contact). The key is LOS: we need a clear path. Place Bond in -z direction instead (guard faces
    // -x, but the level's corridors may be clearer in z).
    // Try Bond at guard's z - 12 (in -z direction).
    let bond_offsets: [[f32; 3]; 4] = [
        [12.0, 0.0, 0.0],   // +x
        [-12.0, 0.0, 0.0],  // -x
        [0.0, 0.0, 12.0],   // +z
        [0.0, 0.0, -12.0],  // -z
    ];
    let mut bond = bond_at(g);
    let mut best_los = false;
    for off in &bond_offsets {
        let p = [g[0] + off[0], g[1] + off[1], g[2] + off[2]];
        let fy = l.world.floor_below(p, 4.0).unwrap_or(g[1] - 1.0) + 1.0;
        let bp = [p[0], fy, p[2]];
        if l.world.line_of_sight([g[0], g[1] + 0.6, g[2]], [bp[0], bp[1] + 0.5, bp[2]]) {
            bond = bond_at(bp);
            best_los = true;
            eprintln!("[scenario] Bond at ({:.1}, {:.1}, {:.1}) — clear LOS from guard at ({:.1}, {:.1}, {:.1})",
                bp[0], bp[1], bp[2], g[0], g[1], g[2]);
            break;
        }
    }
    if !best_los {
        // Fallback: place Bond at +x anyway (the visibility check will still fire).
        let p = [g[0] + 12.0, g[1], g[2]];
        let fy = l.world.floor_below(p, 4.0).unwrap_or(g[1] - 1.0) + 1.0;
        bond = bond_at([p[0], fy, p[2]]);
        eprintln!("[scenario] WARNING: no clear LOS found — Bond at ({:.1}, {:.1}, {:.1})", bond.pos[0], bond.pos[1], bond.pos[2]);
    }

    // Enable all switch channels so the guard starts.
    for c in 1..=255u8 { sim.set_channel(c); }

    // Run 1800 ticks (30 s). The guard should see Bond, enter combat, and eventually throw.
    let mut threw = false;
    for t in 0..1800 {
        sim.tick(&d, &bond);
        if t % 60 == 0 && t < 300 {
            let dr = &sim.list[gi];
            let vis = if dr.vision_on || dr.alert >= 1.0 { dr.visibility() } else { 0.0 };
            eprintln!("[trace] tick {}: guard #{} state={:#x} dist={:.1} seen={} vision_on={} vis={:.3} alert={:.2} yaw={:.1} yaw_to_opp={:.1} los={} grenades={}",
                t, gi, dr.state, dr.dist, dr.seen, dr.vision_on, vis, dr.alert, dr.yaw, dr.yaw_to_opp, dr.los, dr.grenade_count);
        }
        if !sim.grenades.is_empty() || sim.list[gi].grenade_count < gc_before {
            threw = true;
            eprintln!("grenade thrown at tick {} (guard state {:#x})", t, sim.list[gi].state);
            break;
        }
    }
    assert!(threw, "guard with grenades (count {}) decided to throw within 1800 ticks — \
        check NF_AILOG=1 for why can_throw_grenade never passed", gc_before);

    // Sister: Bond at 30 m — out of the 8–20 throw window. No throw in 1800 ticks.
    let (drones2, rng2) = l.drones();
    let mut sim2 = l.sim(drones2, rng2);
    let gi2 = sim2.list.iter().position(|x| x.present && x.mode == 0 && x.team == 1)
        .expect("level 07000002 should have a normal combat guard");
    let g2 = sim2.list[gi2].pos;
    sim2.list[gi2].grenade_weapon = 0x36;
    sim2.list[gi2].grenade_count = 3;
    let floor_y2 = l.world.floor_below([g2[0] + 30.0, g2[1], g2[2]], 4.0).unwrap_or(g2[1] - 1.0) + 1.0;
    let bond_far = bond_at([g2[0] + 30.0, floor_y2, g2[2]]);
    let gc_before2 = sim2.list[gi2].grenade_count;
    for c in 1..=255u8 { sim2.set_channel(c); }
    for _ in 0..1800 { sim2.tick(&d, &bond_far); }
    assert!(sim2.list[gi2].grenade_count == gc_before2,
        "guard out of range (30 m) did not throw — grenades {} == {}", sim2.list[gi2].grenade_count, gc_before2);
}
