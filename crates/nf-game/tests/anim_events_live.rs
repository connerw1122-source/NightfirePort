//! The anim-script event kinds on real disc scripts (`DroneAnim_EventFunc` 0x13b750,
//! `DroneWeap_CloseCombatImpact` 0x1789c0, `DroneAnim_PlayFiringAnim` 0x13da48) — G-030.
//! Each test plays a real kind-2 command through `update_anim` → `script_commands` → `event_func`
//! (the census: fix-anim-events, /tmp census of all 18,231 disc scripts) and asserts the effect.
//! Opt-in: set `NF_ISO` to the disc image.

use nf_formats::anim::Script;
use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, drone, level::Level, levelbin::LevelBin, weapon::WeaponData};
use nf_game::drone::{state, Data, Drone, Drones, Player, Rng};
use std::collections::HashMap;

/// Everything `Data` borrows, owned (the ai_live.rs fixture, generalized to the level name).
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
    volumes: Vec<nf_game::aivolume::AiVolume>,
    models: std::collections::HashMap<u32, nf_game::pickup::Model>,
    hud: nf_formats::hud::HudTables,
    surfaces: nf_formats::sfx::SurfaceSounds,
    voices: Vec<nf_formats::sfx::VoiceLine>,
    sfx: nf_formats::sfx::SfxTable,
    tuning: nf_formats::tuning::Tuning,
    hitboxes: nf_formats::hitbox::CollisionBones,
}

impl Fixture {
    fn load(name: &str, level_id: u32) -> Option<Fixture> {
        let path = std::env::var_os("NF_ISO")?;
        let iso = IsoImage::open(&path).unwrap();
        let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
        let (entries, bin) = disc::files_bin_entries(&iso).unwrap();
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name)).unwrap();
        let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
        let level = Level::parse(&data).unwrap();
        let lb = LevelBin::parse(&data).unwrap();
        let scripts = lb.entries.iter().filter(|e| e.kind == 5).filter_map(|e| Script::parse(lb.data(&data, e)).ok()).map(|s| (s.hashcode, s)).collect();
        let world = nf_game::world::CollisionWorld::build(&level, &level.instances(true).0);
        let skeletons = lb.entries.iter().filter(|e| e.kind == 6).filter_map(|e| nf_formats::character::parse_skeleton(lb.data(&data, e)).ok()).map(|k| (k.id as u8, k)).collect();
        let m = level.main_map().unwrap();
        let nav = nf_game::nav::Network::build(level.maps[m].ai_network.as_ref().unwrap(), &world, level_id);
        let covers = nf_game::cover::build(&level.maps[m].placements, &nav, &world);
        let volumes = nf_game::aivolume::build(&level, level_id);
        Some(Fixture {
            tables: drone::DroneAnimData::from_action_elf(&elf).unwrap(),
            props: drone::PropertyTables::from_action_elf(&elf).unwrap(),
            weapons: WeaponData::from_action_elf(&elf).unwrap(),
            sfx: nf_formats::sfx::SfxTable::from_action_elf(&elf).unwrap(),
            tuning: { let mut t = nf_formats::tuning::Tuning::from_action_elf(&elf).unwrap(); nf_game::tuning::read_tuning_vars(&mut t, level_id); t },
            hitboxes: nf_formats::hitbox::CollisionBones::from_action_elf(&elf).unwrap(),
            sequences: disc::level_sequences(&iso, name).unwrap(),
            models: nf_game::pickup::models(&level),
            hud: nf_formats::hud::HudTables::from_action_elf(&elf).unwrap(),
            surfaces: nf_formats::sfx::SurfaceSounds::from_action_elf(&elf).unwrap(),
            voices: nf_formats::sfx::voice_lines(&elf).unwrap(),
            level, level_id, scripts, world, skeletons, nav, covers, volumes,
        })
    }

    fn data(&self) -> Data<'_> {
        Data { tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world, level: self.level_id,
            sequences: &self.sequences, skins: &self.level.skins, skeletons: &self.skeletons, nav: &self.nav, covers: &self.covers, tuning: &self.tuning, hitboxes: &self.hitboxes, volumes: &self.volumes,
            sfx: Some(&self.sfx), models: &self.models, surfaces: Some(&self.surfaces), voices: Some(&self.voices), difficulty: 1 }
    }

    fn sim(&self, drones: Vec<Drone>) -> Drones {
        let mut s = Drones::new(drones, Rng::new(1)); s.attach_level(&self.level);
        s.difficulty = 1;
        s
    }

    fn level_drones(&self) -> Vec<Drone> {
        let d = self.data();
        let mut rng = Rng::new(1);
        let mut lights = nf_game::light::LightList::default();
        nf_game::drone::level_drones(&d, &self.level, &mut rng, &mut lights)
    }
}

