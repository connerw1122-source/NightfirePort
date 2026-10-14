# fix-blast-scripts2 — blast scripts + cut-scene actors: ONE 0x060007c4 path

Branch `s/fix-blast-scripts2` = `s/fix-blast-scripts` + `origin/master` + `origin/s/fix-samurai-fire`
+ `origin/s/fix-cutscene-actors` (291), in that order. Task: the merge was HELD because
`fix_blast_scripts_live::the_samurai_bolt_plays_the_cyan_blast_script` failed — "exactly one
0x060007c4 one-shot player (the bolt's cyan blast) — left 0, right 1" — with master's Samurai fire
path (fire-modes3/samurai-fire) and this branch's per-source script carry disagreeing about where
the bolt's blast is created and which script it plays.

## What was wrong (the hold's root cause)

The merged tree ran TWO disagreeing paths for the Samurai bolt's contact:

1. **The world path's first arm was mis-shaped.** The fire-modes3-shaped arm gated on
   `flags70 & (4 | 0x100000 | 0x200000) != 0` — it caught the bolt's 0x100000 bit too — but pushed
   the **generic script 0x06000052** into `pending_blasts`. The bolt's own `else if` arm (pushing
   0x06007c4) sat after it and was **dead code**: no bolt bit ever reached it. Result: every bolt
   world hit spawned a generic 0x06000052 blast player and the cyan 0x06007c4 player never came
   from the blast path at all.
2. **fix-cutscene-actors (291) had added the effect from the other side**: a separate direct
   `spawn_once(usize::MAX, 0x06007c4, bm)` block right after the arm, scale 1. That is why the
   hold's symptom was "left 0" on the driver's tree (master without 291: nothing drew the cyan
   script) and "one cyan player at scale 1 + one leaked generic player at scale 8" on this branch's
   merged tree: the scale-1.0 cyan player the test then saw came from 291's block, not from the
   blast — and the generic 0x06000052 player leaked beside it.

The disc evidence reconciles everything ([M] = ACTION.ELF disassembly, little-endian decode, via
`work/elf/ACTION.ELF`; the decompile file `work/decomp/ACTION.ELF/0012ce80_Bullet_CollisionHandler__FP7obj_tag.c`):

- `Bullet_CollisionHandler` 0x12ce80 runs **three independent explode arms** on every non-ricochet
  contact (`bVar7 == 0`), world AND flesh alike:
  - `+0x70 & 4`: `Explode_Create(f12 = +0x08, f13 = +0x08, f14 = +0x0C, script 0x6000052)`
    [M 0x12d0ac-0x12d0c0: `lwc1 $f12, 8($v1)`; `mov.s $f13, $f12`; `lwc1 $f14, 0xc($v1)`;
    `$a3 = 0x6000052`];
  - `+0x70 & 0x100000`: `Explode_Create(f12 = 1.0, f13 = +0x08, f14 = +0x0C, script 0x60007c4)`
    [M 0x12d0ec-0x12d0f0: `lui $at, 0x3f80; mtc1 $at, $f12`; 0x12d118: `lwc1 $f13, 8($t2)`;
    0x12d110: `lwc1 $f14, 0xc($t2)`; 0x12d108: `$a3 = 0x60007c4`];
  - `+0x70 & 0x200000`: identical shape [M 0x12d154-0x12d188].
