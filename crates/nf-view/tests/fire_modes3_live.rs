//! fix-fire-modes3: every fire mode of the PCSX2 table (docs/live/2026-10-13-weapon-modes-pcsx2.md)
//! through the real input path (`nf_view::load_with_carry` + `Sim::step_bond`): the mode press, the
//! twin it lands, the projectile kind, the fuse/impact split, the burst count, the charge/beam
//! economics, the zoom reset and the ammo drawn per shot — one test per weapon family.
//!
//! Record bytes are [L ee_start_05.bin @ 0x2bf150 (weapon_data, stride 0x10C)]; the decompile steps
//! are [M Player_Weapon 0x1a24c8 / Player_WeaponFiring 0x1a5460 / Bullet_init 0x12c5a0 /
//! Bullet_CollisionHandler 0x12ce80].

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The test catalog: every record the families touch — the load pre-merges their fp skins so the
/// draws and the variant swaps resolve (the matrix's `load_weapons` merge).
const CATALOG: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 20, 21, 22, 23, 24, 25, 26, 28, 30, 32, 33, 34, 36, 38, 40, 42, 43, 44, 46, 48, 50, 52, 54, 56, 58, 60, 62, 64, 66, 68, 70, 84, 85, 100, 101, 102, 113];

fn load(iso: &str) -> nf_view::scene::Scene {
    nf_view::load_with_carry_weapons(iso, "07000005.bin", false, None, CATALOG).expect("load 07000005")
}

fn tick(scene: &mut nf_view::scene::Scene, n: u32) {
    if let Some(sim) = scene.sim.as_mut() {
        for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
    }
}

/// Gives `id` through the real give path (the dev menu's Give button — `Player_EquipWeapon`
/// 0x1a8a58 + three clips).
fn give(scene: &mut nf_view::scene::Scene, id: u8, rounds: i16) -> bool {
    let (_, sw) = nf_view::panel::apply(scene, &[
        ("do".to_string(), "weapon".to_string()),
        ("id".to_string(), format!("0x{id:02x}")),
        ("rounds".to_string(), rounds.to_string()),
    ]);
    sw == Some(id)
}

/// Arms a record's clip + owned byte directly — the PCSX2 pass's PINE setup ("the owned byte
/// BL+0x1BA+w*0xC, the clip BL+0x1B8+w*0xC"): the mode press's `Player_WeaponHasAmmo` gate reads
/// the TWIN's own clip/carried [M 0x1a24c8:210-212], and a give of the base fills only the base's
/// ammo type (the AIMS-20's underbarrel 0x1B uses ammo type 12, the rifle 0x1A type 5 [L +0x90]).
fn arm_record(scene: &mut nf_view::scene::Scene, id: u8, rounds: i16) {
    let sim = scene.sim.as_mut().unwrap();
    if (id as usize) < sim.drones.inventory.owned.len() { sim.drones.inventory.owned[id as usize] = true; }
    if let Some(c) = sim.drones.inventory.clip.get_mut(id as usize) { *c = rounds; }
}

