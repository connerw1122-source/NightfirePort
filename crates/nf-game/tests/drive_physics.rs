//! drive-physics headless tests: the tuning DB parsers, the gearbox/RPM, the steering falloff,
//! the drive/brake/reverse accelerations, the gravity bodge, and the flat-surface run — every
//! assertion cites the DRIVING.ELF address (docs/systems/drive-physics.md is the map).

use nf_game::driving::{Car, CarInput, CarParams, FlatWorld, Rigid, Tuning, DT};

fn vanquish() -> Car {
    Car::new(CarParams::vanquish(), [0.0; 3], 0.0)
}

/// The .tun parser keeps the game's `KEY=VALUE` reading [M FUN_001426a8: substring find +
/// sscanf %f; a trailing non-numeric stops the scan]. Sister: an absent key keeps the
/// compiled-in default (the game's no-match arm) — asserted via from_tun below.
#[test]
fn tun_parser_reads_key_value_lines() {
    let t = Tuning::parse("GRAVITY=-34\r\nSLEEP_VEL=5\r\nGROUND_FRICTION_COEFF=0.05\r\n\r\nBROKEN\r\nM_LIMIT=100 abc\r\n");
    assert_eq!(t.get("GRAVITY"), Some(-34.0));
    assert_eq!(t.get("SLEEP_VEL"), Some(5.0));
    assert_eq!(t.get("GROUND_FRICTION_COEFF"), Some(0.05));
    assert_eq!(t.get("M_LIMIT"), Some(100.0), "%f stops at the first non-numeric char");
    assert_eq!(t.get("AM_LIMIT"), None, "an absent key is absent");
    assert_eq!(t.get("BROKEN"), None, "no '=' — no value");
}

/// The Rigid defaults equal the shipped `Physics\Rigid\default.tun` [D; extracted from
/// MIS01.VIV] — the from_tun override keeps everything when the file matches. Sister: an
/// override with a DIFFERENT value lands (the registered-var assignment [M 0x1426a8]).
#[test]
fn rigid_defaults_match_the_shipped_tun_and_overrides_land() {
    let r = Rigid::game_defaults();
    assert_eq!(r.gravity, -34.0, "GRAVITY .data 0x3390d0 [M]");
    assert_eq!(r.bodge_player_gravity, -13.0, "BODGE_PLAYER_GRAVITY 0x3390d4 [M]");
    assert_eq!(r.sleep_vel, 5.0, "SLEEP_VEL 0x339100 [M]");
    assert_eq!(r.ground_friction_coeff, 0.05, "GROUND_FRICTION_COEFF 0x3390e8 [M]");
    assert_eq!(r.natural_angular_damping, 0.995, "NATURAL_ANGULAR_DAMPING 0x3390fc [M]");
    assert_eq!(r.m_limit, 100.0, "M_LIMIT 0x339108 [M]");
    assert_eq!(r.am_limit, 10.0, "AM_LIMIT 0x339104 [M]");
    assert_eq!(r.total_force_limit, 40.0, "TOTAL_FORCE_LIMIT 0x33911c [M]");
    let t = Tuning::parse("GRAVITY=-20\r\n");
    let r2 = Rigid::from_tun(&t);
    assert_eq!(r2.gravity, -20.0, "the tun value wins when the name matches [M]");
    assert_eq!(r2.bodge_player_gravity, -13.0, "and the rest keeps the default [M no-match arm]");
}

/// The vanquish .atr constants [D data\sim\attrib\pvehicle\vanquish.atr] — the numbers the
/// physics asserts below build on. Sister: the parse_atr path reads the SAME file text.
#[test]
fn vanquish_params_and_the_atr_parser_agree() {
    let v = CarParams::vanquish();
    let text = "[vanquish]\r\nMASS.f=1660.0\t\t\r\nMAXACC=3\r\nMAXBRAKE=3.15\r\nMAXSTEERING=0.09\r\n\
                MODSTEERSPEED=10.0       \r\nMINSTEER=0.425\r\nGEAR_LIMIT1=18.0\r\nGEAR_LIMIT5=69.0\r\n\
                GEAR_RATIO1=0.65 // five-speed\r\nCAR_WHEEL_ZF_OFFSET.f=-0.894\r\nSFX_ENGINE.s=vanquish\r\n";
    let p = CarParams::parse_atr(text);
    assert_eq!(p.mass, v.mass, "the .f suffix and the trailing tab parse");
    assert_eq!(p.maxacc, v.maxacc);
    assert_eq!(p.maxbrake, v.maxbrake);
    assert_eq!(p.maxsteering, v.maxsteering);
    assert_eq!(p.modsteer_speed, v.modsteer_speed);
    assert_eq!(p.min_steer, v.min_steer);
    assert_eq!(p.gear_limit[1], v.gear_limit[1]);
    assert_eq!(p.gear_limit[5], v.gear_limit[5]);
    assert_eq!(p.gear_ratio[1], v.gear_ratio[1], "the // comment strips");
    assert_eq!(p.wheel_zf_offset, v.wheel_zf_offset, "negative values parse");
    assert_eq!(p.maxrevacc, CarParams::game_defaults().maxrevacc, "an absent key keeps the schema default [M]");
    assert_eq!(v.gear_limit, [10.0, 18.0, 29.0, 50.0, 54.0, 69.0], "the shipped gearbox [D]");
    assert_eq!(v.gear_ratio, [1.0, 0.65, 0.59, 0.52, 0.45, 0.44], "[D]");
    assert!((v.wheel_zr_offset - v.wheel_zf_offset - 1.9).abs() < 1e-5, "the vanquish wheelbase [D]");
}

