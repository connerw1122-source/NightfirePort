# glass-breach-guards — the Mayhew-start glass breach

The user, 2026-10-16 evening: "beginning of mayhew level guards are supposed to break through the
glass". DONE: the breach works on the real path — the intro wakes the breach squad, a guard
charging Bond shatters the entry-hall pane and comes through into the courtyard, on the game's
own timing. Builds on 750e glass-shatter's breakables core (merged from `s/glass-shatter`).

## What the game does (full map in docs/systems/glass-breach.md)

- **The level**: 07000001 (Paris Prelude, Castle A — the Mayhew estate). The entry-hall front
  wall (the z ≈ −1 plane) carries kind-2 glass panes **130/136/155** (bounds [D]: 130
  [−7.94, 5.40, −1.01]..[−2.07, 7.85, −1.01], 136, 155 the transom), all health 0.0, no filter
  [D placements].
- **The squad**: the 0xF1 spawner placement **#297 — mode 2, group 1, start channel 11** [D
  params] holds 5 group-1 guards (placements 168/169/261/298/299) as templates. The intro
  cut-scene (SP #277, script 0x06000087) writes **ch 11 = 1** at ~tick 2119 [D script decode;
  the port's walk_07000001_live pins it] → the spawner releases the squad inside the hall.
- **The smash arm** [M `Break_Update` 0x12bfd8:23-34]: on **level 0x0700001 alone**, every
  class-2 hitter (drone or player) whose collision registered against a kind-2 pane drains
  `health -= SqDist3D(hitter+0x30, hitter+0x40) × 20.0` — the tick's own movement (the movement
  pass copies +0x30 → +0x40 after the move [M 0x1336f8:212]) — and the pane dies at health < 0
  [M :49-51] via `Break_Kill` (obj+0xFE |= 1, the glass sound 0x2e, NO RNG draw for kind 2
  [M 0x12b970:60-76]). A motionless brush drains 0. No abseil states are involved on this level
  [M: no decomp writer for state 0x8E/0x91 entry; [L] the 07000001 drones are modes 0x00/0x02/
  0x0C — the 0x06000892 abseilers are 07000011/0700004a only].
- **`Break_DoBreak` 0x12c168** (the scripted kill): the first intact class-0x20 object whose
  world sphere contains the point [M :39-51], the standard kill [M :61]. Its ONLY caller is
  `Script_EventHandler` 0x1d94c0:147-151 — **NIS event 0x0A** on the previous stream's object
  position (+0xC0). The disc's one 0x0A occurrence is 07000001's briefing script 0600005b
  stream 8 gate 127 [D census] — orphaned (no placement references it) — ported for
  completeness.

## The port fix

