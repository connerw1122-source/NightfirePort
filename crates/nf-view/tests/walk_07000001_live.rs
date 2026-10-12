//! map-castle walkthrough: 07000001 "Paris Prelude" (Castle A) — the real path from the level's own
//! data. Run with: `NF_ISO=… cargo test -p nf-view --test walk_07000001_live -- --nocapture`
//!
//! The chain [D placements + scripts; M decompile addresses cited inline]:
//! 1. Bond spawns at start #0 (−2.78, 6.84, −12.40, gate 0) [D placement probe].
//! 2. The intro cut-scene SP #277 (script 0x06000087 from pack 07800001, mode 1) plays at level load;
//!    its stream 8 posts event 0xD ch 0xB = 1 [D script decode] — the contact's enable channel.
//! 3. The contact (drone #161, skin 0x0500004F at (−7.70, 6.42, −7.62)) waits on ch 0xB (placement
//!    param 3 [M NDrone2_DefaultInit 0x14b300 enable-channel read; the port's Drone::new switch_channel]).
//!    His DEATH raises ch 0x60 = the mission-fail channel [M Mission_MonitorObjectives 0x1827d8:104-108
//!    sets switch_channels[99(=0x63)] on ch 0x60; the death channel itself is drone param 12 = 0x60,
//!    Drone death_channel → set_death_channel 0x1461xx].
//! 4. Touch #180 "LoadPointActivator" (0xEB, out 0x10, mask 0x18 = player + NPC, filter skin 0x4F)
//!    at (−33.11, 2.88, 14.50) needs Bond AND the contact on its volume [M Trigger_Hit 0x1b1fc8].
//! 5. Exit #18 (0xE8 LOAD_LEVEL, next 0x07000002, gate 0x10) at (−33.15, 1.74, 17.15) fires when
//!    ch 0x10 is on [M Trigger_Update 0x1b1a78 case 10] → ResetMap_LevelToLoad [M Trigger_Activate
//!    0x1b2318:111-121].

use nf_formats::iso9660::IsoImage;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn read_archive(iso: &str, archive: &str) -> Vec<u8> {
    let img = IsoImage::open(iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(archive)).expect("archive");
    img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read")
}

/// The full route: intro cut-scene → the contact enables → Bond + contact on the LoadPointActivator →
/// ch 0x10 → the exit → 07000002 loads.
#[test]
fn intro_enables_the_contact_then_the_load_point_ends_the_level() {
    let iso_v = iso();
    let mut scene = nf_view::load(&iso_v, "07000001.bin", false).expect("load 07000001");
    let sim = scene.sim.as_mut().unwrap();
    assert_eq!(sim.level, 0x0700_0001);
    assert!(!sim.drones.channels.on(0xb), "ch 0xB is off at spawn [L ee_start_07000001.bin 0x26fc90: only 0x97/0x98 on]");
    // The intro cut-scene plays (no input — the player watches). Its stream 8 writes ch 0xB.
    let mut enabled_at = None;
    for t in 0..3000u32 {
        sim.step_bond(Default::default(), false, false, false);
        if sim.drones.channels.on(0xb) { enabled_at = Some(t); break; }
    }
    let enabled_at = enabled_at.expect("the intro script 0x06000087 must write ch 0xB (its stream-8 event 0xD)");
    eprintln!("[01] ch 0xB on at tick {enabled_at} (the intro cut-scene)");
    // The contact leaves the switch-channel wait.
    let npc_i = sim.drones.list.iter().position(|d| d.skin == 0x0500_004f).expect("the skin-0x4F contact exists");
    assert!(sim.drones.list[npc_i].switch_channel == 0xb);
    for _ in 0..10 { sim.step_bond(Default::default(), false, false, false); }
    assert!(sim.drones.list[npc_i].started, "the contact must leave WAIT_SWITCH once ch 0xB is on");
    let npc_pos = sim.drones.list[npc_i].pos;
    eprintln!("[01] contact started at {:?}", npc_pos);
    // The plate and the exit from the sim's own trigger build.
    let plate = sim.drones.triggers.iter().find(|t| t.kind == nf_game::triggers::kind::TOUCH && t.out == 0x10)
        .expect("the LoadPointActivator (Touch out 0x10, mask 0x18, filter skin 0x4F)");
    let centre = [(plate.lo[0] + plate.hi[0]) * 0.5, (plate.lo[1] + plate.hi[1]) * 0.5, (plate.lo[2] + plate.hi[2]) * 0.5];
    // Sister first: Bond ALONE on the plate must NOT fire ch 0x10 (the mask needs the NPC too).
    sim.bond.pos = centre;
    for _ in 0..6 { sim.step_bond(Default::default(), false, false, false); }
    assert!(!sim.drones.channels.on(0x10), "Bond alone must not fire the AND-touch");
    assert_eq!(sim.drones.level_exit, None);
    // Both on the plate: ch 0x10 → the exit fires.
    for i in 0..12 {
        {
            let sim = scene.sim.as_mut().unwrap();
            sim.bond.pos = centre;
            sim.drones.list[npc_i].pos = [centre[0] + 0.5, centre[1], centre[2]];
        }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        if scene.sim.as_ref().unwrap().drones.level_exit.is_some() { eprintln!("[01] level_exit at walk tick {i}"); break; }
    }
    assert_eq!(scene.sim.as_ref().unwrap().drones.level_exit, Some(0x0700_0002),
        "Bond + the contact on the plate must end the level to 07000002");
    // The reload path (the viewer's own next-level step).
    let next = nf_view::level_archive_name(0x0700_0002).to_string();
    let scene2 = nf_view::load(&iso_v, &next, false).expect("07000002 must load after the exit");
    assert_eq!(scene2.sim.as_ref().expect("sim").level, 0x0700_0002);
}

