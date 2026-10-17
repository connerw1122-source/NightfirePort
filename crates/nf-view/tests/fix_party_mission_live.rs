//! fix-party-mission repro/verification tests (07000007 party + 07000008 gondola), driven through the
//! REAL path (`nf_view::load` + `Sim::step_bond`).
//!
//! Playtest 2026-10-12 items this file pins:
//! 1. the start guards shoot Bond (they must not until he is armed/sees a weapon);
//! 2. punching a guard fails the mission at once; shooting a guest does not;
//! 3. punching a guest T-poses the other guests;
//! 4. the mission music keeps playing over the cut-scenes;
//! 5. objective banners show "NEW OBJECTIVE" without the objective text and run during cut-scenes;
//! 6. the guards are not in their proper places after the Kimiko cut-scene.

use nf_game::drone::state;
use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str, level: &str) -> nf_view::scene::Scene {
    nf_view::load(iso, level, false).expect("load")
}

fn tick_idle(sim: &mut nf_view::scene::Sim, n: u32) {
    for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
}

/// Probe: what do the drones at the party start look like (states, teams, health)?
/// faster-live-tests: assertion-free evidence probe (print-only) — the behaviors it surveyed are
/// pinned by `the_party_entrance_guards…` below; `--ignored` keeps it for manual runs.
#[test]
#[ignore = "assertion-free probe (prints drone states/teams/health); the pinned behavior lives in the_party_entrance_guards_never_shoot_unarmed_bond_at_the_start — run with --ignored"]
fn probe_party_start_guards() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000007.bin");
    let sim = scene.sim.as_mut().unwrap();
    // Trace alert growth for the two entrance guards (skins 0x5000013/14 near the spawn).
    for t in 0..3200u32 {
        sim.step_bond(Input::default(), false, false, false);
        if t % 100 == 0 {
            let g: Vec<(usize, f32, u8)> = sim.drones.list.iter().enumerate()
                .filter(|(_, d)| d.present && d.team == 1)
                .filter(|(_, d)| ((d.pos[0] - 41.0).powi(2) + (d.pos[2] + 71.0).powi(2)).sqrt() < 6.0)
                .map(|(i, d)| (i, d.alert, d.state)).collect();
            eprintln!("[party] t={t} disabled={} entrance-guards {:?}", sim.disabled, g);
        }
        if !sim.disabled && t > 100 { break; }
    }
    for _ in 0..3000 { sim.step_bond(Input::default(), false, false, false); if !sim.disabled { break; } }
    tick_idle(sim, 200);
    for (i, d) in sim.drones.list.iter().enumerate() {
        if !d.present { continue; }
        let dist = ((d.pos[0] - sim.bond.pos[0]).powi(2) + (d.pos[2] - sim.bond.pos[2]).powi(2)).sqrt();
        eprintln!("[party] drone {i}: skin {:#x} state {:#04x} team {} alert {:.2} hp {:.0} kind {} dtype {} mode {} mn {} ma {} alt_st {:#x} 2nd_ch {} started {} pos {:?} dist {:.1}",
            d.skin, d.state, d.team, d.alert, d.health, d.kind, d.dtype, d.mode, d.mode_normal, d.mode_alert, d.alt_state, d.second_channel, d.started, d.pos, dist);
    }
    eprintln!("[party] bond at {:?} weapon {}", sim.bond.pos, sim.gun.weapon);
    eprintln!("[party] health {:.1} ch0x47 {}", sim.drones.health.health, sim.drones.channels.on(0x47));
}

/// ── Item 1: the entrance bouncers must not shoot unarmed Bond at the party start ────────────────
/// Playtest 2026-10-12: "At start of this mission, enemies, the guards, are shooting at me. They
/// shouldn't be shooting at me at all. It's only the ones in the beginning."
///
/// The game's gate: CivilianGuard UPDATE (0x1603a8 case 3) attacks only when
/// `NDrone2_NearArmedPerson` 0x1787c8 (prop 0x3c + Bond in sight `+0x228 & 4` +
/// `DroneWeap_OpponentHasWeapon` 0x17a4b8 → `ObjectIsArmed` 0x178768 on Bond's class byte) or
/// `ObjectIsArmed(Bond) && (+0x228 & 4)` — Bond at the party start holds the FISTS: InitWeapon
/// 0x1a7ef8 case 0x7000007 writes +0x63 = 1 and WeaponSelect 0x1a2cc0 copies the request into the
/// class byte (+0x62) at the switch's start, so `ObjectIsArmed` (classes 2..0x47 / 0x6a..0x71) is
/// false for the whole pre-hall phase. The bouncers BondTalk instead.
#[test]
fn the_party_entrance_guards_never_shoot_unarmed_bond_at_the_start() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000007.bin");
    let sim = scene.sim.as_mut().unwrap();
    // The fists switch request lands within ~100 ticks (walk_07000007); tick well past it.
    for _ in 0..600 { sim.step_bond(Input::default(), false, false, false); }
    assert_eq!(sim.gun.weapon, 1, "Bond holds the fists at the party start [M 0x1a7ef8 case 0x7000007]");
    // Sister: nobody else drew either — the party is calm.
    let combat: Vec<usize> = sim.drones.list.iter().enumerate()
        .filter(|(_, d)| d.present && d.started && !d.corpse)
        .filter(|(_, d)| matches!(d.state, 0x56..=0x7e | 0x85..=0x99))
        .map(|(i, _)| i).collect();
    assert!(combat.is_empty(), "no drone is in combat at the party start, got {combat:?} states {:?}",
        combat.iter().map(|&i| sim.drones.list[i].state).collect::<Vec<_>>());
    assert_eq!(sim.drones.health.health, 100.0,
        "Bond takes no damage from the party guards [playtest: they shot him]");
}

