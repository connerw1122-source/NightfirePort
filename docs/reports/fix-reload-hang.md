# fix-reload-hang — the reload state machine (state 6, clip 0)

Branch `s/fix-reload-hang` (from `origin/master` @ 088bdf4; weapon-matrix2 already merged).
Task ID 103i. Files this task changed: `crates/nf-view/src/scene.rs` (the fix + nothing else),
`crates/nf-view/src/matrix.rs` (the dbg line extended with `vs`/`vt` under `NF_MATRIX_DBG` —
diagnosis aid, additive), `crates/nf-view/tests/fix_reload_hang_live.rs` (new, 5 tests), and two
probe examples (`probe_reload_records.rs`, `probe_missile_cam.rs` — kept for the next window).

## The headline finding: the rel=6 hang does NOT reproduce on the merged master

The task brief pins `rel=6` forever with clip 0 after the silenced twin 0x03's auto-reload starts.
**Five headless NF_ISO tests through the real input path (`nf_view::load_weapons` + `panel::apply`'s
give + `Sim::step_bond`) could not reproduce it on this master — the machine returns:**

- `empty_silenced_twin_0x03_reloads_to_idle_with_the_clip_refilled` — empty 0x03 with real trigger
  presses, the auto-reload starts (branch B), rides 5→6, refills the clip and returns to idle; the
  switch back to 0x02 then lands. GREEN.
