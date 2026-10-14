//! fix-objects repro tests, driven through the real path (`nf_view::load` + `Sim::step_bond`).
//!
//! The user's reports (master 0a2fcc4, screenshots on 07000001):
//! 1. TOP: "Where an armour pickup belongs (on a small chest of drawers), a spinning gold 007 bonus
//!    model is drawn" and "a dead guard's drop shows a 007 bonus icon on the floor where it should be
//!    ammunition" — the drawn model and the pickup at a spot disagree.
//! 2. "Every dead guard drops several objects" on all enemies (the sim drops one — fix-player3 pinned
//!    that — so the extra objects must be drawn ones).
//! 3. Two placed piles swapped: an ammo pile behaves as armour, an armour piece as ammo.
//!
//! Opt-in via `NF_ISO`.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn load(iso: &str, level: &str) -> nf_view::scene::Scene {
    let mut scene = nf_view::load(iso, level, false).expect("load");
    // fix-cutscenes: 07000001/07000002 now play their level-start intro (Bond disabled, drones frozen). Skip it
    // the game's way (hold the skip: `Script_FFwd` 0x1dac78), as fix_player2_live does, so the tests start in control.
    let intro: &[u32] = match level { "07000001.bin" => &[0x0600_0087], "07000002.bin" => &[0x0600_0084], _ => &[] };
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

fn tick_idle(sim: &mut nf_view::scene::Sim, n: u32) {
    for _ in 0..n { sim.step_bond(Default::default(), false, false, false); }
}

/// ── 1: the draw-slot identity ───────────────────────────────────────────────────────────────────
/// The renderer draws placed pickup slot `k` with the mesh of build-time `placed_pickups[k]`
/// (`level.by_hash.get(&p.model)` → object) and poses it at `Sim::drones.pickups[k].m` every tick
/// (scene.rs `add_instance` + the pickup branch of the pose step; `View_DrawObjectGlist` 0x1e6780
/// draws each pickup object's OWN `obj+0xD8` at its OWN `obj+0x90` [M]). The game never reorders a
/// live pickup under a drawn instance, so the mesh↔matrix pair is an identity. The port's
/// `update_pickups` ended with `self.pickups.retain(|p| !p.gone)`: the first take (or lifetime
/// expiry) shifts every later index, and every placed mesh after it is drawn at another pickup's or
/// a drop's matrix — the user's "007 model at the armour spot" and "wrong model at the drop".
#[test]
fn pickup_slots_keep_their_pickup_after_a_take() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000001.bin");
    let sim = scene.sim.as_mut().unwrap();
    tick_idle(sim, 2);
    let placed: Vec<usize> = sim.drones.pickups.iter().enumerate()
        .filter(|(_, p)| p.from.is_none()).map(|(i, _)| i).collect();
    assert!(placed.len() >= 4, "07000001 has placed pickups (disc: 6 type-0xF0 placements)");
    let before: Vec<(u32, [f32; 4])> = placed.iter()
        .map(|&k| (sim.drones.pickups[k].model, sim.drones.pickups[k].m[3])).collect();
    // Take the first placed pickup through the real path: stand Bond on it and tick.
    let at = before[0].1;
    let floor = sim.world.floor_below([at[0], at[1] + 1.0, at[2]], 3.0).unwrap_or(at[1]);
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [at[0], floor + 0.5, at[2]], 0.0);
    let mut taken = false;
    for _ in 0..120 {
        sim.step_bond(Default::default(), false, false, false);
        if !sim.drones.taken.is_empty() { taken = true; break; }
    }
    assert!(taken, "Bond standing on a placed pickup takes it (Pickup_Update touch test)");
    // The identity the renderer draws by: the list does not shrink (the taken pickup stays as a
    // tombstone), so slot k still holds pickup k's mesh AND matrix. The taken slot is collapsed
    // (gone/hidden), the others unchanged.
    assert_eq!(sim.drones.pickups.len(), before.len(),
        "a taken pickup stays in place (the renderer's draw slots index this list; removing it shifts every later mesh)");
    let after_gone: Vec<bool> = placed.iter().map(|&k| sim.drones.pickups[k].gone || sim.drones.pickups[k].hidden).collect();
    assert!(after_gone.iter().any(|&g| g), "the taken pickup is marked gone in its own slot");
    for (n, &k) in placed.iter().enumerate() {
        let p = &sim.drones.pickups[k];
        assert_eq!(p.model, before[n].0,
            "slot {k} still carries its own mesh after a take (the drawn model at a spot must not change)");
        if !after_gone[n] {
            assert_eq!(p.m[3], before[n].1,
                "slot {k} still poses at its own matrix after a take");
        }
    }
}

