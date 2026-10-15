# fix-zoom-during-swap — report

## The symptom

Playtest 1015-0046 (`--give 0x02,0x1e,0x1c`): "If I'm holding aim while swap … I'm on the shotgun
and I swap to the sniper rifle … before the sniper rifle comes up and I'm pressing right click, it
doesn't actually zoom in once the sniper rifle's finished. And when I let go of right click, it
like does like a half zoom, but never actually zooms in."

## What was done

**The cause** [M `Player_Weapon` 0x1a24c8 — work/decomp/ACTION.ELF/001a24c8]: the game's
raise/lower queue edge is the **aim BIT's** tick-over-tick change, not the raw button's:

- `bVar5` = the bit at function entry, i.e. after `AnimObjectUpdate` ran (the raise's finish
  included) [M :57];
- the input arm (hold mode) sets the bit from the held action [M :59-79];
- the machine force-clear wipes it again while the view machine runs — the `+0x68 & 0x40` switch
  clears the bit in view states 1..4, 6..8, 0xa, 0xd..0x10 on a scoped record, 1..4, 0xa, 0xd,
  0xe on an unscoped one [M :80-96];
- the edge check compares the post-clear bit against `bVar5` [M :100-119], and on a rise with a
  raise script (`+0xB8`) case D re-clears the bit and queues view state 0xF (the raise at 1.25)
  [M :145-152]; on a fall it queues the lower from the idle machine [M :120-143].

So on the tick the draw finishes (machine back to 0) with the right button STILL held, the bit
rises (cleared during the draw, set by the input arm now) and the raise queues immediately. The
port queued the raise on the **raw button's edge** (run's `input.aiming || input.scope` param vs
`prev_aiming`), which a button held from before the swap never produces again — the raise never
queued, the bit never set, the zoom never moved. On release the lower still queued and played the
raise REVERSED from the raised pose while the weapon sat at idle — the user's "half zoom".

**Fix 1 — the edge is the aim bit's** (crates/nf-view/src/scene.rs, the task's named file):
`aim_gate` now reads its edge reference after its advance/finish half (the game's `bVar5`
position), applies the machine force-clear with the port's machine mapping (switch_state 1/3/8 =
views 1/3/8; reload_state 6/7 = views 6/7 — reload_state 5 = view 5 is NOT in the game's set;
`view_aim_anim` = 0xf/0x10; fire_state = view 9, which the game's case-8 arm keeps, so no fire
term; the grenade hold 0xb is not in the set), and marks two new fields `aim_rise`/`aim_fall`
(own lines) for raise-script records. The view chain's queue arm (the single queue point) reads
them instead of the raw-button edges. Case D's re-clear keeps the final bit false on the rise
[M 0x1a24c8:145-152]. A no-raise record keeps the input arm's bit [M :117-119].

**Fix 2 — the exact `Player_Zoom` ride** [M `Player_Zoom` 0x1a1a70:37-79]: the port interpolated
the zoom value toward the target with a lerp, which converges asymptotically and never reaches
1.0 on release. The game MULTIPLIES the value (×1.025/tick in, ÷1.15/tick out — the rate inverted
when the value sits above the target) and SNAPS on crossing the target [M :71-79]. The port now
does the same, so the release lands on exactly 1.0.

**Test infrastructure** (the new NF_ISO test, `crates/nf-view/tests/fix_zoom_during_swap_live.rs`):
both weapons are owned through panel gives (the user's `--give` shape — `load_weapons`' give list
merges view-model assets only, inventory ownership needs the equip), then the whole scenario runs
through `Sim::step_bond` with `Input { scope: true, zoom: true }` held from before the swap.

## Records used [L ee_start_05.bin @ weapon_data 0x2bf150, stride 0x10C]

- Shotgun 0x1c: NOT scoped (+0x68 = 0x0080_8100), no raise script — the held right button must
  not aim it and the zoom must stay 1.0.
- Tactical Sniper 0x1e: scoped (+0x68 = 0x0040_8050), raises with 0x0600_07b9 (24 frames →
  ceil(23/1.25) = 19 ticks at 1.25), no +0xBC (the lower plays the raise REVERSED), max zoom
  (+0x88) 10.0.

## How it was verified

- **FAIL-FIRST**: with the scene.rs fix stashed, both tests panic at line 135 — "the raise never
  queued from the button held across the swap" — the exact symptom. (The very first pre-fix run
  failed earlier, at "the swap never landed": the sniper was never panel-given; fixed before
  measuring.) Green after the fix.
