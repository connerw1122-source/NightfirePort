# drive-keyboard-input — the driving missions on keyboard and mouse

**Done.** The PC keyboard + mouse mapping for the driving missions is designed onto the game's own
driving actions and ported through the game's real input path (raw pad slot → `poll_device` → the
game's binding evaluator → the car's action queue → the vehicle-layer input stage), with tests
that walk numbers through that whole chain. No pad work, no drawing (the main agent draws every
page; this task delivers the data, the state, the flow and the getters).

## What's done

- **The ramp question, settled** ([M], docs/systems/driving-pc-input.md §1): there is no input-path
  ramp (driving-input.md §8 [M-negative]) and **no d-pad steer at all** — the digital steer actions
  (STEERLEFT/RIGHT 4/5, \*DIGITAL 8/9/0xa/0xb) have **no consumer in DRIVING.ELF** [M-negative: the
  car's GAMEACTION drain `FUN_0018add8` 0x18add8 switches only on 1, 2, 3, 0xc–0x11, 0x16, 0x17,
  0x1a, 0x1b, 0x1c, 0x1d–0x21, and all seven queue-drain sites were enumerated]. Steering reaches
  the car only as GAMEACTION_STEER (1) with a signed float, and the **vehicle layer** ramps
  whatever arrives — analog or digital 0/1 alike: the drain's tail chops the raw steer at
  ±PAD_DEAD_ZONE = 0.2 (`DAT_00331988`, the `Physics\Physical\default.tun` key) with write-back,
  then `car+0x240 = move_toward(car+0x240, target, 0.25)` (`FUN_002104a8` 0x2104a8; the 0.25 is a
  code immediate, no tuning key exists [M-negative]). Digital full lock = **4 ticks (66.7 ms at the
  fixed 60 Hz)**, full-left→full-right = 8, release ramps back in 4. GAS carries a ×2-gain clamp
  (`min(2v, 1.0)` per tick [M :179-181]); BRAKE is raw [M :137-142] and reverse is the engine's own
  ≈30-tick brake-at-standstill state (`FUN_0018cef8` :184-208 [M]) — brake **is** reverse; the
  gas/brake totals exclude each other (smaller zeroed, ties zero the gas [M :223-230]); the
  handbrake latches `+0x259` and zeroes both pedals [M :231-238].
