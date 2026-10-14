//! fix-head-look — the playtest 1013-1758 head-look bugs (07000007): "the models are looking kind
//! of weird, where their heads are turning kind of the wrong direction, wrong angle". Compared in
//! PCSX2: the port's guard head ROLLED sideways toward the camera (the real game's guard keeps his
//! head upright and only yaws it); the guest's head tipped UP well past the camera.
//!
//! Root cause [M `AnimObjectHeadTrack` 0x1bf618 via `psiBuildMatrixPalette` 0x111498]:
//! - The game composes `q_yaw ⊗ q_pitch ⊗ q_world_head` in WORLD axes — yaw about the NECK axis
//!   (normalize(palette[5].pos − palette[2].pos), the +0x30 translation slots), pitch about the
//!   side axis normalize(cross(neck, row2)) — and premultiplies the head bone's world rotation.
//!   The port post-multiplied model-axis rotations in the head bone's own animated local frame:
//!   cross-axis contamination = the roll.
//! - The game's ease targets are geometric angles measured from the FRESH anim pose's head axes
//!   (the palette HeadTrack receives is rebuilt from the anim each tick), so the stored angle never
//!   double-counts the anim's own head pose; the port measured from the BODY axes over a fixed
//!   +1.55 stand-in — the anim's head pitch was applied twice (the "tips UP well past" bug).
//! - The game only tracks within the head-track range² (SetHeadTracking's 3.0² = 9.0
//!   [docs/fields/animobject.md +0xE8, [L-read] drone 9.0]); beyond it the angles ease back to 0.
//!   The port tracked at any sighted distance.

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn row(m: &[[f32; 4]; 4], r: usize) -> [f32; 3] { [m[r][0], m[r][1], m[r][2]] }
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
fn norm(a: [f32; 3]) -> [f32; 3] {
    let l = dot(a, a).sqrt();
    [a[0] / l, a[1] / l, a[2] / l]
}

/// The head bone's (bone 5) world up verticalness (1 = upright) and the angle from its world
/// forward to `target`. `psiBuildMatrixPalette` 0x111498: each palette entry is 0x40 bytes —
/// rotation rows 0..0x30 then the translation at +0x30 — row 1 = the world image of the bone's
/// local +y (up), row 2 = its +z (forward).
fn head_stats(world: &[[[f32; 4]; 4]], target: [f32; 3]) -> (f32, f32) {
    let head = [world[5][3][0], world[5][3][1], world[5][3][2]];
    let fwd = norm(row(&world[5], 2));
    let up = norm(row(&world[5], 1));
    let to = norm([target[0] - head[0], target[1] - head[1], target[2] - head[2]]);
    (up[1], dot(fwd, to).clamp(-1.0, 1.0).acos())
}

/// An enabled, alive drone nearest to Bond (the head-track code is shared by guards and guests;
/// the playtest bugs showed on both).
fn enabled_drones(sim: &nf_view::scene::Sim) -> Vec<usize> {
    let bp = sim.bond.pos;
    let mut v: Vec<usize> = (0..sim.drones.list.len())
        .filter(|&i| sim.drones.list[i].present && sim.drones.list[i].enabled && !sim.drones.list[i].corpse)
        .collect();
    v.sort_by_key(|&i| {
        let p = sim.drones.list[i].pos;
        ((p[0] - bp[0]).powi(2) + (p[2] - bp[2]).powi(2)) as i64
    });
    v
}

