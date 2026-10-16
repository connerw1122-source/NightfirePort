# sg5-semi-laser — the SG5's semi-auto mode lights its laser sight

Playtest 1015-1738: "the SG5, when you turn on semi-auto, it's supposed to have a laser ...
turn on, and it doesn't." Branch `s/sg5-semi-laser` (from origin/master aa3bc1f5; master did
not move during the session, so the final tree is the branch tip).

## What the game does (all [M] unless tagged)

- **The mode gate is a weapon-record bit.** `Player_SetWeaponAnim` 0x1a9338:21-26 — run once per
  completed weapon draw/switch (`Player_SetWeaponAnimObj` 0x1a2e38 state 3, via
  `Player_WeaponRecoil` 0x1a3de8:59 ← `Player_Update` 0x193600) — rebuilds the first-person
  model and latches `BLData+0x964 = (weapon_data[w]+0x68 & 0x8) != 0`.
- **[D]** The disc's table (`weapon_data` 0x2bf150, 0x10c-byte records, built by
  `_GLOBAL_$I$weapon_data`, dumped through the port's own ELF machine): the SG5's **semi twin
  0x17 has `+0x68 = 0x00000088`** (bit 0x8 set) and the **full-auto 0x16 has 0x00000080** (no
  bit). The ○ twin swap 0x16 ↔ 0x17 (`Player_Weapon` 0x1a24c8:246-260) therefore lights the
  sight exactly when semi is equipped — the playtest's expectation.
- **The beam** — `Player_LaserPointer` 0x1a65d8, called every frame from
  `Player_CollisionHandler` 0x1949a8:188 (after `Player_PositionGun` and `AnimObjectUpdate`, so
  the bone pose is current):
  - gates: `BLData+0x964` (l. 93); the bit on the record OR its `+0x05` twin (ll. 95-99); the
    beam/dot sprite objects existing (`Player_InitWeapon` 0x1a7ef8 creates the 32-sprite array
    at `BLData+0x7ec` and the dot at `BLData+0x86c`);
  - start: `AnimGetBoneWorldTrans(BLData+0x7e8, 4, 0)` — the first-person model's **bone 4**
    (the sight datum), world pos + matrix (ll. 104-110);
  - ray: from the **head** (`Player_GetHeadPos` 0x195aa0) to `datum + bone4_Z·100` (the local
    (0,0,100) through the bone matrix, ll. 112-127);
  - `Collide_RayIntersect` 0x1ec690 with the trailing u32 = 8 — the bit-0x8 query family that
    DROPS pass-flagged (foliage) hits (disasm 0x1ebff0-0x1ec010;
    docs/reports/foliage-bullets.md);
  - end: clamped to the hit (closest-point-on-line, ll. 129-141) or the far end when nothing is
    hit; the red dot (`hashtable_set_object_to_entity_gfx` 0x2000132) sits at
    `hit + normal·0.01`, aligned to the normal, scale `1 + dist²/MAX_VIEW_CONE` capped at 4
    (ll. 189-205). `MAX_VIEW_CONE = 2000.0` **[L]** action-symtab.tsv 0x30d0fc (`.sdata` bytes
    00 00 fa 44).
  - `Player_WeaponFiring` 0x1a5460:88-98 hides the sprites every frame before LaserPointer
    re-shows them — a frame without the re-show is the hidden state. While firing (view state 9)
    the hider doesn't run and `Player_MuzzleFlash` 0x1a4298:131-132 redraws the beam from bone 0
    to the shot's trace end when `weapon_data+0x6C & 0x100`.
  - draw-side cosmetics (not sim): beam gfx 0x2000133, or 0x2000131 under night sight
    (`glb_viewer[v]+0x236 == 1`, the vision-mode field); flicker via `Rand_FRand` on the size
    byte; the beam is view-only while `BLData+0x950` (camera mode) is 0 (first person), the dot
    draws in all views.

## What the port does now

- `WeaponData::has_laser_sight` (nf-formats `weapon.rs`) — the `+0x68 & 0x8` accessor with the
  disc values in the doc.
- `Gun::laser_on` (nf-game `gun.rs`) — the `BLData+0x964` latch (new field, own line). Set at
  **every** weapon-commit site in `Sim` (nf-view `scene.rs`, the four
  `gun.weapon = gun.switch_request` points — instant variant, instant no-holster, holster-done,
  retarget), which is where the game's `Player_SetWeaponAnim` runs on the completed draw.
- The per-tick beam at the end of `Sim::run` (after `hud_tick`, so the pose is current): gate
  `laser_on && !disabled`; datum = `view_datum_matrix(4)` (the `AnimGetBoneWorldTrans` port);
  far end = datum + row-2·100 (the port's Mat4 rows are the axes — same convention as
  `tracer_matrix`); trace = `world.segment_surface_bullet(head, far)` — the bit-0x8 drop-pass
  query; end = the hit fraction's point (the game's closest-point clamp reduces to the hit);
  dot = `hit + normal·0.01` with the query's unit normal.
