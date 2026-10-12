//! fix-level probe: dump 07000001's trigger-family placements (the level-end door) and check the
//! 07000005 → 07000006 reload path headless. Run with:
//! `NF_ISO=… cargo test -p nf-view --test level_reload_live -- --nocapture`

use nf_formats::level::Level;
use nf_formats::levelbin::LevelBin;
use nf_formats::iso9660::IsoImage;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn read_archive(iso: &str, archive: &str) -> Vec<u8> {
    let img = IsoImage::open(iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(archive)).expect("archive");
    img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read")
}

#[test]
fn probe_07000001_trigger_placements() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let data = read_archive(&iso, "07000001.bin");
    let level = Level::parse(&data).expect("parse");
    let m = level.main_map().expect("main map");
    for (i, p) in level.maps[m].placements.iter().enumerate() {
        let f = p.flags;
        if (0xd0..=0xff).contains(&f) || f == 0x29 {
            let params: Vec<String> = p.params.iter().take(14).map(|(k, v)| format!("{k}={v:#x}")).collect();
            eprintln!("placement #{i} flags {f:#x} at ({:.1},{:.1},{:.1}) obj {} params [{}]",
                p.translation[0], p.translation[1], p.translation[2], p.index, params.join(", "));
        }
    }
    // Also the placement count per flags value for context.
    let mut counts: std::collections::BTreeMap<u32, usize> = Default::default();
    for p in &level.maps[m].placements { *counts.entry(p.flags).or_default() += 1; }
    eprintln!("flags counts: {:?}", counts.iter().map(|(k, v)| format!("{k:#x}:{v}")).collect::<Vec<_>>());
}

/// Bug 3 repro: 07000001's level-end door (placement #18, 0xE8 LOAD_LEVEL → 0x07000002, gate 0x10) does
/// nothing when Bond presses E on it. Show what the port built for that trigger.
#[test]
fn probe_07000001_load_level_trigger() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let scene = nf_view::load(&iso, "07000001.bin", false).expect("load 07000001");
    let sim = scene.sim.as_ref().expect("sim");
    let lls: Vec<&nf_game::triggers::Trigger> = sim.drones.triggers.iter()
        .filter(|t| t.kind == nf_game::triggers::kind::LOAD_LEVEL).collect();
    eprintln!("LOAD_LEVEL triggers on 07000001: {}", lls.len());
    for t in &lls {
        eprintln!("  placement {} next {:#x} gate {:#x} key {:#x} tris {} lo {:?} hi {:?}",
            t.placement, t.next_level, t.gate, t.key_gate, t.tris.len(), t.lo, t.hi);
    }
    // Bond at the trigger's box centre, press E.
    let t = lls.first().expect("a LOAD_LEVEL trigger exists");
    let c = [(t.lo[0] + t.hi[0]) * 0.5, (t.lo[1] + t.hi[1]) * 0.5, (t.lo[2] + t.hi[2]) * 0.5];
    eprintln!("trigger centre {:?}", c);
    let mut scene = scene;
    let sim = scene.sim.as_mut().unwrap();
    sim.bond.pos = [c[0], c[1], c[2] - 0.5];
    sim.bond.yaw = 0.0;
    sim.step_bond(Default::default(), false, false, true);
    eprintln!("after E: level_exit {:?}", sim.drones.level_exit);

    // Why is the volume empty? Inspect the target object (placement #18 → obj 11) and the placement's
    // hash/index resolution.
    let data = read_archive(&iso, "07000001.bin");
    let level = Level::parse(&data).expect("parse");
    let m = level.main_map().unwrap();
    let p = &level.maps[m].placements[18];
    eprintln!("placement 18: by_hash {} hash {:#x} index {}", p.by_hash(), p.hashcode, p.index);
    if let Some(oi) = level.by_hash.get(&p.hashcode) { eprintln!("by_hash → map {} obj {}", oi.0, oi.1); }
    for om in 0..level.maps.len() {
        for oi in 0..level.maps[om].objects.len() {
            let o = &level.maps[om].objects[oi];
            if om == m && oi == 11 {
                eprintln!("main-map obj 11: name {:?} flags {:#x} has collision {}", o.name.trim(), o.flags, o.collision.is_some());
                if let Some(c) = &o.collision { eprintln!("  collision tris: {}", c.triangles().len()); }
            }
        }
    }
    let counts: Vec<usize> = scene.sim.as_ref().unwrap().drones.triggers.iter().map(|t| t.tris.len()).collect();
    eprintln!("all trigger tri counts on 07000001: {:?}", counts);
    // The level-end Touch (placement 180, out 0x10): volume, lock, filters. And the drones' skins.
    for t in &scene.sim.as_ref().unwrap().drones.triggers {
        if t.kind == nf_game::triggers::kind::TOUCH && t.out == 0x10 {
            eprintln!("level-end touch: placement {} tris {} lo {:?} hi {:?} mask {:#x} list {:?} lock {:#x}",
                t.placement, t.tris.len(), t.lo, t.hi, t.mask, t.list, t.lock);
        }
    }
    for d in &scene.sim.as_ref().unwrap().drones.list {
        eprintln!("drone placement {} skin {:#x} present {} wait_ch {:#x}", d.placement, d.skin, d.present, d.switch_channel);
    }
    // Who writes ch 0x10 (the LOAD_LEVEL gate)? Scan the level's type-7 scripts for channel events.
    for (h, sc) in &scene.sim.as_ref().unwrap().scripts {
        for c in &sc.commands {
            // event 0xD [p0=ch, p1=v]; event 0x11 [a,b,ch,v] with the else-branch writing ch=v.
            if c.op == 0xd && !c.args.is_empty() && c.args[0] == 0x10 {
                eprintln!("script {h:08x}: event 0xD args {:?} sets ch 0x10", c.args);
            }
            if c.op == 0x11 && c.args.len() >= 4 && c.args[2] == 0x10 {
                eprintln!("script {h:08x}: event 0x11 else-branch args {:?} touches ch 0x10", c.args);
            }
        }
    }
    // What is obj 11 "Load Point"? Its mesh + where its world matrices land.
    let o = &level.maps[m].objects[11];
    eprintln!("obj 11 meshes: {} leaves: {}", o.gfx.leaves.len(), o.gfx.leaves.iter().map(|l| l.batches.len()).sum::<usize>());
    let any_script_ref = scene.sim.as_ref().unwrap().scripts.keys().any(|k| *k == 0x06000074);
    eprintln!("script 0x06000074 present: {any_script_ref}");
}

