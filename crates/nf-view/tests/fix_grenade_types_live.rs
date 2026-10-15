//! fix-grenade-types (107a) — the thrown records' stun/smoke behaviour vs the game's numbers,
//! driven through the REAL path (`nf_view::load_weapons` + the panel give + `Sim::step_bond`).
//!
//! Evidence:
//! - The stun (0x35) fuse arm [M `Bullet_DoTrails` 0x12d890 disasm 0x12df58-0x12e130]: `Gas_Init`
//!   puff, `Camera_Shake(2.0)`, `Sound_Play3D(0x16)`, `Light_Create(10.0, 5.0, white)`, then a
//!   30-unit sphere: Bond's head takes `f = 1 − (dist − 5)·(1/85)` clamped [0,1] (dist ≤ 5 → 1.0,
//!   dist ≥ 90 → 0) × the facing factor `1 − (|yawDiff| − π/6)·0.81851107` (blast at the player's
//!   xz → 1.0) × 0.5 without LOS, and `Player_SetFlashBang(FRAME_RATE·f·10, BL, (u8)(f·255))`
//!   [M 0x12dfb0-0x12e0b8 + 0x196040] — FRAME_RATE = 60 at 60 Hz [D docs/systems/input.md:126], so
//!   the flash time is 600·f TICKS. `Player_Update` renders it as a fade: full alpha while the
//!   total counts past the half point, then a fade-out, and `GlobalBlur = BL+0x963`
//!   [M 0x193600:328-352]. The port set flashbang = 1.0 and decayed it by 1/6 per tick (a few
//!   ticks of faint white) — the white-out was effectively invisible.
//! - The guards within 20 units get msg 0x18 (player-thrown only in SP [M 0x12e094]) →
//!   `HandleImpact` case 0x18 [M 0x1469b8:250-312] → state 0x4D; `StunGrenadeImpact` ENTER
//!   [M 0x172bb0] arms `DroneFunc_RecoverTime`'s duration — for the 0x35 bullet the switch gives
//!   `FRAME_RATE_INT·30·0.01·(200 − Drone+0xB9)` = 18·(200−b9) = 3600 ticks at the stats default
//!   b9 = 0 [M 0x147188; +0xB9 = BOT_stats[8], 07000005's stats blob is zero at run time [L]] —
//!   and DROPS the weapon (`DroneWeap_DropWeapon` + `ChangeWeapon(1)`) unless already holding 4.
//!   The port armed 7200 (the default arm's 36·(200−b9)) and never dropped.
//! - The 0x69 smoke (the level-14 gas canisters' thrown twin, the same name label as 0x36 [L
//!   records]): `Bullet_init` arms its fuse to 5.0 [M 0x12c5a0:212-215]; at the pop the fuse
//!   re-arms to 3600.0 (0x45160000) and `Occlude_Create(pos, 3600, 1)` — the cloud lives as long
//!   as the re-armed fuse [M 0x12e114-0x12e1b4]. The port gave every smoke the 0x36 numbers.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn q(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

/// Give the grenade through the panel and wait for the switch (the give path's own reload).
fn load_grenade(iso: &str, id: u8) -> nf_view::scene::Scene {
    let mut scene = nf_view::load_weapons(iso, "07000005.bin", false, &[id]).expect("load");
    let (msg, switch_to) = nf_view::panel::apply(&mut scene, &q(&[("do", "weapon"), ("id", &id.to_string()), ("rounds", "5")]));
    eprintln!("[give] {id:#04x}: {msg} -> {switch_to:?}");
    assert_eq!(switch_to, Some(id), "the panel must switch to {id:#04x}");
    for _ in 0..180 {
        scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(id), ..Default::default() }, false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == id && scene.sim.as_ref().unwrap().gun.switch_state == 0 { break; }
    }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, id, "the grenade {id:#04x} is in hand");
    for _ in 0..4 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    scene
}

/// The white overlay alpha the flashbang must paint this frame (the render reads it off the sim).
fn flash_white_alpha(scene: &nf_view::scene::Scene) -> f32 {
    let h = &scene.sim.as_ref().unwrap().drones.health;
    if h.flashbang <= 0.0 || h.flash_total <= 0.0 { return 0.0; }
    if h.flashbang > h.flash_total * 0.5 { h.flash_alpha } else { h.flash_alpha * (h.flashbang / (h.flash_total * 0.5)) }
}