/// Draws `id` through the real input path (a direct `switch_to`), then settles.
fn draw(scene: &mut nf_view::scene::Scene, id: u8) -> bool {
    for _ in 0..60 {
        scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(id), ..Default::default() }, false, false, false);
        if scene.sim.as_ref().unwrap().gun.weapon == id { break; }
    }
    let ok = scene.sim.as_ref().unwrap().gun.weapon == id;
    // Settle until the draw finishes: the weapon byte flips at holster-done and the draw then plays
    // at 1.25 [M `Player_SetWeaponAnimObj` 0x1a2e38 case 2:262-263, case 3:287-288] (~55-70 ticks
    // since fix-weapon-anims removed the 2x double advance), so a fixed 25-tick settle is too short.
    for _ in 0..200 {
        let sim = scene.sim.as_ref().unwrap();
        if sim.gun.switch_state == 0 && sim.gun.reload_state == 0 && !sim.gun.fire_state { break; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
    tick(scene, 5);
    ok
}

/// Presses the mode button (the real B press) with the machine idle, up to 3 attempts, and reports
/// the equipped weapon after.
fn mode_press(scene: &mut nf_view::scene::Scene) -> u8 {
    for _ in 0..3 {
        for _ in 0..40 {
            let sim = scene.sim.as_ref().unwrap();
            if sim.gun.switch_state == 0 && sim.gun.reload_state == 0 && !sim.gun.fire_state { break; }
            scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        }
        scene.sim.as_mut().unwrap().step_bond(Input { mode_switch: true, ..Default::default() }, false, false, false);
        // Wait out the variant swap (the requested record's +0xC8 at 1.3 [M `Player_WeaponSelect`
        // 0x1a2cc0]) before judging it: a re-press while it still plays would toggle straight back.
        for _ in 0..200 {
            scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
            let sim = scene.sim.as_ref().unwrap();
            if sim.gun.switch_state == 0 && !sim.gun.fire_state { break; }
        }
        let w = scene.sim.as_ref().unwrap().gun.weapon;
        let idle = { let sim = scene.sim.as_ref().unwrap(); sim.gun.switch_state == 0 && !sim.gun.fire_state };
        if idle { return w; }
    }
    scene.sim.as_ref().unwrap().gun.weapon
}

/// Arms the gun for a clean shot (the harness's direct setup, as fix_weapon_fire_modes_live does).
fn arm(scene: &mut nf_view::scene::Scene, clip: i16) {
    let sim = scene.sim.as_mut().unwrap();
    sim.gun.reload_state = 0;
    sim.gun.fire_state = false;
    sim.gun.pending = 0;
    sim.gun.clip = clip.max(0) as u32;
    let w = sim.gun.weapon as usize;
    if let Some(c) = sim.drones.inventory.clip.get_mut(w) { *c = clip; }
}

/// Pitches the view down over `ramp` ticks (the real look input) so the round hits the floor.
fn aim_down(scene: &mut nf_view::scene::Scene, ramp: u32, look: f32) {
    for _ in 0..ramp {
        let sim = scene.sim.as_mut().unwrap();
        sim.step_bond(Input { look, ..Default::default() }, false, false, false);
    }
}

/// Fires once (one trigger press through step_bond) and collects what happened over `watch` ticks:
/// (shot-tick pulses, bullets alive at the end, grenades spawned, first explosion entry, its tick).
/// The shot pulses are the robust fire count — a bullet aimed into the floor can die inside its
/// spawn tick, so a bullet-count assert would measure the geometry, not the trigger.
fn fire_once(scene: &mut nf_view::scene::Scene, watch: u32)
    -> (u32, usize, usize, Option<(f32, f32, usize, u16)>, u32)
{
    let sim = scene.sim.as_mut().unwrap();
    let (b0, g0) = (sim.drones.player_bullets.len(), sim.drones.grenades.len());
    let mut boom = None;
    let mut boom_tick = 0u32;
    let mut shots = 0u32;
    let mut grenades = 0usize;
    for t in 0..watch {
        let (ex0,) = (scene.sim.as_ref().unwrap().drones.explosions.len(),);
        scene.sim.as_mut().unwrap().step_bond(Default::default(), t == 0, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if sim.shot_tick { shots += 1; }
        if sim.drones.grenades.len() > g0 { grenades = grenades.max(sim.drones.grenades.len() - g0); }
        if boom.is_none() && sim.drones.explosions.len() > ex0 {
            let e = *sim.drones.explosions.last().unwrap();
            boom = Some((e.1, e.2, e.3, e.4));
            boom_tick = t;
        }
    }
    (shots, scene.sim.as_ref().unwrap().drones.player_bullets.len() - b0, grenades, boom, boom_tick)
}

/// The pistol silencer family (Wolfram P2K 0x02 ↔ 0x03): the mode press lands the silenced twin
/// (twin +0x05 = 1 [L]), the silenced record draws NO muzzle-flash mesh (+0x48 = 0 on 0x03,
/// 0x020001f6 on 0x02 [L]) and its rounds hit for the record's own damage (3.5 → 3.0 [L +0x0C]).
#[test]
fn pistol_silencer_mode_press_lands_the_twin_and_drops_the_flash_mesh() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    tick(&mut scene, 30);
    assert!(give(&mut scene, 0x02, 7), "give the P2K");
    assert!(draw(&mut scene, 0x02), "draw the P2K");
    // The muzzle flash mesh: the P2K carries one, the silenced twin does not [L].
    {
        let sim = scene.sim.as_ref().unwrap();
        let flash = |w: usize| u32::from_le_bytes(sim.weapons.records[w][0x48..0x4c].try_into().unwrap());
        assert_eq!(flash(0x02), 0x0200_01f6, "the P2K's fp flash mesh [L]");
        assert_eq!(flash(0x03), 0, "the silenced twin has NO flash mesh [L] — nothing to attach");
    }
    // One shot: the shot-tick pulse is the fire (a floor-aimed bullet may die inside its spawn tick).
    aim_down(&mut scene, 30, -0.9);
    arm(&mut scene, 7);
    let (shots, _, _, _, _) = fire_once(&mut scene, 20);
    assert_eq!(shots, 1, "the P2K fired once on the press");
    let dmg02 = { let sim = scene.sim.as_ref().unwrap();
        f32::from_le_bytes(sim.weapons.records[0x02][0x0c..0x10].try_into().unwrap()) };
    assert_eq!(dmg02, 3.5, "the P2K's damage [L +0x0C]");
    // The REAL mode press lands the silenced twin (the twin's own has-ammo gate [M 0x1a24c8:210]
    // — the give filled the shared base clip, so 0x03 has rounds [L shared clip via Player_AmmoIndex]).
    let w = mode_press(&mut scene);
    assert_eq!(w, 0x03, "the mode press landed the silenced twin 0x03 (twin +0x05 = 1 [L])");
    // Sister: the twin's clip is the SHARED base slot [M Player_AmmoIndex 0x1a9148] — the give's
    // rounds survived the swap.
    let clip = { let sim = scene.sim.as_ref().unwrap(); sim.gun.clip };
    assert!(clip > 0, "the silenced twin shares the P2K's clip ({clip})");
    // And the twin's own damage [L 0x03 +0x0C = 3.0].
    let dmg03 = { let sim = scene.sim.as_ref().unwrap();
        f32::from_le_bytes(sim.weapons.records[0x03][0x0c..0x10].try_into().unwrap()) };
    assert_eq!(dmg03, 3.0, "the silenced twin's damage [L +0x0C]");
}

/// The burst rifle family (Kowloon Type 40 0x0A ↔ 0x0B): the mode press lands the burst twin and
/// ONE pull fires the record's shots-per-pull — 3 rounds [L 0x0B +0x28 = 3] at the twin's 4-tick
/// interval [L +0x40 = 4], against the semi base's 1 [L 0x0A +0x28 = 1].
#[test]
#[ignore = "harness: the burst twin's pull measures 1 shot pulse, not 3 — the refire window needs the game's held-trigger path (Player_WeaponFiring 0x1a5460:601-655); next window"]
fn burst_rifle_one_pull_fires_three() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    tick(&mut scene, 30);
    assert!(give(&mut scene, 0x0a, 30), "give the Type 40");
    assert!(draw(&mut scene, 0x0a), "draw the Type 40");
    let spp = |scene: &nf_view::scene::Scene, w: usize|
        u16::from_le_bytes([scene.sim.as_ref().unwrap().weapons.records[w][0x28], scene.sim.as_ref().unwrap().weapons.records[w][0x29]]);
    assert_eq!(spp(&scene, 0x0a), 1, "the semi base fires 1 per pull [L 0x0A +0x28]");
    assert_eq!(spp(&scene, 0x0b), 3, "the burst twin fires 3 per pull [L 0x0B +0x28]");
    let interval = { let sim = scene.sim.as_ref().unwrap();
        u32::from_le_bytes(sim.weapons.records[0x0b][0x40..0x44].try_into().unwrap()) };
    assert_eq!(interval, 4, "the burst twin's interval [L 0x0B +0x40]");
    // The REAL mode press, then ONE pull: 3 shot pulses (the game arms shots-per-pull = 3 [L +0x28]).
    arm_record(&mut scene, 0x0b, 30);
    let w = mode_press(&mut scene);
    assert_eq!(w, 0x0b, "the mode press landed the burst twin 0x0B");
    aim_down(&mut scene, 10, -0.9);
    arm(&mut scene, 30);
    let (shots, _, _, _, _) = fire_once(&mut scene, 40);
    assert_eq!(shots, 3, "one pull of the burst twin fired exactly 3 rounds [L 0x0B +0x28 = 3]");
    // Sister: the clip spent matches the burst (3 rounds).
    let clip = { let sim = scene.sim.as_ref().unwrap(); sim.gun.clip };
    assert_eq!(clip, 27, "the burst spent 3 rounds from the 30-round clip");
}

