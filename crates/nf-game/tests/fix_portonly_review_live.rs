//! G-046's PORT_ONLY trig stand-ins (exact-sine-portonly-review):
//! - the guard casing eject basis is the game's weapon-datum bone world matrix
//!   (`Bullet_init_casing_ex` 0x12f678:102-114, datum base+2 gated by `AnimDatumGetIndex`,
//!   fallback base+1), not the aim-direction yaw/pitch stand-in;
//! - the head-track ease-target measurement is object-space in the port and world-space in the
//!   game (`AnimObjectHeadTrack` 0x1bf618's two acos blocks) — the test proves the two agree
//!   (the object transform is a rigid yaw+translation, so every dot/cross/acos is invariant);
//! - the game's per-plane gates and behind gate (pitch skipped when the target is behind the
//!   head's forward in the vertical plane, 0x1bf618:222-236) — the port measured behind targets
//!   anyway and clamped.
//! Opt-in: set `NF_ISO` to the disc image. Level 07000005 throughout.

use nf_formats::anim::Script;
use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, drone, level::Level, levelbin::LevelBin, weapon::WeaponData};
use nf_game::drone::{Data, Drone, Drones, Player, Rng};
use std::collections::HashMap;

/// Everything `Data` borrows, owned (the ai_live.rs harness, trimmed of the unused loads).
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
        let mut s = Drones::new(drones, rng); s.attach_level(&self.level);
        s.difficulty = 1;
        s
    }

    fn data(&self) -> Data<'_> {
        Data { tables: &self.tables, props: &self.props, scripts: &self.scripts, weapons: &self.weapons, world: &self.world, level: 0x0700_0005,
            sequences: &self.sequences, skins: &self.level.skins, skeletons: &self.skeletons, nav: &self.nav, covers: &self.covers, tuning: &self.tuning, hitboxes: &self.hitboxes, volumes: &self.volumes,
            sfx: Some(&self.sfx), models: &self.models, surfaces: Some(&self.surfaces), voices: Some(&self.voices), difficulty: 1 }
    }

    /// One fresh guard (no ticks — the level's own placement list, first entry).
    fn fresh_guard(&self) -> (Drone, Rng) {
        let d = self.data();
        let mut rng = Rng::new(1);
        let mut lights = nf_game::light::LightList::default();
        let drones = nf_game::drone::level_drones(&d, &self.level, &mut rng, &mut lights);
        (drones.into_iter().find(|dr| !dr.corpse).unwrap(), rng)
    }
}

fn bond_at(p: [f32; 3]) -> Player { Player { pos: p, target: [p[0], p[1] + 0.5, p[2]], eye: [p[0], p[1] + 1.0, p[2]], aim: [-1.0, 0.0, 0.0], sph: p, capsule: ([p[0], p[1] + 1.086, p[2]], p), hanging: false, cel: None } }

