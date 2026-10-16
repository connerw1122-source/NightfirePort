# menu-e2e-local-mp — report

Branch `s/menu-e2e-local-mp` (master ad761677 + this task's commits). Everything below was driven
ONLY through `action()` / `tick()` / host-side calls from `FrontEnd::new()` — no `page`/`cursor`/
state assignments (the playtest-1015-2231 rule). Test file: `crates/nf-view/tests/menus_e2e_mp.rs`
(5 tests, all green).

## What's done

1. **Every scenario row (13/13)** — `every_scenario_row_opens_the_map_or_refuses`. The 6 reward
   rows (Uplink 0x39, Demolition 0x3b, Protection 0x3c, GoldenEye Strike 0x3d, Assassination 0x38,
   Team KOTH 0x3a) refuse Confirm, keep the wheel, store nothing, show `mp_row_locked` and the
   locked description 0x010000ab [M C_SBMPSCEN 0x20c600:40-42]. Quick Game lands on the confirm
   page with the fixed config (Deathmatch, 10/10, 3 bots Drake/Kiko/Rook) and its request samples
   row 0 = Skyrail [S — the headless stand-in for `Rand_Random % 7`]. Each open row stores its
   scenario word [M 0x20c9c8], opens the map wheel, and the setup page re-enters on the TEAM prompt
   (state 2) for the team scenarios and the CHARACTER prompt (state 3) for FFA [M P_MPSETUP
   0x20d7a8's team bit].
2. **Every map row (8/8)** — `every_map_row_carries_its_arena_to_the_request`. Each row's Confirm
   stores its level code and the full walk (setup → options → 1 bot → Continue → Confirm) requests
   exactly `MP_MAP_ROWS[row].3` with `mp_match_settings.level_code` == it, scenario 1, bot_count 1,
   human_count 1. **Ravine (row 7)**: the AI Bots row refuses [M P_MPOPTIONS 0x217678:20-25, text
   0x010000ae] and Continue refuses with the game's 0x39e box — the game's own humans+bots > 1
   check [M C_SBMPOPTIONS 0x20e818] — and no request goes out. The port joins only the keyboard
   agent, so a solo Ravine match is impossible exactly as the game makes it (a second pad is the
   game's route; the port has none). Game-faithful, reported not "fixed".
3. **Every options sub-page** — `every_options_subpage_moves_its_values_and_rides_them`. Rules
   steps Duration 10 → 15 → 20 (the list is 1..10, 15, 20, 30, Unlimited [M P_MPRULES 0x20f790])
   and Points by 1; both ride into the settings (`time_limit` = 20×60 = 1200 s [M the ×0x3c],
   `frag_limit` 12). Player Mods: friendly fire On, weapon set 2, the two mod toggles, radar Off —
   weapon_set and radar ride [M +0x436/+0x452] and **friendly fire now rides too** (the fix below).
   Enviro-Mods: all five rows move; the LOCKED Explosive Scenery row's ◀ ▶ are no-ops and the page
   close writes the 0x10 node back [M 0x210424], which the confirm start masks off [M 0x20f708
   andi 1]. Bots: two slots set up (Ninja with edited stats — the stats store back with dirty
   [M P_MPBOTSETUP], Rook), the locked tuxedo Bond (12) refuses in the choose wheel [M
   C_SBMPBTCHOOSE's gate]. Second Confirm while a request is pending is refused everywhere.
4. **Back out of every page at every prompt, forward again** —
   `back_out_of_every_page_and_walk_forward_again_team` (Team Arena × Fort Knox). Setup Back walks
   4 → 3 → 2 and pops to the map wheel at 2 [M C_RBMPSETUP 0x20de58's 0x6b]; the hub's Back
   re-enters the setup on the TEAM prompt (the playtest's stale-prompt bug) [M P_MPSETUP
   0x20d7a8]; every sub-page closes by Confirm AND by Back; the bots chain's Backs walk one page
   each; the confirm page's Back returns to the hub; the scenario's Back returns to Main on the
   Multiplayer row. The match still reaches its request afterwards (Fort Knox, team_game 1, the
   human's team pick riding).
5. **The debriefing** — `the_debriefing_returns_to_the_scenario_page`. New host-side
   `FrontEnd::mp_debrief_enter()` implements the game's quit handoff [M C_GCPAUSE 0x204408:146 —
   quitting a match with MPSettings+0x180 != 0 sets GameState+0x28 = 0x40000033 and reloads
   0x0700048]; its Confirm returns to the scenario wheel on row 0 [M C_MPDBG 0x217860's 0x4b arm].
   The rematch arm (msg 0x5d) stays unported as before (no in-match host loop — reported).

## Fix (with game evidence)

**`mp_confirm_start` dropped the friendly-fire choice.** The game's Player Mods page writes
MPSettings+0x198 in place and `MP_RegisterBulletHit` 0x1857c0:43-57 reads it to block same-team
hits; the port rebuilds the settings struct at confirm, so the field must be mapped explicitly.
`mp_confirm_start` now carries `s.friendly_fire = self.mp_friendly_fire` (crates/nf-view/src/menu.rs,
cited at the write). Verified by the new test: toggle On in Player Mods → settings.friendly_fire == 1.

## What ran (crates touched: nf-view only; no other crate changed)

- `menus_e2e_mp` (new): 5 passed.
- `menus_e2e` 5, `mp_menu_flow` 9, `mp_net_lobby` 9, `menu_music_flow` 14, `menu_sfx_flow` 14 —
  all passed on the final tree (master ad761677 + the two task commits).
- The fast suite (`tools/test/suite.py`) on the final tree: **build ok: 394 test binaries** (25 s
  with the warm cache); the run phase could not finish inside this sandbox — background runs are
  reaped between tool calls here (nohup and setsid both die), and the run's FIRST binary is
  `fix_striped_box_census`, a multi-minute whole-game census that blocks the per-binary log write
  for its entire run, so a foreground tool-call window (9.5 min cap) ends before anything is
  written. With `--limit 20` the runner is proven healthy: it advanced past the census with the
  documented `TIMEOUT fix_striped_box_census after 20 s` line. **No suite total line.** The
  nf-view binaries the task touches all pass when run directly (listed above); one uninterrupted
  suite run on this tree is the remaining verification (the main agent's runner, or a sandbox
  that keeps background jobs).

## Remaining / for the next round

- The settings the MP core does not consume yet (no struct fields — phase gaps, not menu bugs):
  the Player-Mods words +0x1BC/+0x1C8, the Enviro-Mods rows (+0x1B8/+0x1C0/+0x1D0/+0x1CC/+0x1D4
  beyond the mask), the per-slot Health Handicap (+0x2C) and the per-bot records (name/stats) ride
  only as far as the FE's own fields. Adding them needs the MP core's readers first.
- Wire the real pause-quit → debriefing handoff in the host (lib.rs) once the in-match pause loop
  exists; `mp_debrief_enter` is the entry the game's handoff lands on.
- The hub's list row persists across sub-page closes (the game's lists keep selection too) —
  the tests navigate explicitly; no action taken.

## Live checks for the main agent

- L-1: P_MPSETUP 0x20d7a8 — enter the setup page of a TEAM scenario (e.g. Team Arena), back out of
  the options hub, and confirm the prompt label is 0x37c "Choose Team" (not 0x37d) on re-entry.
  Watch the label bind on the page-enter msg-0x4c.
- L-2: P_MPCONFIRM 0x20ec00 — during a real match start, verify the second ✕ does nothing (the
  manager delete) and that GameState+0x28 reads 0x40000033 after quitting the match from the pause
  menu (the debriefing handoff this task's `mp_debrief_enter` stands in for).

## ntfy

Sent 2026-10-16 ~08:58 UTC via curl (first fallback, id Bn32IyMWzAet).
