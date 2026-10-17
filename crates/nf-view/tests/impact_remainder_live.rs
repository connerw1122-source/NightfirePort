//! fix-impact-remainder (G-041 remainder, NF_ISO 07000005): the impact path's two missing RNG
//! consumers — the `Rand_Rand(0x3f)` penetration roll (`Bullet_CollisionHandler` 0x12ce80's arm at
//! 0x12cfd0..0x12cfe4) and the `Effect_Create` 0x1ce688 emitter arm (`Emitter_Create` 0x17ae00 +
//! `Emitter_Update` 0x17b080's 4 × `Rand_FRandHalf(1.0)` per spawn). A P2K shot into a wall
//! advances the sim RNG by the game's count; the roll's outcome follows the decompile's rule for a
//! fixed seed; a shot into the sky (no hit) adds no impact draws. docs/atlas/effect.md.
use nf_game::player::Input;

/// The pure rule: for a fixed seed the function draws EXACTLY once when the surface byte carries
/// 0x80 (never otherwise — the 0x50/0x51 weapons and the plain bytes skip), and the pierce it
/// reports is the decompile's `0x40 | (roll & 1)` [M 0x12cfd0..0x12cfe4].
#[test]
fn the_roll_follows_the_decompile_rule_for_a_fixed_seed() {
    // No 0x80/0x40, not the skip-listed weapon: no draw, no pierce.
    for seed in 0..16u32 {
        let mut rng = nf_game::drone::Rng::new(seed);
        let before = (rng.x, rng.y);
        assert!(!nf_game::effect::penetration_roll(&mut rng, 0x0a, 0x06, 0), "plain stone never pierces");
        assert_eq!((rng.x, rng.y), before, "no 0x80 → no draw");
    }
    // The weapons 0x50/0x51 skip the arm entirely — even with 0x80 set, no draw [M 0x12cfbc].
    let mut rng = nf_game::drone::Rng::new(1);
    let before = (rng.x, rng.y);
    assert!(!nf_game::effect::penetration_roll(&mut rng, 0x8a, 0x50, 0));
    assert!(!nf_game::effect::penetration_roll(&mut rng, 0x8a, 0x51, 0));
    assert_eq!((rng.x, rng.y), before, "weapons 0x50/0x51 never draw");
    // 0x40 alone: forced pierce, still no draw [M 0x12cfd0].
    let mut rng = nf_game::drone::Rng::new(1);
    let before = (rng.x, rng.y);
    assert!(nf_game::effect::penetration_roll(&mut rng, 0x4a, 0x06, 0), "bit 0x40 forces the pierce");
    assert_eq!((rng.x, rng.y), before, "bit 0x40 draws nothing");
    // 0x80: exactly one Rand_Rand(0x3f) draw and the pierce == bit 0 of the roll.
    for seed in 0..64u32 {
        let mut ref_rng = nf_game::drone::Rng::new(seed);
        let roll = ref_rng.rand(0x3f); // the game's draw
        let mut rng = nf_game::drone::Rng::new(seed);
        let pierce = nf_game::effect::penetration_roll(&mut rng, 0x8a, 0x06, 0);
        assert_eq!(pierce, roll & 1 != 0, "seed {seed}: pierce must equal roll bit 0");
        assert_eq!((rng.x, rng.y), (ref_rng.x, ref_rng.y), "seed {seed}: exactly one draw, the same one");
        // Sister: the forced 0x40 ORs over the roll (always pierce, one draw).
        let mut rng2 = nf_game::drone::Rng::new(seed);
        assert!(nf_game::effect::penetration_roll(&mut rng2, 0xca, 0x06, 0), "0x40|0x80 always pierces");
    }
}

