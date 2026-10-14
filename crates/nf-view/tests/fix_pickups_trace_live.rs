//! fix-pickups-trace repro tests, driven through the real path (`nf_view::load` + `Sim::step_bond`).
//!
//! The real game's PCSX2 trace (tests/traces/01-raptor.trace, god=1 + the recorder's infinite-ammo
//! topping) on 07000001:
//! - 6.0 s: the first Raptor-guard drop is taken as **weapon 0x0E with clip 9** — the Raptor Magnum
//!   (weapon_data[0x0E]: base 0x0E, ammo type 3, clip 9, name label 0x0500002D = "Raptor Magnum"
//!   [D USATxt.dat]; the guard's weapon comes from the placement's param 9 = 0xE [D 07000001.bin],
//!   the drop id is `weapon_data[w]+0x02` = 0xE [M DroneWeap_DropWeapon 0x17a2a0:96-98],
//!   `Upgrade_Weapon(0xE)` = 0xE [M 0x1a8f30 — not in any remap table]);
//! - every later Raptor drop is taken as ammo type 3 (+6 at 19.8 s, +7, +7, +5) — spare `BLData+0x170+6`
//!   grows only by real takes (0→6→13→20→25, never topped, always below the type-3 cap 36 [D ammo_data]);
//!   the drop rounds are `clip_size(0xE)=9 · (Rand_FRand(0.5)+0.5)` truncated = 4..8 (the clip < 10
//!   branch [M 0x17a2a0:88-98]) — all four trace deltas sit in that window;
//! - with the recorder's cheat (tools/trace/trace_record.py --god --ammo, what the user played with)
//!   every drop the user walked over was taken: the cheat tops each **owned** weapon's clip and each
//!   spare type back to the most seen this run, so spares never exceed a take and no cap refuses.
//!
//! The port's bug: the dev-mode infinite-ammo topping wrote `carried[i] = max(carried[i], 200)` —
//! above every ammo cap (type 3 caps at 36) — and topped **unowned** weapons' clips to their clip
//! size. Every later drop then refuses in `Player_EquipAmmo`'s cap check ("some later drops can't be
//! picked up" after the first equips), and the first take pushes the whole drop's rounds to the spare
//! (the game's take adds only the rounds beyond a from-zero clip: none). This file shows both.
//!
//! Opt-in via `NF_ISO`.

use nf_view::dev::Dev;
use nf_view::scene::Scene;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str, level: &str) -> Scene {
    let mut scene = nf_view::load(iso, level, false).expect("load");
    // 07000001's level-start intro (SP #277, 0x06000087) takes control; skip it the game's way
    // (hold the skip action, `Script_FFwd` 0x1dac78) so the tests start in control.
    let intro: &[u32] = match level { "07000001.bin" => &[0x0600_0087], _ => &[] };
    if !intro.is_empty() {
        for t in 0..600u32 {
            let sim = scene.sim.as_mut().unwrap();
            let pending = sim.drones.scripts.players.iter().any(|p| intro.contains(&p.scripts[0]) && (matches!(p.state, 1 | 2 | 7) || p.info[p.cur].as_ref().is_some_and(|s| s.play & 1 != 0)));
            if t >= 2 && !pending && !sim.disabled { break; }
            sim.drones.scripts.skip_request = true;
            sim.step_bond(Default::default(), false, false, false);
        }
    }
    scene
}

/// The user's play setup (the trace header `god=1` + the recorder's --ammo): god mode and the
/// infinite-ammo topping, applied after every tick the way the App loop does (lib.rs: the dev
/// after-tick runs `Dev::after_tick` once per stepped tick).
fn cheats() -> Dev { let mut d = Dev::default(); d.god_mode = true; d.infinite_ammo = true; d }

/// One stepped tick (the App loop's order: `step_bond`, then the dev after-tick).
fn step(scene: &mut Scene, trigger: bool, dev: Option<&mut Dev>) {
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Default::default(), trigger, false, false);
    }
    if let Some(d) = dev { d.after_tick(scene); }
}

fn tick_idle(scene: &mut Scene, n: u32, mut dev: Option<&mut Dev>) {
    for _ in 0..n { step(scene, false, dev.as_deref_mut()); }
}

/// Point Bond's view at `to` from his eye (setup helper).
fn aim_at(scene: &mut Scene, to: [f32; 3]) {
    let sim = scene.sim.as_mut().unwrap();
    let e = sim.bond.head();
    let d = [to[0] - e[0], to[1] - e[1], to[2] - e[2]];
    let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-6);
    sim.bond.yaw = d[0].atan2(d[2]);
    sim.bond.pitch = (d[1] / l).clamp(-1.0, 1.0).asin() / (std::f32::consts::FRAC_PI_2);
}

