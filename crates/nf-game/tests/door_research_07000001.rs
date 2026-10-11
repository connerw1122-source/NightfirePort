//! One-off research probe (Task A-8-2): dump every door's glist hash, whether it is in
//! `level.by_hash`, whether it has a collision shape (block 0x2E) in `CollisionWorld::shapes`,
//! and whether a bullet segment through the closed door's centre hits the world.
//! Run: NF_ISO=/tmp/nightfire.iso cargo test -p nf-game --release door_research_07000001 -- --nocapture
use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, level::Level};
use nf_game::door::Doors;
use nf_game::triggers::Channels;
use nf_game::world::CollisionWorld;

fn load(code: u32) -> Option<Level> {
    let path = std::env::var_os("NF_ISO")?;
    let iso = IsoImage::open(&path).unwrap();
    let (entries, bin) = disc::files_bin_entries(&iso).unwrap();
    let name = format!("{code:08x}.bin");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(&name)).unwrap();
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    Some(Level::parse(&data).unwrap())
}

#[test]
fn door_research_07000001() {
    let Some(level) = load(0x07000001) else { eprintln!("skip: NF_ISO not set"); return };
    let m = level.main_map().unwrap();
    let ps = &level.maps[m].placements;
    let blocks = level.path_blocks();
    let doors = Doors::create(ps, &blocks, |p| {
        let o = if p.by_hash() { level.by_hash.get(&p.hashcode).map(|&(om, oi)| &level.maps[om].objects[oi]) }
                else { level.maps[m].objects.get(p.index as usize) };
        o.map(|o| (o.hashcode, o.centre, o.radius))
    });

    // Build the collision world exactly like Drones::attach_level / scene.rs does:
    // `instances(true)` = CELS ONLY (door placements do NOT create cels, so their collision
    // only enters via `set_dynamic` keyed by the door's glist hash).
    let (instances, _) = level.instances(true);
    let world = CollisionWorld::build(&level, &instances);

    // Verify the NF_DOORLOG diagnostic works (it no-ops without the env var, prints with it).
    std::env::set_var("NF_DOORLOG", "1");
    doors.log_doors(Some(&world));
    std::env::remove_var("NF_DOORLOG");

    println!("\n=== 07000001 doors (5) ===");
    println!("idx | placement | by_hash? | kind | group | ch_open | ch_lock | straddle | snd_o/c/h | flags(0x20=keep) | approach | locked | glist | in by_hash? | in shapes? | tris | sph.pos | sph.r | state | open_timer");
    for (k, d) in doors.list.iter().enumerate() {
        let p = &ps[d.placement];
        let in_by_hash = level.by_hash.contains_key(&d.glist);
        let has_shape = world.has_shape(d.glist);
        let ntris = world.shape(d.glist).map(|s| s.len()).unwrap_or(0);
        let objname = if p.by_hash() { level.by_hash.get(&p.hashcode).map(|&(mm, oo)| level.maps[mm].objects[oo].name.clone()) }
            else { level.maps[m].objects.get(p.index as usize).map(|o| o.name.clone()) };
        let objname = objname.unwrap_or_default();
        println!("{k} | pl={:>3} {objname:<28} | by_hash={} | kind {} | grp {} | ch_open {} | ch_lock {} | straddle {} | snd {}/{:#x}/{:#x} | flags={:#04x} (keep_open={}, spline={}) | appr {} | locked {} | glist={:#010x} | in_by_hash={} | in_shapes={} | tris {} | sph.pos=({:.2},{:.2},{:.2}) | sph.r={:.3} | state {:#x} | open_timer {}",
            d.placement, p.by_hash(), d.kind, d.group, d.ch_open, d.ch_lock, d.straddle,
            d.snd_open, d.snd_close, d.snd_hit, d.flags, d.flags & 0x20 != 0, d.flags & 1 != 0,
            d.approach, d.locked_flag, d.glist, in_by_hash, has_shape, ntris,
            d.sph.0[0], d.sph.0[1], d.sph.0[2], d.sph.1, d.state, d.open_timer);
        // Print the first key of the anim too — confirms the path_blocks lookup.
        if let Some(ks) = d.keys.as_ref() {
            println!("    anim: {} keys, first pos=({:.2},{:.2},{:.2}) frames={} speed={:.4} pos01={:.3}",
                ks.len(), ks[0].pos[0], ks[0].pos[1], ks[0].pos[2], d.frames, d.speed, d.pos01);
        } else {
            println!("    anim: NONE (no 0x19 block)");
        }
        // Bullet test: horizontal segment of length 10 along the door's facing direction
        // (matrix row 2 = Mat_GetDir), passing through the door's world-space sphere centre.
        // For a closed sliding door this must hit the panel; if `in_shapes=false` it sails through.
        let fwd = [d.matrix[2][0], d.matrix[2][1], d.matrix[2][2]];
        let from = [d.sph.0[0] - fwd[0] * 5.0, d.sph.0[1] - fwd[1] * 5.0, d.sph.0[2] - fwd[2] * 5.0];
        let to   = [d.sph.0[0] + fwd[0] * 5.0, d.sph.0[1] + fwd[1] * 5.0, d.sph.0[2] + fwd[2] * 5.0];
        let hit = world.segment(from, to);
        // Tick the world's dynamic layer once so the door's collision is actually placed.
        // (set_dynamic is normally called every tick by Drones::tick.)
        let parts: Vec<(u64, u32, nf_formats::anim::Mat4)> = doors.list.iter().enumerate()
            .filter(|(_, x)| x.glist != 0)
            .map(|(kk, x)| (0x100_0000_0000 | kk as u64, x.glist, x.matrix))
            .collect();
        world.set_dynamic(&parts);
        let hit_dyn = world.segment(from, to);
        // And a clean point-blank test: a 2-unit segment centred on the door's sphere centre
        // along the facing direction — passes through the door panel (thickness ~0.08) and
        // nothing else can be in the way at that range. If `in_shapes=false`, this is None.
        let pf = [d.sph.0[0] - fwd[0] * 1.0, d.sph.0[1] - fwd[1] * 1.0, d.sph.0[2] - fwd[2] * 1.0];
        let pt = [d.sph.0[0] + fwd[0] * 1.0, d.sph.0[1] + fwd[1] * 1.0, d.sph.0[2] + fwd[2] * 1.0];
        let hit_pb = world.segment(pf, pt);
        println!("    horizontal bullet along facing {:?} through sph.centre (len 10): hit_static={:?} hit_with_door_dynamic={:?} | 2u-segment through panel: {:?}", fwd, hit, hit_dyn, hit_pb);
    }

    // Now open door 0 (placement 13), then tick the simulation ~400 times with no actor near
    // to see whether the auto-close path triggers.
    let mut doors = doors;
    let mut ch = Channels::default();
    // Open every door by calling player_use at its sphere centre.
    for i in 0..doors.list.len() {
        let sph = doors.list[i].sph;
        let head = [sph.0[0], sph.0[1], sph.0[2] - 1.0];
        let _ = doors.player_use(head, [0.0, 0.0, 1.0], &mut ch, 0);
    }
    doors.calc();
    // Tick N times with no actor nearby — far away.
    let far_actor = [([1000.0, 1000.0, 1000.0], 0.4f32, false)];
    for f in 1..=400u32 {
        for i in 0..doors.list.len() { doors.update(i, &mut ch, &far_actor, f); }
        doors.calc();
        doors.end_tick(&ch);
    }
    println!("\n=== after 400 ticks with no nearby actor ===");
    for (i, d) in doors.list.iter().enumerate() {
        println!("door {i} (pl {}): state {:#x} pos01 {:.3} open_timer {}", d.placement, d.state, d.pos01, d.open_timer);
    }
}

