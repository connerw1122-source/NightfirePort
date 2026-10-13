# The sky clouds (placement type 0xd4, class 0x24) and the rest of the sweep's leftovers

`Cloud_Create` (0x18d620) / `cloud_update` (0x18d768). The draw already existed (the sky pass:
`Level::sky_instances`, nf-formats/src/level.rs — slot = param 4); fix-world-fx adds the DRIFT sim
in `crates/nf-game/src/world_fx.rs`.

## Cloud (0x18d620 / 0x18d768)

`control_create_object(0x18)`; data[0] = param1·0.001, data[1] = param2·0.001 (the x/z drift per
tick), data[2]/data[3] = the phases (0), data[4] = param0 (the height over the player), data[5] =
param3 (the fmod wrap); `View_SetDrawInNoViews` + `View_AddSkyObj(slot = param 4)` [M 0x18d620:20-31].
Per tick: `phase = fmod(phase + drift, wrap)`; the object parks at `player + (phase_x, height,
phase_z)` [M 0x18d768:20-31]. No RNG, no channels. The port accumulates and wraps the phases in
`WorldFx::tick`; applying them to the sky-pass matrices per frame is the listed follow-up (the
drift rates on 07000006 are 0.001-0.002/tick — sub-pixel per second at cloud distance).

## OneSided (0xd5) — already complete

`OneSided_Create` 0x1e7ce8 / `OneSided_Update` 0x1e7d58: a per-frame backface self-cull (dot of
obj+0x30 − viewer with the facing dir < 0 → the 0x10 hidden bit). The port's triangle culling is
the same rule (nf-view/src/scene.rs MISC_KINDS comment); no sim state exists to port.

## Apocalypse (0x4a, class 0x54)

`Apocalypse_Create` (0x18dcc0) / `Apocalypse_Update` (0x18d4f0). Ported in `world_fx.rs`:
data[0] = data[1] = param1·60 (the remaining/total ticks), +8 = param0 (the latch channel), +0xa =
param2 (rumble), +0xc = param3 (whiteout), +0xd = param4 (flashbang), +0xe = the latch [M
0x18dcc0]. Once `switch_channels[param0]` the latch sets for good; the countdown runs, the
whiteout value is `remaining·5/total` (viewer+0x200), rumble/flashbang feed their systems, and at
0 the object self-destructs [M 0x18d4f0]. The port carries `remaining/total/armed/fade`; the
renderer's screen-fade overlay is the listed follow-up. On 07000014/07000015 (levels.md §7.1).

## EnvTree (0xf3) — draw exists, sway is a follow-up

`Env_Tree_Create` (0x17c200): class 0x3f, `AnimObjectNew` + `AnimScriptAdd(model = param 1; the
0x600008c placement 50-50 swaps to 0x600008d via `Rand_Random` [M 0x17c200:20-26])`, a random sway
phase `Rand_FRand(anim+0xb0)` → anim+0x90 (the shared stream — un-consumed [S]), flags −0x20 +0x80
(kind-0 lights). `Env_Tree_Update` 0x17c328 = `AnimObjectUpdate` unless ScriptCam. The port draws
the trees in bind pose already (the character skin path, levels.rs `character_instances` — the
placement's param 0 skin hash); the sway anim (a per-bone rotation loop) is the follow-up.

## MoviePlayer (0xf4) — a trigger, not a screen

`Trigger_MoviePlayer` (0x1b2a08) rewrites the placement into a kind-0xD TRIGGER (`Trigger_Create`
0x1b1720; param 1 = the movie hash → data+0x20, param 2 = the gate channel → data+0x26) [M
0x1b2a08:10-15]. `Trigger_Update` case 0xD (0x1b1a78:190-199): when the gate fires, the movie hash
goes to `GameState+0x16` and full-screen FMV states 6/7 push — the display is taken over; the
in-world screen texture NEVER receives frames [M]. The port's trigger kinds (triggers.rs) have no
0xD and there is no PSS player (docs/reports/fix-cutscenes.md: the demuxer exists,
nf-formats/src/pss.rs, no decoder) — wiring the placement = a 0xD trigger kind + the movie player,
out of scope here, recorded as the hook.

## SpaceLaser (0x38) / SpaceMissile (0x39) on 0700001b — not gameplay blockers

docs/reports/fix-finale-blockers.md:102-107: the finale completes via Drake's defeat NIS writing
ch 100 directly; the missiles' per-missile channels (21/22/27) are choreography/fail-path writers
whose launch-order chain (ch 19 = AND(0xc1, 0xc2)) has no writer on the disc data. Per the task's
rule ("if they block the mission, they're gameplay") they stay unported; the mechanics are
documented in docs/levels/0700001b.md and the decompile (Create_SpaceLaser 0x1aa8b8 /
SpaceLaser_Update 0x1aa3f8 — states 0-4, the script-driven aim/fire, 2 shared-RNG draws [M];
Create_SpaceMissile 0x1a9ef0 / SpaceMissile_Update 0x1aa118 — the state jump table at 0x2fcc30 the
decompiler dropped).
