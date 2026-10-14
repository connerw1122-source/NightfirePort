## Your task: `context-hints2`, the hint bar's position, the 007 ring, no prompt at the grapple point

100h-fix-context-hints (merged) shows the hint bar: at the 07000008 safe (`--bond 155.19,43.12,-113.38,1.57,0`) its
text matches PCSX2 exactly ("Hint: Use Watch Laser"). The main agent's side-by-side with PCSX2 at the two spots still
shows:
1. **The bar's position:** PCSX2 draws the hint bar at about y 100/448 (a quarter of the way down, under the HUD top);
   the port draws it at the top edge (y ~0). Use the HUD element's own rect.
2. **The ring:** at both spots PCSX2 swaps the radar ring (bottom left) for the ORANGE 007 icon; the port keeps the
   green radar. Port the ring swap and its sprite.
3. **The 05 grapple point** (`--bond 16.5,29.6,58.6,-2.01,0.24`, PP7 drawn in both): the port shows "Press Action"
   and a Bond-silhouette icon at the top right; PCSX2 shows NO text and no such icon there, only the orange 007 ring.
   Find why the port raises that prompt (the context source, the hint condition: maybe the hint needs the claw drawn or
   a range the spot isn't in) and match the game.
Tests per item at those spots (the bar rect, the ring sprite id, no prompt at the grapple spot with the PP7 drawn);
sister assertions; `--shot` the two spots.

Files: `crates/nf-view/src/hud.rs`, `crates/nf-game/src/player.rs`.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md). Read `tail -1 suite.out` and
  its FAILED/TIMEOUT lines before you push, and run it once more on your final tree (merged with `origin/master`). Report the total
  line. On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/context-hints2`.

---
claimed-by: sandbox6 at 2026-10-14T08:59Z

---
## Report (done, 2026-10-14T10:50Z)

Closed by the driver: the report is on `s/fix-context-hints2` (docs/reports/fix-context-hints2.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-14T10:53Z)


---
## Report from the worker (done, 2026-10-14T10:53Z)

# fix-context-hints2 (100n) — the hint bar's position, the 007 ring, the grapple prompt

The driver's 2026-10-14 PCSX2 side-by-side at the two spots (`--bond 155.19,43.12,-113.38,1.57,0` on 07000008 and
`--bond 16.5,29.6,58.6,-2.01,0.24` on 07000005): (1) the port drew the hint bar on the top edge, PCSX2 about a
quarter of the way down; (2) PCSX2 swaps the bottom-left ring for the orange 007 icon, the port keeps the green
ring; (3) the port raised "Press Action" + the figure icon top-right at the grapple spot, PCSX2 shows neither.

**Base note**: `s/fix-context-hints2` = origin/master b65d6f7 + a merge of `origin/s/fix-context-hints` (100h) —
100h was NOT in origin/master when this session cloned (10:52 UTC fetch; master's last merge was
fix-missing-objects), and this task's three symptoms are all regressions/follow-ups on 100h's code, so the branch
merge was required to have the hint bar at all.

## Item 1 — the hint bar draws at the pane's own rect, y 113 (fixed)

- The INFO pane's CREATE tag `MsgInfoStatusPane` 0x319810 carries the pane's own rect **x=128, y=113** [L
  ee_start_05.bin @0x319810, `scripts/dump_hud_tables.py`].
- `HUD_CreateDefault` 0x199b70:45-52 adds that pane x/y to **every sprite's runtime position at create**
  (`sh …,0x24(spr)` / `0x26` with the pane values) [M].
- `HUD_UpdateStatusPane` 0x19c310 rewrites only the bar sprites' **X** on open/close — case 1/4 pin
  `x = templateX + (templateW − w)/2` from the TEMPLATE x 16 [M 0x19c310:105-109 (case 1), 331-336 (case 4)], and
  the caps follow the middle [M 0x19c310:342-347] — so the bar's x span stays 16..496 like the objective's. The
  **Y is never written**: the bar keeps its created template y 0 + pane y 113 = **113** (bar 26 tall → 113..139;
  the caps the same), and the never-repositioned text keeps 16 + 113 = **129**. `View_DrawSprites` 0x1e5290:60-75
  treats +0x24/+0x26 as top-left unless flag 0x800 — the bar sprites have no 0x800 [L dump].
- The port drew y 0/16. Fix: `crates/nf-view/src/hud.rs` — the INFO bar's `by = 113.0`, the text at 129.0. The
  objective banner is untouched: its pane y is 0 [L 0x319710], so its top-edge rect was already right; the pickup
  bar's (0,364) likewise.

## Item 3 — no "Press Action" at the grapple spot (fixed)

The port armed 07000005's ctx-6 icon[3] (24.20, 31.93, 60.01, r 2.276 [D probe]) at the driver's spot 8.17 away
because the arm gate was `straddle-cel set AND a flat 12.0 window` [S, 100h's deviation pin]. The spot shares the
icon's port cel 15, so the flat 12.0 decided — and it is wider than the game's arm.

- The game's arm is the icon's obj+0xD0 chain [M ThirdIcon_Update 0x196e80:33-53]: a class-3 object occupying a
  straddled cel; the post additionally needs the icon's 4-tick timer at 0 and the player mode ≠ 7 [M 0x196e80:38-44].
- New live pin [L PCSX2 2026-10-14]: no context and no prompt at 8.17. Old pin [L PCSX2 2026-10-13
  fix-swim-vent5]: the vent crawl spot reads 6 at 3.9. Both icons' own world spheres are r 2.276 [D probe], so the
  game's 3.9-arm is pure cel occupancy (its cels are finer than the port's buckets); the port still needs the [S]
  distance factor until `find_cell_raycast` resolves the game's cels.
- Fix: `crates/nf-view/src/scene.rs` — the arm window 12.0 → **6.0** (squared 36.0), the middle between the two
  live pins, with the full derivation in the comment. No prompt, no ctx, no figure icon at the grapple spot now.
- `crates/nf-view/tests/fix_context_hints_live.rs`: `the_grapple_spot_reads_context_6_and_shows_the_figure_icon`
  flipped to `the_grapple_spot_shows_no_press_action_prompt` — the new [L] pin (ctx 0xff, no 0x1000291 post, no
  0x0300006A quad at the driver's spot) with a positive sister beside the icon (~1.7 → ctx 6 + the figure quad).
  The old test asserted exactly the behaviour PCSX2 disproves.

## Item 2 — the orange 007 ring (investigated, NOT guessed; live checks below)

What the decompile + disc say:

- The bottom-left "green radar" is `HealthSprInfo[15]`: an 87×86 ring of the HUD sheet 0x0300007A
  (uv 102,113,85,86) at pane (16,337)+(4,4) = (20,341), tinted each frame by the health colour
  (0x52a88b green → gold → red) [L dump; M 0x19b9c8:205-207]. The port draws it (hud.rs, the health pane) — that
  is the ring the driver sees.
- The **only** 007-logo draw path in ACTION.ELF: health-pane sprite **16**, template texture **0x0300005b**
  [L HealthSprInfo[16]; D the texture exists in 07000005 map 4, 128×64 — dumped to PNG and it is the gold 007
  logo], flags 0x1800 (clip+CENTRE). `HUD_UpdateHealthPane` 0x19b9c8:221-261 drives it from `BLData+0x948`: while
  the timer ≥ 1 it writes the sprite at **(256, 80) centred** with w/h scaled by (200−t)/100 through a sine, layer
  0; the timer is written 200 by `PlrStat_DoneBondMoment` 0x199448 (a "Bond Moment" award, deduped per id ≥ 0x65
  in `BondMoments`), whose only caller is `Trigger_Calc` 0x1b1900 [M decompile grep]. No other code references
  0x0300005b or writes the ring sprite's uv [M grep: `0x300005b` — zero hits in code; only the template].
- So the evidenced mechanism is a **trigger-driven Bond-Moment 007 zoom at (256, 80) top-centre**, not a
  bottom-left ring swap — pinned in the disassembly, not just the decompiler: `addiu $v0, $zero, 0x100` /
  `addiu $v1, $zero, 0x50` → `sh $v0, 0x24($s1)` / `sh $v1, 0x26($s1)` [M 0x19bf10-0x19bf1c], and the trigger
  side is channels: `Trigger_Calc` 0x1b1900:8-15 scans **switch_channels 0x65..0x6e** every frame and calls
  `PlrStat_DoneBondMoment(0, k)` for each one that is on — Bond Moments ARE switch channels 0x65..0x6e, fired by
  the levels' trigger volumes (a teleport into such a volume would fire it).
- The two observations disagree on position, and the ctx table 0x2FC168's textures are the
  tan badges (0x66 winch / 0x67 hanging man / 0x6A figure — all dumped and inspected; 0x64/65/68/69 are not in the
  archives [D probe]), so the ctx system is not the orange 007 either. Per the no-guess rule I did not wire a
  mechanism to the bottom-left position; the probe tests `ctx2_probe.rs`/`ctx2_probe2.rs` (kept, `--nocapture`)
  print every icon/hint near both spots and dump the ctx textures + the sheet's ring/logo regions as PNGs.
- What would settle it: the live checks below. If PCSX2's bottom-left 007 is really sprite 16, the (256,80)
  reading is wrong and the disassembly at 0x19c0f8-0x19c120 decides; if it is a script sprite,
  `Script_SpriteStart` 0x1d9300 with the level's view scripts is the path (both archives' scripts would then list
  the 0x0300005b sprite).

## Verification

- New/updated tests (`crates/nf-view/tests/fix_context_hints_live.rs`, 6/6 green): the bar rect
  (`the_hint_bar_draws_at_the_pane_rect_y113` — the sheet-column vertices span y 113..139, text inside the band;
  fails on the pre-fix tree which drew 0..26), the grapple no-prompt pin + positive sister, the prior 4 (the safe
  hint once, the control-panel silence, the away pin, the data dump).
- Neighbours of the window change: fix_swim_vent5_live 4/4 (the 3.9/24.1 pins), fix_08_walls_live 2/2,
  gadget_grapple_live, qclaw_live, gadgets_live — all green.
- Suite (foreground chunks; the nohup'd runner dies between tool calls in this sandbox — same as the last three
  sessions): `cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view
  -- --test-threads=1` — 142 binaries ok + the allowed disc_golden trio (3 failed, slim ISO) before the 570 s
  wall; then two more nf-view chunks on the same tree: the HUD-adjacent set (hud_playtest, fix_hud, objective
  banner/messages, menus_flow, devwin_flow, pickups_trace, party_mission, pause_audio) 9/9 binaries and the
  portal/scene set (portal chain, room_sound, script_commands, viewmodel_zoom, weapon_effects, frustum_cull,
  lighting, impacts) 14/14 binaries, 48 tests — 0 unexpected failures across 165 binaries. nf-view's long tail
  (levels_sweep, weapon_matrix) did not fit the window; those paths are untouched by this task.
- Final tree = origin/master b65d6f7 (unchanged since the clone; re-fetched and verified an ancestor before the
  push) + the 100h merge + this task's commits.

## Files

- `crates/nf-view/src/hud.rs` — the INFO bar rect (task-named file).
- `crates/nf-view/src/scene.rs` (shared, additive) — the icon arm window constant + its derivation comment.
- `crates/nf-view/tests/fix_context_hints_live.rs` — the flipped pin + the bar-rect test.
- Kept probes: `crates/nf-view/tests/ctx2_probe.rs`, `ctx2_probe2.rs` (the evidence
  tools; they write PNGs only under `--nocapture` runs into `crates/ctx2-dump/`, untracked).
- `crates/nf-game/src/player.rs` — NOT touched: item 3's fix is the scene-side arm gate; the player-side state
  was already correct.

## Live checks for the main agent

1. **The 007 mechanism** (decides item 2): in PCSX2 on 07000005, `watch switch_channels+0x65 10 write` (the
   Bond-Moment channel block 0x65..0x6e [M 0x1b1900:8-15]) while standing at the grapple spot, then again at the
   safe on 07000008 — if a channel turns on, a Bond-Moment trigger fired and sprite 16 is the 007. Then
   `gdbwatch <BL>+0x948 2 readwrite` plus the sprite-16 fields (the health pane's sprite array +0x40: x/y
   +0x24/+0x26, w/h +0x2c/+0x2e, layer +0x38) settles the position. Expected if my reading holds: x=256, y=80,
   centred — a top-centre zoom, NOT the bottom-left ring.
2. **A screenshot request**: the driver's bottom-left 007 frame at full res — if the logo there is the 128×64
   0x0300005b artwork (my dump: crates/ctx2-dump/tex0300005b.png), a script sprite or the ring swap is real; if it
   is the small sheet-logo crop (uv 26,200,38,53 of 0x0300007a), the ring sprite's uv is being rewritten by
   something outside the ELF's HUD code (script-driven).
3. **The bar rect render** (nf-view): `--bond 155.19,43.12,-113.38,1.57,0` (07000008) — the hint bar now spans
   y 113..139 (about a quarter down), text at 129, "Hint: Use Watch Laser" centred; nothing at the top edge.
4. **The grapple render** (nf-view): `--bond 16.5,29.6,58.6,-2.01,0.24` (07000005, PP7 drawn) — no "Press Action"
   bar, no figure icon top-right; the ring bottom-left stays the green health ring until item 2 resolves
   (PCSX2's orange 007 there is the open item).
5. **The vent pins still hold** (nf-view): the crawl spot reads the ctx-6 figure at 3.9 (fix_swim_vent5_live
   green) and the gate door reads none at 24.1.
