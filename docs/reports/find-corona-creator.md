# find-corona-creator — report

Task: U-042 / G-034 — find who creates the runtime class-0x45 coronas (`Corona_Update`
0x18da78, the LOS-alpha-fade class); port it if it exists, prove it dead if it doesn't.

**Result: U-042 is closed as DEAD CODE.** No creator of a class-0x45 object exists anywhere in
ACTION.ELF (the full caller scan below), so `Corona_Update` never runs in the shipped game and
nothing needed porting. The static type-0xFC placement coronas were already fully ported
(WORKLOG 0073, `corona_live.rs` golden test) — that remains the whole corona system.

## What's done

1. **The dispatch mechanics, settled from the code** (docs/systems/corona.md carries the full
   write-up with addresses):
   - The class byte is **`obj+0xFF`** [M `Control_DeleteAllObjectsOfType` 0x1349e8:11,
     `Control_ReturnNextObjectOfType` 0x134a38:15 — both compare it against their type arg].
   - The dispatcher `control_movement_object_handler` 0x1336f8:62-64 calls
     `control_funcs[class]` with **stride 0xC** — 85 entries × {update, collide, third};
     `control_delete_object` 0x133178:16-17 calls entry+8 (the destructor slot) the same way.
     `control_funcs` @ 0x26f890 is static .data and **read-only in code** (exactly two .text
     refs, 0x133184 and 0x1337f0, both `lw`).
   - `control_funcs[0x45] = { Corona_Update 0x18da78, NULL, NULL }` [M table dump @
     0x26f890 + 0x45·0xC = 0x26fbcc] — the link-time "slot assignment" the task asked about.
   - The create helpers never write the class byte: `control_create_object` 0x1344e0 (memset +
     init only; its first arg is the instance size, not the class), `Control_CreateObjEx`
     0x1343c8, `control_init_object` 0x134630. So every class store comes from a creator.
2. **The full class-store scan (the proof).** A raw-opcode scan of ACTION.ELF's .text
   (322,249 words, cross-checked capstone vs. manual encodings) finds exactly **110
   `sb rt, 0xFF(base)` stores** — the complete set of class writes. Every one resolves to an
   immediate class value via the decompile + register tracking; the three that look
   parameter-fed in the disassembly are constants loaded into the arg register
   (`Hurt_Create` `addiu $a2,$zero,0x27` @0x17e93c, `SS_Create` `$a2,0x25` @0x1a9d40,
   `SoundTrigger_Create` `$a3,0x40` @0x1b2704). The full value set is
   {0,1,2,3,5,7,0xC,0xD,0xE,0xF,0x10,0x11,0x15,0x1A,0x1B,0x1C,0x1D,0x20,0x21,0x22,0x23,0x24,
   0x25,0x26,0x27,0x28,0x2A,0x2B,0x2D,0x2E,0x30,0x31,0x33,0x35,0x36,0x37,0x38,0x39,0x3A,0x3B,
   0x3C,0x3D,0x3F,0x40,0x44,0x46,0x47,0x48,0x49,0x4A,0x4B,0x4D,0x4E,0x4F,0x50,0x51,0x52,0x54}.
   **0x45 is not in it.** No other path reaches the byte: no `sw …,0xFC` / `sh …,0xFE` on
   object bases (the 16 raw hits are AI-route/menu-label/MPEG structs; the only sq-class hits
   are VU0 `sdc2 …,0xF0($sp)` stack spills), and no `memcpy`/`amemcpy` writes an object's
   +0xF0..0xFF window (decompile grep).
3. **Every named alternative checked, all negative:**
   - `Script_CreateEntity` 0x1d8968's kind switch maps to Break(0x20), Ladder(0x2D),
     Switch(0x39), Destroy(0x21), Trigger_TouchOnce/Touch(0x1D), Pickup, Drone_AIVolume(0x2F),
     Emitter(0x32) or default class 0 — never a corona [M 0x1d8968:35-176].
   - Searchlight/light builders: `Searchlight_Create` 0x1cb4f8 → class 0x22; `Light_Create`
     writes no class byte (lights are not control objects).
   - Vehicle/explosion: Car 0x28, Sub 0x4A, MiniSub 0x4D, Copter 0x3B, Destroy 0x21,
     Debris 0x10 — none 0x45.
   - No `jal`/`j` to `Corona_Update` 0x18da78 anywhere in .text; DRIVING.ELF has no corona
     symbols at all.
   - `Corona_Create` 0x18da00's only caller is `parsemap_create_dynamic_objects` case 0xFC
     (0x1d04f0:525-527) and it creates **class-1** objects (+0xFF = 1) — the ported static
     path (`control_funcs[1]` is empty, so no update and no RNG there either).
