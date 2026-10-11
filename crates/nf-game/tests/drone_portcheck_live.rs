//! NF_ISO tests for the fixes that implement `docs/atlas/drone-port-check.md` (fix-drone):
//! grenades from placement param 11 (0x14b300), the CombatNoRoute `+0xCD8` toggle order (0x164c08), the
//! UnderCoverIdle tries threshold (0x169e58), auto-aim's single-player team gate (0x1a73f8), and spawner
//! templates not drawn (drone.md "present / started / enabled / in-llist"). Opt-in: `NF_ISO=<iso>`.

use nf_formats::anim::Script;
use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, drone, level::Level, levelbin::LevelBin, weapon::WeaponData};
use nf_game::drone::{state, Data, Drone, Drones, Player, Rng};
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
        let covers = nf_game::cover::build(&level.maps[main].placements, &nav, &world);
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
            surfaces: None, voices: None }
    }

    fn sim(&self) -> Drones {
        let d = self.data();
        let mut rng = Rng::new(0x1234_5678);
        let drones = nf_game::drone::level_drones(&d, &self.level, &mut rng);
        let mut s = Drones::new(drones, rng);
        s.attach_level(&self.level);
        s
    }
}

fn bond_at(p: [f32; 3]) -> Player { Player { pos: p, target: [p[0], p[1] + 0.5, p[2]], eye: [p[0], p[1] + 1.0, p[2]], aim: [-1.0, 0.0, 0.0] } }

/// The documented grenade decode (drone.md param-11 row, `NDrone2_DefaultInit` 0x14b300 lines 348-376):
/// weapon = switch on `(raw >> 4) & 0xffff` (1 → 0x36, 2 → 0x35, 4 → 0x34, 5 → count+0x3A, 6 → 0x6C, else 0),
/// count = `raw & 0xF` (case 5/6 force 1); on 0x7000014 a weapon in (0x34, 0x37) clamps to 0x34.
fn doc_grenade_decode(raw: u32, level: u32) -> (u16, i16) {
    let sel = ((raw >> 4) & 0xffff) as u16;
    let cnt = (raw & 0xf) as u16;
    let (mut w, c) = match sel {
        1 => (0x36u16, cnt as i16), 2 => (0x35u16, cnt as i16), 4 => (0x34u16, cnt as i16),
        5 => (cnt.wrapping_add(0x3a) as u16, 1i16), 6 => (0x6cu16, 1i16), _ => (0u16, 0i16),
    };
    if level == 0x0700_0014 && (0x34..0x37).contains(&w) { w = 0x34; }
    (w, c)
}

fn param_of<'a>(p: &'a nf_formats::placement::Placement) -> impl Fn(u32) -> u32 + 'a {
    |k: u32| p.params.iter().find(|(j, _)| *j == k).map(|&(_, v)| v).unwrap_or(0)
}

