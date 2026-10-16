//! pickups-08: the 07000008 safe room's briefcase and rocket launcher (playtests 1012-2337,
//! 1013-1758: "cant pickup the briefcase or rocket launcher"; the invisible wall there is fixed
//! by fix-safe08/fix-safe08b).
//!
//! Where the items come from: the SafePickUps script 0x060009E2 spawns both when ch 136 fires
//! (fix-safe08) — the rocket (kind 0, id 0x2C, the level-conditional 4 rounds [M 0x18dda0:
//! `GameState+0x12 == 0x7000008 && id == 0x2c` → rounds = 4]) at the lower-shelf key
//! (162.59, 42.75, −116.78), the briefcase (kind 4 LABEL, id 6, switch 74 = 0x4A) at the top
//! shelf (162.70, 43.54, −116.22) [D the level-bin script]. The disc's parked 0xF0 placements
//! (#290/#291) sit 10 x east, inside the wall — not takeable, and the disc flags probe below
//! shows their shells carry no skip bits.
//!
//! The take query, decoded to the end this session (the fix-safe08b open item): `Pickup_Update`
//! 0x18e148:213-214 calls `Collide_LineOfSight` 0x1edf50(from = the player's world-sphere centre,
//! to = the pickup's, cel = the player's obj+0x20, exclude = the pickup obj, 0, 0x721); the
//! wrapper disassembly (0x1edf50-0x1edf98) forwards `char = 0` (so `Collide_RayIntersect`
//! 0x1ec690 tests EVERY candidate node, dynamic-owned included), query word `0x721 | 4 = 0x725`
//! (HITTEST+0x98) and mask 0x10 (HITTEST+0x9A, arming the per-triangle `Collide_Filter` 0x1ed888
//! inside `Intersect_RayGeom` 0x10a8c8). `Collide_PickObj` 0x1eae80 keeps a dynamic object unless
//! obj+0xF0 & (0x20 | 0x1000) or its class byte (+0xff) is 0x02/0x03/0x05/0x11/0x12 — the SS door
//! (class 0x25 [M SS_Create 0x1a9c58:52]) and both safe shells (glist flags 0 [D probe]) are all
//! tested. A surface-6 hit therefore survives every filter — the game refuses the doorway take at
//! facings whose body-sphere ray grazes the safe's own shell exactly like the port (the graze is
//! knife-edge: the ray origin is the yaw-rotated body sphere [L M_CONTROL+0x80]), and the player
//! collects by walking in, as these tests do through the real path.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str, level: &str) -> nf_view::scene::Scene {
    nf_view::load(iso, level, false).expect("load")
}

/// The safe-open chain from fix_safe08_live (fusebox → the Q-Laser burns → ch 136 → the door SS).
fn open_safe(scene: &mut nf_view::scene::Scene) {
    let sim = scene.sim.as_mut().unwrap();
    let fb = sim.drones.fuseboxes.iter().position(|f| f.ch[0] == 0x89).expect("the GuardRoom fusebox (ch 137) exists");
    let fp = sim.drones.fuseboxes[fb].pos;
    sim.bond.pos = [fp[0] - 1.2, fp[1], fp[2]];
    for _ in 0..5 { sim.step_bond(Input::default(), false, false, true); }
    for _ in 0..80 { sim.step_bond(Input::default(), false, false, false); }
    for _ in 0..5 { sim.step_bond(Input::default(), false, false, true); }
    for _ in 0..20 { sim.step_bond(Input::default(), false, false, false); }
    let input = Input { switch_to: Some(0x4e), ..Default::default() };
    for _ in 0..40 { sim.step_bond(input, false, false, false); }
    for out in [134u16, 135] {
        let bolt = sim.drones.locks.iter().position(|l| l.out == out).expect("the safe bolt exists");
        let bp = sim.drones.locks[bolt].pos;
        sim.bond.pos = [bp[0], bp[1] - 0.55, bp[2] + 1.5];
        for _ in 0..10 { sim.step_bond(Input::default(), false, false, false); }
        for _ in 0..200 {
            let head = sim.bond.head();
            let dir = [bp[0] - head[0], bp[1] - head[1], bp[2] - head[2]];
            let l = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt().max(1e-4);
            sim.bond.yaw = dir[0].atan2(dir[2]);
            sim.bond.pitch = (dir[1] / l).asin() / (std::f32::consts::FRAC_PI_2);
            sim.step_bond(Input::default(), true, false, false);
            if sim.drones.locks[bolt].burned { break }
        }
        assert!(sim.drones.locks[bolt].burned, "the watch burns bolt out {out}");
    }
    for _ in 0..60 { sim.step_bond(Input::default(), false, false, false); }
    assert!(sim.drones.channels.on(136), "both burns AND into ch 136 (mux #278) — the safe opens");
    // The safe door SS #205 glides to its open pose (t >= 6.9 of 8 keys [D #205 rate 60, count 8]).
    for _ in 0..520 {
        sim.step_bond(Input::default(), false, false, false);
        let t = sim.drones.sss.list.iter().find(|s| s.channel == 136).map(|s| s.t).unwrap_or(0.0);
        if t >= 6.9 { break }
    }
}

