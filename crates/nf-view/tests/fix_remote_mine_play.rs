//! fix-remote-mine-play: the remote mine 0x37 + its detonator twin 0x38 through the REAL input
//! path (`nf_view::load` + the panel give + `Sim::step_bond` with the `Input` lib.rs builds —
//! mouse wheel / Q cycle, G gadgets, B mode press, the trigger), mirroring the user's playtest
//! 1014-1725 (`--give 0x37,0x38`, mouse + keyboard).
//!
//! The facts under test, all off the disc/decompile/snapshot:
//! - The stick [M `Bullet_CollisionHandler` 0x12ce80:255-266, 296-330, 345-352]: the record's
//!   +0x70 carries 0x100 [L ee_snapshot.bin @ 0x2bf150 + 0x37·0x10C + 0x70 = 0x18300; the stick
//!   family 0x3a = 0x80100, 0x3b = 0x300, 0x45 = 0x100; frag/stun/smoke lack the bit] + a world
//!   hit → `obj+0xF4 = 3` (stuck), the matrix aligned to the NEGATED hit normal, parked
//!   `hit + normal·0.04`, NO fuse write (BU+0xFC keeps Bullet_init's 7777.0 player / 420 guard
//!   [M 0x12c5a0:214-223]) and NO sound (the 0x37 arm has no Sound_Play3D — 0x3b/0x45 play
//!   0x4a7, 0x3a plays 0x446). The 6-placed cap walk [:296-330] deletes the oldest past 6.
//! - The stuck fuse [M `Bullet_DoTrails` 0x12d890:268-285]: the 0x37 arm skips the decrement
//!   ONLY while BU+0xFC == 7777.0 — any other fuse (the detonator's mark, a guard's 420) keeps
//!   counting in state 3 and detonates at < 0.
//! - The detonator [M `Player_WeaponInitBullet` 0x1a4bf0's +0x68 & 0x20000 arm, disasm
//!   0x1a4c30-0x1a4cd4]: the walk marks EVERY player-owned bullet of the base with
//!   `BU+0xFC = −10.0` (0xc1200000 — the earlier "+10.0" note dropped the sign) → DoTrails
//!   detonates on the next tick with `Explode_Create(+0x08, +0x08, +0x0C)` — radius 8 /
//!   damage 100 [L record 0x37]; `Player_WeaponHasAmmo(base)` then arms the switch back to the
//!   mine (BL+0x63 = base [M 0x1a4cb8]) or, dry, clears and GetBestWeapon [M 0x1a4cbc-0x1a4cd4].
//! - The binding [L records]: 0x38 = 0x00020010 at +0x68 (the arm), **+0x04 = 0** (the cycle's
//!   gate [M 0x1a1894/0x1a6f80] — the detonator never appears in the weapon wheel or the
//!   gadget/watch cycle), base 0x37, twin −1; 0x37 = 0x14090, sel 1, twin +1 (the runtime patch
//!   [M Player_Init] the loader applies). The mode press (pad ○ / keyboard B) is the game's
//!   route to the detonator — the twin switch through the shared ammo-22 slot [L +0x90 = 22
//!   both]; the HUD mode names resolve to "Grenade" (0x37) / "Detonate" (0x38) [L USATxt.dat
//!   +0x30[0] = 0x0500000b / 0x0500004d].
//!
//! Opt-in: set `NF_ISO` to the disc image.