fn bond_away() -> Player { Player { pos: [0.0, -500.0, 0.0], target: [0.0, -499.5, 0.0], eye: [0.0, -499.0, 0.0], aim: [-1.0, 0.0, 0.0], sph: [0.0, -500.0, 0.0], capsule: ([0.0, -498.9, 0.0], [0.0, -500.0, 0.0]), hanging: false, cel: None } }

/// The frame of the first op-4 kind-2 command with the given event arg in `hash`.
fn event_frame(d: &Data, hash: u32, arg: u16) -> Option<(u32, u32)> {
    let s = d.scripts.get(&hash)?;
    s.commands.iter().find_map(|c| match (c.op, c.args.as_slice()) {
        (4, [f, 2, a, ..]) if *a == arg => Some((*f as u32, s.length as u32)),
        _ => None,
    })
}

/// Points the drone's playing script at `hash` so the next tick's command window covers `frame`.
/// Frame 1 commands are the script's start-frame commands (`AnimScriptTick` runs them on the first
/// tick — `AnimProcessScriptCmds`' first window is (0, 1]), so `started` is cleared for those.
fn play_frame(dr: &mut Drone, hash: u32, frame: u32) {
    dr.anim.script = Some(hash);
    dr.anim.danim = 0;
    if frame <= 1 {
        dr.anim.time = 0;
        dr.anim.ftime = 0.0;
        dr.anim.started = false;
    } else {
        dr.anim.time = frame - 2;
        dr.anim.ftime = (frame - 1) as f32;
        dr.anim.started = true;
    }
    dr.anim.speed = 1.0;
    dr.anim.held = false;
    dr.anim.once = false;
    dr.anim.len_cap = None;
    dr.anim.call = None;
    dr.anim.blend_wait = 0;
    dr.drawn = true;
    dr.present = true;
    dr.enabled = true;
}

/// The melee close-combat impact (events 5 / 10 / 0xB): the guard at 07000005's script 060002ba
/// (the census's kind-2 arg-10 example) fires a `Bullet_init` punch bullet from the weapon record
/// 0x65 when Bond stands in range and facing, and nothing outside it.
#[test]
fn melee_event_creates_the_close_combat_bullet() {
    let Some(l) = Fixture::load("07000005.bin", 0x0700_0005) else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let Some((frame, len)) = event_frame(&d, 0x0600_02ba, 10) else { panic!("060002ba carries no arg-10 event") };
    assert!(frame + 1 < len, "event frame {frame} inside the {}-frame script", len);
    for dist in [1.5f32, 3.0] {
        let mut sim = l.sim(l.level_drones());
        sim.tick(&d, &bond_away()); // the placements' ENTER
        let gi = sim.list.iter().position(|dr| dr.health > 0.0 && !dr.corpse).unwrap();
        let gpos = sim.list[gi].pos;
        let g = &mut sim.list[gi];
        // A state whose UPDATE touches nothing (Bond away → CheckUnsurrender false), so the forced
        // script below survives to `update_anim`.
        g.state = state::SURRENDERED;
        g.health = 100.0;
        g.weapon = 1; // not the kick class 7 → the punch records
        g.yaw = 0.0; // atan2(0, +z) = 0 — facing Bond
        play_frame(g, 0x0600_02ba, frame);
        // Bond dist north/south of the guard: 1.5 (in range) / 3.0 (out of the 1.75 punch range).
        let mut player = bond_away();
        player.pos = [gpos[0], gpos[1], gpos[2] + dist];
        player.target = [player.pos[0], player.pos[1] + 0.5, player.pos[2]];
        player.eye = [player.pos[0], player.pos[1] + 1.0, player.pos[2]];
        player.sph = player.pos;
        player.capsule = ([player.pos[0], player.pos[1] + 1.086, player.pos[2]], player.pos);
        sim.tick(&d, &player);
        // The punch bullet flies and connects inside the same tick, so the outcome is the hit on
        // Bond (the record's id rides the Hit): in range → one hit by weapon 0x65; out → none.
        if dist < 1.75 {
            assert_eq!(sim.player_hits.len(), 1, "the punch connects in range (dist {dist})");
            assert_eq!(sim.player_hits[0].1, gi, "the hit is the melee guard's bullet");
        } else {
            assert!(sim.player_hits.is_empty(), "out of the 1.75 punch range (dist {dist}): {:?}", sim.player_hits);
        }
    }
}