/// The auto-shift walk [M FUN_0018aa88:36-48]: gear g holds |v| ∈ [LIMIT(g−1), LIMIT(g)) for
/// vanquish's 18/29/50/54 limits; gear 5 from 54 up. Sister: the RPM denominator is the walk's
/// LAST-READ limit [M :62] — a wrong denominator fails the paired assert.
#[test]
fn gears_shift_at_the_vanquish_limits() {
    let mut car = vanquish();
    for (v, gear) in [(0.0, 1u8), (17.9, 1), (18.0, 2), (28.9, 2), (29.0, 3), (49.9, 3), (50.0, 4), (53.9, 4), (54.0, 5), (69.0, 5)] {
        car.vel = [v, 0.0, 0.0];
        car.update_rpm_gear(1.0, true);
        assert_eq!(car.gear, gear, "|v| = {v} m/s");
        car.last_gear = car.gear;
        car.shift_inhibit = 0;
    }
    // The RPM formula [M 0x18aa88:62]: rpm = |v|·6000/LIMIT[gear] with the walk's last limit.
    car.vel = [10.0, 0.0, 0.0];
    car.update_rpm_gear(1.0, true);
    assert!((car.rpm - 10.0 * 6000.0 / 18.0).abs() < 1e-3, "gear 1 revs to its own limit's base, got {}", car.rpm);
    car.vel = [20.0, 0.0, 0.0];
    car.update_rpm_gear(1.0, true);
    assert!((car.rpm - 20.0 * 6000.0 / 29.0).abs() < 1e-3, "gear 2's denominator is LIMIT2, got {}", car.rpm);
    car.vel = [60.0, 0.0, 0.0];
    car.update_rpm_gear(1.0, true);
    assert!((car.rpm - 60.0 * 6000.0 / 54.0).abs() < 1e-3, "gear 5's denominator is LIMIT4 (the loop's break order), got {}", car.rpm);
}

/// The neutral rev [M FUN_0018aa88:31-33 — the +0x299 == 0 arm: rpm = throttle·7000]. Sister:
/// the driving branch does NOT return that value at speed.
#[test]
fn rpm_revving_in_neutral_is_throttle_times_7000() {
    let mut car = vanquish();
    car.update_rpm_gear(0.5, false);
    assert_eq!(car.rpm, 3500.0);
    car.update_rpm_gear(1.0, false);
    assert_eq!(car.rpm, 7000.0);
    car.vel = [10.0, 0.0, 0.0];
    car.update_rpm_gear(1.0, true);
    assert!((car.rpm - 10.0 * 6000.0 / 18.0).abs() < 1e-3, "the driving branch uses the gearbox formula");
}

/// The steering falloff [M FUN_0018cef8:458-474]: full MAXSTEERING at ≤ MODSTEERSPEED (vanquish
/// 10 m/s), then 1 − (speed − 10)/25 floored at MINSTEER (0.425). Sister: the floor value —
/// 0.09·0.425 = 0.03825 — is exact.
#[test]
fn steering_falls_off_to_the_min_steer_floor() {
    let mut car = vanquish();
    car.speed = 5.0;
    car.update_steering(1.0);
    assert!((car.steer_angle - 0.09).abs() < 1e-6, "full lock at 5 m/s, got {}", car.steer_angle);
    car.speed = 10.0;
    car.update_steering(1.0);
    assert!((car.steer_angle - 0.09).abs() < 1e-6, "full lock at MODSTEERSPEED");
    car.speed = 22.5; // (22.5−10)/25 = 0.5 → scale 0.5
    car.update_steering(1.0);
    assert!((car.steer_angle - 0.045).abs() < 1e-6, "half falloff at 22.5 m/s, got {}", car.steer_angle);
    car.speed = 35.0; // scale would be 0 → floored at 0.425
    car.update_steering(1.0);
    assert!((car.steer_angle - 0.09 * 0.425).abs() < 1e-6, "the MINSTEER floor at ≥35 m/s, got {}", car.steer_angle);
    car.speed = 60.0;
    car.update_steering(0.5);
    assert!((car.steer_angle - 0.09 * 0.425 * 0.5).abs() < 1e-6, "half input halves the angle");
}