/// ── 2: drawn model matches kind, every pickup on 07000001 and 07000005 ──────────────────────────
/// For placed pickups the game draws the placement's own celglist (`Pickup_Create` param_4 from
/// `parsemap_create_dynamic_objects` case 0xf0 0x1d04f0:459-462), whose object name follows the kind:
/// armour piles are "…BodyArmour…", ammo piles "…Ammo…", the bonus "Pickup 007". A weapon pile's
/// mesh is the weapon's own model (`weapon_data[id]+0x80`). This pins every placed pickup the two
/// levels spawn, plus `Pickup_CreateSimple`'s drops (mesh = `weapon_data[id]+0x80` [M 0x18f348:45]).
#[test]
fn placed_pickup_models_match_their_kind() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for (level, min) in [("07000001.bin", 4), ("07000005.bin", 3)] {
        let scene = load(&iso, level);
        let sim = scene.sim.as_ref().unwrap();
        let level_arc = level_archive(&iso, level);
        let placed: Vec<&nf_game::pickup::Pickup> = sim.drones.pickups.iter()
            .filter(|p| p.from.is_none()).collect();
        assert!(placed.len() >= min, "{level}: {min}+ placed pickups, got {}", placed.len());
        let mut checked = 0;
        for p in &placed {
            let name = drawn_object_name(&level_arc, p);
            match p.kind {
                nf_game::pickup::kind::ARMOUR => assert!(name.to_lowercase().contains("armour"),
                    "{level}: kind-3 pickup draws {name}"),
                nf_game::pickup::kind::AMMO => assert!(name.to_lowercase().contains("ammo"),
                    "{level}: kind-1 pickup draws {name}"),
                nf_game::pickup::kind::BOND_BONUS => assert!(name.contains("007"),
                    "{level}: kind-6 pickup draws {name}"),
                nf_game::pickup::kind::WEAPON => {
                    // The placement's mesh is the weapon's own model (`weapon_data[id]+0x80`); the
                    // levels place the same object the table names.
                    assert_eq!(p.model, sim.weapons.model(p.weapon as usize),
                        "{level}: kind-0 pickup (weapon {:#x}) draws the weapon's model, not {name}", p.weapon);
                }
                _ => {}
            }
            checked += 1;
        }
        assert!(checked >= min);
        // Drops (`Pickup_CreateSimple` 0x18f348): the mesh is `weapon_data[id]+0x80` [M].
        let m4 = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [1.0, 2.0, 3.0, 1.0]];
        let p = nf_game::pickup::Pickup::create_simple(&m4, 6, 30, nf_game::pickup::kind::AMMO,
            &sim.weapons, &nf_game::pickup::models(&level_arc)).expect("ammo drop");
        assert_eq!(p.model, sim.weapons.model(6), "a drop of weapon 6 draws weapon_data[6]+0x80");
    }
}

