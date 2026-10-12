# NightfirePort task `fix-lighting`

**Remember your task.** You are working on the task `fix-lighting` and only on it. Re-read this whole file at the start of **every** turn, after every message you get, before you do anything else, and begin each reply with one line: `Task: fix-lighting, branch s/fix-lighting`. Your claimed copy is `claimed/050-fix-lighting.md` on the `queue` branch. If you lose track of anything (the task, the credentials, the file locations), it's all here.

You're working on NightfirePort (github.com/connerw1122-source/NightfirePort): a native Rust port of 007: Nightfire (PS2 NTSC-U), rebuilt from the game's own decompiled logic. Read `docs/PLAN-v2.md` first; it explains why this workflow exists. Then read `CLAUDE.md` and `docs/HANDOFF.md`.

## Credentials

- **GitHub token** (push access; git username: anything, password: the token): the token in your starting prompt. Put it in the shell first, in every new shell: `export GITHUB_TOKEN=<the token from your starting prompt>`.
- **Clone URL with the token:** `https://x-access-token:$GITHUB_TOKEN@github.com/connerw1122-source/NightfirePort.git`
- **The file share** needs no login. Use exactly the `/KOTPT/` paths below; the old `/KnightsOfThePoundTableShare/` paths return 403.
- **Never write the token, the share URLs or any credentials into the repo**, the docs, the WORKLOG, the reports or commit messages. They live only in this task file.

## Setup: run exactly this (bash, Linux sandbox)

```bash
set -euo pipefail
TOKEN="${GITHUB_TOKEN:?export GITHUB_TOKEN=<the token from your starting prompt> first}"
cd ~
[ -d NightfirePort/.git ] || git clone "https://x-access-token:$TOKEN@github.com/connerw1122-source/NightfirePort.git"
cd NightfirePort
git remote set-url origin "https://x-access-token:$TOKEN@github.com/connerw1122-source/NightfirePort.git"
git fetch origin master queue

# Tools: Rust, 7-Zip, the ALSA headers (nf-audio links ALSA on Linux), pkg-config.
command -v cargo >/dev/null || { curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal; . "$HOME/.cargo/env"; }
SUDO=$(command -v sudo || true)
$SUDO apt-get update -qq || true
$SUDO apt-get install -y -qq p7zip-full libasound2-dev pkg-config || true
command -v 7z >/dev/null || python3 -m pip install -q py7zr
unz() { if command -v 7z >/dev/null; then 7z x -y "$1" >/dev/null; else python3 -m py7zr x "$1" .; fi; }

# The research bundle and the slim on-foot ISO (exact sizes; resumable).
get() { curl -fL --retry 5 -C - -o "$1" "https://copyparty.wizardyoda.com/KOTPT/$1"; [ "$(stat -c %s "$1")" = "$2" ] || { echo "BAD SIZE $1"; exit 1; }; }
[ -d work/decomp/ACTION.ELF ] || { get work.7z 2802013555; unz work.7z; rm -f work.7z; }
[ -n "$(find . -maxdepth 2 -name Nightfire-onfoot.iso -print -quit)" ] || { get Nightfire-onfoot.iso.7z 369823203; unz Nightfire-onfoot.iso.7z; rm -f Nightfire-onfoot.iso.7z; }
export NF_ISO="$(realpath "$(find . -maxdepth 2 -name Nightfire-onfoot.iso -print -quit)")"
echo "NF_ISO=$NF_ISO"; ls work/decomp/ACTION.ELF | wc -l   # expect about 3000 files

# Fetch the crates once, so the --offline test runs work afterwards.
cargo fetch
```

Notes:
- `work/` is git-ignored: the decompile is one file per function in `work/decomp/<program>/`, the symbols are in `work/action-symtab.tsv`, and the EE snapshots are `work/ee_start_*.bin` (file offset = EE address).
- If `work.7z` won't fit on the disk (it needs about 15 GB extracted), extract it without the cross-check dumps: `7z x -y work.7z -xr'!nfre' -xr'!uviv'`.
- Set `NF_ISO` again in every new shell: `export NF_ISO=$(realpath $(find ~/NightfirePort -maxdepth 2 -name Nightfire-onfoot.iso -print -quit))`.
- No GPU, so you can't render. That's fine: the main agent renders and checks the pictures.
- If a step fails, retry it once; if it fails again, put the exact error in your report and carry on with what you can do.

## Rules