- `Explode_Create` 0x17cbd0 stores its first float (f12) into `SCRIPTINFO+0x94` — the script
  player's scale (`Script_GetInterp` 0x1da158:117 multiplies the key scales by it) — and its second
  float (f13) is the blast radius (Camera_Shake ≥ 0.5, the explode object's radius fields)
  [M 0x17cbd0 body].
- The fuse death `Bullet_handle_object_destruction` 0x12cd98 passes f12 = +0x08 (= radius)
  [M 0x12ce1c] and `Destroy_Smash` 0x12bb60 passes f12 = tag+0x8c, f13 = +0x08 [M 0x12bcdc/0x12bd04].

**So the game's Samurai bolt cyan blast plays at scale 1.0 while the blast radius stays 8.0.** The
`fix_blast_scripts_live` test pinned the player scale at 8.0 ("scale = the radius") — that
expectation contradicted the disc and is corrected to 1.0 with the disasm citation. The 0x4-arm
callers genuinely pass f12 = radius, which is why the frag/rocket sisters' 8.0 pins stay correct.

## The fix (ONE path)

All in `crates/nf-game/src/drone.rs` (+ the test's corrected pin):

- **The world arm now picks script AND scale by bits in one arm** — the same shape the flesh arm
  already used (that conflict resolution kept the script-carrying side at the samurai-fire merge):
  `script = if flags70 & 4 != 0 { 0x06000052 } else { 0x060007c4 }`,
  `scale = if flags70 & 4 != 0 { radius } else { 1.0 }`. The handler's arms are the same
  `Explode_Create` on world and flesh contacts, so the rule carries over unchanged.
- **291's separate direct `spawn_once` block is gone** — the queued blast's `spawn_blast` spawns
  the same 0x06007c4 script player (at the game's f12 scale), and its entities draw from the
  scene-build sweep, which keeps ALL FOUR blast hashes (0x06000052/0x06000027/0x0600004f/0x060007c4)
  [M 0x12bc50-0x12bd20 for the Destroy_Smash types]. One creation point, one draw path.
- **`pending_blasts` carries the game's f12 as a 7th tuple element** (pos, radius, damage, owner,
  weapon_type, script, scale) and `spawn_blast` takes the scale instead of deriving it from the
  radius. Every push site cites its arm's f12: 0x4 contact (world + grenade contact) and the fuse
  death = radius [M 0x12d0ac / 0x12ce1c]; the bolt arms (world + flesh) = 1.0 [M 0x12d0ec-0x12d0f0];
  destroy = radius kept (the game's f12 there is tag+0x8c [M 0x12bcdc] — no green test pins that
  scale, flagged as a live check below).

## How it was verified

- **Reproduce-before-fix**: on the merged (pre-fix) tree the test failed at the scale pin — "the
  blast player's scale is the radius 8.0, got 1" — i.e. the cyan player existed (291's block) at the
  game's scale while the blast path had played the generic script. With the scale pin corrected to
  the disc's 1.0, the test then failed at the **sister assert** "the bolt must not play the generic
  0x06000052" — the fail-first proof that the mis-routed arm leaked the generic player. Post-fix all
  three `fix_blast_scripts_live` tests are green (the bolt's cyan script + scale 1.0, no generic
  player; the destroyable's type script; the frag/rocket sisters).
- **Neighbours**: `fix_cutscene_actors_live` 5/5 (the scene sweep still builds 0x06007c4's entity
  streams on 05/06/07), `fix_samurai_fire_live` 3/3 (the bolt's blast record: radius 8 / damage 256
  on the wall contact, the release frame 280, the beam trail), `fix_weapon_fire_modes_live` 5/5
  (the MGL/Sentinel contact-blast arms).
- **Full suite on the final tree** (this sandbox kills background jobs — the 102c/100f/oddjob-hat
  precedent — so the fast runner's workload ran as foreground `--test-threads=1` per-binary chunks
  with the same per-binary 600 s cap):
  **1154 passed / 6 failed / 63 ignored across 288 test binaries.**
  The 6 failures: the 3 ALLOWED `disc_golden` tests (slim ISO) + the 3 pre-existing pylon failures
  (`probe_pylon_render` 1, `fix_0c_pylon_live` 2 — documented on weapon-matrix4b, untouched here).
  `weapon_matrix_live` green in-suite (6 passed / 24 ignored, 340 s — the held timeout stays gone).
  `origin/master` had not moved when last fetched (merge-base check: master already an ancestor).
- Master pulled before starting and re-checked before the final push (the task's pull rule).

## Files

- `crates/nf-game/src/drone.rs` — the ONE-path arm + the scale through `pending_blasts`/`spawn_blast` (task file).
- `crates/nf-view/tests/fix_blast_scripts_live.rs` — the corrected scale pin + citations (task file).
- Merge-only/compile-fix touches (shared, minimal, behaviour-neutral):
  - `crates/nf-view/src/scene.rs` — the conflict resolution keeps the four-hash sweep; the comment
    unions both branches' evidence. No behaviour change.
  - `crates/nf-view/src/matrix.rs` — the `NF_MATRIX_DBG` `has_ammo` call gained master's 103i2
    `clip_slot` argument (a compile fix; dbg-only code).
- Arrived from the merges themselves: `crates/nf-game/src/gun.rs`, `crates/nf-view/tests/weapon_switch_live.rs`,
  `crates/nf-view/tests/fix_samurai_fire_live.rs`, `crates/nf-view/tests/fix_cutscene_actors_live.rs`,
  `docs/reports/fix-samurai-fire.md`, `docs/reports/fix-cutscene-actors.md`, `docs/WORKLOG.md` (both sides kept).

## Renders for the main agent (no GPU here — please render before merging)

The bolt's contact effect is the visible change (the cyan blast's scale is now the game's 1.0, and
the leaked generic 0x06000052 explosion is gone). On **07000005** (Samurai Overcharge bolt into the
spawn's floor, the test's own shape):

- `--bond` at the 07000005 spawn ground spot, e.g. `105.9,25.3,108.1` yaw 0 pitch −1.57 (the test's
  load → `give 0x33` → ○ to 0x32 → hold fire through the 280-tick charge; frame ≈ 320-340, after
  the blast tick):
  - **after (this branch)**: the cyan blast script's entities (Laser_Blast_frame_1/2, Vertical_smoke,
    After_glow_test, cloud_1, smoke [D]) at the floor impact, script scale 1.0; NO generic
    0x06000052 explosion beside it.
  - **before (master/hold)**: a generic 0x06000052 blast at scale 8 AND the scale-1 cyan player
    (or on pre-291 master, only the generic blast) — the visual tell is the wrong-script explosion
    playing beside the cyan stream.
- The same spot with a Sentinel rocket (0x2c) into the floor: the generic blast at scale = radius 8
  must still look like the game's rocket contact (the 0x4 arm keeps scale = radius).

## Live checks for the main agent

1. **PCSX2, 07000005, Samurai Overcharge bolt into a wall**: the cyan impact effect's size should
   read as the script's authored size (f12 = 1.0), NOT radius-scaled 8× — compare against the frag
   grenade's explosion (0x4/fuse arm, f12 = radius 8), which should look the larger scripted blast.
   This is the visible consequence of the corrected scale pin.
2. **gdbwatch/fieldprobe, `Explode_Create` 0x17cbd0 entry** (breakpoint): on a Samurai bolt world
   hit, check the f12/f13/f14 register values at entry = 1.0 / 8.0 / 256.0 [M 0x12d0ec-0x12d118];
   on a frag fuse death, 8.0 / 8.0 / 100.0 [M 0x12ce1c-0x12ce20]. Expected: matches the port's
   scale/radius/damage tuple.
3. **Destroy_Smash scale**: the game passes f12 = tag+0x8c [M 0x12bcdc] while the port keeps
   scale = radius there. A fieldprobe on a type-1 destroyable smash (07000003 placement #266)
   reading SCRIPTINFO+0x94 after the smash would settle whether the port's destroy blasts render
   at the game's scale (the green test pins the script, not the scale).

## What's left

- Nothing on the task's statement: the ONE path exists, both branches' tests + the Samurai gates are
  green, the suite is green (mod the allowed/pre-existing failures), the renders and live checks are
  listed above for the main agent.
- The destroy-path scale nuance (live check 3) is the only open evidence gap, cosmetic at worst.

## ntfy failed

All three notification fallbacks failed with the ntfy.sh daily publish quota (the topic feed shows
the message never landed — polled `https://ntfy.sh/nightfire-yoda-8k3x/json?poll=1` after each
attempt):

1. `curl -sS -m 20 -H "Title: NightfirePort" -d "fix-blast-scripts2: done …" https://ntfy.sh/nightfire-yoda-8k3x`
   → `{"code":42908,"http":429,"error":"limit reached: daily message quota reached; increase your
   limits with a paid plan, see https://ntfy.sh","link":"https://ntfy.sh/docs/publish/#limitations"}`
2. `python3 urllib.request` (same body/headers) → `urllib.error.HTTPError: HTTP Error 429: Too Many Requests`
3. `wget -q -O- --post-data "fix-blast-scripts2: done" https://ntfy.sh/nightfire-yoda-8k3x` → exit 0
   but silent; the topic feed poll shows no message from this task (only the other workers' entries),
   so the POST did not publish.
4. (extra) a second `curl` after a 30 s wait → the same 429 body.

The task state itself is complete: branch pushed (`s/fix-blast-scripts2`, tip `210ebdd5`), the report
committed on the branch, and `finish.sh 105c2-fix-blast-scripts2 done` succeeded (the report is
attached to `done/105c2-fix-blast-scripts2.md` on `origin/queue`). Only the ntfy ping is missing.