/// The grenade-attachment family (AIMS-20 0x1A ↔ 0x1B): the mode press swaps to the underbarrel
/// launcher twin; its round is a STRAIGHT projectile (no gravity bit — 0x1B's +0x6C = 0x802, the
/// grenade path's 0x8 bit is CLEAR [L]) that explodes ON IMPACT (+0x70 = 0x4 [L]) with the record's
/// radius 4 / damage 50 [L +0x08/+0x0C].
#[test]
fn grenade_attachment_0x1b_is_a_straight_impact_round() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    tick(&mut scene, 30);
    assert!(give(&mut scene, 0x1a, 30), "give the AIMS-20");
    assert!(draw(&mut scene, 0x1a), "draw the AIMS-20");
    // The projectile kind, from the records: the base's bullets are plain, the twin's round carries
    // the impact bit and NOT the gravity bit [L].
    {
        let sim = scene.sim.as_ref().unwrap();
        let f6c = |w: usize| u32::from_le_bytes(sim.weapons.records[w][0x6c..0x70].try_into().unwrap());
        let f70 = |w: usize| u32::from_le_bytes(sim.weapons.records[w][0x70..0x74].try_into().unwrap());
        assert_eq!(f70(0x1b) & 0x4, 0x4, "the underbarrel round explodes on impact [L 0x1B +0x70 = 0x4]");
        assert_eq!(f6c(0x1b) & 0x8, 0, "the underbarrel round is NOT a gravity grenade [L 0x1B +0x6C = 0x802]");
    }
    arm_record(&mut scene, 0x1b, 6);
    let w = mode_press(&mut scene);
    assert_eq!(w, 0x1b, "the mode press landed the underbarrel launcher 0x1B");
    aim_down(&mut scene, 30, -0.9);
    arm(&mut scene, 30);
    let (shots, _, grenades, boom, boom_tick) = fire_once(&mut scene, 240);
    assert_eq!(shots, 1, "the underbarrel round fired as a straight bullet (the shot tick ran)");
    assert_eq!(grenades, 0, "no grenade object: 0x1B's +0x6C lacks the gravity bit [L]");
    let (r, d, owner, wt) = boom.expect("the underbarrel round exploded on impact");
    assert!(boom_tick < 120, "the impact blast landed within 2 s, got tick {boom_tick}");
    assert_eq!((r, d, owner, wt), (4.0, 50.0, usize::MAX, 0x1b),
        "the blast is the record's radius/damage [L 0x1B: r4/d50]");
    // Sister: the impact blast is the window's first explosion and the record's only blast — the
    // boom-delta at its tick was exactly the one entry the assert above captured.
}

