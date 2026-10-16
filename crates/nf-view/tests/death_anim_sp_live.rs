//! death-animation-sp: Bond's death plays the death script, holds the death camera and waits the
//! game's time before the failed page (playtest 1015-0046, restart after dying: "missing death
//! animation").
//!
//! Evidence base:
//! - The death starts [M `Player_CheckForDeath` 0x190810:34-97]: ch 0x5e, `Music_Event(6,1)`,
//!   `Player_ClearInertia`, `Player_WeaponNone` (the fists 0x47), the death marker `obj+0xff=0x12`,
//!   the dead-tick stamp `obj+0xec`, **cam mode 0** (`Player_SetCamMode(BL, 0)` [M :64]), health 0,
//!   `obj+0xf4 = 2`, the death sound 0x89, then `Player_ChangeSubState(obj, 0xd)` — 0xe when ch
//!   0x60's bit 0x100 marks a vehicle [M :89-94].
//! - Case 0xd/0xe [M `Player_ChangeSubState` 0x190630:64-73]: `AnimSetDeleteAll` +
//!   `AnimScriptAdd(obj, 0x6000151)` — or 0x60008e0 only when the gravity global is exactly 0.0
//!   (the zero-g level; retail keeps −9.4, so 0x6000151 is the ground death — fix-world-gravity).
//! - The dead branch [M `Player_Update` 0x193600:316-319, `obj+0xf4` 2/3] runs ONLY
//!   `AnimObjectUpdate` + `Player_HandleDeath` — no substate arm, no `Player_MonitorAir`, no
//!   collision pass: Bond's eye freezes where he died. The anim system writes no object position
//!   (`AnimObjectUpdate` 0x1befd8, `AnimSeqTick` 0x1c28d0, `AnimFrameResolve` 0x1c35f8 hold no
//!   obj+0x30 store), so the death camera does NOT follow the collapsing skeleton either.
//! - The camera [M `Player_PositionCamera` 0x18f5d8 case 0:96-134 → `Camera_SetToPlayer` 0x1bc048
//!   → `Player_GetHeadPos` 0x195aa0:39-51: eye = obj+0x30/34/38 + BL+0x908 − BL+0x90c; the dead
//!   arm zeroes BL+0x90c [M 0x18f5d8:130-132], BL+0x908 lerps to 0 on land [M 0x18f5d8:113-128]]:
//!   a frozen first-person eye at the death spot. The visible "death animation" is the view model
//!   posed from the body's collapsing bones — the drawing side (the main agent's render task).
//! - The script's clock [M `AnimScriptInit` 0x1c1600 sets the script time (+0x90) to 1.0 on
//!   `AnimScriptAdd`; [L] the PCSX2 death trace: +2.0 frames per dead tick] — the port's
//!   `DEATH_SCRIPT_FRAMES_PER_TICK` 2.0 at the fixed 60 Hz; end mode 0 stops at the length [M
//!   `AnimScriptEnd` 0x1c1930].
//! - The script's data on 05 [L the parsed 07000005 archive]: length 297, one opcode-0 window
//!   (1, 297, seq 0x04000123) and op-4 effect commands at frames 5 (kind 5), 8 (kind 4), 152
//!   (kind 4), 278 (kind 6, the body). `AnimProcessScriptCmds` 0x1c1b38 case 0 starts the
//!   sequence (the pose the renderer draws), case 4 fires the footstep/body effects — the port's
//!   `DeathEvent`s.
//! - The wait [M `Player_HandleDeath` 0x190bb0:29-33 + `HUD_UpdateBloodPane` 0x19a6a8]: ch 0x62
//!   waits for the blood pane's full cover — 448 ticks ([L] viewer.h 448, 512-wide templates) —
//!   or the ✕ (action 25); `Mission_Update` 0x182c60 then walks 1 → 3 → 7 → 4 and opens the
//!   0x40000042 fail page. `fix_death_restart_live` pins that flow end to end; here it pins that
//!   the death script's frames run out ahead of it.
//!
//! Opt-in via `NF_ISO`.

