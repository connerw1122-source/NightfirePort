//! The animation engine's pose side (docs/systems/anim-engine.md): a script's pose at a fractional time
//! (`AnimProcessScriptCmds` op 0 → `AnimSeqSet` → `AnimFrameSet`) and the blend of several playing scripts
//! (`AnimFrameResolve` → `AnimFrameBlend`).

use nf_formats::anim::{local_pose, Script, Sequence};
use nf_formats::character::{Skeleton, Skin};
use std::collections::HashMap;

/// Per bone: local translation and rotation quaternion (x, y, z, w).
pub type Pose = Vec<([f32; 3], [f32; 4])>;

/// `Quat_Slerp_Acc` (0x1e4188): spherical interpolation along the shorter arc; nearly parallel → linear.
pub fn slerp(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let mut d = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
    let b = if d < 0.0 { d = -d; b.map(|v| -v) } else { b };
    let (s0, s1) = if d > 0.9995 { (1.0 - t, t) } else {
        let th = d.clamp(-1.0, 1.0).acos();
        let s = th.sin();
        (((1.0 - t) * th).sin() / s, (t * th).sin() / s)
    };
    let q = [0, 1, 2, 3].map(|k| a[k] * s0 + b[k] * s1);
    let l = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt().max(1e-12);
    q.map(|v| v / l)
}

/// `AnimFrameBlend(t, out, a, b)`: per bone, translation a + (b − a)·t without overshooting b, rotation slerp; t = 0
/// keeps a, t = 1 copies b.
pub fn blend(a: &mut Pose, b: &Pose, t: f32) {
    if t == 0.0 { return; }
    if t == 1.0 { a.clone_from(b); return; }
    for (x, y) in a.iter_mut().zip(b) {
        for k in 0..3 {
            let d = y.0[k] - x.0[k];
            let v = x.0[k] + d * t;
            x.0[k] = if (d >= 0.0 && v > y.0[k]) || (d < 0.0 && v < y.0[k]) { y.0[k] } else { v };
        }
        x.1 = slerp(x.1, y.1, t);
    }
}

/// The sequence and fractional frame a script shows at script time `time` (from 1): the opcode-0 command covering
/// ⌊time⌋, frame = time − first + 1 (`AnimSeqSet` / `AnimSeqTick`); the last command past the end. When several
/// opcode-0 windows cover the time (e.g. the 07000005 cut-scene scripts pair a 22-channel face track with the
/// 222-channel body track), `AnimProcessScriptCmds` (0x1c1b38) starts **each** as a playing anim in command order
/// and the palette lets the later-started one own the shared bones — so the **last** covering command wins.
pub fn script_frame(script: &Script, time: f32) -> Option<(u32, f32)> {
    let t = time.floor().max(1.0) as u16;
    let (a, _, q) = script.sequences().filter(|(a, b, _)| *a <= t && t <= *b).last().or_else(|| script.sequences().last())?;
    Some((q, (time - a as f32 + 1.0).max(1.0)))
}

/// `AnimFrameSet`: the pose at fractional frame f — frames ⌊f⌋ and ⌊f⌋ + 1 (clamped to the last) blended by the
/// fraction.
pub fn sequence_pose(seq: &Sequence, skel: &Skeleton, skin: &Skin, frame: f32) -> Pose {
    let last = seq.frames.max(1) as u32;
    let f0 = (frame.floor() as u32).clamp(1, last);
    let frac = (frame - frame.floor()).clamp(0.0, 1.0);
    let mut p = local_pose(seq, skel, skin, f0);
    if frac > 0.0002 && f0 < last {
        let q = local_pose(seq, skel, skin, f0 + 1);
        blend(&mut p, &q, frac);
    }
    p
}

/// A script's pose at `time` (`None` when the script or its sequence is not loaded, or the sequence belongs to
/// another skeleton).
pub fn script_pose(scripts: &HashMap<u32, Script>, seqs: &HashMap<u32, Sequence>, skel: &Skeleton, skin: &Skin, hash: u32, time: f32) -> Option<Pose> {
    let (q, f) = script_frame(scripts.get(&hash)?, time)?;
    let seq = seqs.get(&q).filter(|s| s.skeleton == skin.skeleton)?;
    Some(sequence_pose(seq, skel, skin, f))
}

