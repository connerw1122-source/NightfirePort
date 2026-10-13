# fix-sound-remainder — the sound remainder (GAPS G-032)

Branch `s/fix-sound-remainder`, cut from origin/master 3d33a27, 2026-10-13. Session budget ~2 h; this
report is the driver's finish marker. Items 1 and 3 of G-032 are landed; 2 is researched to the
register level and needs the map-parse half; the rest of 4 is scoped below.

## Done: G-032d, the SPU2 Gaussian interpolation (`crates/nf-audio/src/lib.rs`)

The 4-tap Catmull-Rom stand-in is replaced with the SPU's 4-point Gaussian interpolation: the
512×16-bit coefficient table and the formula from the public nocash PSX-SPX documentation,
"SPU ADPCM Sample Decoding → 4-Point Gaussian Interpolation" (problemkaputt.de/psx-spx.htm;
the SPU2 pitch counter's bit 4–11 is the 8-bit index the same doc describes):

- `GAUSS: [i16; 512]` — transcribed from the doc's table and pinned by its own cross-check: each four
  taps (gauss[i], gauss[0xFF−i], gauss[0x100+i], gauss[0x1FF−i]) should sum to 0x8000 but hardware
  sums to 0x7F7F..0x7F81 — all 256 quads of the transcribed table land in that window
  (`gauss_quads_sum_near_0x8000`), so a corrupted or mirrored table fails the test.
- `resample_gauss(data, pos)` — `out = (gauss[0xFF−i]·oldest + gauss[0x1FF−i]·older + gauss[0x100+i]·old
  + gauss[i]·new) SAR 15`, each product shifted separately, `i` = floor(frac·256); taps before the
  sample start read 0 (the SPU's ring buffer starts zeroed, so a voice ramps in).
- The mix loop calls `resample_gauss` in place of the Catmull-Rom kernel.

**Sample-exact test on one VAG** (`one_vag_resampled_sample_exact`, no ISO needed): the PCM
[1000, −2000, 3000, −4000, 5000, −6000, 7000] at pitch 0x1000 must emit exactly
[148, 401, −811, 1212, −1621, 2023, −2432] at positions 1..7, plus 0x2000 → [401, 1212, 2023] and the
fraction index i moving the output (pos 1.25/1.5/1.75 → 299/460/530). The expectations were computed
by an independent Python implementation of the same doc formula, not by the Rust code. Fail-first:
the replaced Catmull-Rom kernel gives −2000 at pos 1.0 (vs 148) and 750 at pos 1.5 (vs 460) on the
same signal — every embedded expectation fails against the pre-fix tree.

## Done: pause in cut-scenes (`PauseInNis`) + the precache (`crates/nf-audio/src/sfx.rs`)

Both from sound.md §8 / §3.2 against `SFX.IRX`:

- **`ES_HandleNisPauseSfx`**: `Driver::set_nis_playing` (the IOP's `NisPlaying`, RPC op 0x19); each
  `SFXUpdate`, a sound whose `SFXParameters+0x3F PauseInNis` (already parsed by `Bank::parse`) is set
  and an NIS is playing gets pitch 0 on its live voices and is **skipped** by the update — no counter,
  no 3-D recompute, no end test — exactly the game's paused-item behaviour; the unpause frame falls
  back through the normal path, which re-sends the stored pitch. *Deviation*: the port has no global
  `SFXPause`, so the "unless globally paused" branch is not modelled.
- **`SFXPreCache(id, frames)`** (RPC cmd 0x26, IOP 0x39f4): `SFXKeyOnFrameDelay = frames + 5`, then a
  start. Only 1-sample **streams** can be precached (`SFXStart3D` step 3); a start inside the window
  takes the same-id item over instead of starting a second one; `HasSampleEnded` reports 0 while the
  countdown runs (`sample_ended`). *Deviation*: the game deletes the silent pre-setup at countdown 0 —
  the port's SPU stand-in has no CD latency, so the voice keyed on at once and expiry just closes the
  window (the same reason fix-audio2 skipped the op-1 warm-up).
- Tests (`crates/nf-audio/tests/fix_sound_remainder_live.rs`, NF_ISO, real SB_4.SFX):
  `pause_in_nis_pauses_while_the_cutscene_plays` (a real +0x3F effect gets the pitch-0 Set; a plain
  effect's sister does not) and `precache_streams_only_and_dedups_the_real_start` (id 175, the
  STREAMS.BIN line bank_live pins: one item, the real start dedups, past the window a new item).

## Done: drone foley — the door line (`crates/nf-game/src/drone.rs`, additive)

`NDrone2_DSTATE_OpenDoor` case 0x20 plays `Drone_SfxInfo[0x27]` before the door activation
(`NDrone2_PlaySFX(1.0, DCVars, 0x27, drone+0x30, enforceDelay 1)`) [M 0x1681f8]. The port now does the
same at its `pending_door_msg` site: `Drones::foley` / `foley_entry` implement `NDrone2_PlaySFX`'s
gates over the voice table the port already parses (`Drone_SfxInfo` @0x2812d0, 32-byte entries): the
30-frame post-load gate, the entry's `repeatDelay` against the table's shared `lastFrame` — stamped
**before** the play (rate-limited even when the play fails) [M 0x141848 disasm 0x141a1c] — and a
zero-sfxId entry returns before any stamp [M 0x141848: 0x1418ac]. Unit test
`foley_gate_repeat_delay_and_stamp_before_play` (all three gates + the stamp order + the silent 0x23
entry, whose sfxId is 0 in the snapshot [L]). The existing `fix_npc_doors_live` walk exercises the
wired door site end-to-end. Shared files: `drone.rs` only (one call site + two private methods).

**Research note that reshapes item 1**: `NDrone2_PlaySFX` is *not* the footsteps path. No caller plays
footsteps through it [M: the caller list is exhaustive — grep over all of `work/decomp/ACTION.ELF/`].
Drone footsteps are the NPC branch of `AnimObjectUpdate` @0x1bf185: anim op-2 sets the foot, then
`Effect_Create(1.0, …, 0x42 | foot)` → `Effect_Player` @0x1cec48 plays `EffectInfo[surface].u16 +0x16`
with the surface byte from `Drone_FeetOnPoint`'s floor raycast — **the port already has this**
(`Drones::tick` footstep → `SurfaceSounds::get` by material), and drone cloth/equipment sounds are raw
op-1 ids in the anim scripts (`Drones::script_commands` plays them). What the port was missing from
the foley family was the `Drone_SfxInfo`-gated one-shot lines; the door line is landed, the rest are
listed below.

## What's left (with the evidence already gathered)

- **Room environment (G-016/G-032 audio half)** — fully mapped, not landed: `Sound_UpdateListeners`
  @0x1dd278 sends `SFXSetEnvironment(cel s16 +0x98, underwater = (cel+0x3c & 0x10000000) || env ≥ 20)`
  from the active viewer's cel (glb_viewer[4] wins when its +0x29 is set) [M, disasm 0x1dd4b0–0x1dd4fc].
  `cel+0x98` is written by `parseentity_fixup_entity` @0x1d00c0: class 0xc023 → 0x9898 (the −26472
  underwater sentinel; the IOP turns it into `SFXUnderWater=1`), class 0xc047 → level_tag+0x50. The
  IOP driver side is already ported (WORKLOG 0075). What's missing is the port's map parse keeping the
  cel env value + the viewer's current cel, then `sound_tick` (nf-view lib.rs, the `drv.set_environment(0, 0)`
  stand-in) feeding the real value. Snapshot cross-check for 07000005: cels with env 0 (0x0804c047) and
  env 40 (0x1804c047 — the underwater bit set) [L].
- **Drone foley call sites** on `Drones::foley` (the engine is landed): kick 0x28 on the kick anim's
  msg 0x20 [M 0x1684c0], interrogation 0x4a (case 3, opponent armed) [M 0x161a40], civilian patrol 0x48
  (female only) [M 0x15d610], hostage-die 10 [M 0x1725a0], spawner 0xd6+Rand(2) (DCVars NULL → ref 0) [M
  0x137e68], and `BOT_handlePain`'s grunt pools 0x14d+Rand(5) / 0x152 / 0x154+Rand(3) / 0x157+Rand(3)
  [M 0x125ed8] — the pain pools consume the shared `Rand_*` stream, so they need the deliberate
  seed-pinned re-pick pass (WORKLOG 0072 procedure) and did not fit this session.
- **op-1 per-weapon id remaps** (the view-model path): pinned from `AnimProcessScriptCmds` case 1 [M
  0x1c1b38 disasm 0x1c2000–0x1c2100] — 1→2 (weapon 7/9), 0x403→2 (3/5), 0xfa→0x1fb (0x14), 0x106→0x284
  (0x1e..0x23 inclusive), 0x1e1→0x602 (0x10), 4→0x604 (0x16/0x17); the weapon byte is
  `*(anim_obj→owner obj+0xdc)+0x62`; owner==0 (script anim objects) skips all six — which is why
  fix-audio2's script path is correct as-is. The port's view-model op-1 site needs the table.
- **The op-1 precache sender**: `SFXPreCache(cmd[2], cmd[1] − (int)cur)` every tick while
  `script+0x78 & 0x10000` (set by `Script_AnimStart` @0x1d8f40) and `(u32)diff < 0x33`; the precache id
  is the RAW cmd[2], the remap applies only to the play [M 0x1c1e9c disasm]. Needs the flag on the
  port's NIS anim scripts.
- Doc fixes for `docs/systems/sound.md` found during the research: §7's "op 2 / sub 3 =
  `Sound_StopAllWithId`" is wrong (op 2 is a no-op; it is op 4 / event 3), and the `cel_tag+0x98`
  field belongs in docs/atlas/cel.md. Both are doc-only edits on shared files, left for the merge
  session to avoid touching sound.md mid-suite.

## Live checks for the main agent

- PCSX2, 07000001 escort route: when Mayhew opens a shoji door, the door creak (bank sfx 1477) plays
  once; a second door within the same entry's repeatDelay (0 frames for 0x27) still plays (entry-level
  gate is per-entry, not per-drone).
- PCSX2, any NIS with looping ambience (07000005 truck ride): ambience with `PauseInNis` set goes
  silent during the scene and resumes at the same pitch afterwards.
- nf-view render (no visual change expected from this task; the audio items are audible-only): none
  requested.

## Verification

- `nf-audio --lib`: 7/7 (the two new G-032d tests + the existing driver suite).
- `nf-game --lib`: the new foley test + the 128 existing.
- Full suite (`NF_ISO=… cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats
  -p nf-audio -p nf-view -- --test-threads=1`): count reported in the WORKLOG entry (started in the
  background before the deadline; the log is `suite-sound.log` outside the repo).
- Determinism: no `Rand_*` draws added anywhere (the foley gate uses none; the precache/window logic
  consumes only the driver's own `SFXrnd` stream inside `start`, as any start does). Seed-pinned sim
  tests are unaffected by design.