use nf_game::anim::{script_frame, script_pose};

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn kill(sim: &mut nf_view::scene::Sim) {
    let (h, r) = (&mut sim.drones.health, &mut sim.drones.rng);
    h.pain_generic(r, 500.0, 1, 0x0700_0005, 0);
}

/// The death script plays the game's frames and the failed page opens after the red fade: kill
/// Bond on 05, hold nothing — the script clock runs 1.0 → 3.0 → … at 2.0/tick, fires the op-4
/// effects at the game's frames, stops at the script's length 297, the sequence window is the
/// renderer's pose input, and the fail page opens on the fade's 448-tick beat.
#[test]
fn death_script_runs_the_games_frames_then_the_fail_page() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 05");
    let sim = scene.sim.as_mut().unwrap();
    sim.disabled = false;
    for _ in 0..10 { sim.step_bond(Default::default(), false, false, false); }

    // The script's shape straight from the archive [L]: one seq window + the four op-4 effects.
    // (Owned copies — the sim keeps ticking below.)
    let (len, wins, ops) = {
        let script = sim.scripts.get(&0x0600_0151).expect("the death script ships in 05's archive");
        (script.length, script.sequences().collect::<Vec<_>>(),
         script.commands.iter().filter(|c| c.op == 4).map(|c| (c.args[0], c.args[1])).collect::<Vec<_>>())
    };
    assert_eq!(len, 297, "the death script's length [L 07000005]");
    assert_eq!(wins, vec![(1, 297, 0x0400_00A3)], "the death pose's sequence window [L]");
    assert_eq!(ops, vec![(5, 5), (8, 4), (152, 4), (278, 6)], "the op-4 frames/kinds [L; kinds 5/4/4 footsteps, 6 the body — M 0x1c1b38 case 4]");

    // Kill: the death starts on the tick the health check sees 0.
    kill(sim);
    sim.step_bond(Default::default(), false, false, false);
    assert!(sim.drones.death.active, "the death sequence started [M 0x190810]");
    assert_eq!(sim.drones.death.script_time, 1.0, "AnimScriptInit's 1.0 [M 0x1c1600]");
    assert_eq!(sim.bond.cam_mode, 0, "cam mode 0 [M 0x190810:64]");
    assert!(!sim.drones.channels.on(0x62), "the red fade holds ch 0x62 [M 0x190bb0:29-33]");

    // +1 dead tick → the clock crossed frame 3; the frame-5 footstep has NOT fired yet.
    sim.step_bond(Default::default(), false, false, false);
    assert_eq!(sim.drones.death.script_time, 3.0, "+2.0 frames per dead tick [L the death trace]");
    assert!(sim.drones.death.events.is_empty(), "0x06000151's first command sits at frame 5");
    // +1 more → the window (1, 5] fires the kind-5 footstep (the probe's event).
    sim.step_bond(Default::default(), false, false, false);
    assert_eq!(sim.drones.death.events.first().map(|e| (e.frame, e.kind)), Some((5, 5)), "the first op-4 at frame 5 [L]");

    // The script runs out at 297 (tick 149 of the death), long before the fade's 448.
    for _ in 0..200 { sim.step_bond(Default::default(), false, false, false); }
    assert_eq!(sim.drones.death.script_time, 297.0, "the clock stops at the script's length [M AnimScriptEnd 0x1c1930]");
    assert_eq!(sim.drones.death.events.len(), 4, "every op-4 fired once [M 0x1c1b38's fired bits]");
    let last = sim.drones.death.events.last().unwrap();
    assert_eq!((last.frame, last.kind), (278, 6), "the body effect closed the script [L]");
    assert!(!sim.drones.death.covered, "the fade is still running at the script's end");
    assert!(!sim.drones.channels.on(0x62), "the game's wait outlasts the script [M 0x190bb0:29-33]");

    // The failed page on the fade's beat: cover at 448, ch 0x62, then Mission_Update's walk.
    for _ in 0..(nf_game::death::RED_FADE_TICKS + 4) { sim.step_bond(Default::default(), false, false, false); }
    assert!(sim.drones.death.ch62 && sim.drones.channels.on(0x62), "the cover raised ch 0x62 [M 0x190bb0:30]");
    assert!(sim.drones.death.covered, "the fade covered [M 0x19a6a8:24]");
    assert!(sim.drones.level_exit.is_some() || !sim.drones.mission.menu_choice.is_none() || sim.drones.fail_menu_open(),
        "the 0x40000042 fail page path ran [M 0x182c60:77-81]");

    // The renderer's pose input: the death seq resolves a pose (the drawing side can play it).
    let drawable = sim.skins.values().any(|s| {
        sim.skeletons.get(&s.skeleton)
            .is_some_and(|sk| script_pose(&sim.scripts, &sim.seqs, sk, s, 0x0600_0151, 150.0).is_some())
    });
    assert!(drawable, "some skin/skeleton poses seq 0x04000123 — the death anim is drawable");
    let (q, f) = {
        let script = sim.scripts.get(&0x0600_0151).expect("the death script");
        script_frame(script, 150.0).expect("the death script frames")
    };
    assert_eq!(q, 0x0400_00A3, "mid-script the pose is the death seq");
    assert!((1.0..=297.0).contains(&f), "the frame tracks the clock");
}

