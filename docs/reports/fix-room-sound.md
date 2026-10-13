# fix-room-sound — the room reverb and the guards' foley calls (G-016/G-032 audio half)

Branch `s/fix-room-sound`, cut from origin/master 7a614ba, 2026-10-13, session 22:04–23:43 UTC.
Item 1 (the room environment) is landed and pinned; item 2 (the foley call sites) is landed for the
kick, the civilian patrol, the spawner and the pain pools; the interrogation and hostage-die states
are researched to the arm level and need new state machines (evidence below). One upstream repair:
master's `scene.rs` carried two corrupted lines (dropped `[m`/`[h` byte pairs in the nav-remainder
sphere closure) that broke the build — fixed in the merge commit 2dc44ad.

## Done: the room environment (`Sound_UpdateListeners` 0x1dd278 + `parseentity_fixup_entity` 0x1d00c0)

- **The parse** (`crates/nf-view/src/scene.rs`, `Cel` + `parse_cel_graph`): the room cels now carry
  `env: i16` (cel+0x98) and `underwater: bool` (the `cel+0x3c & 0x10000000` bit). `0xc023` → env
  0x9898 (the IOP's −26472 underwater sentinel); `0xc047` → env = placement param 9
  (`level_tag+0x50`) as a raw u16, no mask or clamp [M 0x1d00c0: `*(u16*)(cel+0x98) =
  *(u16*)(level_tag+0x50)`]; the underwater flag = param 3 (`level_tag+0x38`, a full u32) != 0,
  both classes. The params are read through the raw 32-bit values, not the low-byte view the
  ambient triple uses.
- **The feed** (`crates/nf-view/src/lib.rs` `sound_tick`, now `pub` for the headless tests): every
  frame, from the cel under the listener's eye — `SFXSetEnvironment(cel.env, underwater =
  cel.underwater || cel.env >= 20)` (a signed s16 compare [M disasm 0x1dd4b0–0x1dd4fc]) — and
  **nothing when the viewer has no cel** (the game skips the send and the IOP keeps its previous
  targets). The hardcoded `drv.set_environment(0, 0)` G-016 stand-in is gone. *Deviations*: the
  port's single viewer is Bond's eye (the script-camera listener call is dropped by the EE batcher,
  as before), and the `glb_viewer[4]`+0x29 script-camera override is not modelled (the port has no
  viewer array); sub-map cels are not in `Scene::cels`, so a viewer inside one sends nothing where
  the game would send the 0x98 fill (0x9898 sentinel) — the port's room system has modelled it
  this way everywhere else.
- **The IOP mapping pinned by the test**: the EE's "underwater" second argument lands in
  `SFXNewIndoorsVolume` (50 = the muffled mix) [M SFX.IRX 0x0db4: `param_2 ? 50 : 100`]; the
  driver's `SFXUnderWater` flag comes only from the 0x9898 sentinel. `Driver::environment()` (new
  public accessor) returns (reverb target, underwater, indoors target).
- **Pinned against the snapshot** (`fix_room_sound_live`, NF_ISO): 07000005 has exactly 19 room
  cels — 15 × (env 0, dry) and 4 × (env 40, underwater bit) [L work/ee_start_05.bin: flags dwords
  0x0804c047 ×15 / 0x1804c047 ×4, env s16 at +0x98 = 0 / 40]. The second test drives `sound_tick`
  end-to-end: the spawn cel's value reaches the driver, and standing in an env-40 cel sends
  `SFXSetEnvironment(40, 1)` → reverb target 40 + indoors 50 (the pre-fix code sent (0, 0) there —
  the fail-before).

## Done: the foley call sites (`crates/nf-game/src/drone.rs`, additive; spawner hook in `Drones::tick`)

- **Kick 0x28** [M 0x1684c0 case 0x20, disasm 0x16857c-90]: fix-nav-remainder had already landed the
  state and the msg-0x20 arm, but played the line as `anim_sounds.push(0x28)` — a raw id through the
  SfxTable (sound 40) instead of `Drone_SfxInfo[0x28]` → sfx 1478 through the `NDrone2_PlaySFX`
  gates, and the `pending_door_msg` drain played the door line 0x27 unconditionally even for a kick
  anim's event 6. The drain now picks the line by the drone's state (OpenDoor → 0x27, KickObject →
  0x28 — the game's case 0x20 is where the line lives) and the raw `anim_sounds` pushes (0x27/0x28)
  are removed. `fix_npc_doors_live` and `nav_kick_live` (incl. the kick-route end-to-end) stay green.
- **Civilian patrol 0x48** [M 0x15d610:60, disasm 0x15d754-a4]: the UPDATE arm's armed-person branch
  queues 0x48 (sfx 924, repeat 300) when `Drone+0x15` (the port's `female`) is set, before the
  scare-state switch. The game's CivilianInit carve-out (`+0xd8 == 0x10` → no scream) has no port
  counterpart — the port's CIVILIAN_PATROL is never entered from CIVILIAN_INIT. Drained through the
  new `foley_reqs` outbox → `foley_entry` (the shared 30-frame gate, the entry's repeatDelay, the
  stamp-before-play).
- **Spawner wave announce 0xd6+Rand(2)** [M 0x137e68, disasm 0x1382fc-0x138324]: mode 3, first wave
  (`wave == 0` = the game's `+0x2C == 0`), none alive, on the power-station pair (0x0700000C/0xD)
  draws `Rand_Rand(2)` and plays 0xd6+draw (1353/1318, repeat 600) at the zero position with DCVars
  NULL → ref 0. The draw precedes the `NDrone2_PlaySFX` call in the game, so the port draws before
  `Spawner::update`'s own template draw — the stream order matches; the 600-frame repeat gate may
  then suppress the play, exactly as the game's.
- **The pain pools** [M 0x125ed8, disasm 0x1260c8-0x1261a4]: `Drones::bot_handle_pain` +
  `pain_pool` implement the verdict/gender pick — death pool male 0x154..0x156 / female 0x157..0x159
  (`Rand_Rand(3)`, no playing gate), pain pool male 0x14d..0x151 (`Rand_Rand(5)`) / female
  0x152..0x153 (`Rand_Rand(2)`), one draw per call before the play, the previous grunt stopped and
  the handle stored (`pain_grunt`, the `comm+0x73c` stand-in). **Not wired into the hit path**: the
  game reaches `BOT_handlePain` only under `MPSettings+0x388` (the bots flag [M 0x145558:15-17]) and
  the port is SP-only — `NDrone2_HitDamage`'s SP arm is `NDrone2_PainTalk` (`pain_talk`), which is
  what the port runs. Consequence: **no `Rand_*` draws were added to the live stream, no seed-pinned
  test moves, and the WORKLOG 0072 re-pick is not needed.** *Deviation*: the "previous grunt still
  playing" gate (`Sound_Playing_Ref` 0x1de3a0) needs the sound pool's node state, which nf-game does
  not expose — the port stops the previous handle and always plays. Unit test
  `bot_pain_pools_pick_by_verdict_and_gender_and_draw_once` pins the four picks against the live
  table values and the draw-once accounting through `foley_entry`.
- Unit test: nf-game lib 135 passed / 0 failed (the 134 pre-existing + the pools test).

## Researched, not landed (the evidence is in the decomp; each needs a new state machine)

- **Interrogation 0x4a** [M 0x161a40 = `NDrone2_DSTATE_Interogate` 0x3C, case 3/UPDATE]: the latch
  `drone+0x228 & 0x10` (recomputed via `DroneWeap_OpponentHasWeapon` both branch paths), then
  `OpponentHasWeapon != 0` → `PlaySFX(1.0, DCVars, 0x4a, 0, 1)` (sfx 192, delay 0) and straight to
  Attack 0x56. The port has no INTERROGATE state (only the global interrogate latch at drone.rs's
  `party_loud`); the other 0x4a call in the same case is a `NDrone2_SpeechSFX` voice line, not foley.
- **Hostage-die 10** [M 0x1725a0 = `NDrone2_DSTATE_HostageDie` 0x0B, ENTER only]:
  `AlertStatusSet(3)`, `PlaySFX(1.0, DCVars, 10, 0, 1)` (sfx 581), `CallAnim(0, 0x40, …, 0xf, …)`,
  `NDrone2_LogDroneDeath`. No death-type/health gate — the state is the die; reached via the mode
  tables and the HostageKiller chain (docs/fields/dronetype-tables.md row 08/0B), neither ported.
- **Exhaustiveness**: the six-list is complete for fixed one-shot foley ids. Two extra sites found:
  `NDrone2_DSTATE_SeenSurrenderedDrone` 0x16ade0 draws `Rand_Rand(6)` on EVERY ENTER and plays 0x23
  only on a 0 — but `Drone_SfxInfo[0x23].sfx == 0` (a dead entry [L]), so the play always no-ops;
  the net effect is a 1-draw RNG drain per entry into state 0x9f (not ported — if/when it lands, the
  draw must come with it). `BOT_soundEffect` 0x126618 is the pool-parameterized helper for the same
  pools (id 1 = death, 0x1f/0x1e = pain), called by the bot death states and `NDrone2_DealWithObjHit`
  case 0x27 — all MP-gated like `BOT_handlePain`.

## Verification

- `fix_room_sound_live` (new, NF_ISO): 2/2 — the 07000005 cel census [L] and the end-to-end
  `sound_tick` → driver feed (the fail-before is the env-40 leg).
- `nf-game --lib`: 135 passed / 0 failed (the pools unit test included).
- `nf-game --test nav_kick_live` 5/1-ignored and `--test ai_live` 35 passed (the kick-route and the
  AI paths over the changed arms); `nf-audio --lib` 7/7.
- **Not run this session** (the window closed first): the full nf-formats / nf-view sweeps. The
  nf-view delta is `sound_tick`'s env feed + the `Cel` fields (covered by the new live tests); the
  nf-formats delta is none. The next session should run the full suite before merging.
- Determinism: the spawner announce adds `Rand(2)` draws on 0700000C/0xD only, on the mode-3
  first-wave none-alive tick; the pain pools draw only through the test. No seed-pinned test on the
  affected path regressed in the suites run (ai_live/nav_kick_live include 0700000C probes — green).

## Shared-file hooks

- `crates/nf-view/src/lib.rs`: `sound_tick` made `pub` (the headless tests drive the loop's exact
  function; the fix-gpu-skinning precedent) + the env feed inside it.
- `crates/nf-audio/src/sfx.rs`: the `Driver::environment()` test accessor (additive).
- `crates/nf-game/src/drone.rs`: the `foley_reqs`/`pain_grunt` fields (own lines, both
  constructors), the civilian arm's queue push, the drain block in `Drones::tick`, the spawner
  announce before `Spawner::update`, the two msg-0x20 arms' line fix, `bot_handle_pain`/`pain_pool`.
- `crates/nf-view/src/scene.rs`: the `Cel` fields + the parse fill (also repaired master's two
  corrupted sphere-closure lines in the merge commit).

## Live checks for the main agent

- PCSX2, 07000005 (the Deep Water level): standing in a normal room the reverb target is 0; wading
  into an underwater room (one of the four 0x1804c047 cels) the mix muffles (indoors volume 50) and
  the reverb ramps to 40 at +16/frame. In nf-view: `--bond` into one of the env-40 cels (their
  bboxes are in `Scene::cels`; the test's scan points work) — same expectation, no visual change.
- PCSX2, 0700000C/0xD (the power-station pair): on the first spawn wave with none alive, one grunt
  (sfx 1353 or 1318, alternating by the RNG) plays from the world origin once per 600 frames.
- nf-view, a female civilian patrol (07000007's garden civilians are the usual female skins):
  walking armed within 6.0 of one plays sfx 924 before she flees; a male civilian stays silent.
- The kick grunt (0700000C's kick-route drones, `nav_kick_live`'s route): one 1478 grunt at the kick
  anim's impact, and NO sound-40 blip (the removed raw play).

## What's left

- The interrogation (0x3C) and hostage-die (0x0B) states — the arms are fully specified above; both
  need their state machines (the mode-table rows and the HostageKiller chain) before the foley line
  has a caller.
- The `Sound_Playing_Ref` gate for the pain pools once nf-game's sound pool exposes a node-state
  query; then wire the pools behind the MP bots flag when the port parses `MPSettings+0x388`.
- The full nf-view suite run (see Verification).
- Doc fixes carried over from fix-sound-remainder: sound.md §7's "op 2 / sub 3" → op 4 / event 3,
  and the `cel_tag+0x98` field in docs/atlas/cel.md (this session's `Cel` doc comments carry the
  addresses meanwhile).

## ntfy failed

All three transports returned the same daily-quota rejection (2026-10-13 ~23:42 UTC):
- `curl`: `{"code":42908,"http":429,"error":"limit reached: daily message quota reached; increase your limits with a paid plan","link":"https://ntfy.sh/docs/publish/#limitations"}`
- `python3 urllib`: `urllib.error.HTTPError: HTTP Error 429: Too Many Requests`
- `wget`: same 429 body (quiet output, exit 0 from the pipe — the server response was the 429 JSON).
