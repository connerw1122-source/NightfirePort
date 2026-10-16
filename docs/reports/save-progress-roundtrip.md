# save-progress-roundtrip — report

Task 707, branch `s/save-progress-roundtrip` (from origin/master `3bb7be9a`, merged `e05a8370`).

## What the task found

**The Explosive-Scenery MP gate was dead after a restart.** The port's `FrontEnd::mp_bonus_mask`
(the `mp_stuff[0x370]` byte [M Menu_UnlockMPSettings 0x1ff698:53-104]) booted 0 and was never
recomputed anywhere — only tests set it by hand. The game re-derives it from the profile's bonus
words on every MP-scenario page open [M P_MPSCENARIO_Handler 0x217348:23 → Menu_UnlockMPSettings
0x1ff698 — decomp `work/decomp/ACTION.ELF/001ff698`: the OR of the four player bonus words
`bonus | DAT_002e1b90 | DAT_002e1b98 | DAT_002e1ba0` @0x2e1b88..0x2e1ba8, the flags cleared, then
the 12 campaign rows × counters 1..4 read through `PlrStarts_ProcessRewardCounter` 0x199210's get
arm; descriptor kind 2 ids 0x38..0x3d raise the scenario/bot unlock flags, kind 3 id 0x3e raises
mp_stuff[0x370]]. On the shipped RewardsTable the only kind-3 descriptor is row 10's 0x07000014
desc[2] = bit 62 (0x3e), kind 3 [D 0x2bc920 row 10]. So a saved profile that had earned Countdown's
reward showed the gate LOCKED on every fresh session — the roundtrip assert failed before the fix.

## The fix (crates/nf-view/src/menu.rs)

- `FrontEnd::unlock_mp_settings()` — the 0x1ff698 walk over the active profile's bonus words
  (slots 0..3, the game's four words); kinds 0/1/4 set nothing, so the mask keeps exactly the
  kind-2/3 ids found set. Without a codename in play the mask clears [S — the game's in-memory
  globals would carry a Play-without-saving session's awards; the port's front end has no session
  layer there].
- `enter_page(Page::MpScenario)` calls it — the same trigger the game uses (the page's 0x4c open).

## What's done (the roundtrip, by the real path)

`crates/nf-view/tests/save_progress_roundtrip.rs`:

- **The roundtrip** — create the codename (`RNDTRIP1`, 8 chars — the game's name buffer
  0x2e1ae8) through the keyboard flow; complete **two missions** (0x07000005 row 1, 0x07000001
  row 4) through the results commit + the save prompt **Yes** card flow (ASK_OVERWRITE 0x10ee38
  → the saving message → one write per mission [M P_NFRESULTS 0x208e78:24-52]); **drop the
  FrontEnd entirely**; a fresh FrontEnd boots, walks Main → Codename → Down Down → Confirm and
  lands Difficulty. Asserted, per row 0..11: the same unlock word `0b110111` (the driving rows
  2/3 stand open through the port's pass-through — they keep NO game bits; the chain stops
  behind the on-foot row 5), the same records/medals/reward rows, and — after walking back out
  into the MP scenario page — the **Explosive-Scenery gate open** (mask bit 0x3e from the loaded
  BNUS bonus word), with the recomputed mask equal to the pre-restart walk.
- **Load-bearing check**: with the `enter_page` hook disabled the gate assert fails ("the
  Explosive-Scenery gate stands open after the restart") — the test reproduces the symptom, the
  fix clears it.
- **"Play without saving" leaves no file** — two completions with no codename in play: no save
  prompt [M 0x208e78:33-36], the card dir stays empty, a fresh program boots at 0x3.
- **A corrupted file reports Load failed and nothing loads** — a flipped byte: the card still
  lists the row, the load surfaces msg 7 / Txt 0x273 "Load failed!" Retry+Cancel [M LOADERROR
  0x10f138], no active profile, the boot word stands; fixing the file behind the dialog → Retry
  loads to Difficulty.

## How it was verified

- The new test binary (2 tests) green on the final tree.
- The touched crate's menu-adjacent binaries, run directly on the merged tree, all green:
  mp_defaults_setup 10p, menus_e2e_mp 5p, memcard_save_flow 7p, fix_save_profile_live 4p,
  mission_unlock_flow 4p, menus_flow 42p, menu_sp_e2e 9p, dossier_pages 7p, codename_entry 14p,
  save_progress_roundtrip 2p, menus_e2e 5p, menus_mouse_parity 17p.
- The full suite (tools/test/suite.py) on the final tree (the branch merged with origin/master):
  see the total below.

**Suite total** (`NF_ISO=… python3 tools/test/suite.py`, 2026-10-16 18:21 UTC, on the branch
merged with origin/master `e05a8370`):

```
1896 passed, 6 failed, 72 ignored  (46 min)
```

FAILED list — all six are the known baseline, none from this task:

- `disc_golden::bigf_header_fields_behave_as_documented`, `disc_golden::bigf_tables_match_the_golden_manifest`,
  `disc_golden::directory_tree_matches_the_golden_manifest` — the 3 disc_golden failures the slim ISO allows.
- `fix_0c_pylon_live::the_0c_pylon_fills_the_view_from_the_closeup_spot`,
  `fix_0c_pylon_live::the_0c_pylon_is_on_screen_at_the_user_spot`,
  `probe_pylon_render::probe_render_pylon_only` — the GPU render probes; this sandbox has no GPU
  (`adapter: NotFound`), the same environment failures recorded before this task.

No TIMEOUT, no CRASH.

## What's left

- `mp_scenario_locked` / `mp_character_locked` still use the [S] stopgap (reward rows locked
  unless the unlock-all cheat) — the same 0x1ff698 walk computes those flags as mask bits
  0x38..0x3d; wiring the rows to the mask is a separate task (it touches the scenario/bot pages'
  routing).
- The live render of the Enviro-Mods page's row 2 (Locked ↔ On/Off label swap) is data-driven
  and already covered headless (`mp_enviro_value_label`); visual work needs the main agent's
  render as usual.
- Live checks for the main agent: none required — everything here is headless-menu logic. If
  wanted: on a profile with Countdown's reward earned, open MP → Enviro-Mods in PCSX2 and
  confirm row 2's list offers On/Off (not the single "Locked" node).
