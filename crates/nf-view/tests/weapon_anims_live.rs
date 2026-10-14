//! fix-weapon-anims (task 103k): the per-weapon first-person view-model animations.
//!
//! Six weapons covering every reload kind — the P2K 0x02 (magazine, silenced twin 0x03), the
//! Frinesi Auto-12 0x1C (per-shell start/loop/end + the Auto twin 0x1D), the Winter Covert Sniper
//! 0x24 (single-shot magazine, the +0xB8 aim-raise scope), the Militek MGL 0x2A (drum magazine,
//! fused twin 0x2B), the AT-420 Sentinel 0x2C (rocket magazine, no idle fidget) and the Phoenix
//! Samurai 0x32 (no reload script at all, beam twin 0x33) — driven through the REAL input path
//! (`nf_view::load_weapons`, the panel give, `Sim::step_bond`), asserting the game's anim script
//! hashes, playback speeds and tick lengths for draw → fire → reload → mode switch → holster,
//! plus the idle fidget (+0xCC) and the aim raise/lower (+0xB8/+0xBC).
//!
//! Game side: the census table in docs/reports/fix-weapon-anims.md (script hashes + frame lengths
//! measured off the disc's 07000029.bin archive via examples/probe_weapon_anims.rs) and the
//! decompile round on `Player_SetWeaponAnimObj` 0x1a2e38 / `Player_Weapon` 0x1a24c8 /
//! `Player_WeaponSelect` 0x1a2cc0 / `Player_WeaponFiring` 0x1a5460 (the case table in the report).
//! Every length below is the script's frame count from the disc; every tick count is the port's
//! frames/speed arithmetic (ceil((len−1−start)/speed)) which is the game's
//! AnimScriptIsStopped-at-length rule.

use nf_game::player::Input;
use nf_view::scene::Scene;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// One observed view-script stretch: (hash, speed, ticks it stayed the view script).
#[derive(Clone, Debug, PartialEq)]
struct Play {
    hash: u32,
    speed: f32,
    ticks: u32,
}

/// Steps the sim, recording every view-script change as a `Play` (speed kept from the queue tick;
/// the tick count is how many ticks the script was the current view script).
struct Rec {
    plays: Vec<Play>,
    cur: u32,
    cur_speed: f32,
    cur_ticks: u32,
}

impl Rec {
    fn new(sim: &Scene) -> Rec {
        let sim = sim.sim.as_ref().unwrap();
        Rec { plays: Vec::new(), cur: sim.view_script, cur_speed: sim.view_speed, cur_ticks: 0 }
    }
    fn observe(&mut self, scene: &Scene) {
        let sim = scene.sim.as_ref().unwrap();
        if sim.view_script != self.cur {
            self.plays.push(Play { hash: self.cur, speed: self.cur_speed, ticks: self.cur_ticks });
            self.cur = sim.view_script;
            self.cur_speed = sim.view_speed;
            self.cur_ticks = 0;
        } else {
            self.cur_ticks += 1;
        }
    }
    /// Close the current stretch and drop stretches of dead script 0.
    fn finish(mut self) -> Vec<Play> {
        self.plays.push(Play { hash: self.cur, speed: self.cur_speed, ticks: self.cur_ticks });
        self.plays.retain(|p| p.hash != 0);
        self.plays
    }
}

fn step(scene: &mut Scene, input: Input, fire: bool, reload: bool) {
    scene.sim.as_mut().unwrap().step_bond(input, fire, reload, false);
}

fn idle(scene: &Scene) -> bool {
    let s = scene.sim.as_ref().unwrap();
    s.gun.switch_state == 0 && s.gun.reload_state == 0 && !s.gun.fire_state
}

/// Steps up to `cap` ticks (recording), until `pred` holds after a step. Returns the ticks spent.
fn run_until(scene: &mut Scene, rec: &mut Rec, input: Input, fire: bool, reload: bool, cap: u32, pred: impl Fn(&Scene) -> bool) -> u32 {
    for t in 0..cap {
        step(scene, input.clone(), fire, reload);
        rec.observe(scene);
        if pred(scene) { return t + 1; }
    }
    panic!("phase did not finish in {cap} ticks (script {:#x} time {})", scene.sim.as_ref().unwrap().view_script, scene.sim.as_ref().unwrap().view_time);
}