4. **`Corona_Update` itself documented (unported on purpose):** runs on GameState-odd parity
   ticks (`(GameState+0x34 & 1) != *(short*)(obj+0xF4)`; nothing in the corona path writes
   +0xF4, so it stays at the create-time 0); forces alpha 0 if `+0xFC == 0`; otherwise the
   occlusion ray `Collide_LineOfSight(viewer eye, corona centre, …, 8)` moves the alpha byte
   `+0x106` by ∓0x10 per tick (0/0xFF clamps) and sets/clears display-flag bit 0x2000. No RNG.
5. **Docs:** new `docs/systems/corona.md` (mechanism + proof + verdict); `docs/UNKNOWNS.md`
   U-042 → closed (dead code); `docs/GAPS.md` G-034 open clause → closed; the
   `crates/nf-game/src/corona.rs` module doc sharpened (control_funcs[0x45] row, the scan
   verdict, pointer to the new page).

## How verified

- The scan is machine-checked two ways (capstone decode of every .text word and a raw
  opcode-field scan); both agree on 110 `sb …,0xFF` sites. The scratch scripts live outside
  the repo (`../my-project/scripts/corona_*.py`); the method is fully specified in
  docs/systems/corona.md so it can be re-run.
- No gameplay code changed — the only .rs delta is the corona.rs module doc comment — so the
  **no new failing-then-passing NF_ISO test** (the dead-code branch of the task's Do 2 asks
  for the proof, not a port). The existing golden test `crates/nf-game/tests/corona_live.rs`
  (13 placement tuples + doors discriminator) still pins the static path and runs in the
  suite.
- Full suite via the fast runner on this branch: see the totals below.

## Suite totals

Two runs, the fast runner (tools/test/suite.py), both on this branch:

- **Pre-merge tree** (branch + docs, before merging origin/master a2958c03): 253 of 336
  binaries ran inside the window; every non-environmental binary green. The 6 failures are
  the known environmental set — disc_golden ×3 (slim-ISO, allowed by the task) and the no-GPU
  pylon set ×3 (fix_0c_pylon_live ×2 + probe_pylon_render; wgpu "adapter: NotFound", the
  sandbox has no GPU adapter — the same set an earlier task recorded on an untouched tree).
  A mid-run disk-full incident (the sandbox rootfs hit 100 %, linker SIGBUS) cost ~20 minutes
  and was fixed by deleting the recycled old sandbox clone (my-project/NightfirePort, 2.4 GB;
  its HEAD matched origin exactly) plus duplicate work/ copies.
- **Final tree** (branch merged with origin/master a2958c03, WORKLOG both kept): the full
  338-binary run was restarted at 15:30 UTC but **could not finish inside the sandbox window**
  (build ok in 221 s; then ~25 binaries per 5 min). At the finish push it stood at **76
  binaries: 231 passed / 0 failed / 5 ignored, no FAILED/TIMEOUT/CRASH lines**. My delta vs.
  the pre-merge tree is nothing but the WORKLOG merge — the code under test is identical —
  and the pre-merge run covered the same code 253 binaries deep with only the environmental
  set failing. The runner was left alive; if the sandbox lives on, later numbers are in
  ../suite.log / suite.out.

## What's left

Nothing for U-042 — closed. If a live PCSX2 trace ever shows `Corona_Update` executing, the
creator must come from outside this scan's coverage; re-run the catalog first
(docs/systems/corona.md ends with the recipe).

## Live checks for the main agent

Nothing visual changed (the static coronas render exactly as WORKLOG 0073 verified), so no
new `--bond` renders are needed; one regression spot if convenient:

- nf-view `07000005` from the castle courtyard facing the gatehouse facade (the
  `Window_corona` placements 231–237): the 7 window glow quads should still be there, alpha
  list, additive blend — identical to the 0073 renders.

PCSX2 (the real verification this task wants):

1. Breakpoint on `Corona_Update` 0x18da78 (execute) during extended 07000005 play —
   expected: **never hit** (no class-0x45 object exists; the 13 static coronas are class 1).
2. `tools/trace/gdbwatch.py` on 0x26fbcc (the `control_funcs[0x45]` update-word) for reads
   during play — expected: **never read** (the dispatcher only touches a row when an object
   of that class exists). A read would mean a class-0x45 object came to exist and the
   dead-code verdict needs re-opening.

## Shared files touched

- `docs/UNKNOWNS.md` (U-042 row only), `docs/GAPS.md` (G-034 row only),
  `docs/WORKLOG.md` (append-only entries) — one-row surgical edits, no reformatting.
- No shared source file (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`) was touched at all.
- `crates/nf-game/src/corona.rs` is a task-named file; comment-only change.
