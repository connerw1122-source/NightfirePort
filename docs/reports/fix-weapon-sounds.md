# fix-weapon-sounds — every weapon's sounds on every level

Branch `s/fix-weapon-sounds` (from `origin/master` 7be3272), task 100b of the four 100x
weapon tasks. Status: **core fix done and tested**; two tests parked on 100a/100c
dependencies; the full 211-binary suite still needs a run on the final tree (see
"Left for the next message").

## The bug

The dev tools (`--give`, the F2 menu) hand Bond any weapon on any level, but each level
loads exactly one sound bank — `Sound_Ready` 0x1dceb8 runs one `SFXLoadSoundBank` per
level (07000005 → bank hash 5 → slot 4; docs/systems/sound.md §3 Init, docs/formats/
sound-bank.md) — and the port's driver resolved every sound id from that one bank. A
weapon the level doesn't carry has fire/reload ids the bank has no row for, so
`SFXStart3D` returned 1 and nothing played. Measured on the disc banks [D,
nf_formats::soundbank over `PS2/ENGLISH/SB_<slot>/SB_<slot>.{SFX,SHF,SBF}` +
`PS2/SBINFO.SBI`]: on 07000005 (slot 4) the missing weapon-sound sets are

| weapons | missing ids (script op-1 events) | held by slots |
|---|---|---|
| 0x0e–0x10 | 0x1e1, 0x1e2, 0x1e3 | 2,10,11,12,13,16,21–24 |
| 0x11 | 0x449–0x44c | 2,7,8,9,14,15,16 |
| 0x12/0x13 | 3, 0x54–0x57 | 2,10–16 |
| 0x1a/0x1b (the AR and its GL) | 0x405–0x40d | 2,7,8,9,16 |
| 0x1c/0x1d | 0x1dd, 0x1de, 0x1e0 | 2,10–13,16,21–24 |
| 0x2a/0x2b | 0x404, 0x40b, 0x410 | 2,16,21–24 |
| 0x2c–0x31 (the AT-420 Sentinel, AT-600 Scorpion and Torpedo Launcher pairs; corrected by the main agent) | 0x10c, 0x10d | 2,6,16,17,20,22–24 |
| 0x32/0x33 | 0x420–0x423, 0x5c7 (looped sample) | 2,7,8,9,16,17 |
| 0x37 | 0x458 | 2,7–13,16,18,19,20 |
| 0x3a (Laser Tripbomb) | 0x445, 0x446 | 2,7,8,9,16,21–24 |
| 0x3b–0x40 | 0x447, 0x448 | 2,7–13,16,18–24 |
| 0x41 | 0x27f | **2 only** |
| 0x42 | 0x41b, 0x5c9 | 2,16 |
| 0x43/0x44 | 0x5ce–0x5d0 | 16,18,19,20 |
| 0x52 | 0x44e, 0x450 | 2,8,14,15,16,22–24 |
| 0x56–0x59 | 0x452, 0x454, 0x456, 0x458, 0x5cc–0x5d0 | 7,9,16,18–20 |
| remap targets / hardcoded | 0x602 (0x10's remap), 0x10e (bullet class 0xe), 0x5ef (0x45), 0x27f (0x41) | 2 everywhere |

Slot 2 (hash 22 — the multiplayer bank, levels 21–29/4b/4c [M `Sound_Ready`'s table])
holds every missing id: it is the weapons superset. The draw/holster ids 0x280–0x283,
0x1db, 0xec, the P2K's 0x403/0x40e/0x40f/0x411 and the loops 0x12 (the Laser 0x4e) and
0x3d (the Stunner 0x4a/0x4c loop; the main agent's correction: 0x4a is the Stunner, not the Q-Claw, per fix-laser-beam; SHF loop-flagged rows [D bank 4])
are in every (or most) banks.

## The fix

- `crates/nf-audio/src/sfx.rs` — the driver keeps the first `load_bank` as the
  **primary** slot (the level's own, `Sound_Ready`'s single load [M 0x1dceb8]); new
  `load_spare_bank` registers the other SBINFO slots; `start` searches the primary
  first, then the spares by ascending slot (a stable, game-derived order; slot 2 wins
  the ties). Every id the level's own bank carries resolves exactly as before — the
  per-level behaviour is bit-identical for the level's own set (the sister test pins
  the P2K's sounds to slot 4 with the bank's own sample bytes). The spare's own effect
  record rides along, so the radii, `MaxVoices` and priority are the **owning bank's**,
  never the level's; the voice limits are untouched. Test probes: `item_slot`,
  `played_sample_hash` (FNV-1a of the started voice's decoded PCM — numbers only),
  `ids`, `pcm_hash`.
- `crates/nf-view/src/lib.rs` — `load_impl` loads every SBINFO slot with files on the
  disc (22 banks, 42 MB of SBF total [D]) into `scene.sound_spares`; `build_sound`
  (now `pub`, so the live tests drive the exact runtime builder) feeds them to
  `load_spare_bank`. Only ids the primary lacks ever reach a spare.
- `crates/nf-view/src/scene.rs` — one additive field `sound_spares` (+ constructor
  init). No other shared-file changes (drone.rs/player.rs untouched).

## Verification (reproduce-first)

`crates/nf-view/tests/fix_weapon_sounds_live.rs` — through the real path:
`nf_view::load_dev` (the dev view-model merge gives foreign weapons their scripts, so
their op-1 sound events fire) → `Sim::step_bond` (equip/draw/fire/reload input) →
`Sounds::update` → `nf_view::sound_tick` → the driver.

- **RED first**: with the lookup neutralised to the one-bank behaviour,
  `a_foreign_weapons_fire_and_reload_sounds_start_voices_on_07000005` fails with
  *"the AR's fire sound 0x406 never started a voice on 07000005"* (the id later
  corrected to 0x40d — the AR's fire script 0x060007b6 op-1 @frame 1; 0x406/0x407/0x40a
  are its reload stages @48/89/132).
- GREEN: the AR's fire 0x40d and reload 0x406/0x407/0x40a start from the owning spare
  bank (slot 2) with that bank's sample bytes (`played_sample_hash` ∈ the owning
  bank's decode set); the draw 0x282 still comes from slot 4.
- Sister: `a_native_weapons_sounds_come_from_the_levels_own_bank` — the P2K's
  0x403/0x40e/0x40f/0x411 all resolve at slot 4 with the level bank's decodes.
- Sweep: `every_weapon_on_07000005_draws_fires_and_reloads_with_sounds` — 69 weapons
  giveable (the dev merge's baseline; the rest have no FP model — 100a), each drawn,
  fired and (where the level bank lacks the reload ids) reloaded; every requested id
  starts a voice with the owning bank's sample. The gadget-context ids (0x52's reel
  0x44e/0x450, the camera gadgets 0x56/0x58 — their scripts only run inside their own
  fire minigame) are asserted at the driver level.
- nf-audio unit tests + `fix_sound_remainder_live` green with the change.

**2 tests `#[ignore]`d with reasons in their comments** (not guess-fixed):
- `the_laser_loop_starts_on_press_and_stops_on_release` — the Laser (0x4e) has no
  first-person model in the dev merge yet ("24 weapon-model hashes still missing" at
  load), so `equip_weapon` refuses. The loop id 0x12 is in bank 5 and the op-4.3 stop
  (`Sound_StopAllWithId`, 0x1c1b38:388) is already ported; re-enable with 100a's assets.
- `a_foreign_weapon_fires_with_sounds_on_the_indoor_level_07000001` — on 07000001 the
  AR draws fine (31 ticks: weapon 0x1a, clip 30, idle script 0x060007b8) but held fire
  spawns no round (clip stays 30 across 30 held ticks), so the op-1 fire sound is never
  requested. The same equip+fire on 07000005 fires immediately. The draw/holster sounds
  resolve from slot 10 — not a bank issue; the foreign weapon's fire path on that level
  is 100c's (or the fire script is outside 07000001's merged set — 100a).

## Left for the next message

1. **The full suite on the final tree** (`NF_ISO=... python3 tools/test/suite.py`): this
   session's sandbox killed background jobs, and the 2-hour window closed before the
   211-binary run; nf-audio's three bins and the new test file ran green. Run it after
   merging `origin/master`, report the total line.
2. Merge `origin/s/weapon-assets` (100a) when it lands — it un-blocks the two ignored
   tests (the Laser's FP model; possibly the 07000001 fire script) and raises the 69
   giveable baseline. 101e's laser-loop branch: not on origin yet.
3. `pss_movies_live`-style opt-outs: none needed; the spares load is ~42 MB per
   `nf_view::load` call — watch the suite's TIMEOUT budget if a slow machine batches.

## Live checks for the main agent

nf-view renders (needs the merged assets; sounds are audible in the window build):
1. `--bond 104.95,25.12,108.83,2.0 --give 0x1a` (07000005): the AR draws with 0x282 and
   fires with 0x40d — the shot sound plays (pre-fix: silent shots).
2. `--bond ... --give 0x2c` (the Sentinel on 07000005): the rocket launch 0x10c and the
   reload 0x10d play.
3. `--give 0x41` on 07000005: its 0x27f (slot 2 only) plays.
4. PCSX2 spot check (the real game, one bank per level): the AR is NOT giveable there —
   the check is only that the game's own AR level (07000002+) still sounds identical,
   i.e. the port's primary-first lookup never disturbs a level's own set.