/// Kill guard `gi` through the real fire path: stand at the nearest spot around him that has a
/// clear line to his chest (alerted guards run for cover; a fixed south spot is often walled),
/// tap the semi-auto trigger, re-aiming every tick. Then let the drop fall and land. Returns
/// whether the guard died.
fn kill_guard(scene: &mut Scene, gi: usize, mut dev: Option<&mut Dev>) -> bool {
    // Firing spots: 8 directions × 2 radii around the guard, on his floor.
    let spots = |gp: [f32; 3], floor: f32| -> Vec<[f32; 3]> {
        let mut v = Vec::new();
        for r in [3.0f32, 4.5] {
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::FRAC_PI_4;
                v.push([gp[0] + r * a.sin(), floor + 1.1, gp[2] + r * a.cos()]);
            }
        }
        v
    };
    let mut spot = 0usize;
    for t in 0..2000 {
        let (gp, gchest) = { let dr = &scene.sim.as_ref().unwrap().drones.list[gi]; (dr.pos, [dr.pos[0], dr.pos[1] + 0.3, dr.pos[2]]) };
        let los = |from: [f32; 3]| scene.sim.as_ref().unwrap().world.line_of_sight([from[0], from[1] + 1.0, from[2]], gchest);
        let floor = scene.sim.as_ref().unwrap().world.floor_below(gp, 6.0).unwrap_or(gp[1]);
        let cands = spots(gp, floor);
        // Move on when the current spot lost sight (the guard runs) or at the start.
        if t == 0 || !los(cands[spot % cands.len()]) {
            spot += 1;
            let Some(next) = (spot..spot + cands.len()).map(|i| i % cands.len()).find(|&i| los(cands[i])) else { continue };
            spot = next;
            let sim = scene.sim.as_mut().unwrap();
            sim.bond = nf_game::player::Bond::stand_at(&sim.world, cands[spot], 0.0);
        }
        aim_at(scene, gchest);
        step(scene, t % 9 == 0, dev.as_deref_mut());
        if scene.sim.as_ref().unwrap().drones.list[gi].health <= 0.0 { break; }
    }
    let dead = scene.sim.as_ref().unwrap().drones.list[gi].health <= 0.0;
    tick_idle(scene, 90, dev.as_deref_mut()); // the drop falls and lands (Pickup_Update's fall)
    dead
}

/// Stand Bond on drop `k` (then at the four cardinal offsets, for drops that landed against
/// geometry) and tick until it is taken. Returns whether it was taken.
fn take_drop(scene: &mut Scene, k: usize, mut dev: Option<&mut Dev>) -> bool {
    let at = {
        let sim = scene.sim.as_ref().unwrap();
        let p = &sim.drones.pickups[k];
        [p.m[3][0], p.m[3][1] + 0.4, p.m[3][2]]
    };
    for (dx, dz) in [(0.0f32, 0.0f32), (2.0, 0.0), (-2.0, 0.0), (0.0, 2.0), (0.0, -2.0)] {
        {
            let sim = scene.sim.as_mut().unwrap();
            sim.bond = nf_game::player::Bond::stand_at(&sim.world, [at[0] + dx, at[1], at[2] + dz], 0.0);
        }
        for _ in 0..40 {
            let before = scene.sim.as_ref().unwrap().drones.taken.len();
            step(scene, false, dev.as_deref_mut());
            let sim = scene.sim.as_ref().unwrap();
            if sim.drones.taken.len() > before { return true; }
            if sim.drones.pickups.get(k).is_some_and(|p| p.gone) { return true; }
        }
    }
    false
}

/// The Raptor guards (weapon 0xE — the placement param 9 [D 07000001.bin]) that are in the world
/// (present, alive): switch-gated ones included — the test enables each target's own gate channel
/// (`Drone+switch_channel` = placement param 3 [M NDrone2_DefaultInit]), the stand-in for the
/// walking that turns those channels on in play (the trace's user walked the level for 76 s).
fn raptor_targets(scene: &Scene) -> Vec<usize> {
    scene.sim.as_ref().unwrap().drones.list.iter().enumerate()
        .filter(|(_, dr)| dr.present && !dr.corpse && dr.health > 0.0 && dr.weapon == 0xe)
        .map(|(i, _)| i).collect()
}

