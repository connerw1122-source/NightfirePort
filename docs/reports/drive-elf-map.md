# drive-elf-map — report

Branch `s/drive-elf-map` (from origin/master 0bcd8077). Task: research phase 3 of
docs/atlas/driving.md — the DRIVING.ELF map. **Deliverable: `docs/atlas/driving-elf.md`**, plus
the committed matching tool `tools/crosscheck/elf-shape-match.py`. Docs + one research tool
only; no gameplay code, no tests added, nothing else touched.

## What's done (each task bullet)

1. **Main loop** — `FUN_00174bf0` 0x174bf0 decomposed: it is a one-shot boot wrapper with **no
   loop** (zero backward branches in its 1,984 bytes — disassembly-checked). The per-frame loop
   is a **vblank-driven thread**: `FUN_00261920(0x167de8, 2, 0)` (start, at the end of the
   mission setup) → `FUN_00167de8` → `FUN_00168060(world)` per frame: pad poll
   `FUN_0025a4a8` (2 ports × 4 slots at 0x419840, per device `FUN_0025a0d0`) → vblank counter
   snapshot (`FUN_00261038` ← 0x3c80d4) → present pacing `FUN_00167e80` (skip flag world+0x9c,
   flip status, event queue) → scheduler `FUN_00168280` (two passes over registered objects at
   world+0x10; Update = **vtable+0x1c**; pass 2 dispatches visibility events via
   `FUN_00168678`, event 0x73 at frame end). **Timestep [M]:** table at 0x3c1a38 = {60, 60, 50,
   60} indexed by video mode, `FUN_00110900(1/FUN_002d6b90())` under "Init Actors" → fixed
   1/60 s NTSC, 1/50 s PAL. driving.md's "60 Hz assumed [S]" is now code-proven.
2. **Function inventory** — `docs/atlas/driving-elf.md` §2: a 24-row address-cluster table
   (0x107100–0x326000) anchored by the mission-setup progress prints (Reset Ray Shells →
   Leaving StartUp, each mapped to functions, §1.4 of the doc), the RTTI/class-name strings
   (`PVehicle`, `PBondCar`, `WRoadNav`, `AIVehicle`, `ActActor`, `RCamera`, `InputToAction`…
   [D] at 0x376010–0x3766f0 and 0x39b500–0x39b730), and the shared-code matches. SYM hints stay
   [S]: `DRIVING.SYM` is not extractable from this sandbox's rebuilt ISO (the /DRIVING
   directory records carry extent/size 0 — raw ISO9660 parse [D]; the same zeroing is noted in
   `crates/nf-formats/tests/driving_golden.rs`), and the SYM describes a different build anyway
   (SYM `main` 0x190590 doesn't exist here). Anchor-based hints recorded (pad poll cluster ≈
   `PS2PadDevice::PollDevice` [S]).
3. **Shape-match with ACTION.ELF** — `tools/crosscheck/elf-shape-match.py` (committed): exact
   byte pass + masked-shape pass (jal/lui/immediates dropped; relative branches kept). Result:
   **329 shared functions** (193 byte-exact, 136 shape): libc string/mem, soft-float (dp+bi),
   libm (floorf/fabsf/tanf/scalbnf/isinf), VSync primitives, SIF/RPC, sceGsResetGraph, and the
   whole libmpeg/IPU movie decoder (the `_ri0_*/_rix_*` family is byte-identical). **Zero
   game-side matches and no BIGF match** — the driving container reader is a different build of
   that code, so the port cannot blindly reuse its BIGF port; the listed libc/libm/mpeg
   versions are the reusable references. Spot-verified: ACTION `memcpy` 0x236388 ≡ DRIVING
   0x2a7e84 byte-for-byte (identical decompiles).