/// A script's pose with EVERY started opcode-0 window playing together. `AnimProcessScriptCmds`
/// (0x1c1b38) starts each window whose time the script enters as its own playing anim — the pack
/// cut-scene scripts pair a narrow 22-channel face track with the full 222-channel body track
/// (060004f1/060004f2 of the 07000007 Kimiko scene: windows [1..0x10e6] seq …467/…468 then
/// [1..0x10e7] seq …332/…3f9; the 07000005 ledge's 06000996/97: [1..0x5dc] 040007fa then
/// [1..0x579] 040007f8) [D `nf-extract script-info` dumps]. Per bone the FIRST-started window that
/// covers it owns it and the later windows fill only the bones the earlier ones do not reach [S:
/// the direction follows the play reference — the game's mouths move while the body animates, which
/// the old single-window read (last wins, fix-drone2) cannot do; both windows start at frame 1 here,
/// so command order breaks the tie]. A window past its end holds its last frame (`AnimScriptTick`
/// 0x1c2630); `sequence_pose`'s clamp provides that. The AI/drone path keeps `script_pose`
/// (last-wins, fix-drone2's pinned behaviour) — this is the cut-scene character path only.
pub fn script_pose_together(scripts: &HashMap<u32, Script>, seqs: &HashMap<u32, Sequence>, skel: &Skeleton, skin: &Skin, hash: u32, time: f32) -> Option<Pose> {
    let script = scripts.get(&hash)?;
    let t = time.floor().max(1.0) as u16;
    let wins: Vec<(u16, u32)> = script.sequences().filter(|(a, _, _)| *a <= t).map(|(a, _, q)| (a, q)).collect();
    // The base: the widest started window on this skeleton (the body track), at its own frame — it
    // also stands in for the bones no window reaches.
    let (base_i, base_n) = wins.iter().enumerate()
        .filter_map(|(i, (_, q))| seqs.get(q).filter(|s| s.skeleton == skin.skeleton).map(|s| (i, nf_formats::anim::bones_covered(s, skel))))
        .max_by_key(|&(_, n)| n)?;
    if base_n == 0 { return None; }
    let (base_first, base_q) = wins[base_i];
    let base_f = (time - base_first as f32 + 1.0).max(1.0);
    // A single window that does not fit the skeleton (the old `fits` filter — e.g. a lone narrow
    // track): the character hid before this function existed; keep that.
    let Some(base_seq) = seqs.get(&base_q) else { return None };
    if !nf_formats::anim::fits(base_seq, skel) { return None; }
    let mut acc = sequence_pose_n(base_seq, skel, skin, base_f, base_n);
    if acc.len() < skel.bones as usize {
        // Pad to the full bone count with the base's last bone pose (never read: the body covers all).
        while acc.len() < skel.bones as usize { acc.push(*acc.last().unwrap()); }
    }
    let mut owned = vec![false; base_n];
    for (first, q) in &wins {
        let Some(seq) = seqs.get(q) else { continue };
        if seq.skeleton != skin.skeleton { continue; }
        let n = nf_formats::anim::bones_covered(seq, skel).min(base_n);
        if n == 0 { continue; }
        let f = (time - *first as f32 + 1.0).max(1.0);
        let p = sequence_pose_n(seq, skel, skin, f, n);
        for (i, v) in p.iter().enumerate().take(n) {
            if !owned[i] { acc[i] = *v; owned[i] = true; }
        }
    }
    Some(acc)
}

/// `sequence_pose` for the first `bones` bones only (a narrow sequence — the face tracks — whose
/// channel list `local_pose` would read past).
pub fn sequence_pose_n(seq: &Sequence, skel: &Skeleton, skin: &Skin, frame: f32, bones: usize) -> Pose {
    let last = seq.frames.max(1) as u32;
    let f0 = (frame.floor() as u32).clamp(1, last);
    let frac = (frame - frame.floor()).clamp(0.0, 1.0);
    let mut p = nf_formats::anim::local_pose_bones(seq, skel, skin, f0, bones);
    if frac > 0.0002 && f0 < last {
        let q = nf_formats::anim::local_pose_bones(seq, skel, skin, f0 + 1, bones);
        blend(&mut p, &q, frac);
    }
    p
}