/// ── 3: the two swapped piles on 07000001 ─────────────────────────────────────────────────────────
/// The disc (nf-extract placements f0): two "PICKUP_Ammo9 (MP5 / Storm)" piles at
/// (−3.25, 5.85, −9.06) and (−6.52, 5.85, −9.06) with params kind 1, id 6, rounds 20; two
/// "PICKUP_BodyArmourHeavy" at (17.44, 6.25, −3.16) and (−14.36, 6.51, 16.06) with kind 3, id 0x47
/// (0x47 is in `Pickup_CreateSimple`'s refuse list — placed armour is a different path), rounds 50.
/// The port must give each pile its disc kind AND its own mesh (the user's swap: an ammo pile acted
/// as armour and an armour piece as ammo).
#[test]
fn the_two_piles_on_07000001_have_their_disc_kinds_and_meshes() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000001.bin");
    let sim = scene.sim.as_mut().unwrap();
    tick_idle(sim, 2);
    let level = level_archive(&iso, "07000001.bin");
    let close = |a: [f32; 3], b: [f32; 3]| (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2) < 0.25;
    let spots: [([f32; 3], u16, u16, i16, &str); 4] = [
        ([-3.2548778, 5.8473516, -9.06046], 1, 6, 20, "Ammo9"),
        ([-6.5186343, 5.8473516, -9.060459], 1, 6, 20, "Ammo9"),
        ([17.441, 6.249076, -3.1553688], 3, 0x47, 50, "BodyArmour"),
        ([-14.36317, 6.510154, 16.06305], 3, 0x47, 50, "BodyArmour"),
    ];
    for (spot, kind, id, rounds, mesh) in spots {
        let p = sim.drones.pickups.iter().find(|p| close([p.m[3][0], p.m[3][1], p.m[3][2]], spot))
            .unwrap_or_else(|| panic!("a placed pickup at {spot:?}"));
        assert_eq!(p.kind, kind, "the pile at {spot:?} has its disc kind [D placement params]");
        assert_eq!(p.weapon, id, "the pile at {spot:?} has its disc id");
        assert_eq!(p.rounds, rounds, "the pile at {spot:?} has its disc rounds");
        let name = drawn_object_name(&level, p).to_lowercase();
        assert!(name.contains(&mesh.to_lowercase()), "the pile at {spot:?} draws {name}, not a {mesh} mesh");
    }
}

/// ── 4: the afc60ed regression guard ──────────────────────────────────────────────────────────────
/// A merge once dropped the placed-pickup `CharVertex` push in `scene.rs::add_instance`; every later
/// dynamic vertex was then posed with the wrong record (afc60ed fixed it). Guard: the dynamic-vertex
/// tail pairs 1:1 with `char_verts`, and every placed pickup, door and searchlight instance
/// contributes its own records — on 07000001 and 07000005.
#[test]
fn dynamic_vertex_records_match_their_instances() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for level in ["07000001.bin", "07000005.bin"] {
        let scene = load(&iso, level);
        let dyn_len = scene.vertices.len() - scene.char_vertex_start;
        assert_eq!(scene.char_verts.len(), dyn_len,
            "{level}: every dynamic vertex carries exactly one CharVertex record (the afc60ed pairing)");
        let sim = scene.sim.as_ref().unwrap();
        let placed = sim.drones.pickups.iter().filter(|p| p.from.is_none()).count();
        // Each placed pickup's instance pushed vertices — every slot index appears at least once.
        for k in 0..placed {
            assert!(scene.char_verts.iter().any(|cv| cv.pickup == Some(k as u16)),
                "{level}: placed pickup slot {k} has drawn vertices");
        }
        let with_pickup = scene.char_verts.iter().filter(|cv| cv.pickup.is_some()).count();
        assert!(with_pickup >= placed, "{level}: pickup records ({with_pickup}) cover the placed pickups ({placed})");
        // Doors and searchlights likewise: every index is drawn.
        for d in 0..sim.drones.doors.list.len() {
            assert!(scene.char_verts.iter().any(|cv| cv.door == Some(d as u16)), "{level}: door {d} has vertices");
        }
        for s in 0..sim.drones.searchlights.parts.len() {
            assert!(scene.char_verts.iter().any(|cv| cv.searchlight == Some(s as u16)),
                "{level}: searchlight part {s} has vertices");
        }
        // Sister: an impossible slot (beyond the list) must not be referenced by any record.
        assert!(scene.char_verts.iter().all(|cv| cv.pickup.map_or(true, |k| (k as usize) < sim.drones.pickups.len())),
            "{level}: pickup records only reference live slots");
    }
}