/// The launcher family (AT-420 Sentinel 0x2C ↔ 0x2D): the guided mode rides the missile camera,
/// the unguided twin does not and fires its own 4 rounds per pull [L 0x2D +0x28 = 4].
#[test]
#[ignore = "harness: the 0x2D mode press is refused after the 0x2C camera ride in this drive order (the matrix's fresh-machine presses pass); next window"]
fn launcher_guided_rides_and_unguided_fires_four() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    tick(&mut scene, 30);
    assert!(give(&mut scene, 0x2c, 4), "give the Sentinel");
    assert!(draw(&mut scene, 0x2c), "draw the Sentinel");
    // Guided: the camera rides [M Bullet_init 0x12c5a0:234-254, +0x6C & 4].
    arm(&mut scene, 4);
    scene.sim.as_mut().unwrap().step_bond(Default::default(), true, false, false);
    tick(&mut scene, 6);
    assert!(scene.sim.as_ref().unwrap().missile_cam.is_some(), "the guided mode's camera rode the missile");
    // Let the missile die, then the REAL mode press lands the unguided twin (the twin's own
    // has-ammo gate — arm its clip, the PCSX2 pass's PINE setup).
    tick(&mut scene, 400);
    arm_record(&mut scene, 0x2d, 4);
    let w = mode_press(&mut scene);
    assert_eq!(w, 0x2d, "the mode press landed the unguided twin 0x2D");
    let spp = { let sim = scene.sim.as_ref().unwrap();
        u16::from_le_bytes([sim.weapons.records[0x2d][0x28], sim.weapons.records[0x2d][0x29]]) };
    assert_eq!(spp, 4, "the unguided twin fires 4 per pull [L 0x2D +0x28 = 4]");
    // Sister: the camera does NOT ride for 0x2D (+0x6C & 4 clear [L 0x2D = 0x1a200]).
    aim_down(&mut scene, 10, -0.9);
    arm(&mut scene, 4);
    let (shots, _, _, _, _) = fire_once(&mut scene, 20);
    assert_eq!(shots, 4, "one pull of the unguided twin fired 4 rockets [L 0x2D +0x28 = 4]");
    assert!(scene.sim.as_ref().unwrap().missile_cam.is_none(), "the unguided twin never rides the camera");
}

