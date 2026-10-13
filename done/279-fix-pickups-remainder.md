## Your task: `pickups-remainder`, pickups: auto-switch, level and script pickups, keys and armour

GAPS G-029 remainder: the auto-switch on pickup (`Player_IsBetterWeapon`, `0x2A38D2`), the toggle-back key, level-placed and
script pickups (`parsemap_create_dynamic_objects` 0x1d04f0, `Script_CreateEntity` 0x1d8968), `Pickup_CreateFromSet` (types
7–11), keys / armour / types 5–6, respawn (state 2), spin, moving-platform landings, the creation push-out, text messages,
`Upgrade_Weapon`'s upgrade bytes. The user: "still can't pick up the rocket launcher" (07000008) and "need to prioritize
getting every weapon and gadget to work".

Start with a census test: for each of the 27 on-foot levels, every pickup the game creates (placement, script, set) is
created by the port with the right type/weapon/ammo, and is takeable from where a player stands. Then port what fails.

Files: `crates/nf-game/src/pickup.rs`, `crates/nf-game/src/player.rs` (the switch), docs/systems/pickups.md.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing** (start it early, in the background, so it finishes inside your session):
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail. Report the count. If your change adds `Rand_*` draws, seed-pinned
  tests may move: re-pick them by the WORKLOG 0072 procedure (`NF_TEST_SEED` search against the same live expectations).
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/pickups-remainder`.

---
claimed-by: sandbox5 at 2026-10-13T19:19Z

---
## Report (done, 2026-10-13T20:48Z)