/// Probe: bouncer props (0x3c, 0x1f, 0x33) + sight variables.
/// faster-live-tests: assertion-free evidence probe (print-only) — the bouncer behaviors are
/// pinned by `punching_a_bouncer…` below; `--ignored` keeps it for manual runs.
#[test]
#[ignore = "assertion-free probe (prints bouncer props/sight vars); the pinned behavior lives in punching_a_bouncer_fails_the_mission_at_once — run with --ignored"]
fn probe_bouncer_props() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000007.bin");
    let sim = scene.sim.as_mut().unwrap();
    tick_idle(sim, 150);
    let data = sim.data();
    for (i, dr) in sim.drones.list.iter().enumerate() {
        if dr.present && dr.dtype == 10 {
            eprintln!("[party] guest {i}: kind {} state {:#x} team {} p53 {} p54 {} p52 {} p1f {}",
                dr.kind, dr.state, dr.team, dr.prop(&data, 0x53) as u8, dr.prop(&data, 0x54) as u8,
                dr.prop(&data, 0x52) as u8, dr.prop(&data, 0x1f) as u8);
        }
        if dr.present && dr.team == 1 && dr.mode == 0x13 {
            eprintln!("[party] bouncer {i}: kind {} p3c {} p1f {} p33 {} p51 {} seen {} los {} bond_armed {}",
                dr.kind, dr.prop(&data, 0x3c) as u8, dr.prop(&data, 0x1f) as u8,
                dr.prop(&data, 0x33) as u8, dr.prop(&data, 0x51) as u8,
                dr.seen as u8, dr.los as u8, dr.bond_armed as u8);
            eprintln!("[party] bouncer {i} props: p53 {} p54 {} p52 {} p10 {}",
                dr.prop(&data, 0x53) as u8, dr.prop(&data, 0x54) as u8,
                dr.prop(&data, 0x52) as u8, dr.prop(&data, 0x10) as u8);
        }
    }
}

/// ── Item 2: punching a guard fails at once; shooting a guest fails ──────────────────────────────
/// Playtest: "If you punch a guard, it should be an automatic mission fail" and the guests07 check
/// ("shooting a guest doesn't fail it — It doesn't work").
///
/// The game's writers:
/// - `NDrone2_PunchImpact` 0x145f28:45-56: on 07000007 a punch on an UNARMED drone (ObjectIsArmed
///   0x178768 on the drone's class byte — the party bouncers are unarmed [M DoModeSettingsOLD
///   0x14cfa0:159-163 zeroes it for modes 9/10/0x13]) → SetMissionFailReason(5) → "Cover Lost" ch 99
///   [M 0x13e560].
/// - `DroneFunc_OnInitDeath` 0x145bd0: a killed prop-0x54 guest (team 3) → reason 3 → "Killed
///   Civilians" ch 0x60 [M; fix-actors ported the death path] — through the REAL bullet path here.
#[test]
fn shooting_a_guest_through_the_bullet_path_fails_the_mission() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000007.bin");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..600 { sim.step_bond(Input::default(), false, false, false); }
    let guest = sim.drones.list.iter().position(|d| d.present && d.dtype == 10 && !d.corpse)
        .expect("a party guest exists");
    // Bond stands 4 units in front of the guest (in its view cone so nothing else interferes) and
    // fires the P2K (weapon 2) through the real fire path until the guest dies.
    let (gp, gy) = { let g = &sim.drones.list[guest]; (g.pos, g.yaw) };
    let from = [gp[0] + gy.sin() * 1.3, gp[1] + 0.6, gp[2] + gy.cos() * 1.3];
    sim.bond.pos = from;
    let snd = nf_formats::sfx::SfxHearing { id: 0, inner: 1.0, outer: 1.0, weight: 0.0, looped: false, cull: false };
    let mut died = false;
    for _ in 0..40 {
        {
            let dir = [gp[0] - from[0], (gp[1] + 0.45) - from[1], gp[2] - from[2]];
            let l = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
            let dir = [dir[0] / l, dir[1] / l, dir[2] / l];
            // A Data built from the sim's pieces (the `data()` accessor borrows all of `sim`).
            let d = nf_game::drone::Data { tables: &sim.tables, props: &sim.props, scripts: &sim.scripts, weapons: &sim.weapons, world: &sim.world, level: sim.level, sequences: &sim.seqs, skins: &sim.skins, skeletons: &sim.skeletons, nav: &sim.nav, covers: &sim.covers, tuning: &sim.tuning, hitboxes: &sim.hitboxes, volumes: &sim.volumes, sfx: None, models: &sim.models, surfaces: None, voices: None, difficulty: 1, ai_points: &sim.ai_points };
            sim.drones.fire_player(&d, from, dir, 2, snd);
        }
        for _ in 0..12 { sim.step_bond(Input::default(), false, false, false); }
        if sim.drones.list[guest].corpse { died = true; break; }
        if sim.drones.mission.state == 2 { break; } // the mission already failed (the guest died)
    }
    let dead: Vec<usize> = sim.drones.list.iter().enumerate().filter(|(_, d)| d.corpse).map(|(i, _)| i).collect();
    let ch60 = sim.drones.channels.on(0x60);
    let hps: Vec<(usize, f32, u8)> = sim.drones.list.iter().enumerate()
        .filter(|(_, d)| d.present && d.dtype == 10).map(|(i, d)| (i, d.health, d.state)).collect();
    eprintln!("[party] after the burst: guest{guest} hp {:.1} corpses {dead:?} mission {} ch60 {ch60} guest-hps {hps:?}",
        sim.drones.list[guest].health, sim.drones.mission.state);
    assert!(!dead.is_empty() || sim.drones.mission.state == 2,
        "shooting the guests kills one (hp table {hps:?})");
    assert!(sim.drones.channels.on(0x60), "the guest's death raises ch 0x60 [M 0x145bd0 reason 3]");
    let mut failed = false;
    for _ in 0..120 { sim.step_bond(Input::default(), false, false, false);
        if sim.drones.mission.state == 2 { failed = true; break; } }
    assert!(failed, "shooting a guest fails the mission [playtest: It doesn't work]");
}

