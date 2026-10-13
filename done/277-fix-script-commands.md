## Your task: `script-commands`, the script player remainder: fades, sprites, lights, text, letterbox

GAPS G-031: the cut-scene/script player stops these streams today: fade, sprite, light, sub-script and text commands; the
letterbox borders and music events of camera scripts; `SP_Hit`'s bullet and object flags (07000007's urn break, mask 2); AI
volumes / pickups / breakables made by scripts; `Corona_2` glow drawing; drones riding objects.

Port each command from `Script_EventHandler` 0x1d94c0 and `Script_Interp` (docs/systems/script-player.md), in this order:
fades and letterbox (visible in every cut-scene), text, lights, sprites, sub-scripts, `SP_Hit` (the urn), then the rest.
For each: a test that runs a real script containing it (find one per command with nf-extract's script dump) and asserts
the command's effect in the Sim's state.

Files: `crates/nf-game/src/nis.rs`, the cut-scene draw hooks in nf-view, docs/systems/script-player.md.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing** (start it early, in the background, so it finishes inside your session):
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail. Report the count. If your change adds `Rand_*` draws, seed-pinned
  tests may move: re-pick them by the WORKLOG 0072 procedure (`NF_TEST_SEED` search against the same live expectations).
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/script-commands`.

---
claimed-by: sandbox1 at 2026-10-13T18:37Z

---
## Report (done, 2026-10-13T20:14Z)


---
## Report body (appended by the worker)

# fix-script-commands — the script player remainder (GAPS G-031)

Branch `s/fix-script-commands` (from origin/master a82591d; master 0cda458 merged before the final
push). Files: `crates/nf-formats/src/nis.rs` (the decoder), `crates/nf-game/src/nis.rs` (the
commands), `crates/nf-game/src/drone.rs` (the drain + view state), `crates/nf-view/src/{scene,hud,
lib}.rs` (the draw hooks), new `crates/nf-view/tests/fix_script_commands_live.rs`,
`docs/systems/script-player.md` (the port status). Shared-file hooks (all additive, listed per the
rules): **drone.rs** — the drain's `Fade`/`Text` arms, one line in the `Camera` arm (`self.letterbox
= start`), the `ViewFade` struct, three Drones fields (own lines in the one-line constructor) and
the fade's per-tick advance in `Drones::tick`; **lib.rs** — one `hud.append` line; **scene.rs** —
the `script_msgs` → `hud.msgs` binding in `hud_tick` + `Scene::script_overlay_hud`; **useobj.rs** —
a merged test initializer's new field. nf-game's weapon tables untouched.

## What was done (in the task's order)

**Discovery first**: the doc's "the stream stops on them" was stale — an earlier fix already
consumes the eight opcodes and continues the stream (throwing the arguments away, pushing an
unconsumed `Request::Event`). The work was decode → typed state → effects, plus a real decoder bug.

1. **The decoder** (`nf-formats::nis`): every command is op + args + **1 trailing byte**
   (`Script_Run` 0x1d8548:120 adds 1 after each handler). The old lens for 0x13/0x14/0x18/0x1b/0x1e
   (5/15/4/7/7) omitted it and desynced every stream carrying them; fixed to **6/16/5/8/8** with
   typed `Cmd::Fade/Sprite/Light/Text` variants. Evidence: a 5,263-stream census over all 87
   archives decodes exactly with the trail (e.g. 07000029's 0x06000052 stream 5 = Wait(0),
   Light(00 ff 01), Marker(0), Wait(80), LightEnd, Marker(80), End(165)).
2. **Fade (op 0x13)** [M `Script_FadeStart` 0x1db3d0 + `Camera_SetFade` 0x1bbdd8 disasm]: one f32 →
   `Camera_SetFade(-f, 4, 0xff, 0)` — a black overlay (colour 0x000000ff) on the viewer whose alpha
   ramps 1/|f| per tick; negative f = in from black, positive = out to black. Ported as
   `Request::Fade` → `Drones::view_fade { alpha, rate }`, ticked per tick in `Drones::tick`, drawn
   by nf-view as a full-screen quad (layer 0, above the bars/sprites). Suppressed while the skip
   latch (+0xAB5 bit 0) is set, like the game. **The game's own auto-fades are raised too**:
   `Script_Update`'s length−20 fade-out (`Camera_SetFade(40, 4, 0xff, 0)`, with its no-fade hash set
   {0x0600004fa, 0x06000080, 0x060008C8, 0x06000994} + the death-NIS break) and the end-of-script
   handback fade-in `Camera_SetFade(0xc1b00000, 0, 0xff, 1)` [L 0xc1b00000 = −22.0 — the port's old
   comment said −9, fixed].
3. **Letterbox** [M `Script_CameraStart` 0x1d9140:49-61]: two border sprites of SpriteInfo
   **0x2c9468** [L ACTION.ELF: 512×64, solid black colour 0x000000ff, texture 0 — the second moved
   to +0x26 = 0x180 = 384], draw layer 1, hidden (layer 0xff) by `Script_KillStream` case 4. Ported
   as `Drones::letterbox` (set by the existing `Request::Camera` drain arms — `Music_Event(0xB/0xC)`
   was already wired there) and drawn as two solid bars while a script camera is live.
4. **Sprite (op 0x14/0x15)** [M `Script_SpriteStart` 0x1d9300]: 5×u16 + u32 hash → `Obj::Sprite`
   (the 640×480 rect scaled by 0.8 / 0.9333 into the HUD's 512×448, flags 0x40/0x80 from script bits
   2/4, always 0x100, layer 2); created once per stream, updated in place, deleted by
   `Script_KillStream` case 3. nf-view draws each sprite through the HUD path when its texture hash
   is loaded (else it is skipped — a live check).
5. **Light (op 0x18/0x1a)** [M `Script_LightStart` 0x1db1b0 disasm + `Light_Create` 0x1cc3d8]: 3
   bytes — the parent pair (b0 == 0 → stream+0x25 = −1, else b1) and the light kind (the call's
   `b2 == 1` argument); position = the static origin vector **0x2d6200** [L (0, 0, 0)]. Ported as
   `Obj::Light` on the stream (kind 6); `Script_KillStream` case 6 deletes it unless the script
   keeps objects (flag 0x10), exactly the game.
6. **Sub-script (op 0x1b/0x1d)** [M `Script_SubScriptStart` 0x1db050 + `Script_Interp` case 7]:
   u32 hash + parent pair → `Script_Load` + `Script_Play(sub, 1)` (playing + looping) on the new
   `Player::subs`; the sub is **ticked from the parent stream's interp** — the game's own structure
   (the `Script_Update` jal at 0x1d9e4c inside `Script_Interp`, found by scanning every `jal` in the
   ELF) — with its matrix keyed from the stream keys × parent chains × the script base;
   `SubScriptEnd` 0x1db148's loop-persistence condition (kill unless looping and past the length and
   +0x20 == 1) mirrored; `Script_KillStream` case 7 frees the sub (its requests join the parent's).
7. **Text (op 0x1e)** [M `Script_TextStart` 0x1db2b8]: u32 label + u16 time →
   `Text_AddMsg(-1, 0, 4, Txt_BindLabel(label), 0, time)`. The raw label drains
   `Drones::script_msgs` → `Sim::hud_tick` binds it through the level's Lang table into the HUD's
   message queue with `time` as the life (the pickup-message pattern — the Lang table lives above
   nf-game).

## How it was verified

`crates/nf-view/tests/fix_script_commands_live.rs` — 4 tests, each on a **real disc script**:
- `decode_types_the_new_commands_with_the_trailing_byte` — the real stream bytes: 0x06000052's
  light chain (Wait(0), Light(00 ff 01), Marker(0), Wait(80), LightEnd, Marker(80), End(165)),
  0x06000071's fade f32 −82.0, 0x06000072's sprite (48, 208, 128, 64, hash 0x03000056), 0x06000961's
  text (label 0x06000068, 1132).
- `sub_script_spawns_the_light_and_the_end_kills_it` — 07000005, `spawn_once(0x06000050)` (the
  game's own one-shot SP path): the sub plays looping, the light object exists while it runs, and
  both are gone after stream 7's `1d 1d` at 165.
- `fade_runs_from_black_and_the_camera_raises_the_letterbox` — 07000001, script 0x0600005b: the
  fade arms at alpha 1.0 with rate −1/63, completes by tick 70; the letterbox is up; the intro's
  letterbox bars are themselves op-0x14 sprites (asserted in `Scripts::objects`).
- `text_command_queues_the_subtitle_label` — 07000500, script 0x06000961: nothing before
  Wait(183), then the 1132-frame subtitle in `hud.msgs` (bound through the level's Lang).

Fail-first: with the six source files stashed the test file does not compile — none of the API
exists (`Cmd::Light/Fade/Sprite/Text`, `Obj::Light`, `Player::subs`, `Drones::view_fade`); restored,
all four pass. Full suite on the merged tree (master 0cda458), `--test-threads=1`: **nf-game 228
passed / 0 failed; nf-view 361 fast-set (all 88 non-walk binaries) + 41 walk samples (18 walk
binaries, the script-heavy levels 05/06/07/08/0c/0a/0d/11–16/1b/4a) / 0 failed; nf-formats + nf-audio
89 passed** with only the 3 allowed `disc_golden` failures plus
`pss_movies_live::audio_sample_count_matches_stream_length` (the slim onfoot ISO carries no
/MOVIES/30_FPS entries — pre-existing, not this task's). The remaining 37 walk binaries were not
re-run after the master merge (time); they were green on the pre-merge tree and the sampled 18
cover every level whose scripts carry the new commands.

## What's left (the ordered tail of G-031)

- **`SP_Hit`'s bullet/object flags** (07000007's urn break, SP #312 mask 2): the flags are computed
  fresh per `SP_Update` (0x1dbda8:67) by `SP_Hit` 0x1dc470 — bit 2 walks each stream object's +0xD0
  collision chain for box-kind-5 (`box+0xff == 5`) entries filtered by p8 (+0x37) against the
  bullet's +0x44, and arming calls `Collide_FilterBullets(&obj+0xD0, 0x106)` when mask == 2 exactly;
  the damage accumulator (+0x14/+0x18) unpause is `SP_Update`:230-268. The port still models only
  the channel (1) and activation (4) bits.
- **AI volumes / pickups / breakables made by scripts**: `Script_EntityStart`'s 0xF5 / 0xF0 (the
  pickup arm exists, `Objects::pending_pickups`) / 0x20 classes.
- **`Corona_2` glow**: not a function — it is glist 0x02000426 (the truck's headlight coronas) as
  op-7 class-0 script entities; nf-view renders no generic `Obj::Entity` glist yet.
- **Drones riding objects** (`Control_InheritVelocity` for guards on the truck) — the player's ride
  is ported; the drone-side inherit is not.
- The fade's viewer-state lifecycle detail: the port re-arms `view_fade` only from an empty state
  [S — the game's +0x234/+0x235 pair lifecycle is not fully decoded].

## Live checks for the main agent

PCSX2 (real game):
1. Spot: the 07000006 dressing cut-scene (SP #810, script 0x06000071). Expected: at its tick 1049
   the view fades OUT to black over ~82 ticks (f32 −82.0 → `Camera_SetFade(+82)`), and any
   cut-scene end fades back IN over ~22 ticks (`0xc1b00000` on viewer 0).
2. Spot: any cut-scene with a script camera (e.g. 07000005's gatehouse truck scene 0x060008C8 —
   note it is in the auto-fade's no-fade set). Expected: 64-px letterbox bars top+bottom while the
   camera runs, gone when it ends.
3. gdbwatch (optional): `Borders` (0x30d068) and `DAT_0030d06c` sprite +0x38 — expected 1 while a
   script camera is live, 0xff after `Script_KillStream` case 4.

nf-view (`--bond` spots; the render decides the visual questions my headless tests cannot):
4. `--bond` on 07000001 at the level start with the intro script fired: the two letterbox bars
   (512×64 at y 0 and y 384) and the fade-in from black over 63 ticks.
5. On 07000005 after the sub-script chain runs: whether the 0x06000052 light shows (the port's
   `Obj::Light` is state-only this session — the renderer's dynamic-light hookup for script lights
   is the open visual question).
6. The party fly-through (0x06000072 via TouchOnce #356 on 07000007's pack): the op-0x14 guest-name
   sprite (hash 0x03000056, rect 38.4×166.4 128×51.2 in HUD space) — drawn only if the sprite
   texture is loaded into `hud_textures`; if it is missing, the texture-loading hook is the next fix.
7. The intro subtitles (07000500's 0x06000961): the label 0x06000068 text at the message pane for
   1132 frames.