/// Make sure guard `gi` is in play: a switch-gated one has its gate channel set (the level's own
/// gates — the channels the triggers/scripts of a walking player would set).
fn ensure_enabled(scene: &mut Scene, gi: usize, dev: Option<&mut Dev>) {
    let ch = scene.sim.as_ref().unwrap().drones.list[gi].switch_channel;
    let on = scene.sim.as_ref().unwrap().drones.list[gi].enabled;
    if !on && ch != 0 {
        eprintln!("[raptor] guard {gi} waits on ch {ch:#x} — enabling (the walking stand-in)");
        scene.sim.as_mut().unwrap().drones.set_channel(ch);
        tick_idle(scene, 30, dev);
    }
}

/// ── the trace's first take ───────────────────────────────────────────────────────────────────────
/// 6.0 s: weapon 0x0E becomes owned with clip 9 (`weap 52710 e clip 0->9 owned 0->1`), and the
/// trace has NO spare type-3 event there — the drop's rounds (≤ 9) all fit the from-zero clip
/// (`Player_EquipWeapon` 0x1a8a58: clip = clip_size, the excess carried). The HUD pickup message
/// binds the RAW id's name label (0x0500002D = "Raptor Magnum" [D USATxt.dat]).
#[test]
fn the_first_raptor_drop_is_weapon_0x0e_clip_9_and_the_hud_names_it() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000001.bin");
    tick_idle(&mut scene, 200, None);
    let guards = raptor_targets(&scene);
    eprintln!("[raptor] 0xE guards after the intro: {guards:?}");
    assert!(!guards.is_empty(), "07000001 has Raptor (0xE) guards");
    let gi = guards[0];
    ensure_enabled(&mut scene, gi, None);
    assert!(kill_guard(&mut scene, gi, None), "the guard died");
    let drop = scene.sim.as_ref().unwrap().drones.pickups.iter().position(|p| p.from == Some(gi))
        .expect("the dead guard dropped his gun");
    {
        let p = &scene.sim.as_ref().unwrap().drones.pickups[drop];
        assert_eq!((p.kind, p.weapon), (nf_game::pickup::kind::WEAPON, 0xe),
            "the drop is a kind-0 pickup of weapon_data[0xE]+0x02 = 0xE [M 0x17a2a0:96-98]");
    }
    assert!(take_drop(&mut scene, drop, None), "Bond takes the drop");
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.taken.iter().any(|(_, t)| matches!(t, nf_game::pickup::Taken::Weapon { weapon: 0xe })),
        "taken = Weapon {{ 0xE }}: {:?}", sim.drones.taken);
    assert!(sim.drones.inventory.owned[0xe], "0x0E owned after the take");
    assert_eq!(sim.drones.inventory.clip[0xe], 9, "clip = weapon_data[0x0E]+0x92 = 9 [D]");
    assert_eq!(sim.drones.inventory.carried.get(3), Some(&0),
        "no spare type-3 event at the first take (the trace has none until 19.8 s)");
    // The HUD pickup message binds name_label(0xE) = 0x0500002D [D USATxt.dat: "Raptor Magnum"].
    let named = scene.sim.as_ref().unwrap().hud.msgs.iter().any(|m| m.text.contains("Raptor Magnum"));
    assert!(named, "the HUD names the take: {:?}", scene.sim.as_ref().unwrap().hud.msgs.iter().map(|m| &m.text).collect::<Vec<_>>());
}

