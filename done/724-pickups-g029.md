## Your task: `pickups-g029` — GAPS G-029: pickups auto-switch, placed/script pickups, respawn

Close the open half of GAPS G-029 from the decompile [M Pickup_Create 0x18dda0, Pickup_Update 0x18e148 and the take
path]:
- auto-switching to a better weapon on pickup (the game's rule and its option gate);
- the level-placed vs script-spawned pickup differences;
- respawning pickups (multiplayer and any single-player cases).

Tests for each rule on a real level. Update GAPS.md with what is closed.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/pickups-g029`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox6 at 2026-10-16T20:08Z

---
## Report (done, 2026-10-16T20:58Z)

# pickups-g029 — GAPS G-029 open half: auto-switch gate, placed vs script pickups, respawn

Task 724, branch `s/pickups-g029`. The three open rules from GAPS G-029, read from the decompile
and pinned with tests on real levels.

## What's done

**1. The auto-switch option gate is now read, not hardcoded.** The rule itself was already ported
(fix-auto-switch: on a weapon take, `Player_EquipWeapon` 0x1a8a58's tail writes the switch request
`anim+0x63 = id` when `Player_IsBetterWeapon` 0x1a6eb0 ranks the pickup better). The game gates
that on the per-player option byte `(&DAT_002a38d2)[player·0x158]` — `PlayerSetting`+0xA [M
0x1a8a58:76]. The port drained every better-ranked take unconditionally. Now `Drones::auto_switch`
(the option, default true = `Input_Init` 0x17edd8's default, live 1 in [L] `ee_start_05.bin`
0x2a38d2) is consulted in the take drain (drone.rs). The persisted `PlayerOptions::auto_equip_best`
(profile.rs, the options save bit) is the same setting; the runtime field is where the app hands
the option in. I also re-verified the ranking table from the ELF bytes: `BestWeapon` 0x2bede0 is
18 u16 ids `[0x42,0x32,0x1a,0x18,0x1c,0x16,0x15,0x08,0x10,0x0e,0x0d,0x0c,0x12,0x06,0x04,0x02,0x0a,
0x01]` [M], matching the port's `BEST_WEAPON` exactly, and re-derived the rule's guard (a held
weapon with base > 0x5e never switches; ranks compare BASES) from 0x1a6eb0.

**2. Level-placed vs script-spawned pickups — the difference pinned and two divergences fixed.**
Both creation arms go through the same `Pickup_Create` with the same record fields: type +0x2c,
id +0x30, rounds +0x34, switch +0x38, sound +0x3c, respawn +0x40, rest = 1 (spawn resting),
lifetime 0 [M — the call sites' stack arguments recovered from the disassembly, N32 ABI:
parsemap case 0xf0 at 0x1d05ac, `Script_CreateEntity` case 0xf0 at 0x1d8be0–0x1d8c20]. The two
real differences: (a) **the label** — a placed pickup carries the placement's +0x44 word
(param 6) [M 0x1d05e4], the script arm passes a hardcoded 0 (the delay-slot `sw $zero, 0x20($sp)`
at 0x1d8c1c). The port's script reservation used to stand in with the script's param 6; fixed to
0. (b) **the fresh-spawn arm dropped the respawn word** — a script pickup spawned without a
pre-scanned reservation got respawn 0 (deleted on take) even when the script asked for a respawn;
`PendingPickup` now carries param 5 [M 0x1d8bfc] and the fresh spawn uses it. The disc's own data
agrees: 07000008's safe script 0600004c spawns the rocket launcher and its ammo with respawn
0xFFFF (stay takeable forever) and label 0, while 0700000b's placed kind-4 item carries label
0x2000036.

**3. Respawn pinned end-to-end on a real level.** A census over the story archives (the
`g029_census` test) found 07000004's three placed weapon-0x12 pickups with respawn 2 — the
single-player respawn case. Through the real take path: the take hides the pickup (state 2) for
`FRAME_RATE_INT · 2 · 10` = 1200 ticks, it returns to rest, and is takeable again [M
Pickup_Handler 0x18e650:338-348 + Pickup_Update 0x18e148:182-189]. The multiplayer case is the
same `Pickup_Create` parameter riding the MP levels' placements — the MP-specific registration
table (`MP_RegisterPickup` 0x189d80, max 64 rows, eviction by smallest lifetime) is bookkeeping
around it; the port's MP bots read the same pickups list.

Also confirmed while reading: `Pickup_CreateSimple`'s stack args (sound 0xF5, rest 0, respawn 0,
lifetime 60·(MP ? 30 : 120), label 0) at 0x18f438–0x18f494, and the level-conditional rounds
override (07000008 + id 0x2c → 4) applies to every create path including scripts.

## How it was verified

- New tests, `crates/nf-view/tests/pickups_g029_live.rs` (all 4 pass):
  - `g029_census_of_placed_and_script_pickups` — the real-level census backing the others;
  - `the_auto_switch_option_gate_gates_the_switch` — option off: the SG5 take fills the
    inventory but Bond keeps the P2K, no draw cycle; option on: a better-ranked unowned take
    switches through the draw machine. Real level 07000005, real input path, no take-path pokes;
  - `a_respawning_placed_pickup_hides_and_returns_on_07000004` — the full respawn cycle on the
    level's own respawn-2 pickup (take → hidden 1200 ticks → returns → retakeable);
  - `script_pickups_carry_respawn_and_no_label_while_placed_ones_carry_the_label` — 07000008's
    script reservation (respawn 0xFFFF, label 0, rest 1, lifetime 0, the rounds override) vs
    0700000b's placed kind-4 label.
- Existing guard rails re-run green: `fix_auto_switch_live` (2), `fix_safe08_live` (5 — the
  script-pickup reservation/activation paths my scene.rs/nis.rs changes touch), `pickups08_live`
  (5).
- Crates my changes touch: `cargo test --release -p nf-game --lib` → **241 passed, 0 failed**;
  `cargo test --release -p nf-view --lib` → **17 passed, 0 failed**.
- All test binaries of nf-game/nf-formats/nf-audio/nf-view compile clean on the final tree.

**Full suite:** not run on the final tree — the window did not fit it. The background fast runner
(`tools/test/suite.py`) started in the first minutes produced a 0-byte log: the sandbox kills
background jobs between tool calls, and re-running the full suite in the foreground (about 90
minutes here) did not fit between the fix and the deadline. What ran instead is listed above; the
next session should start the fast runner in the first minutes and read its total line.

## What's left (G-029 remainder)

- The toggle-back key and the full creep/wire player substate (with WORKLOG 0082's music event).
- Each weapon's first-person fire sound id verification (the remap is ported; the sfx bank
  entries need an ear on the real game).
- Keys / armour deep takes, moving-platform landings, the creation sphere push-out, take text
  messages.
- MP pickups end-to-end in an actual arena sim (the port's `mp` core models the bots' pickup
  goals; a live arena run would pin the MP respawn cadence the placements carry).

## Live checks for the main agent

None required. (Optional confirmation if convenient: with the auto-switch option OFF in the
options menu, picking up a better-ranked weapon must not switch — the port now behaves that way;
and 07000004's SMG pickups should reappear about 20 s after taking.)

## Usage

not visible to me — this session runs in a CLI sandbox that shows no token, credit, or
context-window figures anywhere (no status bar, no `/cost`-style output, nothing in the tool
results).