/// The shelf spawns exist and sit at the script's key positions after the open (the placement
/// gate: nothing at the shelves before ch 136 — fix_safe08_live's closed-safe sister covers the
/// negative side; this asserts the positive side's positions, from the placements).
#[test]
fn the_safe_spawns_sit_at_the_script_keys_after_the_open() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000008.bin");
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    open_safe(&mut scene);
    let sim = scene.sim.as_ref().unwrap();
    // The rocket: kind 0, id 0x2C, 4 rounds (the level-conditional override [M 0x18dda0] — the
    // disc's script param says 5 [D 0x060009E2 s1]), at the lower-shelf key.
    let rocket = sim.drones.pickups.iter().enumerate()
        .find(|(_, p)| !p.gone && p.kind == 0 && p.weapon == 0x2c
            && (p.centre()[0] - 162.59).abs() < 0.3 && (p.centre()[2] + 116.78).abs() < 0.3)
        .expect("the script-spawned rocket rests at the safe's lower shelf [D 0x060009E2 s1 keys]");
    assert_eq!(rocket.1.rounds, 4, "the 07000008 rocket override: 4 rounds whatever the script word [M 0x18dda0]");
    // The briefcase: kind 4 LABEL, id 6, switch 0x4A, at the top-shelf key. NOT a gadget pickup:
    // the briefcase gadget is weapon 0x52 on 0700000c/07000012 [M the Upgrade_Weapon tables]; this
    // pickup's id is 6 and its kind-4 take has no inventory effect [M 0x18e650 case 4].
    let case = sim.drones.pickups.iter().enumerate()
        .find(|(_, p)| !p.gone && p.kind == 4 && p.weapon == 6 && p.switch_channel == 0x4a
            && (p.centre()[0] - 162.7).abs() < 0.3 && (p.centre()[2] + 116.22).abs() < 0.3)
        .expect("the script-spawned briefcase rests at the safe's top shelf [D 0x060009E2 s2 keys]");
    assert_eq!(case.1.rounds, 0, "the kind-4 case carries no rounds");
}

