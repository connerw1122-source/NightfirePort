//! searchlight-flip-06: the 07000006 courtyard searchlights' beam basis must stay CONTINUOUS tick
//! to tick over the sweep (playtest 1013-1825: "sometimes a spotlight will cover a piece of ground
//! and it will like flip on its side"). Driven through the real path (`nf_view::load` +
//! `Sim::step_bond`), Bond parked inside head #2's sweep so the aim crosses the Bond−head direction
//! every period — the `Mat_Align2Dir` 0x1df2a8 near-degenerate window (`|up·row2| > 0.999`
//! [M 0x1df2a8: the 0x3f7fbe77 threshold]; the searchlight beam passes up = fallback = Bond − head
//! [M Searchlight_Update 0x1cb900:279-281]). Opt-in via NF_ISO.

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The beam spot of the 06-searchlight.trace's opening (Bond inside head #2's sweep at the courtyard
/// entrance) — the same spot fix_searchlight_alarm_live uses.
const BEAM_SPOT: [f32; 3] = [4.8, 42.09, -16.0];

fn ang(a: [f32; 3], b: [f32; 3]) -> f32 {
    let d = a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let la = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    let lb = (b[0] * b[0] + b[1] * b[1] + b[2] * b[2]).sqrt();
    if la <= 0.0 || lb <= 0.0 { return -1.0; } // degenerate row (the game's zeroed cross)
    (d / (la * lb)).clamp(-1.0, 1.0).acos()
}

#[test]
fn every_06_beam_basis_stays_continuous_over_the_sweep() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000006.bin", false).expect("load 07000006");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..300 { sim.step_bond(Default::default(), false, false, false); if !sim.disabled { break; } }
    assert!(!sim.disabled, "Bond is free after the intro");
    // Light the heads (the boot NIS stages the on-channels [D 0x06000066 stream 0]).
    let mut lit = false;
    for _ in 0..240u32 {
        sim.step_bond(Default::default(), false, false, false);
        if sim.drones.searchlights.parts.iter().any(|p| p.kind == 1 && p.state == 0) { lit = true; break; }
    }
    assert!(lit, "a courtyard head sweeps (the boot raised its on-channel)");
    // Bond parks inside head #2's sweep: the aim crosses his direction every period — the
    // near-degenerate window — and the spot's pass may arm the tracker (state 1), the playtest's
    // sustained-tracking case.
    sim.bond.pos = BEAM_SPOT;

    let n = sim.drones.searchlights.parts.len();
    let head_of: Vec<Option<usize>> = {
        let parts = &sim.drones.searchlights.parts;
        (0..n).map(|i| parts.iter().position(|p| p.links[3] == Some(i))).collect()
    };
    // The beam basis is only RECOMPUTED while its head sweeps/tracks (state 0/1); across the
    // light-up (3→0) and the re-sweep (1→0) the aim legitimately jumps (the sweep phase resumes),
    // so only same-state active tick pairs assert continuity.
    let mut prev: Vec<Option<[[f32; 3]; 3]>> = vec![None; n];
    let mut prev_state: Vec<u8> = vec![0xff; n];
    let mut active_run: Vec<u32> = vec![0; n];
    let mut violations: Vec<String> = Vec::new();
    let mut tracked = false;
    for t in 0..1800u32 {
        sim.step_bond(Default::default(), false, false, false);
        if sim.drones.searchlights.parts.iter().any(|p| p.kind == 1 && p.state == 1) { tracked = true; }
        for (i, p) in sim.drones.searchlights.parts.iter().enumerate() {
            // The flip lives in the align2dir parts: head 1, mount 0, beam 3 (the spot's Euler
            // mapping is the fix-searchlight-remainder reading, out of scope here).
            if !matches!(p.kind, 0 | 1 | 3) { continue; }
            let rows = [[p.matrix[0][0], p.matrix[0][1], p.matrix[0][2]],
                        [p.matrix[1][0], p.matrix[1][1], p.matrix[1][2]],
                        [p.matrix[2][0], p.matrix[2][1], p.matrix[2][2]]];
            let hstate = head_of[i].map(|h| sim.drones.searchlights.parts[h].state).unwrap_or(0xff);
            let active = matches!(hstate, 0 | 1);
            active_run[i] = if active { active_run[i] + 1 } else { 0 };
            if let (Some(pr), Some(ps)) = (prev[i], prev_state.get(i).copied()) {
                // active_run >= 3 on BOTH ends: the turn-on tick builds the beam from the stale
                // state-3 angles and the next tick's first sweep jumps to the phase position —
                // the game's own case-3 fall-through does exactly that [M 0x1cb900 case 3 tail],
                // so the light-up instant is not a sweep flip.
                if ps != hstate || !active || active_run[i] < 3 {
                    prev[i] = Some(rows); prev_state[i] = hstate; continue;
                }
                // The direction axis (row 2) and the two side axes must not jump: the sweep moves
                // the basis by ~half·2π/480 ≈ 0.006·half rad per tick, so anything beyond ~0.35 is
                // a snap. A row going degenerate↔valid (either side ~zero) is the game's own
                // zeroed-cross path [M 0x1df2a8 degenerate stores], not a flip.
                for r in 0..3 {
                    let (a, b) = (pr[r], rows[r]);
                    let la = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
                    let lb = (b[0] * b[0] + b[1] * b[1] + b[2] * b[2]).sqrt();
                    if la <= 1e-6 || lb <= 1e-6 { continue; } // degenerate on either side
                    let c = ang(a, b);
                    if c > 0.35 {
                        violations.push(format!(
                            "t {t} part {i} kind {} row {r}: angle {c:.3} between {:?} and {:?}",
                            p.kind, a, b));
                    }
                }
            }
            prev[i] = Some(rows);
            prev_state[i] = hstate;
        }
        if !violations.is_empty() { break; }
    }
    for v in violations.iter().take(12) { eprintln!("[flip] {v}"); }
    eprintln!("[flip] tracked={tracked} violations={}", violations.len());
    assert!(violations.is_empty(),
        "a searchlight part's basis snapped tick-to-tick (the playtest's 'flip on its side'); \
         first: {}", violations[0]);
}