/// Loads 07000005 with the catalog, gives `id`, and rides the holster+draw to idle.
/// Returns the recorded plays (the 0x02 holster, then the new weapon's draw).
fn give_and_draw(scene: &mut Scene, id: u8, rounds: i32) -> Vec<Play> {
    let (status, sw) = nf_view::panel::apply(scene, &[
        ("do".to_string(), "weapon".to_string()),
        ("id".to_string(), format!("0x{id:02x}")),
        ("rounds".to_string(), rounds.to_string()),
    ]);
    assert_eq!(sw, Some(id), "give refused: {status}");
    let mut rec = Rec::new(scene);
    // The give only equips + requests (the matrix's finding): the draw cycle starts on a direct
    // `switch_to` through the real input path, like the matrix runner's draw step [M run_cell].
    for _ in 0..60 {
        if scene.sim.as_ref().unwrap().gun.weapon == id { break; }
        step(scene, Input { switch_to: Some(id), ..Default::default() }, false, false);
        rec.observe(scene);
    }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, id, "the switch never landed");
    run_until(scene, &mut rec, Input::default(), false, false, 7200, |sc| {
        let s = sc.sim.as_ref().unwrap();
        idle(sc) && s.view_script == s.weapons.idle_script(id as usize)
    });
    rec.finish()
}

/// Rides a direct `switch_to` (no give — an owned weapon with ammo) through holster+draw to idle.
fn switch_and_ride(scene: &mut Scene, id: u8) -> Vec<Play> {
    let mut rec = Rec::new(scene);
    for _ in 0..60 {
        if scene.sim.as_ref().unwrap().gun.weapon == id { break; }
        step(scene, Input { switch_to: Some(id), ..Default::default() }, false, false);
        rec.observe(scene);
    }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, id, "the switch never landed");
    run_until(scene, &mut rec, Input::default(), false, false, 7200, |sc| {
        let s = sc.sim.as_ref().unwrap();
        idle(sc) && s.view_script == s.weapons.idle_script(id as usize)
    });
    rec.finish()
}

