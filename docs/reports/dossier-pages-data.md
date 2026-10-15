# dossier-pages-data — the Dossier's Rewards / Gadgets / Weapons pages

Branch `s/dossier-pages-data` (cut from origin/master 0157588d, finished merged with origin/master
31f61752). Worker sandbox 2026-10-15 18:57–20:57 UTC.

## What's done

The Dossier wheel's three unported pages now have their data, state, flow and getters (no drawing —
the main agent draws each page and compares against PCSX2):

- `crates/nf-view/src/menu.rs`: `Page::Rewards` / `Page::Gadgets` / `Page::Weapons` (+ ids
  0x4000003b/3c/3d [M C_SBDOSSIER 0x216eb0:12-31; D the menu-def PAGE records]); the Dossier wheel's
  Confirm rows 1–3 open them (cursor reset — every page's init msg 0x2e (0,0)); Up/Down clamp without
  wrap [M Scroll_Update 0x2221b20:186-210 — the +0x144 wrap flag is set only by msg 0x69, which no
  menu-def control carries]; Back/□ return to the Dossier; Confirm does nothing on the pages (no
  handler arm [M C_RBDSREWARDS 0x20c248, C_SBDSGTSCROLL 0x20bcc0, C_SBDSWPSCROLL 0x20bab0]).
- The game's tables as cited statics: `WEAPONS_TABLE` (27 rows, ds_weapons 0x2dfec8), `GADGETS_TABLE`
  (14 rows, ds_gadgets 0x2e0150), `REWARD_BONUS` (12 × 4 (caption, stamp) pairs, RewardsTable
  0x2bc920 +0x1c..+0x28 / +0x30..+0x3c) — all [L ee_start_05.bin] and disc-verified.
- Getters the draw code calls: `FrontEnd::reward_rows(row)` (the four (shown, caption, stamp) rows
  with the cumulative medal gates [M C_RBDSREWARDS 0x20c248:21-74]), `FrontEnd::gadget_row(row)` /
  `weapon_row(row)` / `wheel_row` (preview +0x00, wheel label +0x04, description +0x08, upgrade
  caption, unlocked [M Menu_UpdateWheel 0x1f8388]), `FrontEnd::weapon_overview_row()` (P_DSWEAPONS
  0x217038's watch-level rewrite), `gadget_suffix`/`weapon_suffix` (the upgrade caption maps,
  composed as `"%s\n%s"` [D the format string 0x30d358]).
- `Progress::object_upgrade_level(object, slot)` — Menu_GetObjectUpgradeLevel 0x2025f8's walk
  (kind-4 group, set-bit count), the basis of the Gadgets/Weapons upgrade captions.
- Minimal draw arms for the three pages + `--page rewards|gadgets|weapons` in `lib.rs` (additive
  hooks, the Records precedent) so the main agent can render them headless.
- Docs first: `docs/systems/dossier-pages-data.md` — every address, table, page-data control and
  message id with evidence tags, including the full menu-def parse of the four pages (controls,
  positions, static labels/textures) and the RewardsTable/labels/textures table.

### Found and fixed while working: the gold-medal group bits

`Progress::has_medal` / `process_reward_counter` grouped kind-4 descriptors by
`REWARDS_TABLE[i].desc[0]`'s packed {bit,kind} word — but desc slot 0 is never read by the game and
its word is 0 in all 12 entries, so every level collapsed into one 12-member group with rank = table
index (gold bits 0..11 by level). The game scans descriptors **1..4** of every entry for the target
descriptor's own packed word [M `PlrStarts_ProcessRewardCounter` 0x199210:39-62, disassembly-checked]:
the real groups are {entries 1,5,7} → widened bits 0,1,2 (the WORKLOG test comment "the gold group of
CastleExterior/TowerA/Tower2A spreads bits 0..2" already described this — the code didn't do it) and
{entries 4,11} → bits 9,10; every other kind-4 desc is a singleton at its own bit. Fixed `gold_group`
to take the counter and scan desc[1..=4]; non-gold medal bits unchanged (the EE evidence bit 26 for
0x07000005's bronze and bit 42 = 0x0900006's silver still decode identically). The upgrade getter and
the Rewards page's gold gate depend on this.

## How it was verified

- `crates/nf-view/tests/dossier_pages.rs` (new, 7 tests): the wheel rows open the pages; the scroll
  clamps with each page's own range (12/14/27, the handlers' msg 0x27 (0, 0xd/0x1a)); the Rewards
  rows follow the medal flags with the cumulative gates (bronze → row 1 shows 0x2d2/0x03000134,
  silver+ hidden); the upgrade level counts the kind-4 group (watch 0→3, singletons, the watch pair
  → weapon_suffix level 2); **the port's statics are byte-compared against the disc's ACTION.ELF
  .data at 0x2dfec8/0x2e0150/0x2bc920 (ISO-gated)**; the pages' labels bind from USATxt.dat
  (headers 0x281/0x282/0x19c, hints 0x49e/0x49f, every table label/desc and the caption labels
  0x3b1..0x3b8, ISO-gated). Every ISO-gated test has a headless sister that fails on its own.
- `docs/tools/test-suite.md` suite: the background `suite.py` is reaped by this sandbox (empty
  suite.out twice — the known issue from the previous tasks), so the run was foreground chunks on
  the final merged tree:
  - nf-game lib **197/197**, nf-view lib **9/9**, nf-formats lib **76/76**, nf-audio lib **9/9**;
  - menu integration: menus_flow **36/36**, dossier_pages **7/7**, mp_menu_flow **9/9**,
    dev_menu_flow **5/5**;
  - the medal-bit consumers: progress_live **1/1**, fix_progress_live **5/5**,
    fix_scores_handoff_live **3/3**, fix_save_profile_live **4/4**, campaign_flow_live **1/1**;
  - nf-audio integration: bank_live 1/1, fix_sound_remainder_live 2/2, music_live 1/1;
  - nf-formats integration: disc_golden 11 passed / 3 failed — exactly the 3 allowed slim-ISO
    disc_golden set (bigf_header_fields / bigf_tables / directory_tree vs the full-image golden).
  Zero failures beyond that documented set. The remaining ~180 integration/live binaries were not reached in the
  window (the suite's 13-min build + run does not fit; the diff touches only nf-game/progress.rs's
  gold-group path + nf-view's menu code, all of whose consumers are in the chunks above).

## What's left

- The main agent's draw pass: the three pages' layout against PCSX2 (`--page rewards|gadgets|
  weapons`, plus `--cursor N` for each wheel row). The port's arms use the pages' own geometry
  (parsed 2026-10-15, cited in the doc); the stamp UVs and the Rewards spinner arrows need eyes.
- The Rewards page's list selection could follow GameState+0x72 (the game binds it on open
  [M 0x2172c8:15]) — the port keeps the mission cursor instead (same choice the Records page made).
- Records' own Up/Down still wraps (the pre-existing `%` at menu.rs's Records arm) — the same
  Scroll_Update rule says clamp; left untouched (out of this task's scope, no test pins it).
- Live checks for the main agent (PCSX2): see below.

## Live checks for the main agent

1. Rewards rows vs medals — address `bonus` bitmask 0x2e1b88 (u64), access R; play 07000005 to a
   bronze; expected: bit 26 set, the Rewards page (Dossier → Rewards, level The Exchange) shows only
   the first stamp (0x03000134) + caption 0x2d2 at (76,130)/(137,147).
2. The gold group — after a gold on 07000005 (bit 0) + golds on 0700009/0700011 (bits 1/2), the
   Weapons page's row 0 shows 0x03000150/0x3c7/0x3c8 (the level-3 overview [M 0x217038:29-33]).
3. Gadget upgrade captions — poke bit 18 of bonus[0] (0x2e1b88) to 1; the Gadgets page's row 9
   (0x0900003's laser, object 0x12) should show the 0x3b7 caption under its description.
4. The scroll clamp — on the Weapons wheel, hold Up at row 0 / Down at row 26: no wrap, sound only
   [M 0x2221b20:186-210].
5. ✕ on the pages does nothing (no handler arm); ○/△ return to the Dossier.

## ntfy failed

All three fallbacks hit the daily quota (429):

- `curl -sS -m 20 -H "Title: NightfirePort" -d … https://ntfy.sh/nightfire-yoda-8k3x` →
  `{"code":42908,"http":429,"error":"limit reached: daily message quota reached; increase your limits with a paid plan, see https://ntfy.sh"}`
- `python3 urllib.request` → `urllib.error.HTTPError: HTTP Error 429: Too Many Requests`
- `wget -q -O- --post-data …` → exit 8 (server error), no output

## Shared files touched (for the merge)

- `crates/nf-game/src/progress.rs` — `gold_group(entry_idx, counter)` reworked (same callers), new
  `gold_group_bits`, new `Progress::object_upgrade_level`. No struct/constant changes; the 188→197
  lib tests pass.
- `crates/nf-view/src/menu.rs` — additive: 3 page ids, 3 Page variants, 3 rows()/row_label() arms,
  3 tables + 2 suffix fns, 5 getters, 3 action arms, 3 draw arms. Merge with origin/master resolved
  in the Page enum (mp-menu-flow's Mp* variants kept alongside) and WORKLOG (append-only).
- `crates/nf-view/src/lib.rs` — 3 `--page` names.
- `docs/WORKLOG.md` — append-only entries.