/// Bug 3 end-to-end: the 07000001 level-end chain [D placements 180 + 18]:
/// Touch #180 (out 0x10, mask 0x18, filters [0xE player, skin 0x0500004F]) + the skin-0x4F NPC
/// (drone placement 161) both in the plate volume → ch 0x10 → LOAD_LEVEL #18 (gate 0x10) →
/// level_exit 0x07000002. The NPC waits for ch 0xB [L ee_start_07000001: +0x134 = 0xB, state 1 =
/// disabled]; no disc writer for 0xB exists (placements + type-5 + type-7 scripts scanned) — the
/// enable is the un-ported mission layer's job, so the test sets it (the game's enable state).
/// Sister: Bond on the plate WITHOUT the NPC → the AND-chain refuses → no level end.
#[test]
fn level_end_07000001_needs_bond_and_the_0x4f_npc_on_the_plate() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000001.bin", false).expect("load 07000001");
    let sim = scene.sim.as_mut().unwrap();
    // The plate volume (from the build): put Bond inside it.
    let touch = sim.drones.triggers.iter().find(|t| t.kind == nf_game::triggers::kind::TOUCH && t.out == 0x10)
        .expect("the level-end touch (placement 180) exists").placement;
    let (cx, cy, cz) = {
        let t = sim.drones.triggers.iter().find(|t| t.placement == touch).unwrap();
        ((t.lo[0] + t.hi[0]) * 0.5, (t.lo[1] + t.hi[1]) * 0.5, (t.lo[2] + t.hi[2]) * 0.5)
    };
    sim.bond.pos = [cx, cy, cz];
    sim.bond.yaw = 0.0;
    // Sister first: Bond alone → no ch 0x10.
    for _ in 0..5 { sim.step_bond(Default::default(), false, false, false); }
    assert!(!sim.drones.channels.on(0x10), "Bond alone must not fire the AND-chain touch");
    assert_eq!(sim.drones.level_exit, None);
    // The game's NPC enable (ch 0xB — the mission layer's write, unported), then Bond + NPC on the plate.
    sim.drones.set_channel(0xb);
    for i in 0..10 {
        {
            let sim = scene.sim.as_mut().unwrap();
            sim.bond.pos = [cx, cy, cz];
            let npc = sim.drones.list.iter_mut().find(|d| d.skin == 0x0500_004f).expect("the skin-0x4F NPC exists");
            npc.pos = [cx + 0.5, cy, cz];
        }
        scene.sim.as_mut().unwrap().step_bond(Default::default(), false, false, false);
        if scene.sim.as_ref().unwrap().drones.level_exit.is_some() { eprintln!("[07000001] level_exit at tick {i}"); break; }
    }
    assert_eq!(scene.sim.as_ref().unwrap().drones.level_exit, Some(0x0700_0002),
        "Bond + the enabled skin-0x4F NPC on the plate must set ch 0x10 and fire the LOAD_LEVEL (next 0x07000002)");
}

