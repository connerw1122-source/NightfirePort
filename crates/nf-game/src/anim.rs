//! The animation engine's pose side (docs/systems/anim-engine.md): a script's pose at a fractional time
//! (`AnimProcessScriptCmds` op 0 → `AnimSeqSet` → `AnimFrameSet`) and the blend of several playing scripts
//! (`AnimFrameResolve` → `AnimFrameBlend`).

use nf_formats::anim::{local_pose, Script, Sequence};
use nf_formats::character::{Skeleton, Skin};
use std::collections::HashMap;

/// Per bone: local translation and rotation quaternion (x, y, z, w).
pub type Pose = Vec<([f32; 3], [f32; 4])>;

/// `Quat_Slerp_Acc` (0x1e4188), verbatim [M work/decomp/ACTION.ELF/001e4188]: dot = q0·q1; `neg` = dot < 0 and |dot|
/// used; the slerp branch runs while `0.01 < 1 − dot` — ω = acosf(dot), ωt = ω·t, w1 = PS2Sinf(ωt)·(1/PS2Sinf(ω)),
/// w0 = PS2Sinf(ω−ωt)·(1/PS2Sinf(ω)) (a reciprocal multiply, one division); else linear w0 = 1−t, w1 = t. `neg`
/// negates **w0** (the q0 coefficient — NOT q1), and there is **no renormalisation**: out = w0·q0 + w1·q1 as is. A
/// rounded dot past 1.0 gives 1 − dot ≤ 0 < 0.01 and lands in the linear branch, so no clamp is needed.
pub fn slerp(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let mut d = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
    let neg = d < 0.0;
    if neg { d = -d; }
    let (s0, s1) = if 0.01 < 1.0 - d {
        let w = d.acos();
        let wt = w * t;
        let inv = 1.0 / crate::trig::sinf(w);
        (crate::trig::sinf(w - wt) * inv, crate::trig::sinf(wt) * inv)
    } else {
        (1.0 - t, t)
    };
    let s0 = if neg { -s0 } else { s0 };
    [0, 1, 2, 3].map(|k| a[k] * s0 + b[k] * s1)
}

/// The root translation at a fractional frame — the game's root motion never samples the quintics between frames:
/// `AnimSeqTick` (0x1c28d0) takes the blended pose buffer's root, which `AnimFrameSet` (0x1c75a0) filled as frames
/// ⌊f⌋ and ⌊f⌋+1 blended by `AnimFrameBlend` (0x1c2b80). So: single frame while |frac| ≤ 0.0002 or at the last
/// frame, else the two integer frames' translations lerped by the fraction, clamped into the next frame's direction
/// (AnimFrameBlend's translation rule).
pub fn root_translation_at(seq: &Sequence, frame: f32) -> [f32; 3] {
    let last = seq.frames.max(1) as u32;
    let f0 = (frame.floor().max(1.0) as u32).clamp(1, last);
    let frac = frame - frame.floor();
    let at = |f: u32| [seq.value(0, f), seq.value(1, f), seq.value(2, f)];
    let a = at(f0);
    if frac.abs() <= 0.0002 || f0 >= last { return a; }
    let b = at(f0 + 1);
    let mut r = a;
    for k in 0..3 {
        let d = b[k] - a[k];
        let v = a[k] + d * frac;
        r[k] = if (d >= 0.0 && v > b[k]) || (d < 0.0 && v < b[k]) { b[k] } else { v };
    }
    r
}

/// `AnimDistanceTableCreate(id, axisMask, looping)` (0x1c4060): the cumulative |root Δ| per frame a speed-mode-1
/// script plays by. Table = `[0]` total, `[1]` segment count N (frames, or frames−1 without `looping`), `[2]` 0.0,
/// then cum[k] for k = 1..=N−1 at index k+2 — segment k spans `[cum[k−1], cum[k]]` (`cum[0]` = the reserved 0.0).
/// Each segment's delta = the root translation at frame k+1 minus frame k (via `AnimFrameCopy`, i.e. the integer
/// quintic samples), components masked by `axis_mask` bits 0/1/2 = x/y/z; a segment under 1e-05 becomes 0.01
/// [M 001c4060: `fVar8 = 1e-05`, `if fVar7 < fVar8 → 0.01`]. The walk sets build z-only tables (axis mask 4,
/// `AnimDistanceTableGet` 0x1c76f0) with `looping` set [M AnimSeqSet 0x1c7460 attaches the table to seq+0x80].
pub struct DistanceTable {
    pub total: f32,
    pub segments: u32,
    /// cum[k] = distance walked by the end of segment k (index k−1 in the game's float array + 3).
    pub cum: Vec<f32>,
}

