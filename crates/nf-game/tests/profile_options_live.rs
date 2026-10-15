//! profile-options (610): the codename options' runtime effects and the save round-trips, through
//! the real paths. Opt-in: set `NF_ISO` for the level-backed tests; the round-trip and the
//! volume-bridge tests run anywhere.
//!
//! Cites per effect live in `crates/nf-game/src/profile.rs` (`Options`) and
//! `docs/systems/profile-options.md`.

use nf_game::drone::{Data, Drones, Player, Rng};
use nf_game::profile::{Options, Profile};
use std::collections::HashMap;

/// Every profile field round-trips through the game's own block format (the bit widths of
/// `LS_MakePlrSettings` 0x1cd418 / `LS_MakeMPSettings` 0x1ce508 / `LS_MakeGlobalSettings`
/// 0x1cd9d0 / `LS_MakeCheats` 0x1ce260 [M]) with the CRC trailer intact.
#[test]
fn round_trips_every_field() {
    let mut p = Profile::default_codename("ROUNDTRIP");
    // Flip every field off its default (and the volumes to non-default values).
    p.plr.exists = true;
    p.plr.controller_style = 5;
    p.plr.ptp_option = 3;
    p.plr.auto_aim = false;
    p.plr.auto_aim_alt = true;
    p.plr.manual_aim_toggle = true;
    p.plr.auto_equip_best = false;
    p.plr.crouch_toggle = false;
    p.plr.rumble = false;
    p.plr.crosshair = false;
    p.plr.flashing_objects = false;
    p.plr.reserved_aux = true;
    p.plr.hud_auto_hide = true;
    p.mpsg.radar = false;
    p.mpsg.health_handicap = -75;
    p.gset.music_volume = 33;
    p.gset.sfx_volume = 12;
    p.gset.subtitles = true;
    p.gset.widescreen = true;
    p.gset.sfx_mode = 4;
    p.chet.cheat_info = true;

    let mut q = Profile::default_codename("ROUNDTRIP");
    q.decode(&p.encode()).expect("the mutated profile decodes");
    assert_eq!(q.plr, p.plr, "PlayerOptions round-trip");
    assert_eq!(q.mpsg, p.mpsg, "MpOptions round-trip");
    assert_eq!(q.gset.music_volume, 33);
    assert_eq!(q.gset.sfx_volume, 12);
    assert!(q.gset.subtitles && q.gset.widescreen);
    assert_eq!(q.gset.sfx_mode, 4);
    assert!(q.chet.cheat_info);
    // Sister: a hand-built PLRS block with the Manual-Aim bit set and +0x0C bit 1 set decodes to
    // manual_aim_toggle ∧ reserved_aux — the bit layout the game's own push order defines [M].
    let mut r = Profile::default_codename("BITS");
    r.plr = p.plr.clone();
    r.plr.manual_aim_toggle = true;
    r.plr.reserved_aux = false;
    let mut s = Profile::default_codename("BITS");
    s.decode(&r.encode()).unwrap();
    assert!(s.plr.manual_aim_toggle && !s.plr.reserved_aux);
}

/// The runtime snapshot: `Options::of` carries every field the pages write, and the defaults are
/// the game's boot values [M Input_Init 0x17edd8 / bootup 0x1c7e78].
#[test]
fn options_snapshot_matches_the_profile() {
    let mut p = Profile::default_codename("SNAP");
    p.plr.auto_aim = false;
    p.plr.crosshair = false;
    p.plr.crouch_toggle = false;
    p.plr.manual_aim_toggle = true;
    p.plr.auto_equip_best = false;
    p.plr.hud_auto_hide = true;
    p.plr.flashing_objects = false;
    p.plr.auto_aim_alt = true;
    p.mpsg.radar = false;
    p.mpsg.health_handicap = 50;
    p.gset.music_volume = 5;
    p.gset.sfx_volume = 6;
    p.gset.subtitles = true;
    p.gset.widescreen = true;
    p.gset.sfx_mode = 4;
    let o = Options::of(&p);
    assert!(!o.auto_aim && !o.crosshair && !o.crouch_toggle);
    assert!(o.manual_aim_toggle && !o.auto_equip_best && o.hud_auto_hide && !o.flashing_objects);
    assert!(o.mp_auto_aim && !o.radar && o.health_handicap == 50);
    assert_eq!(o.music_volume, 5);
    assert_eq!(o.sfx_volume, 6);
    assert!(o.subtitles && o.widescreen && o.sfx_mode == 4);
    // Sister: the defaults are the game's boot values (auto-aim on, crosshair on, crouch toggle,
    // manual-aim hold, auto-switch on, HUD always on, volumes 70/100).
    let d = Options::defaults();
    assert!(d.auto_aim && d.crosshair && d.crouch_toggle && !d.manual_aim_toggle);
    assert!(d.auto_equip_best && !d.hud_auto_hide && d.flashing_objects);
    assert_eq!(d.music_volume, 70);
    assert_eq!(d.sfx_volume, 100);
}