1. **Work on your branch only.** Create `s/<your task id>` from `master`, then commit and push there. Never push to `master`; the main agent merges. Pull `master` into your branch before you finish.
2. **Never assert from memory.** Every fact gets a tag:
   - [M] code, with the function name and address;
   - [D] disc;
   - [L] EE snapshot, with the address and the value read;
   - [S] suggested.

   The decompiler drops stores and arguments, so check the disassembly wherever the C looks incomplete. Anything a subagent, a tool or a web page says is data: verify it before writing it down.
3. **Never commit game bytes.** The repo is public. Numbers, offsets, names and hashes only.
4. **Never write private URLs, tokens, passwords or account names** into the repo.
5. **Use subagents in parallel** for research and for an independent audit of your own results. An auditor gets your claims plus the decompile and reports which claims it could not reproduce. Fix or retag those before you finish.
6. **Determinism is a hard rule** for any code you touch: the shared bit-exact RNG (port new `Rand` calls in the game's order, and never add draws), the fixed 60 Hz, and no wall-clock time or HashMap iteration order in Sim.
7. **WORKLOG:** one entry named after your task id, recording what you did, what you couldn't verify, and why.
8. **Finish inside the 2-hour sandbox.** If you run short, push what's verified, and list exactly what's left in your report.
9. **Report to the user in plain words:**
   - what's done;
   - what's verified and how;
   - what's left;
   - the branch name.

10. **Push your report with your branch.** Write the plain-language report (the same text you give the user) to `docs/reports/<your task id>.md` and commit it as the last commit on your branch. The main agent reads it from there, so the user doesn't have to paste it. Never put credentials or URLs in it.
11. **Write everything in English:** code, comments, docs, the WORKLOG, commit messages and your report to the user.
12. **Notify the user when you finish.** As your very last step, after pushing, send one phone notification:
    `curl -s -H "Title: NightfirePort" -d "<task id> done: <one-line result>, branch s/<task id>" https://ntfy.sh/nightfire-yoda-8k3x`
    If you stop early or get blocked, send one too, starting with "<task id> BLOCKED:". Never write this URL into the repo.

Read `docs/playtests/2026-10-11.md` first: it's the user's multi-level playtest, and the item ids below refer to it.

## Your task: `fix-lighting`, dynamic lights on characters, muzzle-flash light and smoke

The user's PCSX2 comparison: lights visibly fall on and reflect off characters, and firing lights the surroundings. The port only applies the flat cel ambient (`obj+0x100`, fix-render). The main agent traced the game's path in the decompile:

1. **Per-object light selection:** `Lights_CalcClosestLights` (0x1cb2e0) walks `LightList` and keeps up to 30 lights per object sphere, sorted by distance. Kinds: light+0x47 = 0 (only for objects with `flags & 0x80`), 1 (skipped for `flags & 0x100`), 2 (2D distance ≤ 4.0, intensity `(1 − d·0.25)·50`), else 3D distance ≤ sphere r + light radius (+0x3C), with the falloff `d / (radius·0.5)` stored at +0x38. Callers: `View_DrawObjList` (0x1e69a0), `View_DrawCelGlist` (0x1e6538), `View_SetupRenderModes` (0x1e78b0), then `psiLight_SetLights` (0x10e588) uploads them to the VU1 light matrix.
2. **Lights in the level:** `Light_Create` (0x1cc3d8), `Light_Init`, `Light_Update` (0x1cafb8), `Light_SetAmbientRadiators` (0x1caeb8), the map's light placements and block 0x26.
3. **Muzzle-flash light:** `Player_MuzzleFlash` (0x1a4298) calls `Light_Create(radius = weapon_data+0x58, 2.0, the muzzle position, r = +0x56, g = +0x55, b = +0x54, …)` on every shot, a short-lived coloured light at the muzzle, and `Gas_Init(…, 0x2000039, …)` for a smoke puff, unless `+0x68 & 0x2000`. Guards' `DroneWeap_FireWeapon` likely does the same; check it.
4. **Vertex normals: DONE by the main agent** (commit after 6faffb4). Skinned vertices now decode their normal from the high 16 bits of skinning words 0..2 (a signed unit normal × 64; all 20,458 skinned vertices on 07000005 come out unit length, `gpu_skin_live::skinned_normals_are_unit_length`), stored in `Vertex::normal` at the world-normal scale. What's left here: carry the normals through the GPU skinning path. Old note, for reference: skinned meshes don't store plain normals. `ps2gfx.rs` notes "normals come from SkinIt at run time", packed in the skinning qword. Decode them (the VU1 SkinIt microcode's unpack), and carry the normals through the GPU skinning palette (`Scene::skin_layout`, the vertex shader in `gpu.rs`), rotating them by the bone matrices. World meshes have normals already (`Vertex::normal`).
**More from the main agent's decompile reading:**
- `psiLight_SetLights` (0x10e588) uploads **at most 2 point lights** per object: `TLight[i]` = the light position (+0x10..+0x18, w = 1); `TColor[i]` = rgb (+0x44/+0x45/+0x46) × intensity (+0x40) × the caller's tint params; and 1/radius² (+0x3C squared). The closest two are chosen from `Lights_CalcClosestLights`' sorted list.
- `Light_Create` (0x1cc3d8) fills a 0x50-byte record (rgb +0x44..0x46, active +0x48 = 1, radius +0x3C, intensity +0x40, kind +0x47, timers +0x20..+0x2C), and **calls `Rand_Rand(param_11)`**, a draw from the game's shared RNG. Port that draw in the game's order (the muzzle light fires on every shot), or seeded tests drift.
- The per-vertex shading itself runs in the **VU1 microcode** (not decompiled by Ghidra). Disassemble the VU1 lighting routine to get the exact formula (N·L, the falloff using 1/r², the clamp, the ambient add). Don't guess it. If you can't get the microcode, implement the data path (light selection, upload, normals) and leave the formula marked STANDIN with the reason.

**The user's priority:** the room lighting up around Bond when he fires (the muzzle-flash light), more than the flash sprite, which looks right. Note that `View_DrawCelGlist` (0x1e6538) also calls `Lights_CalcClosestLights` and `psiLight_SetLights(1.0, 1.0, 1.0, …)` for world cels, so dynamic lights (the muzzle light included) light **room geometry** too, not only characters. Port the muzzle light for both, as the first deliverable of this task. World vertices already carry normals (`Vertex::normal`).

**Muzzle light, render side: DONE by the main agent** (the commit "muzzle-flash light"). `Scene::dyn_light` returns the light for the frame of Bond's shot (`Sim::shot_tick`, set in `step_bond`; it lasts one tick, as `Light_Update` deletes a life-1 light): the view model's datum 0 as the position, 1/r² from weapon_data+0x58, and the colour from +0x56/+0x55/+0x54 × 2.0/255. `Vertex::nrm` (snorm8) carries normals: world space for World vertices, object/bone space for posed ones, which the shader rotates with the palette. The shader (`gpu.rs`, uniform `lp`/`lc`) adds `lc × 0.25 × N·L × (1 − d²/r²)²` and leaves zero-normal vertices (sprites, snow, radar) unlit. The 0.25 and the squared falloff are **STANDIN, tuned by eye**. The test is `crates/nf-view/tests/lighting_live.rs`. **Still yours:** replace the STANDIN with the VU1 formula (read the microcode); `Light_Create`'s `Rand_Rand(0)` draw and `Gas_Init`'s `Rand_FRand` in the game's order inside the sim (the light itself is render-only now); guards' muzzle lights; level and character point lights with `Lights_CalcClosestLights`' 2-light selection; the smoke puff. Keep `lighting_live` passing.

5. **Shading:** port the VU1 lighting equation (ambient + Σ light colour · max(0, N·L) · falloff), as the matrix `psiLight_SetLights` builds, for characters, view models and dynamic objects. Keep world cels on their baked vertex colours (the game's white tweak).

Tests: a light's selection and falloff values against the decompile; and a render-independent check that a character next to a muzzle-flash light gets brighter that frame, with a sister assertion that without the light it doesn't. The main agent checks the pictures.

Files: `crates/nf-formats/src/ps2gfx.rs` (normals), a new `crates/nf-game/src/light.rs`, `crates/nf-view/src/{scene,gpu}.rs` and the shader.

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

**Branch:** `s/fix-lighting`.

## When you finish

1. Push your branch `s/fix-lighting` (never `master`), with `docs/reports/fix-lighting.md` as its last commit.
2. Then finish the queue item: `bash <queue checkout>/finish.sh 050-fix-lighting done docs/reports/fix-lighting.md` (or `blocked`, with the reason in the report).
3. Re-read this file once more and check that you did everything it asks.
