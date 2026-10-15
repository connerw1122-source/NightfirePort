# fix-dev-spawn-visible — the dev window's NPC spawns must show up and fight

Task 109s. Branch `s/fix-dev-spawn-visible` (from `origin/master` 2e5a162b).

The main agent's dry run on master, `nf-view <iso> 07000005 --dev`, Bond at the spawn (105, 25, 109), looking at the snow a few metres ahead: both **"spawn at crosshair"** (`MenuAction::Spawn` → `Dev::spawn_guard`, the log's `[dev] guard #16: spawned (normal) at (101.2,24.3,106.4)`) and **the skin rows** ("skin 05000030 (1/3)", `MenuAction::SpawnNpcSkin` → `panel::spawn_npc`, the pool dropping to 0/3) put no guard anywhere near Bond. Headless NF_ISO tests through the real dev path (`dev::apply_menu_action_to_scene` + `Sim::step_bond`): `crates/nf-view/tests/fix_dev_spawn_visible_live.rs` (new, 2 tests).

## 1. The causes (each measured, not guessed)

### 1.1 "spawn at crosshair": a Character with a palette but NO VERTICES

`Dev::spawn_guard` pushed a fresh `Character` (skin + skeleton + matrix) after the build. The renderer's character geometry is uploaded at build time only — the Character-layer draw groups become `Scene::vertices`/`char_verts` (`build`'s group loop, the `v.character` tag), and `Renderer::new` (rebuilt on the count change, `App::apply_menu_action` in lib.rs) re-uploads **the same buffer** and extends only the palette layout (`char_base`). So the added character got palette slots while **zero vertices** carried its index — the GPU draws nothing no matter what the palette carries. Headless proof: the added character has no `char_verts` entries while every pool copy carries ~4.8k.

### 1.2 Both rows: the drone's ROOT was placed AT ground height — 1 m underground

The port's drone ground convention is `pos.y = floor + hip` with hip 1.0 [M `Drone_CollisionHandler` 0x140968's port snap: `pos.y = floor + self.hip()`]. Both spawn paths set the root at a surface height instead:

- the crosshair row: the look-ray's world hit (the snow surface) — the feet land 1 m under the snow;
- the skin row: `floor_below(...)` returns the floor height itself.

From there `feet_on_point`'s grounding ray (1.6 down from the root, [M `NDrone2_DoCollision`]) starts **past** the surface, misses it every tick, and the guard falls through the world onto whatever is far below — the measured skin-row case settles at y 16.9 under Bond's 24.2 floor (the probe: the palette carrying the guard at (101.5, **16.9**, 107.0)). This is the main agent's "no guard is visible **there or anywhere around Bond**": the guard was inside the terrain. (My first crosshair-row run masked this by chance: with a level look the ray misses and the eye-height fallback lands just above the floor.)

### 1.3 The skin row: the floor pick fell through the bank

`panel::spawn_npc`'s `floor_below(eye+1, depth 20)` scanned 20 units down from the eye and could pick a surface ~8 m below Bond's bank (and the terrain drops ~8 m between 3 and 4 m out at the 07000005 spawn — the candidates' probes). A guard spawned there is inside the hill.

### 1.4 "spawn at crosshair": the drone init had no real parameters

`spawn_guard`'s hand-rolled closure `|k| match k { 0 => skin, 16/21 => mode, _ => 0 }` fed `Drone::new` zeros for every other key — param 9 (the weapon selector), param 14 (the sight class), param 11 (the alert block) — while the level loader's own init reads them from the placement block (`Drone_Create` 0x136be0 copies `placement+0x2C`; `NDrone2_DefaultInit` 0x14b300). The guard spawned with weapon selector 0 and the default sight class.

### 1.5 The AI was never broken — the spawn just couldn't fight from inside the hill

With a guard standing ON the ground, the normal-mode sight works through the game's own rules: Bond starts in a 0.25-shadow AI volume [L WORKLOG 0032 — `visibility_factor` caps every guard's score at ~0.4 < 0.7 at 4 m], and inside 2 m the score is the guaranteed 1.0 [M `DroneVision_OpponentVisibility` 0x175af0's dist < 2 arm] — so the test walks Bond up to the guard and he sees, aims (0x67/0x68) and fires. No AI code changed.

### 1.6 The strict portal walk drops the guard's room — the palette now trusts a live drone

The skin-row guard settles into a room whose cell (31) is in **neither** visibility mask from the eye's cell (14) — measured: game 75/137, strict 5/137 — while the player looks straight at him 4.4 m away. The port's walk is the approximation that already needed `near_los_rescue` for the World draws (fix-portal-sweep: "the near unoccluded generics the walk drops where the port's per-cel attribution approximation disagrees"). `skin_palette` now exempts a **live drone's** character (present + enabled) from the strict room cull — the palette trusts the position and the GPU clip, the hidden/present collapse still removes dead and disabled ones. (scene.rs, one additive condition, the cited trade: the game culls objects per frame at their own world sphere [M `View_DrawObjList` 0x1e69a0 over the VisCelList; the object linked to the room `build_FindCel` 0x1c8cb0 picks at its position]; the port's per-position lookup + this exemption stands in for it on live NPCs.)

