//! fix-melee live tests: the fists 0x01 against the game, through the REAL path
//! (`nf_view::load` + `Sim::step_bond` — the trigger walks `Player_WeaponFiring` →
//! `Player_SetFiringAnim` → the state-9 release at the strike frame `+0x44` →
//! `Player_WeaponInitBullet` → the msg-6 punch).
//!
//! The game facts pinned here (every number cited):
//! - The fists record 1 [L ee_start_05.bin @ 0x2bf25c]: damage +0x0C = 5.0, range +0x1C = 1.75,
//!   speed +0x20 = 1.0, spread 0, one pellet, strike frame +0x44 = 6, refire +0x40 = 30,
//!   flags68 0x880A2 (0x80000 = the left/right pick), ammo_type 0, +0x91 = 0 (spends nothing),
//!   fire 0x060006de / fire_last 0x060006dd, no casing (+0x6C = 0x800 without 0x40, and
//!   ammo_data[0]'s casing mesh is 0 [L 0x2C69B8]).
//! - The fire scripts [D 07000005.bin kind-5 dump]: 0x060006de len 23, op-1 sound 0x142 at frame 7;
//!   0x060006dd len 23, op-1 0x142 at frame 9. Sound 0x142's hearing entry [L SFXOutputData]:
//!   inner 5, outer 30, weight 0 — a punch alerts nobody; 0x403 (the P2K shot) weighs 65.
//! - `NDrone2_PunchImpact` 0x145f28: the resume stamp, `Drone_AlertStatusSet(2)`, `DoHitEffects`,
//!   the level-07000007 unarmed arm (reason 5), the team-≠-1 arm on every level but 07000009/0a
//!   (unarmed → reason 6 [M :69-73]), health ≤ 0 → state 0x42 (KO; mode 0x1E would go 0xF2),
//!   `PainTalk`, the stagger 0x51 on allow, else the resume `SetState`.
//! - `NDrone2_HitDamage` 0x145558: a part-5 (head) hit with weapon 1/0x4a..0x4d reads as torso;
//!   the armour byte Drone+0xBB is 0 for every drone (DefaultInit 0x14b300:566 is the ONLY writer);
//!   the difficulty multiplier (07000005's ReadTuningVars leaves DroneDamage_* static [L]:
//!   torso 1.0, Easy 1.5 — the port's load runs difficulty 1, so a torso punch is 5·1·1.5 = 7.5).
//! - `Player_WeaponInitBullet` 0x1a4bf0 tail: `Bullet_init_casing` only when weapon+0x6C & 0x40.
//!
//! Opt-in via NF_ISO.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The first ENABLED team-1 guard (dtype 1) nearest to `p` — 07000005's Mayhew villa guards
/// (the probe: drones 2-6 enabled around the start, health 10.0, kind 2, weapons 0x16/0x24).
fn pick_guard(sim: &nf_view::scene::Sim, p: [f32; 3]) -> (usize, [f32; 3]) {
    let mut best: Option<(usize, f32)> = None;
    for (i, d) in sim.drones.list.iter().enumerate() {
        if !d.present || d.dtype != 1 || d.team != 1 || d.corpse || !d.enabled || d.health <= 0.0 { continue; }
        let d2 = (d.pos[0] - p[0]).powi(2) + (d.pos[2] - p[2]).powi(2);
        if best.map_or(true, |(_, bd)| d2 < bd) { best = Some((i, d2)); }
    }
    let i = best.expect("an enabled 05 guard exists").0;
    let pos = sim.drones.list[i].pos;
    (i, pos)
}

/// Stand Bond beside `target` facing its chest (the fire ray leaves the eye along the look).
fn aim_at(sim: &mut nf_view::scene::Sim, target: usize) {
    let g = sim.drones.list[target].pos;
    let eye = [sim.bond.pos[0], sim.bond.pos[1] + sim.bond.eye, sim.bond.pos[2]];
    sim.bond.yaw = (g[0] - eye[0]).atan2(g[2] - eye[2]);
    let horiz = ((g[0] - eye[0]).powi(2) + (g[2] - eye[2]).powi(2)).sqrt();
    sim.bond.pitch = ((g[1] + 0.25 - eye[1]) / horiz).atan() / (std::f32::consts::FRAC_PI_2);
}