/// The drive/brake accelerations [M FUN_0018cef8:414-421]: engine = MAXACC·RATIO[gear] per unit
/// throttle; brake subtracts MAXBRAKE. Sister: the gear ratio is actually indexed — gear 2's
/// 0.59 differs from gear 1's 0.65.
#[test]
fn drive_accel_uses_the_gear_ratio_and_brake_subtracts() {
    let mut car = vanquish();
    car.vel = [10.0, 0.0, 0.0];
    car.update_rpm_gear(1.0, true);
    let input = CarInput { throttle: 1.0, brake: 0.0, steer: 0.0, handbrake: false };
    let a = car.drive_accel(&input);
    assert!((a - 3.0 * 0.65).abs() < 1e-6, "gear 1: MAXACC·RATIO1 = 1.95, got {a}");
    car.vel = [20.0, 0.0, 0.0];
    car.update_rpm_gear(1.0, true);
    let a2 = car.drive_accel(&input);
    assert!((a2 - 3.0 * 0.59).abs() < 1e-6, "gear 2: MAXACC·RATIO2 = 1.77, got {a2}");
    let brake = CarInput { throttle: 0.0, brake: 1.0, steer: 0.0, handbrake: false };
    assert!((car.drive_accel(&brake) + 3.15).abs() < 1e-6, "full brake is −MAXBRAKE [M :421]");
    let coast = CarInput::default();
    assert!((car.drive_accel(&coast)).abs() < 1e-6, "no pedals, no drive force");
}

/// The reverse speed cutoff [M FUN_0018cef8:401-407]: MAXREVACC decays by
/// (speed − SPEED_CUTOFF_REVERSE)·SPEED_CUTOFF_RATE_REVERSE below zero at the cutoff+1/0.075.
#[test]
fn reverse_accel_decays_past_the_cutoff() {
    let mut car = vanquish();
    car.reversing = true;
    car.speed = 5.0; // below the 10 m/s cutoff — full MAXREVACC
    let input = CarInput { throttle: 1.0, brake: 0.0, steer: 0.0, handbrake: false };
    assert!((car.drive_accel(&input) - 2.4).abs() < 1e-6);
    car.speed = 20.0; // cut = 10 → 2.4 − 10·0.075 = 1.65
    assert!((car.drive_accel(&input) - 1.65).abs() < 1e-5, "the decay line, got {}", car.drive_accel(&input));
    car.speed = 42.0; // cut = 32 → 2.4 − 2.4 < 0 → floored at 0
    assert_eq!(car.drive_accel(&input), 0.0, "the reverse cutoff floors at 0 [M :405-407]");
}

/// The gravity bodge [M FUN_001f7c60:136-144 + the strings 0x396578/0x396588]: cars integrate
/// under BODGE_PLAYER_GRAVITY (−13); the jungle_truck name alone takes GRAVITY (−34). Sister:
/// the two values differ — a port that hard-codes −34 fails the player-car fall.
#[test]
fn player_car_gravity_is_the_bodge_minus_13() {
    let r = Rigid::game_defaults();
    assert_eq!(r.bodge_player_gravity, -13.0);
    assert_eq!(r.gravity, -34.0, "and jungle_truck's full gravity [M :140]");
    let mut car = vanquish();
    car.grounded = false;
    car.pos = [0.0, 20.0, 0.0];
    let world = FlatWorld::paved();
    let input = CarInput::default();
    for _ in 0..60 {
        car.step(&input, &world, &r, DT);
    }
    assert!((car.vel[1] - (-13.0 * 1.0)).abs() < 1e-3, "one second of the bodge gravity, vy = {}", car.vel[1]);
    assert!((car.pos[1] - (20.0 - 13.0 * (60.0 * 61.0 / 2.0) * DT * DT)).abs() < 1e-2,
            "the semi-implicit Euler sum: y = 20 − g·dt·Σk = 13.392, got {}", car.pos[1]);
    assert!(!car.grounded, "still airborne at 13.5 m");
    for _ in 0..120 {
        car.step(&input, &world, &r, DT);
    }
    assert_eq!(car.pos[1], 0.0, "the flat surface catches the car");
    assert!(car.grounded);
    // The jungle_truck case: the name check swaps the working gravity [M :140].
    let mut truck = vanquish();
    truck.gravity = r.gravity;
    truck.grounded = false;
    truck.pos = [0.0, 20.0, 0.0];
    for _ in 0..60 {
        truck.step(&input, &world, &r, DT);
    }
    assert!((truck.vel[1] - (-34.0)).abs() < 1e-3, "one second of GRAVITY, vy = {}", truck.vel[1]);}

