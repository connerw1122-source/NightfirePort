# Dev mode: `--dev --scenario <name>`

`nf-view` has a `--dev` flag for testing features on demand. Each scenario places Bond and sets up the world
for one feature, then prints one line saying what to check. **Dev mode never changes game logic or the order of
RNG calls** — it only sets Bond's position, the inventory, or spawns a pickup. A scenario run with `--dev` is
bit-identical to a normal run that happens to start in the same state.

## Usage

```
nf-view <iso> 07000005 --dev --scenario <name>
```

The scenarios (level 07000005 "Paris Prelude / Mayhew A"):

| Scenario | What it does | What to check |
|---|---|---|
| `weapon-switch` | Gives Bond the SG5 Commando (0x16) and the sniper (0x24) with ammo, in addition to the P2K. | Scroll the mouse wheel (or press 1–3) to cycle **weapons** (P2K/SG5/sniper). Gadgets are disabled (not ported yet — see `docs/port-map/gadgets.md`). Watch the holster/draw animation and listen for the sounds (0x283 holster + 0x282 draw for weapons). Set `NF_SOUNDLOG=1` to see each animation id logged. |
| `sg5-pickup` | Spawns a dropped SG5 Commando (12 rounds) on the floor 1.5 m in front of Bond. | Walk forward to pick it up — the pickup sound plays, the SG5 enters the inventory. Scroll to switch to it. |
| `wire-music` | Places Bond 5 m from the zip wire (at the castle's outer wall, (−23.5, 34.5, 6.3)). | Walk toward the wire. When Bond is within 1.5 m the music changes to the stealth section (section 2, or 6 if guards are attacking). Walk away and it returns to section 20. |
| `autoaim` | Faces Bond toward the nearest alive guard; auto-aim is off (D-001). | Press T to toggle auto-aim. When on, the crosshair snaps to the guard's bone 2. When off, the mouse aims directly. |
| `sniper-scope` | Gives Bond the sniper rifle (0x24) and selects it. | Hold **right mouse** to zoom — the FOV narrows (divided by the zoom factor) and the mouse sensitivity drops. Release to return to normal. The scope overlay is not yet ported (the view zooms but no black vignette). |

## Coordination

Another session may add `--dev` too. If `--dev` is already on origin when you pull, extend `dev::scenario` /
`dev::SCENARIOS` in `crates/nf-view/src/dev.rs` (and the `mod scenarios` block at the bottom of that file) instead
of duplicating it. The scenarios are matched by name in `dev::scenario`; a new scenario is one `match` arm plus
one `&str` in `SCENARIOS` plus one `pub fn` in `mod scenarios`. (B-3a/b: the older `apply_scenario` in `main.rs`
was merged into `dev::scenario` — there is now exactly one scenario registry.)
# Dev mode (nf-view `--dev`)

In-game test tooling, gated behind `--dev` so normal play is unchanged. Tools only call into existing systems or set
state the way the game itself would — never change game logic or the order of RNG calls. Logic stays at a fixed 60 Hz;
the time scale changes the number of ticks run per render frame, not the tick rate. WORKLOG 0081.

## Starting

```
nf-view <iso> <level> --dev [--scenario <name>]
```

`--dev` turns on the F1 / F2 / F11 keys and prints the list. `--scenario <name>` places Bond and the guards for one
of the named test scenarios (below) and prints what to look for. Both can be combined; a scenario without `--dev` is
ignored.

## Keys (dev mode only)

| Key | Action |
|---|---|
| **F1** | Toggle the AI overlay. |
| **F2** | Toggle the dev menu (mouse-clickable when open; the mouse is released while the menu is up). |
| **F11** | Step one tick (only when the time scale is paused). |
| **P** | Pause (the game's own pause; works without `--dev`). |

Without `--dev` these keys do nothing (F1/F2/F11 fall through to their normal behaviour — F3 is reinforcements, etc.).

## F1 — the AI overlay

Each guard within 30 m of Bond shows:

- its **index, stance, and state name** (e.g. `Sniper_PopUp`, `Ambush`, `GrenadeThrow`, `RecoverFromScary`),
  health, alertness and distance;
- its **active timers** (tC, tD) and grenade count;
- a **green line** to its target (Bond) or its cover point;
- a **"forced"** marker when a dev-menu action forced its next decision (e.g. "throw grenade now").

Live grenades draw their **arc** (an orange line from the grenade along its velocity) and their **explosion radius**
(a red circle). A grenade's explosion flashes a **yellow circle** at the blast.

## F2 — the dev menu

A mouse-clickable list (click the mouse on an item to apply it):

| Item | Action |
|---|---|
| spawn: Normal guard | Spawn a default SG5 guard at the crosshair (the camera's aim point). |
| spawn: Sniper | Spawn a sniper (anim set 0x13). |
| spawn: Ambush | Spawn an ambush guard (mode 0x10). |
| force: throw grenade now | The nearest guard within 30 m enters `GrenadeThrow` (0x2D) and throws — its RNG roll forced to pass, marked "forced" in the overlay. |
| force: impulse (hop up) | The nearest guard gets a velocity impulse straight up — to test falling off ledges. |
| force: SpecialDeath (fall) | The nearest guard enters a SpecialDeath (free root, gravity on) — the falling-off-ledge test. |
| god mode: ON/OFF | Bond's health is locked at 100, armour at 100, the hit flash cleared each tick. |
| infinite ammo: ON/OFF | The gun's clip is refilled each tick. |
| teleport: save spot | Save Bond's current position. |
| teleport: restore spot | Teleport Bond to the saved spot. |
| time: 1x / 0.25x / pause | The time scale. "pause" stops the simulation; F11 steps one tick. |

The spawn uses the game's own spawn path (`Drone::new` + `present`/`enabled`/`llist.push`, the same as `Action::Spawn`).
The force / impulse / SpecialDeath set the same fields the game's own state machine would. God mode and infinite ammo
set the same fields `Player_HandlePain` and `Player_WeaponFiring` read.

## Scenarios (`--scenario <name>`)

Each places Bond and the guards and prints what to check. The user should see the named behaviour in under a minute.

| Name | What to see |
|---|---|
| `grenade-throw` | The gate guard lobs a grenade (anim 0x49) within ~10 s. F1 shows `GrenadeThrow`. |
| `grenade-bounce-explode` | The grenade arcs, bounces (sound 0x51C) and explodes (sound 0x4A7). F1 shows the arc + radius. |
| `sniper-cover` | The guard (anim set 0x13) pops up once a second (state 0x97), ducks after 6 s (0x99), reloads (0x98). |
| `ambush` | The guard (mode 0x10) crouches behind cover (0x37 → 0x38) and attacks when Bond is within 3 m (0x56). |
| `guard-falls-off-ledge` | The guard is lifted 3 m and falls under gravity (the `Drone_CollisionHandler` down-ray). F1 shows `in_air` → `gnd`. |
| `searchlight-tracks-bond` | A searchlight sweeps and, when it sees Bond, tracks him (sound 0xE5 relay). |
| `door-anti-crush` | Open a door (E), stand in the doorway — it reverses (sound 284). |
| `bullet-flyby` | A guard fires from 14 m; a near miss plays the fly-by whiz (0x1dc) when the bullet passes within 3 units. Strafe to make misses pass close. |
| `friendly-actor` | The guard (team 2) does not attack — walk up to it. F1 shows the state stays Idle/Patrol. |

## What it does NOT do

- It does not change game logic or the order of RNG calls. The "force grenade" sets the state and the
  `pending_grenade` flag the way the anim event would; the RNG roll inside `throw_grenade` (the sniper stun 1/3)
  still runs.
- It does not add a new tick rate. The simulation is always 60 Hz; the time scale only changes how many ticks run per
  render frame.
- It is off by default. `nf-view` without `--dev` is unchanged.

## Input record / replay (`--record` / `--replay`) — B-5

Every other session kept shipping "fixes" that failed in the user's play because nobody could reproduce what the
user did. `nf-view` now records and replays the exact input stream the Sim consumes, so a bug the user sees can be
captured as a file and replayed bit-for-bit (headless, no GPU) in a test.

### Recording

```
nf-view <iso> 07000005 --dev --scenario weapon-switch --record tests/replays/bug.nfr
```

- Saves a header (level, scenario, nf-view git hash, starting RNG state) followed by every logic tick's input —
  keys, mouse buttons, wheel steps, mouse-look deltas — in the exact form `Sim::step_bond` consumes.
- Recording costs nothing when it is off (`--record` absent ⇒ no work per tick).
- Esc-Esc to stop and write the file. A panic does NOT flush (a partial file would be useless).

### Replay

```
nf-view <iso> 07000005 --replay tests/replays/bug.nfr
```

- The level and scenario come from the recording's header (you do not pass `--scenario` again).
- The recorded inputs drive the Sim exactly as recorded; the keyboard/mouse are ignored. Watch the session play back.
- When the recording ends, the sim pauses on the last frame.

### Headless replay (for tests)

`nf_view::replay::replay_headless(&mut scene, &replay)` runs the recording through the real `Sim` (no GPU, no window)
and returns the state at each tick (`TickState`: Bond pos/yaw/pitch, weapon, clip, spare, RNG x/y, frame). This is
how tests reproduce a user's session and assert on the result.

### Determinism

`crates/nf-view/tests/replay_live.rs` (NF_ISO opt-in):
- `replay_is_deterministic_two_runs_identical` — builds a 60-tick recording from a fixed input sequence, replays it
  twice from two fresh scene loads, and asserts the Bond position / weapon / ammo / RNG state are IDENTICAL at every
  tick. If it diverges, the test prints the first divergent tick + the differing fields.
- `replay_file_format_round_trips` — serialise → bytes → re-parse, and the re-parsed recording replays to the same
  states (the format is lossless).
- `replay_commit_sample_file` — writes a sample recording to `tests/replays/` (inputs only — never game bytes).

Replay files (`*.nfr`) hold input only; they are safe to commit and share. The magic is `NFREPLAY` (v1).

### How to report a bug with a recording

```
nf-view <iso> 07000002 --dev --scenario grenade-throw --record tests/replays/grenade-bug.nfr
# (play until the bug shows; Esc-Esc)
# then hand the session the file path; the determinism test replays it headless:
NF_ISO=<iso> cargo test -p nf-view --test replay_live -- replay_commit_sample_file --nocapture
```