/// ── The no-roll + on-target test ────────────────────────────────────────────────────────────────
/// Bond stands inside the head-track range (3.0 [M 0x1bf618's range gate; +0xE8 = 9.0 [L]]),
/// eye-level and slightly off-axis so the head must yaw. While the head tracks, its world up
/// stays within a few degrees of vertical (no roll) and its forward points at Bond's eye.
#[test]
fn tracking_head_stays_upright_and_points_at_bond() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..600 { sim.step_bond(Input::default(), false, false, false); }
    let drones = enabled_drones(sim);
    assert!(!drones.is_empty(), "the party level has enabled characters");
    let mut best: Option<(usize, f32, f32)> = None; // (drone, up verticalness, forward angle) at its max-tracking tick
    for &di in &drones {
        let gp = sim.drones.list[di].pos;
        let fwd = [sim.drones.list[di].yaw.sin(), 0.0, sim.drones.list[di].yaw.cos()];
        let right = [fwd[2], 0.0, -fwd[0]];
        // 2.2 ahead + 0.9 to the side (≈ 2.4 away, ~22° off-axis), eye level.
        sim.bond.pos = [gp[0] + fwd[0] * 2.2 + right[0] * 0.9, gp[1], gp[2] + fwd[2] * 2.2 + right[2] * 0.9];
        sim.bond.yaw = (gp[0] - sim.bond.pos[0]).atan2(gp[2] - sim.bond.pos[2]);
        for _ in 0..240 {
            sim.step_bond(Input::default(), false, false, false);
            let stats = {
                let dr = &sim.drones.list[di];
                if !dr.present || dr.corpse { break; }
                let t = dr.head_yaw.abs() + dr.head_pitch.abs();
                if t < 0.05 { continue; }
                let world = match dr.palette(&sim.data()) { Some(w) => w, None => continue };
                if world.len() <= 5 { continue; }
                let target = sim.bond.head();
                (t, head_stats(&world, target))
            };
            let (t, (upness, ang)) = stats;
            if best.map_or(true, |(_, bu, _)| upness > bu || (upness - bu).abs() < 1e-4 && ang < best.unwrap().2) {
                best = Some((di, upness, ang));
            }
        }
    }
    let (di, upness, ang) = best.expect("some enabled character head-tracks Bond standing beside it");
    println!("drone {di}: up verticalness {upness:.4}, forward-to-Bond angle {ang:.3} rad");
    // Probe: the fresh anim pose's head row2 bearing vs the body yaw (the rig's z-direction check).
    {
        let dr = &sim.drones.list[di];
        let target = sim.bond.head();
        if let Some(world) = dr.palette(&sim.data()) {
            if world.len() > 5 {
                let head = [world[5][3][0], world[5][3][1], world[5][3][2]];
                let r2 = row(&world[5], 2);
                let to = norm([target[0] - head[0], target[1] - head[1], target[2] - head[2]]);
                println!("PROBE body yaw {:.3} | head row2 bearing {:.3} | to-Bond bearing {:.3} | head_yaw stored {:.3} | head row2.y {:.3}",
                    dr.yaw, r2[0].atan2(r2[2]), to[0].atan2(to[2]), dr.head_yaw, r2[1]);
            }
        }
    }
    assert!(upness > 0.99,
        "the tracked head's world up stays within ~8° of vertical (playtest: the head ROLLED sideways toward the camera), got {upness:.4}");
    assert!(ang < 0.25,
        "the tracked head's forward points at Bond within the ease convergence, got {ang:.3} rad");
}

/// ── Sister: the range gate ──────────────────────────────────────────────────────────────────────
/// Bond stands 6 units away (beyond the 3.0 head-track range, well within sight): the game leaves
/// the head at its anim pose — the head angles ease back to 0. The pre-fix port swivelled heads
/// across the room.
#[test]
fn head_track_holds_still_beyond_the_game_range() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..600 { sim.step_bond(Input::default(), false, false, false); }
    let drones = enabled_drones(sim);
    assert!(!drones.is_empty(), "the party level has enabled characters");
    let mut swivellers: Vec<(usize, f32)> = Vec::new();
    for &di in &drones {
        let gp = sim.drones.list[di].pos;
        let fwd = [sim.drones.list[di].yaw.sin(), 0.0, sim.drones.list[di].yaw.cos()];
        sim.bond.pos = [gp[0] + fwd[0] * 6.0, gp[1], gp[2] + fwd[2] * 6.0];
        sim.bond.yaw = (gp[0] - sim.bond.pos[0]).atan2(gp[2] - sim.bond.pos[2]);
        for _ in 0..180 { sim.step_bond(Input::default(), false, false, false); }
        let dr = &sim.drones.list[di];
        if !dr.present || dr.corpse { continue; }
        if dr.head_yaw.abs() > 0.05 { swivellers.push((di, dr.head_yaw)); }
    }
    assert!(swivellers.is_empty(),
        "no head swivels at Bond 6 units away (beyond the game's 3.0 head-track range [M 0x1bf618; +0xE8 = 9.0 [L]]): {swivellers:?}");
}