- **Getter for the draw code** (the main agent draws the beam):
  `Sim::laser_sight() -> Option<(start, end, Option<(dot_pos, normal)>)>` — the field
  `Sim::laser_sight` carries the same tuple; `None` = sight off (no bit / no FP datum yet /
  disabled). Draw notes: beam mesh 0x2000133 (0x2000131 under night sight), dot 0x2000132 red,
  scale `1 + d²/2000` cap 4 with d = dist(datum, end), beam view-only in first person.

## Tests (all through the real path)

- `crates/nf-formats/tests/weapon_live.rs::the_sg5_semi_twin_carries_the_laser_sight_bit` [D]:
  0x17 set, 0x16 clear, P2K 0x02/0x03 clear (the accessor reads the record's own bit — the
  OR-with-twin lives in LaserPointer only).
- `crates/nf-view/tests/fix_sg5_laser_live.rs` (07000005 + the dev give + `Sim::step_bond`):
  - `semi_auto_lights_the_sight_and_the_beam_ends_on_a_surface`: full-auto 0x16 → latch off, no
    beam; one `mode_switch` tick → weapon 0x17, latch on; Bond pitched 45° down, the beam starts
    at the bone-4 datum (1.45 from the eye on the measured tick), the end re-traces back onto
    itself through `segment_surface_bullet` (t = 0.50, the floor clamps the 100-unit ray), the
    dot sits at end + normal·0.01 with a unit up normal; a second ○ press → 0x16, latch off, no
    beam (the playtest's full-auto sister).
  - `the_latch_follows_every_committed_draw_not_just_the_switch`: the fresh P2K sim (0x02,
    `+0x68` = 0x1004 [D]) never beams before any draw commits; the 0x16 give commit latches off.

## What I ran

- The crates the tests touch, before the push: `nf-formats --test weapon_live` (2 passed) and
  `nf-view --test fix_sg5_laser_live` (2 passed).
- The **full suite on the final tree** (master unchanged; `tools/test/suite.py`'s semantics via
  a resumable wrapper because background jobs are killed between tool calls in this sandbox —
  same build command `cargo test --offline --release -j4 --no-run --message-format=json` over
  the four crates, each binary `--test-threads=1` with the suite's 600 s limit, NF_SILENT=1):
  **1781 passed, 6 failed, 72 ignored** (404 test binaries).
  - The 3 `disc_golden` failures are the slim-ISO expected set (task rule 5).
  - `probe_pylon_render::probe_render_pylon_only` and both `fix_0c_pylon_live` tests fail on
    `wgpu request_adapter: NotFound` — this sandbox has no GPU; they are render probes, they
    pass on the main agent's machine and touch nothing this task changed (nf-view lib compiled
    clean; the frame-cost binary passed at 124 s).
  - One stale line in my first batch log (`TIMEOUT levels_sweep_live after 120 s`) was my
    wrapper's initial per-binary cap; re-run at the suite's 600 s it passed (3 tests, 128 s).
- Related regressions green in the same run: `fix_sg5_casings_live`, `fix_auto_switch_live`,
  `fix_aims20_modes_lights_live`, `frame_cost_live`.

## Live checks for the main agent

1. **The visual** (no GPU here): with the SG5 out, press ○ (semi) — a thin red beam should run
   from the gun's sight datum to the aim point with a small red dot on the surface; press ○
   again (full-auto) — both vanish. Suggested spots: `--bond` standing 3–4 m from a wall aiming
   at it (semi: beam + dot on the wall; full-auto: nothing), and one aiming at the sky (semi:
   beam to the 100-unit far end, no dot).
2. **Bone 4 is the right datum [S→verify]**: the port's beam starts at the FP model's datum 4;
   if the in-game beam looks offset from the gun's sight housing, check
   `AnimGetBoneWorldTrans(+0x7e8, 4, 0)`'s pos against the drawn model in PCSX2
   (tools/trace/fieldprobe.py on the beam object at `BLData+0x7ec[0]`'s obj+0x30 during play —
   expect it at the sight, not the muzzle).
3. **Firing-frame beam [M, unported]**: while firing semi, the game redraws the beam from bone 0
   to the shot's trace end (`Player_MuzzleFlash` 0x1a4298:131-132, `weapon_data+0x6C & 0x100`).
   The port's getter currently serves the held beam only (the draw's own shot-tracer covers the
   look); if the merge wants the exact firing-frame behaviour, feed the shot's trace end into
   the draw on shot ticks.

## What's left

- The firing-frame beam above (small, draw-side; noted as a live check).
- The anim-script laser toggle (`AnimProcessScriptCmds` 0x1c1b38 case 1 flips `BLData+0x964`)
  is not ported — no scripted user of that command is known; the latch is faithful for the
  draw path, which is what the SG5 uses.