/// Probe: guest hit boxes + bullet flight.
/// faster-live-tests: assertion-free evidence probe (print-only) — the guest behaviors are
/// pinned by `shooting_a_guest…` below; `--ignored` keeps it for manual runs.
#[test]
#[ignore = "assertion-free probe (prints guest hitboxes/bullet flight); pinned behavior lives in shooting_a_guest_through_the_bullet_path_fails_the_mission — run with --ignored"]
fn probe_guest_hitboxes() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000007.bin");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..600 { sim.step_bond(Input::default(), false, false, false); }
    let guest = sim.drones.list.iter().position(|d| d.present && d.dtype == 10 && !d.corpse).unwrap();
    let (gp, gy) = { let g = &sim.drones.list[guest]; (g.pos, g.yaw) };
    let from = [gp[0] + gy.sin() * 1.3, gp[1] + 0.6, gp[2] + gy.cos() * 1.3];
    sim.bond.pos = from;
    eprintln!("[party] guest {guest} pos {gp:?} yaw {gy:.2} state {:#x} enabled {} npc_enabled {} drawn {}",
        sim.drones.list[guest].state, sim.drones.list[guest].enabled, sim.drones.npc_enabled, sim.drones.list[guest].drawn);
    {
        let d = sim.data();
        let dr = &sim.drones.list[guest];
        let boxes = dr.hit_boxes(&d);
        eprintln!("[party] guest hit_boxes: {} boxes", boxes.len());
        let pal = dr.palette(&d);
        eprintln!("[party] guest palette: {}", pal.is_some());
    }
    let dir = [gp[0] - from[0], (gp[1] + 0.45) - from[1], gp[2] - from[2]];
    let l = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
    let dir = [dir[0] / l, dir[1] / l, dir[2] / l];
    {
        let end = [from[0] + dir[0] * 3.0, from[1] + dir[1] * 3.0, from[2] + dir[2] * 3.0];
        eprintln!("[party] wall t = {:?}; guest dist to ray midpoint {:.2}", sim.world.segment(from, end), {
            let m = [from[0] + dir[0] * 1.3, from[1] + dir[1] * 1.3, from[2] + dir[2] * 1.3];
            ((m[0] - gp[0]).powi(2) + (m[1] - gp[1]).powi(2) + (m[2] - gp[2]).powi(2)).sqrt()
        });
        let d = sim.data();
        let dr = &sim.drones.list[guest];
        let ray = [dir[0] * 3.0, dir[1] * 3.0, dir[2] * 3.0];
        eprintln!("[party] box hit: {:?}", nf_game::damage::first_box_hit(from, ray, &dr.hit_boxes(&d)).map(|(p, t)| (p, t)));
        for (k, b) in dr.hit_boxes(&d).iter().enumerate().take(4) {
            eprintln!("[party] box{k}: centre ({:.1},{:.1},{:.1}) half ({:.2},{:.2},{:.2}) part {}",
                b.m[3][0], b.m[3][1], b.m[3][2], b.half[0], b.half[1], b.half[2], b.part);
        }
        eprintln!("[party] guest anim: script {:?} danim {} class {} len {} time {} state {}",
            dr.anim.script.map(|s| s), dr.anim.danim, dr.anim.class, dr.anim.len_cap.map(|l| l).unwrap_or(0), dr.anim.time, dr.anim.state);
        if let Some(g2) = sim.drones.list.iter().position(|x| x.present && x.team == 1 && x.mode == 0x13) {
            let dr2 = &sim.drones.list[g2];
            eprintln!("[party] bouncer{g2} pos {:?} anim: script {:?} danim {} class {}", dr2.pos, dr2.anim.script, dr2.anim.danim, dr2.anim.class);
            for (k, b) in dr2.hit_boxes(&d).iter().enumerate().take(3) {
                eprintln!("[party] bbox{k}: centre ({:.1},{:.1},{:.1}) half ({:.2},{:.2},{:.2}) part {}",
                    b.m[3][0], b.m[3][1], b.m[3][2], b.half[0], b.half[1], b.half[2], b.part);
            }
        }
    }
    let snd = nf_formats::sfx::SfxHearing { id: 0x403, inner: 0.0, outer: 0.0, weight: 0.0, looped: false, cull: false };
    {
        let d = nf_game::drone::Data { tables: &sim.tables, props: &sim.props, scripts: &sim.scripts, weapons: &sim.weapons, world: &sim.world, level: sim.level, sequences: &sim.seqs, skins: &sim.skins, skeletons: &sim.skeletons, nav: &sim.nav, covers: &sim.covers, tuning: &sim.tuning, hitboxes: &sim.hitboxes, volumes: &sim.volumes, sfx: None, models: &sim.models, surfaces: None, voices: None, difficulty: 1, ai_points: &sim.ai_points };
        sim.drones.fire_player(&d, from, dir, 2, snd);
    }
    for t in 0..6 { sim.step_bond(Input::default(), false, false, false);
        let imps: Vec<String> = sim.drones.player_impacts.iter().map(|i| match i {
            nf_game::drone::PlayerImpact::Drone { drone, part, .. } => format!("drone{drone}/part{part}"),
            nf_game::drone::PlayerImpact::World(_) => "world".into(),
        }).collect();
        eprintln!("[party] tick {t}: hp {:.1} state {:#x} log {:?} impacts {:?} hit {}", sim.drones.list[guest].health, sim.drones.list[guest].state,
            &sim.drones.list[guest].log.iter().rev().take(3).collect::<Vec<_>>(), imps, sim.drones.list[guest].hit.is_some()); }
}

/// The punch half of item 2: punching a party bouncer (unarmed — DoModeSettingsOLD 0x14cfa0:159-163
/// zeroes his class byte for mode 0x13) fails the mission at once [M 0x145f28:45-56 → reason 5 →
/// "Cover Lost" ch 99]. Driven through the real path: `fire_player` with the fists (weapon 1) is the
/// game's punch dispatch (`NDrone2_DealWithObjHit` case 5 → msg 6).
#[test]
fn punching_a_bouncer_fails_the_mission_at_once() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000007.bin");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..600 { sim.step_bond(Input::default(), false, false, false); }
    let bouncer = sim.drones.list.iter().position(|d| d.present && d.team == 1 && d.mode == 0x13 && !d.corpse)
        .expect("a bouncer exists");
    let (bp, by) = { let b = &sim.drones.list[bouncer]; (b.pos, b.yaw) };
    let from = [bp[0] + by.sin() * 1.3, bp[1] + 0.6, bp[2] + by.cos() * 1.3];
    sim.bond.pos = from;
    let dir = [bp[0] - from[0], (bp[1] + 0.45) - from[1], bp[2] - from[2]];
    let l = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
    let dir = [dir[0] / l, dir[1] / l, dir[2] / l];
    let snd = nf_formats::sfx::SfxHearing { id: 0x403, inner: 0.0, outer: 0.0, weight: 0.0, looped: false, cull: false };
    {
        let d = nf_game::drone::Data { tables: &sim.tables, props: &sim.props, scripts: &sim.scripts, weapons: &sim.weapons, world: &sim.world, level: sim.level, sequences: &sim.seqs, skins: &sim.skins, skeletons: &sim.skeletons, nav: &sim.nav, covers: &sim.covers, tuning: &sim.tuning, hitboxes: &sim.hitboxes, volumes: &sim.volumes, sfx: None, models: &sim.models, surfaces: None, voices: None, difficulty: 1, ai_points: &sim.ai_points };
        sim.drones.fire_player(&d, from, dir, 1, snd); // the fists = the punch
    }
    for _ in 0..12 { sim.step_bond(Input::default(), false, false, false); }
    assert!(sim.drones.channels.on(99), "the punch raises ch 99 [M 0x145f28 reason 5 → 0x13e560]");
    assert_eq!(sim.drones.mission.fail_label, 0x0400_0033, "FailLabel = Cover Lost [M 0x13e560]");
    let mut failed = false;
    for _ in 0..120 { sim.step_bond(Input::default(), false, false, false);
        if sim.drones.mission.state == 2 { failed = true; break; } }
    assert!(failed, "punching a guard fails the mission [playtest: automatic fail]");
}