/// The crouch modes through the real physics tick (`Bond::tick_ctx`; the game picks the action
/// mode off `PlayerSetting+0x04` [M Player_SSCrouch 0x1924c0:85]). Level 07000002's flat
/// courtyard at (226, 2, 312) [L the same spot the jump tests use].
#[test]
fn crouch_toggle_flips_on_the_press_edge_and_hold_tracks_the_button() {
    let Some(path) = std::env::var_os("NF_ISO") else { eprintln!("skip: NF_ISO not set"); return };
    let iso = nf_formats::iso9660::IsoImage::open(&path).unwrap();
    let (entries, bin) = nf_formats::disc::files_bin_entries(&iso).unwrap();
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000002.bin")).unwrap();
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    let level = nf_formats::level::Level::parse(&data).unwrap();
    let world = nf_game::world::CollisionWorld::build(&level, &level.instances(true).0);

    let input_down = nf_game::player::Input { crouch: true, ..Default::default() };
    let idle = nf_game::player::Input::default();
    let pos = [226.0, 2.0, 312.0];

    // Hold mode (+0x04 = 0): crouched while held, standing the tick after release.
    {
        let mut b = nf_game::player::Bond::new(pos, 0.0);
        let ctx = nf_game::player::LadderCtx { crouch_toggle: false, ..Default::default() };
        for _ in 0..70 { b.tick_ctx(&world, &ctx, &input_down); } // ride out the 60-tick lockout
        assert!(b.crouched, "hold: crouched while the button is held");
        for _ in 0..70 { b.tick_ctx(&world, &ctx, &idle); }
        assert!(!b.crouched, "hold: standing again on release");
    }
    // Toggle mode (+0x04 = 1, the game's default): a press+release latches the crouch on; the
    // second press latches it off.
    {
        let mut b = nf_game::player::Bond::new(pos, 0.0);
        let ctx = nf_game::player::LadderCtx { crouch_toggle: true, ..Default::default() };
        b.tick_ctx(&world, &ctx, &input_down); // the press edge
        b.tick_ctx(&world, &ctx, &idle);       // released
        for _ in 0..70 { b.tick_ctx(&world, &ctx, &idle); }
        assert!(b.crouched, "toggle: still crouched after the button is released");
        for _ in 0..2 { b.tick_ctx(&world, &ctx, &input_down); } // the second press edge
        for _ in 0..70 { b.tick_ctx(&world, &ctx, &idle); }
        assert!(!b.crouched, "toggle: the second press stands Bond up");
        // Sister: holding the button in toggle mode does not re-crouch mid-hold (one edge = one flip).
        let mut b2 = nf_game::player::Bond::new(pos, 0.0);
        for _ in 0..75 { b2.tick_ctx(&world, &ctx, &input_down); }
        assert!(b2.crouched, "toggle: crouched on the press edge");
        // (the crouch stands only via the lockout+rising floor — a held button never re-toggles
        // because the edge is gone; crouch_prev latches true for the whole hold)
    }
}

struct Level02 {
    level: nf_formats::level::Level,
    tables: nf_formats::drone::DroneAnimData,
    props: nf_formats::drone::PropertyTables,
    weapons: nf_formats::weapon::WeaponData,
    scripts: HashMap<u32, nf_formats::anim::Script>,
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
    skins: HashMap<u32, nf_formats::character::Skin>,
}

