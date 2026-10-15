# fix-blast-scripts — each blast plays its own effect script

Branch `s/fix-blast-scripts` (2026-10-14, ~19:16-20:5x UTC). Task 105c: the port's `spawn_blast`
rendered the generic one-shot **0x06000052** for every explosion, while the game passes a
**per-source effect script** to `Explode_Create` 0x17cbd0 (its `Script_Load` argument, register a3
at every call site). This report is the census, the fix, and the verification.

## The census — every `Explode_Create` 0x17cbd0 call site and its script

All from `work/decomp/ACTION.ELF` + `tools/crosscheck/mipsdis.py` on the call sites (the decompiler
garbles the mixed float/int args; the disasm pins a3 = script, f13 = radius → EXPLODEINFO+0x2C /
shake / script scale, f14 = damage → +0x28, t3 = the attacker, 0(sp) = the debris u16 [M, and
`Explode_Create`'s own prologue 0x17cbd0-0x17cc44 confirms the register mapping]):

| # | Call site | Disasm | Script (a3) | Radius / damage | Source |
|---|---|---|---|---|---|
| 1 | `Bullet_handle_object_destruction` 0x12cd98 (the fuse death) | 0x12cdf0-0x12ce2c | **0x6000052** | wd+8 / wd+0xc (`mov.s f13,f12`) | frag 0x34, mines, satchels |
| 2 | `Bullet_CollisionHandler` 0x12ce80 contact arm (`+0x70 & 4`) | 0x12d088-0x12d0c0 | **0x6000052** | wd+8 / wd+0xc (`mov.s f13,f12`) | MGL 'Grenade' 0x2a, the AIMS-20 0x1b, the rockets 0x2c-0x2f, 0x30/0x31/0x41/0x62/0x6d/0x6f [L the records] |
| 3 | `Bullet_CollisionHandler` bolt arm (`+0x70 & 0x100000`) | 0x12d0e8-0x12d124 | **0x60007c4** | 1.0 (f12) / wd+8 (f13) / wd+0xc (f14) | the Phoenix Samurai's bolt **0x32** (radius 8, damage 256) and 0x6e [L records] |
| 4 | `Bullet_CollisionHandler` bolt arm (`+0x70 & 0x200000`) | 0x12d14c-0x12d188 | **0x60007c4** | same shape | **0x6b** (radius 20, damage 20) [L records] |
| 5 | `Destroy_Smash` 0x12bb60, tag type 1 | 0x12bc70-0x12bc7c | **0x6000027** | (obj+0x8C)·4 / tag+8 | the monitors/TVs |
| 6 | `Destroy_Smash` 0x12bb60, tag type 2 | 0x12bc80-0x12bc8c | **0x600004f** | (obj+0x8C)·2 / tag+8 | (no story-level destroyable carries type 2 [D census] — MP demo pack's script) |
| 7 | `Destroy_Smash` 0x12bb60, other non-zero type | 0x12bc90-0x12bc98 | **0x6000052** | (obj+0x8C)·3 / tag+8 | the bottles/crates (types 3-10) |
| 8 | `Destroy_Smash` 0x12bb60, type 0 | 0x12bc50 `beq v1,zero` | **none** | — | 0700000d #470, 07000001 #116, 07000012 #771 [D] |
| 9 | `Copter_Update` 0x17dbb8 case 6 | :270 | 0x6000052 | 10/10 | the copter death — no copter list in the port (pre-existing gap) |
| 10 | `Bullet_DoTrails` 0x12d890 (`+0x70 & 0x20000` timer death) | :391 | 0x6000052 | wd+8 / wd+8 | the airburst timer arm — covered by the port's fuse routing |
| 11 | `Drone_Delete` 0x136ce8 | :92 | 0x6000052 | 5/50 | the 0x36 drone self-destruct — the port's drone delete path has no blast (pre-existing gap) |
| 12 | `GT_Create` 0x17ffa0 / `GT_Update` 0x180498 / `Car_Deactivate` 0x131d88 / `MP_DemolitionProtectionUpdate` 0x1870a0 / `Monitor_Update` 0x1afcc8 | :157/:435/:35/:144/:43 | 0x6000052 / 0x6000052 / 0x6000052 / **0x600004f** / **0x6000027** | — | MP grenade/car/MP demo/monitor — no MP/GT/car/monitor lists in the port (pre-existing gaps, documented in `explode.rs`'s 12-case dispatch note) |

Disc census (probe over every FILES.BIN archive's kind-7 NIS table): **0x06000052, 0x06000027,
0x0600004f and 0x060007c4 ship together in every story archive** (07000001-0d, 11-13, 24, 29, 41,
46, 4b) — any level can play any blast script [D].

## What was wrong in the port

1. **The Samurai's bolt never exploded.** The player-reachable Samurai is the beam twin **0x33** (a
   direct select lands 0x33 — 0x32's owned slot is its base [D census, +0x02]); the ○ mode switch
   lands the bolt record **0x32** (+0x70 = 0x100000 [L the ACTION.ELF records]). The port's only
   world-hit explode was the bit-2 (0x4) contact arm — 0x32 has no bit 2, so the bolt's hits played
   nothing. The game explodes every bolt hit with the cyan blast 0x060007c4 (arms 3/4 above), on
   world AND object contacts (the arms are common tail code of `Bullet_CollisionHandler`).
2. **The destroyables never exploded.** `Destroy_Smash` 0x12bb60's `Explode_Create` (rows 5-8) was
   unported — the port's destroy death only set `dead` + the channel (scene.rs's comment said so).
   The tag's type byte (tag+0x16 = placement param 2's low byte [M 0x12c3c8 disasm]) and the blast
   damage (tag+0x08 = placement param 5 [M 0x12c424 → tag+8; read as f13 at 0x12bd04]) weren't even
   parsed.
3. **The script id was hardcoded.** `spawn_blast` played 0x06000052 unconditionally (drone.rs:5711);
   the view pre-registered only 0x06000052's entity streams (scene.rs:4196).

The generic sites (the frag's fuse, the rocket/MGL contact) already passed 0x06000052 and stay on it
— rows 1/2 — they are pinned by the sister test.

## The fix

- `crates/nf-game/src/drone.rs` — `pending_blasts` carries the script (`Vec<(V3, f32, f32, usize,
  u16, u32)>`); `spawn_blast` takes `script: u32` and plays it. NEW: the bolt arms — a player bullet
  whose weapon `+0x70` carries 0x100000/0x200000 queues the 0x0600_07c4 blast at the hit point with
  radius = wd+8, damage = wd+0xc, on the world hit (the `else if` after the bit-2 arm, the same
  non-ricochet `b.bounces == 0` gate) and on drone hits (after the msg-8 send; direct damage AND the
  blast apply, as in the game). The destroyable smashes (`props.bullet_hit`'s return) queue the type
  script (1 → 0x0600_0027, 2 → 0x0600_004f, other non-zero → 0x0600_0052) with the tag's damage.
- `crates/nf-game/src/props.rs` (additive, listed per the shared-file rule) — `Destroy` gains
  `kind: u8` and `blast_damage: f32` on their own lines, parsed from the placement's params 2/5;
  `bullet_hit` returns the tick's blast-carrying smashes `(pos, radius, damage, kind)`. Kind 0
  returns nothing (no blast [M 0x12bc50]).
- `crates/nf-view/src/scene.rs` (small) — the blast-script pre-registration chains all four hashes
  (0x0600_0052, 0x0600_0027, 0x0600_004f, 0x0600_07c4) so every blast's entity streams get
  attachments.
- `crates/nf-game/src/explode.rs` — the EXPLODEINFO+0x34 doc line now records the per-source script.

No RNG draws added (the blast path draws nothing new); fixed 60 Hz; no wall clock.

## How it was verified

- **Reproduce before fix**: `crates/nf-view/tests/fix_blast_scripts_live.rs`, driven through the
  real input path (`nf_view::load`/`load_weapons` + `Sim::step_bond` with the trigger input). On the
  pre-fix tree (the fix stashed, the final test file unchanged): `the_samurai_bolt_plays_the_cyan_
  blast_script` FAILED (no blast at all — the `Explode_Create` expect), `the_destroyable_smash_
  plays_its_type_script` FAILED (no 0x0600027 player), the frag/rocket sister PASSED (the generic
  id was already right). Post-fix: **3/3 pass**.
  - The Samurai test: give 0x33 → ○ to 0x32 (the twin switch [M 0x1a24c8]) → fire into the floor →
    the blast radius 8.0 / damage 256.0 (the record's wd+8/wd+0xc [L]) and exactly one 0x060007c4
    one-shot player at scale 8.0, and NO 0x06000052 player (the old hardcoded one-shot).
  - The destroyable test: 07000003's destroy[0] (type 1, health 0) smashes on the first P2K contact
    and plays 0x0600027; the sister on 0700000d's destroy[10] (type 0) smashes and plays NOTHING
    [M 0x12bc50 `if (cVar4 != 0)`].
  - The frag/rocket sister: the frag's fuse blast and the Sentinel's contact blast still play
    0x06000052 at scale 8.0 and no cyan blast.
- **Full suite** on the final tree (rebased onto origin/master 62c17571): see "Suite" below.
- Unit tests unchanged: `props.rs`'s camera/door tests compile against the new `bullet_hit` return
  (the return value is additive).

- **Full suite** on the final tree (rebased onto origin/master 62c17571), `NF_ISO` set,
  `python3 tools/test/suite.py`: **build ok: 269 test binaries** — **1070 passed / 6 failed across
  269 binaries**. The 6, none from this task:
  - `disc_golden` x 3 — the allowed slim-ISO failures [D the task rules];
  - `probe_pylon_render` x 1 + `fix_0c_pylon_live` x 2 — the GPU-less sandbox's wgpu
    `request_adapter` panics (`probe_render_pylon_only` verified panicking at the adapter expect,
    line 25, before any scene logic — the main agent's machine renders these);
  - `weapon_matrix_live` — the suite's 600 s per-binary TIMEOUT, documented pre-existing in
    fix-0c-pylon.md ("TIMEOUT weapon_matrix_live after 600 s"), fix-armour-ring.md and
    fix-06-driver.md (the 27-level matrix scan is the tree's heaviest binary).

## Documented gaps (not this task's code)

- The blast-propagate smash (`Explode_Propagate` case 0x21 → `Destroy_Smash`) stays unported — the
  port's `spawn_blast` still passes `destroys: &[]` (pre-existing; explosions don't smash
  destroyables, only bullets do here).
- The smash radius reads the placed obj's `+0x8C` sphere in the game — a field the port doesn't
  parse; the stand-in is the bounds' half-diagonal × the type mult (4/2/3), and the position is the
  bounds centre (the game reads obj+0x80, the placed obj's position) [*suggested*, live check below].
- The placement param float decode: the game reinterprets negative ints as raw float bits
  (`Destroy_Create` 0x12c3d4/0x12c424); every destroyable on the disc carries small positive ints
  (10…3000 [D census]), so the port decodes only the positive branch.
- The twin-meter gap surfaced by the Samurai test: the game's per-tick clip refill gates cover
  clip[0x33] only [M Player_Update 0x193eb8-0x19405c] — the bolt twin 0x32's meter slot never
  refills in the port (the test tops the slots up directly). Whether the game shares one meter
  across the twins is a live check below.
- Rows 9-12 of the census (copter, drone self-destruct, car, MP/GT/monitor) have no object lists in
  the port yet — pre-existing, documented in `explode.rs`'s dispatch note.

## Live checks for the main agent

PCSX2 (`tools/trace/fieldprobe.py` / `gdbwatch.py`):
1. **The Samurai bolt's cyan blast**: any level with the Samurai (the bank's Phoenix Samurai 0x33);
   fire, ○ to the bolt, shoot the floor/wall. Expected: the cyan blast plays at EVERY hit (not only
   kills) — and gdbwatch on `Explode_Create` 0x17cbd0's a3 reads 0x060007c4 at the hit.
2. **The bolt's meter**: after the ○ switch, does firing drain the SAME meter value the beam used
   (BL+0x41C)? Watch BU clip[0x32] vs clip[0x33] — settles the twin-meter gap above.
3. **A bottle/crate smash** (07000001's bar bottles): shoot one — the blast script id read at
   `Explode_Create`'s a3 (expect 0x0600027 for the type-1 tags), the blast radius vs the bottle's
   size (settles the obj+0x8C sphere stand-in), and the blast position vs the bottle's centre.
4. **The type-0 tag**: 0700000d's #470 (if reachable) — a smash with NO blast at all.

nf-view renders (`--bond x,y,z,yaw,pitch --skip-cutscenes --frame N --shot x.png`) — the CRT-filter
caveat applies to the PCSX2 captures, not these:
1. 07000005, Bond at the start, Samurai bolt out (`--bond` at the spawn, pitch slightly down), frame
   at the shot's contact: BEFORE = no blast; AFTER = the cyan blast at the floor point.
2. 07000003 near [28.0, 6.0, 3.5] facing the destroy[0] crate, frame at the smash: BEFORE = the
   crate vanishes silently; AFTER = the 0x0600027 blast at the crate.
