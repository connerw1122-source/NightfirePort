# foliage-bullets — the bullet trace vs foliage (658)

Branch `s/foliage-bullets` (from origin/master 4108d7b2). The playtest: "those trees are, for
some reason, bulletproof, like the leaves" (07000005, the small trees near the start).

## What the game's data and code say

**The foliage pass machinery is real [M]:**
- `Collide_Intersect` 0x1ebaa8 sets the per-hit pass flag (`HITDATA+0x51` bit 1) from TWO sources:
  the glist node's flags bit 0x40 (lines 143-145: `*(int *)(iVar10 + 0x54) + 0x94 & 0x40` — the cel
  word, which `parsemap_block_map_data_static` 0x1d1998:103 fills from the placement's
  `celglist+0x44` = the port's `MapObject.flags`) and the surface byte's bit 0x40 (disasm
  0x1ebf78-0x1ebf80, no query-flag gate). The surface byte's bit 0x80 sets it ONLY for queries
  whose HITTEST+0x98 carries bit 0x1 (disasm 0x1ebf8c-0x1ebf94) — the 0x721-family — not the
  bullet flags.
- `Collide_Update` 0x1ea398:216-229 walks the tracer past pass-flagged hits (the walk advances;
  a non-pass hit stops it), and `Bullet_CollisionHandler` 0x12ce80 re-derives the same pass
  per hit: state 1 (flying) past pass hits, state 2 (dead) at the first stop hit, with
  `Effect_Create` still running per hit (pass hits included) and the ricochet mask 0 on passes.
- The bit-0x8 queries DROP pass-flagged hits outright (fraction 1e8, disasm 0x1ebff0-0x1ec010).
  Confirmed members: the player's aim trace 0x20a / 0x208 (`Player_WeaponInitBullet` 0x1a4bf0:154-159,
  0x208 when weapon_data+0x6C & 0x800), the laser-aim sphere 0xa0a, the guided-missile beam 0x20a,
  and `Bullet_homing`'s LOS 8 (0x12f358:92). The guards' sight (0x721) carries NO bit 0x8 — sight
  stays blocked by foliage.

**The 07000005 foliage [D] (the port's level parse):**
- The small trees by the spawn: obj 96 (188 tris, surface bytes 0x13/0x03) placed 5× and obj 98
  (1517 tris, 0x13/0x04) placed 2×, ALL with glist flags 0x41 — node-flagged bullet-pass.
- Collision-only leaf cards: obj 21/109/144 (2 tris each) and obj 117 (48 tris), flags 0x10 (the
  draw-pass skip bit), surface byte 0x40 — material-flagged bullet-pass.
- A separate wood core obj 97 (30 tris, mat 0x04, flags 0x00000000) sits inside the obj-96 trees —
  NOT flagged: the game's own aim trace reports it as the first stop behind the canopy.

**The flight does NOT pass — the live evidence [L]:**
A bullet's own HITTEST stays inactive forever (`Bullet_init` 0x12c668 disasm: `sh $zero, 0xe4($s1)`
= HITTEST+0x94 = 0), so `Collide_Update`'s per-object walk never processes bullets and the pass
machinery above never lifts a bullet's flight stop. The live-pinned guard cover trace
(`ai_live::guard_takes_cover_like_the_game`, UnderCoverAim 37 ticks — measured on the real game)
fails with BOTH candidate flight models I built (the bit-0x8 drop AND the Collide_Update keep-walk:
90 ticks each) and passes only with the port's original first-hit flight — the round stops on the
flagged tree's first face. So the game's bullet flight has no pass handling at all; whatever traces
bullets per tick (the mechanism is still unidentified — see the live checks), it consults neither
the node flag nor the walk.

## What the port does now (this branch)

- `CollisionWorld` carries the node pass flag per triangle (`Geo.pass`, from `MapObject.flags &
  0x40` at build) and exposes the bit-0x8 query family: `segment_bullet` / `segment_surface_bullet`
  (pass-flagged hits dropped — the aim/LOS semantics) and `segment_surface_flags` (the hit plus its
  node flag, for the keep-walk). `effect::penetration_roll_node` folds the node flag into the roll
  (no draw on a flag pass; the 0x80 roll still drawn exactly when the byte carries it).
- The flight paths keep the first-hit stop (unchanged from master) — the live-pinned behaviour.
- Tests: `nf-game` world unit test `bullet_queries_drop_the_pass_flagged_hits` (node flag drop,
  0x40-byte drop, 0x80 kept for the walk's roll, the wall behind reported); `nf-view`
  `fix_foliage_bullets_live` (3 tests on the real 07000005: the flight stops on the tree's 0x13
  shell while the aim trace reports the unflagged 0x04 core behind it; the guard-style round and
  sight stop on the shell; the 0x40 leaf cards leave the aim trace only).

## Verification

Ran: `nf-game --lib` 235/235; `nf-game` ai_live 35/35 (the cover pin back green after the flight
revert — it was the deciding experiment), grenade_live 2/2, spawner_kill 1/1, nav_kick_live 5/5+1
ignored, level_live 9/9; `nf-view` fix_foliage_bullets_live 3/3. NOT run: the full fast-runner
suite — this sandbox reaps background jobs between tool calls (the runner dies in <15 s), and the
~62 min foreground run does not fit the session window; flagged for the driver.

## The user's symptom, explained

The round stops INSIDE the canopy on the 0x13 shell. Class 19's EffectInfo record has NO decal and
NO gas puff — only debris chips ([D] the port's parsed table: decal 0, all three gas slots empty,
debris mesh 33554730) — so next to every wall that shows a bullet hole, the trees read as
"bulletproof". The game shows the same stop; if the chips are the visible difference, the fix is
the debris render for that mesh, not the trace. The trace work above settles what the trace does;
the render needs the main agent's screen.

## Live checks for the main agent

1. **The bullet's per-tick flight trace** — who fills the bullet obj's `+0xD0` hit list?
   Watch `obj+0xD0` (and `Collide_Pick`'s SrtList writes at 0x1ea508-0x1ea658) during a guard's
   shot on 07000005; expected: a writer that is NOT the object's own inactive HITTEST walk.
2. **A shot at a small tree (e.g. --bond 97.0,24.3,90.8 facing the tree at 91.5,25.5,90.8)**:
   does a chip/debris burst appear ON the canopy (the class-19 debris) and does a bullet hole
   appear anywhere? Expected per this analysis: chips only, no hole, the round dead inside the
   canopy; the aim POINT (the crosshair ray) lands past the tree (the aim trace drops it).
3. **gdbwatch `HITDATA+0x51` (bit 1) for the first hit of a player shot at the tree**: expected
   bit 1 = 1 (the node flag) if the trace that feeds the handler is the aim trace, 0 if the
   flight's own hit feeds it.
4. **The homing LOS** (a heatseeker shot near foliage): whether the target scan sees through the
   canopy (flags 8 [M 0x12f358:92] says yes — the port keeps master's blocking `segment` here,
   one-line flip to `segment_bullet` once confirmed).

## What's left

- The flight-trace mechanism (live check 1) — everything else in the chain is pinned.
- The class-19 debris render check (the main agent's screen) — the visible half of the playtest.
- The full suite on the final tree (the sandbox could not keep the runner alive).