/// The death camera holds the death spot: the game's dead branch runs no movement [M 0x193600:
/// 316-319] and the camera is the frozen eye (`Player_GetHeadPos` 0x195aa0 reads obj+0x30 —
/// nothing in the dead branch writes it). Grounded AND airborne — the pre-fix port kept the walk
/// physics ticking and an airborne death fell (the probe: −3.06 units over ~55 ticks), which is
/// the "missing death animation" flow gap: the game's eye is bit-frozen from the first dead tick.
#[test]
fn dead_bond_holds_the_death_camera_spot() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };

    // Grounded: the eye never moves.
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 05");
    let sim = scene.sim.as_mut().unwrap();
    sim.disabled = false;
    for _ in 0..10 { sim.step_bond(Default::default(), false, false, false); }
    kill(sim);
    sim.step_bond(Default::default(), false, false, false);
    assert!(sim.drones.death.active, "dying");
    let eye = sim.bond.head();
    for _ in 0..60 { sim.step_bond(Default::default(), false, false, false); }
    assert_eq!(sim.bond.head(), eye, "the death camera holds the death spot [M 0x1bc048 → 0x195aa0]");

    // Airborne: the fall stops at the death — the dead branch integrates nothing.
    let mut scene = nf_view::load(&iso, "07000005.bin", false).expect("load 05 #2");
    let sim = scene.sim.as_mut().unwrap();
    sim.disabled = false;
    for _ in 0..10 { sim.step_bond(Default::default(), false, false, false); }
    sim.bond.pos[1] += 3.0;
    sim.bond.grounded = false;
    sim.bond.floor_ny = 0.0;
    kill(sim);
    // The lethal tick's own movement may run once (the death starts inside drones.tick, after the
    // movement section) — from the first DEAD tick on, the eye is bit-frozen.
    sim.step_bond(Default::default(), false, false, false);
    assert!(sim.drones.death.active, "dying mid-air");
    let eye = sim.bond.head();
    for _ in 0..60 { sim.step_bond(Default::default(), false, false, false); }
    assert_eq!(sim.bond.head(), eye, "a dead Bond in the air freezes there [M 0x193600:316-319] — the pre-fix port fell 3.06 units");
    // The death camera: mode 0 stands (the game's Player_SetCamMode(BL, 0) [M 0x190810:64]).
    assert_eq!(sim.bond.cam_mode, 0, "cam mode 0 while dead [M 0x190810:64]");
}