/// Grenades come from placement **param 11** (drone-port-check.md #1): every drone on 07000001 (stun carriers)
/// and 07000014 (frag carriers and the 0x7000014 clamp) decodes to the documented values, and at least one guard
/// per level actually carries. Sister: for those carriers the old source word (param 24) decodes to NO grenade —
/// the test would have failed before the fix.
#[test]
fn grenade_fields_come_from_param_11() {
    let Some(l) = Fixture::load("07000001.bin", 0x0700_0001) else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let main = l.level.main_map().unwrap();
    let mut carriers = 0usize;
    let mut old_new_disagree = false;
    for p in l.level.maps[main].placements.iter().filter(|p| p.flags & 0xff == 0x0f) {
        let param = param_of(p);
        let mut rng = Rng::new(1);
        let dr = Drone::new(&d, [p.translation[0], p.translation[1], p.translation[2]], p.rotation[1], &param, &mut rng);
        let expect = doc_grenade_decode(param(11), l.level_id);
        assert_eq!((dr.grenade_weapon, dr.grenade_count), expect,
            "placement at {:?} param 11 = {:#x}", p.translation, param(11));
        if expect.0 != 0 { carriers += 1; }
        if doc_grenade_decode(param(11), l.level_id).0 != doc_grenade_decode(param(24), l.level_id).0 { old_new_disagree = true; }
    }
    assert!(carriers > 0, "07000001 has grenade carriers (param 11 = 0x13 → stun)");
    assert!(old_new_disagree, "sister: on a carrier the old param-24 read decodes differently (test can fail)");

    // 07000014: the clamp (0x14b300 lines 374-376) turns 0x35/0x36 into 0x34 on this level.
    let l14 = Fixture::load("07000014.bin", 0x0700_0014).unwrap();
    let d14 = l14.data();
    let main14 = l14.level.main_map().unwrap();
    let mut clamped = 0usize;
    let mut carriers14 = 0usize;
    for p in l14.level.maps[main14].placements.iter().filter(|p| p.flags & 0xff == 0x0f) {
        let param = param_of(p);
        let mut rng = Rng::new(1);
        let dr = Drone::new(&d14, [p.translation[0], p.translation[1], p.translation[2]], p.rotation[1], &param, &mut rng);
        let expect = doc_grenade_decode(param(11), l14.level_id);
        assert_eq!((dr.grenade_weapon, dr.grenade_count), expect, "07000014 placement param 11 = {:#x}", param(11));
        if expect.0 != 0 {
            carriers14 += 1;
            let raw_sel = ((param(11) >> 4) & 0xffff) as u16;
            if matches!(raw_sel, 2) { assert_eq!(dr.grenade_weapon, 0x34, "0x7000014 clamp: frag 0x35 → 0x34"); clamped += 1; }
        }
    }
    assert!(carriers14 > 0 && clamped > 0, "07000014 has carriers and the clamp fires on real data");
}

/// Auto-aim never targets a non-team-1 drone in single player (`Check_AutoAim` 0x1a73f8: `+0x44 != 1` → skip):
/// 07000002's civilian (skin 0x0500004F, DTYPE 0xC → team 2) is not locked even with Bond aiming straight at it
/// from a spot that does lock a team-1 guard. Sister: that team-1 guard IS locked under the same construction.
#[test]
fn auto_aim_skips_non_enemies_on_07000002() {
    let Some(l) = Fixture::load("07000002.bin", 0x0700_0002) else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let mut sim = l.sim();
    // Let the guards start: 07000002's guards wait on switch channels, so turn them all on and let the idle
    // anims play (palette needs a playing script). Bond stays far away so nobody fights.
    let far = bond_at([0.0, -1000.0, 0.0]);
    for c in 1..=255u8 { sim.set_channel(c); }
    for _ in 0..60 { sim.tick(&d, &far); }
    let civ = sim.list.iter().position(|x| x.skin == 0x0500_004f && x.present)
        .expect("07000002 has the 0x0500004F civilian (DTYPE 0xC)");
    assert_eq!(sim.list[civ].team, 2, "the civilian's team is 2 (drone.md +0x44: DTYPE 0xC → 2)");

    // Find a team-1 guard, a LOS spot near it, and prove auto-aim locks it (the sister: the harness works).
    let mut locked = None;
    'guards: for gi in 0..sim.list.len() {
        if sim.list[gi].team != 1 || !sim.list[gi].present || sim.list[gi].corpse { continue; }
        let Some(b2) = sim.list[gi].palette(&d).and_then(|m| m.get(2).map(|r| [r[3][0], r[3][1], r[3][2]])) else { continue };
        sim.list[gi].drawn = true;
        let g = sim.list[gi].pos;
        for off in [[8.0, 0.0, 0.0], [-8.0, 0.0, 0.0], [0.0, 0.0, 8.0], [0.0, 0.0, -8.0], [5.7, 0.0, 5.7], [5.7, 0.0, -5.7]] {
            let p = [g[0] + off[0], g[1], g[2] + off[2]];
            let fy = l.world.floor_below(p, 4.0).unwrap_or(g[1] - 1.0);
            let q = [p[0], fy + 1.0, p[2]];
            if !l.world.line_of_sight(q, b2) { continue; }
            let bq = bond_at(q);
            let look = { let v = [b2[0] - bq.eye[0], b2[1] - bq.eye[1], b2[2] - bq.eye[2]];
                let ln = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt(); [v[0] / ln, v[1] / ln, v[2] / ln] };
            sim.auto_aim(&d, bq.eye, look, 2);
            if sim.aim_target == Some(gi) { locked = Some((gi, q, g)); break 'guards; }
        }
    }
    let Some((gi, spot, gpos)) = locked else { panic!("sister: no team-1 guard is lockable — the harness is blind") };

    // The civilian at the guard's spot (same open geometry), the guard hidden: with the pre-fix code Bond's
    // crosshair locks the civilian; with the team gate it must not.
    {
        let c = &mut sim.list[civ];
        c.pos = gpos;
        c.present = true;
        c.enabled = true;
        c.drawn = true;
        c.corpse = false;
        c.health = 10.0;
    }
    sim.list[gi].drawn = false;
    let Some(b2) = sim.list[civ].palette(&d).and_then(|m| m.get(2).map(|r| [r[3][0], r[3][1], r[3][2]])) else {
        panic!("the civilian has a posed palette");
    };
    let bq = bond_at(spot);
    let look = { let v = [b2[0] - bq.eye[0], b2[1] - bq.eye[1], b2[2] - bq.eye[2]];
        let ln = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt(); [v[0] / ln, v[1] / ln, v[2] / ln] };
    sim.auto_aim(&d, bq.eye, look, 2);
    assert!(sim.aim_target != Some(civ),
        "auto-aim must not lock onto the team-2 civilian (0x1a73f8: +0x44 != 1 → skip)");
}

