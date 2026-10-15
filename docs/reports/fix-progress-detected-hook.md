# fix-progress-detected-hook — the detected counter (G-045) and the autoaim distances

Branch `s/fix-progress-detected-hook`. Driver task `110r-fix-progress-detected-hook`, G-045's two
remaining mission-progress pieces. **Both pieces were already implemented on master by
fix-progress-remainder (110d, merged at a958d179)** — its report documents them; G-045's
"Remaining:" clause on master was stale text. Per the driver's instruction ("Check
`s/fix-progress-remainder` on origin first. If it already did either part, skip that part and say
so") I verified 110d's two parts independently at the instruction level, closed the genuine gaps I
found (the 07000005 pin the task names, and a wrong address in the difficulty citations), and left
the working code alone.

## 1. The detected counter's live hook — DONE by 110d; re-verified; 07000005 pin added

**The game's rule, re-verified from the raw bytes** (110d's finding confirmed):
`DroneVision_HaveOpponentSight` 0x175ef0 (called from `DroneVision_ProcessDroneSight`
0x176a28:216-231 on every LOS tick) gates on `DroneFunc_ReactionTime`, sets `+0x228 |= 4`, and
calls `DroneVision_LogSeenOpponent` 0x1780a8. The disassembly (0x1780a8-0x17819c):

- the opponent (+0x170) must be `glb_players` (0x2d8ee0) and the `Drone+0x42` latch clear;
- team byte `Drone+0x44` must not be 2; team 3 passes only on 0x07000009/0x0700000a — the level
  compare is `sltiu` (0x178108, UNSIGNED), so exactly the two tags, as the port's
  `matches!(level, 0x0700_0009 | 0x0700_000a)` implements;
- the 0x07000007 weapon-class gate (obj+0xdc → +0x62) is a different branch, absent on other
  levels;
- then `sb 1, 0x42(...)` (0x178194, the call's delay slot) and `jal 0x199960` with slot 0.
- `PlrStat_LogEnemyDetectedPlayer` 0x199960's disasm: `sltiu slot < 8`, the MP flag at
  **[0x2a4920]** (the decomp's `MPSettings._384_4_` — decimal 384 = +0x180 from the 0x2a47a0
  base bootup memsets), `Mission_Status() ∈ {0,1}` (jal 0x1830d0, `slti <2` + `bgez`), then
  `PlrMissionStats[0x2bcc20 + slot·56] += 1`.

The port (110d, drone.rs): the rule runs at the sight ack (`seen_ticks >= reaction_time`), the
additive `Drone.detected_logged` outboxes the latch, and the fold calls
`Progress::log_detected(ms)` behind the same SP gate (`gated()`: status < 2; no MP mode). Each
guard's first qualifying sight adds exactly one; the latch has no clearer in ACTION.ELF [M
whole-decompile grep].

**What I added — the 07000005 pin the task names** (110d tested 07000006/07000007):
`crates/nf-view/tests/fix_progress_detected_hook_live.rs` — a live team-1 guard spotting Bond
raises `stats[0].detected` exactly once; the counter equals the latched-drone count through 600
more exposed ticks; sister: leave-and-return re-detection by the same guard never recounts, and
only new guards' first sights add (the invariant holds across the leave-and-return).

**Fail-first record**: with the hook locally disabled (`if counted { }` at drone.rs:7207 — a
temporary patch, reverted), the test FAILED at the increment expect
("a guard spotting Bond increments detected [M 0x1780a8:36 → 0x199960]"); restored, it passes
(9.0 s). Targeted chunk green: nf-game lib 168/0, progress_remainder_live +
progress_remainder_failfirst + the new file all ok.

## 2. `Player_InitWeapon`'s per-difficulty autoaim distances — DONE by 110d (not threaded; here's the full evidence)

The task's premise ("the port uses fixed values") is resolved by 110d's finding, which I
re-verified from the raw instructions: **the pair the difficulty arms write is never read by the
game**, so the port must not thread it — that would add behaviour the shipped game does not have.

The arms, decoded from the raw bytes [M 0x1a7ef8, disasm 0x1a7f38-0x1a7fb4], keyed on the
difficulty runtime (see §3 — [0x2a3790]):

