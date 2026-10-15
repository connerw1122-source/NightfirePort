## Your task: `remote-mine-play`, the remote mine bounces; the detonator is on the wrong button

User (07000005, `--give 0x37,0x38`): "The remote mine bounces for some reason. And the detonator, for some reason, you
have it set to the watch rather than the alternate fire of the mine." Make the remote mine 0x37 stick on contact as the
game does (+0x70 & 0x100 [L], like the satchel/hat), and make the detonation the mine's own alternate fire / mode
press as the game binds it (0x38 is its detonator twin - `Player_WeaponInitBullet` 0x1a4bf0's +0x68 & 0x20000 arm);
the detonator must not appear in the watch/gadget cycle.
This comes from the user's playtest 1014-1725 of the real-input build (`nf-view <iso> <level> --dev`, mouse +
keyboard). The user was very disappointed: several merged fixes passed their tests yet were wrong or broken in play.
Your test MUST drive the real input path (`Sim::step_bond` with the `Input` the window builds - lib.rs maps the mouse
and keys), not internal setters, and must fail first on master. The main agent will compare against PCSX2 after you
finish; put the exact steps to reproduce in the report.

Files: `crates/nf-game/src/{grenade,gun}.rs`, `crates/nf-view/src/scene.rs`.

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

**Branch:** `s/remote-mine-play`.

---
claimed-by: sandbox7 at 2026-10-15T01:02Z

---
## Report (done, 2026-10-15T02:00Z)

Closed by the driver: the report is on `s/fix-remote-mine-play` (docs/reports/fix-remote-mine-play.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-15T02:48Z)

# fix-remote-mine-play — the remote mine sticks on contact; the detonation is the mine's own mode press

Branch `s/fix-remote-mine-play` (from origin/master 2ee3f2b8, merged origin/master bf2bd176 mid-task).
Playtest 1014-1725 (07000005, `--give 0x37,0x38`, mouse + keyboard): "The remote mine bounces for some
reason. And the detonator, for some reason, you have it set to the watch rather than the alternate fire
of the mine."

## What the game does (the evidence)

- **The stick** [M `Bullet_CollisionHandler` 0x12ce80:255-266, 296-330, 345-352]: a `+0x70 & 0x100`
  record's world hit sets `obj+0xF4 = 3` (stuck), aligns the matrix to the NEGATED hit normal and
  parks at `hit + normal·0.04`. The mine 0x37's own branch [:296-330] walks the objects for
  same-weapdef same-owner type-5 bullets (no state filter — in flight counts too), `count < 6` gates
  the park; past 6 the OLDEST (min `obj+0xEC` [*suggested*: the port's spawn order]) takes
  `+0xFE |= 1` and the new one still parks. The arm writes NO fuse (BU+0xFC keeps `Bullet_init`'s
  7777.0 player / 420 guard [M 0x12c5a0:214-223]) and plays NO sound (no `Sound_Play3D` in the 0x37
  arm — 0x3b/0x45 play 0x4a7, 0x3a plays 0x446). [L] records (`ee_snapshot.bin` @
  `0x2bf150 + w·0x10C`): 0x37 `+0x70 = 0x18300` (the stick bit set), 0x3a = 0x80100, 0x3b = 0x300,
  0x45 = 0x100; frag 0x34 = 0x280 / stun 0x35 = 0x2280 / smoke 0x36 = 0x20280 lack it — exactly the
  stick family.
- **The stuck fuse** [M `Bullet_DoTrails` 0x12d890:268-285]: the 0x37 arm skips the decrement ONLY
  while `BU+0xFC == 7777.0`; any other fuse (the detonator's mark, a guard's 420) keeps counting in
  state 3 and detonates at `< 0` through the shared flash/smoke/explode tail.
- **The detonator** [M `Player_WeaponInitBullet` 0x1a4bf0's `+0x68 & 0x20000` arm, disasm
  0x1a4c30-0x1a4cd4]: the walk marks EVERY player-owned bullet whose record's +0x02 equals the fired
  record's +0x02 and whose BD+0x30 is the player, with **`BU+0xFC = −10.0`** — disasm 0x1a4c54-0x1a4c88:
  `lui $at, 0xc120; mtc1 $at, $f0; …; swc1 $f0, 0xfc($a1)` = 0xc1200000 = **−10.0f**. The earlier
  port note ("BD+0xFC = 10.0") dropped the sign; −10.0 fails DoTrails' `0.0 <= fuse` gate on the
  NEXT tick — the mine blows at once, which the main agent's live session measured ("Fire on the
  detonator blows the mine at once (a full-screen blast the next frame)").
  docs/live/2026-10-14-weapon-measurements-pcsx2.md). Then `Player_WeaponHasAmmo(base)` arms the
  switch back to the mine (`BL+0x63 = base` [M 0x1a4cb8]) or, dry, clears the base's clip/owned
  bytes and lets `Player_GetBestWeapon` take the hand [M 0x1a4cbc-0x1a4cd4].
- **The binding** [L records]: 0x38 = 0x00020010 at +0x68 (the arm), **+0x04 = 0** — the cycle's gate
  [M 0x1a1894/0x1a6f80] — so the detonator NEVER appears in the weapon wheel or the gadget/watch
  cycle (G walks 0x4A..0x5E [M 0x1a707c]; 0x38 is not in it either). 0x37 = 0x14090, sel 1, twin +1
  (the runtime patch [M `Player_Init`], which the loader already applies). The mode press (pad ○ /
  keyboard B) is the game's route: the twin switch through the shared ammo-type 22 slot [L +0x90 = 22
  on both]. The detonator's view model is Bond's left wrist with the watch and the HUD reads
  "Detonate" — the live session: "It is reached through the mine's mode press, not the gadget cycle",
  and the user's "you have it set to the watch" is the watch-detonator view model, not a watch-gadget
  binding. HUD mode names [L USATxt.dat via `Txt_BindLabel`]: 0x37's +0x30[0] = 0x0500000b =
  "Grenade", 0x38's = 0x0500004d = "Detonate".

