# The menu-def keyframe runtime (menu-def-runtime)

The front end's menus are animated by data in the kind-8 LevelBin entry `08000002` inside
`07000048.bin` (FILES.BIN). The port now parses that whole def (pages, controls, components,
sprites, keyframes, messages) and runs the game's `Script_*` keyframe runtime headlessly, as a
pure data/sim layer in `crates/nf-formats/src/menudef.rs`. The draw code is untouched: the main
agent wires `MenuRuntime::draw_list()` into the renderer and compares against PCSX2.

## Parser — `MenuManager_Create` 0x21b908's tag walk

All [M] (ACTION.ELF decompile + disassembly under `work/decomp/ACTION.ELF/`), unless marked [D]
(read off the disc's own bytes) or [S]:

- The walk consumes ONE leading format dword before the tag switch [M disasm 0x11598c-0x11599c,
  switch entry 0x1159b8-0x1159e0]. On the disc's 08000002 that word is 0xfffffffa [D].
- Tags: PAGE −0x10 {u32 id, u32 file hash, u32 aux, u8 kind, u32 size} (kind 0/3 + hash match →
  page, else skip `size`); CONTROL −0x07 {u32 id, u8 b, u8 type, u16 x/y/w/h, u16 f1, u32 d1,
  u16 f2, u32 d2, u32 d3}; NAME −0x08 (32-byte string); SCRIPT −0x09 {u32 id, u32 flags};
  MSG −0x0a {u8 id, u32 a, u32 b}; KFRAME −0x0b {u16 time, u16 x/y/w/h, u32 flags}; KMSG −0x0c
  {u32 target, u8 id, u32 a, u32 b}; MGRMSG −0x0d; SET −0x03; COMPONENT −0x04; SPRITE −0x05;
  SKIP −0x02; END −0x0e. An unknown tag ends the walk (the switch's default clears the loop
  flag) [M 0x21b908:195-197].
- **KFRAME order**: the time is the FIRST u16 and the flags are the trailing u32 (into kf+0x18 /
  kf+0x1c) [M disasm 0x115fbc-0x116004]. The older "5×u16 + u32" reading without an order was
  ambiguous; this settles it.
- **KMSG's first dword is the TARGET, not a time** [M disasm 0x116028-0x116058: it lands in
  M_MESSAGE+0x8, the slot `Script_RunFrame` routes on]. Keyframe messages fire when their
  keyframe's time passes, so they need no own time. `docs/systems/frontend.md` and menu.rs's
  header called it "{u32 time}" — corrected.
- **MSG 0x18 (label)**: a == 0 → the label id is in B (`Txt_BindLabel(b)`) [M Label_SendMessage
  0x218890:44-53]; a != 0 → a raw string pointer. menu.rs's "MSG 0x18 = Txt_BindLabel" held the
  id in A — corrected in the parser.
- MSG ids 0x1a-0x1c with a == 0x7d6d59ff are rewritten to 0x645a49ff [M 0x115eac-0x115ec4];
  msg 0x1d is read and DISCARDED [M disasm 0x115ed4-0x115ed8].
- COMPONENT is {u16 count, u32 hash, 4×u16} (14 B) [D the disc's own bytes: the def opens
  SET 1 → COMPONENT count 9 hash 0x03000042 (the button/arrow atlas) + 8 zero bytes].