/// Switch to the fists and punch `target` once: the trigger held 12 ticks (the fire at the press,
/// the round spawning at the strike frame), then released until the 30-tick refire window passes.
fn punch_once(sim: &mut nf_view::scene::Sim, target: usize) {
    sim.gun.weapon = 1; sim.gun.switch_request = 1; sim.gun.switch_state = 0;
    // The hold: Bond stays ~1.27 from the guard's CURRENT spot (the 05 guards strafe/attack between
    // punches — the game's punch also needs the eye inside 1.75 [L +0x1C] at the release frame).
    for _ in 0..12 {
        let g = sim.drones.list[target].pos;
        sim.bond.pos = [g[0] - 0.9, g[1], g[2] + 0.9];
        aim_at(sim, target);
        sim.step_bond(Input::default(), true, false, false);
    }
    for _ in 0..30 {
        aim_at(sim, target);
        sim.step_bond(Input::default(), false, false, false);
    }
}

/// ── Three punches on a 05 guard: the game's damage and reaction ────────────────────────────────
/// Punch 1 costs exactly 7.5 (5.0 [L record 1 +0x0C] × torso 1.0 [L 07000005 tuning] × Easy 1.5
/// [L tuning, the sim's loaded difficulty 1]) — the release at the strike frame +0x44 = 6 means the
/// health still reads 10.0 on the press tick [M Player_SetWeaponAnimObj case 9 0x1a2e38:416-420].
/// Punch 2 takes the 2.5-health guard to 0 → state 0x42 KnockedOut_Anim, NOT 0x44 [M 0x145f28:60-84].
/// Sister: the third punch on the KO'd body changes nothing.
#[test]
fn three_punches_on_a_05_guard_do_the_games_damage() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..600 { sim.step_bond(Input::default(), false, false, false); }
    let (guard, _) = pick_guard(sim, sim.bond.pos);
    let h0 = sim.drones.list[guard].health;
    assert_eq!(h0, 10.0, "the 05 guards start at 10.0 health [L probe]");

    // Punch 1: the press tick spawns nothing yet (the strike frame is 6).
    sim.gun.weapon = 1; sim.gun.switch_request = 1; sim.gun.switch_state = 0;
    {
        let g = sim.drones.list[guard].pos;
        sim.bond.pos = [g[0] - 0.9, g[1], g[2] + 0.9];
        aim_at(sim, guard);
        sim.step_bond(Input::default(), true, false, false);
    }
    assert_eq!(sim.drones.list[guard].health, 10.0,
        "the punch spawns at the strike frame +0x44 = 6, not on the press tick [M 0x1a2e38 case 9]");
    for _ in 0..11 {
        let g = sim.drones.list[guard].pos;
        sim.bond.pos = [g[0] - 0.9, g[1], g[2] + 0.9];
        aim_at(sim, guard);
        sim.step_bond(Input::default(), true, false, false);
    }
    let after_one = sim.drones.list[guard].health;
    assert!((after_one - 2.5).abs() < 1e-3,
        "one torso punch costs 5.0 · 1.0 · 1.5 = 7.5 [L record 1 +0x0C, 07000005 tuning, difficulty 1], got {after_one}");
    for _ in 0..30 { aim_at(sim, guard); sim.step_bond(Input::default(), false, false, false); }

    // Punch 2: 2.5 − 7.5 → the knockout.
    // Punch 2: 2.5 − 7.5 → the knockout.
    punch_once(sim, guard);
    {
        let g = &sim.drones.list[guard];
        assert!(g.health <= 0.0, "the second punch (2.5 health left) knocks the guard out, health {}", g.health);
        assert!(matches!(g.state, 0x42 | 0x43),
            "health ≤ 0 by a punch enters 0x42 KnockedOut_Anim (mode ≠ 0x1E), not 0x44 [M 0x145f28:60-84], state {:#x}", g.state);
    }

    // Sister: the third punch on the KO'd body does nothing.
    punch_once(sim, guard);
    let g = &sim.drones.list[guard];
    assert!(g.health <= 0.0 && matches!(g.state, 0x42 | 0x43),
        "a punch on a KO'd guard changes nothing (health {}, state {:#x})", g.health, g.state);
}