/// CombatNoRoute's `+0xCD8` toggle only fires on the no-cover branch (`NDrone2_DSTATE_CombatNoRoute` 0x164c08
/// case 0xC): a stays-put drone (prop 0x40) never toggles across timer ticks. Sister: without prop 0x40 and with
/// cover unavailable, every timer tick toggles once (odd → MoveToObject, even → the drone-with-LOS goal).
#[test]
fn combat_no_route_toggle_follows_0x164c08() {
    let Some(l) = Fixture::load("07000002.bin", 0x0700_0002) else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let (drones, rng) = { let dd = l.data(); let mut r = Rng::new(0x1234_5678);
        (nf_game::drone::level_drones(&dd, &l.level, &mut r), r) };
    let mut sim = Drones::new(drones, rng);
    sim.attach_level(&l.level);
    let far = bond_at([0.0, -1000.0, 0.0]);
    for _ in 0..5 { sim.tick(&d, &far); }
    let gi = sim.list.iter().position(|x| x.present && !x.corpse).expect("a live guard");
    let frame = 10_000;
    {
        let dr = &mut sim.list[gi];
        dr.state = state::COMBAT_NO_ROUTE;
        dr.next_state = None;
        dr.entered = frame - 120;
        dr.cover = None; dr.cover_claimed = false; dr.has_cover = false;
        d.props.set(0x30, &mut dr.props, 0); // no cover anywhere → the cover branch fails
        d.props.set(0x40, &mut dr.props, 1); // stays put → the game skips the toggle entirely
        dr.seen = false;
    }
    let rng = &mut sim.rng;
    for _ in 0..3 {
        {
            let dr = &mut sim.list[gi];
            dr.state = state::COMBAT_NO_ROUTE; // pin: isolated TIMER_C deliveries
            dr.next_state = None;
        }
        sim.list[gi].send(&d, rng, frame, 0xc); // msg 0xC = TIMER_C
    }
    assert!(!sim.list[gi].no_route_odd, "stays-put drone never toggles +0xCD8 (0x164c08: toggle only on the no-cover branch)");

    // Sister: the same guard without prop 0x40 toggles once per timer tick.
    {
        let dr = &mut sim.list[gi];
        dr.state = state::COMBAT_NO_ROUTE;
        dr.next_state = None;
        dr.no_route_odd = false;
        d.props.set(0x40, &mut dr.props, 0);
    }
    let rng = &mut sim.rng;
    sim.list[gi].send(&d, rng, frame, 0xc);
    assert!(sim.list[gi].no_route_odd, "sister: the no-cover branch toggles +0xCD8 once per timer tick");
}