/// Probe: scan the level's type-5 (drone) scripts for channel events — who writes ch 0xB (the NPC's
/// enable) and ch 0x10 (the level-end gate)?
#[test]
fn probe_07000001_type5_script_channel_events() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let data = read_archive(&iso, "07000001.bin");
    let lb = LevelBin::parse(&data).expect("levelbin");
    for e in lb.entries.iter().filter(|e| e.kind == 5) {
        if let Ok(s) = nf_formats::anim::Script::parse(lb.data(&data, e)) {
            let mut writes: Vec<String> = Vec::new();
            for c in &s.commands {
                // event 0xD [ch, v]; event 0x11 [a, b, ch, v].
                if c.op == 0xd && !c.args.is_empty() { writes.push(format!("0xD ch {:#x}={:?}", c.args[0], c.args.get(1))); }
                if c.op == 0x11 && c.args.len() >= 4 { writes.push(format!("0x11 ch {:#x}={}", c.args[2], c.args[3])); }
            }
            if !writes.is_empty() { eprintln!("drone script {:#x}: {}", s.hashcode, writes.join("; ")); }
        }
    }
}

/// Probe: dump the truck script's (0x06000A22) entity streams — the TouchOnce/Touch triggers it creates
/// (ch 0x70 parked plate, ch 0x71 truck bed, ch 0x5C gatehouse) and their masks.
#[test]
fn probe_07000005_truck_script_entities() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let data = read_archive(&iso, "07000005.bin");
    let level = Level::parse(&data).expect("parse");
    for (h, nis) in &level.nis {
        if *h != 0x0600_0A22 { continue; }
        for (si, st) in nis.streams.iter().enumerate() {
            for cmd in st.decode() {
                if let nf_formats::nis::Cmd::Entity { hash, obj_type, params, parent, spline } = &cmd {
                    eprintln!("stream {si}: entity hash {hash:08x} type {obj_type:#x} parent {parent:?} spline {spline} params {:x?}", params.iter().take(10).collect::<Vec<_>>());
                }
                if let nf_formats::nis::Cmd::Event { kind, params } = &cmd {
                    eprintln!("stream {si}: event {kind:#x} params {:x?}", params);
                }
            }
        }
    }
}

/// Bug 2 repro: the user's play — walk to the gatehouse plate, ride the truck to the castle, park with
/// Bond in the back. The level must end (ch 0x70+0x71+0x72 → mux 0x65 → SP #320 → 0x1D → LOAD_LEVEL 118
/// → 0x07000006). Traces the chain every 100 ticks.
///
/// FIXED (fix-truck): two things were wrong. (1) The harness gap — the test assigned `sim.bond.pos`
/// on the level-start Bond, while the viewer's `--bond` path (which does trigger the gatehouse
/// cut-scene) replaces the whole Bond with `Bond::new` at the spot; `Bond::new` is the setup the game
/// matches (a clean standing Bond), so the test now uses it. (2) The ride parks Bond 0.49 m too high
/// (the port's fixed standing capsule vs the game's stance-driven `obj+0xCC = obj+0x3C` [M
/// Player_Update 0x193600:449]) so the ch 0x70 plate at y 37.7418 [D script stream 6 key 8] never
/// fired — the trace's ride is CROUCHED (park pos−floor 0.607 vs the standing 1.086), and the port
/// now models the crouched capsule height (CROUCH_ROOT_HEIGHT, crates/nf-game/src/player.rs), which
/// parks Bond at 37.93 and lets the touch reach the plate. The test crouches Bond as the hand-off
/// re-enables him, like the trace's play. `fix_truck_live.rs` holds the full chain + park assertions.
#[test]
fn truck_ride_to_the_castle_ends_the_level() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    // Bond at the gatehouse plate (98.49, 27.05, 106.36 [D U-048]); the cut-scene hands him onto the truck.
    // The viewer's `--bond 100,26,108` setup path (nf-view lib.rs): a fresh Bond exactly there.
    sim.bond = nf_game::player::Bond::new([100.0, 26.0, 108.0], sim.bond.yaw);
    let mut prev_dis = sim.disabled;
    let mut crouch = false;
    let mut input = nf_game::player::Input::default();
    for t in 1..=6000u32 {
        // Crouch as the ride begins (the hand-off re-enables Bond on the truck bed).
        if !sim.disabled && prev_dis && sim.bond.pos[0] < 95.0 { crouch = true; }
        prev_dis = sim.disabled;
        if crouch && !sim.bond.crouched { input.crouch = true; crouch = false; } else { input.crouch = false; }
        sim.step_bond(input, false, false, false);
        if t % 100 == 0 || t < 10 {
            eprintln!("[ride] t{t} bond ({:.1},{:.1},{:.1}) 0x22:{} 0x70:{} 0x71:{} 0x72:{} 0x65:{} 0x1D:{} exit {:?}",
                sim.bond.pos[0], sim.bond.pos[1], sim.bond.pos[2],
                sim.drones.channels.on(0x22) as i32, sim.drones.channels.on(0x70) as i32,
                sim.drones.channels.on(0x71) as i32, sim.drones.channels.on(0x72) as i32,
                sim.drones.channels.on(0x65) as i32, sim.drones.channels.on(0x1d) as i32, sim.drones.level_exit);
        }
        if sim.drones.level_exit.is_some() { break; }
    }
    eprintln!("[ride] final bond {:?}", sim.bond.pos);
    for t in &sim.drones.triggers {
        if t.script.is_some() && (t.out == 0x70 || t.out == 0x71) {
            eprintln!("[ride] script trigger out {:#x} tris {} lo {:?} hi {:?}", t.out, t.tris.len(), t.lo, t.hi);
        }
    }
    assert_eq!(sim.drones.level_exit, Some(0x0700_0006), "the truck ride must end the level (the user's reference behaviour)");
}

