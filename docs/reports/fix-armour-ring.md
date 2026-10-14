# Task report: fix-armour-ring (branch s/fix-armour-ring, finish id 100p-fix-armour-ring)

## The answer

**The armour branch is already in the port and is game-faithful.** `HUD_UpdateHealthPane` 0x19b9c8:139-158
[M] gives the pane three states, and `crates/nf-view/src/hud.rs` (the block added by eeb8e54, "the game's HUD")
implements all three exactly; this task verified it line-by-line against the decompile and pinned it with tests:

- **armour == 0** (`armour*0.16 == 0.0`): the 8 armour segments (`HealthSprInfo[0..8]`, sprite +0x38) are
  **hidden** (layer 0xFF) — the ring is the green health state: the 7 health segments (sprites 8..14, neutral
  art tinted the health colour 0x52a88b green / 0xc6984e orange / 0x9d1912 red at runtime) + the centre disc
  (sprite 15), also health-tinted [M 0x19b9c8:159-207].
- **armour > 0**: the 8 armour segments appear — template tint **0x7d6d59** (bronze; the update only rewrites
  the alpha byte [M 0x19b9c8:151-155]) over the sheet's **orange-red arc art** (tex 0x0300007A; the 8 uv rects'
  pixel averages are rgb(175,126,96)…(122,44,37) [L the sheet]) — with alpha **1.0 for segment 0**, **0.75**
  while `k < armour*0.16` (armour 50 = the max lights all eight), **0.35** above.
- **Which sprite is "the 007 logo inside the ring": none.** `HealthSprInfo[15]` (87×86 at pane (4,4), uv
  (102,113,85,86), template alpha 0x40, tinted by the health colour each frame) is a **hollow circle outline**
  — its alpha plane is a ring band, the centre empty [L the sheet]. The ring's centre stays empty; the
  "orange 007 ring" in the PCSX2 captures is this pane's own armour state, exactly as 100n-fix-context-hints2's
  hypothesis went: the capture god-cheat writes **health 100 + armour 50 (BLData+0x8B0) every frame**
  [M tools/trace/cheats.py:56-57], so every capture shows the 8 bronze-orange segments (health 100 keeps the
  outer ring green). No bottom-left logo draw exists anywhere else, matching 100n's finding.

Every `BLData+0x8B0` reader agrees it is the armour float, 0..=50 [M]: `Pickup_Handler` 0x18e650:221-224
(refuse ≥ 50, else hard-set 50.0), `Player_HandlePain` 0x1902c8:100-104 (armour absorbs the damage first),
`Player_Init*` (0 at spawn), `RamSave/RamLoad`, the pane (this one) and its MP twin. No reader disagrees.

## What changed

- `crates/nf-view/src/hud.rs`: **unchanged** — verified correct.
- `crates/nf-view/src/lib.rs` (shared file, listed per the rules): one additive hook — `--armour <n>` writes
  `BLData+0x8B0` after the tick loop (the same write the dev panel's armour row and cheats.py's god make) so
  the armour state renders headless. 5 lines incl. the comment.
- `crates/nf-view/tests/fix_armour_ring_live.rs` (new): the pin, through the real paths.
- `docs/WORKLOG.md`: the entry.

## Verification (all on the final merged tree, origin/master 1496b80 merged, commit b84b706)

`fix_armour_ring_live` (NF_ISO, `--test-threads=1`), each assertion citing 0x19b9c8's lines:

1. `at_armour_zero_the_armour_segments_hide_and_the_ring_stays_green` — fresh 07000005 (armour 0.0, health
   100): **0** bronze-tinted quads on the left half; the green ring = **8** quads (7 health segments + the
   centre disc), the disc pinned at its 87×86 rect (20,341)..(107,427).
2. `taking_an_armour_pickup_lights_the_eight_armour_segments` — the real take path: Bond
   `stand_at` the level's kind-3 pickup, `step_bond` until `Pickup_Handler` case 3 fires
   (`Inventory::take` hard-sets 50.0) → **8** segments, alphas {0.75×7, 1.0×1}, segment 0 at its pane
   position (48,367); sister: the green ring keeps its 8. (The take's observable is the armour write: the
   scene's HUD update consumes `drones.taken` within the same tick.)
3. `the_armour_segments_track_the_value` — armour 25 (cheats.py's own write) → {0.35×4, 0.75×3, 1.0×1}
   (`25·0.16 = 4.0`); armour 0 → hidden again.

Sister note: the first quad-tally pass also caught the two tint collisions the filters must handle — the ammo
digits share 0x7d6d59 (AmmoSprInfo[0]) bottom-right, and a pickup message draws its bar from the same sheet
(uv u ≈ 185–196 [M 0x19c310:340-346]) bottom-left; `pane_quads` filters by run texture + u0 + half.

The neighbours on the merged tree: `fix_hud_live` 6/6, `hud_playtest_live` 5/5.

## Suite

Started in the background on the final merged tree at 12:35 UTC (build + 240-binary run; the sandbox
deadline cuts it — the pushed report carries the line that existed at push time, updated below if it landed):

- (filled at push — see the last commit of this branch)

## Renders for the main agent (nf-view --shot; this sandbox has no GPU)

    nf-view <iso> 07000005.bin --frame 30 --shot /tmp/armour0.png
    nf-view <iso> 07000005.bin --frame 30 --armour 50 --shot /tmp/armour50.png
    nf-view <iso> 07000005.bin --frame 30 --armour 25 --shot /tmp/armour25.png

Expected:
- armour0: bottom-left ring, green health segments + the hollow centre outline, **no** armour arcs.
- armour50: the same ring plus the **8 bronze-tinted orange arc discs** inside it (segment 0 brightest at
  alpha 255, 1..7 at 191) — the captures' "orange 007 ring" state. The ring's centre stays empty.
- armour25: the inner four-ish discs dim (0.35), the rest at 0.75.

## Live checks for the main agent (PCSX2)

1. Spot: any level, god-cheat on (health 100 + armour 50 every frame, as the captures) — the bottom-left ring
   should match armour50.png: the orange arcs inside the green ring.
2. Take damage until the armour depletes (`Player_HandlePain` drains it before health) — the orange arcs
   disappear segment by segment (each loss of 6.25 armour drops one disc to the 0.35 ghost, then the ring is
   green-only at 0).
3. `fieldprobe`/`gdbwatch` on BLData+0x8B0 (float, read) while armour drains: the pane's lit-segment count
   should track `armour·0.16` (8 lit at 50).

## Remaining

- Nothing on the port side. If the user's capture still shows something the port's armour state lacks (e.g.
  the exact arc art), the next step is the rendered pair above against a PCSX2 frame — the sprite/tint/alpha
  data here is the checklist to diff against.