| difficulty | [0x30ccf0] | [0x30ccf4] | notes |
|---|---|---|---|
| MP (`[0x2a4920]` != 0) | 10.0 | 16.0 | `lui at,0x4120/0x4180` |
| 2 | 8.0 | 14.0 | `lui at,0x4100/0x4160` |
| SP, anything but 3 | 12.0 | 18.0 | `lui at,0x4140/0x4190` |
| 3 | 0.0 | 0.0 | `sw zero` both |

Two findings beyond 110d's write-up:

1. **The decomp's `MAX_AUTO_AIM_DIST_MIDL/NEAR` labels are crossed against the addresses.** The
   stores put the SMALLER (NEAR) value at 0x30ccf0 and the LARGER (MIDL) at 0x30ccf4 — [L]
   `ee_start_05.bin` with difficulty 1 (the 12/18 arm) reads [0x30ccf0]=12.0, [0x30ccf4]=18.0,
   matching the disasm store order, not the C labels. Documented in
   docs/fields/player-stats.md (new section).
2. **Never-read, re-confirmed**: whole-decompile grep — only `Player_InitWeapon` touches the
   names; the only autoaim consumers (`Check_AutoAim` 0x1a73f8, `Player_AutoAim` 0x1a7c98) touch
   only `Autoaim_Range/Angle_H/Angle_V/Easy/Normal/Hard/LockOnMul` [M decomp]; 110d additionally
   ran the gp-relative and symbolic lui/addiu whole-ELF scans with no other reader. The
   effective per-difficulty autoaim behaviour is `Check_AutoAim` 0x1a73f8:127-141's multiplier
   pick (Normal default, Easy iff d==1, Hard iff d==3/4) — which the port threads at
   `Drones::auto_aim` (drone.rs:6537, the same arms). 110d's real fix (the Q-Claw's
   weapon_data+0x1c cap, 50/100 vs the flat 25) is in and tested (`the_q_claw_auto_aim_reaches_past_the_gun_range`).

## 3. New finding: the difficulty runtime's address is 0x2a3790, not 0x2a37a8

The port's citations (and G-045's text) said the difficulty is `GameState+0x40` (0x2a37a8). The
decomp's `GameState._40_4_` names are **decimal** offsets — the difficulty is GameState+40
decimal = hex +0x28 = **[0x2a3790]**:

- bootup's default-2 store is `sw r8, 40(r17)` at 0x1c7f88 with r17 = GameState 0x2a3768 →
  [0x2a3790] = 2 [M bootup 0x1c7e78:25];
- every gameplay reader `lw`s 0x2a3790: `Player_InitWeapon` (0x1a7f64), `Check_AutoAim`,
  `Player_HandlePain` 0x1902c8, `DroneWeap_DoBulletAccuracy` 0x179740 [M disasm];
- [L] all four EE snapshots: [0x2a3790] = 1 (difficulty 1 active), [0x2a37a8] = 0;
- bootup's separate `GameState._28_4_ = 0x40000034` is decimal 28 = +0x1c → [0x2a3784], a
  different field — no contradiction.

The port's **value** plumbing was always right (mirrors the same field; boot default 2); only the
address comments were off. Fixed the citations: progress.rs (header + `Progress::difficulty`
doc), docs/fields/player-stats.md step 4, and the G-045 row in docs/GAPS.md.

## Files touched

- `crates/nf-view/tests/fix_progress_detected_hook_live.rs` (new — the 07000005 pin + sister).
- `crates/nf-game/src/progress.rs` — comment corrections only (the difficulty address; no
  behaviour change).
- `docs/fields/player-stats.md` — the new `Player_InitWeapon` arms section + the address fix.
- `docs/GAPS.md` — the G-045 row (shared file: the two pieces closed + the address; master's
  G-046 annotations preserved in the merge).
- `docs/WORKLOG.md` — appended entries only.
- **No shared source file was touched** (drone.rs/scene.rs/player.rs/lib.rs all unchanged — the
  only drone.rs edit was the temporary fail-first patch, reverted before commit).

