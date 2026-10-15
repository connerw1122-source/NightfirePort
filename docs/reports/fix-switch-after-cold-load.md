# fix-switch-after-cold-load — a weapon switch is ignored after a cold load

Branch `s/fix-switch-after-cold-load`, task 110n. Tags: [M] decompile (function + address), [D]
disc, [S] suggested.

## The symptom

Cold `nf_view::load` of 07000016.bin (the kit holds 0x11, the load line prints
`switch request 17`), give the P2K (`inventory.equip_weapon(&wd, 2, 7, true)` → true, owned[2]),
then step with `Input { switch_to: Some(2), .. }` — `gun.weapon` stays 0x11 after 90 ticks
(`switch_request` still 0x11, state 0). Setting `sim.gun.switch_request = 2` directly switched
fine. The number keys and the dev panel ride `switch_to`, so a player pressing 2 during the
level-start draw got no switch.

## The cause

The port's `switch_to` arm sat inside the idle-only gate
(`switch_state == 0 && reload_state == 0 && !fire_busy`, `Sim::step_bond` in scene.rs). A cold
load opens with a start switch — `Player_InitWeapon` 0x1a7ef8:62 zeroes the view state and
writes `+0x63` = the kit weapon, and the state-0 arm applies it, so the first ticks run the
holst/draw [M]; the port's `nf_view::load` does the same via
`sim.gun.switch_request = init_level_loadout(...)` with the default weapon still in hand. During
that window `switch_state != 0`, the arm refused, and since `switch_to` is a per-tick input the
press was gone forever.

The game does NOT drop a request during the draw:

- `Player_WeaponChange` 0x1a6f80:24-40 refuses the cycle write only in the view states
  {1, 3, 5, 6, 7, 8, 9, 0xb, 0xd, 0xe, 0xf, 0x10} — the case values are (state − 1), so **state
  4 (the draw anim playing) is ALLOWED**, as are 0 (idle), 2, 0xa, 0xc.
- `Player_SetWeaponAnimObj` 0x1a2e38 **case 4** re-checks `+0x63 != +0x62` EVERY tick while the
  draw anim plays and retargets [M :309-315]: `+0x64 = +0x62` (save the being-drawn weapon),
  then LAB_001a39c8 :492-497 `AnimScriptSetPlayDirection(view, 2)` — the draw anim REVERSES —
  and the view state drops to 2; case 2's tail (`AnimScriptIsStopped`) swaps `+0x62 = +0x63` and
  state 3 draws the new weapon. So a press during the level-start draw queues and fires.
- The direct-swap button path is the strict one: `Player_Weapon` 0x1a24c8:210-211 requires view
  state 0 (and `Player_WeaponHasAmmo`).

## The fix (scene.rs only — no game-side files touched)

1. **The request write during the draw** [M 0x1a6f80's allowed set]: the `switch_to` arm now runs
   while `switch_state == 0 || switch_state == 3` (the port's state 3 spans the game's states
   3+4 — the draw-anim window the game's case 4 polls), with `reload_state == 0 && !fire_busy`
   unchanged (the game refuses the reload states 5-8 and the fire state 9 [M 0x1a6f80]). States
   1 (holsting) and 8 (variant swap) stay refused, matching the game's set. The cycle arms keep
   the idle gate (the game's allowed 2/0xa/0xc have no port counterpart — noted below).
2. **The mid-draw retarget** [M 0x1a2e38 case 4 + LAB_001a39c8]: the port's state-3 arm gained
   the game's poll — while the draw plays and `switch_request != weapon`, it saves
   `saved_weapon = weapon` (`+0x64 = +0x62`), flips to a NEW port state 2 and REVERSES the draw
   (`view_speed = −view_speed`; the game's `AnimScriptSetPlayDirection(2)`). The state-2 arm
   waits for the reversed anim to reach its start, then does the same clip/spare flush +
   prev-weapon bookkeeping the holst end does, swaps `weapon = switch_request`, and draws the
   new weapon at 1.25× — the game's case-2-tail → case-3 sequence. The view_time advance gained
   a `.max(0.0)` floor for the reverse. (One [S] simplification: a same-base retarget mid-draw
   applies as a plain swap instead of the game's twin-swap state 8 — the twin request mid-draw
   is not reachable from the number keys.)

## Verification

- **Fail-first shown**: with scene.rs stashed, both tests fail — the mash test dies at "the P2K
  is in hand after the start draw settles" (tick 1 printed `state 1 request 0x11 weapon 0x2`),
  the single-press test at "the first-possible-tick press switched on 07000016.bin".
- `crates/nf-view/tests/fix_switch_after_cold_load_live.rs` 2/2 on the merged tree:
  - `cold_load_number_key_press_switches_once_the_start_draw_settles` (07000016): the give, then
    `switch_to: Some(2)` on every tick through the start switch → `gun.weapon == 2` and
    `switch_request == 2` after 90 ticks.
  - `first_possible_tick_and_settled_presses_switch_on_both_levels`: on 07000016 (kit 0x11) and
    07000012 (kit 0x10) — a SINGLE press on the first tick of the draw window (state 3) ends with
    the P2K drawn; a press after everything settles still switches (the existing path). The
    retarget path is exercised by the single press (the request lands mid-draw → the reverse →
    the new draw).
- The existing switch tests on the merged tree: `weapon_switch_live` 2/2 (nf-game),
  `fix_auto_switch_live` 2/2 (nf-view).
- The full suite (the fast runner) on the final merged tree: see the WORKLOG's final line — the
  FAILED/TIMEOUT lines and the total are recorded there (the allowed slim-ISO set: 3
  `disc_golden`; the no-GPU pylon render tests fail in this sandbox as on master).

## What's left / notes

- The cycle arms (mouse wheel / Q / G) still gate on idle; the game allows the cycle write in
  the draw state (4) too — relaxing them is the same one-line change if a playtest asks for it;
  the task's scope was the number keys / dev panel (`switch_to`).
- The game's swap-button path additionally requires `Player_WeaponHasAmmo(target)` [M
  0x1a24c8:211-213]; the port's `switch_to` arm checks owned + first-person model but not ammo —
  pre-existing on master, unchanged here (a press for an empty weapon still switches to it, same
  as before this task).

## Live checks for the main agent

1. **The game's mid-draw retarget on a real cold load** (PCSX2): load 07000016 cold (the crossbow
   kit), during the first ~1.5 s press the weapon-cycle button (Q / the pad cycle) — expected:
   the crossbow draw reverses back in and the cycled weapon draws immediately after (the case-4
   retarget), NOT "nothing happens until the draw settles". gdbwatch the view state
   `player_anim_obj+0xf4`: expect … → 3 → 4 → (press) 2 → 3 → 0.
2. **The number-key equivalent**: nf-view `--bond` at the 07000016 start, press 2 during the
   start draw and after it — the P2K should come up in both cases (the fix), and the reverse
   retarget should show the crossbow sliding back in mid-draw before the P2K draw.
3. **The settle case**: a press ~2 s after the load (the draw settled) — unchanged behaviour, the
   normal holst+draw.