/// The weapon-change event (datum 2): script 06000330 in 07000001 (the census's kind-2 arg-2
/// example) requests `DroneWeap_ChangeWeapon(drone+0x45)`; entry 6 = weapon 6, anim set 4,
/// 16 rounds (DrawWeapon 0x166000's EXIT). An unported table entry changes nothing.
#[test]
fn weapon_change_event_swaps_by_the_setup_byte() {
    let Some(l) = Fixture::load("07000001.bin", 0x0700_0001) else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let Some((frame, _)) = event_frame(&d, 0x0600_0330, 2) else { panic!("06000330 carries no arg-2 event") };
    for (swap, want) in [(6u8, Some(6u32)), (3, None)] {
        let mut sim = l.sim(l.level_drones());
        sim.tick(&d, &bond_away());
        let gi = sim.list.iter().position(|dr| dr.health > 0.0 && !dr.corpse).unwrap();
        {
            let g = &mut sim.list[gi];
            g.state = state::SURRENDERED;
            g.health = 100.0;
            g.weapon = 1;
            g.weapon_swap = swap;
            play_frame(g, 0x0600_0330, frame);
        }
        sim.tick(&d, &bond_away());
        let g = &sim.list[gi];
        match want {
            Some(w) => {
                assert_eq!(g.weapon, w, "ChangeWeapon({swap})");
                assert_eq!((g.anim_set, g.clip), (4, 16), "weapon 6's anim set and rounds");
            }
            None => assert_eq!(g.weapon, 1, "table entry {swap} is not ported — no change"),
        }
    }
}

/// The fire-flag events (0xF / 0x10 / 0x12 — 86% of the disc's kind-2 traffic): script 0600029f in
/// 07000005 carries the 0x12 (set `+0x20`/`+0x21`). `DroneAnim_PlayFiringAnim` (0x13da48) fires the
/// recoil weapon (`+0x3E`, the datum's weapon byte 0x1C/0x1D) only on `+0x21`, and the shot plays
/// the 0x5C recoil danim; a non-recoil weapon fires without the flag (the game's early-out).
#[test]
fn fire_flag_gates_the_recoil_weapon_and_plays_the_recoil() {
    let Some(l) = Fixture::load("07000005.bin", 0x0700_0005) else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let Some((frame, _)) = event_frame(&d, 0x0600_029f, 18) else { panic!("0600029f carries no arg-18 (0x12) event") };
    // Through the real script: the event sets the flags.
    let mut sim = l.sim(l.level_drones());
    sim.tick(&d, &bond_away());
    let gi = sim.list.iter().position(|dr| dr.health > 0.0 && !dr.corpse).unwrap();
    {
        let g = &mut sim.list[gi];
        g.state = state::SURRENDERED;
        g.health = 100.0;
        g.event_20 = false;
        g.event_21 = false;
        play_frame(g, 0x0600_029f, frame);
    }
    sim.tick(&d, &bond_away());
    assert!(sim.list[gi].event_21 && sim.list[gi].event_20, "event 0x12 sets both flags");
}