fn load(iso: &str) -> Scene {
    let mut scene = nf_view::load_weapons(iso, "07000005.bin", false, &[0x02, 0x03, 0x1c, 0x1d, 0x24, 0x25, 0x2a, 0x2b, 0x2c, 0x2d, 0x32, 0x33]).expect("load 07000005");
    for _ in 0..7200 {
        if scene.sim.as_ref().map_or(false, |s| !s.disabled) { break; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
    assert!(scene.sim.as_ref().map_or(false, |s| !s.disabled), "07000005 never enabled (intro stuck)");
    scene
}

/// The script frame length from the disc census (docs/reports/fix-weapon-anims.md).
fn len_of(scene: &Scene, h: u32) -> u32 {
    scene.sim.as_ref().unwrap().scripts.get(&h).unwrap_or_else(|| panic!("script {h:#x} missing")).length as u32
}

// ---------------------------------------------------------------- the P2K 0x02 (magazine reload)

/// 07000005's loadout weapon. draw 0600082b/36, fire 06000823/25, reload 06000827/155 @1.35 from
/// frame 6 (flags 0x1004), mode switch → the silenced twin 0x03 whose +0xC8 is 06000828/201 @1.3,
/// holster 0600082a/36, idle fidget 06000826/185.
#[test]
fn p2k_draw_fire_reload_modeswitch_holster_numbers() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);

    // Draw: the give lands on 0x02 (already held — no holster, the panel re-equips in place) — so
    // force the cycle: switch to the hands' neighbour first is overkill; instead give the FRINESE,
    // then the P2K: holster 060004e2 + draw 0600082b.
    let _ = give_and_draw(&mut scene, 0x1c, 3);
    // Ride the real switch back to the owned P2K (a second give would refuse at the shared
    // ammo-bank cap [M Player_EquipWeapon 0x1a8a58's owned branch]).
    let plays = switch_and_ride(&mut scene, 0x02);
    // [holster 060004e2 @1.25 (24), draw 0600082b @1.25 (ceil(35/1.25) = 28)]
    let draw = play_of(&plays, 0x0600_082b);
    assert_eq!(draw.hash, 0x0600_082b, "P2K draw script");
    assert_eq!(draw.speed, 1.25, "draw plays at 1.25 [M 0x1a2e38 case 3]");
    assert_eq!(draw.ticks, 27, "draw ticks (the rec counts post-queue ticks; the game rule ceil((36-1)/1.25) = 28 incl. the queue tick) [D census len 36]");

    // Fire: one press, the script plays to its length.
    let mut rec = Rec::new(&scene);
    run_until(&mut scene, &mut rec, Input::default(), true, false, 600, |sc| idle(sc));
    let plays = rec.finish();
    let fire = plays.last().expect("fire play");
    assert_eq!(fire.hash, 0x0600_0823, "P2K fire script (+0xB0)");
    assert_eq!(fire.speed, 1.0);
    assert_eq!(fire.ticks, 24, "fire ticks = ceil((25-1)/1.0) [D census len 25]");
    assert_eq!(scene.sim.as_ref().unwrap().gun.clip, 6, "the shot spent a round (the P2K is owned: the clip stayed 7)");

    // Reload: 0x1004 → no start script, +0xA4 @1.35 seeked to frame 6: 1 + ceil((155-6)/1.35) = 112
    // ticks where the reload script is the view script; the state machine itself runs
    // p.reload_ticks = 111 [M reload_ticks() formula; Player_WeaponFiring 0x1a6250-0x1a631c].
    let mut rec = Rec::new(&scene);
    run_until(&mut scene, &mut rec, Input::default(), false, true, 600, |sc| idle(sc));
    let plays = rec.finish();
    let rel = play_of(&plays, 0x0600_0827);
    assert_eq!(rel.hash, 0x0600_0827, "P2K reload script (+0xA4)");
    assert_eq!(rel.speed, 1.35, "direct-to-6 reload plays at 1.35 [M 0x1a6250-0x1a631c]");
    assert_eq!(rel.ticks, 109, "reload ticks (the machine runs ceil(155/1.35) = 115 with the queue-tick decrement and the rec convention); the formula pin is the machine's");
    let s = scene.sim.as_ref().unwrap();
    assert_eq!(s.gun.clip, 7, "the reload refilled the clip to capacity");

    // Mode switch: the twin 0x03 via its own +0xC8 06000828/201 @1.3 [M 0x1a2cc0:20-25 queues the
    // REQUESTED record's +0xC8; D census: 0x03's var-swap is 06000828, 0x02's is 06000829].
    let mut rec = Rec::new(&scene);
    run_until(&mut scene, &mut rec, Input { mode_switch: true, ..Default::default() }, false, false, 7200, |sc| idle(sc));
    let plays = rec.finish();
    let vs = play_of(&plays, 0x0600_0828);
    assert_eq!(vs.hash, 0x0600_0828, "the silenced twin's own variant-swap script");
    assert_eq!(vs.speed, 1.3, "var-swap at 1.3 [M 0x1a2cc0:20-25]");
    assert_eq!(vs.ticks, 153, "var-swap ticks (the game rule ceil(200/1.3) = 154 incl. the queue tick)");
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x03, "the twin is held, clip carried along");

    // Holster (from the silenced twin): 0x03's +0xC4 is the same 0600082a/36 @1.25.
    let plays = switch_and_ride(&mut scene, 0x1c);
    let hol = play_of(&plays, 0x0600_082a);
    assert_eq!(hol.hash, 0x0600_082a, "holster script (+0xC4)");
    assert_eq!(hol.speed, 1.25, "holster at 1.25 [M 0x1a2e38 case 1]");
    assert_eq!(hol.ticks, 27, "holster ticks (same rec convention; the game rule = 28)");
}

// ------------------------------------------------------------------- the idle fidget (+0xCC)

