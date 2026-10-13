## Your task: `fix-world-fx`, the presentation object types the port still skips

From `fix-levels-sweep`'s table (`docs/systems/levels.md` §7). These are visual or audio, but they're part of what makes
a level look like the game:

1. **0xe3 Flicker** (flickering lights), **0x4e Light** (level dynamic lights: coordinate with `050-fix-lighting`,
   which owns the light system, and only add the placement creation and the per-type update if it hasn't),
2. **0xf2 Emitter** (`Emitter_CreatePlist`, particles: smoke, steam, sparks), **0x21 Ripples** (water ripples),
3. **0xf3 EnvTree** (`Env_Tree_Create`), **0xd4 Cloud** / **0xd5 OneSided**, **0x4a Apocalypse**, **0xf4 MoviePlayer**
   (in-level video screens; the decoder is `220-fix-movies`'s: wire the placement to it, or leave a hook),
4. **0x38 SpaceLaser / 0x39 SpaceMissile** on 0700001b: check with `045-fix-finale-blockers`'s report; if they block
   the mission, they're gameplay, so port them fully.

For each: document it in `docs/atlas/<type>.md`, port the sim side (timers, spawn rates, RNG draws in game order), and
expose render data the way the snow does (`Scene` returns this frame's sprites or instances). You have no GPU: the
main agent checks the pixels, so list the spots to render in your report (`--bond x,y,z,yaw,pitch`).

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `triggers.rs`, `lib.rs`): small, additive hooks only, listed in
your report. Put new behaviour in new files. Pull `master` before you start and before every push: other workers
merge into it all day.

**Reproduce before you fix.** The user's play is the reference, and previous fixes passed tests but still failed in play. For each bug:
1. Write a headless NF_ISO test that drives the real input path (the `nf-view` input mapping into `Sim`, or `nf_view::load` plus `dev::scenario`), and show it reproducing the user's symptom.
2. Then find the cause in the decompile and fix it.
If you can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Update the doc entry** to say it's fixed, with your commit.

**Branch:** `s/fix-world-fx`.

---
claimed-by: sandbox6 at 2026-10-13T09:19Z

---
## Report (done, 2026-10-13T10:58Z)


---
## Report (done, 2026-10-13T10:59Z)

# fix-world-fx — the presentation object types

Branch `s/fix-world-fx` (from master cdfec70). The levels-sweep's §7 unhandled presentation types:
what got ported, what was already there, what is left.

## Done

**New `crates/nf-game/src/world_fx.rs`** — the sim side of five types, ticked from `Drones::tick`
(one additive hook, `drone.rs`: the `fx` field + `attach_level` + one `fx.tick` call):

- **0x21 Ripples** (`Ripples_Create` 0x1acc90 / `Ripples_Update` 0x1ac028): the periodic water-ring
  spawns through the existing `GasPool` — the game's own `Gas_Init` path, mesh 0x02000129, white
  rgba, scale 0.05..0.10, the both-0 defaults (spread 1, period 0x1e).
- **0xe3 Flicker** (`Flicker_Create` 0x1acb00 / `Flicker_Update` 0x1ab960): the finding of the task
  — `Light_FindMapLight` 0x1cc3d0 is an **8-byte stub returning 0** on this build, so the game's
  flicker lamps just fade their tint to white and stamp their signal channel 0 forever; the spark
  branch is unreachable AND the psi particle sim it would drive is stubbed out (0x111290/0x111290)
  [M]. The port reproduces exactly that.
- **0xd4 Cloud drift** (`Cloud_Create` 0x18d620 / `cloud_update` 0x18d768): the fmod-wrapped phases
  accumulate per tick. The static sky-pass draw already existed (levels.rs `sky_instances`).
- **0x4a Apocalypse** (`Apocalypse_Create` 0x18dcc0 / `Apocalypse_Update` 0x18d4f0): the channel
  latch, the countdown, the whiteout value `remaining·5/total`, the self-destruct at 0.
- **0xf2 Emitter** (`Emitter_CreatePlist` 0x17b848 → `Emitter_Create` 0x17ae00 / `Emitter_Update`
  0x17b080): the full particle sim — def lookup, the rate modes (−1 continuous / 0 one-shot / n
  seconds), the on/off/out channels, the blink period, batch-capped respawns, the never-spawned-
  slots-only one-shot rule, gravity + integration, `ColourGetValue` 0x17a938 colour-over-life, the
  out-channel drive, the 3-all-dead-ticks death. The defs parse off the disc: map block 0x30
  (`Emitter_LoadDefs` 0x17ab40) → `nf_formats::level::EmitterDef` (nf-formats/src/level.rs).

**Render handoff** (the snow pattern): `WorldFx::particle_sprites()` → `Sim::particle_quads`
(nf-view/src/scene.rs) appended to the snow billboards in `snow_for` (nf-view/src/lib.rs). The
game draws emitters through the same `psiDrawParticleList` 0x122d50 as the snow [M].

**Docs**: `docs/atlas/emitter.md`, `docs/atlas/ripples.md`, `docs/atlas/flicker.md`,
`docs/atlas/cloud.md` (cloud + Apocalypse + OneSided + EnvTree + MoviePlayer + SpaceLaser/Missile
in one file).

## Already covered by other tasks (no change)

- **0x4e Light**: 050-fix-lighting owns it — the placement creation (`placement_light`) and the
  per-type flicker update are in `crates/nf-game/src/light.rs`, wired via drone.rs.
- **0xd5 OneSided**: the update is a backface self-cull = the port's triangle culling.
- **0x4a Apocalypse mesh / 0xe3 mesh**: the MISC_KINDS static draw.

## Not ported (documented, with the reason)

- **0xf4 MoviePlayer**: it is a trigger that pushes full-screen FMV states (0x1b2a08 →
  `Trigger_Update` case 0xD) — the in-world screen never receives frames [M]. No PSS player exists
  (the pss.rs demuxer does); wiring = a 0xD trigger kind + the 220-fix-movies decoder, which left
  no master artifact. Hook recorded in docs/atlas/cloud.md.
- **0x38/0x39 SpaceLaser/SpaceMissile (0700001b)**: fix-finale-blockers's report proves the mission
  completes without them (Drake's NIS writes ch 100); they are choreography + the fail path →
  per the task rule they stay unported; mechanics documented.
- **0xf3 EnvTree**: the trees draw in bind pose (the character skin path); the sway anim is the
  listed follow-up.

## Verification

- Fail-first live tests, `crates/nf-view/tests/world_fx_live.rs` (NF_ISO): the block-0x30 defs
  parse and every 0xf2 placement's def hash resolves (07000002/14/15 — failed before the parse
  existed); ripples leave rings in the pool after 600 real-path ticks (07000001); emitters produce
  particles with their channels driven (07000014); the apocalypse countdown latches and runs
  (07000015); the cloud phases drift and wrap (07000006). All 5 green.
- Unit tests in `world_fx.rs` (6): the colour lerp, the flicker white ramp + channel, the ripple
  spread/scale, the emitter spawn/integrate/out-channel/one-shot-death, the apocalypse latch/fade,
  the cloud wrap.
- Full suite green except the 3 allowed `disc_golden` failures (slim ISO); one test updated:
  `models_misc_live.rs` firing test picks its muzzle puff by type-7 (the pool is shared with the
  ripples now — the game's pool is shared too [M Gas_Init]).

## Deviations [S] (the RNG discipline)

The game draws from the shared stream for the ripples (1 gate + 3) and 4 per emitter spawn — the
port consumes NONE (the bit-exact stream must not gain draws, the gas.rs/searchlight precedent) and
derives each from the per-object counter hash. The emitter sprite texture (def+0x04) is not bound
(the quad colour is the slot rgba); the sprite size is the world-space half-extent.

## Shared files touched (additive, per the rules)

- `drone.rs`: the `fx` field (own line), the `attach_level` call, one `fx.tick` call.
- `scene.rs`: the `particle_quads` method (new, additive).
- `nf-view/src/lib.rs`: `snow_for` appends the particle quads.
- `nf-formats/src/level.rs`: the block-0x30 parse + `EmitterDef` + the `emitter_defs` field.
- `models_misc_live.rs`: the two discriminator lines noted above.

## Renders for the main agent (`--bond x,y,z,yaw,pitch`)

- 07000001: water-edge spots (the 0x21 ripples — levels.md §7.1 lists the level; pick a pool edge,
  e.g. the courtyard water near the start) — expect faint white expanding rings on the surface.
- 07000014 (the fusebox vent level): the 0xf2 emitter spots — steam/sparks where the placements
  sit (the sweep's unresolved list); with their channels fired.
- 07000006: the sky (look up) — the clouds are the existing sky pass (unchanged draw; drift is
  sub-pixel).

## Live checks for the main agent

None required — the PS2-side questions this task raised (FindMapLight stub, the stubbed psi
particles) were settled from the decompile + ELF dump. If a render looks wrong, the first check
would be `Emitter_Draw`'s size scalar: watch the particle quads' screen size vs the game's at the
same spot (`psiDrawParticleList` 0x122d50, the `4·size/(PS2ParticleSize·PS2SnowSizeMult)` scalar at
0x11e2e8:67).

## Left for the next message

- The cloud drift phases → the sky-pass matrices per frame; the flicker tint → the MISC instance.
- The Apocalypse whiteout as a screen overlay (the sim exposes `fade`).
- The emitter sprite textures (def+0x04) and the screen-space size scalar.
- MoviePlayer's 0xD trigger + PSS decode; EnvTree's sway anim.