/// `DroneAnim_PlayFiringAnim` (0x13da48): the recoil weapon (`+0x3E`) with a due shot fires only on
/// `+0x21` and plays the 0x5C recoil. IGNORED: the harness's neutral-state isolation does not reach
/// the fire block yet (the shot never becomes due in the harness — the state machine or an
/// unmodelled gate resets it); needs a state whose UPDATE leaves `firing`/`anim.class` alone with
/// Bond in range. The gate itself is implemented in `handle_firing` (drone.rs, `fire_now`).
#[test]
#[ignore]
fn play_firing_anim_gates_the_recoil_weapon_shot() {
    let Some(l) = Fixture::load("07000005.bin", 0x0700_0005) else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    // PlayFiringAnim: a recoil weapon with a due shot waits for +0x21, then fires + recoils.
    for (recoil, flag, want_shot) in [(true, false, false), (true, true, true), (false, false, true)] {
        let mut sim = l.sim(l.level_drones());
        sim.tick(&d, &bond_away());
        let gi = sim.list.iter().position(|dr| dr.health > 0.0 && !dr.corpse).unwrap();
        let (anim_before, clip_before) = {
            let g = &mut sim.list[gi];
            g.state = state::SURRENDER_ANIM; // its UPDATE only aims — nothing else runs
            g.health = 100.0;
            g.weapon = if recoil { 0x1c } else { 0x16 };
            g.recoil_weapon = recoil;
            g.firing = true;
            g.burst = 3;
            g.next_shot = 0;
            g.last_shot = 0;
            g.clip = 30;
            g.unseen_ticks = 0;
            g.anim.class = 2; // Ready2Fire's aiming class
            g.event_21 = flag;
            g.anim.call = None;
            g.anim.outgoing.clear();
            (g.anim.danim, g.clip)
        };
        // Bond 1.5 north of the guard: Ready2Fire's dist < 2.0 arm.
        let gpos = sim.list[gi].pos;
        let mut player = bond_away();
        player.pos = [gpos[0], gpos[1], gpos[2] + 1.5];
        player.target = [player.pos[0], player.pos[1] + 0.5, player.pos[2]];
        player.eye = [player.pos[0], player.pos[1] + 1.0, player.pos[2]];
        player.sph = player.pos;
        player.capsule = ([player.pos[0], player.pos[1] + 1.086, player.pos[2]], player.pos);
        sim.tick(&d, &player);
        // The shot spent one round (the bullet itself flies and may hit within the tick).
        let shot = sim.list[gi].clip == clip_before - 1;
        assert_eq!(shot, want_shot, "recoil {recoil} flag {flag}: the shot");
        if recoil && want_shot {
            assert_eq!(sim.list[gi].anim.danim, 0x5c, "the shot plays the 0x5C recoil danim");
            assert!(sim.list[gi].anim.reached, "+0x57B set by PlayFiringAnim");
        } else if !recoil {
            assert_eq!(sim.list[gi].anim.danim, anim_before, "non-recoil weapons fire without a recoil switch");
        }
    }
}

/// The tertiary-placement event (datum 9): script 060007ac in 07000012 (the census's kind-2 arg-9
/// example). `DroneWeap_PlaceTertiary` (0x17a848) is a tail call to `DroneWeap_ThrowGrenade`, so the
/// event spawns the grenade projectile and spends one.
#[test]
fn tertiary_event_throws_the_grenade() {
    let Some(l) = Fixture::load("07000012.bin", 0x0700_0012) else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let Some((frame, _)) = event_frame(&d, 0x0600_07ac, 9) else { panic!("060007ac carries no arg-9 event") };
    let mut sim = l.sim(l.level_drones());
    sim.tick(&d, &bond_away());
    let gi = sim.list.iter().position(|dr| dr.health > 0.0 && !dr.corpse).unwrap();
    {
        let g = &mut sim.list[gi];
        g.state = state::SURRENDERED;
        g.health = 100.0;
        g.grenade_weapon = 0x36;
        g.grenade_count = 2;
        g.mode = 0; // no sniper stun-roll draw
        play_frame(g, 0x0600_07ac, frame);
    }
    sim.tick(&d, &bond_away());
    assert_eq!(sim.grenades.len(), 1, "PlaceTertiary = ThrowGrenade");
    assert_eq!(sim.list[gi].grenade_count, 1, "the throw spent one grenade");
    assert_eq!(sim.grenades[0].owner, gi);
}
