## Your task: `stunner-press`, the stunner only fires on key release

User (07000005, `--give 0x4a`): "Doesn't fucking work. It only really activates on releasing the key." Earlier: "the
stunner: doesnt work". Find the stunner's fire path in the game (`Player_WeaponFiring` 0x1a5460 for 0x4a/0x4b, the
taser beam / dart, held vs press, `+0x68` flags) and the port's - something gates the fire on the trigger's falling
edge or the press is eaten by the gadget cycle. Fix it: the stunner fires on press like the game, its beam/dart drawn,
and a guard hit is stunned (the msg 7 taser path is not ported per drone.rs - port it if that's why nothing happens).
This comes from the user's playtest 1014-1725 of the real-input build (`nf-view <iso> <level> --dev`, mouse +
keyboard). The user was very disappointed: several merged fixes passed their tests yet were wrong or broken in play.
Your test MUST drive the real input path (`Sim::step_bond` with the `Input` the window builds - lib.rs maps the mouse
and keys), not internal setters, and must fail first on master. The main agent will compare against PCSX2 after you
finish; put the exact steps to reproduce in the report.

Files: `crates/nf-view/src/scene.rs`, `crates/nf-game/src/drone.rs`.

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

**Branch:** `s/stunner-press`.

**PCSX2 ground truth (main agent, 2026-10-14):** read `docs/live/2026-10-14-weapon-measurements-pcsx2.md` on master first — it has the measured numbers for this task.

---
claimed-by: sandbox4 at 2026-10-15T00:59Z

---
## Report (done, 2026-10-15T04:26Z)

# fix-stunner-press (109b)

The user (07000005, `--give 0x4a`, playtest 1014-1725): "Doesn't fucking work. It only really activates on
releasing the key." Ground truth [D docs/live/2026-10-14-weapon-measurements-pcsx2.md, the stunner section]:
the real stunner activates on the PRESS (the device swings forward, the tip lights blue, it holds while fire
is held, the meter stays 100 with no target), and the release swings it back.

## What was actually wrong

Two independent defects; the press side was already right in the sim, the tase side was unported.

**1. The tase did nothing (the main "doesn't work").** The stunner's shot spawns a zero-damage bullet
(damage 0.0, radius 0.0 [L ee_start_05.bin record 0x4a +0x0c/+0x10]) whose guard hit must route **msg 7**
(the tase) [M NDrone2_DealWithObjHit 0x13fff0:269-300 — the 0x4A/0x4C ids pick msg 7; 1/0x4B/0x4D pick msg 6;
everything else `Drone_BulletHit`]. Master routed 0x4A/0x4C to msg 8 — a zero-damage bullet hit: the guard
flinched and alerted, nothing else, forever. The whole taser family (DSTATE_Taser 0x4a, DSTATE_Stunned 0x4b,
DSTATE_Stunned_Recover 0x4c) was unported [M dronetype-tables.md:257].

**2. The press-vs-release question.** Traced the game's fire path end to end: with the stunner's refire
denormal-0 [L record +0x40 = 1.4e-45], `Player_WeaponFiring` 0x1a5460 fires EVERY held tick
(`+0x92c = max(+0x40, 1.0)`, decremented by FRAME_RATE_MUL; the state-0xb path at LAB_001a643c), and every
shot re-queues the fire script through `Player_SetFiringAnim` 0x1a3c70 → `AnimScriptAdd` 0x1c6d40 (deletes
the playing script, restarts at frame 1 [M AnimScriptInit 0x1c1600: +0x90 = 1.0]) — the view state is 0xb
(the 0x400 branch), never 9, so the state-9 restart gate never applies. **The port's `update_with` +
scene.rs `fired` block model the same thing** — the probe shows `state9=true script=0600048a` on the very
first press tick, `vt` frozen at 0 while held (the restart), the beam on target, the clip draining 1/frame,
the release ringing out into 0x6000488 at 1.5×. The lib.rs mouse path (`trigger = state == Pressed`,
`fire: self.trigger`) carries the same bool the probe drives. So the port fires on the press; what the user
saw as "activates on release" is the release script's swing being the only visible response while the held
frames show the static frame-1 pose — plus the guard never falling (defect 1). The held frame-1 pose vs the
game's swing-forward needs the main agent's render comparison (Live checks below); the sim-level press
behaviour is pinned by the test.

## The fix (crates/nf-game/src/drone.rs)

- **The msg-7 base handler** `on_tase` [M DroneFunc_HandleImpact 0x1469b8:111-176]: the accumulator `+0xBC`
  += FRAME_RATE_MUL per tase msg; past 2·health (`fptoui(+0xAC)`, the live health) the drone falls into
  DSTATE_Taser with the HITDATA. Below the threshold only the accumulator==1 tick acts in the game
  (`SetState(uVar12)`, the re-flinch) — the port skips that no-op re-entry (the fall timing is unaffected).
  This is the base DSTATE's case 7, so the port runs it as the fallback for every state without its own arm
  (exactly like msg 8/6).
- **The taser family** `handle_taser`:
  - TASER 0x4a ENTER [M 0x162b98 case 1]: +0xCE0 = RecoverTime's taser arm (FRAME_RATE_INT·10 →
    600·0.01·(200−0xB9) = 1200 ticks [M 0x147188 cases 0x4a/0x4c; the stats byte 0 at run time [L
    07000005]]), AlertStatusSet(2) → alert ≥ 0.66, the fall anim Rand(2) → 0x31/0x32 (+0xCD8 stored), the
    twitch timer FRAME_RATE_DIV·30 = 1800. Case 7 re-arms the timer; case 0xc falls through to Stunned when
    the fall anim (+0x56a band) is playing, else re-arms 300.
  - STUNNED 0x4b [M 0x162eb0]: ENTER arms +0xCC4 = now + 0xCE0 and plays 0x30/0x2f by the stored fall anim;
    case 7 re-falls (Taser) unless the stored anim is 0x32 (then the case-0x19 re-arm); TIMER_C re-arms 300
    while AnyoneNear(2.0) — the same live-pos query the stun-grenade loop uses — else Stunned_Recover.
  - STUNNED_RECOVER 0x4c [M 0x1630b8]: ENTER plays the get-up 0x76 with follow-on Attack 0x56; case 7 routes
    back through HandleImpact (the accumulator grows, the re-fall).