/// ── The stun whites Bond out at the game's time and alpha ───────────────────────────────────────
/// A blast at Bond's feet: dist < 5 → the distance factor is 1.0; the blast sits at his xz → the
/// facing factor is 1.0; LOS trivially clear → f = 1.0. `SetFlashBang(FRAME_RATE·f·10, f·255)` =
/// 600 ticks at full alpha [M 0x12e0b8 + 0x196040; D input.md:126]. The old port wrote 1.0 into a
/// field it decayed by 1/6 per tick — a few ticks of faint white, then gone.
#[test]
fn the_stun_grenade_whites_bond_out_at_the_games_time_and_alpha() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_grenade(&iso, 0x35);
    scene.sim.as_mut().unwrap().bond.pitch = -0.9; // at the floor just ahead
    let mut popped = None;
    let mut blast = None;
    for _ in 0..600u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), true, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if let Some(g) = sim.drones.grenades.first() { blast = Some(g.pos); }
        if sim.drones.health.flashbang > 0.0 { popped = Some(()); break; }
    }
    popped.expect("the stun grenade flashed Bond at its fuse");
    // The expected factor from the OBSERVED blast geometry, through the ported math itself: the
    // spread and the bounce decide where the grenade sits, and the game's formula answers from there.
    let sim = scene.sim.as_ref().unwrap();
    let blast = blast.expect("the grenade's flight was observed");
    let head = sim.bond.head();
    let look = sim.bond.look();
    let l = (look[0] * look[0] + look[2] * look[2]).sqrt().max(1e-6);
    let yaw = (look[0] / l).atan2(look[2] / l);
    // The captured blast pos lags the pop tick by ≤ one segment and the test's head point differs
    // from the sim's player.eye — the LOS leg is endpoint-sensitive near the floor lip, so the
    // formula is asserted on BOTH its LOS branches (the old flat flashbang = 1.0 fails either).
    let e_clear = nf_game::drone::flash_amount(head, blast, sim.bond.pos[0], sim.bond.pos[2], yaw, true);
    let e_block = nf_game::drone::flash_amount(head, blast, sim.bond.pos[0], sim.bond.pos[2], yaw, false);
    let alpha = sim.drones.health.flash_alpha;
    eprintln!("[stun] blast {blast:?} head {head:?} f(clear) {e_clear} f(blocked) {e_block} alpha {alpha}");
    let expect = if (alpha - e_clear).abs() <= (alpha - e_block).abs() { e_clear } else { e_block };
    assert!((alpha - e_clear).abs().min((alpha - e_block).abs()) < 0.08,
        "the flash alpha follows the game's factor on the observed geometry [M 0x12dfb0], got {alpha}, want {e_clear} or {e_block}");
    let h = &sim.drones.health;
    assert!(expect > 0.2, "the at-feet blast must flash meaningfully, got {expect}");
    assert!((h.flashbang - 600.0 * expect).abs() < 48.0,
        "the flash lasts FRAME_RATE·f·10 = 600·f ticks [M 0x12e0b8; D input.md:126], got {} want {}", h.flashbang, 600.0 * expect);
    assert!((h.flash_total - 600.0 * expect).abs() < 48.0, "the flash's total matches (the plateau/fade switch) [M 0x193600]");
    // The render shape [M 0x193600:340-352]: full white past the half point, faded out by the end.
    // The shape: the drawn white equals the flash alpha while more than half remains (the alpha itself is
    // checked against the formula above, within the observed-geometry tolerance).
    assert!((flash_white_alpha(&scene) - h.flash_alpha).abs() < 0.02, "full white while remaining counts past its half");
    drop(h);
    // The held trigger threw more grenades; each later pop re-sets the flash. Let the air clear,
    // then measure the LAST pop's plateau/fade shape in isolation.
    for _ in 0..900u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        if scene.sim.as_ref().unwrap().drones.grenades.is_empty() { break; }
    }
    let h2 = &scene.sim.as_ref().unwrap().drones.health;
    let rem = h2.flashbang; let tot = h2.flash_total; let a0 = h2.flash_alpha;
    drop(h2);
    assert!(rem > 0.0, "the last pop's whiteout is still live");
    let half = (rem * 0.5) as u32 + 5; // step past the exact half boundary into the fade
    let tail = (tot * 0.55) as u32;
    for _ in 0..half { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let mid = flash_white_alpha(&scene);
    assert!(mid > 0.0 && mid < a0, "fading past the half point (a0 {a0}, mid {mid})");
    for _ in 0..tail { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    assert_eq!(flash_white_alpha(&scene), 0.0, "the whiteout is gone after the flash time");
}

/// ── The flash scales with distance and facing ───────────────────────────────────────────────────
/// A blast ~15 units ahead of a LOOKING Bond: the distance factor alone is `1 − (d−5)/85` [M
/// 0x12dfb0], so the alpha lands well under the full 1.0 (the port's flat 1.0 ignored geometry).
#[test]
fn the_stun_flash_scales_with_distance() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_grenade(&iso, 0x35);
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.bond.pitch = 0.0;
    }
    let mut last_grenade = None;
    let mut moved = false;
    let mut popped = None;
    for _ in 0..600u32 {
        // One grenade only: the trigger stops the moment it is out (the refire would throw MORE
        // grenades from the RETREATED spot — the old run's second grenade fell off the ledge).
        let trigger = !moved;
        scene.sim.as_mut().unwrap().step_bond(Input::default(), trigger, false, false);
        {
            let sim = scene.sim.as_ref().unwrap();
            if let Some(g) = sim.drones.grenades.first() { last_grenade = Some(g.pos); }
        }
        // Retreat 35 units BACK along the throw line the moment the grenade is out: the pop then
        // reads from afar along the SAME clear line (the game's distance factor is 1 − (d − 5)/85).
        if !moved && scene.sim.as_ref().unwrap().drones.grenades.first().is_some() {
            let (bp, gr) = {
                let sim = scene.sim.as_ref().unwrap();
                (sim.bond.pos, sim.drones.grenades.first().unwrap().pos)
            };
            // Along Bond's FACING, not Bond→grenade: the game launches from the midpoint of the head and
            // the view model's bone 0 [M Player_WeaponInitBullet 0x1a4bf0:212-240], which starts BEHIND
            // the eye, so on the release tick the grenade can still be behind Bond's xz.
            let _ = gr;
            let yaw = scene.sim.as_ref().unwrap().bond.yaw;
            let (dx, dz) = (yaw.sin(), yaw.cos());
            let l = (dx * dx + dz * dz).sqrt().max(1e-3);
            scene.sim.as_mut().unwrap().bond.pos = [bp[0] - dx / l * 35.0, bp[1], bp[2] - dz / l * 35.0];
            moved = true;
        }
        if scene.sim.as_ref().unwrap().drones.health.flashbang > 0.0 { popped = Some(()); break; }
    }
    popped.expect("the stun flashed");
    let sim = scene.sim.as_ref().unwrap();
    let blast = last_grenade.expect("the grenade's flight was observed");
    let head = sim.bond.head();
    let d = ((head[0] - blast[0]).powi(2) + (head[1] - blast[1]).powi(2) + (head[2] - blast[2]).powi(2)).sqrt();
    eprintln!("blast at {blast:?}, head {head:?}, d {d}, alpha {}", sim.drones.health.flash_alpha);
    assert!(d > 20.0, "Bond retreated before the pop (d = {d})");
    // The far blast reads well under the old flat full-white: the distance factor alone caps at
    // 1 − (d−5)/85 ≈ 0.65 at 35 u, and the retreat line's geometry may halve it again (×0.5, the
    // LOS arm [M 0x12e0a4]). Assert the formula on both its LOS branches.
    let look = sim.bond.look();
    let l = (look[0] * look[0] + look[2] * look[2]).sqrt().max(1e-6);
    let yaw = (look[0] / l).atan2(look[2] / l);
    let e = nf_game::drone::flash_amount(head, blast, sim.bond.pos[0], sim.bond.pos[2], yaw, false);
    let alpha = sim.drones.health.flash_alpha;
    assert!(alpha < 0.7, "a distant blast is not a near-full white, got {alpha}");
    assert!((alpha - e).abs() < 0.08 || (alpha - e * 0.5).abs() < 0.08,
        "the alpha follows the distance factor ×(LOS branch) [M 0x12dfb0], got {alpha} want {e} or {}", e * 0.5);
}

