# MP match rules — scoring, win conditions, respawn and the scenario updates

The per-scenario multiplayer rules behind the menu flow ([mp-menus.md](mp-menus.md) hands the match
over as `MpSettings`): kill scoring, the end-condition latches, the who-won decision, the respawn
rules and every scenario's objective update. All of it lives in `crates/nf-game/src/mp/` (`mod.rs`
core + `rules.rs`), driven headless — numbers only, no rendering.

Offset convention: the decompiler prints Ghidra **decimal** offsets (`MPGame._412_4_`); the port's
field comments carry those same decimal prints, and this doc gives the real hex offset alongside
where it matters (`MPGame._412_4_` = hex `+0x19A`).

## The core state (`MPGame` @0x2a4980, `MPSettings` @0x2a47a0)

- Per slot (8 × 0x30): +0x04 kills i32, +0x08 deaths i32, +0x10 streak i32, +0x18 score f32,
  +0x1c obj ptr, +0x20 u16 last-shooter word (init 0xfffe [M 0x183438:59]; written per bullet hit
  [M 0x1857c0:34]), +0x22/+0x24 u16 message timers (decremented per frame [M 0x184bd0:25-35]),
  +0x26 u16 status bits (bit0 carrying-flag, bit0x10 on-hill [M 0x18a518:88-96, 0x186ae0:95]),
  +0x28 u16 last killer (init 0xffff [M 0x183438:58]).
- Team scores f32 at +0x180/+0x184 [M 0x1861a8:93, disasm-verified]; the running **high score**
  (max personal or team, i32) at +0x18C (`MPGame._396_4_`) [M 0x184268:167-172]; match state at
  +0x188 (`._392_4_`); round clock (s) +0x190 (`._400_4_`), limit (s) +0x194 (`._404_4_`), elapsed
  +0x19A (`._412_4_`), debrief timer +0x19E (`._416_4_`), restart timer +0x198 (`._408_4_`),
  pause stamp +0x1A8 (`._420_4_`).
- Settings: scenario +0x1A4 hex (`._420_4_`), team-game `(scenario>>29)&1` → +0x18C hex
  (`._396_4_`) [M 0x183438:41], bit30 `(scenario>>30)&1` → +0x190 hex (`._400_4_`) [M 0x183438:42],
  frag/death limit +0x19C hex (`._412_4_`, −1 = off), time limit minutes +0x1A0 hex (`._416_4_`,
  −1 = none), total players +0x194 hex (`._404_4_`, written by `MP_Start` 0x183a50:164),
  player count +0x1AC hex, bot count +0x1B0 hex, **friendly fire** +0x198 hex (`._408_4_`) [M
  0x1857c0:45].

## Kill scoring — `MP_PlayerKilled` 0x188f70

- deaths (slot+0x08) += 1 in every scenario [M 0x188f70:92-94]. Scenario 0x10 (Top Agent): the
  slot score (the lives copy, +0x18) −= 1.0, and `frag_limit <= deaths` = the player is out [M
  0x188f70:96-100].
- Self-kill: streak = 0, kills −= 1, last-killer = 0xffff [M 0x188f70:151-155]. Another player:
  last-killer = attacker slot [M :164]; kills/streak += 1 only when FFA or different teams [M
  :157-163].
- Score delta: −1.0 self-kill, +1.0 kill, +2.0 a bot (slot > 3) golden-gun kill (BLData+0xd1c →
  +0xab == 7 and +0x76b == victim slot) [M :167-178]. The delta lands on the attacker's score
  **only** when `bit30 == 0` **and** scenario ∈ {0x1, 0x20000002, 0x40000800, 0x60001000} [M
  :179-184] — KOTH's scenario ids are listed but bit30 = 1 for them, so kill scoring is
  effectively Deathmatch/Team Arena only; never in 0x10/0x400/CTF/Uplink/Demo/Prot/GE/Blueprint.
