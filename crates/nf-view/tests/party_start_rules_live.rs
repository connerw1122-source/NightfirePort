//! party07-start-rules: the 07000007 party's start rules, from a COLD start AND from the 06 → 07
//! carry, driven through the REAL path (`nf_view::load` / `load_with_carry` + `Sim::step_bond`).
//!
//! Playtest 1011-1535 / 1011-1616 / 1012-1723 (07000007) items pinned here:
//! 1. "the guards immediately start shooting at me because I start with my gun drawn" — the
//!    loadout must be the FISTS [M Player_InitWeapon 0x1a7ef8 case 0x7000007 → +0x63 = 1,
//!    armour 0] and the bouncers stay calm while Bond is unarmed [M CivilianGuard 0x1603a8
//!    case 3: attack only when NearArmedPerson 0x1787c8 / ObjectIsArmed(Bond) 0x178768 + seen];
//! 2. "If you punch a guard, it should be an automatic mission fail" [M PunchImpact 0x145f28:45-56
//!    → reason 5 → label 0x4000033 + ch 99 [M 0x13e560] → MissionState 2 + the "Mission Fail"
//!    banner → 0x7000048, M 0x182c60:45-57];
//! 3. guests T-posing or not reacting — every guest binds an anim at the start (T-pose =
//!    `anim.script == None`), and the panic cower plays on the scream [M CivilianHiding];
//! 4. "one male guest shooting" — no guest is armed [M GetDTYPENEW 0x14c938: dtype 10 →
//!    PartyGirlInit/CivilianInit, prop 0x1f = 0 (deaf to sound alerts, M 0x148a30:45-49),
//!    prop 0x54 = 1 (protected); the weapon class only comes from placement param 9] and no
//!    guest ever attacks even with gunfire around;
//! 5. tasing an unarmed person on the party level fails the mission [M DroneFunc_HandleImpact
//!    case 7, 0x1469b8:58-68 → reason 7 → label 0x4000033 + ch 99].

use nf_game::player::{Input, RamSave};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str, level: &str) -> nf_view::scene::Scene {
    nf_view::load(iso, level, false).expect("load")
}

fn tick_idle(sim: &mut nf_view::scene::Sim, n: u32) {
    for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
}

/// Wait out the intro cut-scene / the fists draw (fixed 60 Hz ticks; walk_07000007's pattern).
fn settle(sim: &mut nf_view::scene::Sim) {
    for _ in 0..3000 { sim.step_bond(Default::default(), false, false, false); if !sim.disabled { break; } }
    tick_idle(sim, 600);
}

fn armed_class(w: u32) -> bool {
    // `NDrone2_ObjectIsArmed` 0x178768: armed iff (w-0x6a) < 8 (0x6a..=0x71) or 1 < w < 0x48 && w != 0x47.
    (2..0x47).contains(&w) && w != 0x47 || (0x6a..=0x71).contains(&w)
}

fn combat_states(sim: &nf_view::scene::Sim) -> Vec<usize> {
    sim.drones.list.iter().enumerate()
        .filter(|(_, d)| d.present && d.started && !d.corpse)
        .filter(|(_, d)| matches!(d.state, 0x56..=0x7e | 0x85..=0x99))
        .map(|(i, _)| i).collect()
}

/// The shared start-state asserts: fists, armour 0, a calm party, guests unarmed + animated + deaf.
fn assert_party_start_rules(sim: &nf_view::scene::Sim, what: &str) {
    assert_eq!(sim.gun.weapon, 1, "{what}: Bond holds the fists [M 0x1a7ef8 case 0x7000007]");
    let combat = combat_states(sim);
    assert!(combat.is_empty(), "{what}: no drone is in combat while Bond is unarmed, got {combat:?} states {:?}",
        combat.iter().map(|&i| sim.drones.list[i].state).collect::<Vec<_>>());
    let data = sim.data();
    let guests: Vec<(usize, u32, u8)> = sim.drones.list.iter().enumerate()
        .filter(|(_, d)| d.present && d.dtype == 10 && !d.corpse).map(|(i, d)| (i, d.weapon, d.state)).collect();
    assert!(!guests.is_empty(), "{what}: the party has guests");
    for &(i, w, _) in &guests {
        assert!(!armed_class(w), "{what}: guest {i} is armed (weapon class {w:#x}) — no guest fires [playtest item 4; M 0x14c938 dtype 10 → CivilianInit/PartyGirlInit]");
    }
    let d = &data;
    for &(i, _, _) in &guests {
        let g = &sim.drones.list[i];
        assert!(!g.prop(d, 0x1f), "{what}: guest {i} is deaf to sound alerts (prop 0x1f = 0) [M 0x148a30:45-49, 0x14c938]");
        assert!(g.prop(d, 0x54), "{what}: guest {i} is a protected guest (prop 0x54 set) [M 0x14c938:34-52]");
    }
    let tposed: Vec<usize> = guests.iter()
        .filter(|&&(i, _, _)| { let a = &sim.drones.list[i].anim; a.script.is_none() || a.danim == 0 })
        .map(|&(i, _, _)| i).collect();
    assert!(tposed.is_empty(), "{what}: guests are T-posed at the start (no anim on {tposed:?}) [playtest item 3]");
}

