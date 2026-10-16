# spu-gaussian-reverb (GAPS G-032: the SPU2's Gaussian interpolation and room reverb)

Branch `s/spu-gaussian-reverb` (from `origin/master` 170bcf01). Task 725.

## What's done

### 1. The Gaussian interpolation table — already on master; cross-checked and recorded

Master already ports the SPU2's 4-tap Gaussian interpolation (`crates/nf-audio/src/lib.rs`): the full
512-entry `GAUSS` table and `resample_gauss`, with a golden test (`one_vag_resampled_sample_exact`)
and the table-integrity test (`gauss_quads_sum_near_0x8000`). Per the "don't churn working code" rule
I did not rewrite it; I sourced and cross-checked it as the task asks:

- **Source**: nocash PSX-SPX, "4-Point Gaussian Interpolation" (problemkaputt.de/psx-spx.htm), which
  documents the exact per-product form the port implements:
  `out = (gauss[0FFh−i]·oldest) SAR 15 + (gauss[1FFh−i]·older) SAR 15 + (gauss[100h+i]·old) SAR 15 + (gauss[i]·new) SAR 15`,
  with `i` = pitch-counter bits 4–11. The port and its pinned golden values match this exactly.
- **Cross-check (independent emulator)**: duckstation `src/core/spu.cpp` `GenerateInterpolationCoefficients()`
  — its 512-entry `gauss` table is **identical, entry for entry**, to the port's (script-compared).
  The comparison script output is quoted in the git history of this branch (scratch tooling, not
  committed). Recorded in the `GAUSS` doc comment.
- **Reference divergence, recorded not fixed**: duckstation computes one 32-bit sum then a single
  `>> 15` (via `GSVector4i::madd_s16`), while nocash documents four per-product `SAR 15`s. The two
  differ by ±1–2 LSB on some inputs (e.g. the VAG golden block gives 401 vs 403 at pos 2). The port
  follows the documented hardware form (nocash), which master's golden test already pins.
- PCSX2's SPU2 gaussian table could not be fetched directly (raw.githubusercontent paths for
  `pcsx2/SPU2/Interpolation.cpp` 404 on current master; GitHub API rate-limited). Duckstation is the
  independent cross-check; PCSX2's *reverb* side (below) was fetched and used.

### 2. The room reverb — register-exact port, new `crates/nf-audio/src/reverb.rs`

The game's reverb chain, ported with evidence:

- `PS2_SetupReverb` (SFX.IRX 0xc944) selects **effect mode 4 = SD_EFFECT_MODE_STUDIO_3** on both
  cores, sets EEA 0x1FFFFF (core 0) / 0x1DFFFF (core 1) via `sceSdSetAddr(core|0x1d00, EEA−1)`, and
  enables effects (`sceSdSetCoreAttr(core|SD_CORE_EFFECT_ENABLE,1)` → core-attr bit 7). libsd
  ignores the attr's delay 30 / feedback 200 for non-ECHO/DELAY modes, resolving the old "slightly
  ambiguous" note. The 0x6fe0 local in the decompile is the mode's `EffectSizes[4] = 0x6FE` (×16).
- `psiSetReverb` (0xc840) writes depth→EVOL: `EVOL = (((depth*0x50)/100)*0x3FFF)/100` to
  EVOLL/EVOLR of both cores (libsd entries 0xb80/0xc80/0xb81/0xc81 = `ParamRegList[0xb/0xc]`).
- `SFXSetEnvironment` (0xdb4) / `SFXUpdateEnvironment` (0xe60) / `SFXResetEnvironment` (0xd68):
  depth target/current, indoors wet target (50/100)/current, the −26472 underwater kill, the
  +16/−4 and ±4 ramps — ported as `reverb::SfxEnvironment`.
- Per-voice sends: `PS2_VoiceSetReverb` (0xebcc) builds the bitmask written to the VMIXEL/VMIXER
  wet-mix enable switches (0x1900/0x1b00 + core bit) in `PS2_SendKeyONKeyOff` (0xd544) [M].
- The 10 Sony per-mode preset tables (32 registers each) and the register map: ps2sdk libsd
  `effect.c` (TyRaNiD/Lukasz Bruun — a register-compatible reimplementation of Sony's closed libsd;
  its tables are the Sony presets), cross-checked against PCSX2 SPU2 (`pcsx2/SPU2/Reverb.cpp`
  `V_Core::DoReverb`, `defs.h` `V_Reverb`, `RegTable.cpp`, `regs.h`). Naming disagreements between
  the ps2sdk headers and PCSX2 (mode 9 PIPE vs CLEAR; the P-block volume names) are recorded in the
  module comment, not silently resolved.
- The DSP: the SPU2 reverb equations (IIR same/diff paths + 4 combs + 2 APFs over one shared effect
  area, L/R alternating ticks at half the SPU rate, `MUL = x*y >> 15`, delay offsets = register × 4)
  drive `Output`'s wet path; the old single-comb stand-in ("exact preset coefficients pending") is
  replaced. The per-voice send enters at full scale and EVOL scales the wet return (hardware form),
  replacing the old send-scaling.