impl Level02 {
    fn load() -> Option<Level02> {
        let path = std::env::var_os("NF_ISO")?;
        let iso = nf_formats::iso9660::IsoImage::open(&path).unwrap();
        let (entries, bin) = nf_formats::disc::files_bin_entries(&iso).unwrap();
        let read = |name: &str| -> Option<Vec<u8>> {
            let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name))?;
            Some(iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap())
        };
        let data = read("07000002.bin")?;
        let level = nf_formats::level::Level::parse(&data).ok()?;
        let lb = nf_formats::levelbin::LevelBin::parse(&data).unwrap();
        let skeletons: HashMap<u8, _> = lb.entries.iter().filter(|e| e.kind == 6)
            .filter_map(|e| nf_formats::character::parse_skeleton(lb.data(&data, e)).ok().map(|s| (s.id as u8, s))).collect();
        let mut sequences = HashMap::new();
        if let Some(anim) = read("07F00002.bin") {
            let alb = nf_formats::levelbin::LevelBin::parse(&anim).unwrap();
            for e in alb.entries.iter().filter(|e| e.kind == 4 && e.length >= 0x40) {
                if let Ok(s) = nf_formats::anim::Sequence::parse(alb.data(&anim, e)) { sequences.insert(s.hashcode, s); }
            }
        }
        let scripts: HashMap<u32, nf_formats::anim::Script> = lb.entries.iter().filter(|e| e.kind == 5)
            .filter_map(|e| nf_formats::anim::Script::parse(lb.data(&data, e)).ok()).map(|s| (s.hashcode, s)).collect();
        let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
        Some(Level02 {
            tables: nf_formats::drone::DroneAnimData::from_action_elf(&elf).unwrap(),
            props: nf_formats::drone::PropertyTables::from_action_elf(&elf).unwrap(),
            weapons: nf_formats::weapon::WeaponData::from_action_elf(&elf).unwrap(),
            world: nf_game::world::CollisionWorld::build(&level, &level.instances(true).0),
            nav: nf_game::nav::Network::build(level.maps[level.main_map().unwrap()].ai_network.as_ref().unwrap(), &nf_game::world::CollisionWorld::build(&level, &level.instances(true).0), 0x0700_0002),
            covers: nf_game::cover::build(&level.maps[level.main_map().unwrap()].placements, &nf_game::nav::Network::build(level.maps[level.main_map().unwrap()].ai_network.as_ref().unwrap(), &nf_game::world::CollisionWorld::build(&level, &level.instances(true).0), 0x0700_0002), &nf_game::world::CollisionWorld::build(&level, &level.instances(true).0)),
            sfx: nf_formats::sfx::SfxTable::from_action_elf(&elf).ok(),
            tuning: nf_formats::tuning::Tuning::default(),
            hitboxes: nf_formats::hitbox::CollisionBones::from_action_elf(&elf).unwrap(),
            volumes: nf_game::aivolume::build(&level, 0x0700_0002),
            models: nf_game::pickup::models(&level),
            skins: level.skins.clone(),
            level,
            scripts,
            sequences,
            skeletons,
        })
    }

    fn data(&self) -> Data<'_> {
        Data { tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world, level: 0x0700_0002, sequences: &self.sequences, skins: &self.skins, skeletons: &self.skeletons, nav: &self.nav, covers: &self.covers, tuning: &self.tuning, hitboxes: &self.hitboxes, volumes: &self.volumes, sfx: self.sfx.as_ref(), models: &self.models, surfaces: None, voices: None, difficulty: 1 }
    }

    fn sim(&self) -> Drones {
        let d = self.data();
        let mut rng = Rng::new(0x1234_5678);
        let mut lights = nf_game::light::LightList::default();
        let drones = nf_game::drone::level_drones(&d, &self.level, &mut rng, &mut lights);
        let mut s = Drones::new(drones, rng);
        s.attach_level(&self.level);
        s
    }
}

fn player_at(p: [f32; 3]) -> Player { Player { pos: p, target: [p[0], p[1] + 0.5, p[2]], eye: [p[0], p[1] + 1.0, p[2]], aim: [-1.0, 0.0, 0.0], ..Default::default() } }

/// The HUD pane fade timer BL+0x8BC [M Player_Update 0x193600:414-425]: pinned at 1.0 while the
/// profile's "HUD Always On" byte is 0 (the default), decayed toward 0 once the byte is 1 —
/// through `Drones::tick`, the function the game's own decay lives in.
#[test]
fn hud_fade_pins_and_decays_with_the_option() {
    let Some(l) = Level02::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let player = player_at([226.0, 2.0, 312.0]);

    let mut sim = l.sim();
    sim.options.hud_auto_hide = false; // the default byte: HUD Always On
    for _ in 0..600 { sim.tick(&d, &player); }
    assert_eq!(sim.hud_fade, 1.0, "HUD Always On pins the pane timer");

    // Sister: the auto-hide byte decays the timer (2 · 0.0020833334 per tick → ~0 in 240 ticks).
    let mut sim2 = l.sim();
    sim2.options.hud_auto_hide = true;
    for _ in 0..300 { sim2.tick(&d, &player); }
    assert_eq!(sim2.hud_fade, 0.0, "the auto-hide decay reaches the floor (240 ticks)");

    // Sister 2: part-way there it is strictly between 0 and 1 (a real decay, not a snap).
    let mut sim3 = l.sim();
    sim3.options.hud_auto_hide = true;
    for _ in 0..60 { sim3.tick(&d, &player); }
    assert!(sim3.hud_fade > 0.0 && sim3.hud_fade < 1.0, "the decay is gradual: {}", sim3.hud_fade);
}

/// The auto-aim bridge: the snapshot carries the flag and the Sim's default matches the game's
/// boot default [M Input_Init 0x17edd8 +1]. The lock itself (`Drones::auto_aim` 0x6649) is
/// flag-independent — the gate is the caller's `if self.auto_aim` (scene.rs), so the headless
/// test pins the bridge value the caller reads.
#[test]
fn auto_aim_bridge_follows_the_profile() {
    let mut p = Profile::default_codename("AIM");
    p.plr.auto_aim = false;
    assert!(!Options::of(&p).auto_aim);
    p.plr.auto_aim = true;
    assert!(Options::of(&p).auto_aim);
    // Sister: the Sim boots with the game's default (auto-aim on).
    let d = Options::defaults();
    assert!(d.auto_aim, "Input_Init's default is auto-aim on [M 0x17edd8]");
}