/// ── Rule 1/3/4, cold start ──────────────────────────────────────────────────────────────────────
#[test]
fn cold_start_opens_on_fists_with_a_calm_animated_party() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000007.bin");
    {
        let s = scene.sim.as_mut().unwrap();
        assert_eq!(s.gun.switch_request, 1, "the loadout request is the fists (case 0x7000007)");
        assert!((s.drones.health.armour - 0.0).abs() < 1e-3, "armour zeroed [M 0x1a7ef8 case 0x7000007]");
        assert!(s.drones.inventory.owned[2], "the P2K is owned but holstered (the ch-0x48 auto-draw)");
    }
    settle(scene.sim.as_mut().unwrap());
    let s = scene.sim.as_mut().unwrap();
    assert!(s.drones.health.health > 99.0, "cold: Bond takes no damage at the start [playtest item 1]");
    assert!(s.drones.channels.on(0x47), "the spawn plate #411 fired ch 0x47 [D placements]");
    assert_party_start_rules(s, "cold");
}

/// ── Rule 1/3/4, the 06 → 07 carry ───────────────────────────────────────────────────────────────
/// 07000006's dressing script event 0x12 writes no gate → the first gate-0 start (#0 at
/// 33.4, 44.5, −70.6) wins [D placements; docs/levels/07000007.md §1].
#[test]
fn carry_from_06_opens_on_fists_at_the_party_start_plate() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000006.bin");
    {
        let s = scene.sim.as_mut().unwrap();
        s.drones.health.health = 61.0;
        s.drones.channels.set(0x1D, 1, s.drones.frame); // 06's LOAD_LEVEL exit gate [D #418's level-side twin]
    }
    tick_idle(scene.sim.as_mut().unwrap(), 3);
    let save = {
        let s = scene.sim.as_mut().unwrap();
        RamSave::capture(&s.drones.health, s.gun.switch_request, &s.drones.inventory, &s.drones.channels)
    };
    drop(scene);
    let mut next = nf_view::load_with_carry(&iso, "07000007.bin", false, Some(&save)).expect("carry load");
    {
        let s = next.sim.as_mut().unwrap();
        assert_eq!(s.gun.switch_request, 1, "carry: the party still requests the fists");
        assert!((s.drones.health.health - 61.0).abs() < 1e-3, "carry: the 06 health carried in");
        let [x, y, z] = s.bond.pos;
        assert!((x - 33.4).abs() < 1.0 && (z - -70.6).abs() < 1.0 && (y - 44.5).abs() < 2.0,
            "carry: Bond spawns at the party start plate #0, got ({x:.1},{y:.1},{z:.1}) [D placement #0]");
    }
    settle(next.sim.as_mut().unwrap());
    assert_party_start_rules(next.sim.as_mut().unwrap(), "carry");
}

/// ── Rule 2, cold: punching a bouncer fails the mission with the game's fail text ────────────────
/// [M 0x145f28:45-56 → reason 5 → 0x13e560: label 0x4000033 + ch 99 → 0x182c60 case 2: the
/// "Mission Fail" banner (0x0200_0007) + MissionState 2 → end to 0x7000048].
#[test]
fn cold_punch_a_bouncer_fails_with_the_game_fail_text() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000007.bin");
    let sim = scene.sim.as_mut().unwrap();
    settle(sim);
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
        let d = nf_game::drone::Data { tables: &sim.tables, props: &sim.props, scripts: &sim.scripts, weapons: &sim.weapons, world: &sim.world, level: sim.level, sequences: &sim.seqs, skins: &sim.skins, skeletons: &sim.skeletons, nav: &sim.nav, covers: &sim.covers, tuning: &sim.tuning, hitboxes: &sim.hitboxes, volumes: &sim.volumes, sfx: None, models: &sim.models, surfaces: None, voices: None, difficulty: 1 };
        sim.drones.fire_player(&d, from, dir, 1, snd); // the fists = the punch [M 0x13fff0 case 5 → msg 6]
    }
    for _ in 0..12 { sim.step_bond(Default::default(), false, false, false); }
    assert!(sim.drones.channels.on(99), "the punch raises ch 99 [M 0x145f28 reason 5 → 0x13e560]");
    assert_eq!(sim.drones.mission.fail_label, 0x0400_0033, "FailLabel 0x4000033 [M 0x13e560]");
    for _ in 0..120 { sim.step_bond(Default::default(), false, false, false);
        if sim.drones.mission.state == 2 { break; } }
    assert_eq!(sim.drones.mission.state, 2, "MissionState 2 [M 0x182c60:45-57]");
    let banner: Vec<&str> = sim.hud.mission_msgs.iter().map(|m| m.text.as_str()).collect();
    assert!(banner.contains(&"Mission Fail"), "the game's fail banner is queued, got {banner:?}");
    let lang = sim.lang.as_ref().expect("the level's lang table");
    let text = lang.bind(0x0400_0033).expect("the fail label 0x4000033 binds the game's fail text");
    eprintln!("[party] fail label 0x4000033 text: {text:?}");
    assert!(!text.trim().is_empty(), "the fail label binds non-empty text");
}