/// ── 5: the death-drop dump ───────────────────────────────────────────────────────────────────────
/// "Every dead guard drops several objects (the user sees it on all enemies)". The game's death
/// creates exactly ONE pickup (the base weapon, kind 0 — `DroneWeap_DropWeapon` 0x17a2a0), plus the
/// hidden 0x1B twin for 0x1A and a captain's grenade pickup (`DroneFunc_DropGrenade` 0x14b158) —
/// fix-player3 pinned the sim side. The extra VISIBLE objects the user saw were the placed meshes
/// drawn at shifted matrices (the `retain` desync, test 1). This walks the real path: kill a guard,
/// dump every pickup (kind, id, rounds, hidden, gone, drawn slot), and assert the drawn set is
/// exactly: every live placed pickup at its OWN matrix, plus the drop at the corpse.
#[test]
fn a_death_draws_its_drop_and_nothing_else_moved() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000001.bin");
    let sim = scene.sim.as_mut().unwrap();
    tick_idle(sim, 2);
    let placed_n = sim.drones.pickups.iter().filter(|p| p.from.is_none()).count();
    let placed_before: Vec<([f32; 4], u32)> = (0..placed_n)
        .map(|k| (sim.drones.pickups[k].m[3], sim.drones.pickups[k].model)).collect();
    let guards: Vec<usize> = sim.drones.list.iter().enumerate()
        .filter(|(_, dr)| dr.present && dr.enabled && dr.drawn && !dr.corpse && dr.health > 0.0 && (2..0x47).contains(&dr.weapon))
        .map(|(i, _)| i).collect();
    assert!(!guards.is_empty(), "07000001 starts armed guards");
    let gi = guards[0];
    let gp = sim.drones.list[gi].pos;
    let floor = sim.world.floor_below(gp, 6.0).unwrap_or(gp[1]);
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [gp[0], floor + 1.1, gp[2] - 3.0], 0.0);
    let e = sim.bond.head();
    let d = [gp[0] - e[0], gp[1] + 0.3 - e[1], gp[2] - e[2]];
    let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-6);
    sim.bond.yaw = d[0].atan2(d[2]);
    sim.bond.pitch = (d[1] / l).clamp(-1.0, 1.0).asin() / (std::f32::consts::FRAC_PI_2);
    tick_idle(sim, 10);
    // The guard's START SCRIPT (07000001's first guard plays 06000169 via PlayScript 3 [M 0x15ba68
    // ENTER, `+0x554` = param 1]) walks him 6.8 units over its 181 frames [D nf-extract script-info:
    // the root track runs 0.05 → 6.87], and the game's own recording shows him moving inside the
    // script [L 01-doors: the guard drifts x −4.919 → −6.03 within the first 120 lines, state still 3].
    // A one-shot aim at his pre-script position goes stale, so Bond tracks him (the player's own
    // behavior) while firing.
    for t in 0..600 {
        let g = sim.drones.list[gi].pos;
        let e = sim.bond.head();
        let d = [g[0] - e[0], g[1] + 0.3 - e[1], g[2] - e[2]];
        let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-6);
        sim.bond.yaw = d[0].atan2(d[2]);
        sim.bond.pitch = (d[1] / l).clamp(-1.0, 1.0).asin() / (std::f32::consts::FRAC_PI_2);
        sim.step_bond(Default::default(), t % 9 == 0, false, false);
        if sim.drones.list[gi].health <= 0.0 { break; }
    }
    assert!(sim.drones.list[gi].health <= 0.0, "the guard died");
    // fix-guard-weapons2: the guard fire path now consumes the game's per-shot RNG draws
    // (FireWeapon 0x179234's light-orientation FRand x3 + the Rand(500) pitch, NextBulletTime
    // 0x179d40's burst sizing), so the guard dies a few ticks differently and the corpse — the
    // drop's spawn point — can land within `Pickup_Update`'s touch radius (bounds.radius + 1.1
    // [M 0x18e...]) of where this harness parked Bond: the game then auto-takes the drop
    // (walk-over pickup) and the assertion below would see `gone`. Step Bond clear of the corpse
    // before the fall — the drop's existence/visibility is what's under test, not Bond's position.
    {
        let at = sim.drones.list[gi].pos;
        sim.bond.pos = [at[0] + 10.0, at[1], at[2]];
    }
    tick_idle(sim, 90);
    // The dump (kind, id, rounds, hidden, gone, from, position).
    let mut drawn_drops = 0;
    for (k, p) in sim.drones.pickups.iter().enumerate() {
        let drawn = if k < placed_n { !p.gone && !p.hidden } else { p.from == Some(gi) && !p.hidden && !p.gone };
        eprintln!("[death-dump] slot {k} kind {} id {:#x} rounds {} hidden {} gone {} from {:?} at ({:.2}, {:.2}, {:.2}) drawn {}",
            p.kind, p.weapon, p.rounds, p.hidden, p.gone, p.from, p.m[3][0], p.m[3][1], p.m[3][2], drawn);
        if k >= placed_n && p.from == Some(gi) && !p.hidden && !p.gone { drawn_drops += 1; }
    }
    // Placed slots unchanged by the death: every live placed pickup still at its own matrix.
    for k in 0..placed_n {
        let p = &sim.drones.pickups[k];
        if !p.gone && !p.hidden {
            assert_eq!((p.m[3], p.model), placed_before[k], "placed slot {k} unmoved after a death");
        }
    }
    // Exactly one visible drop for this death (kind 0 weapon; fix-player3's pin, re-driven here).
    let drops: Vec<&nf_game::pickup::Pickup> = sim.drones.pickups.iter()
        .filter(|p| p.from == Some(gi)).collect();
    assert_eq!(drawn_drops, 1, "one visible drop at the corpse: {:?}",
        drops.iter().map(|p| (p.kind, p.weapon, p.hidden, p.gone)).collect::<Vec<_>>());
}