/// ── Item 3: punching a guest must not T-pose the other guests ───────────────────────────────────
/// Playtest: "Punching a guest makes all the other guests T-pose." A T-pose = no animation playing
/// (`anim.script == None`, the bind pose). Reproduced through the real punch path (fire_player with
/// the fists on a guest → msg 6 → `NDrone2_PunchImpact`).
#[test]
fn punching_a_guest_keeps_the_other_guests_animated() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000007.bin");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..600 { sim.step_bond(Input::default(), false, false, false); }
    let guest = sim.drones.list.iter().position(|d| d.present && d.dtype == 10 && !d.corpse)
        .expect("a party guest exists");
    let (gp, gy) = { let g = &sim.drones.list[guest]; (g.pos, g.yaw) };
    let from = [gp[0] + gy.sin() * 1.3, gp[1] + 0.6, gp[2] + gy.cos() * 1.3];
    sim.bond.pos = from;
    // Bond holds the fists and punches through the REAL input path until the guest drops (the playtest's
    // punch KOs the guest), then watches the others' animations.
    sim.gun.weapon = 1; sim.gun.switch_request = 1; sim.gun.switch_state = 0;
    let mut ko = false;
    for _ in 0..200 {
        sim.step_bond(Input::default(), true, false, false); // trigger held = a punch swing
        sim.step_bond(Input::default(), false, false, false);
        let g = &sim.drones.list[guest];
        if g.knocked_out || g.corpse || g.state == 0x42 { ko = true; break; }
    }
    for _ in 0..120 { sim.step_bond(Input::default(), false, false, false); }
    let states: Vec<(usize, u8, Option<u32>, u16)> = sim.drones.list.iter().enumerate()
        .filter(|(_, d)| d.present && d.dtype == 10 && !d.corpse)
        .map(|(i, d)| (i, d.state, d.anim.script, d.anim.danim)).collect();
    eprintln!("[party] after the punch-ko ({ko}): guest states {states:?}");
    let tposed: Vec<usize> = states.iter().filter(|(i, _, s, dn)| *i != guest && (s.is_none() || *dn == 0)).map(|(i, _, _, _)| *i).collect();
    assert!(tposed.is_empty(), "punching a guest T-poses the others (no anim on {tposed:?}) [playtest item 3]");
}

/// ── Item 4: the music changes at NIS start and end ──────────────────────────────────────────────
/// Playtest: "The mission music keeps playing over the cut-scenes", and on 07000008 "the cutscene
/// music still hasn't finished while playing".
///
/// The game's per-level music updaters jump sections on the NIS events: 07000008's
/// `UpdateMFX_01CastleD` 0x1133b8 — the intro NIS 0x06000077's Music_Event(0xB, script) at start
/// (event 11, `MusicLastJump == 0`) → `SFXJumpToMusicMarker(2, instant)`; Music_Event(0xC, script)
/// at end (event 12) → marker 4 instantly. 07000007's `UpdateMFX_01CastleC` 0x113188 — the Drake
/// reveal 0x6000072: start → 2, end (or the 0x172 NIS music sound [M 0x115a68]) → 5; the hall
/// scene 0x6000073: start → 7, end → 0xa. Ported as `Music::castle_c`/`castle_d`; the test drives
/// the REAL path (the level's intro NIS through `Sim::step_bond`).
#[test]
fn the_cutscene_music_jumps_at_nis_start_and_end() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000008.bin");
    let sim = scene.sim.as_mut().unwrap();
    // The intro NIS (SP #294, 0x06000077) fires from the intro plate #237 (139.2, 49.5, −81.2) [D].
    for _ in 0..600 { sim.step_bond(Input::default(), false, false, false); }
    sim.bond.pos = [139.2, 49.5, -81.2];
    let mut jumps: Vec<i32> = Vec::new();
    for _ in 0..4200 {
        sim.step_bond(Input::default(), false, false, false);
        for c in sim.drones.music.out.drain(..) {
            if let nf_game::music::Cmd::Jump { section, .. } = c { jumps.push(section); }
        }
        if sim.disabled { break; } // the script took control — the NIS is running
    }
    // Let the NIS play out and watch its end jump too.
    for _ in 0..4200 {
        sim.step_bond(Input::default(), false, false, false);
        for c in sim.drones.music.out.drain(..) {
            if let nf_game::music::Cmd::Jump { section, .. } = c { jumps.push(section); }
        }
        if !sim.disabled { break; }
    }
    for _ in 0..10 { sim.step_bond(Input::default(), false, false, false);
        for c in sim.drones.music.out.drain(..) {
            if let nf_game::music::Cmd::Jump { section, .. } = c { jumps.push(section); }
        } }
    eprintln!("[party] 07000008 music jumps around the intro: {jumps:?} (nis start → 2, end → 4 [M 0x1133b8])");
    assert!(jumps.contains(&2), "the intro NIS's start jumps the music to section 2 [M 0x1133b8:8-12]");
    assert!(jumps.contains(&4), "the intro NIS's end jumps the music to section 4 [M 0x1133b8:13-16]");
}

/// Probe: what the objective banners bind to on 07/01/02/08, and WHEN they land (NIS running?).
/// faster-live-tests: assertion-free evidence probe (print-only — the L362 comment's "pins the
/// bound text" was wrong; the text pins live in `the_07000001_banner_survives_the_intro_cutscene`
/// and `no_status_pane…` below). `--ignored` keeps it for manual runs.
#[test]
#[ignore = "assertion-free probe (prints banner landings on 07/01/02/08); the pinned behavior lives in the_07000001_banner_survives_the_intro_cutscene — run with --ignored"]
fn probe_objective_banners() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for level in ["07000007.bin", "07000001.bin", "07000002.bin", "07000008.bin"] {
        let mut scene = load(&iso, level);
        let sim = scene.sim.as_mut().unwrap();
        // The announcements live 300 ticks [M 0x1827d8 case 0] — read them as they land.
        let mut seen: Vec<(u32, bool, String, f32)> = Vec::new();
        for t in 0..5200 {
            sim.step_bond(Input::default(), false, false, false);
            for m in &sim.hud.objective_msgs {
                if !seen.iter().any(|(_, _, s, _)| *s == m.text) { seen.push((t, sim.disabled, m.text.clone(), m.time)); }
            }
            if !sim.disabled && t > 500 && seen.len() >= 3 { break; }
        }
        let nis: Vec<u32> = seen.iter().filter(|(_, d, _, _)| *d).map(|(t, _, _, _)| *t).collect();
        eprintln!("[banners] {level}: landed {nis:?} (true = during the NIS); all: {seen:?}");
    }
}

