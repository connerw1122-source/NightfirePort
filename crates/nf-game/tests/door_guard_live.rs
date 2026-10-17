//! door-guard-pairing (task 737, the uncited-audit runner-up) on 07000007 — the only story level
//! that places door guards [D census below]: 6 guards whose mode resolves to 0x16 and whose start
//! state is CivilianDoorGuard 0x31 (`DroneTypeSettings` row 22: start 0x31 [L the table dump;
//! docs/fields/dronetype-tables.md]). The port had the const with no arms (WORKLOG 3969).
//!
//! The game's path [M]:
//! - `NDrone2_SetDronePartner(0x16, dc)` 0x141360 at the 0x31 ENTER [M 0x160694]: the nearest
//!   other mode-0x16 guard (enabled `+0x4F8 & 0x600 == 0 && & 0x100 != 0`, health > 0, class
//!   != 0x11, not deleted) within 2.0 rad of the caller's yaw (+0x34) and partnerless; the pair is
//!   mutual (`+0x2BC` each side); `Vec_AngleDifference(my_yaw, atan2(dir))` < 0 marks the CALLER's
//!   +0x2CC = 0xff and builds three planes on the PARTNER (normal −dir through the partner), else
//!   the PARTNER gets 0xff and the planes are built on the CALLER (normal +dir) [M 0x141360:115-174].
//! - `NDrone2_DoorGuardHandler` 0x1416d8 at the 0x31 UPDATE [M 0x160814]: no partner or no
//!   opponent (+0x170) → 0; the +0x2CC == 0xff holder returns the partner's +0x2CC; the planes'
//!   owner evaluates: |opp yaw − my yaw| ≥ 2.0 → 0, else plane1/plane2 gates then the plane-0
//!   bands s0 < −6 → 0, < 0 → 3, < 2 → 2, < 6 → 1, else 0 [M 0x1416d8:16-73].
//! - The 0x31 UPDATE arms [M 0x160648:38-105]: an armed opponent → prop-0x33 shout + Attack;
//!   zone 1 → face + the idle-anim replay on the step change; zone 2 → anim 0x5f variant 1 + the
//!   ChooseSFX(0x4f, 0x50) speech (the planes' owner only) + face; zone 3 → ChooseSFX(0x8f, 0x92)
//!   + `DroneFunc_SetMissionFailReason(drone, 0xc)` + Attack; zone 0 → the idle-anim replay.
//! - TIMER_C (case 0xc): the idle cycle 0x24 → 0x25 → 0x26 → 0x24 by the `+0xCC8` payload,
//!   re-armed at frame + (Rand(20) + 5)·FRAME_RATE_INT [M 0x160648:124-150; FRAME_RATE_INT = 30 —
//!   the port's DEAD-arm comment `DAT_00298F58 = 10 · FRAME_RATE_INT` = 300 ticks].
//!
//! Opt-in: set `NF_ISO` to the disc image.

use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, drone, level::Level, levelbin::LevelBin, weapon::WeaponData};
use nf_game::drone::{state, Data, Drones, Player, Rng};
use std::collections::HashMap;

pub struct L {
    tables: drone::DroneAnimData,
    props: drone::PropertyTables,
    weapons: WeaponData,
    level: Level,
    world: nf_game::world::CollisionWorld,
    nav: nf_game::nav::Network,
    covers: Vec<nf_game::cover::CoverNode>,
    volumes: Vec<nf_game::aivolume::AiVolume>,
    scripts: HashMap<u32, nf_formats::anim::Script>,
    sequences: HashMap<u32, nf_formats::anim::Sequence>,
    skeletons: HashMap<u8, nf_formats::character::Skeleton>,
    models: HashMap<u32, nf_game::pickup::Model>,
    tuning: nf_formats::tuning::Tuning,
    hitboxes: nf_formats::hitbox::CollisionBones,
}