/// `Bullet_init_casing_ex` 0x12f678:102-114 (the guard arm): the eject basis is the weapon datum's
/// bone world matrix — datum `+0x50 + 2` when the skin's attachment table carries it
/// (`AnimDatumGetIndex`), else `+1` (the muzzle datum) — so the casing's origin is that datum's
/// world position and its velocity is that matrix's rotation rows on the weapon-local eject vector
/// (`+0xf8, −+0xfc, +0x100`, the +0xfc negation at 0x12f83c) plus the `(0.01, 0.005)` MVar2 jitter.
/// The old stand-in built the basis from the shot's yaw/pitch instead.
#[test]
fn guard_casing_ejects_from_the_weapon_datum_matrix() {
    let Some(l) = Level05::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let (drones, gi, rng) = {
        let mut rng = Rng::new(5);
        let mut lights = nf_game::light::LightList::default();
        let drones = nf_game::drone::level_drones(&d, &l.level, &mut rng, &mut lights);
        let gi = drones.iter().position(|dr| (dr.pos[0] - 61.342).abs() < 0.01).unwrap();
        (drones, gi, rng)
    };
    let mut sim = l.sim(drones, rng);
    const WARM: u32 = 10;
    let away = bond_at([0.0, -500.0, 0.0]);
    for _ in 0..WARM { sim.tick(&d, &away); }
    let player = bond_at([67.31, 26.5, 99.47]);
    // Step until the first shot, then the shell's slot is the only non-dead casing.
    let mut shot_tick = None;
    for t in 0..400 {
        sim.tick(&d, &player);
        if !sim.list[gi].shots.is_empty() { shot_tick = Some(t); break; }
    }
    let shot_tick = shot_tick.expect("the guard must fire within 400 ticks (ai_live: 59-63 + burst)"); let _ = shot_tick;
    let fresh: Vec<&nf_game::casing::Casing> = sim.casings.list.iter().filter(|c| c.state != 2).collect();
    assert_eq!(fresh.len(), 1, "one shell in flight after the first shot");
    let c = fresh[0];
    let g = &sim.list[gi];
    // The game's eject basis, recomputed from the public data (Skin.datums + the posed palette).
    let m = g.eject_matrix(&d).expect("the guard's weapon datum must resolve");
    let w = &d.weapons.records[g.weapon as usize];
    let f = |o: usize| f32::from_le_bytes(w[o..o + 4].try_into().unwrap());
    let local = [f(0xf8), -f(0xfc), f(0x100)];
    let expected_e = [
        m[0][0] * local[0] + m[1][0] * local[1] + m[2][0] * local[2],
        m[0][1] * local[0] + m[1][1] * local[1] + m[2][1] * local[2],
        m[0][2] * local[0] + m[1][2] * local[1] + m[2][2] * local[2],
    ];
    // Origin: the datum's world position (no jitter rides the position, 0x12f678:160 Vec_Copy2).
    // The expected matrix is evaluated AFTER the tick while the eject read it at its instant inside
    // the tick (the guard's anim/position can advance past handle_firing in the same tick), so the
    // assert tolerates that one-tick pose drift.
    for k in 0..3 {
        assert!((c.pos[k] - m[3][k]).abs() < 5e-3, "casing pos {:?} vs eject datum world {:?}", c.pos, [m[3][0], m[3][1], m[3][2]]);
    }
    // Velocity: the matrix rows on the local vector + 3× MVar2(0.01, 0.005) ∈ [0.005, 0.015] each.
    for k in 0..3 {
        assert!((c.vel[k] - expected_e[k]).abs() <= 0.016, "casing vel[{}] {} vs datum-eject {} ± jitter", k, c.vel[k], expected_e[k]);
    }
    // Sister: the old aim-direction basis must NOT reproduce the same velocity (the test has teeth —
    // the datum matrix's rotation genuinely differs from the shot's yaw/pitch basis). A 90°-rotated
    // basis must also miss the measured velocity.
    let yaw = g.yaw; let pitch = 0.0f32;
    let (cy, sy) = (yaw.cos(), yaw.sin());
    let standin_fwd = [sy, pitch.sin(), cy];
    let standin_e = [standin_fwd[0] * local[1], standin_fwd[1] * local[1], standin_fwd[2] * local[1]];
    let datum_vs_standin: f32 = (0..3).map(|k| (expected_e[k] - standin_e[k]).abs()).fold(0.0, f32::max);
    assert!(datum_vs_standin > 0.05, "the datum basis must differ from a body-yaw basis (got {datum_vs_standin})");
    let (r90c, r90s) = (90f32.to_radians().cos(), 90f32.to_radians().sin());
    let rot90 = [[r90c, 0.0, -r90s], [0.0, 1.0, 0.0], [r90s, 0.0, r90c]];
    let wrong_e = [
        rot90[0][0] * local[0] + rot90[1][0] * local[1] + rot90[2][0] * local[2],
        rot90[0][1] * local[0] + rot90[1][1] * local[1] + rot90[2][1] * local[2],
        rot90[0][2] * local[0] + rot90[1][2] * local[1] + rot90[2][2] * local[2],
    ];
    let wrong_dist = ((expected_e[0] - wrong_e[0]).powi(2) + (expected_e[1] - wrong_e[1]).powi(2) + (expected_e[2] - wrong_e[2]).powi(2)).sqrt();
    assert!(wrong_dist > 0.05, "sister: a 90°-rotated basis must not reproduce the datum eject vector, dist {wrong_dist}");
}