## 2. The fixes

- **`dev.rs::spawn_guard` rewritten onto the NPC pool** (the skin-row path's mechanism — the port's own spawn path): the skin of the first active guard → a free pool slot → `Drone::new` with the SLOT's real placement params (only 16/21 carry the mode, 3/4 zeroed — exactly `panel::spawn_npc`) → present + enabled + llist → the slot's characters bound through `Sim::char_drone` (+ the ambient reset). The fresh-Character push is gone (1.1), the zero params are gone (1.4), and the guard **faces Bond** (`yaw = atan2(eye − pos)`, the skin rows' own convention) so his sight cone can take the player. No renderer rebuild is needed any more (the character count never changes).
- **The root convention in both rows** (1.2): the crosshair row lifts the ray hit by the hip (`pos[1] += 1.0`); the skin row spawns at `floor + 1.0`.
- **The skin row's ground pick** (1.3): the candidates walk IN from 4 m (4.0/3.0/2.0/1.2) and take the first whose floor sits within 1.5 of Bond's, falling back to a spot beside Bond at his own floor.
- **`scene.rs::skin_palette`**: the live-drone exemption (1.6).

Files: `crates/nf-view/src/dev.rs`, `crates/nf-view/src/panel.rs`, `crates/nf-view/src/scene.rs` (shared — the one additive condition, listed here), `crates/nf-view/tests/fix_dev_spawn_visible_live.rs` (new). `drone.rs`, `player.rs`, `lib.rs` untouched.

## 3. Fail-first and the sister

The test's stages separate every candidate (on the floor / in front / in a room / posed / in the palette / mesh-in-the-buffer / fights):

- Pre-fix, the skin row fails at the **floor** assert (the guard settles 8 m under the bank); pre-fix, the crosshair row fails at the **fights** assert ("the spawned normal guard never noticed Bond (state 0x4, drawn true, no bullet, no damage)") — both shown on the unfixed tree.
- The sister for 1.1: the bound character's mesh must exist in `char_verts` — the old post-build Character fails it (no vertices), the pool copy passes (~4.8k verts).
- Post-fix, both tests pass: the crosshair guard's palette carries (100.8, 25.9, 106.1) and **he shoots Bond — bond health 100 → 58.7, the guard in 0x68 AIM_STAND_FIRE at tick 637**; the skin-row guard's palette carries (101.6, 25.0, 106.6) and **he fires at tick 2145** (0x6d AIM_BACKOFF — backing off while shooting).

## 4. The frame render

This sandbox has no GPU adapter (lesson 6; `--shot` exits with "No suitable graphics adapter found" — lavapipe is not loadable here), so the frame is described headless: `skin_palette` with a full layout carries the spawned guard's object matrix at (100.8, 25.9, 106.1) — standing on the snow ~4.6 m in front of Bond's spawn, inside the spawn camera's frustum, posed by the drone's anim (frame_state's `posed[ci] = Some`, `hidden[ci] = false`). What the main agent should see at the `--bond` spots below: a guard standing on the snow bank facing Bond, who draws his weapon and fires when Bond walks up.

## 5. Verification

- `fix_dev_spawn_visible_live` **2/2** (the real dev path + `step_bond`).
- The sweep (the touched files' blast radius): `walkreal_07000005_live` 4/4, `walkreal_0700000c_live` 1/1, `fix_ladder2_live` 4/4, `fix_swim_live` 4/4, `fix_portal2_live` 3/3, `fix_portal3_live` (2 ignored — pre-existing), `fix_portal4_live` 4/4 — all green.
- The full fast suite on this tree: **(total lands at the finish commit)**.

## 6. What's left

- The pool has NPC_POOL = 3 slots per skin; both rows now draw from it. "spawn at crosshair" repeated more than 3 times per skin returns "no free NPC" until ClearNpcs — acceptable for the checks, flagged for a follow-up if the playtests want more.
- The strict-walk gap itself (cell 31 invisible from cell 14 at 4.4 m) is unchanged for non-drone objects; the exemption covers live NPCs only.

## 7. Live checks for the main agent

PCSX2 (the game has no dev window; these confirm the port's stand-ins against the game's own spawner):

1. A spawner-driven guard appearing near the player (`07000005`'s alarm): the guard draws ON the ground and fights — the same `Drone` init path the port now uses for the dev rows.
2. `tools/trace/gdbwatch.py` on the spawned object's `+0x30` (the position) writes around `Action::Spawn`'s object creation: expected the root at `floor + hip` above the ground the moment it spawns (never at/below the floor).

nf-view (`--bond x,y,z,yaw,pitch`, after `--dev` + one "spawn at crosshair"):

- A. `--bond 104.95,25.68,108.83,0.9,0.0` — the spawn camera: a guard on the snow ~4.6 m ahead, facing Bond, drawn (the palette's (100.8, 25.9, 106.1)).
- B. The same spot after walking Bond to the guard (hold forward ~2 s): the guard aims and fires — Bond's health drops.
- C. The skin row (`Enter` on "skin 0500002e (1/3)"): the same picture from the pool slot — the guard in front, not inside the hill.
