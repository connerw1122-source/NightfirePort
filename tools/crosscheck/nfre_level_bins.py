"""Independent cross-check of the level .bin reader with nightfire-re's extract_bin (no licence: we only RUN it).

Runs their `extract_bin.extract_all` on the 118 files their FILES.BIN step produced (nfre_files_bin.py must have run
first), then hashes what it wrote. Their outputs are `<archive>_extract/<name>_<type hex>.bin`; equal (name, type)
pairs in one archive overwrite each other (the last one wins) and empty names become `unknown_at_offset_<n>`.
So the comparison is per archive on the set of (name, type, sha1), with our side reduced the same way: last
occurrence per (name, type), empty names compared as a multiset of (type, sha1).

Usage: work/third_party/.venv-nfre/Scripts/python -I tools/crosscheck/nfre_level_bins.py \
         <nightfire-re dir> <their ps2_archives_extracted dir> <our level_bin_entries.tsv>
"""
import collections
import hashlib
import os
import sys


def main(nfre_dir, extracted, ours_tsv):
    sys.path.insert(0, os.path.abspath(nfre_dir))
    from platform_ps2 import extract_bin  # noqa: E402  (their code)
    extract_bin.extract_all(extracted)

    theirs = collections.defaultdict(set)
    their_unnamed = collections.defaultdict(collections.Counter)
    for d in os.listdir(extracted):
        if not d.endswith("_extract"):
            continue
        # Folders are "<file>_extract" where <file> is "07F00029.bin" or a renamed "07000001_HT_Level_X.bin".
        archive = d[:8] + ".bin"
        for fn in os.listdir(os.path.join(extracted, d)):
            stem, typ = fn[: -len(".bin")].rsplit("_", 1)
            h = hashlib.sha1(open(os.path.join(extracted, d, fn), "rb").read()).hexdigest()
            if stem.startswith("unknown_at_offset_"):
                their_unnamed[archive][(int(typ, 16), h)] += 1
            else:
                theirs[archive].add((stem.lower(), int(typ, 16), h))

    ours = collections.defaultdict(dict)
    our_unnamed = collections.defaultdict(collections.Counter)
    dup = 0
    for line in open(ours_tsv).read().splitlines()[1:]:
        a, _i, t, name, _o, _l, h = line.split("\t")
        if name == "":
            our_unnamed[a][(int(t), h)] += 1
            continue
        key = (name.lower(), int(t))
        if key in ours[a]:
            dup += 1
        ours[a][key] = h                                   # last occurrence wins, like their overwrite
    diffs = 0
    for a in sorted(set(ours) | set(theirs)):
        o = {(n, t, h) for (n, t), h in ours[a].items()}
        if o != theirs[a]:
            diffs += 1
            print(f"{a}: ours-only {sorted(o - theirs[a])[:3]} theirs-only {sorted(theirs[a] - o)[:3]}")
        if our_unnamed[a] != their_unnamed[a]:
            diffs += 1
            print(f"{a}: unnamed entries differ")
    print(f"archives compared: {len(set(ours) | set(theirs))}; named entries ours {sum(len(v) for v in ours.values())} "
          f"(after collapsing {dup} duplicate name+type), unnamed {sum(sum(c.values()) for c in our_unnamed.values())}; "
          f"archives with differences: {diffs}")


if __name__ == "__main__":
    main(*sys.argv[1:4])