### 3. Golden tests (numbers only, silent — NF_SILENT)

In `reverb.rs` (`cargo test -p nf-audio`):
- `studio3_preset_decodes_to_the_register_set` — all 32 mode-4 register values the game writes.
- `presets_decode_consistently` — all 10 modes: input coefficients (±1.0 except OFF) and every
  delay register inside its own effect area at the ×4 sample scaling.
- `evol_depth_numbers` — `psiSetReverb`'s exact integer chain (0→0, 100→0x3332, 50→0x1999, 20→0x0A3D).
- `game_setup_registers` — mode 4, EEA per core, ESA = EEA−(0x6FE·16−2), effects bit, switch entries.
- `environment_ramp_and_underwater` — the ramp clamps, the −26472 magic, and the exact EVOL sequence
  written while a depth-100 room fades in ([0x07AD, 0x0FFF, 0x1851, 0x20A3, 0x28F5, 0x30A3, 0x3332]).
- `reverb_core_silence_impulse_and_tail` — a synthetic impulse through the preset: silence-in/silence-out,
  bounded deterministic left-side tail, first returns pinned at ticks 948/2531, 968/2919, 1624/2279
  (mirror-computed; regression pin for the equation transcription).

`lib.rs` keeps the existing Gaussian golden test (`one_vag_resampled_sample_exact` — the synthetic
VAG block through the real path, values pinned from the nocash formula) and `gauss_quads_sum_near_0x8000`.

## How it was verified

- `NF_SILENT=1 cargo test --offline --release -j2 -p nf-audio --lib` — **17 passed, 0 failed**
  (the 6 new reverb tests plus the prior Gaussian/environment/mixer ones untouched).
- The fast suite (`tools/test/suite.py`) on the final tree (this branch = origin/master 170bcf01, no
  drift): see the total line below.
- **Disk caveat**: this sandbox never had room for the disc image (2.9 GB free at setup, 337 MB
  after the four-crate build — the ISO archive alone is 352 MB), so `NF_ISO` was unset for the suite.

Suite on the final tree (`tools/test/suite.py`, `--test-threads=1`, NF_SILENT):
**1809 passed, 145 failed, 72 ignored** (434 test binaries). All 145 failures are the missing-disc
environmental panics — 62 binaries `panic!("skip: NF_ISO not set")` / `env::var("NF_ISO").unwrap()`
→ `NotPresent`, and 2 more (`walkreal_probe_fb8/fb9`) the same `EnvVar NotPresent` via unwrap: every
failing block is an `NF_ISO`-gated probe/live test, verified by scanning `suite.log` per binary.
Nothing my task touches fails: `nf_audio` lib = **17 passed / 0 failed**, `bank_live` and
`music_live` pass. (On a disc-bearing run only the 3 `disc_golden` failures are expected.)

## What's left

- **Per-room reverb depth values** (which room requests which depth): that is the cel/room data path
  (`Sound_UpdateListeners`@1dd278 reads `room+0x98` s16 and the `room+0x3c & 0x10000000` indoors bit
  and calls `SFXSetEnvironment`) — tracked as G-016, not this task. The decode from a depth value to
  the register set is done and tested here.
- **DSP bit-exactness** [S]: the equations follow PCSX2's `DoReverb` (itself derived from drhell's
  PS1 page), but hardware-side confirmation (e.g. against a PCSX2 A/B of the same register write) is
  pending. The register DECODE is exact [M]; the audible topology is the reference emulator's.
- PCSX2's SPU2 gaussian table was not directly diffed (fetch failures above); duckstation's identical
  table is the recorded cross-check.

## Live checks for the main agent

1. After the intro of any level with reverb, dump SPU2 core-0 reverb registers at 0xB40002EC..0xB400033B
   (read, u16 pairs): expect the STUDIO_3 pattern — 0xB40002EC = 0x0000 and 0xB40002EE = 0x37EC
   (SAME_L_DST 0x0DFB stored as val<<2), 0xB40002F4/0x2F6 = 0x0000/0x3424 (COMB1_L_SRC 0x0D09),
   0xB4000334/0x336 = 0x0000/0x0550 (APF2_L_DST 0x0154), and P-block 0xB4000774 = 0x6F60 (IIR_VOL),
   0xB4000784/0x786 = 0x8000/0x8000 (IN_COEF).
2. Watch EVOL (0xB4000764, 0xB4000766, write) while walking between a dry and a reverberant room:
   expect it to step by at most +16/−4 depth units through `(((d*80)/100)*0x3FFF)/100` per frame.
3. Watch VMIXEL (0xB40001E4? — the switch entry 0x1900 maps through libsd's `ParamRegList[0x19]`)
   when a ReverbSend sound keys on: the voice's bit should set.

## Usage

not visible to me