/// ── Item 5: the banners wait for the cut-scene, and carry the objective text ────────────────────
/// Playtest: "It just says new objective" (the user's build predates fix-mission's text binding —
/// `Mission_MonitorObjectives` 0x1827d8 posts the objective record's label [0x04000000/0x02000000
/// style] and the port binds it through the USATxt.dat table; the bound text is pinned by
/// `the_07000001_banner_survives_the_intro_cutscene` and `no_status_pane…` below), "they show during cut-scenes — It needs to wait until after the
/// cutscene's finished", and on 07000001 "no banner came up": its only banner ("Escort Mayhew to
/// bunker") is announced by the state-0 arm [M 0x1827d8:41-54] at mission start — while the intro
/// NIS holds control from tick 0 to ~2120 — and died at tick 300 in the pre-fix build.
///
/// The game's gate: `Script_CameraStart` 0x1d9140:41-43 disables every viewer's HUD
/// (`Camera_Enable(i,0,0,0)` → viewer+0x2a = 0 [M 0x1b9920]); while +0x2a is 0 `Text_UpdateMsg`
/// 0x1b11d8:16-18 returns without ticking any message timer, and `HUD_UpdateStatusPane` 0x19c310
/// case 0 never opens the pane — a banner queued mid-NIS waits closed, then opens (and its SFX
/// play, case 2) when `Script_KillStream` 0x1d9860:71 restores the states.
#[test]
fn the_07000001_banner_survives_the_intro_cutscene() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000001.bin");
    // The intro NIS holds control from tick 0; its objective banner must still be queued — and
    // drawing — when control returns.
    let mut nis_end = 0;
    for t in 0..6000u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        if !scene.sim.as_ref().unwrap().disabled && t > 100 { nis_end = t; break; }
    }
    assert!(nis_end > 1000, "the intro NIS holds control well past the banner's 300-tick life (ended at {nis_end})");
    for _ in 0..10 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    {
        let sim = scene.sim.as_ref().unwrap();
        let msgs: Vec<&str> = sim.hud.objective_msgs.iter().map(|m| m.text.as_str()).collect();
        eprintln!("[party] 07000001 after the intro NIS (t={nis_end}): {msgs:?}");
        assert!(msgs.iter().any(|t| *t == "Escort Mayhew to bunker"),
            "the mission-start banner survives the intro cut-scene (playtest: 'no banner came up')");
        assert!(!sim.hud.objective_msgs.is_empty() && sim.hud.objective_msgs.front().unwrap().time > 250.0,
            "the banner's timer did not run while the NIS played [M 0x1b11d8:16-18]");
    }
    let f = nf_view::hud::overlay(&scene, 4.0 / 3.0);
    let bars = banner_bars(&scene, &f);
    eprintln!("[party] 07000001 banner bar quads after the NIS: {bars}");
    assert!(bars >= 3, "the banner draws once control returns [M 0x19c310 case 0 → case 1]");
}

/// The pane gate itself, on 07000002 (its intro NIS runs from tick 0 and four banners land around
/// it): through the whole cut-scene no status pane draws; when control returns the queued banners
/// do.
#[test]
fn no_status_pane_draws_during_the_07000002_intro_cutscene() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000002.bin");
    let mut worst = 0;
    for t in 0..4000u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        // The NIS detector is the game's own gate (viewer+0x2a): on the flush tick it is already
        // back on while the port's `disabled` clears a tick later — on that tick the banner
        // legitimately opens [M Script_KillStream 0x1d9860:71 Camera_PopStates → case 0 opens].
        let sim = scene.sim.as_ref().unwrap();
        if sim.disabled && sim.drones.hud_frozen {
            let f = nf_view::hud::overlay(&scene, 4.0 / 3.0);
            worst = worst.max(banner_bars(&scene, &f));
        }
        if !sim.disabled && t > 100 { break; }
    }
    for _ in 0..10 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    eprintln!("[party] 07000002 worst mid-NIS banner bar quad count: {worst}");
    assert_eq!(worst, 0, "no banner draws while the cut-scene runs [M 0x19c310 case 0 + 0x1b11d8:16-18]");
    assert!(!scene.sim.as_ref().unwrap().hud.objective_msgs.is_empty(), "the banners queued during the NIS are still pending");
    let f = nf_view::hud::overlay(&scene, 4.0 / 3.0);
    assert!(banner_bars(&scene, &f) >= 3, "the first banner draws once the cut-scene ends");
}

/// Every quad (6 vertices) drawn from the status-pane sheet 0x0300007A's BAR columns (the pickup
/// bar, the objective banner and the mission banner all stamp u 185..199 of it [M 0x19c310; L
/// ObjectiveStatusSprInfo/MissionStatusSprInfo]; other panes' templates share the sheet — e.g. the
/// radar's sprite at u 5 [L ee_start_05 SpriteInfo] — and must not count).
fn banner_bars(scene: &nf_view::scene::Scene, f: &nf_view::gpu::HudFrame) -> usize {
    let Some(ti) = scene.hud_textures.iter().position(|(h, _)| *h == 0x0300_007a) else { return 0 };
    let (tw, th) = (scene.hud_textures[ti].1.width as f32, scene.hud_textures[ti].1.height as f32);
    let mut n = 0;
    for r in f.runs.iter().filter(|r| r.0 == ti + 1) {
        for q in f.v[r.2 as usize..(r.2 + r.3) as usize].chunks(6) {
            let u0 = q[0].uv[0] * tw;
            let v0 = q[0].uv[1] * th;
            // The bars stamp v 0..56/103 from the sheet's top edge; the weapon pane's template
            // sprites further along u have v0 ≥ 6.
            if (184.0..=199.0).contains(&u0) && v0 < 2.0 { n += 1; }
        }
    }
    n
}