- `holding_scope_across_the_swap_zooms_the_sniper_once_its_draw_finishes`: the held button must
  not aim the unscoped shotgun (bit clear, zoom 1.0); the swap lands while held; the zoom stays
  1.0 for the whole draw; the raise queues from the held button (no new press) with the machine
  idle; the raise runs exactly 19 ticks; the bit sets on its last tick; the zoom target jumps to
  the sniper's max 10.0 and the value starts climbing the same tick; with the button still held
  the zoom climbs past 4.0 and the raise never re-queues (the finish's bit-set is not an edge).
- Sister `releasing_scope_after_the_swap_zooms_back_out_and_lowers`: the release tick drops the
  bit, returns the zoom target to 1.0 at once, queues the lower (the raise REVERSED — 0x1e has no
  +0xBC) and the zoom returns to exactly 1.0 with the idle back.
- Sisters green on the fixed tree: fix_aim_raise_live 5/5, mouse_scope_live 3/3,
  fix_viewmodel_zoom_live 3/3, fix_zoom_overlays_live 6/6, weapon_anims_live 3/3,
  fix_aims20_modes_lights_live 3/3, fix_samurai_fire_live 7/7.
- Full suite on the final tree (merged with origin/master): see the total line below.

## Shared files touched

- `crates/nf-view/src/scene.rs` — the task's named file (the aim gate). Additive: two new Sim
  fields (`aim_rise`, `aim_fall`, own lines + constructor lines), the `aim_gate` edge rework, the
  queue arm's edge source, the zoom value integration, and one comment block replaced at the old
  raw-edge capture (the `prev_aiming` field is kept and still updated from the raw param, now
  unused by the edge). No other shared file touched.

## What's left / residuals

- **One-tick latency vs the game's intra-tick order**: the game's `Player_Weapon` runs after the
  anim update inside the same tick, so its raise queues on the very tick the draw ends; the
  port's `aim_gate` runs before `run`'s machine advance, so the re-rise lands the next tick.
  Invisible in play; the tests ride ticks and pin the sequence, not the absolute tick index.
- **A press during reload view state 5** (the game does NOT clear the bit there, so its rise
  branch could queue 0xF mid-reload): the port's queue arm keeps its idle-machine gate, so the
  raise waits for the reload to end. Sane, one-ish tick late, not this task's case — noted.
- **A re-press while the lower is still playing**: the port defers the raise to the lower's end
  (the queue arm requires `view_aim_anim == None`); the game's rise branch would displace it.
- The toggle-aim option (the game's `DAT_002a38cb` per-weapon flag, action 4 XOR) is not modeled —
  hold-aim only, as before.
- The zoom TARGET still jumps straight to max on the bit's rise (the shipped model, pinned by the
  existing tests' feel) instead of riding 1.025×/tick like the game's `+0x8D4`; only the VALUE now
  uses the game's exact multiply+snap form. The user-visible climb rate is unchanged.

## Live checks for the main agent

PCSX2 (07000005, loadout = the user's `--give 0x02,0x1e,0x1c` shape):

1. Spot: the shotgun out. Action: hold RMB, swap to the Tactical Sniper while holding, never
   release. Expected: the scope zooms in as the draw finishes (the raise queues the tick the
   machine idles [M 0x1a24c8:113-152]); no zoom during the draw itself.
2. Spot: zoomed after (1). Action: release RMB. Expected: the lower plays the raise reversed and
   the zoom returns to 1.0 cleanly — no half-zoom pose from idle.
3. Spot: the sniper out. Action: hold RMB through a full reload. Expected: the zoom drops during
   the reload's late machine states (the scoped clear set [M 0x1a24c8:80-96]) and comes back up
   when the reload ends with the button still held.
4. gdbwatch: the view anim object's aim bit (obj+0x60 bit 0) across (1) — expect 0 during the
   draw, 1 from the machine-idle tick on while the button stays held.

nf-view `--bond` spots (07000005): start 105.9,25.3,108.1 facing the gate at 104.95,25.12,108.83 —
with the sniper and the held-across-swap input the FOV should narrow only after the draw (world
magnified, the view model in the raised pose), and back out to the normal FOV after the release.

## Suite

Run on the final tree (merged with origin/master @ 4da7220e), fast runner, slim ISO, started
09:50 UTC:

```
1267 passed, 6 failed, 66 ignored  (53 min)
```

The 6: the 3 allowed `disc_golden` (slim ISO) + `fix_0c_pylon_live` (2) + `probe_pylon_render` —
the pylon set another session verified pre-existing on master by a stashed re-run (their WORKLOG
entry, 2026-10-15). No failure touches the aim gate, the zoom or the switch machine.