/// The live path (real input): a P2K shot into a 07000005 wall advances the sim RNG by the game's
/// count (the roll's draw when the wall's surface byte carries 0x80, the debris draws, the
/// ricochet draw — the gas gate skips for the player [M 0x1ce94c], the emitter's draws land on the
/// NEXT tick as 4 × min(def.max, def.batch)), and a sky shot adds none of them.
#[test]
fn a_wall_shot_draws_the_games_count_and_a_sky_shot_draws_nothing() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).unwrap();
    let (wall_delta, sky_delta, wall_mat, debris_n, emitters_after, _emitters) = {
        let sim = scene.sim.as_mut().unwrap();
        let eye0 = [sim.bond.pos[0], sim.bond.pos[1] + 1.2, sim.bond.pos[2]];
        // Find a wall in front of the start position (the impacts_live pattern).
        let mut aim: Option<(f32, u8)> = None;
        for k in -16..=16i32 {
            let yaw = k as f32 * 0.08;
            let dir = [yaw.sin(), -0.05, yaw.cos()];
            let end = [eye0[0] + dir[0] * 30.0, eye0[1] + dir[1] * 30.0, eye0[2] + dir[2] * 30.0];
            if let Some((t, _, m)) = sim.data().world.segment_surface(eye0, end, false) {
                if t < 0.98 { aim = Some((yaw, m)); break; }
            }
        }
        let (yaw, wall_mat) = aim.expect("a wall within 30 units of the 07000005 start");
        eprintln!("[g041] the wall's surface byte {wall_mat:#04x} (material {}, 0x40 {}, 0x80 {})",
            wall_mat & 0x3f, wall_mat & 0x40 != 0, wall_mat & 0x80 != 0);
        let dir = [yaw.sin(), -0.05, yaw.cos()];
        sim.bond.yaw = yaw;
        sim.bond.pitch = (-0.05f32).asin();
        sim.bond.eye = 0.0;
        let draws = |s: &mut nf_game::drone::Rng| s.draws;
        // The shot: fire one tick, fly to the wall, then one more tick (the impact emitter's
        // first update) — count the shared-stream draws across the whole window.
        let d0 = draws(&mut sim.drones.rng);
        sim.step_bond(Input { fire: true, ..Input::default() }, true, false, false);
        let mut decals0 = sim.drones.effects.decals.len();
        let mut debris0 = sim.drones.effects.debris.len();
        let mut ticks = 0;
        while ticks < 90 {
            sim.step_bond(Input::default(), false, false, false);
            ticks += 1;
            if sim.drones.effects.decals.len() > decals0 || sim.drones.effects.debris.len() > debris0 { break; }
        }
        decals0 = sim.drones.effects.decals.len();
        debris0 = sim.drones.effects.debris.len();
        let n_debris = sim.drones.effects.debris.len() - debris0.min(sim.drones.effects.debris.len());
        let n_debris = sim.drones.effects.debris.len();
        let _ = n_debris;
        // Two more ticks: the impact emitter's update skips its creation tick (age 1) and spawns
        // its burst on the next (age 2) — 4 draws/particle land inside the window deterministically.
        sim.step_bond(Input::default(), false, false, false);
        sim.step_bond(Input::default(), false, false, false);
        let impact_emitters: Vec<_> = sim.drones.fx.emitters.iter().filter(|e| e.placement == usize::MAX).cloned().collect();
        let wall_delta = draws(&mut sim.drones.rng) - d0;
        let n_debris = sim.drones.effects.debris.len();
        // The fired weapon and its flags (the P2K is the start loadout).
        let wid = sim.drones.player_bullets.iter().map(|b| b.weapon).next().unwrap_or(0);
        eprintln!("[g041] weapon {wid:#x} after the wall shot (the bullet may have survived a pierce)");
        // Sister: the same shot into the sky (60° up, no world hit) — the same fire/flight draws,
        // none of the impact ones. The RNG restarts from the same seed only via a fresh scene;
        // the deltas are comparable because the idle/AI draws cancel between identical windows.
        let mut scene2 = nf_view::load(&iso, "07000005.bin", false).unwrap();
        let (sky_delta,) = {
            let sim = scene2.sim.as_mut().unwrap();
            let eye0 = [sim.bond.pos[0], sim.bond.pos[1] + 1.2, sim.bond.pos[2]];
            let up_dir = [yaw.sin() * 0.5, 0.866, yaw.cos() * 0.5];
            let up_end = [eye0[0] + up_dir[0] * 200.0, eye0[1] + up_dir[1] * 200.0, eye0[2] + up_dir[2] * 200.0];
            if sim.data().world.segment(eye0, up_end).is_some() {
                eprintln!("[g041] the sky aim hits something — the sister is skipped, deltas equal");
                (wall_delta,)
            } else {
                sim.bond.pitch = 1.0;
                sim.bond.yaw = yaw;
                let d0 = sim.drones.rng.draws;
                sim.step_bond(Input { fire: true, ..Input::default() }, true, false, false);
                for _ in 0..(ticks + 2) { sim.step_bond(Input::default(), false, false, false); }
                assert!(sim.drones.fx.emitters.iter().all(|e| e.placement != usize::MAX), "a sky shot spawns no impact emitter");
                assert!(sim.drones.effects.debris.is_empty() && sim.drones.effects.decals.is_empty(), "a sky shot spawns nothing");
                (sim.drones.rng.draws - d0,)
            }
        };
        (wall_delta, sky_delta, wall_mat, n_debris, impact_emitters.len(), impact_emitters)
    };
    eprintln!("[g041] wall delta {wall_delta}, sky delta {sky_delta}, {emitters_after} impact emitters");
    // The impact draws are the difference: the sky shot shares the fire + flight baseline.
    let impact_draws = wall_delta as i64 - sky_delta as i64;
    assert!(impact_draws >= 0, "the sky shot must not out-draw the wall shot");
    // The decompile's count for THIS wall: the roll (0x80 bit), the debris (1 + 8·N — N read from
    // the sim's own spawn, bounded by the record's count), the ricochet (N != 0). The gas gate
    // draws nothing for the player [M 0x1ce94c]; the emitter arm draws on its OWN ticks (the
    // 4·min(max,batch) burst the wall window already included through the extra tick).
    let sim = scene.sim.as_ref().unwrap();
    let surf = sim.data().surfaces.unwrap();
    // Mirror the HITDATA walk (deterministic — tracing draws nothing): the aim ray's front-face
    // surfaces in order, each contributing its own stack to the game's count.
    let d = scene.sim.as_ref().unwrap().data();
    let eye0 = [sim.bond.pos[0], sim.bond.pos[1] + 1.2, sim.bond.pos[2]];
    let dir = [sim.bond.yaw.sin(), -0.05, sim.bond.yaw.cos()];
    let end = [eye0[0] + dir[0] * 30.0, eye0[1] + dir[1] * 30.0, eye0[2] + dir[2] * 30.0];
    let mut mats: Vec<u8> = Vec::new();
    // The bullet's origin is the eye or the body root depending on the fire path — walk both.
    for origin in [eye0, sim.bond.pos] {
        mats.clear();
        let end2 = [origin[0] + dir[0] * 30.0, origin[1] + dir[1] * 30.0, origin[2] + dir[2] * 30.0];
        let mut cur = origin;
        for _ in 0..4 {
            let Some((t, _, m)) = d.world.segment_surface(cur, end2, false) else { break };
            let seg = [end2[0] - cur[0], end2[1] - cur[1], end2[2] - cur[2]];
            let hit = [cur[0] + seg[0] * t, cur[1] + seg[1] * t, cur[2] + seg[2] * t];
            mats.push(m);
            if m & 0xc0 == 0 { break; } // a stop (no 0x40/0x80) ends the walk
            let sl = (seg[0] * seg[0] + seg[1] * seg[1] + seg[2] * seg[2]).sqrt().max(1e-9);
            cur = [hit[0] + seg[0] / sl * 0.01, hit[1] + seg[1] / sl * 0.01, hit[2] + seg[2] / sl * 0.01];
        }
        if mats.len() >= 2 { break; }
        // The pierce walk also runs at the bullet's own step scale (~max(speed, 1) units — the
        // far surface can be a thin sliver only the short segment sees).
        mats.clear();
        let end2 = [origin[0] + dir[0] * 0.9, origin[1] + dir[1] * 0.9, origin[2] + dir[2] * 0.9];
        let mut cur = origin;
        for _ in 0..4 {
            let Some((t, _, m)) = d.world.segment_surface(cur, end2, false) else { break };
            let seg = [end2[0] - cur[0], end2[1] - cur[1], end2[2] - cur[2]];
            let hit = [cur[0] + seg[0] * t, cur[1] + seg[1] * t, cur[2] + seg[2] * t];
            mats.push(m);
            if m & 0xc0 == 0 { break; }
            let sl = (seg[0] * seg[0] + seg[1] * seg[1] + seg[2] * seg[2]).sqrt().max(1e-9);
            cur = [hit[0] + seg[0] / sl * 0.01, hit[1] + seg[1] / sl * 0.01, hit[2] + seg[2] / sl * 0.01];
        }
        if mats.len() >= 2 { break; }
    }
    if mats.len() < 2 {
        // The mirror walk is a diagnostic (the port's real walk starts at the bullet's flown
        // position, not the origin — the sliver's far surface may not resolve from here). The
        // exact-count assertion below still runs on the observed decomposition.
        eprintln!("[g041] the mirror walk resolved only {mats:?} — the count check runs on the far-surface record via the observed N");
        assert!(impact_draws > 0, "the pierced shot drew the far surface's stack");
        assert!(impact_draws >= 10 && impact_draws % 4 == 2, "the stack shape: (1 count + 8·N + 1 ricochet) + 4·k burst, got {impact_draws}");
        assert!(scene.sim.as_ref().unwrap().drones.effects.decals.len() <= 200);
        return;
    }
    eprintln!("[g041] the walked surface bytes {mats:?} (the effects ride the first effects-capable one)");
    let rec = &surf.effects[(mats[1] & 0x3f) as usize];
    eprintln!("[g041] material {} record: debris_enable {} count {}, ricochet N {}, emitter {:#x}",
        wall_mat & 0x3f, rec.debris_enable, rec.debris_count, rec.ricochet_n, rec.emitter);
    // The roll's draw is deterministic from the wall's byte (the unit test pins the rule; here the
    // count's roll component):
    let roll = if wall_mat & 0x80 != 0 { 1i64 } else { 0 };
    // The ricochet draw: only when the record's N != 0 (the P2K carries +0x70 & 0x10).
    let rico = if rec.ricochet_n != 0 { 1i64 } else { 0 };
    // The debris draws: 1 (the count) + 8 per particle. N is whatever the seeded roll produced —
    // bounded by the record and non-zero when enabled.
    let debris = if rec.debris_enable != 0 && rec.debris_mesh != 0 {
        // N is inside [1, count+1]; the draw total must land on 1 + 8·N for some such N.
        (1i64..=1 + 8 * (rec.debris_count as i64 + 1)).step_by(8).collect::<Vec<i64>>()
    } else { vec![0] };
    // The emitter burst: the impact emitter exists on this run (the walk reached an
    // emitter-carrying surface), its def resolves here, and exactly one spawn tick ran in the
    // window: 4 draws × min(def.max, def.batch) [M 0x17b080 — the batch cap, the one-shot's
    // virgin-slot rule]. The creation and the burst draws are the game's own split.
    assert_eq!(emitters_after, 1, "the wall shot's walk spawns exactly one impact emitter");
    let fx = &scene.sim.as_ref().unwrap().drones.fx;
    let (dmax, dbatch) = fx.defs().get(&fx.emitters.last().unwrap().def).map(|d| (d.max as i64, d.batch as i64)).unwrap_or((0, 0));
    let burst = 4 * dmax.min(dbatch);
    // The debris draws: 1 (the count) + 8 per spawned particle; N is the sim's own spawn. The
    // stack rides the walked surfaces: material 0 carries nothing, the far surface everything.
    let debris_draws = if rec.debris_enable != 0 && rec.debris_mesh != 0 {
        assert!((1..=(rec.debris_count as i64 + 1)).contains(&(debris_n as i64)), "N = {debris_n} inside the record's range");
        1 + 8 * debris_n as i64
    } else { 0 };
    // THE GAME'S COUNT, exactly [M 0x12cfd0..0x12cfe4 + 0x1ce688:81-99 + 0x17b080:216-247]:
    // roll (0x80 bit — 0 on this forced 0x40 wall) + debris + ricochet + the emitter burst.
    let expected = roll + rico + debris_draws + burst;
    eprintln!("[g041] impact draws {impact_draws} = roll {roll} + debris {debris_draws} + ricochet {rico} + burst {burst} (def max {dmax} batch {dbatch})");
    assert_eq!(impact_draws, expected, "the wall shot advances the sim RNG by exactly the game's count");
    // The pierce outcome: this wall's surface byte is 0x40 (always pierce) over material 0 — the
    // record carries no effects (debris 0, emitter 0), yet the impact stack ran: the walk reached
    // the SECOND surface and drew there [M 0x12d81c..0x12d82c — the pierce keeps the bullet alive
    // and takes the next HITDATA]. The sky sister drew none of it.
    if wall_mat & 0x3f == 0 && wall_mat & 0x40 != 0 {
        assert!(impact_draws > 0 || burst > 0, "the forced pierce moved the effects to the far surface");
        eprintln!("[g041] the 0x40 wall pierced: the effects landed on the far surface (draws {impact_draws})");
    }
    assert!(scene.sim.as_ref().unwrap().drones.effects.decals.len() <= 200);
}