- **Every mapped action's semantics pinned** [M, doc §2]: fire primary (0x1d/0x1e), fire secondary
  (0x1f/0x20), gadget (0x24/0x25, re-fires while held via WeaponMan's per-frame `FUN_0020d700`),
  handbrake (0x1b/0x1c) and look back (0x29/0x2a, latched in the chase-cam dispatch `FUN_001bddc8`)
  are HOLD pairs; weapon cycle (0x21/0x22 next, 0x23 previous — WeaponMan 0x20c078, index
  `+4 = list[(cur±1)%n]`), CHANGECAMERA (0x26, MissionMan one-shot request flag) and POVRELOAD
  (0x3f) are one-shot edges. CAMZOOMBUTTON (0x2b) is dead in this build. Nothing needs auto-repeat.
- **The mapping** (doc §3, [S] per row — the shipped `Master.def` bindings are unextractable on the
  slim ISO): A/D + ←/→ → LX bytes 0/255 → STEER ±1.0 (the arrows steer through the analog axis
  *because* the digital steer actions are dead — d-pad bits would do nothing); W → △ pressure →
  GAS; S → ✕ pressure → BRAKE (pedals as pressures so each is its own value — a shared axis would
  send one action the other's sign; the game's GASBRAKE exists for the pad's shared axis and splits
  at ±0.2 with negative = gas [M :119-135]); Space → ✕ bit → HANDBRAKE pair; left mouse → R1 bit →
  FIREPRIMARY pair; B → R2 bit → FIRESECONDARY pair (the port's on-foot Alt. Fire key, kept
  consistent); right mouse → ○ bit and E → L1 bit → FIREGADGET pair; Q → Select bit →
  TOGGLESECONDARY; wheel up/down → L3/R3 bits → TOGGLESECONDARYUP/DOWN; C → Start bit →
  CHANGECAMERA; Tab/middle-mouse hold → □ bit → CAMLOOKBACK pair; R → d-pad Up bit → POVRELOAD.
- **The port** (`crates/nf-game/src/input/driving.rs`): `move_toward` (bit-exact `FUN_002104a8`),
  `CarInputStage` (the drain's input block + per-tick tail as a struct; car offsets in the field
  docs; the wheelspin machine and the camera-relative handbrake bit stay out — level context),
  `KeyboardMouse` + `to_raw_pad` (mode 0 so the pressure path runs), `pc_key_pad` (the synthesized
  bits/slots), `pc_driving_bindings()` (the binding table through the game's own evaluator), and
  `PcDrivingInput::tick(now)` — one 60 Hz step of the whole chain, returning the action messages
  and leaving the car inputs on the stage for the level loop/getters. The driving levels stay
  unwired (per the task, as in driving-input.md §12).
- **The table**: `PC_DRIVING_CONTROLS` (13 rows) in `crates/nf-view/src/menu.rs`, right next to the
  on-foot `PC_CONTROLS` — (key, the game's action label id from USATxt, fallback). The driving
  labels are the game's own controls vocabulary block: bank 0, ids 0x30a Gas … 0x315 Pause, plus
  0x100032a Brake/Reverse [M, parsed from the disc's USATxt.dat]; no secondary-fire or reload label
  exists in the whole 2801-entry table [M-negative], so those two rows are id-0 fallbacks exactly
  like the on-foot table's port-only rows.
- **Fixed a pre-existing master-side break** found on the way: `input/driving.rs`'s test module had
  a syntax error since G-004 (`bindings: vec!sg(0),` — a `vec![msg(0)` with the `[m` eaten;
  present on origin/master, `cargo test -p nf-game` could not compile). One-line fix, obvious
  intent restored.

## How it was verified

All headless, numbers-only, each with a sister assertion that can fail (docs/systems/driving-pc-input.md §6):

- **nf-game lib 221/221** (`cargo test --offline --release -p nf-game --lib`): the ramp goldens
  (0→1 in exactly 4 ticks with the 0.25/0.5/0.75 steps, the ±1 sweep in 8, release back in 4, the
  ±0.2 chop write-back, the strict 0.9 full-lock edge, the +0x438 override), the gas ×2 clamp
  (0.3→0.6→1.0; digital 1.0 full from tick 1), the tie exclusion (equal gas/brake zeroes the gas),
  the handbrake zeroing (and the descriptor's keep-drive override), the HUD gate on the fire sets,
  and the key chain: each of the 15 keys' first tick produces exactly its action id with the
  PollDevice-normalized value; holds never repeat over 10 ticks; releases produce exactly the
  RELEASE ids; W+D end-to-end through 5 ticks then released (steer 0.25→1.0→0.75→0.5→0.25→0.0,
  throttle 1.0→0.0). 221 = the 201 pre-existing + 20 new/updated (2 pre-existing expectation bugs
  in my own first draft fixed before push).
- **nf-game integration 407/0** (all 42 test binaries, `--tests`), including the merged driving
  physics/AI sets (drive_physics 13, driving_ai_follow 9, driving_ai_physics 8) and the input sets
  (input_ramp_live, input_styles_live) — my change sits in that crate and regresses nothing.
- **nf-formats 79/0, nf-audio 9/0** (the other two crates of the rule-5 line).
- **nf-view lib 15/0; menus_flow 37/0** (menu.rs's own consumers); mp_menu_flow, dev_menu_flow,
  dossier_pages, campaign_flow_live, fix_progress_live 29/0.
- **driving_controls_live 1/1** (`crates/nf-view/tests/driving_controls_live.rs`, `$NF_ISO`-gated):
  binds every non-zero `PC_DRIVING_CONTROLS` label id from the disc's real USATxt.dat and asserts
  the exact words ("Gas", "Steer Left-Right", "E-Brake", "Look back", …), with sister negatives
  (0x30a ≠ "Brake", 0x30d ≠ "Use Gadget", the unused "Cycle Weapon " keeps its trailing space) and
  an alignment check via 0x30b = "Brake".
- The branch is merged with origin/master (7ac86f13, the drive-ai-traffic2 batch — clean, no
  conflicts) and all the above ran on the merged tree.
- **Suite** (`tools/test/suite.py`): the background runner is reaped by this sandbox between tool
  calls (it died three times with an empty `suite.out` — the same thing the subtitle and dossier
  tasks documented). Foreground runs covered everything the diff can reach (the list above: the
  whole nf-game crate lib + integration, nf-formats, nf-audio, nf-view's lib + menu flows + the
  disc label test). One timed foreground suite attempt on the merged tree: build ok (369 test
  binaries, 338 s), then it completed **15 binaries / 110 passed / 0 failed / 0 TIMEOUT** before
  the tool-call time limit killed it (the remaining ~354 are the long nf-view `_live` render sets
  that have not fit any worker window here — flagged for the next window, as in previous
  reports).

## What's left

- The main agent's draw pass for a driving controls page: consume `PC_DRIVING_CONTROLS`
  (crates/nf-view/src/menu.rs) — data only was delivered, by design.
- The driving level loop stays unwired (per the task): `PcDrivingInput::tick` is the entry a level
  calls per 60 Hz step; nothing in level code calls it yet.
- The shipped `Master.def`/`FrontEnd.def` binding data (inside the mission VIVs) remains
  unextractable on the slim ISO — the PC table is the port's design [S], evidence-tagged per row in
  the doc.
- Mouse-look camera orbit in the car (the game's right-stick CAMROTATE\* axes) has no PC key by
  design — the task's camera actions are CHANGECAMERA (C) and CAMLOOKBACK (Tab/MMB).

## Live checks for the main agent

These settle what the sandbox cannot (addresses from the research; car pointer at
`*(u32*)(0x339190+0xdcc)` [M FUN_00159d10]):

1. **D-pad steer negative** [M-negative claim to confirm in play]: in a driving mission, press
   only d-pad Left/Right and watch the car's raw steer `car+0x2c4` (read, per frame). Expected:
   **no change** — the digital steer actions are dead ids. Confirms the shipped config steers by
   stick only, which is what justifies mapping the PC arrows through the analog axis.
2. **GAS binding shape**: hold analog gas at a mid value with the stick still, watch `car+0x278`
   (read, per frame). If it walks 0.3 → 0.6 → 1.0 the shipped GAS binding is change-gated (the ×2
   clamp compounds); if it sits at 2·v the binding re-sends every frame. The PC's digital W is
   unaffected either way.
3. **Steer ramp feel**: tap and release the left stick full-left, watch `car+0x240` (read, per
   frame). Expected: 0.25-steps toward −1.0 over 4 frames, back to 0 over 4 — the exact shape the
   port's `CarInputStage` reproduces for the A/D keys.

## ntfy

(the notification is sent after this push; any failure's exact error would be recorded here.)
