## Your task: `weapon-fire-modes`, every weapon's fire modes: the guided missile camera, impact grenades, the rest

The user (2026-10-13): "We also need heavy emphasis on weapon fire modes. The guided launcher has a literal camera that
the rocket turns into: you're controlling the rocket with camera movement, you ARE the rocket. The MGL grenade launcher
is always delayed when I believe it should be impact for one of the modes. Things like that."

What's known (docs/reports/fix-weapon-modes.md, fix-weapon-modes2.md): the mode button is the pad's circle (action 12,
the port's B key), read in `Player_Weapon` 0x1a24c8:204-260. A record with `+0x05 == 0` cycles its MODE BYTE
(`BL + w*0xC + 0x1BB`) up to the record's mode count (`+0x2E`); only the grenade records 0x60/0x61 have count 2 [L].
A record with `+0x05 != 0` swaps to its TWIN (P2K silencer 0x02/0x03, Scorpion heat-seeker/unguided 0x2E/0x2F, the
'Guided'/'Unguided' launcher 0x2C/0x2D (the report calls it "Torpedo Launcher": check the name), the SG5 0x16/0x17,
the watch 0x54/0x55). Homing (`Bullet_homing` 0x12f358 for `+0x6C` & 0x20000) is ported.

What is NOT ported, and is this task:
1. EVERY weapon's modes, as a table first: for each record 0x01..0x71, the mode count, each mode's label (the `+0x30`
   lane), what the mode changes (fuse, fire rate, burst, zoom, projectile class, `+0x6C` flags), and the twin. Put it in
   `docs/systems/weapons.md` (new) with the decompile line for each fact. Check it against the multiplayer weapon set too
   (multiplayer weapons that never appear in single player still need their modes).
2. The GUIDED (remote-controlled) missile: the launcher whose rocket becomes the camera. Find the game's code: the
   projectile class / `+0x6C` flag that hands the view to the missile, the camera it uses (`Camera_*`, the viewer swap,
   any screen overlay or noise effect), the steering from the look input (rates, limits), the speed, how it ends (impact,
   timeout, the fire button detonating it?), and how the view returns to Bond. Port it: firing in that mode puts the
   camera on the rocket, the mouse/look steers it, the HUD shows the game's overlay, the explosion returns the view.
3. The MGL grenade launcher's modes: impact vs timed (the user: it's always delayed now). Find which mode byte value
   means impact detonation, where the fuse is chosen (the grenade projectile's create path reads the mode byte), and
   port both. Also every other grenade/mine/launcher mode (remote mines: the detonate press; proximity; sticky?).
4. The rest that the table turns up: burst vs auto vs semi fire rates, scope/zoom-only modes, alt-fire.
Tests per mode through the real input path (fire with the mode set by the real B press): the projectile class, fuse
behaviour (an impact-mode grenade explodes on its first contact, a timed one after the fuse), the guided missile follows
the look input and the camera rides it then returns; numbers only; sister assertions. List `--give` + key steps for the
user to try each mode (`--give <id>` then B to switch mode).

PCSX2 ground truth is on master: `docs/live/2026-10-13-weapon-modes-pcsx2.md` (read it first):
- the AT-420 Sentinel 0x2C ("Guided") puts the view ON the rocket: a red-tinted camera, a red ring + bracket overlay,
  scanlines, the normal HUD's health bar and crosshair hidden; the look input steers; on impact one or two frames of red
  static, then back to Bond;
- the Militek MGL is a TWIN pair: 0x2A "Grenade" = IMPACT detonation, 0x2B "Delay" = a fuse of about 3-4 s (take the
  exact value from the decompile); the twin switch needs the twin to have rounds;
- with the default "Classic Bond" layout the mode switch is SQUARE ("Alt. Fire"), not circle: find the action and keep
  the port's B key on it.
- the same doc now has EVERY weapon's two modes as the real game labels and switches them (a table of 32 weapons:
  Semi/Silenced, Semi/Burst (3 rounds), Auto/Semi, Burst/Grenade (the AIMS-20 underbarrel launcher), Pump/Auto,
  Single/Armor Piercing, Heatseeker/Unguided, Overcharge/Beam (the Phoenix Samurai's charge-and-release vs drain beam),
  the remote mine's detonator, the satchel's countdown, the tripbomb's laser, the flare gun, the Q-Pen dart, ...). Port
  every row and test each against it.

Files: `crates/nf-game/src/{gun,grenade,bullet,explode}.rs`, `crates/nf-view/src/{lib,scene,hud}.rs` (the missile camera
and overlay), a new `docs/systems/weapons.md`, tests.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md). Read `tail -1 suite.out` and
  its FAILED/TIMEOUT lines before you push, and run it once more on your final tree (merged with `origin/master`). Report the total
  line. On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/weapon-fire-modes`.

---
claimed-by: sandbox3 at 2026-10-14T03:33Z

---
## Report (done, 2026-10-14T04:57Z)


---
## Report from the worker (done, 2026-10-14T05:33Z)

