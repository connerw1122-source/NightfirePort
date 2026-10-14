# 103i2-fix-reload-hang2 — the shared-magazine fix silences the gadget draw sound

**Status: done** (the driver's held failure is fixed and regression-tested; 4 residual
`missing sounds never played` cells in the same sweep test remain and are documented below with
everything that was tried).

## The held failure

`fix_weapon_sounds_live::every_weapon_on_07000005_draws_fires_and_reloads_with_sounds` failed on
the merge of `s/fix-reload-hang` with `origin/master`, quoting
`0x4c / 0x4f / 0x51 / 0x52 / 0x53 / 0x56 / 0x57 / 0x58 / 0x59: draw sound 0x281 never played`.
The real failure list was longer: **16** "draw sound 0x281 never played" records (the quoted 9
plus 0x5b, 0x5d, 0x5e, 0x6a, 0x6d, 0x6e, 0x6f) and **5** "none of its missing sounds ever played"
(0x3a, 0x3b, 0x41, 0x42, 0x43). The non-quoted gadget ids (0x4a, 0x4b, 0x4d, 0x4e, 0x50, 0x54,
0x55, 0x5a, 0x5c) were never checked — their gives are refused by the known 100a first-person
asset gap, so 0x4c was the first gadget the sweep actually equipped and checked.

## Root cause (one bug explains all 16 draw-sound cells)

The switch machine **wedged in switch state 3 at weapon 0x38** and never recovered, so no weapon
after 0x38 ever drew again — every later check hit the global `seen` map, where 0x282 (weapon
draw) was present from earlier weapons but 0x281 (gadget draw) had never played. Evidence
[NF_SOUNDLOG, this branch pre-fix]:

```
tick 2466: state 0→1 holster 0600072b (weapon 0x37)
tick 2478: state 1→3 draw 0600072c (weapon 0x38)     ← 0x0600072c is 0x37's DRAW script
                                                        (0x38's own is 0x060004ee [record dump])
(no "3→0 draw done" line ever; the log ends)
```

Minimal repro (0x37 drawn and drained, then equip 0x38): `sw=3` forever, `weapon` already 0x38.

Mechanism [M]: `Sim::clip_slot` (the base-unification predicate,
`Player_ReloadAmmoType` 0x1a1988:18-22) is legitimate for 0x38 — its +0x05 is set and its ammo
type (0x16) equals base 0x37's. But the merge fed the unified **clip slot** into the switch
machine's `draw_script(nwi)` / `idle_script(nwi)` lookups, while the state-3 completion check
measures `draw_script(self.gun.weapon)`. The game reads the draw script from the weapon's **own
record** — `Player_SetWeaponAnimObj` 0x1a2e38 case 3: `weapon_data[iVar15 + 0xc0]` with
`iVar15 = held weapon's own id · 0x10c`; only the clip array (`clip[0x1B8 + slot·0xC]`) routes to
the base [M 0x1a1988:18-22]. So the port started 0x37's draw script for the held 0x38, clamped
`view_time` to 0x0600072c's length, and waited for 0x060004ee's length — a wait that never ends.

## The fix

`crates/nf-view/src/scene.rs` — the four switch-machine script lookups use the weapon's own id;
the clip/spare reads and write-backs keep the unified slot:

- instant-swap arm: `draw_script(self.gun.weapon)` (was `draw_script(nwi)`), its no-draw
  `idle_script(self.gun.weapon)`;
- state 1→3 arm: `draw_script(self.gun.weapon)`, its no-draw `idle_script(self.gun.weapon)`.

Citations are in the code comments ([M 0x1a2e38 case 3] / [M 0x1a1988:18-22]). `clip_slot`
itself is unchanged — `twin_hop_keeps_the_magazine_shared_with_the_base` stays green.

## Verification

- Repro before fix: the sweep failed with the 16+5 list (reproduced exactly on this tree);
  the minimal 0x37→0x38 repro wedged at sw=3 forever.
- Pre-fix regression test: `fix_reload_hang_live::the_twins_draw_completes_and_the_machine_stays_alive`
  **FAILED pre-fix** ("the twin 0x38 never finished its draw — the switch machine wedged"),
  **passes post-fix**; sister assertion: a fresh switch request back to 0x37 still lands (the
  machine stayed alive). The whole `fix_reload_hang_live` file is 6 passed / 0 failed post-fix
  (twin_hop included).
- Post-fix sweep: all 16 "draw sound 0x281 never played" cells are gone; the switch log runs
  through every weapon to 0x53+ with each draw completing.
- `matrix.rs` and the other shared files: no runtime change (see below).

## What's left (the 4 residual cells)

The sweep still fails on `0x3a [1093, 1094]`, `0x41 [639]`, `0x42 [1051, 1481]`,
`0x43 [1486, 1487, 1488]` — "none of its missing sounds ever played". Post-fix these records
**draw fine** (switch log: 0x3a drawn at 2548, done 2562; 0x41 at 2697; 0x42 at 2835; 0x43 at
2919 — all completed), but their fire never happens. A focused probe (0x3a equipped alone,
25 fire ticks through the real `step_bond` path) shows `clip=1 spare=3 rel=0 sw=0 timer=-1` on
every tick — the trigger never opens the fire gate — identically alone and after the 0x37→0x38
twins, so it is not downstream state pollution. The same probe under master's `scene.rs` was
attempted but its output was not captured before the tree was restored, so the master-vs-branch
comparison for the probe itself is not on record. Next window: why the fire gate never opens for
these four records (`Gun::update_with`'s trigger path / `+0x44` release-frame family / the
`has_fire_script` gate).

## Suite

The full `tools/test/suite.py` was started on the final tree (this branch merged with
origin/master) at 20:03 UTC; the sandbox window ended before it printed its total. Per-file
results on the final tree (`--test-threads=1`, NF_ISO): `fix_reload_hang_live` 6 passed / 0
failed; `fix_weapon_sounds_live` 2 passed / 1 failed / 2 ignored (the sweep = the 4 residuals
above); `zz_reload2_*` scratch probes are untracked and not committed.

## Shared files touched

- `crates/nf-view/src/scene.rs`: the four script-lookup lines + comments (the task's named file).
- `crates/nf-view/tests/fix_reload_hang_live.rs`: one additive test at the end (the task's named
  test neighbourhood; the file belongs to this branch's base task).
- `crates/nf-view/src/matrix.rs`: NO change by this task (the diff vs master is the base branch's
  `vs/vt` debug eprintlns, kept).
- `docs/WORKLOG.md`: appended only.

## Live checks for the main agent

- PCSX2, any level: give the AT-420 Sentry-family twin 0x38 while 0x37 is held and drained —
  the draw must complete and the swap sound 0x282 plays; then every later weapon switch still
  works (on the pre-fix port the machine wedged here).
- nf-view `--bond` spots: none needed (no visual change; the fix only restores the switch
  machine's progress).

## ntfy

(sent at the end of the session; if this line lacks a confirmation, the send failed — see the
WORKLOG.)
