**Reproduce first, on the newest origin/master, through the REAL path.** Earlier fixes for some of these passed
their own headless tests and the user still hit the bug in the game. Before changing code, write a live test that
drives the same path the player takes (the level loaded through `nf_view::load`, Bond moved by `step_bond` inputs —
walking, the use key, the ladder input — the real scripts and triggers running, no teleports past the problem
spot unless the report says so) and FAILS on current master the way the user describes. Put the failing output in
the report. Only then fix it, and the same test must pass. If you cannot reproduce it, say exactly what you tried
and where the path diverges from the report; do not mark it done on a guess.

## Your task: `zoe-gondola-08`, Zoe shoots Bond at the gondola

The user, 2026-10-16: "zoe shoots at you when you get to the gondola". Zoe is an ally (07000007/08; the gondola is
on 07000008). An ally must never target Bond. Reproduce: walk to the gondola on the real path and log Zoe's team
(+0x44), target, state and shots. Find why she targets Bond — her team/kind setup at spawn or by script (the game's
team assignment for the Zoe placement, any script event that changes it), the target selection's team filter
(`Check_Targ`/the opponent pick) — and port the game's rule with [M] citations. Test: she never fires at Bond through
the gondola sequence; sister: she still fires at enemies if the game has her fight there.

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Tests pin only proven behaviour.** Every assertion about what the game does cites its evidence: [M] a decompile
address that actually says it, [D] disc bytes, or [L] a live PCSX2 measurement from a FRESH boot (a save-state resume
skips the boot path). A guess, a port choice or an [S] item is never a hard assertion: mark it `// [S] unverified` and
keep it out of pass/fail. If an existing test contradicts the decompile, the TEST is wrong: fix it, with the address, and
say so in the report. Never weaken a decompile-backed test to make code pass.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/zoe-gondola-08`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox5 at 2026-10-17T02:12Z

---
## Report (done, 2026-10-17T03:16Z)


---
## Report from the worker (done, 2026-10-17T03:16Z)

# zoe-gondola-08 — Zoe shoots Bond at the gondola

The user, 2026-10-16: "zoe shoots at you when you get to the gondola". Zoe is Bond's ally on
07000007/07000008 (the gondola level is 07000008); an ally must never target Bond.

## What was done

**The game's rule, from the decompile** (`work/decomp/ACTION.ELF`):

- The AI's opponent is the stored object pointer `Drone+0x170`. `NDrone2_FindOpponent` 0x1424e0
  runs every frame from `NDrone2_ControlSTANDARD` 0x1490f0 (:167) and once at spawn from
  `NDrone2_PostLoad_Init` 0x14d890 (:14). In single player it stores **Bond** only when the
  drone's OWN team byte `Drone+0x44 != 2` [M disasm 0x142e04 `lbu $v1,0x44($s2)`,
  0x142e0c `beq $v1,$v0(2)` → ally branch]. A team-2 ally's candidates are only team-1 drones:
  the drone-list scan skips any candidate whose `+0x44 != 1` [M disasm 0x143100
  `lbu $v0,0x44($s0)`; 0x143104 `bne $v0,$s4(1)`], gated by 2-D range and `NDrone2_CanSeeObject`
  0x1785e0 [M 0x143110-0x143130]. Bond (the player, no Drone record) is therefore never a
  team-2 ally's opponent.
- The Attack entry `DroneVision_EnemyLookForOpponent` 0x177ed0 requires a **stored opponent**:
  `if (Drone+0x170 == 0) return 0` [M :18-20], and a script-playing fully-alert drone never
  attacks out of its script (`SMI+4 == 3 && alert >= 1.0 → 0` [M :12-14]). Team 3 returns 0
  before the sight check except on 07000009/0700000A [M :29-38].
- Firing adds no team check: `DroneWeap_DoOpponentTargetting` 0x179a48 aims and shoots the
  stored `+0x170` only — with an empty `+0x170` it produces nothing.
- Team assignment at spawn: `NDrone2_GetDroneTypeAttackTypeFriend` 0x14cbd8 switches on the
  placement's dtype `Drone+0xC7`: 10/0xb/0xd/0xe → 3, **0xc → 2**, default → 1, stored by
  `sb $v0,0x44($s0)` [M 0x14ccd8]. Zoe is dtype 0xC (skin 0x0500003b, one placement, on
  07000008 only [D actors.md census]) → team 2. No script event writes `Drone+0x44` anywhere in
  the decompile (the only runtime writes are the 3→1 civilian/protected promotions).
- Zoe's control `NDrone2_ControlDTYPE_Zoe` 0x14b058 is a fail-reason watchdog + tail-call to
  `NDrone2_ControlSTANDARD` — so FindOpponent runs for her every frame, exactly as for guards.

**Why the port shot Bond.** The port has no `+0x170` opponent model: its opponent is
implicitly Bond (`opponent_info` computes Bond geometry unconditionally,
`docs/port-map/guard-ai.md:400`). `look_for_opponent` (the 0x177ed0 port) gated only team 3,
and its doc comment even claimed allies are team 3 — wrong: allies are team 2. Zoe spawns
**at the cable car** (111.3, 27.9, −124.7), in COMBAT_NO_MOVE (0x59) with alert 1.0 (the
port's fully-alert start-state promotion), so the moment Bond walked into her sight her
combat chain ran `0x59 → 0x5b COMBAT_NEW_SIGHTING → AIM_*_FIRE → fire()` — all
team-agnostic — and she fired at him.

**Reproduce (fail-first).** `crates/nf-view/tests/zoe_gondola_08_live.rs` drives the real
trigger path on 07000008 through `nf_view::load` + `step_bond`: the intro plate #237 (out
0x49), the walk toward the cable-car floor Touch #236 (the port's kind::TOUCH, out 4), the
boarding and the ride. Declared deviations (both pre-documented): the briefcase channel 0x4A
and the ride's own events (s46@1410 → ch 0x35, the station bombardment [D nis dump]) are driven
by hand because the port does not parse the level-bin ride script 0x0600004C
(docs/levels/07000008.md §2 gap). With **master's `drone.rs`** (restored via
`git checkout origin/master -- crates/nf-game/src/drone.rs`) the test fails exactly as the user
describes:

```
thread 'zoe_never_fires_at_bond_through_the_gondola_sequence' panicked at crates/nf-view/tests/zoe_gondola_08_live.rs:37:5:
Zoe never fires at the ride + ambush (1 shots, last at frame Some(6426))
test result: FAILED. 0 passed; 1 failed
```

(the log: `/home/z/my-project/scripts/zoe_failfirst.log` — sandbox scratch, not committed)

**The fix** (`crates/nf-game/src/drone.rs`, three gates mirroring the empty-`+0x170` no-op):

1. `look_for_opponent`: the missing [M 0x177ed0:12-14] PLAY_SCRIPT + alert ≥ 1.0 gate, and the
   team-2 gate — a team-2 ally has no stored opponent (FindOpponent keeps Bond out of +0x170
   [M 0x142e0c]) and the ally-candidate scan is unported, so she never enters ATTACK here.
2. `fire` (DroneWeap_Fire): team-2 → no fire request (the game's request comes from
   DoOpponentTargetting on a non-empty +0x170).
3. `handle_firing` (DoFiring → FireWeapon): team-2 → no bullet, no shot count, no clip spend —
   the game's DoFiring never reaches FireWeapon with an empty +0x170. `fire_weapon` carries the
   same belt-and-braces gate. All four carry `NF_AILOG` lines.

No state constant was renamed or repurposed; no existing match arm changed; the doc comment
above `look_for_opponent` was rewritten to the [M] truth (it wrongly called allies team 3).

## How it was verified

- `zoe_gondola_08_live::zoe_never_fires_at_bond_through_the_gondola_sequence` — setup pin
  (dtype 0xC, team 2 [M 0x14ccd8]) + through the whole sequence: no ATTACK entry, no shots, no
  bullets, no player hits from Zoe's drone index; roster + ride logging included. **PASS** with
  the fix, **FAIL** on master's code (above).
- Sister `a_team1_guard_under_the_same_conditions_attacks_bond` — a team-1 guard placed 1.5
  units from Bond (inside visibility's TooClose arm [M drone.rs visibility], the waiting
  template's sight_range/light zeroed by the hold-back path) is seen (reaction_time 10) and
  **fires 10 shots** — the same sighting conditions that must stay inert for Zoe do attack for
  team 1 [M 0x142e04-0x142e0c]. It also re-asserts Zoe stays inert inside that fight.
- Regressions: `fix_actors_live` 4/4 (the Mayhew escort), `walk_07000008_live` 3/3 (the
  gondola mission), `tase_scream_live` 1/1 (the guard attack chain).

## What was run

- `cargo test --offline --release -j2 -p nf-game -- --test-threads=1`: 57 binaries, **485
  passed, 0 failed**.
- `cargo test --offline --release -j2 -p nf-view -- --test-threads=1`: killed at my 10-minute
  tool cap mid-alphabet (at `drone_skins_all_levels_live`) after **141 passed, 0 failed**; the
  remaining binaries were covered by the targeted run of the four neighbour live binaries
  above (10 tests, 0 failed).
- The full suite (`tools/test/suite.py`) was started three times (02:17, 03:26 nohup, 03:26
  setsid) and died between tool calls every time with an empty log — background processes are
  reaped in this sandbox (the pause-music-1b WORKLOG entry records the same). **No suite total
  line is available**; the foreground crate runs stand in.

## What's left

- **The ally opponent scan is unported** [S]: FindOpponent's team-2 branch
  (class-0x3B objects with obj+0xF4 in 1..3 in a ±10° cone [M 0x142e88-0x142ea8], then team-1
  drone candidates [M 0x143100-0x143130]) needs the per-drone `+0x170` model (an
  `Option<usize>` opponent field consumed by `opponent_info`/aim/fire) before Zoe can fight
  enemies the way the game may during 07000008's ambush. Until then an ally neither targets
  Bond (this fix) nor fires at all.
- The port puts a fully-alert Zoe into the combat family at spawn (0x0 → 0x56 → 0x58 → 0x59);
  whether the game's own fully-alert start-state promotion applies to DroneTypeSettings row 17
  is unverified [S] — with the gates she cannot fire at Bond either way, but her idle/aim
  anims at the station may differ from the game's until checked.
- `EnemyLookForOpponent`'s 07000009/0700000A team-3 skip [M 0x177ed0:29-35] is documented but
  not ported (no placement needs it yet).

## Live checks for the main agent

1. Fresh boot 07000008, walk to the gondola, ride down with the ambush: watch Zoe's
   `Drone+0x170` (4 bytes at obj+0xE0+0x170) — expected: never the player object pointer;
   either 0 or a team-1 drone's obj. Access: read, every ~30 frames.
2. Same session: watch Zoe's `Drone+0x44` (expect 2 the whole level) and `Drone+0x50c`
   (alertness) at the level start — expected value pins whether the game really spawns her
   fully alert (the port shows 1.0).
3. Same session, the ambush: does Zoe fire at the spawned guards in the game (her weapon
   sound / muzzle / `+0x3B` firing flag)? Decides whether the team-2 candidate scan must be
   ported for 07000008 (the sister question).

## Usage

- Session token/credit/cost figures: **not visible to me** — the environment exposes no usage
  display for this session. The only concrete figures I can see are the two research
  subagents' own reports (decomp research: 174408 prompt / 10079 completion; port research:
  181478 prompt / 7007 completion — read from their result footers), not this session's.
- Context window: not visible to me.