- Control fixups [M 0x21b908:305-314]: type 5 full-screen (x<1, y<1, w>0x27f, h>0x1df) →
  w=0x200 h=0x1c0; id 0x10000224 → (340,198); else type != 0xd → `FixupResolution` 0x21b610.
  The port does NOT bake the resolution scale: it keeps the authored 640×480 space (the
  transcription's space) and maps ×512/640 ×448/480 at draw time — exact, because the keyframe
  `FixupResolution` 0x21b7a8 is a pure per-axis affine (x·0.8, y·0.9333334; the centre terms
  cancel; w/h clamp ≥ 2 after scaling) and both interpolators are affine-invariant (the
  Catmull-Rom basis weights sum to 1).
- Keyframes attach sorted by time; a same-time keyframe UPDATES the existing one in place
  unless its flags bit 0x20 is set, in which case the NEW frame is dropped (and the parser's
  current-keyframe pointer becomes NULL, so a following KMSG attaches nowhere) [M
  `Script_AddKeyFrame` 0x2201c8:24-38].

## Runtime — `Script_RunFrame` 0x220580 (per control per frame, from `Page_Update` 0x21edc8:84)

State on the control [M `Script_PrepareScripting` 0x220ed8]: script +0x38, segment-end
keyframe +0x3c, loop-return keyframe +0x40 / index +0x44, segment-start index +0x4c (init −1),
last message result +0x50, frame timer +0x54, flags +0x58, delayed processes +0x5c, animated
rect +0x70..+0x77.

- t = idx + (timer − prev.time)/(cur.time − prev.time); t = idx+1 when the times tie; prev =
  the current keyframe's list-predecessor, itself when none [M 0x220580:23-41].
- Interpolation [M `Script_InterpolateLine` 0x220bc8]: clamp t to [0, count−1], i = trunc,
  frac = t − i; script flags bit 1 → `Script_CalculateSpline` 0x221410 (Catmull-Rom over
  kf[i−1], kf[i], kf[i+1], kf[i+2], with the i−1 slot = min(count−1, i−1) as UNSIGNED — at
  i = 0 that is the LAST keyframe [M 0x220bc8:53-56]); otherwise `Script_CalculateLine`
  0x2214c8 = **p1 + (p2−p1)·frac, i.e. linear between kf[i] and kf[i+1]** [M disasm 0x11b3c8:
  `a1 + (a2−a1)·t` — the outer two of the four fetched points are unused]. Results store
  through a C float→int cast (trunc toward zero) [M 0x220bc8:74].
- When the timer reaches the segment-end keyframe's time: kf-flags bit 2 records the loop
  return and arms the script (script flags |= 8) [M 0x220580:54-62]; kf-flags bit 4 returns to
  the recorded point when armed (control flags +0x58 |= 0x10, timer = the return keyframe's
  time, then ++) and otherwise plays on [M 0x220580:121-134]. Each passed keyframe fires its
  messages [M 0x220580:73-120].
- Messages [M 0x220580:83-116]: id < 0xe1 sends immediately (target −3 = the manager, −2 = the
  control itself, else control-by-id via `__Menu_Send` 0x1f29c0; arg A −4 = the control's last
  message result +0x50). id ≥ 0xe1 becomes a delayed process (`Process_Create` 0x21f7a8) with
  delay = next keyframe's time − timer − 1 (16-bit wrapped; 15 when there is no next), owned by
  the target control (msg 0x39 resolves the id to the pointer [M Label 0x218890:157-159]).
  `Process_RunFrame` 0x21f380 blends each byte channel of the owner's colour start→target over
  the delay (start captured via msg 0x30 [M Label 0x218890:148-150]) and the expiry sends the
  exact target — **the disassembly hardcodes msg 0x1a at both call sites** (0x1195ec, 0x119634),
  so every delayed process fires a colour set.
- Scripts play via `Script_Play` 0x212a8 (mode 0: cur = first keyframe, idx = −1, timer = 0)
  and `Script_PlayDefault` 0x220350: msg → default slot {0x4c/0x51→0, 0x4d/0x4e→1, 0x4b/0x63→2,
  0x49→3, 0x4f→4, 0x5b→5}; msgs 0x4c/0x4d/0x63 run the PAGE's default-script table on the page
  (the page-open chain: Manager msg 0x44 → `Script_PlayDefault(page, 0x4c)` [M
  `Manager_SendMessage` 0x21c858:346]), the rest the control's own table (type 0xd windows
  skip) [M 0x220350:98-100]. The tables bind via msg 0x21 {slot, script id} in the widgets'
  SendMessage [M Label 0x218890:89-96, Window 0x224190:19-26]. `Script_Stop` 0x221400 ends the
  script (cur = 0); `Script_ExitLoop` 0x2213d0 clears control-flags bit 0x8 (its connection to
  the loop gate, which reads the SCRIPT's own bit 8 [M 0x220580:122], is not in the decompile —
  no cycle-ending claim is made).

## What the disc data holds [D, from the port's own parse]

52 of 56 PAGE records register (the rest are skipped by hash/kind exactly as the game does);
830 controls; keyframe messages, msg-0x21 bindings and the per-page script census print from
`menu_def_runtime_settles_into_the_authored_layout` (nf-formats/tests/menudef_disc.rs).

## Tests

- `crates/nf-formats/tests/menudef_script_runtime.rs` — pure unit pins, every assertion cites
  the address: tick-0 = kf[0] and settle = kf[last] (line), the exact Catmull-Rom value −3 at
  the i=0 p0-wrap (which fails under any other reading), loop arm/return/stop ticks, the
  same-time replace + 0x20-drop, the 0xe1 colour fade to the exact target, message routing
  (by-id, −2, dead target), the PlayDefault slot map and the type-0xd skip.
- `crates/nf-formats/tests/menudef_disc.rs` — ISO-gated: parses the real 08000002, pins the
  structure, compares every transcription-cited control (rect + label + texture; layered
  controls matched by any-at-rect), runs every page through page-open + 1200 ticks and pins
  that non-scripted controls hold their authored rects, and pins the two control fixups.

## Wiring notes for the renderer (main agent)

- Build `MenuRuntime::new(&page_def)` when a page opens, call `page_shown(0x4c)`, then
  `tick()` once per frame and read `draw_list()`.
- Focus changes: `focus_msg(new_row_ix, 0x4e)` (gained) / `focus_msg(old_row_ix, 0x4f)` (lost)
  [M `Page_Update` 0x21edc8:45-46].
- Rects come in the authored 640×480 space; map with menu.rs's existing mx/my.
- `animating` marks controls whose timer/processes are still running — the settled rect of a
  non-looping script is its last keyframe; looping scripts never settle (the game keeps them
  cycling too).
