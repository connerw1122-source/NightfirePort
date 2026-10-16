## Your task: `fix-gas-rng4` — land 111d3's deferred impact draws on top of master's HITDATA walk

`s/fix-gas-rng3` (111d3, tip fbf62779) was HELD on 2026-10-15. It does not merge with master: a code conflict in
`crates/nf-game/src/drone.rs` (Bond's bullet world-hit block, about line 6530) and `crates/nf-game/src/effect.rs`
(a test's `bullet_impact` call, about line 385).
- Master has G-041: the player bullet walks the trace's HITDATA LIST inline. Every front-face hit takes the
  penetration roll `Rand_Rand(0x3f)` [M 0x12cfd0..0x12cfe4] and then the full effect stack, and a pierce retraces
  past the surface. `bullet_impact` also gained `&mut self.fx` and an extra trailing argument.
- 111d3 defers the impact effects' shared-RNG draws to a flush after the guard control loop (the collide pass,
  `Collide_Update` 0x1ea398 [M control_movement_object_handler 0x1336f8:238]) through `pending_world_impacts`, and
  moves the explode gate and the ricochet re-aim into that flush.

Both are about RNG draw ORDER, so a mechanical merge is wrong. Start from `origin/master` and port 111d3's
deferral onto the HITDATA walk:
- Decide from the disassembly where the penetration roll falls relative to the effect draws, given that both run
  inside `Bullet_CollisionHandler` during the collide pass.
- Decide whether a pierce's exit-side hits defer too.
- Make the flush keep the per-hit order.
Keep every G-041 test and every 111d3 test green, and re-run the cover/fire-rate AI tests the first gas-rng hold
broke. Report the draw-order table (site → draws → pass) and before/after tallies.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/fix-gas-rng4`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox1 at 2026-10-15T23:47Z

---
## Report (done, 2026-10-16T01:17Z)


---
## Report from the worker (done, 2026-10-16T01:18Z)

# fix-gas-rng4 — 111d3's deferred impact draws, ported onto master's HITDATA walk

Branch `s/fix-gas-rng4` (from `origin/master` 24b1f4dd). One file changes:
`crates/nf-game/src/drone.rs`. `effect.rs` and `gas.rs` are untouched — the held
merge's conflicts there existed only for a mechanical merge (111d3's 10-arg
`bullet_impact` tests against master's 12-arg signature, and 111d3's
`init_effect_puff` against master's counter-hash `init_ex`); porting the deferral
alone avoids both.

## What the game does (the order this lands)

The frame is two passes over one newest-first object list: the update pass
(`control_movement_object_handler` 0x1336f8 calls `control_funcs[class].update`
head→tail), then the collide pass (`Collide_Update` 0x1ea398 at 0x1336f8:238).
`Bullet_CollisionHandler` 0x12ce80 is a collide-pass handler, so EVERY draw it
makes — the penetration roll and the effect stack alike — lands after every
guard's update-pass roll. Its decompiled per-hit order (all [M 0x12ce80]):

1. the penetration roll: HITDATA+0x51 bit 1, surface byte 0x40 forced, else ONE
   `Rand_Rand(0x3f)` when the surface byte carries 0x80 [0x12cfd0..0x12cfe4] —
   drawn BEFORE any effect of this hit;
2. the bullet state (dead / pierce-alive);
3. `Effect_Create(1.0, HITDATA, obj, flags)` [0x12d060] — the debris/gas/decal
   stack AND the ricochet roll (`Effect_RicochetProb` 0x1cf290, mask 0 on a
   pierce contact — the roll draws, it just cannot reflect);
4. the three explode arms [0x12d068..0x12d188], each gated on the contact NOT
   being a pierce (`bnel $s4` reads bVar7);
5. else the ricochet re-aim [0x12d6f8..0x12d7f0];
6. loop tail [0x12d81c..0x12d82c]: non-pierce exits the loop, a pierce advances
   to the NEXT HITDATA — so a pierce's EXIT-SIDE hit takes its own roll +
   effects in the same handler call, same collide pass.

## The draw-order table (site → draws → pass)

| Site | Draws | Game pass | Port position before | Port position after |
|---|---|---|---|---|
| `Bullet_init` spread (the player's shot) | spread ×3 | update | update (unchanged) | update (unchanged) |
| Guard fire op `AnimProcessScriptCmds` 0x1c1b38:451-461 | hit roll + light + spread ×3 | update | `fire_weapon` inline (unchanged) | unchanged |
| Guards' AI rolls (`control_funcs` update walk 0x1336f8:62-66) | vision/decision rolls | update | guard control loop | guard control loop |
| Player bullet's world hits — penetration roll [0x12cfd0] | `Rand_Rand(0x3f)` per 0x80 surface hit | collide | mid-update, BEFORE the guard rolls (G-041 inline) | **flush: after the guard control loop, after the guard bullets' own impacts** |
| Player bullet's world hits — `Effect_Create`/`Effect_Bullet`/`Effect_RicochetProb` [0x12d060] | debris 1+8·n, the NPC coin (skipped for the player), the ricochet roll | collide | same inline position | **the flush, per hit, after that hit's roll** |
| Explode arms / ricochet re-aim [0x12d068/0x12d6f8] | none (they key on the roll) | collide | inline in the walk | **the flush, per hit** |
| Guard bullets' world hits | their own stacks | collide | guard-bullet block, post-loop (already correct) | unchanged |

Net per-tick draw count: unchanged. What moves: the player-bullet impact draws
shift from mid-update (before every guard's AI roll — the first gas-rng hold's
cover/fire-rate symptom) to the game's collide-pass slot, newest-first after the
guard bullets, exactly 111d2/111d3's model.

## The port

- `pending_world_impacts: Vec<(usize, V3, V3)>` — one record per player bullet
  whose flight segment ended on the world: `(bullet index, segment start, flight
  ray)`. The record carries the segment rather than per-hit payloads because the
  walk's continuation keys on the (deferred) penetration roll — the whole walk
  defers, entry face and pierce exit-side hits together.
- `flush_world_impacts(&d)` — called in `tick` right after the guard bullets'
  `retain` (111d3's slot). It replays the G-041 HITDATA walk per record in
  order, verbatim from the inline version: roll → `player_impacts`/accuracy log
  on the first world hit → the laser/sound gates → `bullet_impact` (mask 0 on a
  pierce) → the explode gate → else the ricochet re-aim → next HITDATA on
  pierce. The trace is bit-identical to the inline walk: the re-query is
  RNG-free, the static world cannot change mid-tick, `end = start + ray`
  recomputes the flight segment, and the pierce revive reuses the raw `ray` (the
  inline walk's `b.pos += ray`).
- The reflection reads the bullet's LIVE direction: the game's collide pass sees
  the update pass's `Bullet_homing` steer (0x12f358 runs from `Bullet_update`
  0x12e7b0:483), and the flight segment is the moved path (the pre-steer ray).
  This also makes the port's per-tick order homing → hit-processing, which is
  the game's (update pass → collide pass); master's inline order was
  hit-processing → homing.
- The dead-bullet `retain` moved from `update_player_bullets` to the end of the
  flush (111d2's device): a deferred ricochet/pierce revives its bullet in place
  through `player_bullets[bi]`, so the list order — and with it the per-bullet
  draw order — survives. No reader touches `player_bullets` between the walk
  and the flush (checked: the only consumers are post-tick).

## Deliberately out of scope

`fix_gas_rng_live.rs` (111d3's muzzle-smoke test) is not ported: it pins 111d2's
item 2 (the muzzle light + smoke latch in `scene.rs` + `gas.rs`'s
`init_muzzle_smoke`), which master deliberately keeps at the counter hash (the
`gas.rs` module doc: the muzzle/effects callers "keep the counter hash until
their own sites are re-based"). This task lands item 1 (the deferral). Every
test that exists on the branch — all of master's G-041 set and every 111d3-lineage
test shared with master — is green (tallies below).

## Verification (all release, `--test-threads=1`, NF_ISO on the slim ISO)

- `nf-game --lib`: **206 passed / 0 failed** (the G-041 `bullet_impact` unit
  tests, `penetration_roll`, decals, debris, ricochet, basis).
- `nf-game` integration (all binaries): **375 / 0** — including
  `ai_live` 35/35 (`guard_takes_cover_like_the_game`, the cover/fire-rate hold's
  test), `grenade_live` 2/2, `cover_selection_live` 2/2, `mp_match_rules` 14/0.
- `nf-view` (the shot/impact binaries run directly): `fix_guard_weapons_live`
  **3/0** (the 3×3 matrix + the fire-rate clamp + TooClose — the fire-rate AI
  test the first gas-rng hold broke), `models_misc_live` 10/0, `fire_modes3_live`
  5/0, `fix_weapon_effects_live` 6/0, `impact_remainder_live` 3/0,
  `fix_samurai_fire_live` 3/0, `weapon_fx_levels_live` 2/0, `fix_handcannon_live`
  3/0.
- `nf-view` (fast-runner-style per-binary slices over the remaining non-walkreal
  set): **506 / 0** across chunks (a few slow binaries — `fix_guest_reactions`,
  `fix_remote_mine_play` — hit my tool-call cap mid-binary; the binaries that
  completed all passed).
- `nf-formats`: 110 / 3 — the 3 `disc_golden` failures are the allowed slim-ISO
  ones (`bigf_header_fields_behave_as_documented`,
  `bigf_tables_match_the_golden_manifest`,
  `directory_tree_matches_the_golden_manifest`). `nf-audio`: 13 / 0.
- **Grand total: 1245 passed / 3 failed (the allowed disc_golden) / 1 ignored
  (pre-existing stun-grenade ignore).**
- Not run: the `walkreal_*` walkers (35 binaries, multi-minute live walks) and
  the tail probe chunks — window-cut, as every session on this suite; background
  jobs die between tool calls in this sandbox (the 23:59 `suite.py` start died,
  `suite.out` empty), so the runner's discipline (per-binary, `--test-threads=1`)
  was applied in foreground slices instead.

## Before/after tallies

| Test | Master (inline draws) | This branch (the flush) |
|---|---|---|
| `ai_live::guard_takes_cover_like_the_game` | green (seed 4 re-picked at the G-041 merge) | **green, same seed — no re-pick needed** (no player shots in it; the guard stream is untouched) |
| `fix_guard_weapons_live` (fire-rate/matrix/TooClose) | green | **green** |
| G-041 unit tests (`effect.rs`) | green | green (the file is untouched) |
| `models_misc_live`, `fix_weapon_effects_live`, `impact_remainder_live`, `fix_samurai_fire_live` | green | green (the flush replays the same draws within the tick) |

No test needed a re-seed: the deferral shifts only the player-shot stream's
position relative to the guards' update-pass rolls, and no branch test pins the
old (unfaithful) position.

## What's left

1. The full fast-runner total on the final tree including the `walkreal_*`
   walkers (multi-minute live walks — they have never fit a worker window; the
   driver's runner call should be re-run where the window allows).
2. `fix_gas_rng_live.rs`'s muzzle-latch site (111d2's item 2) lands separately,
   with `gas.rs`'s `init_muzzle_smoke`/`init_effect_puff`, and re-bases the
   muzzle/impact puff draws for real — then the gas.rs module doc's counter-hash
   caveat closes.
3. A live check for the main agent (optional, confirmatory): PCSX2, gdbwatch the
   shared seed 0x30d0a0/0x30d0a4 while Bond fires one shot at a wall with a
   patrol guard rolling in the same frame — the impact stack's draws (roll,
   debris, ricochet) must land AFTER the guard's update-pass rolls, matching the
   flush order this branch implements.