/// Task A-9-2: demonstrate that the throttled diagnostic fires every 60 ticks and that
/// the `blocked` column correctly reflects whether Bond (or any actor) is inside the
/// `(r1+r2)²·2` proximity radius — the exact condition the auto-close path checks at
/// `open_timer >= 360`. Two scenarios:
///   1. Bond parked 0.5 units from door 0's sphere centre (well inside the blocking
///      radius for every door on 07000001 — door 0's threshold is `(1.564+0.4)²·2 ≈ 7.71`,
///      so dist² = 0.25 → blocked). The door must stay OPEN and `open_timer` must cycle
///      0→360→0→360→…  — i.e. the auto-close never fires.
///   2. Bond parked 100 units away (far outside). The door auto-closes after the timer.
#[test]
fn door_autoclose_blocked_diagnostic_07000001() {
    let Some(level) = load(0x07000001) else { eprintln!("skip: NF_ISO not set"); return };
    let m = level.main_map().unwrap();
    let ps = &level.maps[m].placements;
    let blocks = level.path_blocks();
    let mut doors = Doors::create(ps, &blocks, |p| {
        let o = if p.by_hash() { level.by_hash.get(&p.hashcode).map(|&(om, oi)| &level.maps[om].objects[oi]) }
                else { level.maps[m].objects.get(p.index as usize) };
        o.map(|o| (o.hashcode, o.centre, o.radius))
    });
    let mut ch = Channels::default();

    // Open door 0 (placement 13, SlidingDoor) by activating it from the front.
    let sph0 = doors.list[0].sph;
    let head = [sph0.0[0], sph0.0[1], sph0.0[2] - 1.0];
    let _ = doors.player_use(head, [0.0, 0.0, 1.0], &mut ch, 0);
    doors.calc();

    // SCENARIO 1 — Bond stands 0.5 units from door 0's sphere centre (inside the
    // blocking radius). Enable the throttled diagnostic so it prints at frames 60, 120,
    // 180, 240, 300, 360, 420. After 420 ticks an unblocked door would have auto-closed
    // (31 to open + 360 to time out + 30 to close), but a blocked one stays OPEN forever.
    std::env::set_var("NF_DOORLOG", "1");
    let bond_near = [(sph0.0, 0.4f32, false)]; // Bond at door 0's sphere centre, radius 0.4.
    for f in 1..=420u32 {
        for i in 0..doors.list.len() { doors.update(i, &mut ch, &bond_near, f); }
        doors.calc();
        doors.end_tick(&ch);
        doors.log_doors_throttled(None, &bond_near, f);
    }
    std::env::remove_var("NF_DOORLOG");

    println!("\n=== after 420 ticks with Bond 0.5u from door 0 ===");
    for (i, d) in doors.list.iter().enumerate() {
        println!("door {i} (pl {}): state {:#x} pos01 {:.3} open_timer {} (keep_open={})",
            d.placement, d.state, d.pos01, d.open_timer, d.flags & 0x20 != 0);
    }
    // Door 0 (placement 13) has keep_open=false, so it WOULD auto-close at ~tick 391
    // if Bond were far. With Bond blocking, it must stay OPEN (state 4) and the timer
    // must have cycled (open_timer < 360 because it reset at 360).
    let d0 = &doors.list[0];
    assert_eq!(d0.state, 4, "door 0 stays OPEN — the auto-close was blocked by Bond");
    assert!(d0.open_timer < 360, "open_timer must have reset at 360 (cycling forever)");

    // SCENARIO 2 — Bond walks 100 units away. The door auto-closes after the timer.
    let far = [([sph0.0[0] + 100.0, sph0.0[1], sph0.0[2]], 0.4f32, false)];
    for f in 421..=840u32 {
        for i in 0..doors.list.len() { doors.update(i, &mut ch, &far, f); }
        doors.calc();
        doors.end_tick(&ch);
    }
    println!("\n=== after another 420 ticks with Bond 100u away ===");
    for (i, d) in doors.list.iter().enumerate() {
        println!("door {i} (pl {}): state {:#x} pos01 {:.3} open_timer {}",
            d.placement, d.state, d.pos01, d.open_timer);
    }
    let d0 = &doors.list[0];
    assert!(d0.state == 2 || d0.state == 0x10, "door 0 closes once Bond leaves (state {:#x})", d0.state);
}