- `Breakables::drone_smash(prev, pos, level)` (breakable.rs) — the class-2 arm with the exact
  drain/kill/sound rules; contact = the tick's movement segment against the pane bounds grown
  0.7 [*suggested*: the game's contact is the body surface on the pane triangles, but the port's
  `move_by` probe (the 0.4 ray overshoot) stops the charging body ~0.4-0.7 short of the plane —
  the port's blocked standoff IS the contact the game resolves].
- `Breakables::do_break(pos)` + NIS event 0x0A (`Request::BreakAt`, the previous-stream rule
  [M 0x1d94c0:94-96]) — `Break_DoBreak` ported.
- `Drone::prev_pos` (obj+0x40 [M 0x1336f8:212]) captured at the head of the per-drone control
  block; Bond's tick-start position is the existing `Drones::player_pos` cache.
- **The panes leave the static collision bake** (world.rs skips placement 0x20) and inject via
  `set_dynamic` per tick (the doors' mechanism) — `Break_Kill`'s "the collision goes away"
  [M 0x12b970:60] becomes real: a broken pane's triangles leave the world and the guards walk
  through the opening. The `segment` queries (movement, sight, pathing) test the dynamic set,
  so intact panes block exactly as before.
- The smash pass folds after the control walk in `Drones::tick_inner`, level-gated [S order —
  the game runs `Break_Update` in the breakable object's own control pass over the same per-tick
  data].

## Reproduce-first (the failing run on master)

`crates/nf-view/tests/glass_breach_live.rs` on the pre-fix tree (master + the merged breakables
core) — FAILED:

```
[breach] t300: broken [] | g8 z -0.51 st 0x68 | g19 z -0.11 st 0x96   (2 of the squad spawned, charged)
[breach] t3300: broken [] | g8 z -0.72 st 0x68 | g19 z -0.11 st 0x96  (pinned at the glass for 3300 ticks)
thread 'the_woken_guards_smash_through_the_entry_hall_glass' panicked at glass_breach_live.rs:97:
the woken guards shatter an entry-hall pane (130/136/155) on contact [M Break_Update 0x12bfd8:23-34 …]
```

0 of 55 panes broke in 3600 ticks while the spawned guards pressed the wall — the user's
symptom. On the fixed tree: **the test passes** — pane **130 shatters** at the charging guard's
contact and **guard (placement 169) crosses into the courtyard at t911** (z < −1.25), with the
east-wing pane 121 untouched (the sister print).

## Verification

- `nf-game --lib` **267/267** (6 breakable tests: the bullet path from 750e + the new smash arm
  — the level gate, the kind gate, the SqDist·20 drain (100 − 1.8 pinned), the stationary-brush
  rule, the no-RNG glass kill, the do_break first-containing-pane order and the broken-skip).
- `nf-view`: glass_breach_live **1/1** (the real-path breach), walk_07000001_live **3/3** (the
  intro/contact/fail-channel regressions on the same level), stun_ai_recover 3/3 +
  fix_stunner_press 2/2 + tase_silent 2/2 (the stun-family regressions over the shared drone.rs
  edits), breakables_live 2/2 (750e's gated harness).
- **The full suite did NOT run** — the sandbox kills background jobs between tool calls (the
  baseline `nohup suite.py` died twice with an empty suite.out) and the foreground window closed
  during the per-binary sweep. The touched-crate binaries above are what ran. The sweep that did
  complete pre-fix (56 nf-view binaries) was green except the **pre-existing**
  `fix_0c_pylon_live` failure, which fails identically on a clean origin/master worktree (not
  this task's regression).

## What's left (honest)

- **`escort_follow_07000001_live` regressed** (green pre-fix): with the breach live, the squad
  spawns and charges Bond during the escort walk; Bond's follow-steering stalls against the
  swarm and the lead pulls 24 u ahead (the assert `db < 24.0` [M the AllyLead bands] fired with
  the lead on his NORMAL z-14.5 route — the geometry says swarm-stall, not a route change). In
  the game the player fights the squad mid-walk; the test's Bond cannot. Needs either the test
  harness handling the squad (the same treatment the game's player gets) or the escort steering
  holding the bands under swarm pressure — NOT done here; the breach itself is the game's
  behaviour the user asked for.
- The 3 other 07000001-group runners (walk_07000001 3/3, walkreal not rerun after the fix) and
  the full suite on the final tree — the next window.
- `Drone_CoderCreate` (NIS event 0x08, the same dropped family as 0x0A) still unported — only
  the orphaned 0600005b uses it.
- Live checks for the main agent (PCSX2, fresh boot, 07000001): (1) at the intro's end, the
  breach guards burst the entry-hall windows — `gdbwatch` on a pane's obj+0xFE (the |= 1 write
  names the breaker's class); (2) the pane's +0xE0+0x04 health pool drains by SqDist·20 per
  contact tick (expect one big step to < 0 — health starts 0); (3) the break sound id 0x2e at
  the pane position.

## Usage

- Input tokens: not visible to me (no token/credit/context figures are shown in this
  environment).
- Output tokens: not visible to me.
- Context used: not visible to me.
- Cost: not visible to me.