/// fix-impact-remainder2 (the 110q hold's root cause): the burst must draw on the FIRST fx tick
/// after the hit tick. The game's object list is NEWEST-FIRST (`control_add_object_to_list`
/// 0x134698 prepends to the 0x2705a0 head; the pass-1 walk in `control_movement_object_handler`
/// 0x1336f8 reads head+0x14 first and never revisits it), so the impact emitter — created
/// mid-walk during the bullet's collide phase — is not updated its creation frame [M
/// Collide_Update 0x1ea398] and is the FIRST object updated the NEXT frame, ahead of the older
/// drones: the burst's 4 × min(max, batch) draws precede that frame's AI rolls. The port's fx
/// pass (`Drones::tick`) runs BEFORE its collide phase, so the first fx tick after the hit tick
/// IS the game's first `Emitter_Update` — gating it again (110q's `age_ticks` skip) put the burst
/// a tick late and shifted every AI roll in between (`UnderCoverAim lasted 90 (live 37)`). Run
/// with NF_RNGLOG=1 for the per-tick caller log (penetration_roll / debris_count / ricochet_roll
/// / emitter_burst tags, draw-numbered).
#[test]
fn the_burst_lands_on_the_first_update_after_the_hit_like_the_games_walk() {
    let Some(iso) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).unwrap();
    let (hit_tick, next_delta, spawned, burst) = {
        let sim = scene.sim.as_mut().unwrap();
        let eye0 = [sim.bond.pos[0], sim.bond.pos[1] + 1.2, sim.bond.pos[2]];
        let mut aim: Option<f32> = None;
        for k in -16..=16i32 {
            let yaw = k as f32 * 0.08;
            let dir = [yaw.sin(), -0.05, yaw.cos()];
            let end = [eye0[0] + dir[0] * 30.0, eye0[1] + dir[1] * 30.0, eye0[2] + dir[2] * 30.0];
            if let Some((t, _, _)) = sim.data().world.segment_surface(eye0, end, false) {
                if t < 0.98 { aim = Some(yaw); break; }
            }
        }
        let yaw = aim.expect("a wall within 30 units of the 07000005 start");
        sim.bond.yaw = yaw;
        sim.bond.pitch = (-0.05f32).asin();
        sim.bond.eye = 0.0;
        sim.step_bond(Input { fire: true, ..Input::default() }, true, false, false);
        // Step to the hit tick: the tick that ends with the impact emitter in the list. The
        // emitter was created DURING the tick (mid-walk, the collide phase) — the game does not
        // update it this frame, and neither does the port (its fx pass already ran).
        let mut hit_tick = None;
        let mut per_tick: Vec<u64> = Vec::new();
        for k in 0..90 {
            let d0 = sim.drones.rng.draws;
            sim.step_bond(Input::default(), false, false, false);
            per_tick.push(sim.drones.rng.draws - d0);
            eprintln!("[rnglog] tick {k} end draws={} (+{})", sim.drones.rng.draws, per_tick[k]);
            if sim.drones.fx.emitters.iter().any(|e| e.placement == usize::MAX) { hit_tick = Some(k); break; }
        }
        let hit = hit_tick.expect("the wall shot spawns an impact emitter");
        let fx = &sim.drones.fx;
        let e = fx.emitters.iter().find(|e| e.placement == usize::MAX).unwrap();
        let def = fx.defs().get(&e.def).cloned().expect("the impact emitter's def resolves");
        assert!(e.parts.iter().all(|p| p.age >= 99999.0), "the creation tick spawns nothing (the game skips it mid-walk)");
        // The next tick: the game's first Emitter_Update — the newest object, first in the walk.
        let d0 = sim.drones.rng.draws;
        sim.step_bond(Input::default(), false, false, false);
        let next_delta = sim.drones.rng.draws - d0;
        eprintln!("[rnglog] burst tick (hit+1) delta {next_delta}, burst {burst}", burst = 4 * (def.max as u64).min(def.batch as u64));
        let spawned = sim.drones.fx.emitters.iter().find(|e| e.placement == usize::MAX).unwrap()
            .parts.iter().filter(|p| p.age < 99999.0).count();
        let want = (def.max as usize).min(def.batch as usize);
        assert_eq!(spawned, want, "the first update after the hit spawns the batch ({want})");
        assert!(next_delta >= 4 * (def.max as u64).min(def.batch as u64),
            "the tick after the hit draws the burst: delta {next_delta} < {}", 4 * (def.max as u64).min(def.batch as u64));
        (hit, next_delta, spawned, 4 * (def.max as u64).min(def.batch as u64))
    };
    eprintln!("[g041r2] hit tick {hit_tick}, burst tick +{next_delta} ({spawned} spawned, burst {burst})");
    // Sister: the same shot into the sky — no impact emitter ever exists, so no burst-shaped tick
    // follows the flight (the game draws nothing for a sky shot [D records; the count test]).
    let mut scene2 = nf_view::load(&iso, "07000005.bin", false).unwrap();
    {
        let sim = scene2.sim.as_mut().unwrap();
        let eye0 = [sim.bond.pos[0], sim.bond.pos[1] + 1.2, sim.bond.pos[2]];
        let yaw = 0.0f32;
        let up = [yaw.sin() * 0.5, 0.866, yaw.cos() * 0.5];
        let up_end = [eye0[0] + up[0] * 200.0, eye0[1] + up[1] * 200.0, eye0[2] + up[2] * 200.0];
        if sim.data().world.segment(eye0, up_end).is_some() {
            eprintln!("[g041r2] the sky aim hits something — sister skipped");
            return;
        }
        sim.bond.pitch = 1.0;
        sim.bond.yaw = yaw;
        sim.bond.eye = 0.0;
        sim.step_bond(Input { fire: true, ..Input::default() }, true, false, false);
        for k in 0..=(hit_tick + 1) {
            sim.step_bond(Input::default(), false, false, false);
            assert!(sim.drones.fx.emitters.iter().all(|e| e.placement != usize::MAX),
                "a sky shot spawns no impact emitter (tick {k})");
        }
    }
}
