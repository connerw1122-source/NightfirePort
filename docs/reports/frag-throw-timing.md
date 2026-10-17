# frag-throw-timing (741) — the frag leaves the hand on the game's release tick

Branch `s/frag-throw-timing`. Playtest 1016-1355 (07000014): "frag grenades are flying out way too
soon. They don't let the animation finish. We're going in the opposite direction with that one."

## What the game does (the evidence chain)

**The fire tick** [M `Player_WeaponFiring` 0x1a5460 tail]: the shot spends, `Player_SetFiringAnim`
queues the fire script, and — the part the port had wrong — for the release-family records
(`weapon_data+0x6C & 0x400` with a `+0xB4` script; the hand grenades 0x34-0x37
[L ee_start_05.bin records: +0x6C = 0x8c08/0x8c0a, +0xB4 nonzero]) the view state goes to **0xb**
and the queued script's rate is set to **1.5** (`*(AnimList+0x98) = 0x3fc00000`): the state store
and the rate store sit in the fire tick itself (disasm around 0x1a6250). The state therefore
leaves 9 inside the fire tick — **case 9's release check can never fire for the family**.

**The wait** [M `Player_SetWeaponAnimObj` 0x1a2e38 case 0xb :499-507]: the machine sits in 0xb
until `AnimScriptIsStopped` (0x1c70d8 — "some playing script has the ended bit `+0x78 & 0x10000000`",
set by `AnimScriptEnd` 0x1c1930:30 when the playhead reaches the script's end) — then it stores 0xc
and queues the `+0xB4` script (`AnimScriptAdd` deletes every playing entry and starts the new one at
playhead 1.0 [M `AnimScriptInit` 0x1c1600:37], advancing 1.0/tick [M `AnimScriptTick` 0x1c2630
mode 0: `+0x90 += +0x98·FRAME_RATE_MUL`]).

**The release** [M case 0xc :505-519, disasm 0x1a3268-0x1a32c4]: `weapon_data+0x44 <= AnimList(4)+0x90`
(filter 4 = the list head [M `AnimListBuild` 0x1c0fc0 bit-4 arm: `entry+0x44 == 0` = the prev
pointer]) with the `+0x95C` latch — **the playhead compared is the +0xB4 script's**. Case 9's
identical check [M :411-422] exists for the non-0x400 family whose state really is 9.

**The frag's numbers** [L ee_start_05.bin record 0x34 + the port's script parse]: `+0x44` = 26,
fire `0x0600049d` = 70 frames (the pin sound 0x1e4 at frame 16), `+0xB4` `0x0600049e` = 35 frames
(the throw-grunt sound 65 at frame 22), `+0x20` speed 0.1667. So: the 70-frame wind-up plays at
1.5× for ceil(69/1.5) = 46 ticks, then the 35-frame throw script's playhead (1 + k) reaches 26
after 25 more → **the grenade spawns at press + 71 ticks**. The op-1 sounds bracket it coherently:
the pin pull early in the wind-up (frame 16 ≈ tick 11), the grunt 4 ticks before the release.

## When the port's release changed (the bisect)

- fix-player2 (eb1564b7) introduced the release check against the **fire script's** playhead
  (`release_frame_reached`): release at playhead-26 = press+25 at 1.0×. Already the wrong gate —
  the playtest 1014-1725 ("released while I'm still pulling the pin") and the audit of
  fix-grenade-release (109e, no code change) both stumbled on it without reproving headlessly.
- f986a6b3 (Oct 15, the throw-cadence commit "hand grenades throw at the game's 1.5x script speed,
  then play the bring-up") added the game-correct 1.5× rate but kept the wrong gate: the same
  playhead-26 crossing now lands at **press+17** — the playtest 1016-1355's "way too soon …
  opposite direction".
- grenade-rattle2 (693b) touched the bounce-speed cap (0x12d6a0), not the release path — cleared
  by the bisect.

The root error in both: the check ran during the fire script. In the game the check for this
family lives in the post-fire-script state 0xc against the +0xB4 script's playhead.

## The fix

`crates/nf-view/src/scene.rs`, the release block: for the release family
(`+0x6C & 0x400 != 0 && +0xB4 != 0`) the spawn requires the view to be ON the `+0xB4` script
(`view_script == rd(0xb4)`) before `release_frame_reached(view_time, fire_anim_len)` runs —
the case-0xb → case-0xc phase; `view_time` there is the +0xB4 playhead (the port's time+1 frame
convention matches the game's playhead-starts-at-1.0). Everything else is unchanged: the
`fire_anim_len == 0` press-tick spawn, the non-family case-9 check, the latch, the sounds, the
0xb → 0xc → draw chain and the +0x40 refire (whose float read of the frag's integer 63 clamps to
1 tick — the state machine gates the cadence, [M 0x1a5460:652-656 + the state gate :631-648]).

## Verification

- `crates/nf-view/tests/frag_throw_timing_live.rs` — through the real input path
  (`nf_view::load` + `Sim::step_bond`, the pickup take + switch machine, a real press):
  - **The repro/pin**: the grenade must NOT spawn at the old playhead-26 crossing (the test
    measured **exactly press+17** on the pre-fix tree — the symptom), and must spawn at exactly
    press+71 (46 + 25, computed from the decompile facts, not hardcoded); the fire script is still
    playing at tick ≤ 20 (the sister: the wind-up is not interrupted); the world grenade carries
    weapon_type 0x34 and the record's +0x20 speed; one grenade per press (the latch).
  - On the fixed tree the test passes; the neighbour suites all still pass:
    fix_grenade_throw_live (4/4, including `the_frag_throws_on_the_release_frame_and_refires_at_the_interval`)
    and fix_player2_live (5/5, including the 0x35 stun's release + gun recovery).
- Crates run before pushing: `cargo test --offline --release -p nf-view -- --test-threads=1` and
  `-p nf-game` (the touched crates) — totals below with the fast suite.

## What's left

- A live PCSX2 confirmation of the +71 release (the checks below) — the decompile chain is
  complete but the exact same-tick ordering of `AnimScriptTick` vs `Player_SetWeaponAnimObj` can
  shift the spawn ±1 tick; the port's tick is the decompile-literal one.
- The smoke (0x36) shares the family path (+0x6C 0x8c0a) and inherits the fix; its own live
  pass on 07000015 would confirm the cloud timing unchanged.

## Live checks for the main agent

1. **The release tick**: fresh boot 07000014, frag equipped, gdbwatch the grenade spawn —
   `Bullet_init` (0x12c5a0) entry, or watch `Drones`-side BU allocation; count frames from the
   fire-press. Expected: the world grenade exists at press+71 (±1 for the tick-order caveat), NOT
   at +17.
2. **The playhead at the release**: gdbwatch the view model's AnimList entry `+0x90` (the +0xB4
   script 0x0600049e's entry — the list head) at the spawn tick. Expected: 26 (the +0x44 value)
   when the spawn fires; during the wind-up the head entry is the fire script's, advancing 1.5/tick.
3. **The state machine**: watch the view anim's `+0xF4` (the anim object at `BLData+0x7E8`):
   0xb from the fire tick, → 0xc when the fire script's playhead hits 70 (≈ tick 46), the spawn
   inside 0xc, → 3 (draw) when the +0xB4 ends (≈ tick 81).

## Usage

not visible to me
