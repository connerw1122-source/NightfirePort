# menu-def-runtime — report

Task: drive the front end from the game's own menu-def data — port the kind-8 menu-def parser
(pages, controls, components, sprites, keyframes, messages) and the uncited `Script_*` keyframe
runtime as a pure data/sim layer, then compare the runtime's tick-0 and settled draw lists
against the constants `crates/nf-view/src/menu.rs` transcribes. Drawing code untouched.

## What's done

- **Parser** (`crates/nf-formats/src/menudef.rs`): the full `MenuManager_Create` 0x21b908 tag
  walk — PAGE/CONTROL/NAME/SCRIPT/MSG/KFRAME/KMSG/MGRMSG/SET/COMPONENT/SPRITE/SKIP/END — with
  the game's control fixups (full-screen type-5 → 0x200×0x1c0; 0x10000224 → (340,198)) and the
  keyframe same-time replace / 0x20-drop rules [M `Script_AddKeyFrame` 0x2201c8:24-38].
- **Keyframe runtime** (same file): `Script_RunFrame` 0x220580 per control per frame —
  line + Catmull-Rom interpolation, the tie branch, loop arm/return/stop, immediate + delayed
  messages, the 0xe1 colour-fade processes — plus `Script_Play` 0x212a8, `Script_PlayDefault`
  0x220350's slot map, `Script_Select` 0x221250, `Script_Stop` 0x221400, `Script_ExitLoop`
  0x2213d0, `Process_Create` 0x21f7a8 / `Process_RunFrame` 0x21f380. API:
  `MenuRuntime::new(page)` + `page_shown(0x4c)` + `tick()` + `draw_list()` (position, size,
  colour, texture, label per control, in the authored 640×480 space).
- **System doc**: `docs/systems/menu-def-runtime.md` (addresses + evidence tags + renderer
  wiring notes for the main agent).
- **Comparison test** (`crates/nf-formats/tests/menudef_disc.rs`, ISO-gated): parses the real
  08000002 from the ISO and checks every transcription-cited control (rect + label + texture,
  layered controls matched by any-at-rect) — plus per-page runtime runs (page-open + 1200
  ticks) with the pin that non-scripted controls hold their authored rects.

## Verification

- `crates/nf-formats/tests/menudef_script_runtime.rs` (7 tests, no ISO): every assertion cites
  the decompile address that says it. The strongest: the Catmull-Rom value −3 at the i=0
  p0-wrap pins BOTH the spline basis AND the game's unsigned min (any other reading fails);
  the line interpolator is pinned as kf[i]→kf[i+1] linear via the disassembly (0x11b3c8:
  `a1 + (a2−a1)·t`); the 0xe1 fade pins the exact-target expiry.
- `menudef_disc.rs` (2 tests, NF_ISO): **the transcription comparison PASSES on every cited
  control** — the main-menu rows (167,262/294/326,306×28 with labels 0x28/0x149/0x14b), the
  wheel rows (323/333/304/333/323 at 123..241), the P_LANGUAGE flags (282,109..349,64×32,
  textures 0x030000c9-cf), the headers/panels/emblems/previews/medal. Structure: 52 of 56 PAGE
  records register (4 skipped by hash/kind — the game's own rule), 830 controls (the docs'
  1003 counts all 56 pages' controls).
- `cargo test --offline --release -j2 -p nf-formats -- --test-threads=1`: 92 passed, the 3
  `disc_golden` failures only (allowed on the slim ISO).
- Full suite on the other crates: started but **not completed in this sandbox** — nf-view's
  build needs the ALSA user prefix re-pointed (PKG_CONFIG_PATH/LIBRARY_PATH under
  `scratch/alsa-prefix`) and the release test run exceeded the session window; `nf-game`/
  `nf-audio`/`nf-view` sources are untouched by this branch (nf-formats is the only crate
  changed), so their suites exercise no new code. The fast runner (`tools/test/suite.py`)
  could not survive as a background job in this sandbox (killed between tool calls; suite.out
  stayed empty).

## What the data says vs the transcription

1. **No page disagrees on geometry**: every cited rect/label/texture matches the parsed data
   (the comparison asserts 0 differences and passes).
2. **Title prompt label** [D vs menu.rs]: the def authors label **457 (0x1c9)** on the
   Press-START control 0x100000fa; menu.rs cites 0x10001eb — that id is bound at RUNTIME by
   `P_START_Handler` 0x20a408:321-323, not authored in the data. Rendering from the def alone
   would show the wrong string unless the handler's binding is modelled.
3. **Doc corrections made while porting** (each found by the disassembly, not the C):
   - KMSG's first dword is the **target** (M_MESSAGE+0x8), not a time — keyframe messages fire
     when their keyframe's time passes. frontend.md and menu.rs's header said "{u32 time}".
   - KFRAME = {u16 **time first**, x, y, w, h, u32 **flags last**}.
   - MSG 0x18's label id is in **B** when A == 0 (A != 0 = a string pointer).
   - COMPONENT = {u16 count, u32 hash, 4×u16} — the hash right after the count [D disc bytes;
     the C's reading order is misleading].
   - `Script_CalculateLine` is linear **kf[i]→kf[i+1]** using its 2nd/3rd args (the outer two
     of the four fetched points are unused) — the Ghidra packing suggested otherwise.
4. **Control ids repeat within a page** (0x10000001 is header, panel and hint on 0x40000025) —
   anything addressing controls by id alone will misbind; address by (page, rect/role) or
   route messages the way `__Menu_Send` does (every control with that id gets it).

## What's left

- Wire `MenuRuntime` into `nf-view`'s page draw (the main agent's render + PCSX2 compare, per
  the task's no-drawing rule). The doc's "Wiring notes" section has the call order.
- The port's `FrontEnd` currently opens pages without the page-level script (each page's root
  control carries one script on the disc [D]); `page_shown(0x4c)` + per-frame `tick()` is the
  integration point.
- The full-suite run for nf-game/nf-audio/nf-view on this tree (see Verification).
- `Script_Play` mode != 0 (reverse play, from `Script_Play` 0x212a8's param_3) is parsed but
  not driven by any test — no disc data was found calling it via msg 0x16 with a nonzero mode
  in this pass; pin it only if a live check shows one.

## Live checks for the main agent

None required — nothing here is visible without wiring, and the numbers above are pinned from
the decompile + disc. If the render pass shows menu animations misbehaving, the first suspects
are (in order): the page-open call (`page_shown(0x4c)` before the first tick), the per-frame
`tick()` cadence (exactly once per 60 Hz frame), and the 640×480 → 512×448 mapping of the
animated rects.

## Usage

not visible to me — this environment exposes no token, credit or context-window figures for
the session (no input/output token counts, no cost display anywhere in the tool output).