/// The failure condition: the contact's death channel is ch 0x60 — the mission-fail channel the game's
/// Mission_MonitorObjectives turns into switch_channels[0x63] ("mission failed") [M 0x1827d8:104-108].
#[test]
fn killing_the_contact_raises_the_mission_fail_channel() {
    let iso_v = iso();
    let mut scene = nf_view::load(&iso_v, "07000001.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..3000u32 { sim.step_bond(Default::default(), false, false, false); if sim.drones.channels.on(0xb) { break; } }
    assert!(sim.drones.channels.on(0xb));
    let npc_i = sim.drones.list.iter().position(|d| d.skin == 0x0500_004f).expect("the contact");
    let npc_pos = sim.drones.list[npc_i].pos;
    // Kill him through the game's own grenade path [M Bullet_CollisionHandler → Explode_Propagate
    // 0x17c518 → Drone_ExplosiveHit msg 9]: a grenade dropped at his feet explodes on impact.
    // (The bullet path lands on his hit boxes but the civilian-class damage application drops it — the
    // C2/015-fix-npcs gap, noted in the report.)
    sim.drones.grenades.push(nf_game::grenade::Grenade {
        pos: [npc_pos[0], npc_pos[1] + 0.5, npc_pos[2]], velocity: [0.0, 0.0, 0.0], speed: 0.0,
        travelled: 0.0, range: 60.0, damage: 100.0, radius: 3.0, weapon_type: 0x35, mesh: 0x0200_0726, owner: usize::MAX, state: 0, bounces: 0,
    });
    let mut dead = false;
    for _ in 0..300u32 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        if scene.sim.as_ref().unwrap().drones.list[npc_i].corpse { dead = true; break; }
    }
    assert!(dead, "the explosion must kill the contact (health {:?})",
        scene.sim.as_ref().unwrap().drones.list[npc_i].health);
    // The death channel raises when the death anim exits (DEATH_ANIM msg::EXIT → set_death_channel
    // 0x13e660) — a few dozen ticks after the corpse appears.
    let mut raised = false;
    for _ in 0..300u32 {
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        if scene.sim.as_ref().unwrap().drones.channels.on(0x60) { raised = true; break; }
    }
    assert!(raised, "the contact's death channel 0x60 (drone param 12) = the mission-fail channel [M 0x1827d8]");
    // Sister: the mission-fail channel is what Mission_Update 0x182c60 case 1 turns into the failed
    // state via switch_channels[0x63]; assert nothing cleared it.
    for _ in 0..30 { scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false); }
    assert!(scene.sim.as_ref().unwrap().drones.channels.on(0x60));
}
