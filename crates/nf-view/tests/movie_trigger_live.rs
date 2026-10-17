//! movie-trigger-41: the movie-player trigger (placement 0xF4 → action 0xd) and the NIS events
//! 0x08/0x0a of script 0600005b, through the real paths.
//!
//! Evidence base:
//! - `Trigger_MoviePlayer` 0x1b2a08 (disasm): backs the param block up to TmpParam 0x319b88, writes
//!   slot 0 = 0xd (@0x1b2a58), slot 1 = old p0 (@0x1b2a6c), slot 2 = old p2 (@0x1b2a74), slot 9 =
//!   old p1 (@0x1b2a7c), then tail-calls `Trigger_Create` 0x1b1720 (out = slot 1 as u32 into
//!   ext+0x20 :68, inputs = slots 2..8 :69-77, mask = slot 9 :81-84, lock = slot 10 :87, hide =
//!   slot 11 :88).
//! - `Trigger_Update` case 0xd 0x1b1a78:190-201 (disasm 0x1b1f24-0x1b1f78): fires only when the
//!   FIRST input slot (ext+0x26) is non-zero AND its channel is on; clears the channel, latches
//!   ext+0x20 into `GameState+0x10` and 0 into `GameState+0x14`, pushes GameFlow states 6 (f12 =
//!   0.0) and 7 (→ `Camera_SetFade(6, 30.0, 1)` + state 8, `GameFlow_PushState` 0x1c9f38:16-22).
//!   NOTHING starts a movie or loads a level on this path: state 6 pops the same tick when
//!   `BGFMVPlaying == 0` (`psiMovieFinished` 0x1082e8; `GameFlow_Main` 0x1c98c8 case 6) and only
//!   `ResetMap_Load` phase 2 (0x1d6360:173) ever reads `GameState+0x10` — phase 0 overwrites it
//!   first (0x1d6360:103).
//! - `Trigger_Activate` 0x1b2318:53-61 (the E-key path): the `mask & 4` gate, then the same
//!   GameState+0x10/+0x14 latch and push 6 + push 7 (0x43960000 = 300.0f). No channel writes, no
//!   level load, no spent latch.
//! - The disc's only 0xF4 record: 07000041.bin main-map placement #81, params (0, 4, 0) [D —
//!   measured placement bytes], volume a 1×1×1 box at (15.64..16.64, 0..1.0, −0.91..0.09).
//! - NIS event 8: `Script_EventHandler` 0x1d94c0 case 8 (disasm 0x1d96d4-0x1d96f4) — the previous
//!   stream record's object (kind must be 1/2, else NULL [M 0x1d94c0:75-96]) is the position
//!   source; NULL skips the call (`beq s5,zero` @0x1d96d4); the 7th argument is hard-coded 4
//!   (`addiu t2,zero,4` @0x1d96ec → `DCVars+0x5A2` @0x13a1a0). `Drone_CoderCreate` 0x13a108 maps
//!   p0 → tag slot 0 (skin), p1 → slot 1 (start script), p3 → slot 9 (setup); param 2 is dead.
//! - NIS event 0xa: case 0xa (disasm 0x1d96bc-0x1d96c8) — `Break_DoBreak(prev_obj+0xC0)`, the
//!   same NULL skip; `Break_DoBreak` 0x12c168 scans the control list for the first class-0x20
//!   object whose world sphere (obj+0x80/+0x8C) contains the position, `Break_Kill` 0x12b970
//!   deletes it (obj+0xFE |= 1) and plays 0x2e (glass) / 0x5f2-0x5f3 (wood) at the object.
//! Opt-in via NF_ISO.

