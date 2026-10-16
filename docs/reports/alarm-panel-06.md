# alarm-panel-06 — the 07000006 alarm panel (playtest 1015-2142, "no panel to turn off the alarm")

Branch `s/alarm-panel-06` (from master f42de6ff). Everything below is measured on the slim on-foot ISO.

## What the panel actually is (the game's silencer chain)

There is **no E-usable switch object** on 07000006: `SwitchList` is 0 in the live EE snapshot
[L ee_start_05.bin @0x002bf0f8], the only kind-0x29 switch placements in the whole level file sit in
map 4 (placements #2/#15, hashes 0x02000649/0x0200064a, both param0 = 1) which is **never instanced**
(no `CREATE_CEL` placement resolves into map 4, and no main-map placement hashes to its two objects),
and none of the 61 live class-0 SP objects has an owner (`data+0` = 0 for all [L]), so
`SP_Activate` 0x1dc338 → `Switch_Activate` 0x1b02a8 can never fire here. The map-4 switches are dead
shared data (the same hashes/params reappear in 07000009 map 7).

The panel the player uses is a **walk-in trigger + inverter pair by the fusebox** [D placements;
all addresses [M]]:

1. **TouchOnce #765** (0xEA, out **ch 177**, mask 8 = player contact, lock/hide 0; placement at
   (−0.58, 0, −1.40), volume = main-map object 195's world-space collision tris
   (31.12, 42.22, −59.06)..(36.96, 46.79, −48.87)). Bond walks into the wall panel next to the
   fusebox and ch 177 rises (`Trigger_Activate` 0x1b2318; the mask-4 E-arm is not involved).
2. **InverterTrigger #749** (0x41, p0 = 177, p1 = 55; `InverterTrigger_Create` 0x1b3040,
   `InverterTrigger_Update` 0x1b30a8) — **the only channel-clearing object in the game**: every
   frame, while ch 177 is on, `switch_channels[55] = 0`.
3. The alarm channel **55** is what the four gate searchlights hold every frame while tracking
   (`Searchlight_Update` 0x1cb900, ext+0x36 = param 3 = 55; placements #2/#4/#6/#8 [D]) and never
   auto-clear — after the head gives up (dist > 2.0 for `FRAME_RATE·15` ticks, 0x1cb900) ch 55 stays
   latched, which keeps the alarm music posting (`DroneFunc_CheckAlarmRaised` 0x1453a8: ch 0x37 →
   `Music_Event(2,1)` for the first 600 frames) and the siren playing.
4. **The siren** is SoundTrigger **#702** (0xF9 at (28.66, 40.42, −17.30): p0 = 55 on-channel,
   p1 = 1 self-delete-after, p2 = **329** the siren sample, p3 = 1 → 2-D `Sound_Play`, p4 = **177**
   the OFF channel, p6 = 0 difficulty gate, p7 = 0 keep byte; `SoundTrigger_Create` 0x1b2628,
   `SoundTrigger_Update` 0x1b2790). It stops when ch 55 clears (keep byte 0) or ch 177 rises —
   either panel effect silences it. Neighbours: #703 (ch 57 → sound 241, the searchlights-off
   clunk), #712 (ch 103 → sound 331), and the O4 **FuseBox #747** (0x31, (28.00, 40.00, −18.15),
   script 0x0600006B, wires → ch 57 + ch 103 and ch 55 + ch 55; `FuseBox_Create` 0x1b0448,
   `FuseBox_Update` 0x1adfe8) — still unported (060-fix-objects2 territory), a separate path to the
   same alarm channels.

So "the panel turns the alarm off" = walk to the panel (no E, no prompt) → ch 177 → the inverter
holds ch 55 low → the siren stops and the alarm music stops re-posting. Guards are not re-alerted
(the port's ch 0x96 rebroadcast, drone.rs `Drone_InitComms`, has no writer on this level [D — no
07000006 placement or script references ch 150]; the guards' wake here rides the searchlight/siren
alerts).

## Why the port failed the playtest, and the fix

The port already had every object of the chain (triggers, inverters via worldobj, sound triggers via
sound.rs — a previous fix-06-alarm task). The break was in **`SoundTriggers::create`**
(crates/nf-game/src/sound.rs):

- `keep_on` read `p.params.len()`, but the game reads **param 7** (`level-tag +0x48`,
  `SoundTrigger_Create` 0x1b2628:48 — the level-tag carries the placement's param values at
  +0x2C+4k, the same layout the switch/door param mapping uses). #702 has **8 params**, so the
  port's keep byte was 8: the "stop when the ON channel clears" arm (`SoundTrigger_Update`
  0x1b2790:49-59, `keep_on == 0`) could never fire and **the siren rode ch 55 forever** — the
  player had no way to end the alarm, i.e. "no panel to turn off the alarm".