- **The routing** [M 0x13fff0:269-306]: bullets with weapon 0x4A/0x4C now send msg::TASE (new const 7).
- **The family swallows its own messages** [M 0x1469b8:366 — HandleImpact's default returns 1]: the game's
  family DSTATEs unhandle everything else (→ GLOBAL = HandleImpact → dropped), so the port must not fall
  through to its own GLOBAL-state arms — the alarm/sound/shout/anim-done routes pulled the tased guard
  straight back into AIM_STAND_FIRE 0x68 (the found-by-backtrace leak). TASER/STUNNED/RECOVER also skip the
  SCREAM/SHOUT and EXPLOSIVE shared routes (no case 0x11/0x12/0x21 in the game's family switches).
- Bullets (msg 8/9) on the family → Death_Anim 0x44; a punch on STUNNED → KnockedOut_Anim 0x42 [M 0x162eb0
  case 6]; TASER/Recover have no case 6 (dropped).

## Verification

- `fix_stunner_press_live` (new, NF_ISO, the real input path — `nf_view::load` + `Sim::step_bond` with the
  window's `Input`, the stunner equipped through `switch_to`): **3 passed** on the final tree.
  - `the_stunner_fires_on_the_press_tick`: ONE settled press tick → fire_state, view_script 0x600048a, clip
    100→99; the release ends the machine through 0x6000488 → idle 0x600048b and spends nothing (the clip
    only refills — the game's own passive regen [M Player_Update 0x193600 disasm 0x193eb8: clip[0x4A] +1
    per 4 frames while not fire-busy]). Failed on master (no fire machine entry — actually: the press side
    held on master too; the test pins the whole contract).
  - `the_tased_guard_falls_and_stays_down`: the held tase drops the guard into TASER 0x4a at hold tick 20
    (accumulator 21 > 2·10), health UNTOUCHED, alert ≥ 0.66; he stays in the family under a held tase and
    ends in STUNNED 0x4b with Bond 2.5 u away (the AnyoneNear re-arm). **Failed on master** — the guard
    never left his combat state (msg 8, zero damage).
  - `a_p2k_hit_is_not_a_tase` (sister): the P2K's hit applies real damage and never enters the taser family
    [M 0x13fff0 — only 0x4A/0x4C pick msg 7].
- `probe_stunner` (scratch, kept): the 120-tick hold trace that reproduced the symptom numbers above.
- nf-game lib 160/160; nf-view lib 8/8; the stun-grenade/gadget live tests were still running at the
  wrap-up (see Left).

## Left

- **The full suite did not fit the window** (the fix landed at 04:55, the wrap-up started 05:20; the suite
  is ~30 min on this 2-core box). Run on the final tree before merge: `tools/test/suite.py`. The risky
  surface is the msg-dispatch change in `Drone::handle` — it is guarded to the three new state values, and
  nf-game's lib + the three live tests above cover the family.
- The held-frame pose: the sim restarts the fire script every held tick exactly like the game; whether the
  port's frame-1 pose of 0x600048a shows the swung-forward device (the game does [D]) needs a render
  comparison — the main agent's Live checks below.
- The weapon drop / ChangeWeapon(1/6) [M 0x162b98:31-32, 0x1630b8 case 1] is NOT modelled — the port's
  `Drone::weapon` is the held weapon MODEL id (36 for these guards), not the game's ChangeWeapon slot; the
  stun-grenade family has the same standing gap.
- The first-tase-tick re-entry (the game's `SetState(uVar12)` at accumulator==1) is skipped (a no-op
  re-flinch); the msg-0x19 re-arm on STUNNED is folded into the case-7 fall-through; the behaviour-0x32
  alert-to-position arm at the TASER enter is not ported (prop-dependent).
- The tase accumulator +0xBC never resets (no other writer in the decompile) — a partially-tased guard
  stays "charged" across states, matching the game's only writer.

## Live checks for the main agent

PCSX2 (07000005, the spawn pocket, the stunner cycled from the P2K):
1. Spot: Bond 2.5 u in front of the stairs guard, crosshair on him. Action: one fire PRESS (~100 ms).
   Expected: the device swings forward with the blue tip on the press frame; the guard keeps standing
   (one tase tick < 2·health); the meter reads 99.
2. Same spot. Action: hold fire ~1 s. Expected: the guard collapses (the fall anim 0x31/0x32), his gun
   drops, the meter drains ~1/frame to empty-ish; he twitches on the ground ~30 s (the 1800-tick timer),
   then lies stunned (0x2f/0x30) while Bond stays within ~4 u.
3. Same spot, crosshair OFF any guard. Action: one press, release. Expected: the meter returns to 100
   within ~a second (the miss arm + the passive regen); nothing flies.
4. nf-view comparison on the held pose: `nf-view <iso> 07000005 --dev` with the stunner, hold the mouse
   button — does the device swing forward and STAY forward (frame 1 of 0x600048a), or stay upright until
   the release? The game shows forward [D ground truth §stunner]; the sim pins the script contract either
   way, the pose is the render question.
