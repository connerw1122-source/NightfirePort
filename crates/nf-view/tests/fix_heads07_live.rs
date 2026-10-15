//! fix-heads-07 — the playtest 1014-1725 head bug (07000007, user "It looks awful. I took an F12"):
//! at the F12 spots the guests' heads lie sideways on their shoulders/back (spots 1 and 3), and the
//! hair mesh rides at the collar (spot 2) instead of on the head.
//!
//! Root cause [M `AnimObjectHeadTrack` 0x1bf618 via `psiBuildMatrixPalette` 0x111498]: the game
//! turns ONLY the head's orientation — HeadTrack hands the palette entry itself to
//! `Quat_QuatToMat` 0x1e3e80 (0x1bfcc0 `lw $a1, 0xd8($sp)` = palette + bone·0x40 [0x1bf6ec
//! `sll $v0, $v1, 6` + 0x1bf6f0 `addu $v0, $t1, $v0`], 0x1bfcc4 `jal 0x1e3e80`), and that helper
//! stores offsets 0x00..0x2c only (`swc1` to ($a1), 4, 8, 0x10, 0x14, 0x18, 0x20, 0x24, 0x28 —
//! three rotation rows), never the +0x30 translation row (the palette entry is 0x40 bytes:
//! rotation rows 0..0x30 then the translation at +0x30, docs/fields/animobject.md +0xB8). The
//! head keeps its anim-posed position on the neck while only its rotation turns.
//!
//! The port multiplied the head bone's FULL 4×4 world matrix by the two track rotations, which
//! rotated the translation row about the object origin too: at the saturated track angles (a guest
//! at ~1 unit from Bond drives yaw to the ±1.1 clamp and pitch to the up-clamp [M HeadTrack's
//! clamp block; the game's pitch range is [-1.0, +0.6] in its positive-down convention,
//! docs/reports/fix-head-look.md]) the head position swung ~0.5 units off the neck onto the
//! shoulders/back, and attachment meshes anchored to the head bone (the hair) followed it down.
//! Measured pre-fix at the spot-3 placement: neck→head dir.y 0.308/0.298 (upright anims hold
//! ~0.97..0.99), head 0.49 off the neck (0.40 in the anim).

use nf_game::player::Input;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn row(m: &[[f32; 4]; 4], r: usize) -> [f32; 3] { [m[r][0], m[r][1], m[r][2]] }
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 { a[0]*b[0] + a[1]*b[1] + a[2]*b[2] }
fn norm(a: [f32; 3]) -> [f32; 3] { let l = dot(a, a).sqrt(); [a[0]/l, a[1]/l, a[2]/l] }

/// Bond stands at the playtest spot-3 F12 placement (`--bond 40.58,44.13,-70.77,0.682,-0.080`,
/// frame 1488 of the user's run) with no other input — the headless real-input path
/// (`Sim::step_bond`). Two guests end up inside the head-track range tracking him hard enough to
/// saturate the stored angles. For every strongly tracking head:
/// - the head bone's world position stays ON the anim's neck (the +0x30 translation row the game
///   never rewrites): the neck→head direction stays within 45° of the anim's own up-held neck
///   axis (dir.y ≥ 0.7 measured 0.97..0.99 on non-tracking heads; pre-fix 0.31/0.30) and the
///   neck→head length stays the anim's ≤ 0.45 (pre-fix 0.49 with the head swung onto the back);
/// - the track still RUNS (the orientation turns: |head_yaw| ≥ 0.9 on the found heads).
#[test]
fn tracked_head_stays_on_the_neck_at_the_playtest_spot() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    sim.bond = nf_game::player::Bond::new([40.58, 44.13, -70.77], 0.682);
    sim.bond.pitch = -0.080;
    for _ in 0..1488 { sim.step_bond(Input::default(), false, false, false); }
    let bp = sim.bond.pos;
    let mut checked = 0;
    for di in 0..sim.drones.list.len() {
        let (hy, hp, dpos) = {
            let dr = &sim.drones.list[di];
            if !dr.present || dr.corpse { continue; }
            (dr.head_yaw, dr.head_pitch, dr.pos)
        };
        if (dpos[0]-bp[0]).powi(2) + (dpos[2]-bp[2]).powi(2) > 64.0 { continue; }
        if hy.abs() < 0.9 { continue; } // a strongly tracking head (the saturated-clamp case)
        let world = { let dr = &sim.drones.list[di]; match dr.palette(&sim.data()) { Some(w) => w, None => continue } };
        if world.len() <= 5 { continue; }
        let head = [world[5][3][0], world[5][3][1], world[5][3][2]];
        let neck = [world[2][3][0], world[2][3][1], world[2][3][2]];
        let off = [head[0]-neck[0], head[1]-neck[1], head[2]-neck[2]];
        let len = dot(off, off).sqrt();
        let dir_y = norm(off)[1];
        println!("drone {di}: head_yaw {hy:.3} head_pitch {hp:.3} neck->head len {len:.3} dir.y {dir_y:.3}");
        checked += 1;
        assert!(len <= 0.45,
            "the tracked head stays on the anim's neck (the game's HeadTrack writes only the palette entry's \
             rotation rows — Quat_QuatToMat 0x1e3e80 stores 0x00..0x2c; the +0x30 translation row is never \
             rewritten): neck→head {len:.3} (the anim's 0.40; the pre-fix detached head 0.49)");
        assert!(dir_y >= 0.7,
            "the tracked head's neck→head direction stays the anim's up-held neck axis: dir.y {dir_y:.3} \
             (upright anims 0.97..0.99; the pre-fix swing 0.31)");
    }
    assert!(checked >= 1, "the spot-3 placement has a strongly tracking head (head_yaw past ±0.9)");
}