/// Probe (item 6): drive 07000007 into the Kimiko hand-back scene (trigger 412 → SP #420,
/// 0x06000073) through the real path; snapshot every drone before and after.
/// faster-live-tests: assertion-free evidence probe (print-only) — the same scenario's outcomes
/// are pinned by `the_guards_reappear…` below; `--ignored` keeps it for manual runs.
#[test]
#[ignore = "assertion-free probe (prints drone snapshots around the Kimiko scene); the pinned behavior lives in the_guards_reappear_at_their_places_after_the_kimiko_scene — run with --ignored"]
fn probe_kimiko_scene_guards() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000007.bin");
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [94.3, 44.6, -45.9];
        for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    }
    let mut started = None;
    for t in 0..600u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        if scene.sim.as_ref().unwrap().disabled { started = Some(t); break; }
    }
    let started = started.expect("the hand-back scene must take control");
    type Snap = Vec<(usize, u32, u8, u8, [f32; 3], bool, bool)>;
    let snap = |scene: &nf_view::scene::Scene| -> Snap {
        scene.sim.as_ref().unwrap().drones.list.iter().enumerate()
            .filter(|(_, d)| d.present)
            .map(|(i, d)| (i, d.skin, d.state, d.team, d.pos, d.enabled, d.corpse)).collect()
    };
    let pre = snap(&scene);
    eprintln!("[kimiko] scene started at t={started}; {} drones present", pre.len());
    for t in 0..6000u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        if !scene.sim.as_ref().unwrap().disabled { eprintln!("[kimiko] control back at +{t}"); break; }
    }
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let post = snap(&scene);
    let bond = &scene.sim.as_ref().unwrap().bond;
    eprintln!("[kimiko] bond at {:?} yaw {:.2} (game marker: 91.52, 43.85, -31.09 yaw 3.14 [D key 498])", bond.pos, bond.yaw);
    let mut moved_n = 0;
    for &(i, skin, st, team, pos, en, corpse) in &post {
        let p = pre.iter().find(|&&(j, s2, ..)| j == i).map(|(.., p, _, _)| *p);
        let moved = p.map(|pp| ((pp[0] - pos[0]).powi(2) + (pp[2] - pos[2]).powi(2)).sqrt()).unwrap_or(-1.0);
        if moved > 0.25 || !en {
            moved_n += 1;
            eprintln!("[kimiko] drone {i} skin {skin:#x} state {st:#04x} team {team} enabled {en} corpse {corpse} moved {moved:.2}: {pos:?} (pre {p:?})");
        }
    }
    eprintln!("[kimiko] {moved_n} of {} post drones moved >0.25 or disabled", post.len());
}

/// Probe: drone 23 (state 0xb4) across the Kimiko scene.
/// faster-live-tests: assertion-free evidence probe (print-only) — the same scenario's outcomes
/// are pinned by `the_guards_reappear…` below; `--ignored` keeps it for manual runs.
#[test]
#[ignore = "assertion-free probe (prints drone 23 across the Kimiko scene); the pinned behavior lives in the_guards_reappear_at_their_places_after_the_kimiko_scene — run with --ignored"]
fn probe_drone23() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000007.bin");
    for _ in 0..300 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    for i in [9usize, 16, 23] {
        let d = &scene.sim.as_ref().unwrap().drones.list[i];
        eprintln!("[d23] pre: drone {i} skin {:#x} state {:#04x} present {} enabled {} hp {:.0} second_ch {} second_tag {:#x} switch_ch {} wait_ch {} alt_state {:#x}", d.skin, d.state, d.present, d.enabled, d.health, d.second_channel, d.second_tag, d.switch_channel, d.wait_channel, d.alt_state);
    }
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [94.3, 44.6, -45.9];
        for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    }
    for t in 0..6000u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        if t % 500 == 0 {
            let d = &scene.sim.as_ref().unwrap().drones.list[23];
            eprintln!("[d23] t={t} disabled={} state {:#04x} present {} enabled {} corpse {} hp {:.0}", scene.sim.as_ref().unwrap().disabled, d.state, d.present, d.enabled, d.corpse, d.health);
        }
        if !scene.sim.as_ref().unwrap().disabled && t > 100 { break; }
    }
    {
        let d = &scene.sim.as_ref().unwrap().drones.list[23];
        eprintln!("[d23] post: state {:#04x} present {} enabled {} corpse {} hp {:.0}", d.state, d.present, d.enabled, d.corpse, d.health);
    }
}