/// The tyre grip boost curve [M FUN_00190f88:395-397] and the ±12.5 lateral clamp [M :506-508]:
/// the port's yaw rate is limited so the lateral acceleration stays inside
/// 12.5·boost·surface. Sister: the boost reaches exactly 1.0 at 15 m/s.
#[test]
fn lateral_grip_boost_and_the_12_5_clamp() {
    let boost = |speed: f32| (1.0 - (speed / 15.0).min(1.0)) * 2.5 + 1.0;
    assert!((boost(0.0) - 3.5).abs() < 1e-6, "the rest boost [M :396-397]");
    assert!((boost(7.5) - 2.25).abs() < 1e-6, "half speed, half boost");
    assert!((boost(15.0) - 1.0).abs() < 1e-6, "the 15 m/s crossover [M]");
    assert!((boost(30.0) - 1.0).abs() < 1e-6, "and it never goes below 1");
    // The clamp: sweep speeds and steer inputs, the lateral acceleration never exceeds
    // 12.5·boost·friction [M :506-508 with the surface table at 1.0 [M .data 0x3319d8]].
    let r = Rigid::game_defaults();
    let world = FlatWorld::paved();
    for speed in [2.0f32, 8.0, 15.0, 25.0, 40.0, 60.0] {
        for steer in [-1.0f32, -0.5, 0.5, 1.0] {
            let mut car = vanquish();
            car.vel = [speed, 0.0, 0.0];
            car.update_rpm_gear(0.0, true);
            car.update_steering(steer);
            car.step(&CarInput { throttle: 0.0, brake: 0.0, steer, handbrake: false }, &world, &r, DT);
            let lat = (car.yaw_rate * speed).abs();
            let limit = 12.5 * boost(speed);
            assert!(lat <= limit + 1e-3, "speed {speed} steer {steer}: lateral {lat} > limit {limit}");
        }
    }
}

/// A scripted flat run through the real step path [M 0x159d10's once-per-tick physics]: full
/// throttle reaches and crosses every vanquish shift point in order; rolling resistance keeps
/// the coast-down gentle (the ROLLING_RESISTANCE 0.1 reading, doc §5's open question).
#[test]
fn flat_run_shifts_through_every_gear_in_order() {
    let mut car = vanquish();
    let r = Rigid::game_defaults();
    let world = FlatWorld::paved();
    let input = CarInput { throttle: 1.0, brake: 0.0, steer: 0.0, handbrake: false };
    let mut seen = [false; 6];
    let mut top = 0.0f32;
    for _ in 0..60 * 90 {
        car.step(&input, &world, &r, DT);
        seen[car.gear as usize] = true;
        top = top.max(car.speed);
        if car.speed > 60.0 { break; }
    }
    assert!(seen[1] && seen[2] && seen[3] && seen[4] && seen[5], "every gear visited: {seen:?}");
    assert!(car.speed >= 54.0, "the run reaches gear 5's territory, speed {}", car.speed);
    assert!(top > 54.0);
    // Coast: throttle off, the car decelerates but keeps rolling (no per-tick ×0.8 cliff).
    let coast = CarInput::default();
    let v0 = car.speed;
    for _ in 0..60 {
        car.step(&coast, &world, &r, DT);
    }
    assert!(car.speed < v0, "rolling resistance slows the car");
    assert!(car.speed > v0 * 0.8, "…gently (v0 {v0} → {}), not the 0.8-per-tick cliff", car.speed);
}

// --- The real-file tests: the mission VIVs' own tuning data (no game bytes in the repo) ---
// The slim ISO's /DRIVING records are zeroed [D; docs/atlas/driving-elf.md §2], so the VIVs come
// from the working set (default /home/z/driving-refpack/DRIVING, override NF_DRIVING_VIV dir);
// the tests skip when it is absent.