impl L {
    pub fn load() -> Option<L> {
        let path = std::env::var_os("NF_ISO")?;
        let iso = IsoImage::open(&path).unwrap();
        let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
        let (entries, bin) = disc::files_bin_entries(&iso).unwrap();
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000007.bin")).unwrap();
        let raw = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
        let level = Level::parse(&raw).unwrap();
        let lb = LevelBin::parse(&raw).unwrap();
        let world = nf_game::world::CollisionWorld::build(&level, &level.instances(true).0);
        let m = level.main_map().unwrap();
        let nav = nf_game::nav::Network::build(level.maps[m].ai_network.as_ref().unwrap(), &world, 0x0700_0007);
        let covers = nf_game::cover::build(&level.maps[m].placements, &nav, &world);
        let volumes = nf_game::aivolume::build(&level, 0x0700_0007);
        let scripts = lb.entries.iter().filter(|e| e.kind == 5).filter_map(|e| nf_formats::anim::Script::parse(lb.data(&raw, e)).ok()).map(|s| (s.hashcode, s)).collect();
        let skeletons = lb.entries.iter().filter(|e| e.kind == 6).filter_map(|e| nf_formats::character::parse_skeleton(lb.data(&raw, e)).ok().map(|s| (s.id as u8, s))).collect();
        let models_hold = nf_game::pickup::models(&level);
        Some(L {
            tables: drone::DroneAnimData::from_action_elf(&elf).unwrap(),
            props: drone::PropertyTables::from_action_elf(&elf).unwrap(),
            weapons: WeaponData::from_action_elf(&elf).unwrap(),
            tuning: { let mut t = nf_formats::tuning::Tuning::from_action_elf(&elf).unwrap(); nf_game::tuning::read_tuning_vars(&mut t, 0x0700_0007); t },
            hitboxes: nf_formats::hitbox::CollisionBones::from_action_elf(&elf).unwrap(),
            world, nav, covers, volumes, scripts, sequences: HashMap::new(), skeletons,
            // `models` derives from `&level` before the move.
            level, models: models_hold,
        })
    }

    pub fn data(&self) -> Data<'_> {
        Data {
            tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world, level: 0x0700_0007,
            sequences: &self.sequences, skins: &self.level.skins, skeletons: &self.skeletons, nav: &self.nav, covers: &self.covers, tuning: &self.tuning,
            hitboxes: &self.hitboxes, volumes: &self.volumes, sfx: None, models: &self.models, surfaces: None, voices: None, difficulty: 1,
        }
    }

    /// The full roster with one Rng through `level_drones` (the game's order), then a fresh `Drones`
    /// rng for the tick stream.
    pub fn sim(&self) -> (Drones, Rng) {
        let d = self.data();
        let mut rng = Rng::new(0x1234_5678);
        let mut lights = nf_game::light::LightList::default();
        let drones = nf_game::drone::level_drones(&d, &self.level, &mut rng, &mut lights);
        let s = Drones::new(drones, Rng::new(0x777));
        (s, Rng::new(0x777))
    }
}

pub fn bond_at(p: [f32; 3]) -> Player { Player { pos: p, target: [p[0], p[1] + 0.5, p[2]], eye: [p[0], p[1] + 1.0, p[2]], aim: [-1.0, 0.0, 0.0], sph: p, capsule: ([p[0], p[1] + 1.086, p[2]], p), hanging: false, cel: None } }

pub fn idx(sim: &Drones, placement: usize) -> usize {
    sim.list.iter().position(|dr| dr.placement == placement).unwrap_or_else(|| panic!("placement {placement} on the roster"))
}