/// Sister: a synthetic straight-down beam uses the game's `Mat_Align2Dir` fallback (0x1df2a8).
/// With a REAL fallback (the game's other callers pass CONSTANT_UP1/UP2 0x2d6210/0x2d6220 [M
/// symtab; Bullet_init 0x12c5a0]), the straight-down beam gets a well-defined basis from the
/// fallback: |up·row2| = 1 > 0x3f7fbe77 fires the fallback branch and row 0 = normalize(cross(
/// fallback, row2)) = [1,0,0] for fallback [0,0,1]. With the searchlight beam's own arguments
/// (up = fallback = Bond − head [M Searchlight_Update 0x1cb900 disasm 0x1cc020: `daddu a3,a2`])
/// a exactly-parallel up makes the cross degenerate and the game ZEROES the rows [M 0x1df2a8
/// disasm 0x1df3a8-0x1df3e0] — the old [1,0,0]/[0,1,0] stand-ins asserted world axes instead.
#[test]
fn a_straight_down_beam_uses_the_games_fallback() {
    use nf_game::searchlight::align2dir;
    // 1) the fallback branch with a real fallback vector: a stable basis, row 2 the aim.
    let m = align2dir([0.0, -1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]);
    assert_eq!(m[2][0..3], [0.0, -1.0, 0.0], "row 2 = the aim");
    assert_eq!(m[0][0..3], [1.0, 0.0, 0.0], "row 0 = normalize(cross(fallback, row2))");
    assert_eq!(m[1][0..3], [0.0, 0.0, 1.0], "row 1 = normalize(cross(row2, row0))");
    // 2) the searchlight beam's own arguments at exact parallel: the game's zeroed rows.
    let m = align2dir([0.0, -1.0, 0.0], [0.0, -1.0, 0.0], [0.0, -1.0, 0.0]);
    assert_eq!(m[2][0..3], [0.0, -1.0, 0.0], "row 2 = the aim");
    assert_eq!(m[0][0..3], [0.0, 0.0, 0.0], "the degenerate cross zeroes row 0 [M 0x1df2a8]");
    assert_eq!(m[1][0..3], [0.0, 0.0, 0.0], "cross(row2, 0) zeroes row 1 [M 0x1df2a8]");
    // 3) near-parallel but not exact: the tiny cross is normalized to its own (exact-FP)
    //    direction — the sound continuous basis the game keeps — never snapped to a world axis.
    let m = align2dir([0.01f32, -1.0, 0.0], [0.0, -1.0, 0.0], [0.0, -1.0, 0.0]);
    let l0 = (0.01f32 * 0.01 + 1.0).sqrt();
    let z: [f32; 3] = [0.01 / l0, -1.0 / l0, 0.0];
    // cross(up, z) with up = [0,-1,0]: (u.y·z.z − u.z·z.y, u.z·z.x − u.x·z.z, u.x·z.y − u.y·z.x)
    let (ux, uy, uz) = (0.0f32, -1.0f32, 0.0f32);
    let cross: [f32; 3] = [uy * z[2] - uz * z[1], uz * z[0] - ux * z[2], ux * z[1] - uy * z[0]];
    let l = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
    let r0 = [cross[0] / l, cross[1] / l, cross[2] / l];
    for k in 0..3 {
        assert!((m[0][k] - r0[k]).abs() < 1e-6,
            "row 0 = the sound tiny-cross direction (got {:?}, want {:?})", m[0], r0);
    }
}