/// The user's symptom, the game-faithful resolution: standing in the safe doorway after walking in
/// (the spot the capsule settles at), BOTH items collect — the briefcase through ch 0x4A, the
/// rocket into the inventory with the HUD "Picked up" message. The doorway ray grazes the safe's
/// own shell at some body-yaws (surface 6 survives every filter of the 0x725 query, header above),
/// so the body faces south here — the walk-in's leftover facing, like the game's.
#[test]
fn the_safe_pickups_collect_from_the_doorway() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000008.bin");
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    open_safe(&mut scene);
    let sim = scene.sim.as_mut().unwrap();
    sim.bond.pos = [161.87, 43.39, -116.54]; // the doorway (the user's spot, fix-safe08)
    sim.bond.vel = [0.0; 3];
    let (rockets_before, carried_before) = (
        sim.drones.inventory.owned.get(0x2c).copied().unwrap_or(false),
        sim.drones.inventory.carried.clone(),
    );
    let clip_before = sim.drones.inventory.clip.clone();
    for _ in 0..12 { sim.step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    let kinds: Vec<String> = sim.drones.taken.iter().map(|(_, t)| format!("{t:?}")).collect();
    assert!(kinds.iter().any(|k| k.contains("Weapon { weapon: 44 }")), "the rocket (0x2C) takes: {kinds:?}");
    assert!(kinds.iter().any(|k| k.contains("Label { label: 0 }")), "the briefcase takes as the label-0 kind 4: {kinds:?}");
    assert!(sim.drones.channels.on(0x4a), "the briefcase's take writes ch 0x4A [M 0x18e650:330-333]");

    // The inventory: the rocket is owned with a full clip (4 rounds — the override — against the
    // weapon's own clip size); the case added nothing (kind 4 has no inventory effect [M 0x18e650
    // case 4] — and it is NOT the 0x52 briefcase gadget).
    assert!(rockets_before == false, "the rocket is not owned before the take");
    let owned = sim.drones.inventory.owned.get(0x2c).copied().unwrap_or(false);
    assert!(owned, "the rocket is owned after the take");
    let clip = sim.drones.inventory.clip.get(0x2c).copied().unwrap_or(0);
    let clip_size = sim.weapons.clip_size(0x2c);
    assert_eq!(clip, clip_size as i16, "the take fills the rocket's clip ({clip_size}) [M Player_EquipWeapon 0x1a8a58]");
    let carried = sim.drones.inventory.carried.clone();
    let extra = 4i16.wrapping_sub(clip_size as i16);
    let t = sim.weapons.ammo_type(0x2c);
    for (k, (now, before)) in carried.iter().zip(&carried_before).enumerate() {
        let expected = if k == t as usize { before + extra.max(0) } else { *before };
        assert_eq!(*now, expected, "carried ammo type {k}: the take credits only the rocket's overflow (4 − clip {clip_size})");
    }
    // The view's gadget recharge (the laser watch / Q-Claw gates, scene.rs) tops those clips up
    // every tick — outside the take path; only the non-gadget slots must hold still.
    const RECHARGE: [usize; 6] = [0x4a, 0x4e, 0x33, 0x4c, 0x4f, 0x45];
    for (k, (now, before)) in sim.drones.inventory.clip.iter().zip(&clip_before).enumerate() {
        if k == 0x2c || RECHARGE.contains(&k) { continue; }
        assert_eq!(*now, *before, "clip {k} unchanged by the two takes");
    }
    // No auto-switch: the rocket's base is not in BestWeapon [M 0x1a6eb0 — the port keeps the held gun].
    let switched = sim.drones.weapon_switch_request.is_some();
    assert!(!switched, "the rocket never auto-switches [M 0x1a6eb0: the 0x2C base ranks -1]");

    // The HUD: the rocket queues the "Picked up <name>" message (0x200000D [M 0x18e650:243-247]);
    // the briefcase's label word is 0 → no message from its take [M 0x18e650 case 4].
    let lang = sim.lang.as_ref().expect("the language table is live headless");
    let name = lang.bind(sim.weapons.name_label(0x2c)).unwrap_or("");
    let expected = lang.format(0x200_000D, &[name.to_string()]);
    assert!(sim.hud.msgs.iter().any(|m| m.text == expected),
        "the HUD shows the picked-up message: {:?} vs [{expected}]", sim.hud.msgs.iter().map(|m| &m.text).collect::<Vec<_>>());
}

/// Walking collects: Bond walks forward through the safe mouth (the touch test runs every tick,
/// `Pickup_Update` 0x18e148's per-tick player walk — no button) and both items collect mid-walk.
#[test]
fn walking_through_the_safe_mouth_collects_both() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000008.bin");
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    open_safe(&mut scene);
    let walk = Input { forward: 1.0, ..Default::default() };
    let sim = scene.sim.as_mut().unwrap();
    sim.bond.pos = [161.87, 43.39, -116.54]; // the doorway
    sim.bond.vel = [0.0; 3];
    sim.bond.yaw = 0.0; // along the safe wall, past the mouth — the walk grazes both shelves
    for i in 0..6 {
        let before = sim.drones.taken.len();
        for _ in 0..5 { sim.step_bond(walk, false, false, false); }
        if sim.drones.taken.len() >= 2 { break; }
        let _ = i;
    }
    let sim = scene.sim.as_ref().unwrap();
    let kinds: Vec<String> = sim.drones.taken.iter().map(|(_, t)| format!("{t:?}")).collect();
    assert!(kinds.iter().any(|k| k.contains("Weapon { weapon: 44 }")), "the rocket takes during the walk: {kinds:?}");
    assert!(kinds.iter().any(|k| k.contains("Label { label: 0 }")), "the briefcase takes during the walk: {kinds:?}");
    assert!(sim.drones.channels.on(0x4a), "ch 0x4A on after the walk-through");
}

