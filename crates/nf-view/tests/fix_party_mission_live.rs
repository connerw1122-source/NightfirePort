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
#[test]
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
#[test]
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
            let d = nf_game::drone::Data { tables: &sim.tables, props: &sim.props, scripts: &sim.scripts, weapons: &sim.weapons, world: &sim.world, level: sim.level, sequences: &sim.seqs, skins: &sim.skins, skeletons: &sim.skeletons, nav: &sim.nav, covers: &sim.covers, tuning: &sim.tuning, hitboxes: &sim.hitboxes, volumes: &sim.volumes, sfx: None, models: &sim.models, surfaces: None, voices: None };
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
#[test]
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
        let d = nf_game::drone::Data { tables: &sim.tables, props: &sim.props, scripts: &sim.scripts, weapons: &sim.weapons, world: &sim.world, level: sim.level, sequences: &sim.seqs, skins: &sim.skins, skeletons: &sim.skeletons, nav: &sim.nav, covers: &sim.covers, tuning: &sim.tuning, hitboxes: &sim.hitboxes, volumes: &sim.volumes, sfx: None, models: &sim.models, surfaces: None, voices: None };
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
        let d = nf_game::drone::Data { tables: &sim.tables, props: &sim.props, scripts: &sim.scripts, weapons: &sim.weapons, world: &sim.world, level: sim.level, sequences: &sim.seqs, skins: &sim.skins, skeletons: &sim.skeletons, nav: &sim.nav, covers: &sim.covers, tuning: &sim.tuning, hitboxes: &sim.hitboxes, volumes: &sim.volumes, sfx: None, models: &sim.models, surfaces: None, voices: None };
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
#[test]
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
/// style] and the port binds it through the USATxt.dat table; probe_objective_banners pins the
/// bound text on 07/01/02/08), "they show during cut-scenes — It needs to wait until after the
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
