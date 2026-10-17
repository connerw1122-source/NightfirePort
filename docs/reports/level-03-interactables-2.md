# level-03-interactables-2 — the re-land of the level-03 breakables onto master

Branch `s/level-03-interactables-2`, from origin/master 0083ce97 + merge of origin/s/level-03-interactables (bd4af099).

## What was done

The re-land keeps ONE breakables system: master's `crate::breakable::Breaks` (`Drones::breaks`,
`Break_Create` 0x12c230 / `Break_Update` 0x12bfd8 / `Break_ApplyDamage` 0x12c128 / `Break_Kill`
0x12b970), with the level-03 branch's evidenced features ported onto it. The level-03
`Breakables`/`Drones::breakables`/`BreakEvent`/`drone_smash`/`prev_pos` are gone.

Ported from level-03 onto `Breaks`:

- **The bullet reject mask, evidenced form** [M 0x12bfd8:15 + 0x1ece30:24-33]: the contacts filter
  through `Collide_FilterBullets(list, filter | 0x106)` — a class-5 hitter whose
  `weapon_definition+0x10` word ANDs the mask is dropped. `Breaks::bullet_hit` now takes the
  bullet's weapon word (guns 0x0008, Laser 0x0002, watch 0x0100 [L dump]); the base 0x106 mask
  rejects the laser/watch classes even with no filter param, guns pass. This replaces master's
  `[S]` stopgap (`1 << (weapon & 15)` — the weapon-bit layout had no evidence; the drone.rs call
  site now reads `weapons.records[b.weapon][0x10..0x12]` like the game).
- **The per-kind kill sounds, evidenced form** [M 0x12b970:65-76]: kind 2 → 0x2e with NO RNG
  draw; every other kind rolls `Rand_Rand(100) < 0x29` → 0x5f2 else 0x5f3, drawn only when a
  non-glass pane dies. This replaces master's deterministic-0x5F3-no-draw deviation (also an [S]
  stopgap) — master's old unit test pinning an unconditional 0x5F3 contradicted the decompile and
  was fixed (see "Test changes").
- **`do_break`** [M Break_DoBreak 0x12c168:39-61] — the scripted kill of the first intact placed
  breakable whose bounds (grown 0.2 [S]) contain the point, wired into master's NIS
  `Request::DoBreak` arm BEFORE the script-entity scan (the game walks the control list in
  creation order: placed 0x20 objects precede script entities [M 0x12c168:39-51]); the sound
  plays via `audio.play3d` and the kill joins `break_kills`.
- **The glass-breach body side**: world.rs keeps the 0x20 placements out of the static bake;
  drone.rs injects each INTACT pane per tick through `set_dynamic` (id `0x300_0000_0000 |
  placement` — the level-03 pick collided with the SimpleScripts' `0x200_0000_0000` id, fixed
  here), a broken pane injects nothing [M 0x12b970:60 — obj+0xFE |= 1 takes it out of the world].
- **The class-2 contact, upgraded**: `Breaks::drone_step` keeps master's signature; internals =
  the movement segment vs the kind-2 bounds grown 0.7 [S — the game's contact is the body volume
  against the pane's glist triangles; 0.7 covers the port's blocked standoff], drain
  `SqDist(from, to) · 20`, level 0x0700001 only [M 0x12bfd8:23-34]. **Bond joined the pass**
  (`player_pos` → `player.pos`, the tick-start cache) — the player is a class-2 body too.
  Master's per-drone `prev_pos` field was NOT taken (master's tick-start `drone_from` snapshot
  already holds the pre-move positions). Kill sounds play via `audio.play3d`
  [M 0x12b970:69,75 — Sound_Play3D(100, −1, −1, id, obj+0x80, custom=0)] and join `break_kills`.
- **`broken_at(placement)`** getter + `shape_key` per breakable (the hashcode or world.rs's
  anonymous synthetic [S]) for the injection.
- **The disc census** [D, `probe_breakables`] into the module doc: 443 placements — 259 kind 2,
  171 kind 0, 13 kind 1; plus the pierce note (the panes' surface byte 0x10 and the glass rule
  [M 0x12cf44..0x12cf64] already ported on master by fix-rocket-glass-08 in crate::effect).

