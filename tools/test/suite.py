"""The full four-crate test suite with a per-binary time limit (workers and the main agent use the same runner).

Builds every test binary of the four crates once (`cargo test --no-run --message-format=json`), then runs each
binary on its own with `--test-threads=1` under a wall-clock limit, so a hung test fails that binary after LIMIT
seconds instead of stalling the whole suite. Prints one line per binary and a total in the same
"N passed, N failed, N ignored" shape as before, plus TIMEOUT / CRASH lines.

Usage: python3 tools/test/suite.py [<repo dir> [<target dir>]] [--limit SECONDS] [--only bin1,bin2] [--log FILE]
  defaults: repo = this checkout, target = $CARGO_TARGET_DIR or <repo>/target, limit 600 s, log <repo>/../suite.log
  NF_ISO must point at the disc image (as for `cargo test`). Start it in the background early in a session:
    nohup python3 tools/test/suite.py > suite.out 2>&1 &
  then read the last line of suite.out ("N passed, N failed, N ignored") and the FAILED/TIMEOUT/CRASH lines above it.
"""
import json, os, re, subprocess, sys, time

here = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
pos = [a for a in sys.argv[1:] if not a.startswith("--")]
flag_vals = {sys.argv[i + 1] for i, a in enumerate(sys.argv[1:-1], 1) if a.startswith("--")}
pos = [a for a in pos if a not in flag_vals]
repo = pos[0] if pos else here
target = pos[1] if len(pos) > 1 else os.environ.get("CARGO_TARGET_DIR", os.path.join(repo, "target"))
args = sys.argv[1:]
LIMIT = int(args[args.index("--limit") + 1]) if "--limit" in args else 600
ONLY = set(args[args.index("--only") + 1].split(",")) if "--only" in args else None
LOG = args[args.index("--log") + 1] if "--log" in args else os.path.join(repo, "..", "suite.log")

env = dict(os.environ)
env["CARGO_TARGET_DIR"] = target
if not env.get("NF_ISO"):
    print("warning: NF_ISO is not set; the ISO-reading tests will skip", flush=True)

t0 = time.time()
build = subprocess.run(
    ["cargo", "test", "--offline", "--release", "-j4", "--no-run", "--message-format=json",
     "-p", "nf-game", "-p", "nf-formats", "-p", "nf-audio", "-p", "nf-view"],
    cwd=repo, env=env, capture_output=True, text=True, encoding="utf-8", errors="replace")
bins = []
for line in build.stdout.splitlines():
    try:
        m = json.loads(line)
    except ValueError:
        continue
    if m.get("reason") == "compiler-artifact" and m.get("executable") and m.get("profile", {}).get("test"):
        name = m["target"]["name"]
        pkg_dir = os.path.dirname(m["manifest_path"])
        bins.append((name, m["executable"], pkg_dir))
if build.returncode != 0:
    errs = [l for l in build.stderr.splitlines() if l.startswith("error")]
    print("BUILD FAILED:\n" + "\n".join(errs[:20]))
    sys.exit(2)
print(f"build ok: {len(bins)} test binaries in {int(time.time() - t0)} s", flush=True)

tot = {"passed": 0, "failed": 0, "ignored": 0}
bad = []
with open(LOG, "w", encoding="utf-8") as log:
    for name, exe, pkg_dir in bins:
        if ONLY and name not in ONLY:
            continue
        e = dict(env)
        e["CARGO_MANIFEST_DIR"] = pkg_dir
        s = time.time()
        try:
            r = subprocess.run([exe, "--test-threads=1"], cwd=pkg_dir, env=e, capture_output=True, text=True,
                               encoding="utf-8", errors="replace", timeout=LIMIT)
            out = r.stdout + r.stderr
            status = r.returncode
        except subprocess.TimeoutExpired as te:
            out = (te.stdout or "") if isinstance(te.stdout, str) else (te.stdout or b"").decode("utf-8", "replace")
            running = re.findall(r"^test (\S+) \.\.\. ?$", out, re.M)
            bad.append(f"TIMEOUT {name} after {LIMIT} s (last started: {running[-1] if running else '?'})")
            log.write(f"===== {name} TIMEOUT\n{out}\n")
            tot["failed"] += 1
            print(bad[-1], flush=True)
            continue
        log.write(f"===== {name} ({int(time.time() - s)} s)\n{out}\n")
        res = re.findall(r"test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored", out)
        for p, f, i in res:
            tot["passed"] += int(p); tot["failed"] += int(f); tot["ignored"] += int(i)
        for t in re.findall(r"^test (\S+) \.\.\. FAILED", out, re.M):
            bad.append(f"FAILED {name}::{t}")
            print(bad[-1], flush=True)
        if status != 0 and not res:
            bad.append(f"CRASH {name} (exit {status})")
            tot["failed"] += 1
            print(bad[-1], flush=True)
print(f"{tot['passed']} passed, {tot['failed']} failed, {tot['ignored']} ignored  ({int((time.time() - t0) / 60)} min)")