/// Bug 1 (fix-level): the level-transition reload. `LoaderLoad` (0x1d4a00) masks the loader nibble
/// OFF (`code & 0xff0fffff`) before `sprintf("%8.8x.bin")` — `ResetMap_LevelToLoad` (0x1d3800) stores
/// the remapped code (0x07000006 → 0x07100006), and the load masks it back to 07000006.bin.
/// Sister: the anim nibble set (0x07F00006, what loader type 0/3 produce) maps to 07000006 too.
#[test]
fn level_archive_name_masks_the_loader_nibble() {
    // The value `ResetMap_LevelToLoad` stores for the 07000005 exit (param_3 == 0 remap, decomp switch).
    assert_eq!(nf_view::level_archive_name(0x0710_0006), "07000006.bin");
    // The trigger's raw out value (atlas trigger.md [L] ext+0x20 = 0x07000006).
    assert_eq!(nf_view::level_archive_name(0x0700_0006), "07000006.bin");
    // Sister: the anim archive nibble (07F00006 = the loader's type-0 name) still maps to the level.
    assert_eq!(nf_view::level_archive_name(0x07f0_0006), "07000006.bin");
    // And a genuinely different level keeps its own name.
    assert_eq!(nf_view::level_archive_name(0x0700_0001), "07000001.bin");
}

/// Bug 1 (fix-level): the reload path end-to-end — drive the truck chain to `level_exit`, then run the
/// reload exactly as the viewer does: `load(iso, level_archive_name(next))`. The next archive must parse,
/// build a sim, and report the new level id. Sister: the unmasked remapped code (07100006.bin) is not a
/// FILES.BIN name — `load` must fail (proving the comparison can fail).
#[test]
fn reload_after_load_level_exit_loads_07000006() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 07000005");
    let sim = scene.sim.as_mut().unwrap();
    // The route-a chain: ch 0x65 (the MuxIn output) → SP #320 → event 0x11 → ch 0x1D → LOAD_LEVEL 118.
    sim.drones.set_channel(0x65);
    let mut next = None;
    for _ in 0..600 {
        sim.step_bond(Default::default(), false, false, false);
        if let Some(n) = sim.drones.level_exit.take() { next = Some(n); break; }
    }
    let next = next.expect("the exit chain must fire (route a)");
    // The viewer's reload step (lib.rs RedrawRequested).
    let name = nf_view::level_archive_name(next);
    eprintln!("[reload] exit {next:#010x} → archive {name}");
    let scene2 = nf_view::load(&iso, &name, false).expect("the reload must load the next archive");
    let sim2 = scene2.sim.as_ref().expect("the next level has a sim");
    assert_eq!(sim2.level, 0x0700_0006, "the reloaded sim must be level 0x07000006");
    // Sister: the unmasked name is not an archive — the load fails.
    assert!(nf_view::load(&iso, "07100006.bin", false).is_err(), "07100006.bin is not a FILES.BIN name");
}

#[test]
fn probe_07000006_loads_headless() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let data = read_archive(&iso, "07000006.bin");
    let _level = Level::parse(&data).expect("parse 07000006");
    let _lb = LevelBin::parse(&data).expect("levelbin 07000006");
    let scene = nf_view::load(&iso, "07000006.bin", false).expect("nf_view::load 07000006.bin");
    eprintln!("07000006: sim present {}", scene.sim.is_some());
    if let Some(sim) = scene.sim.as_ref() {
        eprintln!("level id {:#x}, bond at {:?}", sim.level, sim.bond.pos);
    }
}