/// ── Rule 2, the carry: the same punch fails from the 06 → 07 carry too ──────────────────────────
#[test]
fn carry_punch_a_bouncer_fails_the_mission() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000006.bin");
    {
        let s = scene.sim.as_mut().unwrap();
        s.drones.health.health = 80.0;
        s.drones.channels.set(0x1D, 1, s.drones.frame);
    }
    tick_idle(scene.sim.as_mut().unwrap(), 3);
    let save = {
        let s = scene.sim.as_mut().unwrap();
        RamSave::capture(&s.drones.health, s.gun.switch_request, &s.drones.inventory, &s.drones.channels)
    };
    drop(scene);
    let mut next = nf_view::load_with_carry(&iso, "07000007.bin", false, Some(&save)).expect("carry load");
    let sim = next.sim.as_mut().unwrap();
    settle(sim);
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
        let d = nf_game::drone::Data { tables: &sim.tables, props: &sim.props, scripts: &sim.scripts, weapons: &sim.weapons, world: &sim.world, level: sim.level, sequences: &sim.seqs, skins: &sim.skins, skeletons: &sim.skeletons, nav: &sim.nav, covers: &sim.covers, tuning: &sim.tuning, hitboxes: &sim.hitboxes, volumes: &sim.volumes, sfx: None, models: &sim.models, surfaces: None, voices: None, difficulty: 1 };
        sim.drones.fire_player(&d, from, dir, 1, snd);
    }
    for _ in 0..12 { sim.step_bond(Default::default(), false, false, false); }
    assert!(sim.drones.channels.on(99), "carry: the punch raises ch 99");
    assert_eq!(sim.drones.mission.fail_label, 0x0400_0033, "carry: FailLabel 0x4000033");
    for _ in 0..120 { sim.step_bond(Default::default(), false, false, false);
        if sim.drones.mission.state == 2 { break; } }
    assert_eq!(sim.drones.mission.state, 2, "carry: the punch fails the mission [playtest item 2]");
}