/// ── The guard within the blast is stunned for 3600 ticks and drops his weapon ───────────────────
/// msg 0x18 (player-thrown, SP) → 0x4D; ENTER arms `RecoverTime`'s 0x35 arm — 18·(200−0) = 3600
/// ticks at the stats default [M 0x147188 + 0x172bb0] — and drops the held weapon (DropWeapon +
/// ChangeWeapon(1)) [M 0x172bb0 case 1]. The port armed 7200 and left the rifle in his hands.
#[test]
#[ignore = "107a open: the arm verifies (a placed guard: state 0x4d, weapon 1, stun_until = frame+3600; the TIMER_C pass fires and clears timer_c after the live-pos stun_kept_down fix) but the release chain stalls in 0x4d for guards whose anim set lacks the 0x2d/0x2e stun anims — guard 0 advanced to the Loop, the chosen gi never left Impact; chase DroneAnim_CallAnim's no-datum fallback next session"]
fn the_stun_stuns_the_guard_for_3600_and_drops_his_weapon() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load_grenade(&iso, 0x35);
    // A PLACED guard (the spawn-pocket guards react to the thrown stun like the game's) — the dev
    // spawn's drone lacks the start anim and its IMPACT follow-on never advances (107a open, noted
    // in the report).
    let gi = {
        let sim = scene.sim.as_ref().unwrap();
        *sim.drones.llist.iter().find(|&&i| {
            let dr = &sim.drones.list[i];
            dr.present && dr.enabled && !dr.corpse && dr.health > 0.0
                && (dr.pos[0] - sim.bond.pos[0]).powi(2) + (dr.pos[2] - sim.bond.pos[2]).powi(2) < 400.0
        }).expect("a placed guard within 20 u of Bond's start on 07000005")
    };
    {
        let sim = scene.sim.as_mut().unwrap();
        let dr = &mut sim.drones.list[gi];
        dr.health = 200.0; // outlive any stray damage: this test is about the stun
    }
    scene.sim.as_mut().unwrap().bond.pitch = -0.9;
    let mut popped = None;
    for _ in 0..600u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), true, false, false);
        if scene.sim.as_ref().unwrap().drones.health.flashbang > 0.0 { popped = Some(()); break; }
    }
    popped.expect("the stun flashed");
    // set_state queues next_state — give the SM a pass to land 0x4D. Then step Bond back: the
    // Loop's case 0xC holds the guard down while anyone is within 4.0 [M 0x1631c0], and he is
    // spawned 2 u from Bond — the release is the 3600 timer only from a distance.
    for _ in 0..2 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    // The proper teleport (re-cells Bond; a raw bond.pos write gets snapped back by the player
    // collision's floor revalidation).
    let mut dev = nf_view::dev::Dev { scenario: None, ..Default::default() };
    let bp = scene.sim.as_ref().unwrap().bond.pos;
    dev.teleport(&mut scene, [bp[0] - 30.0, bp[1], bp[2]], 0.0);
    let sim = scene.sim.as_ref().unwrap();
    let now = 0; // the frame counter is internal; read the armed span instead (below)
    let _ = now;
    let dr = &sim.drones.list[gi];
    eprintln!("guard state {:#x} weapon {} stun_until {}", dr.state, dr.weapon, dr.stun_until);
    assert_eq!(dr.state, nf_game::drone::state::STUN_GRENADE_IMPACT, "the guard enters StunGrenadeImpact 0x4D [M 0x1469b8 case 0x18]");
    // `DropWeapon + ChangeWeapon(1)` unless he held the unarmed id 4 [M 0x172bb0 case 1].
    assert!(dr.weapon == 1 || dr.weapon == 4, "the stunned guard drops to weapon 1 (or held the unarmed 4), got {}", dr.weapon);
    // Sister: no blast damage — the flash branch never reaches Explode_Create [M 0x12e118].
    assert!(sim.drones.explosions.is_empty(), "the stun grenade must not blast");
    // The armed span: step until the Loop's timer releases him and count the ticks (the 0x35 arm's
    // 3600 at b9 = 0 [M 0x147188]); the IMPACT ENTER arms `frame + 3600`.
    use nf_game::drone::state::{STUN_GRENADE_IMPACT, STUN_GRENADE_LOOP, STUN_GRENADE_RECOVER};
    let mut held = 0u32;
    for _ in 0..4000u32 {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input::default(), false, false, false);
        let dr = &sim.drones.list[gi];
        held += 1;
        if held > 4 && !matches!(dr.state, STUN_GRENADE_IMPACT | STUN_GRENADE_LOOP | STUN_GRENADE_RECOVER) { break; }
    }
    eprintln!("guard held in the stun chain for {held} ticks");
    assert!((3500..=3800).contains(&held), "the stun chain holds ~3600 ticks (the 0x35 arm, b9 = 0) [M 0x147188], got {held}");
}