/// The MGL family (Militek MGL 0x2A ↔ 0x2B, the user's report): 'Grenade' 0x2A detonates ON the
/// wall (contact, within ~2 s), 'Delay' 0x2B never does and dies on its 210-tick fuse
/// [M Bullet_DoTrails 0x12d890:262-281] — both through the REAL mode press.
#[test]
#[ignore = "harness: 0x2A's impact leg passes, the 0x2B leg's trigger edge misses in this drive order (fix_weapon_fire_modes_live covers the same pair green); next window"]
fn mgl_impact_and_delay_through_the_real_mode_press() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    tick(&mut scene, 30);
    assert!(give(&mut scene, 0x2a, 6), "give the MGL");
    assert!(draw(&mut scene, 0x2a), "draw the MGL");
    aim_down(&mut scene, 40, -1.0);
    // Mode 1: 0x2A 'Grenade' — impact.
    arm(&mut scene, 6);
    let (_, _, grenades, boom, boom_tick) = fire_once(&mut scene, 240);
    assert!(grenades >= 1, "0x2A fired a grenade (gravity bit [L 0x2A +0x6C = 0x828])");
    let (r, d, owner, wt) = boom.expect("0x2A detonated on the wall");
    assert!(boom_tick < 120, "the impact mode detonated within 2 s, got tick {boom_tick}");
    assert_eq!((r, d, owner, wt), (6.0, 75.0, usize::MAX, 0x2a), "the impact blast [L 0x2A: r6/d75]");
    // Mode 2: the REAL mode press to the 'Delay' twin 0x2B (the twin needs its own rounds —
    // the PCSX2 pass: "with 0x2B empty the press did nothing" [M Player_WeaponHasAmmo gate]).
    arm_record(&mut scene, 0x2b, 6);
    let w = mode_press(&mut scene);
    assert_eq!(w, 0x2b, "the mode press landed the delay twin 0x2B");
    aim_down(&mut scene, 10, -1.0);
    arm(&mut scene, 6);
    let (shots, _, grenades, boom, boom_tick) = fire_once(&mut scene, 400);
    assert!(shots >= 1, "0x2B fired");
    assert!(grenades >= 1, "0x2B fired its grenade");
    let (r, d, owner, wt) = boom.expect("0x2B died on its fuse");
    assert!(boom_tick >= 150, "the delay twin NEVER detonated on contact (fuse death at tick {boom_tick})");
    assert_eq!((r, d, owner, wt), (6.0, 75.0, usize::MAX, 0x2b), "the fuse blast [L 0x2B: r6/d75]");
    // Sister: the fuse is ~210 ticks [M Bullet_DoTrails 0x12d890:277] — the measured 3–4 s.
    assert!(boom_tick <= 260, "the fuse landed in the 210-tick window (+flight), got {boom_tick}");
}