/// ── Item 6: after the Kimiko cut-scene the guards are where the game puts them ──────────────────
/// Playtest: "Guards aren't in the proper place after the Kimiko cut-scene."
///
/// The game's post-NIS repositioning is exactly two things [M]:
/// - the PLAYER is teleported to the marker entity the script's event 6 named: `Script_Free`
///   0x1da880:44-47 → `Player_Enable(glb_players, SI+0x40, +0xAB6)` with the previous stream's
///   matrix (Script_EventHandler 0x1d94c0:125-134) — the hand-back scene 0x06000073's marker is
///   key 498 @t4325 = (91.52, 43.85, −31.09) yaw ≈ 180° [D nf-extract nis 07800007.bin 06000073];
/// - the DRONES are re-shown where they were: `Drone_EnableAll(1)` 0x1383e8 walks the llist
///   (skipping WaitSwitch drones, +0xF4 == 1 [:14]) → `NDrone2_Enable(1)` 0x149dd0:16-28 refuses
///   corpses (+0x4F8 & 0x200) and restarts the anim clock at 1.0 (+0x98 = 0x3f800000) — no drone
///   position is ever written.
///
/// The port already froze/kept every drone at its pre-scene position and teleported Bond to the
/// marker; what it missed was the clock restart — a guard frozen mid-stride resumed mid-stride
/// (the re-shown characters replay from their script's first frame in the game), and corpses
/// would have re-enabled.
#[test]
fn the_guards_reappear_at_their_places_after_the_kimiko_scene() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000007.bin");
    // Let the level settle (guards idle at their posts), snapshot, then walk into trigger 412.
    for _ in 0..300 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    // Pre-scene: an enabled, non-WaitSwitch drone whose animation is past its first frames.
    type Snap = Vec<(usize, u8, [f32; 3], u32, bool)>;
    let pre: Snap = scene.sim.as_ref().unwrap().drones.list.iter().enumerate()
        .filter(|(_, d)| d.present && d.enabled && d.state != nf_game::drone::state::WAIT_SWITCH)
        .map(|(i, d)| (i, d.state, d.pos, d.anim.time, d.corpse)).collect();
    let walker = pre.iter().copied().max_by_key(|&(_, _, _, t, _)| t).filter(|&(_, _, _, t, _)| t > 5);
    eprintln!("[party6] pre-scene: {} live drones, max anim.time {:?}",
        pre.len(), pre.iter().map(|&(_, _, _, t, _)| t).max());
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [94.3, 44.6, -45.9];
        for _ in 0..90 { sim.step_bond(Input::default(), false, false, false); }
    }
    for _ in 0..600u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        if scene.sim.as_ref().unwrap().disabled { break; }
    }
    for t in 0..6000u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        if !scene.sim.as_ref().unwrap().disabled { eprintln!("[party6] control back at +{t}"); break; }
    }
    // One tick after control returns: the re-shown drones have restarted their animations and
    // still stand where they stood.
    scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
    let sim = scene.sim.as_ref().unwrap();
    let bond = &sim.bond;
    eprintln!("[party6] bond {:?} yaw {:.2}", bond.pos, bond.yaw);
    assert!((bond.pos[0] - 91.52).abs() < 0.1 && (bond.pos[2] - -31.09).abs() < 0.1 && ((bond.yaw - std::f32::consts::PI).abs() < 0.06 || (bond.yaw + std::f32::consts::PI).abs() < 0.06),
        "Bond stands at the scene's return marker [D key 498; M 0x1da880:44-47] (yaw ±180°)");
    if let Some((i, _, pre_pos, pre_t, _)) = walker {
        let d = &sim.drones.list[i];
        eprintln!("[party6] drone {i} anim.time {pre_t} → {} pos {:?}", d.anim.time, d.pos);
        assert!(d.anim.time <= 2, "the re-shown drone restarts its animation from the script's first frame [M 0x149dd0:21-22] (was mid-stride at {pre_t})");
        let drift = ((pre_pos[0] - d.pos[0]).powi(2) + (pre_pos[2] - d.pos[2]).powi(2)).sqrt();
        eprintln!("[party6] drone {i} drift {drift:.2}");
        assert!(drift < 0.5, "the re-shown drone stands where it stood (no teleport in the game either [M 0x149dd0])");
    } else {
        eprintln!("[party6] no mid-animation drone before the scene — the clock restart is not observable this run");
    }
    // Nobody who was live pre-scene is left hidden while present: the re-shown drones are live,
    // WaitSwitch drones stay parked on their channels [M 0x1383e8:14], and the guests the scene's
    // channel removes (their second_channel is the trigger's 0x48 with the alternate state 0xb4
    // DeleteMe [M NDrone2_PreDroneControl]) leave present=false — the game streams them out too.
    for &(i, _, _, _, _) in &pre {
        let d = &sim.drones.list[i];
        assert!(!d.present || d.enabled || d.state == nf_game::drone::state::WAIT_SWITCH,
            "drone {i} ({:#04x}) was live before the scene: re-shown, parked on a switch, or removed — not hidden while present", d.state);
    }
}