use nf_game::player::Input;
use nf_view::scene::Scene;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str) -> Scene {
    // The give ids preloaded (the fp models in memory — Player_EquipWeapon 0x1a8a58:20 refuses a
    // weapon whose +0xDC model was never loaded; the user's --give 0x37,0x38 does the same).
    let mut scene = nf_view::load_weapons(iso, "07000005.bin", false, &[0x37, 0x38, 0x34]).expect("load 07000005");
    for _ in 0..7200 {
        if scene.sim.as_ref().map_or(false, |s| !s.disabled) { break; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
    assert!(scene.sim.as_ref().map_or(false, |s| !s.disabled), "07000005 never enabled (intro stuck)");
    scene
}

fn idle(scene: &Scene) -> bool {
    let s = scene.sim.as_ref().unwrap();
    s.gun.switch_state == 0 && s.gun.reload_state == 0 && !s.gun.fire_state
}

fn settle_idle(scene: &mut Scene, ticks: usize) {
    for _ in 0..ticks {
        if idle(scene) && scene.sim.as_ref().unwrap().gun.switch_request == scene.sim.as_ref().unwrap().gun.weapon { break; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
}

/// Gives `id` through the dev panel (the give is the user's `--give` path) and selects it
/// through the direct switch (`switch_to` — owned[] is per base, and 0x37 IS a base).
fn give_and_reach(scene: &mut Scene, id: u8) {
    let (status, _sw) = nf_view::panel::apply(scene, &[
        ("do".to_string(), "weapon".to_string()),
        ("id".to_string(), format!("{id:#x}")),
    ]);
    assert!(status.contains("gave"), "the give of {id:#x} was refused: {status}");
    for _ in 0..400 {
        if scene.sim.as_ref().unwrap().gun.weapon == id { break; }
        scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(id), ..Default::default() }, false, false, false);
    }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, id, "{id:#x} never reached");
    settle_idle(scene, 7200);
    assert!(idle(scene), "{id:#x} never settled idle");
}

/// A wall within 6 m of `pos` from the level's own collision, wall-ish (the normal mostly
/// horizontal) — the throw target.
fn find_wall(sim: &nf_view::scene::Sim, pos: [f32; 3]) -> Option<([f32; 3], [f32; 3])> {
    for k in 0..24u32 {
        let a = k as f32 * std::f32::consts::TAU / 24.0;
        let dir = [a.sin(), 0.0, a.cos()];
        let far = [pos[0] + dir[0] * 6.0, pos[1], pos[2] + dir[2] * 6.0];
        if let Some((t, n)) = sim.world.segment_hit(pos, far) {
            if n[1].abs() < 0.5 {
                let hit = [pos[0] + (far[0] - pos[0]) * t, pos[1] + (far[1] - pos[1]) * t, pos[2] + (far[2] - pos[2]) * t];
                return Some((hit, n));
            }
        }
    }
    None
}

/// Faces `hit` from Bond's position (yaw only — the throw input does the rest).
fn face(sim: &mut nf_view::scene::Sim, hit: [f32; 3]) {
    let dx = hit[0] - sim.bond.pos[0];
    let dz = hit[2] - sim.bond.pos[2];
    sim.bond.yaw = dx.atan2(dz);
}

/// The record facts under the test, read from the port's disc-parsed table.
fn record(sim: &nf_view::scene::Sim, w: usize) -> (u8, i8, u32, u32) {
    let r = &sim.weapons.records[w];
    (r[4], r[5] as i8, u32::from_le_bytes(r[0x68..0x6c].try_into().unwrap()), u32::from_le_bytes(r[0x70..0x74].try_into().unwrap()))
}

/// The thrown mine STICKS on contact and never bounces [M 0x12ce80:255-266, 296-330]: `stuck`
/// rises on the FIRST world contact with bounces == 0, the phase normal is the hit normal, the
/// position stays parked forever (a silent placed mine — no explosion, state < 2). FAILED BEFORE
/// THE FIX: the mine bounced (bounces climbed, then it rested loose on the ground — the port's
/// own grenade.rs comment deferred "the mine 0x37's own stick branch" to this task).
#[test]
fn the_remote_mine_sticks_on_contact_and_never_bounces() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    give_and_reach(&mut scene, 0x37);
    {
        let sim = scene.sim.as_ref().unwrap();
        let (sel, twin, f68, f70) = record(sim, 0x37);
        assert_eq!((sel, twin), (1, 1), "0x37 is selectable with the +1 twin [L runtime patch]");
        assert_eq!(f68 & 0x100, 0, "0x37's +0x68 lacks the detonator arm bit");
        assert_eq!(f70 & 0x100, 0x100, "0x37's +0x70 carries the stick bit [L 0x18300]");
    }
    let (hit, normal) = {
        let sim = scene.sim.as_ref().unwrap();
        find_wall(sim, sim.bond.pos).expect("a wall near Bond's start")
    };
    face(scene.sim.as_mut().unwrap(), hit);
    // The throw: hold the trigger through the throw script (the release frame +0x44 = 26 [L]).
    let mut stuck_tick = 0usize;
    let mut first_contact_bounces = 0u8;
    for t in 1..=90 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), true, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if let Some(g) = sim.drones.grenades.iter().find(|g| g.owner == usize::MAX && g.weapon_type == 0x37) {
            if g.stuck { stuck_tick = t; first_contact_bounces = g.bounces; break; }
            assert_eq!(g.bounces, 0, "tick {t}: the mine bounced before it stuck (pre-fix behaviour)");
        }
    }
    scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
    assert_eq!(first_contact_bounces, 0, "the stick lands on the FIRST contact with no bounce");
    assert!(stuck_tick > 0, "the mine never stuck within 90 ticks of the throw");
    let g = scene.sim.as_ref().unwrap().drones.grenades.iter().find(|g| g.weapon_type == 0x37 && g.owner == usize::MAX).expect("the placed mine");
    let stuck_at = g.pos;
    assert!((g.range - 7777.0).abs() < 1e-3, "the stick writes NO fuse — BU+0xFC keeps 7777.0 [M 0x12ce80 the 0x37 arm], got {}", g.range);
    let p = g.phase.as_ref().expect("the stick phase box");
    for k in 0..3 { assert!((p.normal[k] - normal[k]).abs() < 1e-3, "the phase normal is the hit normal"); }
    // The placed mine stays parked, silent, alive: no explosion, no drift, state < 2.
    let ex0 = scene.sim.as_ref().unwrap().drones.explosions.len();
    for _ in 0..120 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        let sim = scene.sim.as_ref().unwrap();
        assert!(sim.drones.explosions.len() == ex0, "a placed mine must not explode on its own");
        let g = sim.drones.grenades.iter().find(|g| g.weapon_type == 0x37 && g.owner == usize::MAX).expect("the mine vanished while parked");
        assert!(g.stuck && g.state < 2, "the mine left the stuck state on its own");
        for k in 0..3 { assert!((g.pos[k] - stuck_at[k]).abs() < 1e-4, "the parked mine drifted"); }
    }
    // Sister: the frag 0x34 (0x280 — no stick bit [L]) still bounces and never sticks.
    give_and_reach(&mut scene, 0x34);
    let (hit, _) = {
        let sim = scene.sim.as_ref().unwrap();
        find_wall(sim, sim.bond.pos).expect("a wall near Bond")
    };
    face(scene.sim.as_mut().unwrap(), hit);
    let mut frag_bounced = false;
    for _ in 0..120 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), true, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if let Some(g) = sim.drones.grenades.iter().find(|g| g.owner == usize::MAX && g.weapon_type == 0x34) {
            if g.bounces > 0 { frag_bounced = true; break; }
            assert!(!g.stuck, "the frag must not stick [L +0x70 = 0x280 lacks 0x100]");
        }
    }
    assert!(frag_bounced, "the frag sister never bounced — the stick arm leaked onto 0x34");
}