use nf_game::player::{Bond, Input};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The disc's single 0xF4 record parses with the wrapper's mapping and CANNOT fire from
/// `Trigger_Update`: its input slot (p2) is 0, so case 0xd's first guard fails every frame even
/// with channel 0 armed [M 0x1b1f24-0x1b1f28; D placement #81 params]. Sister: no amount of
/// walking arms it either — the update path needs a script/trigger writing p2's channel, which
/// nothing in the level does [D the level's channel writers: the 0xEA plates write 0x6e/0x01].
#[test]
fn the_discs_movie_record_parses_and_never_fires_from_update() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000041.bin", false).expect("load 07000041");
    let sim = scene.sim.as_mut().unwrap();

    let m = sim.drones.triggers.iter().enumerate()
        .filter(|(_, t)| t.kind == nf_game::triggers::kind::MOVIE)
        .map(|(i, t)| (i, t.out, t.mask, t.list.clone(), t.movie_code, t.gone))
        .collect::<Vec<_>>();
    assert_eq!(m.len(), 1, "exactly one movie trigger — the disc's single 0xF4 record [D census]");
    let &(i, out, mask, ref list, code, gone) = &m[0];
    let t = &sim.drones.triggers[i];
    assert_eq!(out, 0, "out slot = p0 = 0 [D placement bytes; M 0x1b2a6c slot 1 = old p0]");
    assert_eq!(mask, 4, "mask slot = p1 = 4 [D placement bytes; M 0x1b2a7c slot 9 = old p1]");
    assert!(list.iter().all(|&c| c == 0), "input slots = raw p2..p8, all 0 [D placement bytes; M 0x1b2a74]");
    assert_eq!(code, 0, "the movie code (ext+0x20's u32) = 0 [D; M 0x1b1720:68]");
    assert!(!gone);
    assert_eq!(t.tris.len(), 6, "the placement's own collision builds the volume [D object 66's mesh]");
    // The volume box [D measured]: (15.64..16.64, 0..1.0, −0.91..0.09) — the 1 × 1 × 1 player
    // trigger at (16.13, 0, −0.41).
    for k in 0..3 {
        assert!((t.lo[k] - [15.641, 0.00003, -0.912][k]).abs() < 0.01 && (t.hi[k] - [16.641, 1.0, 0.088][k]).abs() < 0.01,
            "the volume box [D placement #81 collision]");
    }

    // Arm channel 0 (a real channel — `SS_Update` reads and writes it [M 0x1a9948:20-23]) and run
    // the sim: the SLOT guard (list[0] == 0) must keep the trigger silent.
    sim.drones.channels.set(0, 1, 1);
    for _ in 0..300 { sim.step_bond(Input::default(), false, false, false); }
    assert!(sim.drones.channels.on(0), "channel 0 still on — the fire's clear never ran [M 0x1b1f48 only on a fire]");
    assert_eq!(sim.drones.movie_request, None, "case 0xd never fires with an empty input slot [M 0x1b1f24-28]");
    assert_eq!(sim.drones.level_exit, None, "and nothing loads — the movie path has no ResetMap call [M 0x1b1a78 case 0xd]");
    assert!(!sim.drones.triggers[i].gone, "no spent latch in the update arm [M]");
    // Sister: the level's own script chain runs its plates (ch 0x6e via #100 when Bond touches it)
    // and still nothing movie-shaped happens — the movie trigger only listens on its input slot.
    assert!(!sim.drones.triggers[i].movie_fired, "no movie_fired latch ever raised [M 0x1b1f24-28]");
}

/// The E-key path: standing in the record's volume and pressing the use key runs
/// `Trigger_Activate`'s type-0xd arm — `mask & 4` passes (mask 4 [D]), so the game latches
/// `GameState+0x10 = 0` and pushes states 6/8 [M 0x1b2318:53-61]; the port carries `movie_request
/// = Some(0)`. No channel write, no level load, and NO spent latch — a second press fires again
/// [M: the arm has no ext+0x38 = 99 write]. Sister: away from the volume the use key does nothing.
#[test]
fn the_e_key_path_raises_the_movie_request_and_refires() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000041.bin", false).expect("load 07000041");
    let sim = scene.sim.as_mut().unwrap();

    // Sister first: the wrapper's start, E pressed — no trigger within the use sphere.
    for _ in 0..60 { sim.step_bond(Input::default(), false, false, true); }
    assert_eq!(sim.drones.movie_request, None, "the use key away from the volume does nothing [M 0x1a5170:100-108]");

    // Into the record's volume [D (16.13, 0, −0.41)] and press E.
    sim.bond = Bond::stand_at(&sim.world, [16.14, 0.0, -0.41], std::f32::consts::PI);
    for _ in 0..30 {
        sim.step_bond(Input::default(), false, false, true);
        if sim.drones.movie_request.is_some() { break; }
    }
    assert_eq!(sim.drones.movie_request, Some(0), "the E path raises the movie request with the record's code 0 [M 0x1b2318:56-58]");
    assert_eq!(sim.drones.level_exit, None, "no level load rides the movie path [M 0x1b2318:53-61 — no ResetMap call]");
    let i = sim.drones.triggers.iter().position(|t| t.kind == nf_game::triggers::kind::MOVIE).unwrap();
    assert!(!sim.drones.triggers[i].gone, "no spent latch — the game's arm never writes ext+0x38 = 99 [M]");
    // Re-fire: the request consumed, a second press raises it again.
    sim.drones.movie_request = None;
    for _ in 0..30 { sim.step_bond(Input::default(), false, false, true); }
    assert_eq!(sim.drones.movie_request, Some(0), "a second E press fires again (no latch) [M]");
}