impl DistanceTable {
    pub fn build(seq: &Sequence, axis_mask: u32, looping: bool) -> Option<DistanceTable> {
        let frames = seq.frames.max(1) as i32;
        let n = if looping { frames } else { frames - 1 };
        if n < 2 { return None; }
        let at = |f: i32| {
            let f = f.clamp(1, frames) as u32;
            [seq.value(0, f), seq.value(1, f), seq.value(2, f)]
        };
        let mut cum = Vec::with_capacity((n - 1) as usize);
        let mut acc = 0.0;
        for k in 1..n {
            let (a, b) = (at(k), at(k + 1));
            let mut d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            if axis_mask & 1 == 0 { d[0] = 0.0; }
            if axis_mask & 2 == 0 { d[1] = 0.0; }
            if axis_mask & 4 == 0 { d[2] = 0.0; }
            let m = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            acc += if m < 1e-05 { 0.01 } else { m };
            cum.push(acc);
        }
        Some(DistanceTable { total: acc, segments: (n - 1) as u32, cum })
    }

    /// `AnimDistanceTableDistanceToFrame2` (0x1c4270): the speed a mode-1 script plays at this tick. `walked` is the
    /// walked accumulator (`animObj+0x6C` = obj+0xDC) plus the stride (`animObj+0x64` = obj+0xD4); it is wrapped into
    /// `[0, total)` and **stored back** (the decomp writes `param_2 + 0x6c`); the frame is the segment plus the
    /// distance into it; the returned speed is that frame minus the script time, wrapped in the stride's direction.
    pub fn distance_to_frame(&self, walked: &mut f32, time: f32, forward: bool) -> f32 {
        if self.segments == 0 || self.total <= 0.0 { return 0.0; }
        let mut w = *walked;
        while self.total <= w { w -= self.total; }
        while w < 0.0 { w += self.total; }
        // Segment search: the estimate then the walk (0x1c4270: i = (int)(w·N/total), down while w < cum[i−1], up
        // while cum[i] ≤ w).
        let mut i = ((w * self.segments as f32) / self.total) as i32;
        let cum_at = |k: i32| -> f32 { if k <= 0 { 0.0 } else { self.cum[(k as usize).min(self.cum.len()) - 1] } };
        while i > 0 && w < cum_at(i) { i -= 1; }
        while (i as u32) < self.segments && cum_at(i + 1) <= w { i += 1; }
        let seg_end = cum_at(i + 1);
        let seg_start = cum_at(i);
        let seg_len = seg_end - seg_start;
        let frame = (i + 1) as f32 + if seg_len > 0.0 { (w - seg_start) / seg_len } else { 0.0 };
        *walked = w;
        let speed = frame - time;
        if forward {
            if speed < 0.0 { self.total - time + frame } else { speed }
        } else if speed >= 0.0 {
            -(self.total - frame + time)
        } else {
            speed
        }
    }
}