/// ── the user's symptom, under the play cheats ────────────────────────────────────────────────────
/// god=1 + the recorder's infinite ammo (the user's session): kill the Raptor guards one by one and
/// walk over each drop. The trace: EVERY drop is taken; the first as weapon 0x0E clip 9 (no spare
/// event), the later ones as ammo type 3, each within the game's round range 4..8
/// (`9 · (Rand_FRand(0.5)+0.5)` truncated, one draw per drop [M 0x17a2a0:88-98]), the spare growing
/// only by takes. On the pre-fix port the cheat topped every spare to 200 (above the type-3 cap 36)
/// and topped unowned clips, so the first take leaked the drop's rounds into the spare and every
/// later take refused the cap check — "some later drops can't be picked up".
#[test]
fn with_dev_god_and_infinite_ammo_every_raptor_drop_is_taken_and_the_spare_matches_the_trace() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut dev = cheats();
    let mut scene = load(&iso, "07000001.bin");
    tick_idle(&mut scene, 200, Some(&mut dev));
    let mut guards = raptor_targets(&scene);
    assert!(!guards.is_empty(), "07000001 has Raptor (0xE) guards");
    guards.truncate(5);
    let mut takes: Vec<nf_game::pickup::Taken> = Vec::new();
    // The inventory state right after the FIRST take (the trace's 6.0 s row).
    let mut first_take_state: Option<(bool, i16, i16)> = None;
    let mut killed = 0;
    for &gi in &guards {
        ensure_enabled(&mut scene, gi, Some(&mut dev));
        let taken0 = scene.sim.as_ref().unwrap().drones.taken.len();
        if !kill_guard(&mut scene, gi, Some(&mut dev)) { continue; }
        killed += 1;
        // The game's own touch takes first: a drop landing at Bond's feet is taken by
        // `Pickup_Handler`'s touch (`update_pickups` → `p.touches` → `inventory.take`) and the
        // ledger records it. The head-look merge's corrected head box (the game's own
        // AnimGetCollData 0x1c4ac8 → psiBuildMatrixPalette 0x111498 → HeadTrack 0x1bf618 coupling)
        // changes which death pose drops where, so a drop can land inside Bond's touch bounds
        // during the kill — account for those ledger takes before walking to the rest.
        for (_, t) in &scene.sim.as_ref().unwrap().drones.taken[taken0..] {
            eprintln!("[raptor] take {}: {:?} (the game's touch take)", takes.len() + 1, t);
            takes.push(*t);
            if takes.len() == 1 {
                let sim = scene.sim.as_ref().unwrap();
                first_take_state = Some((sim.drones.inventory.owned[0xe], sim.drones.inventory.clip[0xe], sim.drones.inventory.carried[3]));
            }
        }
        // Take every drop this guard made (the gun; a captain would add grenades).
        loop {
            let Some(k) = scene.sim.as_ref().unwrap().drones.pickups.iter()
                .position(|p| p.from == Some(gi) && !p.gone) else { break };
            assert!(take_drop(&mut scene, k, Some(&mut dev)),
                "drop {k} (weapon {:#x}) is taken — the user's symptom is a drop that cannot be picked up",
                scene.sim.as_ref().unwrap().drones.pickups[k].weapon);
            let sim = scene.sim.as_ref().unwrap();
            let (_, t) = sim.drones.taken.last().expect("a take was just recorded");
            eprintln!("[raptor] take {}: {:?}", sim.drones.taken.len(), t);
            takes.push(*t);
            if takes.len() == 1 {
                first_take_state = Some((sim.drones.inventory.owned[0xe], sim.drones.inventory.clip[0xe], sim.drones.inventory.carried[3]));
            }
        }
    }
    eprintln!("[raptor] killed {killed} Raptor guards; takes: {takes:?}");
    assert!(killed >= 3, "enough guards died to see later drops ({killed})");
    // The first take: the weapon, clip 9, and NO spare leak (the game's equip fills the from-zero
    // clip; the pre-fix port's unowned-clip topping made `old` = 9 and pushed the rounds to spare).
    assert_eq!(takes[0], nf_game::pickup::Taken::Weapon { weapon: 0xe }, "the first take is the gun");
    let (owned, clip, spare_at_first) = first_take_state.expect("the first take was snapshotted");
    assert!(owned && clip == 9, "owned 0x0E, clip 9 right after the first take (got owned {owned}, clip {clip})");
    assert_eq!(spare_at_first, 0, "the first take adds no spare (trace: no type-3 event at 6.0 s)");
    // Every later take: ammo type 3 (the Raptor Magnum's +0x90), within the game's round range.
    assert!(takes.len() >= killed, "every drop taken ({takes:?})");
    for t in &takes[1..] {
        match t { nf_game::pickup::Taken::Ammo { weapon: 0xe, rounds } =>
            assert!((4..=8).contains(rounds), "rounds = clip_size(0xE)=9 · (FRand(0.5)+0.5) truncated ∈ 4..8 [M 0x17a2a0:88-98]: {rounds}"),
            other => panic!("a later Raptor drop is type-3 ammo, got {other:?}") }
    }
    // The spare grows only by the takes and never crosses the cap (the trace: 0→6→13→20→25, cap 36;
    // the pre-fix port sat at 200 → every take refused).
    let sum: i16 = takes[1..].iter().map(|t| match t { nf_game::pickup::Taken::Ammo { rounds, .. } => *rounds, _ => 0 }).sum();
    let sim = scene.sim.as_ref().unwrap();
    assert_eq!(sim.drones.inventory.carried[3], sum, "spare type 3 = the sum of the takes only");
    assert!(sim.drones.inventory.carried[3] <= 36, "below the type-3 cap [D ammo_data cap 36]");
    // Sister: nothing of the killed guards' drops is left on the floor.
    let left: Vec<_> = sim.drones.pickups.iter().filter(|p| p.from.is_some() && !p.gone).collect();
    assert!(left.is_empty(), "every guard drop was taken; left: {:?}", left.iter().map(|p| (p.weapon, p.rounds)).collect::<Vec<_>>());
}


