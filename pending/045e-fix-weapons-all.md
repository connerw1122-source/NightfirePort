Read `docs/playtests/2026-10-11.md` first: it's the user's multi-level playtest, and the item ids below refer to it.
10. **Push your report with your branch.** Write the plain-language report (the same text you give the user) to `docs/reports/<your task id>.md` and commit it as the last commit on your branch. The main agent reads it from there, so the user doesn't have to paste it. Never put credentials or URLs in it.
11. **Write everything in English:** code, comments, docs, the WORKLOG, commit messages and your report to the user.

## Your task: `fix-weapons-all`, every weapon in `weapon_data`

Go through every selectable weapon id (`docs/atlas/weapon-data.md`): fire modes and the fire-mode switch, alt-fire, scopes and zoom steps, burst and auto timing, spread, ammo types, reload kinds (clip versus shell), the rocket launcher (projectile, explosion), the grenade launcher, the crossbow and darts if present, and melee.

For each one: a headless test that equips it through `Pickup`/`Player_EquipWeapon`, fires through input, reloads, and asserts the rounds, projectile or bullet, and sound against `weapon_data`.

Also the user's report: on the gondola level, the rocket launcher and its ammo can't be picked up. Coordinate with `fix-objects` if it has already fixed that.

Files: `crates/nf-game/src/{gun,bullet,pickup}.rs`, and the weapon parts of `scene.rs`.

## New from the user's playtest of 2026-10-12 (highest priority: "need to prioritize getting every gadget and weapon to work properly")

- **Every weapon needs a first-person model** (the dev panel: "Every single weapon needs to have a first person model"; "still missing models").
- **Fire modes:** the Scorpion launcher's heat-seeker mode doesn't work, and the user doesn't know how to change modes. Port the alt-fire/mode switch and say which key it is in your report.
- **The frag grenade** "does not throw well. And it looks really buggy when you throw it. And it blows up immediately as soon as it hits the ground"; an earlier grenade "bounced back and hit me … I heard the bouncing sound, but no explosion" (the explosion visual/sound may be `120-fix-explosions`'s; you own the throw and the fuse).
- **The flashbang kills enemies** "even if I dont throw it directly at them". A stun grenade shouldn't kill.
- **Guards drop multiple objects** each ("all enemies are dropping multiple objects").
- **07000012: the starting pistol "does not sound right"**.
- **Punching:** "fists alternate though I cant really tell if I knocked them out or killed them". Make a knock-out look like the game's.
- **Switching levels in the dev panel** "eventually was given a weapon I cant use and had to swap".
- On 07000012 there were "killable MP, unkillable NPCs, like enemies": find which drones can't be killed.

## New from the user's second round (2026-10-12 evening)

- **The dev panel gives the wrong items** (`nades05`): "a frag grenade spawns what looks to be a stun grenade that explodes like a frag grenade. I cant tell whats what. no clue what the rocket launcher is". Audit the panel's give list against `weapon_data` (ids, names via the game's labels, the pickup kind it creates) so every entry gives exactly the named weapon with its own view model and projectile (models-weapons-items pinned the projectile meshes: frag 0x35 → 0x02000726, stun 0x36 → 0x0200071f, rocket 0x3A → 0x0200074f). Test: each panel entry → the equipped weapon id, its view model and the thrown/fired projectile's mesh.

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

**Branch:** `s/fix-weapons-all`.
