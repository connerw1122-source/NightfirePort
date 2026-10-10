# B-3a/b — Merge scenario systems + fix scenario placement

## Status: COMPLETE (committed locally — push requires credentials not available in this env)

Commit `599d373` is on local `master`. `git push` failed with "could not read Username for
'https://github.com'": the sandbox has no GitHub credentials (`~/.git-credentials`, `~/.ssh`, `GH_TOKEN`,
`gh` CLI all absent). The previous agent's worklog claims earlier commits were pushed — likely via a separate
mechanism outside this shell. The local commit is ready to push when credentials are available.

## Summary

Two parallel `--scenario` systems had drifted in `nf-view`:
1. `dev::scenario` / `dev::SCENARIOS` in `crates/nf-view/src/dev.rs` — 13 named scenarios incl. `grenade-throw`.
2. `apply_scenario(sim, name)` in `crates/nf-view/src/main.rs` — 5 named scenarios (later +1 from C-9 = `shotgun-reload`).

At startup `apply_scenario` ran first and printed `unknown scenario 'grenade-throw'`; both systems ran, causing
double-setup. Merged into a single registry in `dev.rs` (now 19 scenarios), deleted `apply_scenario`, and fixed the
placement helpers to spiral-search a valid LOS spot instead of guessing `guard.z + dist`.

## Changes

### `crates/nf-view/src/dev.rs`

- **6 new `pub fn`s** in `mod scenarios`: `weapon_switch`, `sg5_pickup`, `wire_music`, `autoaim`, `sniper_scope`,
  `shotgun_reload`. Bodies are the apply_scenario arms verbatim, with `sim.foo` → `if let Some(sim) = scene.sim.as_mut() { sim.foo }`.
- Added 6 arms to `scenario()` match and 6 names to `SCENARIOS`. **One registry: 19 scenarios.**
- New helper `spiral_search_for_spot(world, guard_pos, dist)`:
  - tries requested `dist` first, then `[8, 10, 12, 14, 16, 18, 20]` m fall-backs;
  - 24 angles at 15° intervals (0° = +z);
  - each candidate must have a `floor_below` within 4 m of guard.y, `|floor_y − guard.y| ≤ 3 m`, and a clear
    `line_of_sight(bond_eye, guard_head)` (eye = pos.y + 1.0, head = pos.y + 1.5);
  - first valid spot wins; yaw = `atan2(dx, dz)` faces Bond toward the guard.
- `place_bond_near_guard` / `place_bond_near_grenade_guard` rewritten: iterate over **all** matching guards; if no
  spot works for any guard → print ERROR, return None (scenario does not start).

### `crates/nf-view/src/main.rs`

- Deleted `apply_scenario` entirely (~88 lines including the C-9 `shotgun-reload` arm).
- Post-`App` block now reads (only `dev::scenario` is consulted):
  ```rust
  if dev {
      if let (Some(d), Some(n)) = (app.dev.as_deref_mut(), scenario.as_ref()) {
          if let Some((_, f)) = dev::scenario(n) { f(&mut app.scene); d.scenario = Some(leak_str(n)); }
          else { eprintln!("[dev] unknown scenario '{n}'. Available: {}", dev::SCENARIOS.join(", ")); }
      }
  }
  ```

### `docs/tools/dev-mode.md`

- Updated stale "Coordination" section to point at `dev::scenario` / `dev::SCENARIOS` (was: `apply_scenario`).

## Verification

- `cargo build -p nf-view` ✓ (no new warnings; 4 pre-existing warnings in scene.rs/gpu.rs/nf-game/nf-audio).
- `NF_ISO=/home/z/Nightfire-onfoot.iso cargo test -p nf-game --test ai_live` = **32 passed, 0 failed**.
- The 3 `nf-formats` disc_golden failures remain pre-existing (ISO mismatch, not a regression).

## Build environment note

`cargo` is at `/home/z/.cargo/bin/cargo`. ALSA dev headers + libasound.so are in
`/home/z/NightfirePort/tools-bin/alsa-prefix/usr/lib/x86_64-linux-gnu/`. Use:
```bash
export PATH=/home/z/.cargo/bin:$PATH
export PKG_CONFIG_PATH=/home/z/NightfirePort/tools-bin/alsa-prefix/usr/lib/x86_64-linux-gnu/pkgconfig
export LIBRARY_PATH=/home/z/NightfirePort/tools-bin/alsa-prefix/usr/lib/x86_64-linux-gnu:$LIBRARY_PATH
export LD_LIBRARY_PATH=/home/z/NightfirePort/tools-bin/alsa-prefix/usr/lib/x86_64-linux-gnu:$LD_LIBRARY_PATH
```

## Merge conflict during `git pull --rebase`

Origin's C-9 commit (`b9d5797`) added a `shotgun-reload` arm to `apply_scenario` *after* my stashed work had
deleted the whole function. Resolved by keeping my deletion and porting the `shotgun-reload` arm to dev.rs as
`shotgun_reload` (above). Also picked up two new test files (`doors_01_live.rs`, `textures_live.rs`) and a new
example (`dump_vertex_colours.rs`) from origin cleanly.

## Files modified

- `crates/nf-view/src/dev.rs` (533 → 695 lines, +162)
- `crates/nf-view/src/main.rs` (884 → 812 lines, −72)
- `docs/tools/dev-mode.md` (Coordination section updated)
- `/home/z/my-project/worklog.md` (this entry appended)