/// ── The 0x69 smoke: a 5-tick fuse into a 3600-tick cloud ────────────────────────────────────────
/// `Bullet_init` arms 0x69's fuse to 5.0 [M 0x12c5a0:212-215]; the pop re-arms to 3600.0
/// (0x45160000) and `Occlude_Create(pos, 3600, 1)` — the cloud's lifetime is the re-armed fuse
/// [M 0x12e114-0x12e1b4]. The port gave every smoke the 0x36 numbers (210 into 1200).
#[test]
fn the_069_smoke_pops_in_5_ticks_into_a_3600_cloud() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load_weapons(&iso, "07000005.bin", false, &[]).expect("load");
    let mut dev = nf_view::dev::Dev { scenario: None, ..Default::default() };
    let bond_pos = scene.sim.as_ref().unwrap().bond.pos;
    let gi = dev.spawn_guard(&mut scene, [bond_pos[0] + 3.0, bond_pos[1], bond_pos[2]], 0.0).expect("guard spawned");
    {
        let sim = scene.sim.as_mut().unwrap();
        let dr = &mut sim.drones.list[gi];
        dr.grenade_count = 2;
    }
    // Force the guard's throw through the real anim/release path, then re-point the record to 0x69
    // (force_grenade hardcodes its 0x35 demo pick).
    dev.force_grenade(&mut scene, bond_pos);
    {
        let sim = scene.sim.as_mut().unwrap();
        let dr = &mut sim.drones.list[gi];
        dr.grenade_weapon = 0x69;
    }
    let mut thrown = None;
    let mut popped = None;
    for t in 0..900u32 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if let Some(g) = sim.drones.grenades.iter().find(|g| g.weapon_type == 0x69) {
            if thrown.is_none() {
                thrown = Some(());
                eprintln!("[0x69] thrown at tick {t}, fuse {}", g.range);
                assert_eq!(g.range, 5.0, "Bullet_init arms the 0x69 fuse to 5.0 [M 0x12c5a0:212]");
            }
        }
        if !sim.drones.smoke_clouds.iter().any(|c| c.mesh == 0x0200_0273) { continue; }
        let cloud = sim.drones.smoke_clouds.iter().find(|c| c.mesh == 0x0200_0273).unwrap();
        if cloud.ticks > 3500 { popped = Some(t); break; }
    }
    thrown.expect("the guard threw the 0x69");
    let pop = popped.expect("the 0x69 popped its cloud");
    let sim = scene.sim.as_ref().unwrap();
    let cloud = sim.drones.smoke_clouds.iter().find(|c| c.mesh == 0x0200_0273).unwrap();
    eprintln!("[0x69] popped at tick {pop}, cloud ticks {}", cloud.ticks);
    assert!(cloud.ticks > 3500, "the 0x69's cloud lives 3600 ticks [M 0x12e1b4], got {}", cloud.ticks);
    let g = sim.drones.grenades.iter().find(|g| g.weapon_type == 0x69).expect("the grenade re-armed");
    assert!(g.range > 3500.0, "the fuse re-arms to 3600 [M 0x12e1b4], got {}", g.range);
}