/// Sister: out of range does nothing. Bond waits in the bedroom mid-floor (~7 units from the
/// shelves, well outside the radius + 1.1 [M 0x18e148:203-210]), the safe open, the items live —
/// nothing takes, ch 0x4A stays off, the inventory is untouched.
#[test]
fn out_of_range_the_safe_pickups_do_nothing() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000008.bin");
    for _ in 0..120 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    open_safe(&mut scene);
    let sim = scene.sim.as_mut().unwrap();
    sim.bond.pos = [155.6, 43.35, -114.0]; // the bedroom mid-floor
    sim.bond.vel = [0.0; 3];
    let rocket_type = sim.weapons.ammo_type(0x2c) as usize;
    let rocket_clip_before = sim.drones.inventory.clip.get(0x2c).copied().unwrap_or(0);
    let rocket_carried_before = sim.drones.inventory.carried.get(rocket_type).copied().unwrap_or(0);
    let use_key = Input::default();
    for _ in 0..240 { sim.step_bond(use_key, false, false, true); } // mash E for four seconds
    let sim = scene.sim.as_ref().unwrap();
    assert!(sim.drones.taken.is_empty(), "nothing took from out of range: {:?}", sim.drones.taken);
    assert!(!sim.drones.channels.on(0x4a), "ch 0x4A stays off");
    assert!(sim.drones.pickups.iter().any(|p| !p.gone && p.kind == 0 && p.weapon == 0x2c
        && (p.centre()[0] - 162.59).abs() < 0.3), "the shelf rocket is still there");
    assert!(sim.drones.pickups.iter().any(|p| !p.gone && p.kind == 4 && p.switch_channel == 0x4a
        && (p.centre()[0] - 162.7).abs() < 0.3), "the shelf briefcase is still there");
    // Targeted: the rocket's own slots hold still (the gadget-recharge gates — the laser watch /
    // Q-Claw tops-up — are level gameplay outside the take path and are not asserted here).
    assert!(!sim.drones.inventory.owned.get(0x2c).copied().unwrap_or(false), "the rocket stays unowned out of range");
    assert_eq!(sim.drones.inventory.clip.get(0x2c).copied().unwrap_or(0), rocket_clip_before, "the rocket's clip slot untouched");
    assert_eq!(sim.drones.inventory.carried.get(rocket_type).copied().unwrap_or(0), rocket_carried_before, "the rocket's ammo pool untouched");
}

/// The disc-flags probe (NF_PICKUPS08_FLAGS=1, report evidence): the glist flags (+0x44 → the
/// object's obj+0xF0 base and the static parser's cel+0x94) of everything around the safe —
/// `Collide_PickObj` 0x1eae80 skips a dynamic object whose obj+0xF0 has 0x20 (or 0x1000 under the
/// take's 0x725 word) and `Collide_Pick` skips nodes whose cel+0x94 has 0x20; none of the safe's
/// objects carry any, so the game's take ray tests them all — the port's refusal from the
/// knife-edge facings is the game's too.
#[test]
fn probe_glist_flags() {
    if std::env::var_os("NF_PICKUPS08_FLAGS").is_none() { return; }
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let img = nf_formats::iso9660::IsoImage::open(&iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000008.bin")).expect("level entry");
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read archive");
    let level = nf_formats::level::Level::parse(&data).expect("parse level");
    let m = level.main_map().unwrap_or(0);
    for (i, p) in level.maps[m].placements.iter().enumerate() {
        if p.flags & 0xff != 0xe2 { continue; }
        let param = |k: u32| p.params.iter().find(|(j, _)| *j == k).map_or(0u32, |&(_, v)| v);
        let obj = if p.by_hash() { level.by_hash.get(&p.hashcode).and_then(|&(mm, oo)| Some(&level.maps[mm].objects[oo])) }
            else { level.maps[m].objects.get(p.index as usize) };
        if let Some(o) = obj {
            let ch = param(3);
            if ch != 0 || o.name.to_lowercase().contains("safe") || o.name.to_lowercase().contains("door") {
                eprintln!("[flags] SS placement {i} ch {ch} obj {} hash {:08x} glist_flags {:#x} (0x20={} 0x1000={} 0x40={})",
                    o.name, o.hashcode, o.flags, o.flags & 0x20 != 0, o.flags & 0x1000 != 0, o.flags & 0x40 != 0);
            }
        }
    }
    for (mi, map) in level.maps.iter().enumerate() {
        for (oi, o) in map.objects.iter().enumerate() {
            if o.name.to_lowercase().contains("safe") {
                eprintln!("[flags] map{mi} obj{oi} {} hash {:08x} flags {:#x} (0x20={} 0x1000={}) centre {:?} r {:.3}",
                    o.name, o.hashcode, o.flags, o.flags & 0x20 != 0, o.flags & 0x1000 != 0, o.centre, o.radius);
            }
        }
    }
}