fn driving_viv(name: &str) -> Option<std::path::PathBuf> {
    let base = std::env::var_os("NF_DRIVING_VIV").map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/home/z/driving-refpack/DRIVING"));
    let p = base.join(name);
    p.exists().then_some(p)
}

/// Extracts one entry from a mission VIV through the repo's own BIGF + RefPack readers.
fn viv_entry(viv: &std::path::Path, name: &str) -> Option<Vec<u8>> {
    let data = std::fs::read(viv).ok()?;
    let bigf = nf_formats::bigf::Bigf::parse(&data, data.len() as u64).ok()?;
    let e = bigf.find(name)?;
    let raw = data.get(e.offset as usize..(e.offset + e.size) as usize)?.to_vec();
    if nf_formats::refpack::is_refpack(&raw) { nf_formats::refpack::decompress(&raw).ok() } else { Some(raw) }
}

/// The shipped `data\sim\attrib\pvehicle\vanquish.atr` parses to exactly the values the port's
/// `vanquish()` hard-codes [D]. Sister: any drifted constant fails the element-wise compare.
#[test]
fn the_real_vanquish_atr_matches_the_ported_constants() {
    let Some(viv) = driving_viv("MIS01.VIV") else { eprintln!("skip: no mission VIVs"); return };
    let Some(bytes) = viv_entry(&viv, r"data\sim\attrib\pvehicle\vanquish.atr") else { eprintln!("skip: no vanquish.atr in MIS01.VIV"); return };
    let text = String::from_utf8_lossy(&bytes);
    let p = CarParams::parse_atr(&text);
    let v = CarParams::vanquish();
    assert_eq!(p.mass, v.mass, "MASS [D]");
    assert_eq!(p.maxacc, v.maxacc, "MAXACC [D]");
    assert_eq!(p.maxrevacc, v.maxrevacc, "MAXREVACC [D]");
    assert_eq!(p.maxbrake, v.maxbrake, "MAXBRAKE [D]");
    assert_eq!(p.maxsteering, v.maxsteering, "MAXSTEERING [D]");
    assert_eq!(p.modsteer_speed, v.modsteer_speed, "MODSTEERSPEED [D]");
    assert_eq!(p.min_steer, v.min_steer, "MINSTEER [D]");
    assert_eq!(p.gear_limit, v.gear_limit, "GEAR_LIMIT R,1..5 [D]");
    assert_eq!(p.gear_ratio, v.gear_ratio, "GEAR_RATIO R,1..5 [D]");
    assert_eq!(p.friction_limit_front, v.friction_limit_front, "FRICTIONLIMITFRONT [D]");
    assert_eq!(p.wheel_radius, v.wheel_radius, "WHEEL_RADIUS [D]");
    assert_eq!(p.handbrake_force, v.handbrake_force, "HANDBRAKE_FORCE [D]");
}

/// The shipped `Physics\Rigid\default.tun` overrides nothing — its values equal the ELF's
/// compiled-in initials [M .data 0x3390d0..; D the file]. Sister: a changed constant fails.
#[test]
fn the_real_rigid_tun_matches_the_compiled_in_defaults() {
    let Some(viv) = driving_viv("MIS01.VIV") else { eprintln!("skip: no mission VIVs"); return };
    let Some(bytes) = viv_entry(&viv, r"data\tuning\Physics\Rigid\default.tun") else { eprintln!("skip: no Rigid tun"); return };
    let text = String::from_utf8_lossy(&bytes);
    let t = Tuning::parse(&text);
    let r = Rigid::from_tun(&t);
    let d = Rigid::game_defaults();
    assert_eq!(r.gravity, d.gravity, "GRAVITY [D file == M .data]");
    assert_eq!(r.bodge_player_gravity, d.bodge_player_gravity, "BODGE_PLAYER_GRAVITY");
    assert_eq!(r.sleep_vel, d.sleep_vel, "SLEEP_VEL");
    assert_eq!(r.ground_friction_coeff, d.ground_friction_coeff, "GROUND_FRICTION_COEFF");
    assert_eq!(r.natural_angular_damping, d.natural_angular_damping, "NATURAL_ANGULAR_DAMPING");
    assert_eq!(r.m_limit, d.m_limit, "M_LIMIT");
    assert_eq!(r.am_limit, d.am_limit, "AM_LIMIT");
    assert_eq!(r.total_force_limit, d.total_force_limit, "TOTAL_FORCE_LIMIT");
}