/// ── The punched guard turns hostile at once ────────────────────────────────────────────────────
/// `Drone_AlertStatusSet(2)` 0x145f28:30 → the port's alert ≥ 0.66, and the tail's resume
/// `SetState` (the common (ATTACK, false) pair) takes the idle guard straight into the attack
/// machinery — the reaction the driver's playtest described. Sister: the KO'd-by-punch team-1
/// guard never fails the mission (0x145bd0:20-22 — the death half's team-1 arm has no civilian
/// reason, unlike the guests').
#[test]
fn a_punched_05_guard_turns_hostile_at_once() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..600 { sim.step_bond(Input::default(), false, false, false); }
    let (guard, _) = pick_guard(sim, sim.bond.pos);
    let before = sim.drones.list[guard].alert;
    punch_once(sim, guard);
    let g = &sim.drones.list[guard];
    assert!(g.alert >= 0.66, "the punch sets AlertStatusSet(2) [M 0x145f28:30], alert {:.2} from {:.2}", g.alert, before);
    assert!(g.health > 0.0, "one punch on 10.0 health leaves the guard alive (7.5 damage)");
    assert!(g.state != 0x4 && g.state != 0x1 && g.state != 0x7,
        "the punched guard left the idle family (state {:#x} — the resume SetState 0x145f28 tail)", g.state);
    // The 05 guards are team 1: no mission fail reason of either kind [M 0x145f28:45-73].
    assert!(!sim.drones.channels.on(99) && !sim.drones.channels.on(0x60),
        "punching a 05 guard (team 1) raises no fail channel");
}

/// ── A punch does not alert the room like a gunshot ─────────────────────────────────────────────
/// The fire path fed every weapon's hearing through the P2K's 0x403 (weight 65, outer 40 [L
/// SFXOutputData]) — the game's punch sound is the fire scripts' op-1 0x142 at frames 7/9 [D dump],
/// whose hearing weight is **0** [L]: a punch alerts nobody until the victim screams (msg 0x12,
/// prop-gated) or SEES Bond. With the bug, every guard inside 40 units gets a 65-weight "shot" per
/// punch (`Sound_Alertness`: weight × the radius falloff → `+0x510`), alerting the room to a
/// gunfight that never happened. Asserts the second enabled guard ~8 units away hears NOTHING
/// (`loudness` stays 0 — the scream path never touches `+0x510`).
#[test]
fn a_punch_does_not_alert_the_room_like_a_gunshot() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..600 { sim.step_bond(Input::default(), false, false, false); }
    // The closest ENABLED pair of guards (drones 2/3 sit 8.1 apart at the villa entrance [L probe]):
    // the victim punched, the hearer 5-15 units away — inside the bug's 40-unit gunshot outer radius,
    // outside nothing relevant (the scream path never touches +0x510, so `loudness` isolates hearing).
    let cands: Vec<usize> = (0..sim.drones.list.len()).filter(|&i| {
        let d = &sim.drones.list[i];
        d.present && d.dtype == 1 && d.team == 1 && d.enabled && !d.corpse && d.health > 0.0
    }).collect();
    let mut best: Option<(usize, usize, f32)> = None;
    for (k, &a) in cands.iter().enumerate() {
        for &b in cands.iter().skip(k + 1) {
            let (pa, pb) = (sim.drones.list[a].pos, sim.drones.list[b].pos);
            let r = ((pa[0] - pb[0]).powi(2) + (pa[2] - pb[2]).powi(2)).sqrt();
            if r > 5.0 && r < 15.0 && best.map_or(true, |(_, _, br)| r < br) { best = Some((a, b, r)); }
        }
    }
    let (victim, hearer, _) = best.expect("two enabled guards 5-15 units apart on 07000005");
    let vpos = sim.drones.list[victim].pos;
    // Punch twice with the trigger held tick-by-tick; the shot lands at the strike frame
    // (+0x44 = 6), so sample the hearer's loudness on the shot tick and the one after (the
    // hearing pass consumes the sound list the tick it runs; +0x510 is per-frame in the game
    // [M 0x148a30:55-57] and read nowhere else, so only the shot window is meaningful).
    let mut max_loudness = 0.0f32;
    for _ in 0..2 {
        sim.gun.weapon = 1; sim.gun.switch_request = 1; sim.gun.switch_state = 0;
        let mut last_shot = sim.last_shot;
        for _ in 0..12 {
            let g = sim.drones.list[victim].pos;
            sim.bond.pos = [g[0] - 0.9, g[1], g[2] + 0.9];
            aim_at(sim, victim);
            sim.step_bond(Input::default(), true, false, false);
            if sim.last_shot != last_shot { last_shot = sim.last_shot; max_loudness = max_loudness.max(sim.drones.list[hearer].loudness); }
        }
        for _ in 0..30 {
            aim_at(sim, victim);
            sim.step_bond(Input::default(), false, false, false);
            if sim.last_shot != last_shot { last_shot = sim.last_shot; max_loudness = max_loudness.max(sim.drones.list[hearer].loudness); }
        }
    }
    assert_eq!(max_loudness, 0.0,
        "the hearer's loudness (+0x510) must stay 0: the punch's only sound weighs 0 [L SFXOutputData 0x142], not 65 like the 0x403 shot");
    assert!(sim.drones.list[victim].health <= 2.5, "the punches landed (the victim took the 7.5s)");
}