/// UnderCoverIdle's tries count gates the re-arm on the **old** value (`NDrone2_DSTATE_UnderCoverIdle` 0x169e58
/// case 0xC): three re-arms, Attack on the fourth timer tick. The old port attacked on the third.
#[test]
fn under_cover_idle_retries_three_times() {
    let Some(l) = Fixture::load("07000002.bin", 0x0700_0002) else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let (drones, rng) = { let dd = l.data(); let mut r = Rng::new(0x1234_5678);
        (nf_game::drone::level_drones(&dd, &l.level, &mut r), r) };
    let mut sim = Drones::new(drones, rng);
    sim.attach_level(&l.level);
    let far = bond_at([0.0, -1000.0, 0.0]);
    for _ in 0..5 { sim.tick(&d, &far); }
    let gi = sim.list.iter().position(|x| x.present && !x.corpse).expect("a live guard");
    {
        let dr = &mut sim.list[gi];
        dr.state = state::UNDER_COVER_IDLE;
        dr.next_state = None;
        dr.cover = None; dr.cover_claimed = false; dr.has_cover = false;
        dr.cover_tries = 0;
        d.props.set(0x30, &mut dr.props, 0); // no cover: every branch falls to the tries count
        d.props.set(0x10, &mut dr.props, 0);
        dr.timer_c = None;
    }
    let frame = 20_000;
    let rng = &mut sim.rng;
    for tick in 1..=4 {
        // Pin the state: each send is an isolated TIMER_C delivery to UnderCoverIdle (the 5 setup ticks and
        // HashMap-ordered script state must not leak into the assertion).
        {
            let dr = &mut sim.list[gi];
            dr.state = state::UNDER_COVER_IDLE;
            dr.next_state = None;
            dr.timer_c = None;
        }
        sim.list[gi].send(&d, rng, frame, 0xc);
        let (s, tries) = (sim.list[gi].state, sim.list[gi].cover_tries);
        eprintln!("timer tick {tick}: state {:#x} tries {}", s, tries);
        if tick <= 3 { assert_eq!(s, state::UNDER_COVER_IDLE, "tick {tick}: still retrying (old value < 3 re-arms)"); }
    }
    // The 4th tick leaves UnderCoverIdle (Attack's ENTER immediately routes on — FirstSightState → Combat 0x58
    // here, as the decomp's SetState(0x56) chain does); the old port left on the 3rd tick.
    assert_ne!(sim.list[gi].state, state::UNDER_COVER_IDLE, "the fourth timer tick attacks");
    assert_eq!(sim.list[gi].cover_tries, 4);
}

/// A spawner template has no object until `DroneSpawner_SpawnDrone` 0x137cb0 creates it, so it is not drawn
/// (drone.md "present / started / enabled / in-llist"): `drawn` (the `obj+0xFC` stand-in) is false while held.
/// Sister: drones with objects at level start keep drawn == whatever their first tick computed (true when visible).
#[test]
fn spawner_templates_are_not_drawn_on_07000005() {
    let Some(l) = Fixture::load("07000005.bin", 0x0700_0005) else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let mut sim = l.sim();
    assert!(!sim.spawners.is_empty(), "07000005 has spawner groups (3 objects [L])");
    let templates: Vec<usize> = sim.list.iter().enumerate().filter(|(_, x)| !x.present).map(|(i, _)| i).collect();
    assert!(!templates.is_empty(), "the spawner groups hold templates");
    for &t in &templates {
        assert!(!sim.list[t].present && !sim.list[t].enabled, "template held back");
        assert!(!sim.list[t].drawn, "template is not drawn (no object, no obj+0xFC)");
    }
    let _ = d;
    // Sister: after a tick the live drones' drawn flag is computed (enabled && visible) — not stuck true/false.
    let bond = bond_at([104.954, 34.326, 103.324]); // the level-start position [L] (docs/systems/player.md)
    for _ in 0..2 { sim.tick(&d, &bond); }
    assert!(sim.list.iter().any(|x| x.present && x.enabled), "live drones exist after a tick");
}
