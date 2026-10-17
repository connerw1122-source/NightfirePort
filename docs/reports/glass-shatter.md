# glass-shatter — breakable glass cannot be shattered

Task 750e, branch `s/glass-shatter`. First window (sandbox deadline hit mid-task).

## What's done

**The breakables system is ported and wired** (`crates/nf-game/src/breakable.rs`, doc
`docs/systems/breakables.md`, all addresses + evidence tags there):

- **The objects**: placement type 0x20 → `Break_Create` 0x12c230 — class 0x20, instance at
  `obj+0xE0` = { kind = placement param 0, health = param 1's float bits, bullet reject mask =
  `~(u16)param(2)` } [M 0x12c230:17-35]. `Breakables::create` builds them in placement order in
  `Drones::attach_level` (alongside `crate::props`), bounds per the props glist-bounds rule.
- **The damage path**: `Break_Update` 0x12bfd8 filters the pane's mirrored hit list with
  `Collide_FilterBullets(list, filter | 0x106)` [M 0x12bfd8:15] — class-5 (bullet) hitters whose
  `weapon_definition+0x10` word ANDs the mask are dropped [M 0x1ece30:24-33] — subtracts
  `HITDATA+8` from the health [M 0x12bfd8:36-37] and kills at < 0 [M 0x12bfd8:49-51]. Ported as
  `Breakables::bullet_hit`, called per player-bullet tick in `update_player_bullets` beside
  `props.bullet_hit` (drone.rs).
- **The death**: `Break_Kill` 0x12b970 — `obj+0xFE |= 1` (the pane leaves the world: no further
  hits, the collision "goes away") [M 0x12b970:60]; sound kind 2 → 0x2e, other kinds
  `Rand_Rand(100) < 0x29` → 0x5f2 else 0x5f3 [M 0x12b970:65-76] played through
  `Sounds::play3d` (the game's `Sound_Play3D(100, −1, −1, id, obj+0x80, custom=0)`). The roll is
  the game's own draw and is taken on the shared RNG **only** when a non-glass pane dies; the
  glass panes (kind 2) never draw, so the determinism stream is untouched for them.
- **Bullets never stop on a pane** — the finding that shapes the port: the panes' collision
  triangles all carry surface byte 0x10 (glass) and their glist flags (0x201/0x1201) lack 0x40/0x60
  [D census], so `Bullet_CollisionHandler` 0x12ce80's glass rule [M disasm 0x12cf44..0x12cf64,
  fix-rocket-glass-08] passes the round through an INTACT pane while the mirrored hit still
  breaks it. A shot shatters the pane and flies on; the second shot through the same spot breaks
  nothing. Non-breakable glass (the 08 corridor's world-geometry material-0x10 window) is not a
  0x20 object and never breaks.
- **The disc census** (`examples/probe_breakables.rs`, scratch): 443 breakable placements — 259
  kind 2, 171 kind 0, 13 kind 1; 07000008 has 3 `Glass_Controlroom*` panes (kind 2, health 0,
  no filter), 07000006 one `Window_Securityroom` (kind 2, filter param 0xf8 → mask 0xff07),
  07000005 none, 07000001 has 55 (the guards-smash-glass level).
- **State for the draw layer**: `Breakables::broken_at(placement)` + the public `list`
  (`scene_instances` appends the 0x20 placements in the same order — the renderer can zip them);
  `BreakEvent` carries position/kind/sound for the future shard/debris hookup.

## How it was verified

- **4 unit tests, green** (`cargo test --offline --release -j2 -p nf-game --lib -- breakable` —
  4 passed, 0 failed): one shot shatters a kind-2 pane with sound 0x2e; a miss does nothing;
  a broken pane registers nothing while the neighbouring pane still shatters; the
  `Collide_FilterBullets` reject mask drops matching weapon words (0x0008 vs 0xfff8|0x106) and
  passes others; pooled health whittles with the exactly-0.0-survives rule [M 0x12bfd8:49] and
  the kill plays a rolled 0x5f2/0x5f3.
- `nf-game` builds clean release; the gated live-test binary compiles and its gate runs green
  (`-p nf-view --test breakables_live`: 2 passed, early-return WIP mode).
- **The full suite did NOT run** — the background runner died to the sandbox's between-calls job
  reaping twice (empty `suite.out`), and the foreground chunks did not fit the remaining window.
  **The next window must run it** (only nf-game's breakable module + the drone.rs wiring changed;
  the 08 rocket test's RNG stream is unaffected — kind-2 kills draw nothing).

## What's left (the honest gap)

- **The real-path test is not green yet** — the task's "shooting a breakable pane on the real
  path" assertion is written (`crates/nf-view/tests/breakables_live.rs`, gated behind
  `NF_BREAKABLES_LIVE=1` so it stays out of pass/fail) but **blocked on the fire harness**: from
  `Bond::stand_at` inside the 08 control room, aiming at the pane, the gun fires (clip 7 → 6,
  weapon 2 after `equip_weapon` + `switch_request = 2`; weapon 1 — the level's default "clip 1" —
  never fires at all) but **no bullet ever appears in `sim.drones.player_bullets`** and no
  `player_impacts` record lands (repro: `cargo build --release --example probe_glass_shot -p
  nf-view && ./target/release/examples/probe_glass_shot`, the trace prints TAPs with a
  decrementing clip and no bullet lines). The same harness pattern works in `rocket_glass08_live`
  (fired from the level start) — the divergence is the stand_at spot deep in the level vs the
  spawn, or a muzzle/cel condition the port's fire path checks. Next window: debug the fire from
  a stand_at position (compare against a walked approach), then un-gate the two tests:
  1. shot 1 at the front pane (placement 176) → breaks, sound 0x2e live; shot 2 same spot →
     nothing new; the side pane (170) then still shatters (sister);
  2. the corridor default-facing shot pierces the non-breakable world glass (impact z < −98.5)
     and no breakable breaks.
- The suite run on the final tree (above).
- Not ported (documented, non-blocking): the kind-2 shard/debris presentation
  (ShardList/Debris_Create_Loop — the draw layer builds on `BreakEvent`), the
  guards-smash-glass arm (0x12bfd8:23-34, level 07000001), `Break_DoBreak` 0x12c168 (NIS event
  0x0a, one disc occurrence).

## Live checks for the main agent

1. **A shot at the 08 control-room window**: expect the pane to shatter (sound 46, the shard
   polys), the round to continue past it, and Bond to be able to shoot through the empty frame.
2. **The intact pane vs the crosshair**: confirm bullets do NOT stop on intact panes on PS2 (the
   decompile says they pass — material 0x10 — but no test here can pin it yet).

## Usage

Token, context-window and cost figures for this session: not visible to me. The environment
shows no per-session token or cost counters I can read from the sandbox.