/// ── The swing whoosh ───────────────────────────────────────────────────────────────────────────
/// The fists' fire scripts carry op-1 sound 0x142 at frame 7 (0x060006de) / 9 (0x060006dd) [D
/// 07000005.bin dump] — the audible punch. The EE sound layer must see a Start(0x142) per swing.
/// Sister: the taser-CC twins 0x4b/0x4d's fire script 0x0600048d has NO op-1 [D] — their swing is
/// silent, and their hearing entry (whatever id) weighs 0.
#[test]
fn the_punch_swing_plays_the_whoosh() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..600 { sim.step_bond(Input::default(), false, false, false); }
        let (guard, _) = pick_guard(sim, sim.bond.pos);
        punch_once(sim, guard);
    }
    let listener = ([0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]);
    let sim = scene.sim.as_mut().unwrap();
    sim.drones.audio.update(None, listener, &|_| true);
    let starts: Vec<u32> = sim.drones.audio.out.iter().filter_map(|c| match c {
        nf_game::sound::Cmd::Start { id, .. } => Some(*id), _ => None,
    }).collect();
    assert!(starts.contains(&0x142), "the punch's swing sound 0x142 (op-1 frame 7/9 [D]) never played; starts: {starts:?}");
}

/// ── The alternation and its RNG draw ───────────────────────────────────────────────────────────
/// Flag 0x80000 (the fists' 0x880A2 [L record 1 +0x68]): `Player_SetFiringAnim` 0x1a3c70:34-36
/// draws ONE `Rand(100)` per shot; > 50 plays +0xB4 (the other fist). The port's
/// `Gun::firing_anim_script` must consume exactly one shared-RNG draw per punch (bit-exactness
/// downstream) and pick both scripts over a run. Sister: without the flag the fallback is +0xB0
/// with no draw.
#[test]
fn the_fists_alternate_with_one_rand_per_punch() {
    use nf_game::drone::Rng;
    let mut rng = Rng::new(0);
    // The expected picks, replayed against the shared generator's own rand(100).
    let mut replay = Rng::new(0);
    let expected: Vec<bool> = (0..40).map(|_| replay.rand(100) > 50).collect();
    let mut picks = Vec::new();
    for _ in 0..40 {
        // clip 1 (ammo_type 0 keeps it), fire 0x060006de, fire_last 0x060006dd.
        picks.push(nf_game::gun::Gun::new(1, 1, 0)
            .firing_anim_script(0x880a2, 1, 1, 0x0600_06de, 0x0600_06dd, &mut rng));
    }
    for (k, (got, want_left)) in picks.iter().zip(expected.iter()).enumerate() {
        let want = if *want_left { 0x0600_06dd } else { 0x0600_06de };
        assert_eq!(*got, want, "punch {k}: the 0x80000 pick diverged from one Rand(100) > 50 per shot");
    }
    assert!(picks.contains(&0x0600_06de) && picks.contains(&0x0600_06dd),
        "both fists' scripts get picked over a run (the left/right alternation)");
    // Sister: a weapon without 0x80000 always plays +0xB0 and never draws.
    let mut rng2 = Rng::new(1);
    let mut replay2 = Rng::new(1);
    for _ in 0..8 {
        let pick = nf_game::gun::Gun::new(2, 7, 41)
            .firing_anim_script(0x20, 2, 5, 0x0600_0827, 0x0600_0828, &mut rng2);
        assert_eq!(pick, 0x0600_0827, "without 0x80000 the fallback is the fire script");
        assert_eq!((rng2.x, rng2.y), (replay2.x, replay2.y), "no draw without 0x80000");
    }
}