/// A script that is fading out (`AnimScriptAppendBlend`: fade −1, flags 0x500000): it keeps playing while its blend
/// clock runs down; weight = clock / length.
#[derive(Clone, Debug)]
pub struct Outgoing {
    pub script: u32,
    /// Script time (from 1) and speed (frames per tick); a one-shot stops at its end, a looping script wraps.
    pub time: f32,
    pub speed: f32,
    pub looping: bool,
    pub clock: f32,
    pub length: f32,
    /// Root yaw moved into the object (anim rotation, `+0x78 & 0x20000`): removed from this script's root.
    pub unyaw: f32,
}

impl Outgoing {
    pub fn weight(&self) -> f32 { if self.length > 0.0 { (self.clock / self.length).clamp(0.0, 1.0) } else { 0.0 } }

    /// One tick (`AnimScriptTick` + `AnimScriptEnd` + the fade in `AnimFrameResolve`); false once faded out.
    pub fn tick(&mut self, len: f32) -> bool {
        if len >= 1.0 {
            let t = self.time + self.speed;
            self.time = if t <= len { t } else if self.looping { t - len } else { len };
        }
        self.clock -= 1.0;
        self.clock > 0.0
    }
}

/// The yaw (about +Y) of a root rotation: its forward (+Z) turned, `atan2(x, z)` (`Mat_GetDir` + `Vec_DirToRot`).
pub fn root_yaw(q: [f32; 4]) -> f32 {
    let [x, y, z, w] = q;
    let f = [2.0 * (x * z + w * y), 1.0 - 2.0 * (x * x + y * y)];
    f[0].atan2(f[1])
}

/// Removes `yaw` about +Y from a root rotation (pre-multiplies by the inverse turn).
pub fn unyaw(q: [f32; 4], yaw: f32) -> [f32; 4] {
    let (s, c) = ((-yaw * 0.5).sin(), (-yaw * 0.5).cos());
    let r = [0.0, s, 0.0, c];
    // r ⊗ q
    [r[3] * q[0] + r[0] * q[3] + r[1] * q[2] - r[2] * q[1],
     r[3] * q[1] - r[0] * q[2] + r[1] * q[3] + r[2] * q[0],
     r[3] * q[2] + r[0] * q[1] - r[1] * q[0] + r[2] * q[3],
     r[3] * q[3] - r[0] * q[0] - r[1] * q[1] - r[2] * q[2]]
}

/// `AnimFrameResolve`'s order: the outgoing scripts (oldest first) then the current one; result = first, then for each
/// next: blend(result, next, 1 − weight(previous)). The current script has full weight.
pub fn blend_playing(current: Pose, outgoing: &[(Pose, f32)]) -> Pose {
    let Some(((first, _), rest)) = outgoing.split_first() else { return current };
    let mut out = first.clone();
    let mut prev_w = outgoing[0].1;
    for (p, w) in rest {
        blend(&mut out, p, 1.0 - prev_w);
        prev_w = *w;
    }
    blend(&mut out, &current, 1.0 - prev_w);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slerp_and_blend_follow_the_weights() {
        let id = [0.0, 0.0, 0.0, 1.0];
        let half_turn_y = [0.0, 1.0, 0.0, 0.0];
        let q = slerp(id, half_turn_y, 0.5);
        assert!((q[1] - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-5 && (q[3] - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-5, "{q:?}");
        let a: Pose = vec![([0.0; 3], id)];
        let b: Pose = vec![([2.0, 0.0, 0.0], half_turn_y)];
        // An outgoing script at weight 0.75 under the current one: 25 % of the way to the current pose.
        let p = blend_playing(b.clone(), &[(a.clone(), 0.75)]);
        assert!((p[0].0[0] - 0.5).abs() < 1e-6);
        // Faded out (weight 0): the current pose; no outgoing: the current pose.
        assert_eq!(blend_playing(b.clone(), &[(a, 0.0)])[0].0, b[0].0);
        // An outgoing one-shot holds its last frame while fading over 8 ticks.
        let mut o = Outgoing { script: 0, time: 9.0, speed: 1.0, looping: false, clock: 8.0, length: 8.0, unyaw: 0.0 };
        let mut n = 0;
        while o.tick(10.0) { n += 1; }
        assert_eq!((n, o.time), (7, 10.0));
    }
}
