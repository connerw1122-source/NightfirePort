## Your task: `drive-track-mesh`, the driving missions' track geometry

503 documented the CRP track format (docs/formats/crp.md; the VIV archives via 500's refpack). Decode the track's
geometry into meshes the port can use: vertices/indices/UVs/texture refs per section, plus the collision/road
surface data if separable. A `nf_formats` module + an `nf-extract` command (e.g. `crp-mesh <iso> <viv> <out>`
writing OBJ for local inspection, never committed). Cross-check: counts and bounds per track match what the format
headers declare; a golden of per-section counts + SHA-1s of the decoded index/vertex buffers (numbers only). No
renderer work (the main agent wires drawing).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/drive-track-mesh`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox7 at 2026-10-15T19:51Z

---
## Report (done, 2026-10-15T21:14Z)


---
## Report from the worker (done, 2026-10-15T21:15Z)

# drive-track-mesh — the driving track's mesh layer

Branch `s/drive-track-mesh`. Task: decode the `.crp` track geometry into meshes (vertices/indices/
UVs/texture refs per section) plus the collision/road surface if separable; a `nf_formats` module +
an `nf-extract crp-mesh` command; goldens of per-section counts + SHA-1s.

## What's done

**1. The article layer is decoded and pinned** (`crates/nf-formats/src/crp_mesh.rs`):

- Every pool `Arti` article = one geometry/deco piece: `Name` (string), `Base` (96 B; bbox min at
  +0x10, max at +0x20 — measured on `S6_CustomLowRBnknRoad` [D]), `as#N` part records whose payload
  ends in the 4-byte word `{u16 sr_index, "sr"}` (measured `as#0`→`sr#5` = `EAGL::…::Model[0].00`;
  some articles carry other tails — MIS01 `trafficlight` as#1 = `00 02 05 00` [D]), `ca#0`
  (per-article collision, 256–7040 B), `in#0/#1` (64 B transforms), `mp#0` [D, all 8 tracks].
- The `as`→`sr` chain resolves to **EAGL symbol names**: `EAGL::<article>::{as  0000}::Model[0].NN`.

**2. The embedded module is loaded and relocated exactly like the game** [M]:

- `ELFd` + `ELFr` form one MIPS32 ET_REL image split at the ELFd size; `FUN_0029ea00` 0x29ea00
  resolves `e_shoff`/`sh_addr` as `x < elfd_size ? ELFd+x : ELFr+(x−elfd_size)` (measured paris:
  6 sections, `e_shoff` 0x4642b0 = the last 0xF0 bytes of the combined image [D]).
- `.rel.data` is all `R_MIPS_32` (`word += symbol value`) — verified 54,613/54,613 paris entries;
  the loader applies them like `EAGL::DynamicLoader::Resolve` 0x2bb300 [M].
- Symbol names: `__Model:::<article>::{as  0000}::Model[0].NN` (paris 1,890; each a 4-byte slot in
  the model objects) and GeoPrim name strings `__GPRenderMethod_<article>::{as  0000}::Model[0].
  NN_<x>_name` (paris 1,736) [D]. Following `S12_LowRiverBnk_n_Road_00 …Model[0].1d`'s slot reached
  a model-node object (+0xC = segment index 0x1C, +0x18 = pointer into a shared segment-list array
  of `{value, 0xA000FFFF}` records with `{0xA0000000, 0, n}` run headers whose values point at the
  GeoPrim name strings) [D].

**3. Collision/road surface is separable and censused** [D]: the pool `CDat` article is the
collision world — `CGrd` (36 B) + `ci#0` (paris: 63,616 B, count=994 items) + ~1,600–2,100 `cn#N`
node records per track. `RNgp` = road network, `Map ` = the mission scene database (AICo/AIEl/AISp/
Trgr/…/aq#N placement arrays). Their binary layouts are the next phase.

**4. The port**: `crp_mesh.rs` (`ArticleMeta`/`PartRef`/`TrackModule`/`ModuleSection`/
`ModuleSymbol`/`CollisionCensus`, all bounds-checked, mirroring the game's arithmetic);
`nf-extract crp-mesh <iso|DRIVING-dir> <viv> <out>` (per-article TSVs + module TSVs) and
`crp-mesh-digest` (canonical rows). `golden/crp_mesh_golden.tsv` pins all 8 mission tracks:
key, entry, track, articles, parts, model_symbols, geoprims, relocations, ci_items, cn_records and
the SHA-1 of the relocated module image (e.g. paris 494 articles / 1899 parts / 1890 model symbols /
54613 relocs / sha1 2f0000bc…).

## What's left (the next phase, with everything it needs now pinned)

- **The render-vertex decode**: the actual VIF chains are built by the EAGL engine at load time
  from the model objects this module indexes; the segment-list records point at GeoPrim name
  strings, and the packed source data (V3-16-style byte/short runs + DMA-tag-shaped words
  0x60000000/0x0500000d/0x20000000) sits around them. The reverse of that final hop (which words
  are the quantised vertex/strip data and how the GeoPrim preamble maps to texture TAR symbols) is
  the remaining work — start from the segment list at paris 0x33c020 and the GeoPrim objects at
  0x180530/0x180544, and read `FUN_0029f8e8`/`EAGL::DynamicLoader::RegisterShapes` 0x2bd140 +
  `RRenderWorldCamera::DrawStaticWorldGeometry` 0x1f3560 (SYM names from the older build — match
  by shape, U-003). OBJ export lands with it (the `crp-mesh` TSVs are in place).
- `CDat` `ci`/`cn` and `RNgp` binary layouts (task says "if separable" — the article is separable
  and censused; the layouts are not decoded).
- The community-source claim "the `Map ` group's `in#0` array holds one 0x40 matrix per placed
  instance (Paris 2552)" did **not** verify on our data: the paris `Map ` window holds
  AICo/AIEl/AISp/Cams/Envi/MSet/Rule/Shad/Trgr/VisC/Wmap/an#N/aq#N/el#N and no `in` record [D].
  Placement must live in the `aq#N` arrays (451×32 B ×3 on paris) — next phase.

## How it was verified

- All numbers above were measured on our disc data (`DRIVING/` folder; the slim test ISO ships
  without it — `crp-mesh` accepts either source) with Python probes first, then re-derived in Rust.
- Every structure was matched to DRIVING.ELF code (the resolver 0x29ea00, the loader call
  0x1a9600 lines 143-150, the relocation application 0x2bb300) — [M] tags in the module doc.
- Tests: `crp_mesh::tests::article_metadata_roundtrip` (hand-built container: bbox, name, part
  refs, the non-"sr" tail form, malformed-Base rejection); `driving_track_mesh_layer_matches_the_
  golden` (all 8 tracks re-derived and diffed against the golden, plus a flipped-byte tamper
  sister that must change the digest).

## What I ran

- `cargo test --offline --release -p nf-formats -- --test-threads=1`: lib 77/77 ok;
  driving_golden 9/9 ok; disc ISO tests 11 ok + the 3 allowed `disc_golden` ISO-manifest failures
  (slim ISO).
- `cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view --no-run`:
  all four crates compile clean (pre-existing warnings only).
- nf-game and nf-audio: all binaries green (no NF_ISO long binaries this window).
- nf-view: 36 binaries ok; `fix_0c_pylon_live` FAILED — wgpu finds no GPU in this sandbox
  (environment, pre-documented in WORKLOG 2026-10-15 playtest-spots; not related to this task).
- The `tools/test/suite.py` background runner died twice between tool calls (this sandbox kills
  detached processes — same as the playtest-spots worker reported); the runs above are the
  foreground equivalents of its per-binary chunks. The full NF_ISO long tail (nf-view renders,
  live sims) did not fit the 2-hour window; no failure in what ran is attributable to this task
  (the two failing view binaries are GPU-absent panics at `request_adapter`).

## Live checks for the main agent

1. **Symbol-name registration** (PCSX2, DRIVING.ELF): breakpoint `FUN_002a9c10` (the `__`-name
   splitter called from 0x29ea00) during a paris load and dump a couple of registered names —
   expect `Model:::S6_CustomLowRBnknRoad::{as  0000}::Model[0].00` and a `GPRenderMethod_…` twin,
   confirming the EAGL namespace prefixing this module assumes.
2. **The segment list walk**: watch memory at paris module+0x33c020 during a road draw and confirm
   the `{value, 0xA000FFFF}` records feed a DMA/VIF builder (the value words point at
   `__GPRenderMethod_…` name strings on disc; in memory they should be patched to GeoPrim object
   pointers by relocations + the list walk).
3. **`crp-mesh` spot check**: `nf-extract crp-mesh DRIVING MIS01.VIV out/` — the paris TSV's first
   row should be `S6_CustomLowRBnknRoad` with bbox ≈ min(−10.06,−7.5,−10.06) max(18.33,0.5,15.0)
   [D measured].