## Verification

- New test + the 110d trio + nf-game lib: green (see §1).
- Full suite on the final tree (merged with origin/master d4b67a10), tools/test/suite.py in
  foreground chunks (this sandbox reaps background jobs): **1320 passed / 7 failed / 42 ignored**
  across all 334 binaries. The 7: the 3 allowed `disc_golden` (bigf, the slim-ISO manifest trio) +
  the documented pre-existing pylon-render trio (2 `fix_0c_pylon_live` + `probe_pylon_render`) +
  `weapon_matrix_live` (killed at my 200 s per-binary limit mid-matrix on 0700000c — it needs the
  suite's default 600 s; unrelated to this task's changes, which touch no weapon code). The other
  two heavy tests that hit my 90/200 s limits (`fix_guard_weapons_live`, `levels_sweep_live`,
  `campaign_flow_live`) PASSED on re-runs with more time. Chunk walls: the first two chunks ran
  with `--limit 60/90` and re-ran the timeouts at 200/260 s.

## What's left (for the next task, not this one)

- Nothing on G-045's two pieces. G-045's remaining clause now lists only the menus' screens
  (180-fix-menus).

## Live checks for the main agent

PCSX2 (retail or the on-foot ISO), any SP level (07000005 ideal — it has no weapon gate):

1. **The detection counter** — `tools/trace/gdbwatch.py 0x2bcc20 4 readwrite --seconds 30` while
   a guard first spots Bond. Expected: exactly one +4 write from `PlrStat_LogEnemyDetectedPlayer`
   0x199960 (caller chain 0x175ef0 → 0x1780a8) per seeing guard, then nothing more from that
   guard (the +0x42 latch); a second guard adds one more.
2. **The difficulty field** — `gdbwatch.py 0x2a3790 4 readwrite`: reads from InitWeapon/
   Check_AutoAim/HandlePain/DoBulletAccuracy; boot writes 2; the menus' difficulty select writes
   the picked value. `gdbwatch.py 0x2a37a8 4 readwrite` should show NO gameplay reads (it is a
   different field) — that would confirm the address correction on hardware.
3. **The MAX pair is write-only** — `gdbwatch.py 0x30ccf0 8 readwrite` across a level load:
   expected writes only at the load (0x1a7f58-0x1a7fb4) and no reads at all.

nf-view (`--bond x,y,z,yaw,pitch`): the detection test is headless-deterministic; a visual spot
is not needed. If a render is wanted: on 07000005, stand Bond 4 units in front of the first
team-1 guard — the pause menu's STATS page row 4 (the undetected row) should show the allowed
detections shrinking by the counter growth (the port's `rows[4]` wiring, progress.rs).

## ntfy failed

All three fallbacks failed with the same server error — ntfy.sh's daily message quota for the
free plan is exhausted (nothing about the sandbox/network):

1. `curl -sS -m 20 -H "Title: NightfirePort" -d "fix-progress-detected-hook: done …" https://ntfy.sh/nightfire-yoda-8k3x`
   → exit 0 but the response body is the error:
   `{"code":42908,"http":429,"error":"limit reached: daily message quota reached; increase your limits with a paid plan, see https://ntfy.sh","link":"https://ntfy.sh/docs/publish/#limitations"}`
2. `python3 -c "… urlopen(Request('https://ntfy.sh/nightfire-yoda-8k3x', data=b'fix-progress-detected-hook: done', headers={'Title':'NightfirePort'}), timeout=20)"`
   → `urllib.error.HTTPError: HTTP Error 429: Too Many Requests`
3. `wget -O- --post-data "fix-progress-detected-hook: done" https://ntfy.sh/nightfire-yoda-8k3x`
   → `HTTP request sent, awaiting response... 429 Too Many Requests` / `ERROR 429: Too Many Requests.`

The queue close (finish.sh) and the branch push both succeeded; only the push notification is
undelivered. Retry it once the daily quota resets: the one-liner is
`curl -H "Title: NightfirePort" -d "fix-progress-detected-hook: done" https://ntfy.sh/nightfire-yoda-8k3x`.