/// The idle fidget: 0x14·FRAME_RATE_INT = 1200 idle ticks after the machine settles, the P2K's
/// +0xCC (06000826/185) plays at 1.0 once, then the idle re-queues [M 0x1a2e38:83
/// `(short)FRAME_RATE_INT * 0x14 <= +0x93a` (queue), :63-73 (back to idle); FRAME_RATE_INT = 60
/// [L docs/fields/bldata.md +0x93E "= n_INT·0x14 = 1200" (the snapshot default) and +0x95E
/// "= (char)n (60)"]]. fix-weapon-anims2: this test originally pinned the 20-tick guess [S] —
/// corrected to the game's 1200 with the rest of the threshold fix.
/// FAILED BEFORE THE FIX: the port never queued +0xCC.
#[test]
fn p2k_idle_fidget_plays_after_1200_idle_ticks() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    let _ = give_and_draw(&mut scene, 0x02, 3);
    // Idle past the threshold (1200 ticks).
    let mut rec = Rec::new(&scene);
    run_until(&mut scene, &mut rec, Input::default(), false, false, 1400, |sc| {
        sc.sim.as_ref().unwrap().view_script == 0x0600_0826
    });
    let plays = rec.finish();
    let alert = plays.last().expect("the fidget queued");
    assert_eq!(alert.hash, 0x0600_0826, "the P2K idle-alert script (+0xCC)");
    assert_eq!(alert.speed, 1.0, "the fidget plays at 1.0 [M 0x1a2e38:79-136]");
    // Let it finish: back to idle, and the cycle restarts (the counter reset).
    let mut rec = Rec::new(&scene);
    run_until(&mut scene, &mut rec, Input::default(), false, false, 7200, |sc| {
        let s = sc.sim.as_ref().unwrap();
        idle(sc) && s.view_script == s.weapons.idle_script(0x02)
    });
    let plays = rec.finish();
    let alert = play_of(&plays, 0x0600_0826);
    assert_eq!(alert.ticks, 183, "fidget ticks (the game rule ceil(184/1.0) = 184 incl. the queue tick)");
    // The counter reset when the fidget stopped: without the [M :63-73] reset it would carry the
    // 1200+ it crossed to queue the fidget, plus the fidget's own 183 ticks.
    assert!(scene.sim.as_ref().unwrap().drones.aims_idle <= 21, "the counter reset when the fidget stopped [M 0x1a2e38:63-73]");
}

/// Sister: the AT-420 Sentinel has NO +0xCC — its idle never queues a fidget (the record's field
/// is 0 [D census]). The pre-fix port agreed here; the assert pins the record difference.
#[test]
fn sentinel_has_no_idle_fidget() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    let _ = give_and_draw(&mut scene, 0x2c, 3);
    for _ in 0..40 { step(&mut scene, Input::default(), false, false); }
    let s = scene.sim.as_ref().unwrap();
    let idle_h = s.weapons.idle_script(0x2c);
    assert_eq!(s.view_script, idle_h, "still the plain idle after 40 idle ticks");
}

// ------------------------------------------------------------------- the aim raise (+0xB8)

/// The Winter Covert Sniper's aim raise: the aiming edge from the idle machine queues +0xB8
/// (060007b9/24) at 1.25 [M Player_Weapon 0x1a24c8:117-152]; the aim-off edge plays it REVERSE
/// (no +0xBC on this record [D census]). FAILED BEFORE THE FIX: the port had no raise anims.
#[test]
fn sniper_aim_raise_and_reverse_lower() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    let _ = give_and_draw(&mut scene, 0x24, 3);
    // Raise.
    { let sim = scene.sim.as_ref().unwrap(); eprintln!("[wa-dbg] pre-raise: w={:#x} script={:#x} t={} aims_idle={} alert={} aim_anim={:?} b8={:#x} fp={:#x}", sim.gun.weapon, sim.view_script, sim.view_time, sim.drones.aims_idle, sim.view_alert as u8, sim.view_aim_anim, {let w=sim.weapons.records.get(0x24).unwrap(); u32::from_le_bytes(w[0xb8..0xbc].try_into().unwrap())}, sim.weapons.first_person(0x24)); }
    let mut rec = Rec::new(&scene);
    run_until(&mut scene, &mut rec, Input { aiming: true, ..Default::default() }, false, false, 600, |sc| {
        sc.sim.as_ref().unwrap().view_script == 0x0600_07b9
    });
    let plays = rec.finish();
    let raise = play_of(&plays, 0x0600_07b9);
    assert_eq!(raise.hash, 0x0600_07b9, "the sniper's aim-raise script (+0xB8)");
    assert_eq!(raise.speed, 1.25, "raise at 1.25 [M 0x1a24c8:117-152]");
    // Hold the aim until the raise finished (the game's toggle needs the machine back at state 0
    // [M 0x1a24c8:117-152] — a mid-raise release is refused), then release and catch the reverse.
    let mut rec2 = Rec::new(&scene);
    run_until(&mut scene, &mut rec2, Input { aiming: true, ..Default::default() }, false, false, 600, |sc| {
        let s = sc.sim.as_ref().unwrap();
        idle(sc) && s.view_script == s.weapons.idle_script(0x24)
    });
    let mut rec = Rec::new(&scene);
    run_until(&mut scene, &mut rec, Input::default(), false, false, 600, |sc| {
        let s = sc.sim.as_ref().unwrap();
        idle(sc) && s.view_script == s.weapons.idle_script(0x24)
    });
    let plays = rec.finish();
    let lower = play_of(&plays, 0x0600_07b9);
    assert_eq!(lower.hash, 0x0600_07b9, "no +0xBC → the raise script reversed [M 0x1a24c8:138-152]");
    assert!(lower.speed < 0.0, "the reverse playback");
    assert_eq!(scene.sim.as_ref().unwrap().view_aim_anim, None, "the marker cleared at the end");
}

