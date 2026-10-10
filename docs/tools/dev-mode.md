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
| `weapon-switch` | Gives Bond the SG5 Commando (0x16) and the sniper (0x24) with ammo, in addition to the P2K and gadgets. | Scroll the mouse wheel (or press 1–7) to switch weapons. Watch the holster/draw animation and listen for the sounds (0x280 holster, 0x281 draw for weapons; 0x282/0x283 for gadgets). Non-P2K weapons play their own fire sound. |
| `sg5-pickup` | Spawns a dropped SG5 Commando (12 rounds) on the floor 1.5 m in front of Bond. | Walk forward to pick it up — the pickup sound plays, the SG5 enters the inventory. Scroll to switch to it. |
| `wire-music` | Places Bond 5 m from the zip wire (at the castle's outer wall, (−23.5, 34.5, 6.3)). | Walk toward the wire. When Bond is within 1.5 m the music changes to the stealth section (section 2, or 6 if guards are attacking). Walk away and it returns to section 20. |
| `autoaim` | Faces Bond toward the nearest alive guard; auto-aim is off (D-001). | Press T to toggle auto-aim. When on, the crosshair snaps to the guard's bone 2. When off, the mouse aims directly. |

## Coordination

Another session may add `--dev` too. If `--dev` is already on origin when you pull, extend the `apply_scenario`
function and this document instead of duplicating it. The scenarios are matched by name in `main.rs ::
apply_scenario`; a new scenario is one `match` arm.