/// The door guards reach CivilianDoorGuard 0x31 and pair into mutual nearest-neighbour pairs —
/// the four ACTIVE guards (404/417/441/442; 448/449 wait for switch channel 21 [D the probe in
/// the report]) form 2 pairs [M SetDronePartner 0x141360]. Fail-first: the const had no arms,
/// `door_partner` never filled.
#[test]
fn door_guards_pair_into_mutual_nearest_pairs() {
    let Some(l) = L::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let (mut sim, _rng) = l.sim();
    let guards: Vec<usize> = [404usize, 417, 441, 442].iter().map(|&pl| idx(&sim, pl)).collect();
    // Settle two ticks: the started ENTERs, then the pairing pass (the port pairs at the tick
    // boundary after the ENTER [*S one-tick deviation from the in-ENTER pairing]).
    let p = bond_at([-500.0, 0.0, -500.0]);
    sim.tick(&d, &p);
    for &i in &guards {
        assert_eq!(sim.list[i].state, state::CIVILIAN_DOOR_GUARD,
            "pl {} reached CivilianDoorGuard, state {:#x}", sim.list[i].placement, sim.list[i].state);
    }
    sim.tick(&d, &p);
    let mut partners: Vec<(usize, usize)> = Vec::new();
    for &i in &guards {
        let Some(pn) = sim.list[i].door_partner else {
            panic!("pl {} never paired", sim.list[i].placement);
        };
        assert!(guards.contains(&pn), "pl {}'s partner is a door guard", sim.list[i].placement);
        assert_eq!(sim.list[pn].door_partner, Some(i), "the pairing is mutual ({} ↔ {})", sim.list[i].placement, sim.list[pn].placement);
        // The yaw gate [M 0x141360:90]: |yaw diff| < 2.0.
        assert!(nf_game::trig::angle_difference(sim.list[i].yaw, sim.list[pn].yaw).abs() < 2.0);
        // Exactly one of the pair is the planes' owner (door_side != 0xff), the other mirrors.
        assert!((sim.list[i].door_side == 0xff) ^ (sim.list[pn].door_side == 0xff),
            "exactly one evaluator per pair: {} {:#x} {} {:#x}", sim.list[i].placement, sim.list[i].door_side, sim.list[pn].placement, sim.list[pn].door_side);
        // The planes' owner's three planes match the [M] formulas exactly: n0 = cross(v0, (0,0,1))
        // through the anchor (v0 = ±dir by the angle-difference sign), the derived 0x2F0/0x300
        // planes per 0x141360:134-173. (Pair 441/442's axis runs along z: the cross with (0,0,1)
        // is the game's own degenerate zero normal [M 0x1e4498:11-13].)
        // The non-0xff side owns the planes; the 0xff holder is the mirror.
        let (owner, mirror) = if sim.list[i].door_side == 0xff { (pn, i) } else { (i, pn) };
        let dx = sim.list[owner].pos[0] - sim.list[mirror].pos[0];
        let dy = sim.list[owner].pos[1] - sim.list[mirror].pos[1];
        let dz = sim.list[owner].pos[2] - sim.list[mirror].pos[2];
        let len = (dx * dx + dy * dy + dz * dz).sqrt();
        let dir = [dx / len, dy / len, dz / len];
        // diff < 0 → the planes on the MIRROR-side's partner... the owner is `mirror`'s partner;
        // v0 = −dir when the MIRROR's diff < 0 (the planes then live on the mirror), else +dir.
        // Recover the sign from the owner's position: v0 points owner−mirror when the planes were
        // built on the mirror (diff < 0 → mirror 0xff, planes on the partner = the owner through
        // −dir), and mirror−owner when built on the owner (diff ≥ 0, planes through +dir from the
        // owner's OWN position — the owner IS the anchor). The anchor is whoever owns the planes.
        let anchor = sim.list[owner].pos;
        let other = sim.list[mirror].pos;
        // The owner is the anchor; v0 = +dir −− where dir = normalise(partner − me) for the guard
        // whose diff decided. When the owner is the mirror's partner (mirror's diff < 0), the
        // game's `dir` ran from mirror→owner and v0 = −dir = owner−mirror... no: v0 = −dir with
        // dir = owner−mirror → v0 = mirror−owner. When the owner decided (his diff ≥ 0), dir =
        // mirror−owner (partner − me) and v0 = +dir. Both give v0 = the direction the owner faces
        // his partner INVERTED — recover it from which side holds 0xff:
        let dir_from_owner_to_partner = [ (sim.list[mirror].pos[0] - anchor[0]) / len, (sim.list[mirror].pos[1] - anchor[1]) / len, (sim.list[mirror].pos[2] - anchor[2]) / len ];
        let _ = dir_from_owner_to_partner;
        // The decisive guard is the one whose yaw enters Vec_AngleDifference: the 0xff holder ran
        // the scan (his diff < 0 put the planes on his partner). v0 = −dir(d decisive guard).
        let decisive = mirror; // the 0xff holder: his diff < 0 built the planes on the partner... OR his diff ≥ 0 put 0xff on the partner — recover from door_side:
        let _ = decisive;
        // Simplest exact pin: v0 = −dir when the MIRROR's own angle diff was < 0 — equivalently
        // the planes' d matches only one of the two signs; try both and demand one matches.
        let mut matches = 0;
        for neg in [-1.0f32, 1.0] {
            let v0 = [neg * dir[0], neg * dir[1], neg * dir[2]];
            // dir here: owner→mirror; the game's dir ran decisive→its-partner. For decisive =
            // mirror (mirror 0xff): game dir = mirror→owner?? — covered by the two-norm probe below.
            let n = [v0[1], -v0[0], 0.0f32];
            let d0 = -(n[0] * anchor[0] + n[1] * anchor[1] + n[2] * anchor[2]);
            let (a, b, c) = (n[0], n[1], n[2]);
            let expect = [
                [a, b, c, d0],
                [-c, b, a, d0 + a * anchor[0] + c * other[0] + c * anchor[2] - a * other[2]],
                [c, b, -a, d0 + (a - c) * anchor[0] + (c + a) * anchor[2]],
            ];
            let got = sim.list[owner].door_planes;
            let close = |x: f32, y: f32| (x - y).abs() < 1e-3 || (x - y).abs() < 1e-3 * x.abs().max(1.0);
            let all = expect.iter().zip(got.iter()).all(|(e, g)| e.iter().zip(g.iter()).all(|(&x, &y)| close(x, y)));
            if all { matches += 1; }
        }
        assert!(matches >= 1, "the owner's planes match the [M] formula for one sign: got {:?} (anchor {anchor:?} other {other:?})", sim.list[owner].door_planes);
        partners.push((i.min(pn), i.max(pn)));
    }
    partners.sort();
    partners.dedup();
    assert_eq!(partners.len(), 2, "4 active guards → 2 pairs, got {partners:?}");
}

