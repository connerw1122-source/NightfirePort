## Your task: `fix-gadgets2`, the gadgets as the user played them

fix-gadgets (merged, `docs/reports/fix-gadgets.md`) made G select gadgets and ported the laser burn, the decryptor
burn and the Q-Claw headless. The user played them (2026-10-12, checks `gadgets` and `camera0c2`):

1. **The hold-to-use input is backwards.** "Both the stun and the laser do not work properly. If I click in, they
   start. But then when I let go, they actually release and actually do what they're supposed to do briefly."
   Port the game's input handling for held gadgets (the fire button held → the beam/effect runs every tick while
   held; release stops it) from `Player_LaserPointer` 0x1a65d8, `Player_WeaponFire`/the gadget fire paths and the
   pad state they read. Test through the real input path: press, hold N ticks (the effect runs each tick), release
   (it stops the same tick).
2. **The sounds keep playing after you stop.** Stop the gadget's loop sound when the game does.
3. **The grapple "doesn't work at all"** in play (screenshot `07000005 --bond -18.28,38.19,-71.83,-4.717,0.151`:
   the gadget is selected, labelled "Grapple", aimed at the point on the wall). The headless test drove it from a
   fixed setup; reproduce the user's case from that spot and fix what stops it (the aim test against the grapple
   sphere, the range, the fire input).
4. **The micro-camera zooms through a sniper scope** (`0700000c --bond 5.61,12.49,-154.44,6.286,-0.059`): it needs
   the camera's own overlay. Find the camera's HUD/overlay in the game (the zoom HUD selection by weapon) and port it.
   **The real overlay (main agent, PCSX2 screenshot 2026-10-12, 0c roof, zoomed):** a rounded-rectangle viewfinder
   with black outside it; left and right a horizontal level bar ending in a curved hook, with a column of 5 short
   tick marks above and 5 below each; a centre circle half filled (a level indicator); small green corner brackets
   in the top-left corner; the "Micro-Camera" label bottom right. No scope reticle or cross-hair lines.
5. **"I have the Q-worm on a level that doesn't need Q-worm"** (0700000c). Check where the port's start loadout
   for 0c comes from versus the game's (`Player_RamSave` carry versus the level's own loadout), and whether `--dev`
   adds it. The loadout must match the game's.
6. **Red and yellow boxes** the user saw "that look to be transitions for the wall shimmy … and even one for the
   grapple. You should get rid of those." Likely placement meshes the game never draws (creep walls 0x3F, grapple
   points 0x3A, or touch volumes) that models-misc's `misc_static_sweep` started drawing. For each kind the sweep
   draws, prove from the decompile that the game draws it (`View_DrawObjList` / the creator's attach), and stop
   drawing the ones it doesn't.

Files: `crates/nf-game/src/gadget.rs`, `player.rs` (the fire input), `crates/nf-view/src/{scene,hud}.rs`.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** The user's play is the reference, and previous fixes passed tests but still failed in play. For each bug:
1. Write a headless NF_ISO test that drives the real input path (the `nf-view` input mapping into `Sim`, or `nf_view::load` plus `dev::scenario`), and show it reproducing the user's symptom.
2. Then find the cause in the decompile and fix it.
If you can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Update the doc entry** to say it's fixed, with your commit.

**Branch:** `s/fix-gadgets2`.

---
claimed-by: sandbox6 at 2026-10-13T04:18Z
