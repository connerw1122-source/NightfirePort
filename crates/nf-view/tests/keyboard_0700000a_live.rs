//! The security keyboard (SCRIPTPLAYER_KeyboardMH, placement 60) activated through the REAL use
//! key — the level-0a-complete walk's objective-5 fix.
//!
//! Mechanism [M]: the SP's mode-64 preload (`SP_CreateScriptPlayer` 0x1db9a8's load beat +
//! `SP_LoadScript` 0x1dbc08's mode dispatch) runs script 0x06000597's t=0 Entity op, so the
//! keyboard prop (glist hash 0x020005E1, `Script_EntityStart` 0x1d8d78's obj+0xE4 = the owner SP)
//! exists from level load with its baked-absolute model sphere at (−1.54, 1.33, 8.48) r 0.66 —
//! `Control_BuildWorldSph` 0x133070 multiplies the model's bounds by the (identity) entity matrix.
//! The E pick (`Player_Activate` 0x1a5170's class-0 +0xE4 dispatch → `SP_Activate` 0x1dc338's
//! `ext+0x2C = 1`) reaches it from the corridor wedge at (−3.00, 1.10, 9.68): the pick sphere is
//! head+look vs the entity sphere (1.0 + r), and the wedge's head+look sits ~0.94 from the centre.
//! `SP_Update` 0x1dbda8's state-1 activation plays the script and toggles the out channel
//! (`switch_channels[0x3A] = 1`) [M]. The dispatch runs on the PRESS: a held use key re-activates
//! each tick and every activation TOGGLES the out channel [M 0x1dbda8's bottom write], so the test
//! pulses the key and stops once ch 0x3A reads on — the way the game's tap works.
//!
//! Sister: on the old tree this test fails — the walkreal drove ch 0x3A with `set_channel` and no
//! real E path to the keyboard was exercised (the prior session's "the game's own use ray has no
//! target" premise missed the mode-64 preload's entity, the elevator-buttons-09 mechanism).

#[path = "common/walker_b2.rs"]
mod walker;

use walker::{Walker, Wp};

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

fn load() -> Walker {
    let cold = nf_view::load_with_carry(&iso(), "0700000a.bin", false, None).expect("load 0700000a cold");
    let sim = cold.sim.as_ref().unwrap();
    let save = nf_game::player::RamSave::capture(&sim.drones.health, sim.gun.switch_request, &sim.drones.inventory, &sim.drones.channels);
    Walker::new(nf_view::load_with_carry(&iso(), "0700000a.bin", false, Some(&save)).expect("load 0700000a warm"))
}

#[test]
fn keyboard_sp_activates_through_the_use_key() {
    let mut w = load();
    w.wait(150); // the SS lift doors' create sweep (the walkreal's wait)
    // Out of the lift cab south, then hug the cab's east wall to its south end — the wall (x ≈ −2.0,
    // z 8.5–12.5 [D probe segments]) stops the direct east walk, and Bond wedges at
    // (−3.00, 1.10, 9.68) rounding it (the lenient walk reports the wedge and continues).
    w.walk_to(Wp::new(-3.9, 0.0, 13.2), "out of the lift");
    w.walk_to(Wp::new(-3.2, 0.0, 9.4).r(1.0), "hug the cab east wall south end");
    let _ = w.try_walk_to(Wp::new(-2.55, 0.0, 8.1).r(1.0), "round the wall end");
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!((p[0] + 3.0).abs() < 0.3 && (p[2] - 9.68).abs() < 0.3,
            "the corridor wedge in front of the keyboard nook, got {p:?}");
    }
    // The keyboard's entity sphere [D the preload's target — the probe prints it] at
    // (−1.54, 1.33, 8.48) r 0.66; pulse the use key aimed at it.
    let target: [f32; 3] = [-1.54, 1.33, 8.48];
    let mut fired = false;
    for _cycle in 0..12u32 {
        for t in 0..40u32 {
            let sim = w.scene.sim.as_ref().unwrap();
            let bp = sim.bond.pos;
            let eye = bp[1] + sim.bond.eye - sim.bond.crouch_drop;
            let dx = target[0] - bp[0];
            let dz = target[2] - bp[2];
            let dy = target[1] - eye;
            let desired = dx.atan2(dz);
            let err = walker::wrap_pi(desired - sim.bond.yaw);
            let perr = (dy / walker::dxz(bp, target).max(0.5)).atan() / std::f32::consts::FRAC_PI_2 - sim.bond.pitch;
            let input = nf_game::player::Input {
                turn: err.clamp(-0.05, 0.05),
                look: perr.clamp(-0.05, 0.05),
                ..Default::default()
            };
            let hold = t < 3 && err.abs() < 0.06;
            w.scene.sim.as_mut().unwrap().step_bond(input, false, false, hold);
            w.total_ticks += 1;
        }
        if w.scene.sim.as_ref().unwrap().drones.channels.on(0x3a) { fired = true; break; }
    }
    assert!(fired, "the keyboard SP's E activation writes its out channel 0x3A \
        [M SP_Activate 0x1dc338 + SP_Update 0x1dbda8 state 1]");
}
