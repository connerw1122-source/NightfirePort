# fix-level-chain — how the no-exit levels move on, ported and pinned

Task 104b. Blockers 2-4 of docs/reports/campaign-flow.md: 07000006 / 07000011 / 0700004a have no
0xE8 exit trigger in any map, and 09/0a's LOAD_LEVEL triggers "point at their own level". They don't:
**the old blocker table was built on a stale parse.** Every one of these levels chains through a real,
data-backed mechanism — found this session with disc probes over the current parser — and the port now
runs each of them headless. Suite total at the bottom.

## The mechanisms (all four, evidence first)

**07000006 → 07000007 — the party cut-scene's script event 0x12.** 06 has no 0xE8 (confirmed again,
all 42+ maps [D probe, `probe_level_chain.rs`]) and the mission AND cannot fold mid-mission (it spans
every part's condition [M `Mission_MonitorObjectives` 0x1827d8:22-38]). The chain: main-map 0xEB TOUCH
placement whose out channel is **ch 0x83** gates ScriptPlayer #220 (script **0x06000071**, trigger
channel 0x83, mode 0 — waits, plays on the channel [M docs/systems/script-player.md]); the script's
**stream 9 fires event 0x12 with param 0x07000007** [D probe], and `Script_EventHandler` case 0x12 is
`Player_RamSave` + `ResetMap_LevelToLoad(param, 1, 0)` [M 0x1d94c0:194-199]. The port already had the
whole chain (`Request::LoadLevel` → `Drones::level_exit`); nothing in it was broken — the campaign
runner just never walked Bond into the party zone, and its `no_exit_trigger` fallback force-completed
the mission instead.