/// Sister: the fix turns the head's ORIENTATION and nothing else — a moderately tracking head
/// (0.15 < |head_yaw| < 0.9, the 101c regime its head_look_live tests pin at a staged spot) still
/// aims its head forward at Bond's eye and keeps the head's world up near vertical, and the
/// attachment anchor (the hair rides the head bone) sits at the head, not at the collar: the
/// head-to-neck length equals the anim's.
#[test]
fn moderate_track_still_aims_the_head_at_bond() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000007.bin", false).expect("load");
    let sim = scene.sim.as_mut().unwrap();
    sim.bond = nf_game::player::Bond::new([41.71, 44.13, -72.05], -1.488);
    sim.bond.pitch = -0.154;
    for _ in 0..1102 { sim.step_bond(Input::default(), false, false, false); }
    let bp = sim.bond.pos;
    let mut checked = 0;
    for di in 0..sim.drones.list.len() {
        let (hy, dpos) = {
            let dr = &sim.drones.list[di];
            if !dr.present || dr.corpse { continue; }
            (dr.head_yaw, dr.pos)
        };
        if (dpos[0]-bp[0]).powi(2) + (dpos[2]-bp[2]).powi(2) > 36.0 { continue; }
        if hy.abs() < 0.15 || hy.abs() >= 0.9 { continue; }
        let (world, target) = {
            let dr = &sim.drones.list[di];
            match dr.palette(&sim.data()) { Some(w) => (w, sim.bond.head()), None => continue }
        };
        if world.len() <= 5 { continue; }
        let head = [world[5][3][0], world[5][3][1], world[5][3][2]];
        let neck = [world[2][3][0], world[2][3][1], world[2][3][2]];
        let off = [head[0]-neck[0], head[1]-neck[1], head[2]-neck[2]];
        let len = dot(off, off).sqrt();
        let fwd = norm(row(&world[5], 2));
        let up = norm(row(&world[5], 1));
        let to = norm([target[0]-head[0], target[1]-head[1], target[2]-head[2]]);
        let aim = dot(fwd, to).clamp(-1.0, 1.0).acos();
        let upy = up[1];
        println!("drone {di}: head_yaw {hy:.3} up.y {upy:.3} fwd-to-Bond {aim:.3} neck->head len {len:.3}");
        checked += 1;
        assert!(len <= 0.45, "the attachment anchor (hair) rides the head bone at the anim's neck position, got {len:.3}");
        assert!(upy >= 0.85, "the moderately tracked head stays upright (no roll), got up.y {upy:.3}");
        assert!(aim < 0.9, "the tracked head still aims its forward at Bond's eye (the track path runs), got {aim:.3} rad");
    }
    assert!(checked >= 1, "the spot-2 placement has a moderately tracking head (0.15 < |head_yaw| < 0.9)");
}