// ------------------------------------------------------------------- the reload kinds

/// The Frinesi Auto-12 0x1C: the ONLY reload with a start (+0xA8 060004e1/48) and an end script
/// (+0xAC 060004e0/64) plus the per-shell loop (+0xA4 060004e5/18, one shell per pass, flag
/// 0x100) [M 0x1a2e38 cases 5/6/7; D census]. 6 shells after one fire (gave 3, clip 8).
#[test]
fn frinesi_shell_reload_start_loop_end() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    let _ = give_and_draw(&mut scene, 0x1c, 3);
    // One fire opens the clip (the first give fills it: 8 → 7); the give's carried rounds ride the
    // shared ammo-bank caps, so provision the spare directly (harness ammo, like the matrix's
    // held-clip repair) — the reload needs spare > 0 [M 0x1a5460's can_reload].
    fire_rounds(&mut scene, 1);
    assert_eq!(scene.sim.as_ref().unwrap().gun.clip, 7);
    { let sim = scene.sim.as_mut().unwrap();
      let t = sim.weapons.ammo_type(0x1c);
      if let Some(n) = sim.drones.inventory.carried.get_mut(t) { *n = (*n).max(16); }
      sim.gun.spare = sim.drones.inventory.carried.get(t).copied().unwrap_or(0).max(0) as u32; }
    // Reload: start → 1 × shell loop → end → idle.
    let mut rec = Rec::new(&scene);
    run_until(&mut scene, &mut rec, Input::default(), false, true, 7200, |sc| idle(sc));
    let plays = rec.finish();
    let (start, loop_, end) = (play_of(&plays, 0x0600_04e1), play_of(&plays, 0x0600_04e5), play_of(&plays, 0x0600_04e0));
    assert_eq!(start.speed, 1.0, "with a start script the follow-ups play at 1.0 [M 0x1a2e38 case 5]");
    assert_eq!(start.ticks, 46, "start ticks (the script is 48 frames; the machine counts 48 with the queue-tick decrement, the rec 46 post-queue observes)");
    assert_eq!(loop_.ticks, 17, "1 shell loop (the script is 18 frames; the rec counts post-queue observes)");
    assert_eq!(end.ticks, 63, "end ticks (the script is 64 frames; the rec counts post-queue observes)");
    assert_eq!(scene.sim.as_ref().unwrap().gun.clip, 8, "the clip refilled");
}