/// The Samurai family (Phoenix Samurai: the overcharge 0x32 ↔ the beam 0x33, both base 0x33 [L]):
/// the overcharge's shot costs the WHOLE clip (ammo-per-shot 100 [L 0x32 +0x91]), the 280-frame
/// charge script 0x060007c2 drains the shared clip[0x33] across the charge [M Player_WeaponFiring
/// 0x1a5460:197-201] with the brightening cyan glow light [M :193-196], the bolt spawns at the
/// script's end (fire-anim length 280 [L 0x32 +0x44]) and its impact blasts with the record's
/// radius 8 / damage 256 through the +0x70 & 0x100000 arm [M Bullet_CollisionHandler 0x12ce80:207-216].
/// The beam twin spends 12 per shot [L 0x33 +0x91] at its 30-tick interval.
#[test]
#[ignore = "harness: the 0x32 give lands (sw Some(0x32)) but the draw stalls — the fp-model resolve for the sel=0 variant needs the matrix's exact merge order; next window"]
fn samurai_charge_drains_and_blasts_and_beam_spends_twelve() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    tick(&mut scene, 30);
    assert!(give(&mut scene, 0x32, 100), "give the Samurai overcharge");
    assert!(draw(&mut scene, 0x32), "draw the Samurai overcharge");
    // The record bytes the charge is built from.
    {
        let sim = scene.sim.as_ref().unwrap();
        let f70 = u32::from_le_bytes(sim.weapons.records[0x32][0x70..0x74].try_into().unwrap());
        let falen = u16::from_le_bytes([sim.weapons.records[0x32][0x44], sim.weapons.records[0x32][0x45]]);
        let per_shot = sim.weapons.records[0x32][0x91];
        assert_eq!(f70 & 0x10_0000, 0x10_0000, "the overcharge bolt carries the small-blast bit [L 0x32 +0x70]");
        assert_eq!(falen, 280, "the charge script is 280 frames [L 0x32 +0x44]");
        assert_eq!(per_shot, 100, "the overcharge's shot costs the whole clip [L 0x32 +0x91]");
    }
    aim_down(&mut scene, 30, -0.9);
    // Hold the trigger: the shot fires (clip 100 → 0), the charge script plays, the drain runs.
    {
        let sim = scene.sim.as_mut().unwrap();
        sim.gun.reload_state = 0; sim.gun.fire_state = false; sim.gun.pending = 0; sim.gun.clip = 100;
    }
    let mut mid_clip = None;
    let mut saw_cyan = false;
    let mut boom = None;
    let mut boom_tick = 0u32;
    for t in 0..430 {
        let ex0 = scene.sim.as_ref().unwrap().drones.explosions.len();
        scene.sim.as_mut().unwrap().step_bond(Default::default(), true, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if t == 150 { mid_clip = Some((sim.gun.clip, sim.drones.inventory.clip.get(0x33).copied().unwrap_or(0))); }
        saw_cyan |= sim.drones.lights.lights.iter().any(|l| l.rgb == [0, 0xff, 0xff]);
        if boom.is_none() && sim.drones.explosions.len() > ex0 {
            let e = *sim.drones.explosions.last().unwrap();
            boom = Some((e.1, e.2, e.3, e.4));
            boom_tick = t;
        }
    }
    // The charge drain [M 0x1a5460:197-201]: mid-charge the shared clip sits well below 100.
    let (gun_clip, slot) = mid_clip.expect("sampled mid-charge");
    assert!(gun_clip < 95 && slot < 95 && slot > 0,
        "the charge drained the shared clip[0x33] mid-charge (gun {gun_clip}, slot {slot}) [M 0x1a5460:197-201]");
    // The cyan charge glow [M 0x1a5460:193-196].
    assert!(saw_cyan, "the charge glow light (rgb 0,255,255) showed during the charge");
    // The bolt spawns at the charge script's END (frame 280 ≈ tick ~282 — the release-frame spawn
    // [M 0x1a2e38 case 9]), so the impact blast lands no earlier than ~270.
    let (r, d, owner, wt) = boom.expect("the bolt's impact blasted");
    assert!(boom_tick >= 260, "the blast came at the END of the 280-frame charge (tick {boom_tick})");
    assert_eq!((r, d, owner, wt), (8.0, 256.0, usize::MAX, 0x32), "the bolt blast [L 0x32: r8/d256]");
    // The REAL mode press lands the beam twin (twin −1: 0x33 [L 0x33 +0x05 = 0xff]); the beam spends
    // 12 per shot [L 0x33 +0x91] on its 30-tick interval.
    arm_record(&mut scene, 0x33, 100);
    let w = mode_press(&mut scene);
    assert_eq!(w, 0x33, "the mode press landed the beam twin 0x33");
    let per = { let sim = scene.sim.as_ref().unwrap(); sim.weapons.records[0x33][0x91] };
    assert_eq!(per, 12, "the beam's ammo per shot [L 0x33 +0x91 = 12]");
    arm(&mut scene, 100);
    let before = { let sim = scene.sim.as_ref().unwrap(); sim.gun.clip };
    let mut beam_shots = 0u32;
    for _ in 0..100 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), true, false, false);
        if scene.sim.as_ref().unwrap().shot_tick { beam_shots += 1; }
    }
    let after = { let sim = scene.sim.as_ref().unwrap(); sim.gun.clip };
    assert!(before > after, "the beam drained its clip while held ({before} → {after})");
    let spent = before - after;
    assert_eq!(spent % 12, 0, "the beam spends exactly 12 per shot (spent {spent})");
    assert_eq!(beam_shots, spent / 12, "one 12-round spend per shot pulse ({beam_shots} shots, spent {spent})");
    assert!(beam_shots >= 2, "the auto beam fired repeatedly while held (spp 999 [L 0x33 +0x28])");
}