- The difficulty gate read the raw placement word `extra[8]` (+0x44) instead of **param 6**
  (0x1b2628:24). Benign on 07000006 (param 6 = 0 for all four triggers) but wrong per the game.

Fix: `keep_on: param(7)`, `gate: param(6)` [M addresses above], plus a `SoundTriggers::playing(id)`
probe (the test-facing read of the private handles).

## Verification (headless, real input path)

`crates/nf-view/tests/alarm_panel_06_live.rs` (opt-in NF_ISO), both tests pass:

1. `the_searchlight_alarm_silences_at_the_wall_panel` — loads 07000006 through the real loader
   (`nf_view::load_with_carry`, truck-ride spawn), waits out the intro, stands Bond under the
   courtyard sweep until a gate searchlight locks on (state 1) and asserts **ch 55 rises** and the
   siren (329) is live; walks Bond to the panel room, waits for the head's 900-tick give-up and
   asserts **ch 55 stays latched and the siren keeps playing** (the game's symptom); puts Bond into
   the panel volume and asserts **ch 177 fires**, the **inverter holds ch 55 low**, and the **siren
   trigger releases its voice** (keep byte 0). Also pins the created objects' fields against the
   disc data (#702 off = 177, is2d = 1, keep = 0; inverter in 177 → out 55).
2. `the_panel_with_no_alarm_changes_nothing_but_its_own_channel` — the sister: with no alarm, the
   panel still fires ch 177 (the game's TouchOnce has no alarm gate [M 0x1b2318 kind-1 arm]), ch 55
   stays low and the siren never starts.

Crates run before push: `nf-game --lib` (234 passed), `nf-view --lib` (15),
`alarm_panel_06_live` (2), `sound_coverage_live` (4) — all green.

## Full suite

The background fast runner (`tools/test/suite.py`) was started twice in this sandbox and was killed
by the sandbox between tool calls both times without completing its build phase (empty `suite.out`;
the sandbox reaps background processes — the driver's own warning). The four foreground crate runs
above are what I can report. **The whole-suite run on the final tree is owed**; nothing in the diff
touches code outside `sound.rs`'s create (one trigger-field source) plus the new test file.

## What's left / live checks for the main agent

- **Render asks** (the panel has no visual in the port's HUD sense — it is an invisible walk-in
  volume; whether the wall-panel MESH draws there is render-side):
  `--bond 34.0,43.5,-54.0,0.2,0` (inside the panel volume, facing the wall panel/fusebox nook) and
  `--bond 28.0,41.5,-14.0,3.1,0.1` (the fusebox #747 corner from the gate side). If the panel mesh
  (object 195's gfx or the fusebox) doesn't draw, that's a render gap to file — the gameplay chain
  does not depend on it.
- **Live check 1** (pcsx2): walk Bond into the panel volume (34, 43.5, −54) with the alarm raised;
  watch `switch_channels` (0x0026fc90) bytes 177 (0xB1) and 55 (0x37): expect 177 → 1 on contact and
  55 forced to 0 the same frame the inverter ticks.
- **Live check 2**: does siren sample 329 loop in the real bank? (The port's SFX table drives the
  stop test through the trigger's stop arms; if the real sample loops forever, the stop is purely
  the channel arms as modelled; if it does not, the trigger's self-delete after one pass also
  applies [M 0x1b2790:71-84].)
- **Live check 3**: what raises the guards on this level in the real game — confirm the guards wake
  via the searchlight/siren alerts (the ch 0x96 rebroadcast has no writer on 07000006 in the disc
  data), i.e. the panel silencing the siren does not themselves re-alert.
- `FuseBox_Update` 0x1adfe8 (the O4 wire-cut box) remains unported (documented in
  docs/levels/07000006.md; 060-fix-objects2's territory). Its wire-cut writes ch 57 (searchlights
  off) + ch 103 and ch 55 + ch 55 — worth doing before the next 07000006 playtest round.
- The main map's placement-matrix offset (−0.58, 0, −1.40) is applied to the panel volume's
  world-space tris by the port's `triggers::build`; the game's `Trigger_Hit` 0x1b1fc8 reads the
  glist object's world tris. The offset is small (≤ 1.5 m); a live check of the exact trigger box
  would settle whether `triggers::build` should skip the placement transform for by-index main-map
  trigger placements like it effectively does for their glist siblings.