- Team-score effects, scenario 0x20000002 only [M :246-274]: enemy kill → the opposing team += |Δ|
  (victim team 0 → +0x184, else +0x180); same-team kill or no-credit death → the victim's team −=
  |Δ| (a self-kill costs the team 1).
- Scenario 0x400 (Assassination): the assassin killing the target +5.0 (sound 0x14b); the target
  killing the assassin +3.0, any other death of the assassin scores nothing; then
  `MP_assassinReset(1)` [M :229-243]. These bypass the bit30/scenario gate.
- Carried objects drop: every registered MP object owned by the victim frees its hit list, and —
  types 0 (flag) / 5 (blueprint) / 6+7 (GoldenEye) — its update runs with died=1 [M :276-303]. If
  the killing hurt type was 4..7 (explosive) the object's return counter preloads to
  `30*FRAME_RATE_INT` frames [M :193-198, 276-299].

### Assassin reset — `MP_assassinReset` 0x18aa70

The dead assassin's slot team words are zeroed, the new assassin = the dead assassin's last-killer
slot when it killed them (else a random pick), the new target = random among the other players
(`MP_GetTarget` 0x1832b8, `Rand_Rand`), and the assassin's settings record team word = 1 (everyone
else 0) [M 0x18aa70:12-41]. The port keeps `assassin`/`assassin_target` slots on `MpMatch` and
draws through the shared bit-exact `Rng`.

## Friendly fire — `MP_RegisterBulletHit` 0x1857c0

MP off → allowed; no attacker → blocked. Team game (`+0x18C hex != 0`): different teams → allowed,
self-hit → allowed, `friendly_fire == 0` → **blocked**; otherwise the warn messages go out and the
victim's +0x22 timer = 3 s (decremented by `MP_Update`) [M 0x1857c0:33-57]. The victim's
last-shooter word (+0x20) = attacker slot in every allowed case [M :34].

## Respawn — `MP_ReSpawn` 0x185368

- Gates (return 0): master off; `MPGame` state != 0 (round over); scenario 0x10 with
  `frag_limit <= deaths` (out of lives) [M 0x185368:16-20].
- The position comes from `MP_GetSpawnPoint` 0x185098 (ported in phase 1). Reset: streak = 0,
  last-shooter = 0xfffe, status bits = 0 [M 0x185368:55-57]; human health =
  `MPSettings[slot].armor_bonus + 100.0` [M :65-73]; weapons via `MP_EquipPlayer` 0x1855f8 (the
  weapon-set row of `PickupMatrix`, the capture-reward second weapon when `+0x1BC hex != 0`, and
  the scenario-bonus weapon 0x3b for Protection/team 0 or Demolition/team 1) [M 0x1855f8:14-47].
- Timing: the caller (`Player_HandleDeath` 0x190bb0) respawns only after 5 s of death and, in 0x10,
  only while `deaths < limit` [M 0x190bb0:41-54].

## End condition — `MP_CheckForEndCondition` 0x184268

Per frame (unpaused): elapsed (+0x19A) += dt, round clock (+0x190) += dt [M :44-46,55,140].

- Demolition (0x20000040) / Protection (0x20000080): `limit > 0 && limit <= clock` latches
  `switch_channels[0xfe]` and enters state 6 (round over) [M :57-60].
- Scenario 0x10: counts humans out (class 0x12, `limit <= deaths`) and bots out (0x11, or 0x02
  with the invuln bits); `(total_players − 1) <= out` or `humans_out == player_count` → state 1
  [M :85-135]. When all humans are out and bots remain, one live bot is marked out (class → 0x11)
  [M :110-133].
- Time-up: `time_limit != −1` latches `switch_channels[0xfe]` when `limit <= clock` [M :142-145].
- High score (+0x18C) = max(team 0, team 1) always, then max over the 8 slot scores for FFA — the
  demo/prot branch takes both team and personal maxes [M :147-172].