/// ── Item 7: the 07000008 safe chain works in play ───────────────────────────────────────────────
/// Playtest: "There's a giant pillar just blocking" the missile closet (spot 152.35,43.13,-120.76:
/// the locked GuardRoom door #122 + bolt #217 + safe door #205 [D placements] — with nothing to
/// open them: the fusebox stand-in was invisible, the laser burn had no caller), "the gondola can't
/// be activated" (ch 0x4A is written only by the briefcase inside the sealed safe [D #290 →
/// Pickup_Handler 0x18e650:330-333]) and "Still can't pick up the rocket launcher" (behind the same
/// door [D #291, id 0x2C]).
///
/// The game's chain [M/D]: the GuardRoom fusebox #219 (0x31, script 0x060008AD) cuts a wire on the
/// second E → `FuseBox_Update` 0x1adfe8 writes ch 137 (param 1) → the bolt SS #217 opens and the
/// door #122 (lock param 3 = 137) unlocks. The safe bolts #207/#208 (0x2E, record word 1) burn
/// under the Lazer Watch 0x4E/0x4F's contacts (Lock_Update 0x1ae3d8 case 0: −3/−10 per hit at
/// +0x104/+0x105; Lock_Unlock 0x1b0650 writes switch_channels[record+8] = 134/135 [:194-195]) →
/// the mux #278 ANDs them into ch 136 [D] → the safe door SS #205 opens → the briefcase (ch 0x4A)
/// and the rocket launcher are reachable → the gondola ride starts (fix-objects2's chain).
///
/// Fail-first shown on the pre-fix tree: no fusebox SP, no mesh entity (the user's pillar had no
/// opener in play), and the watch burned nothing.
#[test]
fn the_07000008_safe_chain_works_in_play() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000008.bin");
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let (sp, had_mesh) = {
        let sim = scene.sim.as_ref().unwrap();
        (sim.drones.scripts.players.iter().any(|p| p.scripts[0] == 0x0600_08ad),
         sim.drones.scripts.objects.list.iter().flatten()
             .any(|o| matches!(o, nf_game::nis::Obj::Entity { src: (0x0600_08ad, _), .. })))
    };
    eprintln!("[safe] fusebox SP present {sp}, mesh entity {had_mesh}");
    assert!(sp && had_mesh, "the fusebox's own ScriptPlayer ran and built the visible box [M 0x1b0448 SP_Create mode 0x40]");

    // The GuardRoom fusebox cut through the REAL path: the box's script registers its wire
    // (`FuseBox_Register` 0x1b0970 via the script's event 0xF [L NF_SCRIPTLOG stream 2 → group 0]),
    // the Laser's beam bullets hit it (the 0x0002 category word survives `Collide_FilterBullets`'s
    // 0xfffd [L weapon_data 0x4E; M 0x1ece30:24-31]) and `FuseBox_Update` writes ch 137
    // [M 0x1adfe8:110-115]. `Player_Activate` 0x1a5170 has no class-0x3C arm — E does nothing.
    let (fb, wire_pos) = {
        let sim = scene.sim.as_ref().unwrap();
        let f = sim.drones.fuseboxes.iter().position(|f| f.ch[0] == 0x89).expect("the GuardRoom fusebox (ch 137) exists");
        let obj = sim.drones.fuseboxes[f].alive_wires().next().expect("the wire is registered by the box's script").2;
        let pos = nf_game::drone::Drones::script_obj_pos(&sim.drones.scripts, &sim.data(), obj).expect("the wire has a position");
        eprintln!("[safe] fusebox wire at {pos:?}");
        (f, pos)
    };
    {   // settle on the floor near the box, then close inside the Laser's 4.0 range [L +0x1C].
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pos = [wire_pos[0] + 1.2, wire_pos[1] - 1.0, wire_pos[2]];
        for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
        for _ in 0..4 {
            let head = sim.bond.head();
            let d = [wire_pos[0] - head[0], wire_pos[1] - head[1], wire_pos[2] - head[2]];
            let dist = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            if dist <= 2.4 { break; }
            let h = (d[0] * d[0] + d[2] * d[2]).sqrt();
            if h < 0.3 { break; }
            let step = (dist - 2.0).min(h);
            sim.bond.pos = [sim.bond.pos[0] + d[0] / h * step, sim.bond.pos[1], sim.bond.pos[2] + d[2] / h * step];
            for _ in 0..10 { sim.step_bond(Input::default(), false, false, false); }
        }
    }
    for _ in 0..40 { scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(0x4e), ..Default::default() }, false, false, false); }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x4e, "the Laser is selected [D USATxt 0x05000043]");
    let mut ch137 = false;
    for _ in 0..200u32 {
        {
            let sim = scene.sim.as_mut().unwrap();
            let head = sim.bond.head();
            let dir = [wire_pos[0] - head[0], wire_pos[1] - head[1], wire_pos[2] - head[2]];
            let l = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt().max(1e-4);
            sim.bond.yaw = dir[0].atan2(dir[2]);
            sim.bond.pitch = (dir[1] / l).asin() / (std::f32::consts::FRAC_PI_2);
        }
        scene.sim.as_mut().unwrap().step_bond(Input::default(), true, false, false);
        if scene.sim.as_ref().unwrap().drones.channels.on(137) { ch137 = true; break; }
    }
    eprintln!("[safe] ch137 {ch137} after the laser burn");
    assert!(ch137, "the laser burn cuts the wire and writes ch 137 [M 0x1adfe8 group 0 → param 1]");
    let _ = fb;
    let sim = scene.sim.as_mut().unwrap();
    let bolt_t = sim.drones.sss.list.iter().find(|s| s.channel == 137).map(|s| s.t).unwrap_or(-1.0);
    eprintln!("[safe] doorway bolt SS t={bolt_t}");
    assert!(bolt_t > 0.0, "the doorway bolt SS opens on ch 137 [D #217 param 3]");

    // The Lazer Watch (0x4E, owned with clip 500 [D loadout tail 0x1a7ef8:318-325]) burns the safe
    // bolts: stand 1.5 from each bolt, aim through its centre, hold fire.
    let input = Input { switch_to: Some(0x4e), ..Default::default() };
    for _ in 0..40 { sim.step_bond(input, false, false, false); } // the draw
    eprintln!("[safe] weapon {:#x}", sim.gun.weapon);
    assert_eq!(sim.gun.weapon, 0x4e, "the Lazer Watch is selected");
    // The mux #278 is an AND: both bolts burn (the game's two-bolt safe [D #207/#208, out 134/135]).
    for out in [134u16, 135] {
        let bolt = sim.drones.locks.iter().position(|l| l.out == out && !l.burned)
            .or_else(|| sim.drones.locks.iter().position(|l| l.out == out)).expect("the safe bolt exists");
        let (bp, br) = { let b = &sim.drones.locks[bolt]; (b.pos, b.radius) };
        eprintln!("[safe] bolt out {out} at {bp:?} r {br}");
        sim.bond.pos = [bp[0], bp[1] - 0.55, bp[2] + 1.5];
        for _ in 0..10 { sim.step_bond(Input::default(), false, false, false); } // settle (the solid door pushes out)
        let mut burned = false;
        for t in 0..200 {
            {   // re-aim every tick through the bolt's centre (the sim may nudge Bond).
                let head = sim.bond.head();
                let dir = [bp[0] - head[0], bp[1] - head[1], bp[2] - head[2]];
                let l = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt().max(1e-4);
                sim.bond.yaw = dir[0].atan2(dir[2]);
                sim.bond.pitch = (dir[1] / l).asin() / (std::f32::consts::FRAC_PI_2);
            }
            sim.step_bond(Input::default(), true, false, false);
            if t % 50 == 0 { eprintln!("[safe{out}] t={t} bond {:?} weapon {:#x} clip {}", sim.bond.pos, sim.gun.weapon, sim.gun.clip); }
            if sim.drones.locks[bolt].burned { burned = true; break; }
        }
        assert!(burned, "the watch's beam burns bolt out {out} [M 0x1ae3d8 case 0 arm 1]");
    }
    let mut ch136 = false;
    for _ in 0..60 { sim.step_bond(Input::default(), false, false, false);
        if sim.drones.channels.on(136) { ch136 = true; break; } }
    eprintln!("[safe] ch136 {ch136}");
    assert!(ch136, "both burns AND into ch 136 (mux #278 [D]) — the safe opens");
    let door_t = sim.drones.sss.list.iter().find(|s| s.channel == 136).map(|s| s.t).unwrap_or(-1.0);
    eprintln!("[safe] safe door SS t={door_t}");
    assert!(door_t > 0.0, "the safe door SS opens on ch 136 [D #205 param 3 = 0x88]");

    // The briefcase inside the safe writes ch 0x4A (the gondola's enable) [M 0x18e650:330-333];
    // the rocket launcher is a weapon pickup (id 0x2C).
    let case = sim.drones.pickups.iter().position(|p| p.kind == 4 && !p.gone)
        .or_else(|| sim.drones.pickups.iter().position(|p| !p.gone));
    let Some(case) = case else { panic!("the safe's pickups exist") };
    let pp = [sim.drones.pickups[case].m[3][0], sim.drones.pickups[case].m[3][1], sim.drones.pickups[case].m[3][2]];
    eprintln!("[safe] pickup {case} at {pp:?}");
    sim.bond.pos = [pp[0], pp[1] - 1.0, pp[2]];
    for _ in 0..20 { sim.step_bond(Input::default(), false, false, false); }
    let taken: Vec<String> = sim.drones.taken.iter().map(|t| format!("{t:?}")).collect();
    let ch4a = sim.drones.channels.on(0x4a);
    eprintln!("[safe] taken {taken:?} ch0x4a {ch4a}");
    assert!(ch4a, "taking the briefcase writes ch 0x4A [M 0x18e650:330-333]");
    assert!(taken.iter().any(|t| t.contains("Weapon")), "the rocket launcher (0x2C) is takeable [D #291]");

    // With 0x4A on, stepping onto the cable-car floor starts the ride (ch 4 → mux → ch 1).
    sim.bond.pos = [112.0, 28.0, -127.4];
    let mut ch1 = false;
    for _ in 0..60 { sim.step_bond(Input::default(), false, false, false);
        if sim.drones.channels.on(1) { ch1 = true; break; } }
    eprintln!("[safe] gondola ride ch1 {ch1}");
    assert!(ch1, "the gondola ride starts with the briefcase taken (ch 0x4A · ch 4 → mux → ch 1) [D #235/#236]");
}