## What changed

- **`crates/nf-game/src/grenade.rs`** — the mine 0x37 joins the `+0x100` world-hit stick arm
  (offset 0.04, no fuse write, the phase normal = the hit normal; the comment carries the branch
  line-cites), and the stuck `0x37` fuse arm follows DoTrails' rule: frozen ONLY at 7777.0, any
  other fuse (the detonator's mark, a guard's 420) falls through to the shared fuse arm.
- **`crates/nf-game/src/drone.rs`** (shared, additive — listed per the rules): the Stick consumer
  mutes 0x37 (the game's mine arm plays no sound; the hat/satchel keep 0x4a7, the tripbomb 0x446);
  the 6-placed cap walk (the tripbomb's every-tick consumer) now counts stuck 0x37 mines with the
  same oldest-out rule.
- **`crates/nf-view/src/scene.rs`** — the detonator's mark writes `range = travelled − 10.0` (the
  port's fuse model is `range − travelled` = BU+0xFC; the old `+ 10.0` stand-in produced a 10-tick
  fuse where the game's is a next-tick boom), and the mark comment now records the walk's
  no-break/all-mines and no-state-filter facts from the disasm.
- **`crates/nf-view/tests/fix_remote_mine_play.rs`** (new) — three NF_ISO tests through the real
  input path (the panel give = the user's `--give`, then trigger/mode/wheel/G via `Sim::step_bond`'s
  `Input`), no internal setters on the weapon path.

## Reproduce before you fix (shown failing on master first)

`git stash push crates/nf-game/src/grenade.rs crates/nf-game/src/drone.rs crates/nf-view/src/scene.rs`,
then `NF_ISO=$NF_ISO cargo test --offline --release -p nf-view --test fix_remote_mine_play -- --test-threads=1`:

- `the_remote_mine_sticks_on_contact_and_never_bounces` FAILED — "tick 48: the mine bounced before
  it stuck (pre-fix behaviour), left: 1, right: 0" (the user's bounce).
- `the_mode_press_detonates_the_placed_mine_and_returns_to_the_mine` FAILED — "the mine never stuck"
  (with the mine resting instead of parked, the game-faithful detonation route is unreachable).
- `the_detonator_never_appears_in_any_cycle` passed pre-fix — correct: the +0x04 gate was already
  right on master; the test LOCKS the binding (B ↔ the twin pair, the wheel/G never offer 0x38) so
  the playtest regression cannot come back.

Restored (pop), the same command: **3 passed**. A scratch probe also measured the pre-fix mark at
+10.0 (the boom ~11 ticks after the detonator press, hidden between the per-tick `explosions.clear()`
windows) vs the disasm's −10.0 next-tick boom — the timing assert (`boom_tick == 1`) fails on the
+10.0 tree by construction.

## Verification

- The three tests green on the merged tree (origin/master bf2bd176).
- Neighbours green: `fix_satchel_tripbomb_live` 5, `oddjob_hat_live` 2, `explode_live` 2,
  `fix_grenade_throw_live` 4, `fix_grenade_types_live` 3 (+1 ignored), `fire_modes3_live` 5 (+4
  ignored), `fix_samurai_fire_live` 3, `weapon_switch_live` 2, `nf-game --lib` 157.
- The re-locked `fix_handcannon_live` 3/3 (see "the sister test re-locked" below).
- Full fast suite on the final tree (`tools/test/suite.py`, foreground chunks — the sandbox reaps
  background processes between tool calls; every one of the 292 test binaries ran):
  **1207 passed, 6 failed, 58 ignored** (~43 min wall). The 6 failures:
  - `disc_golden` ×3 — the allowed slim-ISO set.
  - `fix_0c_pylon_live` ×2 and `probe_pylon_render` ×1 — the GPU-adapter environmental class
    (`adapter: NotFound { supported_backends: VULKAN | GL }` — this box has no adapter; the same
    class the fix-samurai-fire session documented as "the 2 GPU-adapter"). They fail identically
    with and without this task's changes (the src diffs touch grenade/stick/detonator code only).

### The sister test re-locked

The task's −10.0 correction changed an observable the existing `fix_handcannon_live::
the_detonator_detonates_the_placed_mine_and_switches_back_to_it` had pinned to the old +10.0
stand-in: the boom now lands on the press tick, and the grenade section's per-tick
`explosions.clear()` wipes it before that test's first look, so it saw "no blast ever". The test's
doc comment cited "BD+0xFC = 10.0" — the sign the disasm disproves. Re-locked: it now reads the
explosion on the press tick first (with the ≤14-tick fallback for the game's next-update shape),
and the doc cites disasm 0x1a4c54-0x1a4c88. Green 3/3.

## Live checks for the main agent

PCSX2 (07000014, the pickup at (38.7, 0, 33.6) — the 2026-10-14 session already measured the game's
side; these check the PORT):

1. **The stick**: `nf-view <iso> 14 --dev`, pick up the mine, throw at any wall/floor: the mine
   parks motionless from the first contact (no bounce), NO stick sound (the satchel's 0x4a7 clang
   must NOT play), the mine stays past 60 s with no self-detonation.
2. **The route**: ○ (the mode press) → the watch-detonator view model, HUD "Detonate"; ○ again →
   back to "Grenade". G and the wheel must never land the detonator.
3. **The boom**: fire on the detonator → the mine blows at once (the game: the next frame; the port
   marks BU+0xFC = −10.0 and DoTrails booms the next update — if the port's boom lands on the SAME
   frame as the press, that is a sub-tick ordering difference of one 16 ms tick, reported here as
   [*suggested*] acceptable) with the record's radius 8 / damage 100, then the view returns to
   "Grenade".
4. **The 6-placed cap** [M 0x12ce80:296-330]: place 6 mines, throw the 7th — when it sticks, the
   OLDEST placed mine vanishes (no explosion — a silent `+0xFE |= 1` delete) and the 7th parks.
5. **A guard-thrown mine** (07000005's sel throwers or a level with mine guards): it sticks where it
   lands and blows on its own ~7 s later (the 420-tick fuse keeps counting while stuck [M
   0x12d890:268-285]).
6. **The dry detonator** [M 0x1a4cbc-0x1a4cd4]: with no mines left, fire the detonator: the mine
   leaves the inventory and GetBestWeapon takes the hand.

nf-view `--bond` spots: `--bond 38.7,1,33.6,0,0` on 14 (the pickup), `--bond 33.8,4.6,-3,0,0` (the
second pickup's area) — throw at the nearest wall for checks 1/3/4.

## What's left / notes

- The stick's char-hit side (a sticky record hitting a GUARD bounces per [M 0x12ce80:239-249]) is
  the port's pre-existing grenade-vs-guard behaviour — unchanged here, untested (no guard-contact
  grenade test exists; the satchel task left the same gap).
- The 0x39 MP twin (base 0x37, the −2 link) shares every tested property but is only reachable in
  MP — untested.
- The panel's Give of 0x38 returns the twin id as the switch target (`panel.rs:305`), while
  `Player_EquipWeapon` writes the BASE into `anim+0x63` [M 0x1a8a58:39-41] — a panel-only
  convenience path outside this task's file list; the `--give` at load is unaffected.
- The port's mark-and-boom can land on the same tick (the grenade update runs after the mark within
  one `step_bond`), where the game's DoTrails booms the next update — a ≤1-tick difference, noted in
  live check 3.
- Sandbox note: this checkout carried a leftover foreign stash (`WIP on s/fix-guard-weapons3`) from
  an earlier session; a clean-tree check's `git stash pop` consumed it and briefly restored its
  scratch test into `crates/nf-game/tests/ai_live.rs`. The scratch (a `zz_scratch_cover_stream`
  panic-by-design probe) was reverted immediately — no committed work of that session was touched.
  Nothing is left on the stash stack.