- Score limit: `frag_limit != −1` latches `switch_channels[0xfd]` when `frag_limit <= high` —
  skipped for 0x10 (lives only) [M :170-172].
- Final: `state = 1` if the frag latch, else `2` if the time latch [M :173-184].

### State machine — `MP_Update` 0x184bd0

State 0 plays (the port's `update`). State 2 (time-up): scenario 0x10 **falls through** to state
1's body, every other scenario → state 3 with the time-up text [M :42-49]. State 1 →
`MP_SortOutWhoWon` [M :51-52]. State 3: non-0x10 goes to 4 at once, 0x10 accumulates the debrief
timer (+0x19E) and waits 5 s [M :54-61]. State 4 loads the front-end map (state 5) [M :62-78].
State 6: the restart timer (+0x198) runs; at 5 s → state 0 + `MP_RestartScenario` [M :79-92].

### Who won — `MP_SortOutWhoWon` 0x1848d0

Sets state 3 [M :20]. 0x10: the winner is the **unique** alive player with the fewest deaths; a tie
at the minimum (or nobody alive) is a draw [M :21-54]. FFA: if exactly one player sits at/over the
limit that player wins, otherwise the draw text [M :55-86]. **Team games never compare the team
scores here** — always the generic text; the scoreboard is the judge [M :87-92] (the port returns
`Winner::None` there and the report notes it).

### Round restart — `MP_RestartScenario` 0x183fc0

Round clock = 0, a new random site/item is picked from the registered places (`Rand_Rand`), every
human and bot respawns, and every slot's last-killer (+0x28) and last-shooter (+0x20) reset to
0xffff [M 0x183fc0:40-82]. Scores are **not** cleared (multi-round Demolition/Protection).

## Scenario updates (`MP_ObjectUpdate` 0x18b198 dispatch)

Object live state (`obj_tag+0xF4/+0xF6`, `MPOBJECT+0x02 team / +0x04 counter / +0x08 carrier /
+0x50 last-hitter / +0x52 controller`) is carried by `rules::MpObjState`; the port's `MpObjectsRt`
mirrors the game's per-type globals (`Flags`[2] @0x317210, `Bases`[2] @0x317330, `Uplinks`[8]
@0x317450, `Hill` @0x318CE0, `GoldenEye`[2] @0x317FF0/0x318080, `BluePrint` @0x318830,
`EsponageBase`[2]) [M 0x185de8:133-178]. Timers count **frames** at 60 Hz; the drip scores use
`REC_FRAME_RATE * 0.2` = 0.2 points/s [M 0x186d98:59-60, 0x186ae0:118-121].

- **CTF 0x20000004** — `MP_FlagUpdate` 0x1861a8. States: 0 at base, 1 carried, 2 dropped. Pickup:
  an enemy touch takes the flag (any state 0/2) [M :170-190, 223-263]; a teammate touching a
  dropped flag returns it at once [M :195-221]; a dropped flag otherwise returns home when its
  counter passes `30*FRAME_RATE_INT` frames [M :226-259]. Capture: the carried flag within
  sq-dist 2.0 of the enemy base pos +1.0 y → **team score of the flag-taker's team += 1.0** (flag
  team 0 → +0x184) and the carrier's score += 1.0, then the flag resets home [M :74-112].
  `MP_HasTeamFlag` 0x18b430: the team's flag's holder == the player [M 0x18b430:9-11].
- **Uplink 0x60000008** — `MP_UplinkUpdate` 0x186d98. Terminal states: 2 neutral, 0/1 owned by
  team 0/1. Every frame an owned terminal drips 0.2/s into its team score; a recorded controller
  (+0x52) also drips 0.2/s personally; in team games the owning team's score is then **recomputed
  as Σ personal scores of its members** [M :59-80]. An enemy touch flips an owned terminal at
  once (state, colour, controller, sound 0x14f); a neutral terminal is claimed by the first
  toucher [M :24-114]. Win: the global limits only.