/// The detonation is the mine's OWN alternate fire: B (the mode press) from the held mine lands
/// the detonator twin 0x38 [M the twin switch through the shared slot; L the runtime +0x05 = 1],
/// and firing it detonates the placed mine within 2 ticks of the press — the game's mark is
/// `BU+0xFC = −10.0` [M disasm 0x1a4c54-0x1a4c88: `lui $at, 0xc120` = 0xc1200000] and DoTrails
/// detonates at < 0 [M 0x12d890:268-285] — then the hand returns to the mine [M 0x1a4cb8].
/// FAILED BEFORE THE FIX: the mark stored a +10.0 stand-in (a 10-tick fuse — the sign was lost)
/// and the stuck mine's fuse never ran at all, so the boom took ~11 ticks — or never came once
/// the stick landed.
#[test]
fn the_mode_press_detonates_the_placed_mine_and_returns_to_the_mine() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    give_and_reach(&mut scene, 0x37);
    // The HUD facts [L USATxt.dat]: the mine's mode name is "Grenade", the detonator's "Detonate".
    {
        let sim = scene.sim.as_ref().unwrap();
        let lang = sim.lang.as_ref().expect("the lang table");
        assert_eq!(lang.bind(0x0500_000b), Some("Grenade"), "0x37's +0x30[0] mode name [L]");
        assert_eq!(lang.bind(0x0500_004d), Some("Detonate"), "0x38's +0x30[0] mode name [L]");
    }
    let (hit, _) = {
        let sim = scene.sim.as_ref().unwrap();
        find_wall(sim, sim.bond.pos).expect("a wall near Bond's start")
    };
    face(scene.sim.as_mut().unwrap(), hit);
    let mut stuck_at = None;
    for _ in 0..90 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), true, false, false);
        if let Some(g) = scene.sim.as_ref().unwrap().drones.grenades.iter().find(|g| g.owner == usize::MAX && g.weapon_type == 0x37 && g.stuck) {
            stuck_at = Some(g.pos); break;
        }
    }
    scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false);
    let stuck_at = stuck_at.expect("the mine never stuck");
    settle_idle(&mut scene, 7200);
    // The mode press: the twin switch 0x37 → 0x38 (the +1 twin [L], the shared ammo-22 slot).
    scene.sim.as_mut().unwrap().step_bond(Input { mode_switch: true, ..Default::default() }, false, false, false);
    for _ in 0..7200 {
        if scene.sim.as_ref().unwrap().gun.weapon == 0x38 && idle(&scene) { break; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
    {
        let sim = scene.sim.as_ref().unwrap();
        assert_eq!(sim.gun.weapon, 0x38, "the mode press must land the detonator twin 0x38");
        let (sel, twin, f68, _f70) = record(sim, 0x38);
        assert_eq!((sel, twin), (0, -1), "0x38 is the non-selectable −1 twin [L]");
        assert_eq!(f68 & 0x20000, 0x20000, "0x38 carries the detonator arm bit [L 0x00020010]");
    }
    // Fire the detonator: the mark + the boom within 2 ticks of the press (the −10.0 store).
    let mut boom_tick = 0usize;
    let mut blast = None;
    for t in 1..=6 {
        scene.sim.as_mut().unwrap().step_bond(Input::default(), true, false, false);
        let sim = scene.sim.as_ref().unwrap();
        if boom_tick == 0 && !sim.drones.explosions.is_empty() {
            boom_tick = t;
            blast = Some(*sim.drones.explosions.last().unwrap());
            break;
        }
    }
    assert_eq!(boom_tick, 1, "the detonator's press detonates within 2 ticks (the −10.0 mark [M 0x1a4c88]), got tick {boom_tick}");
    let (pos, radius, damage, owner, wt) = blast.expect("the blast");
    assert_eq!(wt, 0x37, "the blast is the mine's Explode_Create [L record 0x37]");
    assert!((radius - 8.0).abs() < 1e-3 && (damage - 100.0).abs() < 1e-3, "the blast carries the record's radius 8 / damage 100 [L +0x08/+0x0C]");
    assert_eq!(owner, usize::MAX, "the blast belongs to the player's mine");
    for k in 0..3 { assert!((pos[k] - stuck_at[k]).abs() < 0.05, "the mine explodes where it hangs"); }
    assert!(scene.sim.as_ref().unwrap().drones.grenades.iter().all(|g| g.weapon_type != 0x37), "the detonated mine is gone");
    // The hand returns to the mine [M 0x1a4cb8: BL+0x63 = base].
    for _ in 0..7200 {
        if scene.sim.as_ref().unwrap().gun.weapon == 0x37 && idle(&scene) { break; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x37, "after the boom the hand returns to the mine [M 0x1a4cb8]");
    // Sister: the mine's own fire (before the mode press) throws, it never detonates the placed
    // mine in the hand — a second press of the trigger with 0x37 held throws the NEXT mine
    // instead (the throw path, untouched by this task — the probe's pre-fix run shows the same).
}

/// The detonator must not appear in the watch/gadget cycle [L +0x04 = 0 — the cycle's gate
/// [M 0x1a1894/0x1a6f80]]: the wheel never offers it, the G cycle stays in the gadget band
/// 0x4A..0x5E [M 0x1a707c], and B from the detonator goes straight back to the mine (the twin
/// toggle) — the ONLY route to 0x38 is the mine's own mode press.
#[test]
fn the_detonator_never_appears_in_any_cycle() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso);
    for id in [0x37u8, 0x38] {
        let (status, _) = nf_view::panel::apply(&mut scene, &[
            ("do".to_string(), "weapon".to_string()),
            ("id".to_string(), format!("{id:#x}")),
        ]);
        assert!(status.contains("gave"), "the give of {id:#x} was refused: {status}");
    }
    // Reach the mine (owned already — no second give: a re-give hits the carried cap).
    for _ in 0..400 {
        if scene.sim.as_ref().unwrap().gun.weapon == 0x37 { break; }
        scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(0x37), ..Default::default() }, false, false, false);
    }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x37, "0x37 never reached");
    settle_idle(&mut scene, 7200);
    {
        let sim = scene.sim.as_ref().unwrap();
        assert!(!sim.weapons.selectable(0x38), "0x38 is not selectable [L +0x04 = 0]");
        assert!(sim.weapons.selectable(0x37), "0x37 is selectable [L +0x04 = 1]");
        assert!(sim.drones.inventory.owned[0x37], "the give owns the mine");
    }
    // The wheel: walk the whole weapon band with +1 steps — 0x38 never comes up.
    let mut visited = Vec::new();
    for _ in 0..24 {
        scene.sim.as_mut().unwrap().step_bond(Input { weapon_cycle: 1, ..Default::default() }, false, false, false);
        for _ in 0..7200 {
            if idle(&scene) && scene.sim.as_ref().unwrap().gun.switch_request == scene.sim.as_ref().unwrap().gun.weapon { break; }
            scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        }
        let w = scene.sim.as_ref().unwrap().gun.weapon;
        if let Some(first) = visited.first() { if w == *first { break; } }
        visited.push(w);
        assert_ne!(w, 0x38, "the wheel offered the detonator — the +0x04 gate leaked");
    }
    assert!(visited.len() > 1, "the wheel never moved off 0x37 (visited {visited:?})");
    // The G cycle from the mine: wherever it lands, it is in the gadget band — never 0x38.
    scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(0x37), ..Default::default() }, false, false, false);
    settle_idle(&mut scene, 7200);
    scene.sim.as_mut().unwrap().step_bond(Input { gadget_cycle: 1, ..Default::default() }, false, false, false);
    for _ in 0..7200 {
        if idle(&scene) { break; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
    let w = scene.sim.as_ref().unwrap().gun.weapon;
    assert_ne!(w, 0x38, "the G cycle landed the detonator");
    if w != 0x37 { assert!((0x4a..=0x5e).contains(&w), "the G cycle left the gadget band: {w:#x}"); }
    // B from the mine lands 0x38; B from 0x38 returns to 0x37 (the twin toggle — the mine's
    // alternate fire is a two-way press, as the Samurai's 0x32/0x33 pair).
    scene.sim.as_mut().unwrap().step_bond(Input { switch_to: Some(0x37), ..Default::default() }, false, false, false);
    settle_idle(&mut scene, 7200);
    scene.sim.as_mut().unwrap().step_bond(Input { mode_switch: true, ..Default::default() }, false, false, false);
    for _ in 0..7200 {
        if scene.sim.as_ref().unwrap().gun.weapon == 0x38 && idle(&scene) { break; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x38, "B from the mine lands the detonator");
    scene.sim.as_mut().unwrap().step_bond(Input { mode_switch: true, ..Default::default() }, false, false, false);
    for _ in 0..7200 {
        if scene.sim.as_ref().unwrap().gun.weapon == 0x37 && idle(&scene) { break; }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
    }
    assert_eq!(scene.sim.as_ref().unwrap().gun.weapon, 0x37, "B from the detonator returns to the mine");
}