4. **§7 open question 5 — the `.crp` consumer** — answered from code [M]: the track object at
   global `DAT_0033aac8` (0x3c-byte object, ctor `FUN_0022f820`); filename `FUN_0022f898`
   sprintf `%s.crp` (0x39b620, ref 0x22f8cc) with dir `data\track\` (0x375b78); async load
   `FUN_001a9d70`; decompressed buffer at obj+0x14; chunk finders `FUN_002d51f0`/`FUN_002d5a10`
   (0x10-byte directory records, flags 0x8/0x10 = binary search, flag 0x2 = relative pointer);
   consumers mapped per chunk: `DBN `, and `Map ` → {`Wmap` (waypoints/roads →
   `FUN_0022c730`, obj+0x1c), `Rule`, `MSet`, `Audi`, `Trgr`, `Envi`, `AIEl` (AI elements →
   `FUN_0012be48`), `Shad`, `VisC`, `in`, `iN`, `ps`, `pt`, `Arti`, `Name`, `Shar`, `CDat`,
   `RNgp`, `sr`}. The roads/AI join is `FUN_00230730` (reads `Wmap`+`AIEl`+`Rule`+`MSet`); the
   art/render side `FUN_002cd8c0`/`FUN_002dc1b0`; visibility `FUN_0021eae8`. Physics-side
   per-frame reader of `Wmap` = phase 4.

Plus: the mission setup/teardown call trees are fully annotated by their own progress strings
(doc §1.4), the phase-4 (vehicle physics) task description is written at the end of the doc
(§5), and 5 live checks for the main agent are listed (doc §6, L-1…L-5: vblank counter 0x3c80d4
+1/frame; breakpoint `FUN_0022f898` for the `.crp` name; the world object list dump at 0x3c87b0
after `Leaving StartUp` to pin the per-frame subsystem order and vtables; the thread-start
breakpoint on `FUN_00261920`; the skip-flag world+0x9c watch).

## How it was verified

- Every [M] claim cites function + address from `work/decomp/DRIVING.ELF/` or the raw ELF
  bytes (tables/FourCCs dumped from `work/elf/DRIVING.ELF` in this sandbox).
- The shape-match script re-runs from the committed tool in seconds; its two passes and the
  ≥32-byte stub filter are documented in its header.
- `cargo test --offline --release --no-run` builds all four crates' 338 test binaries green
  on the branch (one linker bus error was disk-full — sandbox at 100%; freed 1.3 GB by
  removing the stale `target/debug`, then clean build).
- Suite chunk run (the runner's background mode is killed between tool calls in this sandbox —
  two launches died; setsid did not survive either — so it ran in the foreground, bounded):
  `tools/test/suite.py --limit 240 --only driving_golden,disc_golden,pss_movies_live,
  weapon_live,exact_vs_host_probe,facial_probe,bank_live,music_live,fix_sound_remainder_live`
  → **27 passed, 3 failed, 0 ignored (5 min)**; the 3 failures are exactly the 3 `disc_golden`
  tests that may fail on the slim ISO. The full 338-binary sweep did not fit the remaining
  window; the change is docs+tool only (no Rust code), so the binary surface is untouched.

## What's left

- `Map `/`Wmap`/`RNgp` record formats inside the decompressed `.crp` (phase 4 entry work).
- Pinning each registered object's Update vtable (needs live check L-3, or a vtable walk from
  the constructors).
- SYM hint names remain unmatched beyond the anchors (needs the full-disc SYM or shape-matching
  against the SYM's build if it ever surfaces).
- The parallel event-slot array at world+0x50 and `FUN_001749e8`'s callee-level split stay [S].

## Shared files touched

- `docs/WORKLOG.md` — append-only entries (branch start + completion).
- No source files touched; `docs/atlas/driving-elf.md` and
  `tools/crosscheck/elf-shape-match.py` are new files.

## ntfy failed

All three fallbacks were attempted in order and all returned the same quota error:

1. `curl -sS -m 20 -H "Title: NightfirePort" -d "drive-elf-map: …" https://ntfy.sh/nightfire-yoda-8k3x`
   → `{"code":42908,"http":429,"error":"limit reached: daily message quota reached; increase your limits with a paid plan, see https://ntfy.sh","link":"https://ntfy.sh/docs/publish/#limitations"}`
2. `python3 urllib` → `urllib.error.HTTPError: HTTP Error 429: Too Many Requests`
3. `wget --post-data` → `ERROR 429: Too Many Requests.` (HTTP request sent, awaiting response... 429)

(The daily ntfy.sh quota was already exhausted by earlier tasks today — see WORKLOG
find-corona-creator's identical 429 note. The finish.sh queue ack itself succeeded:
`OK done 501-drive-elf-map`.)
