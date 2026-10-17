//! script-negative-delay (task 732, GAPS G-047): the 0700001b solar panels. The 16
//! `ScriptPlayer_SolarPanel` SPs (placements 41-56, type 0xD9, mode 1, hit points 20 = param 7
//! [D the placements]) play script 0x06000832; its stream 11's Event 3 p0 = 0 at t0 sets
//! `SCRIPTINFO+0x9C` to −1 and a NEGATIVE delay HOLDS the script [M Script_Update 0x1da9c4-
//! 0x1da9dc: `beql` on zero runs, `blez` returns, only a positive delay decrements] until
//! `SP_UnPause` 0x1dca58 clears it — for the panels, SP_Update's damage path (0x1dc1c0-0x1dc330)
//! accumulating `SP_GetHitDamage` 0x1dc5f8 to the 20 hit points. Opt-in via NF_ISO (the slim ISO
//! is enough).

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

use nf_game::nis::Obj;
use nf_game::player::Input;

const PANEL: u32 = 0x0200_0A42; // OBJECT_SolarPanelWhole (glist 0x81) [D the script's stream 12]
const SCRIPT: u32 = 0x0600_0832;

fn panel_places(sim: &nf_view::scene::Sim) -> Vec<[f32; 3]> {
    sim.drones.scripts.players.iter().filter(|p| p.scripts[0] == SCRIPT).map(|p| [p.matrix[3][0], p.matrix[3][1], p.matrix[3][2]]).collect()
}

fn panels(sim: &nf_view::scene::Sim) -> Vec<([f32; 3], usize)> {
    let mut out = Vec::new();
    for (oi, o) in sim.drones.scripts.objects.list.iter().enumerate() {
        if let Some(Obj::Entity { hash, matrix, .. }) = o {
            if *hash == PANEL { out.push(([matrix[3][0], matrix[3][1], matrix[3][2]], oi)); }
        }
    }
    out
}

/// The debris pieces (the script's streams 0-9 glists) within 3 units of `at`.
fn debris_near(sim: &nf_view::scene::Sim, at: [f32; 3]) -> usize {
    let mut n = 0;
    for o in &sim.drones.scripts.objects.list {
        if let Some(Obj::Entity { hash, matrix, src, .. }) = o {
            if src.0 == SCRIPT && src.1 < 10 && *hash >= 0x0200_0848 && *hash <= 0x0200_0852 {
                let d = [matrix[3][0] - at[0], matrix[3][1] - at[1], matrix[3][2] - at[2]];
                if d[0] * d[0] + d[1] * d[1] + d[2] * d[2] < 9.0 { n += 1; }
            }
        }
    }
    n
}

#[test]
fn the_16_solar_panels_hold_in_their_girder_frames() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "0700001b.bin", false).expect("load 0700001b");
    let sim = scene.sim.as_mut().unwrap();
    for _ in 0..600 { sim.step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    let places = panel_places(sim);
    assert_eq!(places.len(), 16, "the 16 ScriptPlayer_SolarPanel placements [D 41-56]");
    let got = panels(sim);
    assert_eq!(got.len(), 16, "all 16 panels present after 600 ticks, got {}", got.len());
    for t in &places {
        assert!(got.iter().any(|(q, _)| (q[0] - t[0]).abs() < 0.05 && (q[1] - t[1]).abs() < 0.05 && (q[2] - t[2]).abs() < 0.05),
            "a panel sits at its SP placement {t:?} (the held entity takes its first key [M Script_EntityStart 0x1d8d78 → Script_Interp])");
    }
    // Each panel player is HELD: state 2, the negative delay, hit points 20 [D param 7].
    for p in sim.drones.scripts.players.iter().filter(|p| p.scripts[0] == SCRIPT) {
        let s = p.info[p.cur].as_ref().expect("the panel script is loaded");
        assert_eq!(p.state, 2, "SP #{} plays (the mode-1 state-7 load beat)", p.placement);
        assert_eq!(s.delay, -1, "SP #{} is HELD on the negative delay (stream 11's event 3 p0 = 0 [D])", p.placement);
        assert_eq!(p.hit_points, 20.0, "SP #{} carries the 20 hit points (param 7 [D])", p.placement);
    }
}