**07000009 → 0700000a — the lift touch gates a real 0xE8.** The 0xE8 sits in the main map (map39
placement #60) with **next_level = 0x0700000a and gate ch 0x32** [D probe] — the "self-loop" reading
was the old parse. The gate's writer is the adjacent TouchOnce (out = 0x32) [D probe]. The exit
auto-fires on the gate [M `Trigger_Update` case 10, 0x1b1a78:162-166]. The runner never reached it
because of its own FAIL-flag bug (below): it raised the alarm itself (ch 0x61) and the fail-restart
(`level_exit = 0x07000009`) masked the real exit.

**0700000a → 0700000b — the balcony door, unlocked by an AND mux over the objectives.** The 0xE8
(map37 #2) has next = 0x0700000b, key gate ch 0x4a, no auto gate [D probe] — again not a self-loop.
ch 0x4a is **not** a key pickup's channel: it is main-map 0xEC AND mux #568's output with inputs
{0x48, 0x49} = 0a's first two objectives (Q-Worm install, security centre) [D probe]. The mux
recomputes its output every tick, so any one-shot write to 0x4a is reclaimed — the test has to hold
the inputs, the way the live writers do (the Q-Worm script player's out 72 [D levels.md §5.2] and the
security-centre zone; their full drive is per-mission walkthrough material, campaign-flow blocker 6).

**0700004a → 07000013 — the standalone movie code.** 4a has no 0xE8 and no 0x12 anywhere in its own
archive [D probe]. The chain lives in the **loadable pack 0790004a**: ScriptPlayer #276 (trigger ch
0x84, written by main-map TouchOnce #62) plays pack script **0x060006d3**, whose **stream 9 fires
event 0x12 with param 0x0720004a** [D probe] — a MOVIE code, not a level hashcode. The load pipeline's
phase 2 resolves it: `GetLevelToLoadFromMovieCode` (0x1d3560) maps 0x0720004a → 0x07100013 → (second
cycle's fallback mask, 0x1d3560:97-98) → **0x07000013** [M 0x1d6360:173-182]. **This was the real port
gap**: the port handed raw exit codes to the archive loader, and `0720004a.bin` does not exist — the
view would have logged a load failure and stuck.

**07000011 → 07000012 — the exits exist; the gate writer is gameplay.** Two 0xE8s in the main map
(#118 next = 0x07000012 key gate ch 0x1e; #124 next = 0x07000012 gate ch 0x1f) [D probe]. No
placement param, script event, `Trigger_Calc` latch (0x1b1900 has only 05's 0xa4→0x6f and 0b's
0x34→0x2d [M]) or script entity carries 0x1f/0x1e anywhere in the archive — the writer is runtime
gameplay (lift/door/switch state) the headless harness cannot drive. The auto-fire and keyed-E arms
themselves are pinned by test.

**07000011's 12 → 4a and 13 legs already worked** (12's 0xE8 gate ch 0x1f; 13's 0xE8 next =
0x07300013, the DRIVING.ELF interlude — the port logs and skips it, and the game's own chain from a
0x0730xxxx code ends at the front end 0x07000048 [M 0x1d3560:24-50]).

## The port changes

1. **`crates/nf-game/src/triggers.rs`** — `movie_code_next_level(code, fallback)` is the decomp's
   full binary-search tree of 0x1d3560 (all 15 story pairs 0x0720xxxx → the next 0x0710xxxx, every
   driving code 0x0730xxxx → 0x07000048, everything else → the masked fallback), and
   `resolve_exit_code(code)` compresses `ResetMap_LevelToLoad` (0x1d3800) + `ResetMap_Load`'s
   phase-0/phase-2 cycle (0x1d6360) to the archive the port loads: plain 0x0700xxxx passes through,
   0x0710xxxx masks, 0x0720xxxx maps then masks, 0x0730xxxx maps to its front-end landing. The
   prefix test is bits 20-23 (0x07**2**0xxxx), the nibble that actually distinguishes the four code
   families. Unit tests pin every table arm plus the fallbacks.
2. **`crates/nf-view/src/lib.rs`** (the level-exit consumption) — `level_exit` codes now go through
   `resolve_exit_code` before `level_archive_name`, so 4a's movie code lands on 13 and a driving exit
   lands on the front end instead of a nonexistent `0720004a.bin`/`07300013.bin`. Additive, one call
   site; the log line notes the resolve.
3. **`crates/nf-view/tests/campaign_flow_live.rs`** (the shared harness, two real bugs):
   - the objective skip tested **bit 0 (`flags & 1`)** for "channel ON = FAIL"; the FAIL flag is
     record+0x10 **bit 1 (0x02)** [M 0x1827d8:32-38, the port's own `Objective` doc]. The runner was
     raising 09's alarm objective itself and then recording the fail-restart as the level's exit.
   - the E-path aimed Bond at `(lo+hi)/2`: a trigger with no mesh produced NaN, and a door-slab
     trigger pushed Bond out beyond the reach of a yaw-0 use sphere. Now: unbounded boxes are
     skipped with a note, and the sweep tries the four facings [M 0x1a5170:100-137 — the use sphere
     is radius 1.0 at head + camera dir, so the facing matters] while holding the key channel each
     tick (the 0a mux reclaims a one-shot write).
4. **`crates/nf-view/tests/fix_level_chain_live.rs`** (new) — the five NF_ISO chain tests, each
   through the real input path (real load, real trigger volumes, the game's own channels):
   06 party touch → `Some(0x07000007)` (+ sister: the mission must NOT complete mid-mission);
   09 lift touch → `Some(0x0700000a)` (+ sister: an idle level never exits; the 0xE8's gate/key
   bytes pinned); 0a objectives-AND unlock → E-path `Some(0x0700000b)` (+ sister: locked without
   the unlock); 11 gate on → auto-fire `Some(0x07000012)` (+ sister: nothing fires before the gate;
   both exits' bytes pinned); 4a lobby touch → `Some(0x0720004a)` and `resolve_exit_code` →
   0x07000013. Plus `probe_level_chain.rs` (kept probe: placement histograms, script events, pack
   events, main-map trigger graphs, the E8 map locations).
5. Shared files (`drone.rs`, `scene.rs`, `player.rs`): **untouched**. `lib.rs` is listed because the
   exit consumption lives there (one resolve call); no other shared-file edits.

## The campaign table after (the rows this task covers)

| mission | level | before | after |
|---|---|---|---|
| 07000005 | 07000006 | forced complete → 0x07000048 | unchanged in the runner (the real chain is the party touch — `fix_level_chain_live` drives it → 0x07000007); the runner still force-completes because it cannot walk to the zone |
| 07000009 | 07000009 | 2/3, "self-loop" → 0x07000009 | **3/3, exit → 0x0700000a** |
| 07000009 | 0700000a | 2/3, "self-loop" | **3/3, exit → 0x0700000b** |
| 07000009 | 0700000b | 2/5 forced | 5/5, mission end (driving, skipped) |
| 07000011 | 07000011 | forced → 0x07000048 | unchanged (the gate writer is gameplay — live check below) |
| 07000011 | 0700004a | forced → 0x07000048 | unchanged in the runner; `fix_level_chain_live` drives the real chain → 0x0720004a → 0x07000013 |

Mission 07000009's three levels now chain end-to-end headless through real exits.

## Reproduce-before-fix notes

The fail-first demonstration for the **port** change (the resolve): on the pre-fix tree the 4a test
cannot pass — `resolve_exit_code` does not exist (the nf-game test file does not compile), and the
lib.rs call site is absent, so `level_exit = Some(0x0720004a)` flows to `level_archive_name` and the
archive is missing on the disc. The chain tests themselves are new pins: 06/09/11 chains were already
live on master (the research found no break in them), so their tests pin behaviour rather than fix it;
the **runner** fixes are demonstrable from the before/after table above (09: "self-loop restart" →
the real 0x0700000a; the old `& 1` skip is visible in the before rows' `unfired objective channels
[71]` + exit 0x07000009).

## Live checks for the main agent

PCSX2 (spot / action / expected), via `tools/trace/gdbwatch.py` on `switch_channels` (0x2b8ee4-ish
base [D campaign-flow]):

1. **07000011, ch 0x1f and ch 0x1e writers** (the one open mechanism): watch writes to
   `switch_channels[0x1f]` (31) and `[0x1e]` (30) during a real Tower-2 playthrough, access type W,
   expected: one of them rises shortly before the level ends (the 0xE8s' gate/key — placements #124/
   #118, both next = 0x07000012). The writer is not any placement param, script event (0xd/0x11),
   `Trigger_Calc` latch or script entity — likely a door/lift state machine. If it is a door, the
   port's door→channel path may need the same one-line latch.
2. **07000006 party chain**: walk into the party-entrance zone (main-map 0xEB #189, out ch 0x83) —
   expect the cut-scene script 0x06000071 to play and the level to load 07000007. Watch
   `switch_channels[0x83]` W then the script event log (`NF_SCRIPTLOG=1`).
3. **0700004a**: complete the lobby objective — expect ch 0x84 → the pack cut-scene 0x060006d3 →
   `ResetMap_LevelToLoad(0x0720004a)` → the cut-scene movie → 07000013.

nf-view (`--bond` spots, `--skip-cutscenes`): the five chain tests are the checks — re-run
`cargo test --offline --release -p nf-view --test fix_level_chain_live -- --nocapture` and
`... --test campaign_flow_live ...` and compare the tables above.

## What's left

- 06/4a/11/12's campaign-runner rows still force-complete where the real exit needs a walkthrough
  (the party zone, the lobby zone, 11's gate writer, 12's ch 0x4d objective) — the mechanisms are
  ported and pinned per level; driving them in the runner is walkthrough work (campaign-flow
  blockers 1/5/6 territory).
- The view could play the 0x0720xxxx cut-scene movie instead of skipping it (the movie machinery
  exists for the front-end flow; the skip is the game's own X-skip semantic).
- 0a's `0x48/0x49` live writers (the Q-Worm SP drive + the security-centre zone) belong to the
  09-0d walkthrough task.

## Suite total

(filled from suite_lc.out below)