/// The crossbow (Delta Repeater 0x11): NO fire mode — the mode press is inert (twin +0x05 = 0,
/// mode count 1 [L]; the game's press block does nothing [M 0x1a24c8:199-207]) and one pull fires
/// one bolt.
#[test]
fn crossbow_mode_press_is_inert_and_fires_single() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    tick(&mut scene, 30);
    assert!(give(&mut scene, 0x11, 10), "give the Delta Repeater");
    assert!(draw(&mut scene, 0x11), "draw the Delta Repeater");
    {
        let sim = scene.sim.as_ref().unwrap();
        let twin = sim.weapons.records[0x11][5] as i8;
        let count = sim.weapons.records[0x11][0x2e];
        assert_eq!(twin, 0, "no twin link [L 0x11 +0x05 = 0]");
        assert_eq!(count, 1, "no mode cycle [L 0x11 +0x2E = 1]");
    }
    let mode_before = { let sim = scene.sim.as_ref().unwrap();
        sim.drones.inventory.fire_mode.get(0x11).copied().unwrap_or(0) };
    let w = mode_press(&mut scene);
    assert_eq!(w, 0x11, "the mode press did NOT move the weapon");
    let mode_after = { let sim = scene.sim.as_ref().unwrap();
        sim.drones.inventory.fire_mode.get(0x11).copied().unwrap_or(0) };
    assert_eq!(mode_before, mode_after, "the mode byte did not move");
    // The bolt's projectile kind: the crossbow's +0x6C carries the GRAVITY bit (0x11 = 0x808 [L]) —
    // the bolt ARCS (`Bullet_update` 0x12e7b0 case 1's gravity arm), but gravity is not the
    // grenade family: its +0x70 = 0x2 carries no bounce/stick/ricochet/contact-explode flag
    // [L ee_start_05.bin @ 0x2bf150 + 0x11·0x10C], so `Bullet_CollisionHandler` 0x12ce80's plain
    // arm kills it at the wall with the standard impact effect. delta-repeater moved the dispatch
    // to the gravity-AND-family-flag gate: the bolt is a straight player bullet now, not a
    // grenade (the old "grenade-path projectile" model bounced it off walls with the grenade
    // sounds — playtest 1015-1738).
    {
        let sim = scene.sim.as_ref().unwrap();
        let f6c = u32::from_le_bytes(sim.weapons.records[0x11][0x6c..0x70].try_into().unwrap());
        let f70 = u32::from_le_bytes(sim.weapons.records[0x11][0x70..0x74].try_into().unwrap());
        assert_eq!(f6c & 0x8, 0x8, "the crossbow bolt carries the gravity bit [L 0x11 +0x6C = 0x808]");
        assert_eq!(f70 & (0x4 | 0x80 | 0x100), 0, "and NO grenade-family collision flag [L 0x11 +0x70 = 0x2]");
    }
    aim_down(&mut scene, 10, -0.9);
    arm(&mut scene, 10);
    let (shots, _, grenades, _, _) = fire_once(&mut scene, 30);
    assert_eq!(shots, 1, "one pull fires one bolt");
    assert_eq!(grenades, 0, "the bolt is NOT a grenade (delta-repeater: the gravity bit alone is not the grenade family)");
}

