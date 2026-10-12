# NightfirePort task `fix-ai-remainder`

**Remember your task.** You are working on the task `fix-ai-remainder` and only on it. Re-read this whole file at the start of **every** turn, after every message you get, before you do anything else, and begin each reply with one line: `Task: fix-ai-remainder, branch s/fix-ai-remainder`. Your claimed copy is `claimed/200-fix-ai-remainder.md` on the `queue` branch. If you lose track of anything (the task, the credentials, the file locations), it's all here.

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

## Your task: `fix-ai-remainder`, the guard AI items still open in GAPS G-026 / G-023 / G-021

`015-fix-npcs` owns the playtest bugs (C1–C3, the torso twist, the 07000006 stationary guards, the searchlight alarm). `110-fix-impacts` owns impact effects and ricochets, and `050-fix-lighting` owns the muzzle light. Don't touch those. Yours:

1. **SpecialDeath scripts:** guards with a scripted death (falls from ledges and balconies, death animations chosen by the placement). Find the placement field and the dispatch in `docs/systems/drone-damage.md` / the decompile (`NDrone2_*Death*`, `SpecialDeath`), and port them.
2. **Head tracking and the aim bone:** guards look at and aim towards Bond in the pose (the head and spine bones). Port the look-at the game applies after the animation sample, with its clamps. Coordinate with the torso-twist fix in `fix-npcs`: if master already has a look-at, extend it rather than adding a second one.
3. **Corpse fading:** the renderer's alpha fade-out of dead guards (the timer and the alpha ramp), and the removal at the end.
4. **Bond's 16 hit boxes:** guards' bullets currently hit Bond as one shape. Port the per-bone hit boxes and the per-location damage multipliers.
5. **Reload animation and the other drone weapons:** drones reloading (clip count, animation and sound) for every weapon guards carry, not just the default.
6. **Weapon swaps by animation events** (G-023: `DroneAnim_EventFunc`, datum 0x28/100), and dropped weapons matching the held one.
7. **Missing drone skins** (G-021, 33 placements): load the common animation file (`CommonAnimFile`, LoaderLoad type 4) and the sleeve substitutes (`AnimSleeveGetEntity`), so those guards draw and animate.
8. **Drone modes 9 / 0x12 / 0x1E in the alarm**, and the cover-node order in `Drone_InitComms` (G-026 notes).

Do them in this order; each one is separate. Every `Rand_*` call goes in the game's order.

Files: the drone modules in `crates/nf-game/src/`, `crates/nf-view/src/scene.rs` (the pose and fade hooks, minimal), and the docs in `docs/systems/drone-*.md`. Update G-021, G-023 and G-026 in `docs/GAPS.md`.

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

**Branch:** `s/fix-ai-remainder`.

## When you finish

1. Push your branch `s/fix-ai-remainder` (never `master`), with `docs/reports/fix-ai-remainder.md` as its last commit.
2. Then finish the queue item: `bash <queue checkout>/finish.sh 200-fix-ai-remainder done docs/reports/fix-ai-remainder.md` (or `blocked`, with the reason in the report).
3. Re-read this file once more and check that you did everything it asks.
