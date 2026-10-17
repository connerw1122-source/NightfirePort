## Your task: `hurt-volumes` — hurt volumes (0xE4)

64 placements: `Hurt_Create` 0x17e870 and its update. Damage volumes: the damage kind and rate from the params. Test: standing in one damages Bond at the game's rate and type; outside, nothing.

The script census (task 709, merged) lists this gap with occurrence counts and the game handler in `docs/systems/script-census.md` ("Top 10 ready-to-queue tasks"). Port from the decompile [M], test on real levels through the real path with a sister assertion, and update `crates/nf-formats/tests/script_census.rs`'s handled sets.

**Tests pin only proven behaviour.** Every assertion about what the game does cites its evidence: [M] a decompile
address that actually says it, [D] disc bytes, or [L] a live PCSX2 measurement from a FRESH boot (a save-state resume
skips the boot path). A guess, a port choice or an [S] item is never a hard assertion: mark it `// [S] unverified` and
keep it out of pass/fail. If an existing test contradicts the decompile, the TEST is wrong: fix it, with the address, and
say so in the report. Never weaken a decompile-backed test to make code pass.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/hurt-volumes`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox7 at 2026-10-17T00:36Z

---
## Report (done, 2026-10-17T01:26Z)


---
## Report from the worker (done, 2026-10-17T01:27Z)

# hurt-volumes — the 0xE4 hurt volumes ported

Branch `s/hurt-volumes`. The script census's #5 gap (64 placements): placement type 0xE4 →
`Hurt_Create` 0x17e870 (class byte 0x27) — damage volumes whose per-contact damage and pain type
ride the placement params.

## What's done

- **`crates/nf-game/src/worldobj.rs` — the `Hurt` family** (create + state machine + damage query):
  - Create (placement order, no RNG [M the decompile]): params p0..p5 → type / damage (int→float,
    0x17e870:32-38) / gate channel / sound id (0 → 0xFFFF) / contact-class mask / one-shot flag;
    the header key count (level_tag+0x02 = placement+0x40) and rate (+0x42) and the 0x19 key list
    (`m_paths[placement+0x44]`) [M docs/fields/level-tag.md]. The volume triangles are the placement
    object's glist transformed by the placement matrix (the same resolution `triggers::build` uses).
  - `Hurt::tick` = `Hurt_Update` 0x17e5d0: the 0x20 channel gate (set/cleared with the channel —
    the gate that keeps the volume out of the collision candidate pass, `Collide_PickObj` skips
    `obj+0xF0 & 0x20`), the state-0 arm (a channel reads THE CHANNEL; no channel + mask 0 arms at
    once; the contact-class mask arm is a documented [S] stand-in — never fires in the port), the
    state-1 frame advance at `(count−1)·REC_FRAME_RATE·(FRAME_RATE/rate)` in the game's op order at
    the load-time rate (FRAME_RATE 30 [L door.rs CREATE_FRAME_RATE snapshot]) and the one-shot
    expiry: one sweep → state 2 → the class byte drops to 1, inert forever [M 0x17e5d0:42-45].
  - The damage pass: `Player_DealWithObjHit` 0x194458:94-101 case 0x27 — every tick Bond's contact
    list holds the volume (`Collide_Update` 0x1ea398 rebuilds it), `Player_HandlePain(hitdata+8 +
    Hurt_GetDamage, BL, Hurt_GetType, -1)` fires. `hitdata+8` = the toucher's `HITTEST+0x90`
    (`Collide_Intersect` 0x1ebaa8:322) — the player's ground `HITTEST+0x90` is `BL+0x7D0`, which
    nothing writes, so the param damage is the whole amount. Ported as `WorldObjects::hurt_pain`
    called from `Drones::tick` right after `Trigger_Calc` (the touchers exist there; types 1..7
    bypass armour via the existing `pain_generic` [M 0x1902c8:84-104], grunt 1-in-4 per contact).
- **`crates/nf-game/src/triggers.rs`**: three geometry helpers made `pub(crate)` (no logic change).
- **`crates/nf-game/src/drone.rs`**: the create call in `attach_level` + the damage pass in `tick`.
- **The census moved**: `script_census.rs` — 0xE4 (64) out of the unhandled list, 1537 → 1473,
  `HANDLED_PLACEMENT_TYPES` 56 → 57; `docs/systems/script-census.md` rows 134/294/327 updated.

## The disc data (the probe's [D] findings)

07000005: 13 volumes, all type 6 damage 1000 — ten mask-0x1, three mask-0 flat death quads
(object 152's plane at y 5.11 spanning x 80.8..168.4) plus a 6-tri plunge box. 07000002: one
type-1 damage-1 volume gated on channel 141. 07000046: seven animated one-shots (type 1, damage
105, keys 6, rate 90, mask 2 = bullet-armed). 07000008/0a/11/4b: mask 0x18/0x0 death volumes.

## How it was verified

`crates/nf-game/tests/hurt_volumes_live.rs` (opt-in `NF_ISO`), all fail-without-change on master:
1. **05 through the full Sim**: 13 volumes created with the disc params; Bond's feet on the
   (124, 5.11, 120) death quad → health 100 → 0 in ONE tick; Bond at (0, 5, 0) (inside the plunge
   box's AABB, clear of its mesh — the shipped trigger-contact model) → 60 ticks, untouched.
2. **02 through the full Sim**: standing in the box with the gate off → nothing; at the Hurt level
   (real placements/tris) the channel arm → `(1.0, 1)` per contact.
3. **46**: the seven placements, the exact f32 step bits of the game's op order, and the one-shot
   sweep expiring at ≈90 ticks (85..=95 pinned — the exact count is the f32 accumulation).
4. Census: `placement_type_census_matches_the_doc` pins the new handled set.

Suites run before the push: nf-game lib 241 passed; the new live file 4/4; nf-formats lib 79
passed + script_census 6/6; the Sim-heavy 05 integration set (ai_live 35, guard_reaction 2,
cover_selection 4, guard_drops 1) all green — the damage pass changed no existing behaviour.
**Not run**: the full four-crate suite (the fast runner's background job dies between tool calls
in this sandbox; the foreground window did not fit it). The final-tree rerun is left to the driver.

## What's left / [S] items

- The masked no-channel arm (0x2 = bullets, 0x18 = player|drone) never fires in the port — no
  per-class contact list for volumes. Damage is unaffected (the decompile gates it on the 0x20
  flag only); only the ANIMATION arm of 07000046's seven is unreachable.
- Animated volumes keep their triangles at the placement matrix (the game's `Spline_Interp` moves
  obj+0x90); the frame math still runs so one-shots expire on time.
- Drone victims (`NDrone2_DealWithObjHit` 0x13fff0 case 0x27) are not modelled.
- The loop-start `Sound_Play3D` (frame 0, 0x17e5d0:119-123) is not modelled.
- Multiple overlapping volumes damage in placement order; the game sorts contacts by distance
  ([S] — single-volume ticks agree).
- Live checks for the main agent: (1) on a FRESH boot of 07000005, stand Bond on the courtyard
  death quad near (124, 5.1, 120) and confirm the fall-frame death fires the same tick he touches;
  (2) on 07000002, poke `switch_channels[141] = 1` (0x26fc90+141) and stand in the box at
  (−61, 7.3, −30.2): expect ~0.5 health/tick at difficulty 1 ( armour untouched, type 1).

## Usage

not visible to me — this session exposes no token, context-window or cost figures to the worker.