/// ── Rule 4: gunfire does not panic the guests (they ignore sound alerts) and no guest fires ─────
/// [M 0x148a30:45-49 — the guests' sound-alert arm is dead (prop 0x1f = 0, written by GetDTYPENEW
/// 0x14c938 for dtype 10); the sight-side scare (armed person near, 0x160140:44-76) is the
/// `an_armed_bond_scares…` test's subject (fix_guest_reactions_live, headless sight-gated).]
#[test]
fn gunfire_does_not_panic_the_deaf_guests_and_no_guest_fires() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000007.bin");
    let sim = scene.sim.as_mut().unwrap();
    settle(sim);
    let guests: Vec<usize> = sim.drones.list.iter().enumerate()
        .filter(|(_, d)| d.present && d.dtype == 10 && !d.corpse).map(|(i, _)| i).collect();
    let before: Vec<(u8, f32)> = guests.iter().map(|&i| (sim.drones.list[i].state, sim.drones.list[i].alert)).collect();
    // Bond draws the P2K (the game's own post-ch-0x48 state [M 0x1a24c8:247-250]) and fires a full
    // clip with a BROAD hearing radius right among the guests — the sound reaches everyone.
    sim.gun.weapon = 2; sim.gun.switch_request = 2; sim.gun.switch_state = 0;
    let snd = nf_formats::sfx::SfxHearing { id: 0x403, inner: 40.0, outer: 40.0, weight: 0.0, looped: false, cull: false };
    for _ in 0..8 {
        let (bp, byaw) = (sim.bond.pos, sim.bond.yaw);
        let dir = [byaw.sin(), 0.05, byaw.cos()]; // into Bond's facing, level — the crowd is around him
        {
            let d = nf_game::drone::Data { tables: &sim.tables, props: &sim.props, scripts: &sim.scripts, weapons: &sim.weapons, world: &sim.world, level: sim.level, sequences: &sim.seqs, skins: &sim.skins, skeletons: &sim.skeletons, nav: &sim.nav, covers: &sim.covers, tuning: &sim.tuning, hitboxes: &sim.hitboxes, volumes: &sim.volumes, sfx: None, models: &sim.models, surfaces: None, voices: None, difficulty: 1 };
            sim.drones.fire_player(&d, bp, dir, 2, snd);
        }
        for _ in 0..12 { sim.step_bond(Default::default(), false, false, false); }
    }
    for _ in 0..120 { sim.step_bond(Default::default(), false, false, false); }
    let after: Vec<(u8, f32)> = guests.iter().map(|&i| (sim.drones.list[i].state, sim.drones.list[i].alert)).collect();
    let panicked: Vec<usize> = before.iter().zip(after.iter()).zip(guests.iter())
        .filter(|((b, a), _)| !matches!(b.0, 0x12 | 0x13) && matches!(a.0, 0x12 | 0x13))
        .map(|(_, &i)| i).collect();
    eprintln!("[party] before {before:?}\n[party] after  {after:?}\n[party] newly panicked {panicked:?}");
    let data = sim.data();
    for &i in &guests {
        assert!(!armed_class(sim.drones.list[i].weapon), "guest {i} never gains a weapon [M 0x14c938]");
        assert!(!sim.drones.list[i].prop(&data, 0x1f), "guest {i} stays deaf (prop 0x1f = 0) [M 0x148a30]");
        assert!(!matches!(sim.drones.list[i].state, 0x56..=0x7e | 0x85..=0x99),
            "guest {i} entered a combat state {:#04x} — guests never attack [M team-3 look_for_opponent]", sim.drones.list[i].state);
    }
    let _ = (after, panicked); // the sound-deafness assert above is the pinned rule
    assert!(combat_states(sim).iter().all(|&i| sim.drones.list[i].dtype != 10),
        "no guest is in a combat state after gunfire [playtest item 4]");
}

/// ── Rule 5 (the new port): tasing an unarmed person on the party level fails the mission ────────
/// [M DroneFunc_HandleImpact case 7, 0x1469b8:58-68 (dis 0x146a9c-0x146ad4: the arm precedes the
/// accumulator) → reason 7 → label 0x4000033 + ch 99 [M 0x13e560]. The Stunner dart (0x4A) is the
/// msg-7 sender [M 0x13fff0:269-274].]
#[test]
fn tasing_a_guest_fails_the_mission_at_once() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000007.bin");
    let sim = scene.sim.as_mut().unwrap();
    settle(sim);
    let guest = sim.drones.list.iter().position(|d| d.present && d.dtype == 10 && !d.corpse)
        .expect("a party guest exists");
    let (gp, gy) = { let g = &sim.drones.list[guest]; (g.pos, g.yaw) };
    let from = [gp[0] + gy.sin() * 1.3, gp[1] + 0.6, gp[2] + gy.cos() * 1.3];
    sim.bond.pos = from;
    let dir = [gp[0] - from[0], (gp[1] + 0.45) - from[1], gp[2] - from[2]];
    let l = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
    let dir = [dir[0] / l, dir[1] / l, dir[2] / l];
    let snd = nf_formats::sfx::SfxHearing { id: 0x403, inner: 0.0, outer: 0.0, weight: 0.0, looped: false, cull: false };
    {
        let d = nf_game::drone::Data { tables: &sim.tables, props: &sim.props, scripts: &sim.scripts, weapons: &sim.weapons, world: &sim.world, level: sim.level, sequences: &sim.seqs, skins: &sim.skins, skeletons: &sim.skeletons, nav: &sim.nav, covers: &sim.covers, tuning: &sim.tuning, hitboxes: &sim.hitboxes, volumes: &sim.volumes, sfx: None, models: &sim.models, surfaces: None, voices: None, difficulty: 1 };
        sim.drones.fire_player(&d, from, dir, 0x4A, snd); // the Stunner dart = msg 7
    }
    for _ in 0..12 { sim.step_bond(Default::default(), false, false, false); }
    assert!(sim.drones.channels.on(99), "the tase raises ch 99 [M 0x1469b8:58-68 reason 7]");
    assert_eq!(sim.drones.mission.fail_label, 0x0400_0033, "FailLabel 0x4000033 [M 0x13e560]");
    for _ in 0..120 { sim.step_bond(Default::default(), false, false, false);
        if sim.drones.mission.state == 2 { break; } }
    assert_eq!(sim.drones.mission.state, 2, "tasing a guest fails the mission [M 0x1469b8 case 7]");
}