- **King of the Hill 0x40000800 / 0x60001000** — `MP_KOHUpdate` 0x186ae0. Every alive player
  intersecting the hill drips 0.2/s personally; in team games the scoring player's team score is
  recomputed as Σ that team's personals [M :78-151]. Everyone on the hill scores at once; no
  ownership. The status bits carry the on-hill marker (0x10) [M :95-115].
- **Demolition 0x20000040 / Protection 0x20000080** — `MP_DemolitionProtectionUpdate` 0x1870a0,
  one active site, HP = obj+0xF4 init 2000 [M 0x185de8:174], `+0xF6 = 1` marks the round done.
  Damage subtracts from the HP and records the last hitter (+0x50) [M :81-91]. HP < 1: the
  attacking team's score += 1.0 (defenders team 0 for Demolition, team 1 for Protection — the
  attacking offset is `(param_3==0) ? +0x184 : +0x180`), the last hitter +1.0 personally — an
  own-goal (the destroyer defending) instead subtracts the defender team 1.0 and the hitter's 1.0
  — then the site is done and the round enters state 6 at once [M :125-165]. Timeout (`limit <
  clock`): the **defending** team += 1.0 and the site is done [M :112-124]. Between rounds
  `MP_RestartScenario` re-picks the site and respawns everyone [M 0x183fc0:40-82].
- **GoldenEye Strike 0x20000200** — `MP_GoldenEyeUpdate` 0x187568. Exactly two neutral items
  (types 6/7, sub 0/1); states 0 free / 1 carried / 2 dropped / 3 strike. Any player may carry
  either (team wildcard) [M :231-253]. When both items are carried by the same team, the strike
  launches: each carrier +1.0 and the target is picked from the opposing team (`MP_GetTarget`,
  random) [M :356-384]. When the strike completes (SP state 2 — the port exposes
  `ge_strike_complete()`; the flight itself is the SP effect) the target dies, the **opposite**
  team's score += 1.0, and both items respawn at random pool spawns [M :96-179]. A dropped item
  respawns at a random pool spawn after 30 s [M :187-228].
- **Industrial Espionage 0x20000100** — `MP_BluePrintUpdate` 0x1882d8 + `MP_BluePrintReachedBase`
  0x188970. One neutral blueprint (state 0 free / 1 carried / 2 dropped); any team carries it. The
  carrier reaching their own team's base scores **team +1.0 and carrier +1.0**, and the blueprint
  respawns at a random registered spawn [M 0x1882d8:50-79, 0x188970:27-55]. Dropped 30 s → resets
  to free where it lies (no re-spawn pick) [M 0x1882d8:147-180].
- **Deathmatch 0x1 / Team Arena 0x20000002** — no objects; kills only (above).

## Deviations (model gaps, marked, not silent rewrites)

- The round clock runs on the fixed 60 Hz tick, not `psiGetTimeIn100ths` (determinism rule).
- `MP_HitBy` (0x1830e0) is the game's spatial overlap test; the port keeps the *rules* (team
  filters, the 4-tick spawn-protection window [M 0x1830e0:36-39], the dead-player rejection) and
  exposes touch events for the caller's collision. The blueprint's base check uses the CTF capture
  radius (sq-dist 2.0) in place of the game's room-list overlap [S].
- The GoldenEye strike flight and the SP effects are not simulated; `ge_strike_complete()` is
  explicit. The KOTH hill volume comes from level data — the caller passes who is on the hill.
- Text/messages/sounds (kill feed, banners, `Text_AddMsg`) are skipped — numbers only.
- `MP_ApplyDamage` 0x18b2c8 decrements `obj+0xF4` only for the hit-class {3,8} sub-objects
  (verified in disassembly); player health is `BLData+0x894` outside this port's scope.
