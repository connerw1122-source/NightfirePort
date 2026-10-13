## Your task: `fix-script-start-tick`, a script's first commands run on the tick it starts

**The user's bug** (playtest 2026-10-12, check `gate05`): on 07000005, stepping on the wall-hug plate makes the two
gate guards vanish: "why the hell would they disappear? that doesnt seem right at all."

**The main agent's measurements:**
- **The real game** (PCSX2, GDB server and save-state snapshots; `docs/fields/hitdata-spawner-ally.md` §Live
  result): plate #301 → ch 0x43 → SP #298 plays script 0x06000AD4, whose event 5 calls `Drone_EnableAll(0)`.
  **During the whole scene the two spawned gate guards are still present** (15 drones; the gate spawner's latch
  +0x20 still 0, because `DroneSpawner_Control` 0x137e68 returns at its first line while `NPCGlobals` == 0). When
  `Script_Free` re-enables the NPCs, the latch fires and the gate pair is removed, and at the same moment the two
  placed guards behind them (drones at (27.6, 37.2, 49.9) and (32.3, 37.2, 50.1), object flags 0x8b0 → 0x880)
  switch on. The player never sees an empty gate.
- **The port** (a probe on master after fix-spawner-p7): the gate pair is removed **on the tick of the touch**,
  before the freeze lands, and the placed pair stays disabled until the scene ends. So the gate is **empty for the
  whole ~1,400-tick scene**.

**The cause** (documented as a deviation in `crates/nf-view/tests/spawner_kill_live.rs` lines 78-84): the port's
script player runs a newly started script's first commands **one tick after `start`**. In the game, SP #298
(placement 298) runs its first commands, including event 5, in the same frame and **before** the spawner (placement
268) in the control walk, so the freeze holds the kill latch off for the whole scene.

**Fix:** a script started this tick runs its first commands in the same tick, in the game's control-walk order (the
placement order of the script player relative to the other objects). Find the game's exact rule in
`SP_Update`/`Script_Load`/`Script_Update` (0x1dbc08 and neighbours) and port it. Then re-check every script-driven
sequence the change could shift (the cut-scene starts, `fix-mission`'s channels, the 07000008 ride, 07000001's
intro), and update `spawner_kill_live.rs` to assert the game's visible order: during the scene the gate pair is
present and the placed pair is hidden; after it, the reverse.

Tests (NF_ISO, fail-first): the 07000005 sequence above, asserting at the touch tick, mid-scene and after; a sister
for a script started on a tick whose first command is a wait.

Files: `crates/nf-game/src/nis.rs` and the script player, `crates/nf-view/tests/spawner_kill_live.rs`.

**Shared files** (`drone.rs`, `scene.rs`, `lib.rs`): small, additive hooks only, listed in your report. Pull `master`
before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** The user's play is the reference, and previous fixes passed tests but still failed in play. For each bug:
1. Write a headless NF_ISO test that drives the real input path (the `nf-view` input mapping into `Sim`, or `nf_view::load` plus `dev::scenario`), and show it reproducing the user's symptom.
2. Then find the cause in the decompile and fix it.
If you can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Update the doc entry** to say it's fixed, with your commit.

**Branch:** `s/fix-script-start-tick`.

---
claimed-by: sandbox1 at 2026-10-13T02:39Z