/// The PORT_ONLY object-space head-track measurement (drone.rs `head_track_want`) must equal the
/// game's world-space measurement — `AnimObjectHeadTrack` 0x1bf618:189-281's two acos blocks run on
/// the WORLD palette's axes (neck = normalize(head − neck-base), side = neck×fwd, fwd = the head's
/// row 2). The port's pose is object-space and it un-rotates the target by −yaw; for the rigid
/// yaw+translation object transform every dot/cross/acos is rotation-invariant, so the two agree.
#[test]
fn head_track_want_matches_the_game_s_world_space_measurement() {
    let Some(l) = Level05::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let (g, _rng) = l.fresh_guard();
    assert_eq!(g.head_yaw, 0.0, "fresh guard: no stored head rotation");
    let pal = g.palette(&d).expect("the fresh guard must pose");
    let head = [pal[5][3][0], pal[5][3][1], pal[5][3][2]];
    let neckv = [head[0] - pal[2][3][0], head[1] - pal[2][3][1], head[2] - pal[2][3][2]];
    let fwd = [pal[5][2][0], pal[5][2][1], pal[5][2][2]];
    let cross = |a: [f32; 3], b: [f32; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    let unit = |a: [f32; 3]| -> Option<[f32; 3]> { let s = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt(); if s < 1e-9 { None } else { Some([a[0] / s, a[1] / s, a[2] / s]) } };
    let neck = unit(neckv).unwrap();
    let side = unit(cross(neck, fwd)).unwrap();
    let fu = unit(fwd).unwrap();
    // The game's two acos blocks on the world axes (0x1bf618:189-281), port sign conventions out.
    let measure = |target: [f32; 3]| -> (f32, f32) {
        let tv = [target[0] - head[0], target[1] - head[1], target[2] - head[2]];
        // Yaw block: the (side, fwd) plane — remove the neck component (0x1bf618:241-281).
        let dn = tv[0] * neck[0] + tv[1] * neck[1] + tv[2] * neck[2];
        let to_y = [tv[0] - neck[0] * dn, tv[1] - neck[1] * dn, tv[2] - neck[2] * dn];
        let raw_yaw = match unit(to_y) {
            Some(to_y) if to_y[0] * to_y[0] + to_y[1] * to_y[1] + to_y[2] * to_y[2] >= 0.0 => {
                let c = (fu[0] * to_y[0] + fu[1] * to_y[1] + fu[2] * to_y[2]).clamp(-1.0, 1.0);
                let mut y = c.acos();
                let cf = cross(fu, to_y);
                if cf[0] * neck[0] + cf[1] * neck[1] + cf[2] * neck[2] < 0.0 { y = -y; }
                y
            }
            _ => 0.0,
        };
        // Pitch block: the (neck, fwd) plane — remove the side component (0x1bf618:192-236), the
        // game's sign positive down, skipped when the target is behind the forward.
        let ds = tv[0] * side[0] + tv[1] * side[1] + tv[2] * side[2];
        let to_p = [tv[0] - side[0] * ds, tv[1] - side[1] * ds, tv[2] - side[2] * ds];
        let raw_pitch = match unit(to_p) {
            Some(to_p) => {
                let c = (fu[0] * to_p[0] + fu[1] * to_p[1] + fu[2] * to_p[2]).clamp(-1.0, 1.0);
                if c < 0.0 { 0.0 } else {
                    let mut p = c.acos();
                    let det = fwd[0] * (to_p[1] * side[2] - to_p[2] * side[1]) - fwd[1] * (to_p[0] * side[2] - to_p[2] * side[0]) + fwd[2] * (to_p[0] * side[1] - to_p[1] * side[0]);
                    if det < 0.0 { p = -p; }
                    -p // the port stores pitch positive up
                }
            }
            _ => 0.0,
        };
        // The game's clamps (0x1bf618:290-298): yaw 0 beyond 2.6 else ±1.1; pitch [-1.0, +0.6] in
        // the game's positive-down convention.
        let yaw = if raw_yaw.abs() > 2.6 { 0.0 } else { raw_yaw.clamp(-1.1, 1.1) };
        let pitch = raw_pitch.clamp(-1.0, 0.6);
        (yaw, pitch)
    };
    // Targets around the guard's head: ~7.5° right of the forward, 45° right, up-front, down-front,
    // 45° left. (Exactly dead ahead is degenerate for the acos sign — cross of near-parallel
    // vectors — so the off-axis target stands in for it.)
    let targets = [
        [head[0] + fu[0] * 1.9 + side[0] * 0.25, head[1] + fu[1] * 1.9 + side[1] * 0.25, head[2] + fu[2] * 1.9 + side[2] * 0.25],
        [head[0] + (fu[0] * 0.7 + side[0] * 0.7) * 2.0, head[1] + (fu[1] * 0.7 + side[1] * 0.7) * 2.0, head[2] + (fu[2] * 0.7 + side[2] * 0.7) * 2.0],
        [head[0] + fu[0] * 1.8 + neck[0] * 0.9, head[1] + fu[1] * 1.8 + neck[1] * 0.9, head[2] + fu[2] * 1.8 + neck[2] * 0.9],
        [head[0] + fu[0] * 1.8 - neck[0] * 0.9, head[1] + fu[1] * 1.8 - neck[1] * 0.9, head[2] + fu[2] * 1.8 - neck[2] * 0.9],
        [head[0] + (fu[0] * 0.7 - side[0] * 0.7) * 2.0, head[1] + (fu[1] * 0.7 - side[1] * 0.7) * 2.0, head[2] + (fu[2] * 0.7 - side[2] * 0.7) * 2.0],
    ];
    for t in targets {
        let want = g.head_track_want(&d, t).expect("in-range target must measure");
        let (wy, wp) = measure(t);
        // Near-zero yaw (a target in the (fwd, neck) plane, e.g. the up-front one) is degenerate for
        // the acos SIGN — cross of near-parallel vectors — in the game's own formula too, so the
        // comparison there is magnitude-only; elsewhere the signed values must match to 1e-3.
        let yaw_ok = (want.0 - wy).abs() < 1e-3 || (want.0.abs() < 0.05 && wy.abs() < 0.05);
        assert!(yaw_ok, "yaw: port {} vs world measurement {} at {:?}", want.0, wy, t);
        assert!((want.1 - wp).abs() < 1e-3, "pitch: port {} vs world measurement {} at {:?}", want.1, wp, t);
    }
    // Sister: dead-ahead-ish must measure a small positive yaw (the axes, not the object, define
    // the zero), 45° right ≈ +π/4, and the 45° LEFT target must mirror it — sign teeth.
    let (y_near, _) = measure(targets[0]);
    let (y_right, _) = measure(targets[1]);
    let (y_left, _) = measure(targets[4]);
    assert!(y_near.abs() < 0.2, "near-forward yaw small, got {y_near}");
    assert!((y_right - std::f32::consts::FRAC_PI_4).abs() < 1e-2, "45° right yaw ≈ +π/4, got {y_right}");
    assert!((y_left + y_right).abs() < 1e-2, "45° left must mirror 45° right, got {y_left} vs {y_right}");
}

/// The game skips the pitch measurement when the target is BEHIND the head's forward in the vertical
/// plane (0x1bf618:222-236: `dot(unit(fwd), unit(toTarget)) < 0` → pitch stays 0) and gates each
/// PLANE-projected distance at 0.25 separately (:209-210, :257-258). The port used to measure behind
/// targets anyway and clamp them. Fail-first: an above-behind target must want pitch 0 (the head
/// returns to level), not the measured/clamped value.
#[test]
fn head_track_pitch_stays_level_for_behind_targets() {
    let Some(l) = Level05::load() else { eprintln!("skip: NF_ISO not set"); return };
    let d = l.data();
    let (g, _rng) = l.fresh_guard();
    let pal = g.palette(&d).expect("the fresh guard must pose");
    let head = [pal[5][3][0], pal[5][3][1], pal[5][3][2]];
    let neckv = [head[0] - pal[2][3][0], head[1] - pal[2][3][1], head[2] - pal[2][3][2]];
    let fwd = [pal[5][2][0], pal[5][2][1], pal[5][2][2]];
    let nl = (neckv[0] * neckv[0] + neckv[1] * neckv[1] + neckv[2] * neckv[2]).sqrt();
    let neck = [neckv[0] / nl, neckv[1] / nl, neckv[2] / nl];
    // Above and behind: 1 unit behind the head's forward, 1.2 up the neck — well past the 0.25
    // plane-projection gates, so only the behind gate can zero the pitch.
    let behind = [head[0] - fwd[0] + neck[0] * 1.2, head[1] - fwd[1] + neck[1] * 1.2, head[2] - fwd[2] + neck[2] * 1.2];
    let want = g.head_track_want(&d, behind).expect("the target is in range");
    assert!(want.1.abs() < 1e-6, "behind target: the game's pitch stays 0, the port measured {}", want.1);
    // Sister: the mirrored FRONT target (1 ahead, 1.2 up) must still measure an up pitch — the gate
    // must not have swallowed the whole pitch block.
    let front = [head[0] + fwd[0] + neck[0] * 1.2, head[1] + fwd[1] + neck[1] * 1.2, head[2] + fwd[2] + neck[2] * 1.2];
    let want_front = g.head_track_want(&d, front).expect("the target is in range");
    assert!(want_front.1 > 0.1, "front-above target must measure an up pitch, got {}", want_front.1);
}