The level-03 tests came across and run on the new API: `breakables_live.rs` (env-gated
NF_BREAKABLES_LIVE, the 08 pane-shot walks), `glass_breach_live.rs` (the Mayhew breach walk),
`interact_07000003_live.rs` (the per-kind walks; the breakable pane walk stays env-gated
NF_INTERACT03_ALL), the probe examples, and the system docs (docs/systems/breakables.md,
glass-breach.md, docs/reports/glass-{shatter,breach-guards}.md, docs/reports/level-03-interactables.md).

## Test changes (with addresses)

- **`interact_07000001_live::bullets_shatter_the_glass_and_vases_and_the_sister_stays_whole`** —
  the fresh-load sister pinned "every breakable whole". That contradicted the game's own scene on
  07000001: the intro wakes the entry-hall squad and their class-2 contact smashes the entry-hall
  glass [M Break_Update 0x12bfd8:23-34 — the arm the re-land reproduces; the user report
  "beginning of mayhew level guards are supposed to break through the glass"]. Fixed to pin the
  decompile instead: without a shot only kind-2 glass may die (the kind gate [M 0x12bfd8:25]);
  every kind-0 vase stays whole. The test now PASSES with the breach reproduced.
- **`interact_07000003_live::a_bullet_shatters_the_glass_breakable`** (env-gated) — it called
  `broken_at(bi)` with the LIST INDEX where the getter takes the PLACEMENT (a latent bug from
  703a; the sister passed vacuously). Fixed to `broken_at(241)`.
- The incoming tests were renamed onto master's API (`drones.breaks`, field `dead`), and the
  breakable.rs unit tests were reworked for the evidenced filter/sound rules (words 0x0008/0x0002/
  0x0100/0x0001 vs filter 0xfff8; the roll pinned to 0x5f2/0x5f3).

## How it was verified

- nf-game lib: **282 passed / 0 failed** (incl. 10 breakable units: filter reject matrix, the
  roll, the drone arm's level/kind/motion gates, do_break first-intact, broken_at).
- nf-game integration binaries: all green (no FAILED).
- nf-audio 15/0; nf-formats green except the 3 expected slim-ISO `disc_golden` failures.
- nf-view: `interact_07000003_live` 9/9 (census 97 breakables [D] on master's `Breaks::create`),
  `interact_07000001_live` 10/10 (the fixed sister; the breach kills only kind-2), 
  `glass_breach_live` **1/1 — the breach walk reproduces the user's scene: pane 130 breaks at
  ~t300, guard 18 crosses the plane at t911, the east-wing pane 121 stays intact**.
- The full four-crate suite runner (`tools/test/suite.py`) cannot run in this sandbox: background
  jobs are killed between tool calls (both attempts died instantly; the p2k-fire-07 session saw
  the same). The crates were run in the foreground instead: the numbers above, plus the nf-view
  alphabetical run a–cutscene… (31 binaries, 0 failures) in the time left. The rest of nf-view
  (the d–z walk binaries) ran after this report was drafted — see the WORKLOG entry for the final
  tally.

## What's left

- **`escort_follow_07000001_live` FAILS on this branch** (the contact parks at closed door 1,
  state 0x2, at (7.9, 11.1) for 12 rounds; Bond E-opens it each round and the contact does not
  cross — trace in the WORKLOG). The stall point is a DOOR, not a pane, and no master baseline
  could be run in this sandbox (background jobs die between tool calls), so master-vs-branch is
  unproven. First job of the next message: run it on a clean master worktree; if master passes,
  suspect the escorts' hearing of the now-playing break sounds (the game plays them in-world
  [M 0x12b970:69,75]) or a doubled body at a pane that shares a map object with door 1.
- The env-gated pane-shot walks (`breakables_live`, the NF_INTERACT03_ALL walks) need the
  fire-at-pane harness proven on the current tree (they were WIP on 703a; the 07000001/07000003
  gun paths now work, so they are worth un-gating next task).
- The 0.7-grown contact is still [S]; a live check (`tools/trace/fieldprobe.py`) measuring the
  game's blocked-guard standoff at pane 136 on 07000001 would replace the guess.
- `Break_DoBreak`'s sphere-radius stand-in (bounds grown 0.2) is [S]; the disc's single firing
  (0600005b stream 8) is orphaned, so only a live check can measure obj+0x8c.

## Usage

not visible to me (no token, context-window or cost figures are exposed to this session).
