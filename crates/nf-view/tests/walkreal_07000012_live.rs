//! walkreal-b: 07000012 (HT Tower2 B, the lower offices) walked with the player's own movement —
//! the route lives in `common/tower2.rs` (shared with the 07000013 warm chain). Every leg is the
//! real input path except the ONE documented stand-in: the door[19] doorway crossing (the corner
//! pincer — see the report's live checks). The level's summary:
//!
//! the elevator start #0 (22.49, 0.19, 19.16 [D]) → the spawn TouchOnce #679 fires ch 0x48 → the
//! REAL walk south around the shaft core → the REAL Q-Decryptor G-cycle + beam burn of the keypad
//! locks #656/#694 ("Lock - KeypadLocked", out 0xa [D]) → ch 0xa's rising edge OPENS door[19]
//! ([M Door_Update's lock path; the 0x1336f8 two-latch snapshot — the fix this task landed]) →
//! the doorway stand-in → the REAL walk to the security-centre console → E → ch 0x49 (objective
//! 1) + ch 0x59 (the SS lift doors #677/#678 slide to key 1 [M SS_Update 0x1a9948]) → the REAL
//! walk into the shaft entrance → #683 holds ch 0x1F → the gated exit #669 auto-fires ([M
//! Trigger_Update 0x1b1a78 case 10]) → level_exit 0700004A. Objective 2 (ch 0x4D) has no writer
//! here — its writer is TouchOnce #277 on 0700004A (the late AND [D]).

#[path = "common/walker.rs"]
mod walker;
#[path = "common/tower2.rs"]
mod tower2;

use walker::Walker;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

#[test]
fn walk_spawn_to_the_shaft_exit() {
    let iso = iso();
    let mut w = Walker::new(nf_view::load_with_carry(&iso, "07000012.bin", false, None).unwrap());
    {
        let sim = w.scene.sim.as_ref().unwrap();
        let p = sim.bond.pos;
        assert!(p[0] > 21.0 && p[0] < 24.0 && p[1] > 0.0 && p[1] < 2.0 && p[2] > 18.0 && p[2] < 20.0,
            "the elevator start #0 [D], got {p:?}");
    }
    tower2::walk_12(&mut w);
    tower2::walk_12_exit(&mut w);
}