- `sentinel_0x2d_fire_then_reload_returns_to_idle` — the 0x2d twin (reached the way the player
  reaches it: give 0x2c, mode press; 0x2d is base 0x2c's twin [L probe dump: +0x05 = 0xff]), fire,
  reload, idle, clip refilled. GREEN.
- `shotgun_per_shell_loop_moves_one_per_pass_and_fire_interrupts_it` — the SPAS-12 (0x1c, flag
  0x100): each +0xA4 pass moves one shell, a held trigger during state 6 pushes the loop to
  state 7 (+0xAC) and the machine returns. GREEN.
- `switch_pressed_mid_reload_lands_only_after_the_reload_settles` — mid-reload switch_to presses
  are refused (the refused set {5,6,7} [M 0x1a6f80:33-47]); after the reload settles the same
  request lands. GREEN.
- `twin_hop_keeps_the_magazine_shared_with_the_base` — the fix's test (below). GREEN.

The game-side evidence agrees that no path can stick [M]:
- `Player_SetWeaponAnimObj` case 6 (0x1a2e38:345-373): when the +0xA4 script stops,
  `Player_ReloadAmmoType(_, 0)` moves the rounds; non-0x100 records go to LAB_001a3678 (the +0xA0
  idle script, state 0, `Input_ClearAction(9)`); 0x100 records probe with mode 1 — shells left and
  no fresh fire press replay +0xA4 (state 6), else +0xAC and state 7. Every branch exits.
- `Player_ReloadAmmoType` 0x1a1988: the moved count is `min(clip_size − clip, carried)`, clamped to
  1 per call for flag-0x100; carried == 0 or a full clip returns 0 — and the caller EXITS on 0.
- `Player_WeaponFiring` 0x1a5460's shared tail probes mode 1 BEFORE entering state 5/6 — no ammo,
  no reload (`Player_HandleHasNoAmmo` instead).

## What the red mass actually is (the matrix re-run evidence)

`nf-extract weapon-matrix` re-run on 07000005 (twice, instrumented), 07000002, 0700000c — the rows
are IDENTICAL to weapon-matrix2's full-sweep row per level. Instrumenting the runner
(`NF_MATRIX_DBG` + a per-cell dump of `missile_cam`/`disabled`/`death.active`/`fail_menu_open`)
decomposes the D/M/R mass:

1. **The D-block from a mid-sweep death (the mass of D 1278).** On 07000005 Bond DIES during the
   0x2c guided-Sentinel cell — `death.active = true` from cell 0x2d to the end of the row (172
   dbg2 samples) — and `step_bond` zeroes every input while the death is active (scene.rs's death
   branch; the machine itself stays idle: sw=0 rel=0 fire=0, view idle). Every later cell's
   give/draw/switch is silently eaten → D. The level's rocket/grenade cells kill Bond mid-sweep
   and the matrix never dismisses the fail menu, so the row is dead below the killing cell. This
   also explains why the D-block start column differs per level (whichever cell's blast reaches
   Bond on that level's spawn spot).
2. **The R marks are timing artifacts, not the machine.** The reload pulse (R2 edge, 1 tick) is
   lost when the cell's fire script is still playing (`busy` blocks `can_reload`) or when the
   guided-missile camera is riding (07000002's 0x2c–0x30 rockets all R with gcam1 on 0x2c — the
   camera eats the whole reload phase). The machine itself completes (my tests ride it green).
3. **The twin-refusal D (0x18, 0x2d, ...).** `switch_to` cannot select a twin record — `owned[]`
   is per base (`Player_EquipWeapon` 0x1a8a58:39-41) and the direct select needs `owned[id]` — the
   game's own model: the player reaches twins with the mode button. The runner's `mode_to` presses
   the mode button on whatever is held, which bounces 0x02↔0x03 (each twin's +0x05 is ±1) and
   never reaches the target → D. Harness limitation, not a sim bug.
4. **The M marks**: the mode check runs on the same bounced twin state (mode_tried && !mode_ok).

## The port fix this task lands: the twin clip slot at the runtime boundary

[M `Player_ReloadAmmoType` 0x1a1988:18-22] a twin record (+0x05 set) whose ammo type matches its
base reloads `clip[base]` — one physical magazine per twin family (0x02/0x03, 0x16/0x17, 0x1c/0x1d,
0x24/0x25, 0x2a/0x2b, 0x2c/0x2d [L probe dump]). The give path already equips into that slot
(pickup.rs `clip_slot`). The runtime boundary did NOT: the per-tick read (old line 2158), the two
swap paths' reads, and the end-of-run write-back used the held weapon's OWN slot. In continuous play
the write-back re-mirrors the magazine on each hop, but the two slots are separate state — any
external write to one slot (the harness's per-cell reset, a future pickup write to clip[0x03])
leaves the other stale, and the machine then reads a 0 where the game reads the shared magazine and
fires a spurious reload. The fix adds `Sim::clip_slot` (the same base-unification predicate) and
uses it at all four boundary sites. Structurally the runtime now has ONE magazine per family, as
the game does.

- `twin_hop_keeps_the_magazine_shared_with_the_base`: reload on the twin, hop back — the base
  reads the shared reloaded magazine, no second reload, `spare` spent exactly once. GREEN with the
  fix. (Honest note: in undisturbed continuous play this test also passes pre-fix, because the
  write-back re-mirrors the slots on every hop — the fix closes the divergence class rather than a
  continuous-play symptom.)
- The reload exits themselves needed NO change: the port's `Gun::reload_tick` already implements
  the game's case 5/6/7 exits (the per-pass shell, the fire interrupt to +0xAC, the full/empty
  exits) — the tests above pin them.

## Verification

- `cargo test --offline --release -p nf-view --test fix_reload_hang_live -- --test-threads=1`
  (NF_ISO): **5 passed / 0 failed** (5.5 s).
- `nf-extract weapon-matrix` on 07000005/07000002/0700000c: 0 panics; rows identical to
  weapon-matrix2's sweep for the same levels (the red decomposition above). The full 27-level sweep
  (≈16 min) did not fit this window next to the suite — next window's first item, WITH the runner
  fixes below so the numbers mean something.
- The fast suite (tools/test/suite.py) on this tree: build ok (258 binaries); FAILED lines while
  it ran: the 3 `disc_golden` (the slim-ISO allowance), `weapon_matrix_live` TIMEOUT 600 s (the
  suite runs the full 27-level matrix — ~16 min — against a 600 s per-binary cap; pre-existing,
  weapon-matrix2's report notes the same), and `fix_0c_pylon_live`/`probe_pylon_render` while the
  3-level matrix sweep ran concurrently on the same 2-CPU box (render timing; both binaries pass
  in isolation with the GPU env).
- `fix_weapon_fire_modes_live::rocket_hit_applies_the_blast_exactly_once` failed on this tree —
  the test pokes `clip[0x2d]` for the held AT-420 twin, but under the game's model the live
  magazine IS `clip[0x2c]` [M 0x1a1988:18-22] and the fixed runtime boundary now reads the base
  slot. The test's poke was aligned to the game's slot (both slots poked, comment added) — the
  binary is 5 passed / 0 failed after. This is the one shared file this task had to touch
  (`fix_weapon_fire_modes_live.rs`, 4 lines: the poke + the comment).

## Shared files touched

- `crates/nf-view/src/scene.rs`: the `clip_slot` helper + the four boundary sites (the fix), all in
  the weapon block. No other lines.
- `crates/nf-view/src/matrix.rs`: the `NF_MATRIX_DBG` draw-sample line extended with `vs`/`vt`
  (additive; no behavior).

## What's left (next window)

1. **The matrix runner needs three harness fixes before its numbers mean anything**: (a) dismiss
   the fail menu / keep Bond alive (the death eats every input below the killing cell — the
   D-mass); (b) reach twins the way the player does (give base → cycle to it → mode press) instead
   of `switch_to` + the bouncing `mode_to`; (c) wait out the fire script AND the missile camera
   before the reload pulse (or lengthen the window). Then re-run the full 27-level sweep and put
   the new red counts next to weapon-matrix2's.
2. `Gun::has_ammo`'s clip read is per-id; the game's `Player_AmmoInGun` presumably reads the shared
   base slot for twins (same unification) — only observable when carried == 0 and the shared clip
   holds rounds; not covered by a failing case yet.
3. The full suite re-run on the final merged tree.

## Live checks for the main agent (PCSX2)

1. **The 0x03 auto-reload** (any level): give the P2K, mode-button to the silenced twin, empty the
   clip (hold fire), release — the real game's 0x03 reload should refill and return to idle. The
   port now matches headlessly; confirm on hardware that nothing sticks (`gdbwatch.py` on the
   reload state byte at anim+0xF4 and `reload_ticks`).
2. **The twin magazine** — give the P2K with a partial clip, mode to 0x03, fire once, mode back:
   the HUD ammo count must show the SAME magazine minus one on both records (the game's shared
   clip[0x02]), and the spare pool drops only by what was fired. The port's twin-hop test pins this
   headlessly.
3. **The Sentinel self-kill** — on the Nightfire estate (07000005) fire the guided AT-420 (0x2c)
   from the spawn spot at the wall ahead: does the blast kill Bond on the real game (the matrix
   rows die there; we could not confirm the game's own behaviour)?
4. **nf-view spots** (after the fix, 07000005): `--give 0x03` then fire/reload — the first-person
   reload script 0x06000827 (155 frames at 1.35×) should play with the three reload sounds
   (0x40F/0x40E/0x411 at frames 0x2E/0x59/0x74) and the clip should refill to 7.