/// ── 6: the 007 bonus take ────────────────────────────────────────────────────────────────────────
/// "The 007 bonus item appears (good), but picking it up does nothing." The game's take runs
/// `PlrStat_LogBondBonus()` and the shared tail: message 0x200004A and sound 0x14C — `Pickup_Handler`
/// overwrites the pickup's sound with the per-kind 0x14C before `Sound_Play3D` [M 0x18e650:319-321,
/// :234-236]; a placed bonus's create sound is 0xFFFF (none). The port consumed the bonus silently.
#[test]
fn taking_the_007_bonus_logs_and_plays_the_take_sound() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = load(&iso, "07000001.bin");
    let sim = scene.sim.as_mut().unwrap();
    tick_idle(sim, 2);
    // The bonus pile at (−13.24, 5.63, 10.35) [D placement 315, kind 6 id 6, sound param 65535].
    let close = |a: [f32; 3], b: [f32; 3]| (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2) < 0.25;
    let k = sim.drones.pickups.iter().position(|p| p.kind == nf_game::pickup::kind::BOND_BONUS
        && close([p.m[3][0], p.m[3][1], p.m[3][2]], [-13.243799, 5.629803, 10.35467]))
        .expect("the placed 007 bonus");
    assert_eq!(sim.drones.pickups[k].sound, None, "the bonus is placed with sound 0xFFFF [D]");
    let at = sim.drones.pickups[k].m[3];
    let floor = sim.world.floor_below([at[0], at[1] + 1.0, at[2]], 3.0).unwrap_or(at[1]);
    sim.bond = nf_game::player::Bond::stand_at(&sim.world, [at[0], floor + 0.5, at[2]], 0.0);
    let mut what = None;
    for _ in 0..120 {
        sim.step_bond(Default::default(), false, false, false);
        if let Some((_, t)) = sim.drones.taken.first() { what = Some(*t); break; }
    }
    assert_eq!(what, Some(nf_game::pickup::Taken::Bonus), "the take is a logged Bonus, not a silent consume");
    assert!(sim.drones.pickups[k].gone, "the bonus pickup is consumed");
}

fn level_archive(iso: &str, level: &str) -> nf_formats::level::Level {
    use nf_formats::{disc, iso9660::IsoImage, level::Level};
    let image = IsoImage::open(iso).unwrap();
    let (entries, bin) = disc::files_bin_entries(&image).unwrap();
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(level)).unwrap();
    let data = image.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    Level::parse(&data).unwrap()
}

/// The name of the object the renderer draws for a pickup: `p.obj` (the resolved object — the game
/// draws the celglist handed to `Pickup_Create`), falling back to the model hashcode.
fn drawn_object_name(level: &nf_formats::level::Level, p: &nf_game::pickup::Pickup) -> String {
    let target = p.obj.or_else(|| level.by_hash.get(&p.model).copied());
    target.map(|(om, oi)| level.maps[om].objects[oi].name.trim().to_string())
        .unwrap_or_else(|| format!("<unresolved {:#x}>", p.model))
}