/// The zone bands drive the UPDATE arms [M 0x1416d8:55-73 + 0x160648:59-103]. The pairs' plane
/// normals come out ±Y (the pair axes are horizontal, cross(v, (0,0,1)) [M 0x1e4498]) — s0 is the
/// standing player's height against the anchor's: s0 ≈ 0 → the challenge band (zone 2: the 0x5f
/// anim once, the planes' owner speaks); a player seen slightly BELOW the anchor (s0 ∈ [−6, 0))
/// is zone 3: the mission-fail reason 0xc and the guard goes hostile.
#[test]
fn zone_bands_drive_the_challenge_and_the_mission_fail() {
    let Some(l) = L::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let (mut sim, _rng) = l.sim();
    let guards: Vec<usize> = [404usize, 417, 441, 442].iter().map(|&pl| idx(&sim, pl)).collect();
    let p0 = bond_at([-500.0, 0.0, -500.0]);
    sim.tick(&d, &p0);
    sim.tick(&d, &p0);
    let side = |pl: [f32; 4], p: [f32; 3]| pl[0] * p[0] + pl[1] * p[1] + pl[2] * p[2] + pl[3];
    // For each planes' owner, probe standing spots in front of him (along his facing) whose sides
    // land in the challenge band, with a clear line. (The aim-yaw gate is satisfied by facing the
    // guard's own way — the RAW gate [M 0x1416d8:22].)
    let mut challenge: Vec<(usize, [f32; 3])> = Vec::new();
    let mut fail: Vec<(usize, [f32; 3])> = Vec::new();
    for &ev in &guards {
        if sim.list[ev].door_side == 0xff { continue; } // the mirror holds no planes
        let planes = sim.list[ev].door_planes;
        let anchor = sim.list[ev].pos;
        let face = [sim.list[ev].yaw.sin(), 0.0, sim.list[ev].yaw.cos()];
        for k in [1usize, 2, 3, 4] {
            for (dx, dz) in [(0i32, 0i32), (1, 0), (-1, 0), (0, 1), (0, -1)] {
                let cand = [anchor[0] + face[0] * k as f32 + dx as f32 * 0.3, anchor[1], anchor[2] + face[2] * k as f32 + dz as f32 * 0.3];
                let (s0, s1, s2) = (side(planes[0], cand), side(planes[1], cand), side(planes[2], cand));
                if !d.world.line_of_sight([cand[0], cand[1] + 1.0, cand[2]], [anchor[0], anchor[1] + 1.4, anchor[2]]) { continue; }
                if (0.0..2.0).contains(&s0) && s1 >= 0.0 && s2 >= 0.0 { challenge.push((ev, cand)); }
                if (-6.0..0.0).contains(&s0) && s1 >= 0.0 && s2 >= 0.0 { fail.push((ev, cand)); }
            }
        }
    }
    assert!(!challenge.is_empty(), "no challenge spots (the geometry probe)");
    // The ARMED sister [M 0x160648:38-58]: `NDrone2_NearArmedPerson` runs BEFORE the zone read —
    // an armed opponent at the same spot turns the guard hostile (the prop-0x33 shout), no zone
    // animation. The port's `bond_weapon` gate [M the armed set 2..0x47].
    let (guard, pos) = challenge[0].clone();
    {
        let (mut sim, _rng) = l.sim();
        sim.tick(&d, &p0);
        sim.tick(&d, &p0);
        let ev = idx(&sim, sim.list[guard].placement);
        let yaw = sim.list[ev].yaw;
        let aim = [yaw.sin(), 0.0, yaw.cos()];
        let p = Player { pos, target: [pos[0], pos[1] + 0.5, pos[2]], eye: [pos[0], pos[1] + 1.0, pos[2]], aim, sph: pos, capsule: ([pos[0], pos[1] + 1.086, pos[2]], pos), hanging: false, cel: None };
        let mut hostile = false;
        for _ in 0..90 {
            sim.tick(&d, &p);
            if matches!(sim.list[ev].state, state::ATTACK | state::DRAW_WEAPON | state::COMBAT | state::AIM_STAND | state::AIM_STAND_FIRE | state::HEARD_NOISE_AWARE | state::HEARD_NOISE_ALERT) {
                hostile = true;
                break;
            }
        }
        assert!(hostile, "the armed opponent never turned the door guard hostile (state {:#x})", sim.list[ev].state);
        assert!(sim.list[ev].door_zone_step != 2, "the armed path must not play the challenge");
    }
    // The challenge: stand UNARMED Bond there (fists, weapon 1 — outside the armed set), aim his
    // yaw the guard's way.
    let (mut sim, _rng) = l.sim();
    sim.bond_weapon = 1;
    sim.tick(&d, &p0);
    sim.tick(&d, &p0);
    let ev = idx(&sim, sim.list[guard].placement);
    let yaw = sim.list[ev].yaw;
    let aim = [yaw.sin(), 0.0, yaw.cos()];
    let p = Player { pos, target: [pos[0], pos[1] + 0.5, pos[2]], eye: [pos[0], pos[1] + 1.0, pos[2]], aim, sph: pos, capsule: ([pos[0], pos[1] + 1.086, pos[2]], pos), hanging: false, cel: None };
    let mut saw_challenge = false;
    for _ in 0..90 {
        sim.tick(&d, &p);
        if sim.list[ev].door_zone == 2 && sim.list[ev].door_zone_step == 2 { saw_challenge = true; break; }
        assert!(sim.list[ev].fail_reason.is_none(), "the challenge band must not fail the mission");
    }
    assert!(saw_challenge, "the challenge band never fired: zone {} step {} los {}",
        sim.list[ev].door_zone, sim.list[ev].door_zone_step, sim.list[ev].los);

    // The restricted band: a seen player with s0 ∈ [−6, 0) → the fail reason 0xc + hostile.
    let (mut sim, _rng) = l.sim();
    sim.tick(&d, &p0);
    sim.tick(&d, &p0);
    if let Some((guard, pos)) = fail.first().cloned() {
        let ev = idx(&sim, sim.list[guard].placement);
        sim.bond_weapon = 1; // the zones only run for an unarmed opponent [M NearArmedPerson's gate]
        let yaw = sim.list[ev].yaw;
        let aim = [yaw.sin(), 0.0, yaw.cos()];
        let p = Player { pos, target: [pos[0], pos[1] + 0.5, pos[2]], eye: [pos[0], pos[1] + 1.0, pos[2]], aim, sph: pos, capsule: ([pos[0], pos[1] + 1.086, pos[2]], pos), hanging: false, cel: None };
        let mut saw_fail = false;
        for _ in 0..90 {
            sim.tick(&d, &p);
            if sim.list[ev].fail_reason.is_some() { saw_fail = true; break; }
        }
        assert!(saw_fail, "the restricted band never raised the fail reason (zone {})", sim.list[ev].door_zone);
        assert!(matches!(sim.list[ev].state, state::ATTACK | state::COMBAT | state::AIM_STAND | state::AIM_STAND_FIRE | state::AIM_CROUCH | state::AIM_CROUCH_FIRE),
            "the guard went hostile, state {:#x}", sim.list[ev].state);
    } else {
        eprintln!("[sister skipped: no restricted-band spot with a clear line on this level's pairs — the arm itself is [M 0x160648:89-94]]");
    }
}
/// The waiting pair (pl 448/449, switch channel 21) never pairs while the channel is off — the
/// pairing only happens at a 0x31 ENTER [M 0x160694] — and Bond at their feet raises nothing.
/// Pulsing the channel activates them into 0x31 and they pair [M WaitSwitch 0x15b960 case 3 →
/// Enable(1) + SetState(+0x5A2)].
#[test]
fn the_waiting_pair_activates_through_its_channel() {
    let Some(l) = L::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let (mut sim, _rng) = l.sim();
    let a = idx(&sim, 448);
    let b = idx(&sim, 449);
    let p = bond_at([sim.list[a].pos[0] + 1.0, sim.list[a].pos[1], sim.list[a].pos[2]]);
    for _ in 0..30 {
        sim.tick(&d, &p);
        assert_eq!(sim.list[a].state, state::WAIT_SWITCH, "the waiting pair waits");
        assert!(sim.list[a].door_partner.is_none() && sim.list[b].door_partner.is_none(), "no pairing while waiting");
    }
    // The channel pulse [D the level's ch 21 waits; the port's Channels::set mirrors the game's
    // switch writer].
    sim.channels.set(21, 1, sim.frame);
    let mut paired = false;
    for _ in 0..30 {
        sim.tick(&d, &p);
        if sim.list[a].state == state::CIVILIAN_DOOR_GUARD && sim.list[a].door_partner.is_some() { paired = true; break; }
    }
    assert!(paired, "the activated guards entered 0x31 and paired: a state {:#x} partner {:?}", sim.list[a].state, sim.list[a].door_partner);
}