/// `AnimScriptTick`'s speed for a script (`+0xB4`), which the tick then adds to the script time [M 001c2630]:
/// mode 0 — `speed × FRAME_RATE_MUL` (1.0 at the port's fixed 60 Hz); mode 1 — the distance table's frame minus the
/// time (flags 0x80000000); mode 2 — synced to the master script `+0x4C`: Δ = (M.time−1)/(M.len−1) − (t−1)/(len−1),
/// wrapped in the master speed's direction, `speed = Δ·(len−1) + 1.0`; mode 3 — `(len+1)·phase(+0x8C) − time` (no
/// writer found in ACTION.ELF); any other byte keeps the speed.
pub fn speed_for_mode(mode: u8, speed: f32, time: f32, length: f32, phase: f32, master: Option<(f32, f32, f32)>) -> f32 {
    match mode {
        0 => speed,
        1 => speed, // replaced by the distance table's `distance_to_frame` at the call site (needs the walked state)
        2 => match master {
            Some((m_time, m_len, m_speed)) if length > 1.0 && m_len > 1.0 => {
                let mn = (m_time - 1.0) / (m_len - 1.0);
                let tn = (time - 1.0) / (length - 1.0);
                let d = mn - tn;
                let d = if m_speed >= 0.0 {
                    if d < 0.0 { (1.0 - tn) + mn } else { d }
                } else if d >= 1.0 {
                    (tn + 1.0) - mn
                } else {
                    d
                };
                d * (length - 1.0) + 1.0
            }
            _ => speed,
        },
        3 => (length + 1.0) * phase - time,
        _ => speed,
    }
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

/// A script's pose at `time` (`None` when the script or its sequence is not loaded, or the sequence
/// cannot pose this skeleton).
pub fn script_pose(scripts: &HashMap<u32, Script>, seqs: &HashMap<u32, Sequence>, skel: &Skeleton, skin: &Skin, hash: u32, time: f32) -> Option<Pose> {
    let (q, f) = script_frame(scripts.get(&hash)?, time)?;
    // The skeleton-id match is the historical strictness; the pose machinery itself
    // (`local_pose_with`) reads the channels by the SKEL's translation mask, never by
    // `seq.skeleton`. Shared human tracks ride on other rigs' ids: 07000004's seated target
    // (skin 0x0500001e, skeleton 8) plays 0x0400028a, whose record byte is skeleton 0 — the
    // strict filter left him with no palette and no hit boxes (the bullets could never land).
    // [S the game's use of the seq's +0x3c byte is unverified; the channel-count guard keeps
    // short tracks (a 22-channel face track on a body need) rejected.]
    let seq = seqs.get(&q).filter(|s| {
        s.skeleton == skin.skeleton
            || {
                let need: usize = (0..skel.bones as usize).map(|b| if skel.has_translation(b) { 6 } else { 3 }).sum();
                s.channels.len() >= need
            }
    })?;
    Some(sequence_pose(seq, skel, skin, f))
}

/// The drone path's pose: every started opcode-0 window plays together (`AnimProcessScriptCmds`
/// 0x1c1b38 starts each window as its own playing anim) and per bone the LAST-started window that
/// covers it owns it — [L] the 07000006 truck driver (start script 0600011d, body 04000064 then the
/// 22-channel face track 04000065, both covering 1..880) holds his position to 0.01 over the whole
/// 700-frame trace window while the body track's root runs 0.57: the face track's all-zero root owns
/// his root bone, so the body's root motion never reaches the object. A single-window script (the
/// 07000001 guard's 06000169) keeps its root motion — [L] the same trace file's 01-doors segment
/// shows the guard moving 1.1 units inside his start script. Bones no window covers fall to the
/// widest one (the game's frame copy leaves them at the anim object's previous values). A shared
/// human track riding another rig's record byte (0x0400028a, record 0, on skeleton-8 skin —
/// drone-port-check #3) counts as full-coverage when its channels cover the skeleton. Never panics:
/// each window poses only the bones its channels reach (`local_pose_bones`).
pub fn script_pose_drone(scripts: &HashMap<u32, Script>, seqs: &HashMap<u32, Sequence>, skel: &Skeleton, skin: &Skin, hash: u32, time: f32) -> Option<Pose> {
    let script = scripts.get(&hash)?;
    let t = time.floor().max(1.0) as u16;
    let wins: Vec<(u16, u32)> = script.sequences().filter(|(a, _, _)| *a <= t).map(|(a, _, q)| (a, q)).collect();
    // The bones a window's channels reach; the shared-track arm mirrors `script_pose`'s filter.
    let covered = |q: u32| -> usize {
        let Some(s) = seqs.get(&q) else { return 0 };
        if s.skeleton == skel.id as u8 {
            nf_formats::anim::bones_covered(s, skel)
        } else {
            let need: usize = (0..skel.bones as usize).map(|b| if skel.has_translation(b) { 6 } else { 3 }).sum();
            if s.channels.len() >= need { skel.bones as usize } else { 0 }
        }
    };
    // The base: the widest started window — it also stands in for the bones no window reaches.
    let (base_i, base_n) = wins.iter().enumerate()
        .filter_map(|(i, (_, q))| { let n = covered(*q); (n > 0).then_some((i, n)) })
        .max_by_key(|&(_, n)| n)?;
    let (base_first, base_q) = wins[base_i];
    let base_f = (time - base_first as f32 + 1.0).max(1.0);
    let base_seq = seqs.get(&base_q)?;
    let mut acc = sequence_pose_n(base_seq, skel, skin, base_f, base_n);
    if acc.len() < skel.bones as usize {
        while acc.len() < skel.bones as usize { acc.push(*acc.last().unwrap()); }
    }
    // Then every started window in command order overwrites the bones it reaches — the last one
    // wins; the base's own pass (first pass over its bones) is its place in that order.
    for (first, q) in &wins {
        let n = covered(*q);
        if n == 0 { continue; }
        let Some(seq) = seqs.get(q) else { continue };
        let f = (time - *first as f32 + 1.0).max(1.0);
        let p = sequence_pose_n(seq, skel, skin, f, n);
        for (i, v) in p.iter().enumerate().take(n) { acc[i] = *v; }
    }
    Some(acc)
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

    /// `Quat_Slerp_Acc` negates the w0 coefficient and never renormalises [M 0x1e4188:30-36]: for opposite-w quats
    /// (the port's channel decode can produce both signs) the result keeps q1's sign — the old port negated q1
    /// instead and renormalised, giving +1 where the game gives −1.
    #[test]
    fn slerp_negates_w0_and_never_renormalises() {
        // dot = −1: |dot| = 1 → the linear branch (1 − 1 = 0 is not > 0.01); neg flips w0.
        let q = slerp([0.0, 0.0, 0.0, 1.0], [0.0, 0.0, 0.0, -1.0], 0.5);
        assert_eq!(q, [0.0, 0.0, 0.0, -1.0], "neg must negate w0 (the q0 weight), not q1");
        // No renormalisation: the game adds w0·q0 + w1·q1 as is (0x1e4188:33-36) — a length-2 quat stays length 2.
        let two = [0.0, 0.0, 0.0, 2.0];
        let q = slerp(two, two, 0.5);
        assert_eq!(q, two, "a renormalised result would shrink the length-2 quat");
        // The slerp branch (dot 0 → ω = π/2): w0 = w1 = PS2Sinf(π/4)·(1/PS2Sinf(π/2)) = √½.
        let id = [0.0, 0.0, 0.0, 1.0];
        let half_turn_y = [0.0, 1.0, 0.0, 0.0];
        let q = slerp(id, half_turn_y, 0.5);
        assert!((q[1] - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-5 && (q[3] - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-5, "{q:?}");
    }

    /// The distance table plays frames by walked distance [M 0x1c4060/0x1c4270]: equal walked steps advance the
    /// implied root distance equally — the feet-don't-slide property the walk sets need.
    #[test]
    fn distance_table_maps_walked_distance_to_frames() {
        use nf_formats::anim::{Sequence, Segment};
        // 5 frames whose root z walks 1.0 per frame (channels: z = t, t = frame − 1); every channel
        // carries a segment (`AnimFrameCopy` evaluates the tracks the sequence carries, and an
        // empty track's `segs.last()` has no neutral stand-in on the read side).
        let mk = |end: u32, z: f32| vec![Segment { end, c: [0.0, z, 0.0, 0.0, 0.0, 0.0] }];
        let seq = Sequence {
            hashcode: 1,
            flags: 0,
            frames: 5,
            skeleton: 0,
            channels: vec![mk(4, 0.0), mk(4, 0.0), mk(4, 1.0), mk(4, 0.0), mk(4, 0.0), mk(4, 0.0)],
        };
        let t = DistanceTable::build(&seq, 4, true).expect("a 5-frame looping walk has a table");
        assert_eq!(t.segments, 4, "looping → frames−1 segments [M 001c4060]");
        assert!((t.total - 4.0).abs() < 1e-4, "each frame walks 1.0, total 4.0, got {}", t.total);
        // walked 0.5 → halfway through segment 1 → frame 1.5; the frame advance per walked unit stays 1.
        let mut w = 0.5;
        let speed = t.distance_to_frame(&mut w, 1.0, true);
        assert!((speed - 0.5).abs() < 1e-5, "frame 1.5 − time 1.0 = 0.5, got {speed}");
        assert!((w - 0.5).abs() < 1e-6, "the wrapped walked value is stored back [M 001c4270]");
        // Wrapping: walked total + 0.25 plays like 0.25.
        let mut w = t.total + 0.25;
        let _ = t.distance_to_frame(&mut w, 1.0, true);
        assert!((w - 0.25).abs() < 1e-6, "walked wraps into [0, total), got {w}");
        // Backward stride wraps the other way (0x1c4270's negative branch).
        let mut w = 0.25;
        let speed = t.distance_to_frame(&mut w, 4.5, false);
        assert!(speed < 0.0, "a backward stride plays the walk in reverse, got {speed}");
    }

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