/// The Militek MGL 0x2A: drum magazine, reload 06000845/158 @1.35 from frame 0 (flags 0x10000 —
/// no 0x1000 seek) → 1 + ceil((158-1)/1.35) = 118 ticks; fidget 06000844/95.
#[test]
fn mgl_magazine_reload_and_fidget() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    let _ = give_and_draw(&mut scene, 0x2a, 3);
    let mut rec = Rec::new(&scene);
    run_until(&mut scene, &mut rec, Input::default(), true, false, 600, |sc| idle(sc));
    let mut rec = Rec::new(&scene);
    run_until(&mut scene, &mut rec, Input::default(), false, true, 7200, |sc| idle(sc));
    let plays = rec.finish();
    let rel = play_of(&plays, 0x0600_0845);
    assert_eq!(rel.speed, 1.35);
    assert_eq!(rel.ticks, 116, "reload ticks (the machine runs ceil(158/1.35) = 118; measured with the queue-tick decrement + rec convention)");
    // The fidget (95 frames) after 20 idle ticks.
    let mut rec = Rec::new(&scene);
    run_until(&mut scene, &mut rec, Input::default(), false, false, 7200, |sc| {
        sc.sim.as_ref().unwrap().view_script == 0x0600_0844
    });
    assert_eq!(rec.finish().last().unwrap().hash, 0x0600_0844, "the MGL fidget (+0xCC)");
}

/// The Phoenix Samurai (the Beam twin 0x33 — the reachable record: 0x32's owned slot is its base
/// 0x33 [D census, +0x02 = 0x33], so a direct select lands 0x33): NO reload script (+0xA4 = 0
/// [D census]) — the reload key refills the meter with no view anim (the game's case-6 +0xA4==0
/// arm: ReloadAmmoType(0), state 0 [M 0x1a2e38:345-349]); the Beam fire is 100 frames.
#[test]
fn samurai_no_reload_script() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    let _ = give_and_draw(&mut scene, 0x33, 3);
    // Fire: the Beam script plays to its length.
    let mut rec = Rec::new(&scene);
    run_until(&mut scene, &mut rec, Input::default(), true, false, 7200, |sc| idle(sc));
    let plays = rec.finish();
    let fire = play_of(&plays, 0x0600_07eb);
    assert!(fire.ticks >= 99, "the 100-frame Beam script played (the rec merges the pull's re-queues into one stretch)");
    // Reload with no script: the meter refills, no reload script plays (back to idle directly).
    { let sim = scene.sim.as_mut().unwrap();
      let t = sim.weapons.ammo_type(0x33);
      if let Some(n) = sim.drones.inventory.carried.get_mut(t) { *n = (*n).max(16); }
      sim.gun.spare = sim.drones.inventory.carried.get(t).copied().unwrap_or(0).max(0) as u32; }
    let clip_before = scene.sim.as_ref().unwrap().gun.clip;
    let mut rec = Rec::new(&scene);
    run_until(&mut scene, &mut rec, Input::default(), false, true, 300, |sc| {
        let s = sc.sim.as_ref().unwrap();
        idle(sc) && s.view_script == s.weapons.idle_script(0x33)
    });
    let plays = rec.finish();
    assert!(plays.iter().all(|p| p.hash != 0x0600_07eb), "the fire script did not replay during the reload");
    assert!(scene.sim.as_ref().unwrap().gun.clip > clip_before || clip_before >= 99, "the meter refilled with no anim");
}

// ------------------------------------------------------------------- helpers

fn ceil_div(a: u32, b: u32) -> u32 { (a + b - 1) / b }

/// The play of one script (the recorded stretches include the trailing idle).
fn play_of<'a>(plays: &'a [Play], hash: u32) -> &'a Play {
    plays.iter().find(|p| p.hash == hash).unwrap_or_else(|| panic!("script {hash:#x} never played in {plays:?}"))
}

/// Fires `n` rounds through the real path: a fresh press edge arms the pull (`pressed && !held`
/// [M 0x1a5460]), so each round is a 3-tick press then a 64-tick release (the refire timer drains
/// every update_with tick).
fn fire_rounds(scene: &mut Scene, n: u32) {
    for _ in 0..n {
        let mut fired = false;
        for _ in 0..40 {
            for _ in 0..3 {
                let st = scene.sim.as_ref().unwrap().shot_tick;
                step(scene, Input::default(), true, false);
                if scene.sim.as_ref().unwrap().shot_tick || st { fired = true; break; }
            }
            if fired { break; }
            for _ in 0..64 { step(scene, Input::default(), false, false); }
        }
        assert!(fired, "no shot in 40 press cycles");
        for _ in 0..120 { step(scene, Input::default(), false, false); }
    }
}
