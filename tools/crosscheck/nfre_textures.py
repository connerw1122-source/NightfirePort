"""Pixel-exact cross-check of map textures against nightfire-re's PS2 texture export (no licence: we only RUN it).

Their `parse_map.parse_maps` walks each map entry's block table with their own handlers and writes every texture with
their own depalettizer (`ps2_depalettize`) to `ps2_converted/<map path>/<index or texture hashcode>.png`. Mesh export is
stubbed out (not part of this check). Animated textures are written as WebP, which PIL saves lossy by default, so
only single-frame textures (PNG) are compared; the rest are counted as not checked.

Comparison per texture: RGB exact; alpha compared on the PS2 scale (theirs `floor(a*255/128)`, ours
`round(a*255/128)` — both invertible for a in 0..=128). Our side is `nf-extract tex-export` output plus the TSVs from
`bin-list` and `tex-list`.

Usage: work/third_party/.venv-nfre/Scripts/python -I tools/crosscheck/nfre_textures.py <nightfire-re dir>
         <their ps2_archives_extracted dir> <our tex dir> <level_bin_entries.tsv> <texture_entries.tsv>
"""
import collections
import os
import sys

from PIL import Image


def main(nfre_dir, extracted, our_tex, bin_tsv, tex_tsv):
    sys.path.insert(0, os.path.abspath(nfre_dir))
    from common.parser import parse_map, parse_mesh  # noqa: E402  (their code)
    from common import external_knowledge  # noqa: E402
    parse_mesh.interpret_ps2gfx = lambda *a, **k: None
    parse_mesh.generate_materials = lambda *a, **k: None

    # Their decoder is untouched; only the writer changes: animated textures become one lossless PNG per frame
    # (`<name>_fNN.png`) instead of a lossy WebP.
    def frames_to_png(frames, filename):
        if frames is None:
            return
        if len(frames) == 1:
            frames[0].save(filename + ".png", "PNG")
        else:
            for n, fr in enumerate(frames):
                fr.save(f"{filename}_f{n:02}.png", "PNG")
    parse_map.util.framesToFile = frames_to_png
    parse_map.parse_maps(extracted)

    # Their output: map path -> files. Reverse the path naming to the map hashcode.
    converted = os.path.abspath(os.path.join(extracted, "..", "ps2_converted"))
    # Case-insensitive: their table spells one folder both "Weapons/" and "weapons/", which share one folder on NTFS.
    name_to_hash = {v.replace("/", os.sep).lower(): k for k, v in external_knowledge.archive_names.items()}
    theirs = {}
    for dp, _d, fs in os.walk(converted):
        rel = os.path.relpath(dp, converted).lower()
        base = os.path.basename(dp)
        h = name_to_hash.get(rel) or (base[len("unknown_"):] if base.startswith("unknown_") else None)
        if h is None:
            continue
        for f in fs:
            if f.endswith(".png"):
                theirs[(h.lower(), f[:-4].lower())] = os.path.join(dp, f)

    names = {}
    for line in open(bin_tsv).read().splitlines()[1:]:
        a, i, _t, n, *_ = line.split("\t")
        names[(a, int(i))] = n.lower()
    inv_round = {round(a * 255 / 128): a for a in range(129)}
    inv_round[255] = 128
    inv_floor = {int(a * (255 / 0x80)): a for a in range(129)}
    stats = collections.Counter()
    bad = []
    for line in open(tex_tsv).read().splitlines()[1:]:
        c = line.split("\t")
        a, e, k = c[0], int(c[1]), int(c[2])
        if c[9] == "short":
            stats["short (not decoded by us)"] += 1
            continue
        frames = int(c[6])
        mh = names[(a, e)]
        hc = c[8].lower()
        stem = hc if hc != "ffffffff" else str(k)
        kind = "animated" if frames > 1 else "single"
        for f in range(frames):
            key = (mh, stem if frames == 1 else f"{stem}_f{f:02}")
            if key not in theirs:
                stats[f"{kind}: missing on their side"] += 1
                bad.append(("missing", a, e, k, key))
                continue
            ours_name = f"{e:04}_{k:03}.png" if frames == 1 else f"{e:04}_{k:03}_f{f:02}.png"
            ours = Image.open(os.path.join(our_tex, a[:-4], ours_name)).convert("RGBA")
            them = Image.open(theirs[key]).convert("RGBA")
            if ours.size != them.size:
                stats[f"{kind}: size differs"] += 1
                bad.append(("size", a, e, k, f, ours.size, them.size))
                continue
            o, t = ours.tobytes(), them.tobytes()
            same = o == t or all(o[i:i + 3] == t[i:i + 3] and inv_round.get(o[i + 3]) == inv_floor.get(t[i + 3])
                                 for i in range(0, len(o), 4))
            stats[f"{kind}: frames pixel-exact" if same else f"{kind}: frames differ"] += 1
            if not same:
                bad.append(("pixels", a, e, k, f, key))
    print(dict(stats))
    for b in bad[:10]:
        print(b)


if __name__ == "__main__":
    main(*sys.argv[1:6])