#[test]
fn shooting_one_panel_shatters_only_that_one() {
    let iso = iso();
    let mut scene = nf_view::load(&iso, "0700001b.bin", false).expect("load 0700001b");
    let target = 2; // placement 43: (23.277, −4.04, 4.623) — the arrival platform's west panel
    let (at, pi43);
    {
        let sim = scene.sim.as_mut().unwrap();
        for _ in 0..600 { sim.step_bond(Input::default(), false, false, false); }
        pi43 = sim.drones.scripts.players.iter().position(|p| p.scripts[0] == SCRIPT && p.placement == 43).expect("SP #43");
        let places = panel_places(sim);
        at = places[target];
        // Bond stands on the arrival floor above the panel's girder frame, then aims down at it.
        sim.bond = nf_game::player::Bond::stand_at(&sim.world, [at[0], -1.9, at[2] - 2.6], 0.0);
    }
    // Aim + tap the trigger through the real fire path (the gun, the bullets, the contacts),
    // walker-style: turn/look onto the panel centre from the eye, tap when on target, reload
    // when the clip runs dry.
    let mut shattered = false;
    let mut last_tap = 0u32;
    for t in 0..3000u32 {
        let (input, trigger, reload) = {
            let sim = scene.sim.as_ref().unwrap();
            let (bp, dp) = (sim.bond.pos, at);
            let eye = [bp[0], bp[1] + sim.bond.eye - sim.bond.crouch_drop, bp[2]];
            let dx = dp[0] - eye[0];
            let dy = dp[1] - eye[1];
            let dz = dp[2] - eye[2];
            let dxz = (dx * dx + dz * dz).sqrt().max(0.5);
            let desired = dx.atan2(dz);
            let err = { let mut e = desired - sim.bond.yaw; while e > std::f32::consts::PI { e -= 2.0 * std::f32::consts::PI; } while e < -std::f32::consts::PI { e += 2.0 * std::f32::consts::PI; } e };
            let want_pitch = (dy / dxz).atan() / std::f32::consts::FRAC_PI_2;
            let perr = want_pitch - sim.bond.pitch;
            let clip_empty = sim.gun.clip == 0 && sim.gun.spare > 0;
            let on_target = err.abs() < 0.02 && perr.abs() < 0.028 && t.saturating_sub(last_tap) > 15;
            (Input { turn: err.clamp(-0.08, 0.08), look: perr.clamp(-0.08, 0.08), ..Default::default() },
             on_target && !clip_empty,
             clip_empty && t % 20 == 0)
        };
        if trigger { last_tap = t; }
        scene.sim.as_mut().unwrap().step_bond(input, trigger, reload, false);
        let sim = scene.sim.as_ref().unwrap();
        let p = &sim.drones.scripts.players[pi43];
        if p.info[p.cur].as_ref().map_or(true, |s| s.delay != -1) { shattered = true; break; }
    }
    assert!(shattered, "enough bullets unpause the panel (SP_GetHitDamage to the 20 hit points [M 0x1dc1c0-0x1dc330])");

    // The shatter: the panel object dies, its 10 debris pieces play (streams 0-9, from t5),
    // and the other 15 panels are untouched — still held, still whole.
    for _ in 0..20 { scene.sim.as_mut().unwrap().step_bond(Input::default(), false, false, false); }
    let sim = scene.sim.as_ref().unwrap();
    let places = panel_places(sim);
    let got = panels(sim);
    assert_eq!(got.len(), 15, "15 panels remain after one shatters, got {}", got.len());
    assert!(!got.iter().any(|(q, _)| (q[0] - at[0]).abs() < 0.05 && (q[2] - at[2]).abs() < 0.05),
        "the shot panel's object is gone");
    let d = debris_near(sim, at);
    assert!(d >= 8, "the shot panel's debris plays near its frame (10 pieces [D streams 1-10]), got {d}");
    for p in sim.drones.scripts.players.iter().filter(|p| p.scripts[0] == SCRIPT && p.placement != 43) {
        let s = p.info[p.cur].as_ref().expect("the other panels stay loaded");
        assert_eq!(s.delay, -1, "SP #{} is still HELD — only the shot one resumed", p.placement);
    }
    assert!(places.len() == 16, "the SPs all remain");
}
