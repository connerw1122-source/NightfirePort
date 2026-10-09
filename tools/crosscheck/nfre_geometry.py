"""Cross-check of static PS2_GFX geometry against nightfire-re's mesh exporter (no licence: we only RUN it).

Their `parse_mesh.interpret_ps2gfx` (driven by `parse_map.parse_maps`) walks each glist box's VIF data with their own
unpacker and writes `<map path>/temp/<entity hash>_<name>.obj` with `v x y z 1.0`, `vt u 1-v` and strip triangles with
their own winding. Their texture writer is stubbed (already checked by nfre_textures.py).

Comparison per object (key: map-entry hashcode, entity hashcode, entity name), as multisets so order, winding and
vertex duplication cannot matter:
  * triangles: each as a sorted triple of positions, positions as f32 bit patterns (both sides print round-trip
    decimal: Python repr of the f32-as-double on theirs, Rust shortest f32 on ours);
  * per triangle corner, (position bits, uv) — our f32 uv gets their (u, 1 - v) transform in double and the doubles
    are compared exactly (their printed repr round-trips; inverting their flip is not exact for tiny v).
Our side: `nf-extract gfx-export` (all archives) + `bin-list` TSV for entry names.

Usage: work/third_party/.venv-nfre/Scripts/python -I tools/crosscheck/nfre_geometry.py <nightfire-re dir>
         <their ps2_archives_extracted dir> <our gfx dir> <level_bin_entries.tsv>
"""
import collections
import os
import re
import struct
import sys


def f32bits(s):
    return struct.unpack("<I", struct.pack("<f", float(s)))[0]


def read_obj(path, flip_v):
    vs, vts, tris, corners = [], [], collections.Counter(), collections.Counter()
    for line in open(path):
        p = line.split()
        if not p:
            continue
        if p[0] == "v":
            vs.append(tuple(f32bits(x) for x in p[1:4]))
        elif p[0] == "vt":
            # Theirs prints (u, 1 - v) computed in double from the f32 v; that flip is not invertible for tiny v
            # (1 - 1.7e-16 rounds), so ours gets the same forward transform instead and doubles are compared exactly.
            if flip_v:
                uv = (float(p[1]), float(p[2]))
            else:   # ours prints the shortest f32 decimal: recover the f32 exactly, widen, then apply their flip
                u, v = (struct.unpack("<f", struct.pack("<f", float(x)))[0] for x in p[1:3])
                uv = (u, 1.0 - v)
            # Some vertices have NaN UVs on the disc (UNKNOWNS U-022); their text prints "nan", so NaN == NaN here.
            vts.append(tuple("nan" if x != x else x for x in uv))
        elif p[0] == "f":
            idx = [tuple(int(x) - 1 for x in c.split("/")[:2]) for c in p[1:4]]
            tris[tuple(sorted(vs[i] for i, _ in idx))] += 1
            for i, t in idx:
                corners[(vs[i], vts[t])] += 1
    return tris, corners


def main(nfre_dir, extracted, our_gfx, bin_tsv):
    sys.path.insert(0, os.path.abspath(nfre_dir))
    from common.parser import parse_map  # noqa: E402  (their code)
    parse_map.util.framesToFile = lambda *a, **k: None
    converted = os.path.abspath(os.path.join(extracted, "..", "ps2_converted"))
    if os.environ.get("NFRE_REUSE") == "1" and os.path.isdir(converted):
        print("reusing existing", converted)       # their export takes ~1 h; rerun only when asked
    else:
        parse_map.parse_maps(extracted)

    from common import external_knowledge  # noqa: E402
    name_to_hash = {v.replace("/", os.sep).lower(): k for k, v in external_knowledge.archive_names.items()}
    theirs = {}
    for dp, _d, _fs in os.walk(converted):
        if os.path.basename(dp) != "temp":
            continue
        rel = os.path.relpath(os.path.dirname(dp), converted).lower()
        base = os.path.basename(os.path.dirname(dp))
        mh = name_to_hash.get(rel) or (base[len("unknown_"):] if base.startswith("unknown_") else None)
        if not mh:
            continue
        # A "/" in an entity name became a subfolder on their side ("…(MP5 " / " Storm).obj"): rebuild from the path.
        for sub, _d2, fs in os.walk(dp):
            for f in fs:
                if f.endswith(".obj"):
                    ent = os.path.relpath(os.path.join(sub, f), dp)[:-4].replace(os.sep, "/")
                    theirs[(mh.lower(), ent[:8].lower(), ent[9:])] = os.path.join(sub, f)

    names = {}
    for line in open(bin_tsv).read().splitlines()[1:]:
        a, i, _t, n, *_ = line.split("\t")
        names[(a, int(i))] = n.lower()
    stats = collections.Counter()
    bad = []
    seen = set()
    for arch in sorted(d for d in os.listdir(our_gfx) if os.path.isdir(os.path.join(our_gfx, d))):
        for f in sorted(x for x in os.listdir(os.path.join(our_gfx, arch)) if x.endswith(".obj")):
            path = os.path.join(our_gfx, arch, f)
            # "# <archive> entry <n> object <k> hashcode <h> <name...>": keep the name's exact spacing (some end in " ").
            head = open(path).readline().rstrip("\n").split(" ", 8)
            entry, hc = int(head[3]), head[7].lower()
            name = head[8] if len(head) > 8 else ""
            key = (names[(arch + ".bin", entry)], hc, name)
            if key in seen:
                stats["duplicate key (same map entry in several archives)"] += 1
                continue
            seen.add(key)
            if key not in theirs:   # their writer turns "/" in a name into "__"
                key = (key[0], key[1], key[2].replace("/", "__"))
            if key not in theirs:
                stats["no OBJ on their side"] += 1
                if len(bad) < 5:
                    bad.append(("missing", arch, f, key))
                continue
            o_tri, o_cor = read_obj(path, False)
            t_tri, t_cor = read_obj(theirs[key], True)
            if o_tri == t_tri and o_cor == t_cor:
                stats["identical (triangles + uv corners)"] += 1
            else:
                stats["triangles differ" if o_tri != t_tri else "uv corners differ"] += 1
                if len(bad) < 10:
                    bad.append(("diff", arch, f, key, sum(o_tri.values()), sum(t_tri.values()),
                                sum((o_tri - t_tri).values()), sum((t_tri - o_tri).values())))
    print(dict(stats))
    for b in bad:
        print(b)


if __name__ == "__main__":
    main(*sys.argv[1:5])