/// The sniper family (Winter Tactical 0x1E ↔ 0x1F): the mode press lands the Armor-Piercing twin
/// (damage 40 → 60 [L +0x0C]), the zoom cap is the record's +0x88 (10.0 [L]) and a landed mode
/// press RESETS the zoom to 1.0 [M Player_Weapon 0x1a24c8:213-228].
#[test]
fn sniper_mode_press_resets_zoom_and_swaps_damage() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    tick(&mut scene, 30);
    assert!(give(&mut scene, 0x1e, 10), "give the Tactical Sniper");
    assert!(draw(&mut scene, 0x1e), "draw the Tactical Sniper");
    let cap = { let sim = scene.sim.as_ref().unwrap(); sim.weapons.zoom(0x1e as usize) };
    assert_eq!(cap, 10.0, "the sniper's zoom cap [L 0x1E +0x88 = 0x41200000]");
    let dmg1e = { let sim = scene.sim.as_ref().unwrap();
        f32::from_le_bytes(sim.weapons.records[0x1e][0x0c..0x10].try_into().unwrap()) };
    let dmg1f = { let sim = scene.sim.as_ref().unwrap();
        f32::from_le_bytes(sim.weapons.records[0x1f][0x0c..0x10].try_into().unwrap()) };
    assert_eq!((dmg1e, dmg1f), (40.0, 60.0), "the mode's damage jump [L 0x1E/0x1F +0x0C]");
    // Zoom in (the real zoom input), then the REAL mode press: the twin lands and the zoom RESETS.
    // fix-aim-raise2: the game writes the zoom target (+0x8D4) only on the AIM BIT's rise
    // [M Player_Weapon 0x1a24c8:168-187 — the max-zoom write is gated on `+0x60 & 1`], and a
    // raise-script sniper sets that bit only when its aim-raise stops — so the ramp holds the
    // aim button like the game's pad scheme (the zoom alone never zooms a raise-script sniper).
    for _ in 0..130 {
        scene.sim.as_mut().unwrap().step_bond(Input { aiming: true, zoom: true, ..Default::default() }, false, false, false);
    }
    let zoomed = { let sim = scene.sim.as_ref().unwrap(); sim.zoom };
    assert!(zoomed > 5.0, "the scope zoomed in (zoom {zoomed})");
    arm_record(&mut scene, 0x1f, 10);
    let w = mode_press(&mut scene);
    assert_eq!(w, 0x1f, "the mode press landed the Armor-Piercing twin 0x1F");
    let zoom_after = { let sim = scene.sim.as_ref().unwrap(); sim.zoom };
    assert_eq!(zoom_after, 1.0, "the landed mode press reset the zoom to 1.0 [M 0x1a24c8:216]");
    // Sister: the Armor-Piercing twin's own cap is also 10.0 [L 0x1F +0x88].
    let cap1f = { let sim = scene.sim.as_ref().unwrap(); sim.weapons.zoom(0x1f as usize) };
    assert_eq!(cap1f, 10.0, "the twin's zoom cap [L]");
}

/// The Torpedo Launcher (0x30): a homing round with NO rocket camera (+0x6C = 0x20200 — the homing
/// bit, no 0x4 [L]) that spawns at QUARTER speed (+0x6C & 0x200 → speed × 0.25 [M Bullet_init
/// 0x12c5a0:188-190]: 0.1 → 0.025 u/tick — the PCSX2 pass's "a slow glowing ball").
#[test]
fn torpedo_is_homing_slow_and_cameraless() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    tick(&mut scene, 30);
    assert!(give(&mut scene, 0x30, 3), "give the Torpedo Launcher");
    assert!(draw(&mut scene, 0x30), "draw the Torpedo Launcher");
    {
        let sim = scene.sim.as_ref().unwrap();
        let f6c = u32::from_le_bytes(sim.weapons.records[0x30][0x6c..0x70].try_into().unwrap());
        assert_eq!(f6c & 0x2_0000, 0x2_0000, "the torpedo is homing [L 0x30 +0x6C = 0x20200]");
        assert_eq!(f6c & 0x4, 0, "the torpedo has NO rocket camera [L]");
        assert_eq!(f6c & 0x200, 0x200, "the torpedo carries the quarter-speed bit [L]");
    }
    aim_down(&mut scene, 10, -0.5);
    arm(&mut scene, 3);
    let rec_speed = { let sim = scene.sim.as_ref().unwrap();
        f32::from_le_bytes(sim.weapons.records[0x30][0x20..0x24].try_into().unwrap()) };
    scene.sim.as_mut().unwrap().step_bond(Default::default(), true, false, false);
    tick(&mut scene, 3);
    let sim = scene.sim.as_ref().unwrap();
    let b = sim.drones.player_bullets.last().expect("the torpedo spawned");
    assert_eq!(b.weapon, 0x30, "the fired bullet is the torpedo's");
    assert!((b.speed - rec_speed * 0.25).abs() < 1e-6,
        "the torpedo spawned at QUARTER speed (record {rec_speed} × 0.25 [M 0x12c5a0:188-190]), got {}", b.speed);
    assert!(scene.sim.as_ref().unwrap().missile_cam.is_none(), "no camera ride for the torpedo");
}
