//! Differential call tests: the real game's calls, recorded in PCSX2 (`tools/trace/gdbcall.py`, format v1 —
//! docs/tools/call-traces.md), replayed through the port's version of each function and required to match
//! **bit for bit**. The recordings live in `tests/calls/<function>.calls` at the repo root (runtime register
//! and memory values only — no disc bytes).
//!
//! A function with a registered adapter must match every recorded call; a function without one is reported as
//! `not covered` (the recording requests table in docs/reports/verify-bitexact.md says what to record next).
//! The sister test proves the comparison catches a one-bit change in a recorded output.

use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Format v1 (docs/tools/call-traces.md)
// ---------------------------------------------------------------------------

/// One recorded memory region: the spec expression, its resolved address, the bytes.
#[derive(Debug, Clone)]
struct Region {
    expr: String,
    addr: u32,
    bytes: Vec<u8>,
}

/// One recorded call: every register at entry, the `in` regions, the return registers, the `out` regions.
#[derive(Debug, Clone)]
struct CallRecord {
    function: String,
    addr: u32,
    level: String,
    n: u32,
    frame: u32,
    /// Register order: zero at v0 v1 a0..a3 t0..t7 s0..s7 t8 t9 k0 k1 gp sp fp ra.
    gpr: [u32; 32],
    /// IEEE bit patterns of f0..f31 at entry.
    fpr: [u32; 32],
    ins: Vec<Region>,
    /// Return registers (v0, v1, f0 bit patterns) read at the matched `$ra`.
    ret: [u32; 3],
    outs: Vec<Region>,
}

fn hex_u32(s: &str) -> Result<u32, String> {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).map_err(|e| format!("bad hex {s:?}: {e}"))
}

fn parse_calls(text: &str) -> Result<(String, u32, String, Vec<CallRecord>), String> {
    let mut function = String::new();
    let mut addr = 0u32;
    let mut level = String::new();
    let mut calls = Vec::new();
    let mut cur: Option<CallRecord> = None;
    for (ln, line) in text.lines().enumerate() {
        let bad = |msg: &str| Err(format!("line {}: {msg} ({line:?})", ln + 1));
        if let Some(rest) = line.strip_prefix("# gdbcall v1 ") {
            for field in rest.split_whitespace() {
                if let Some(f) = field.strip_prefix("function=") { function = f.to_string(); }
                else if let Some(a) = field.strip_prefix("addr=0x") { addr = hex_u32(a)?; }
                else if let Some(l) = field.strip_prefix("level=") { level = l.to_string(); }
            }
        } else if let Some(rest) = line.strip_prefix("call ") {
            if cur.is_some() { return bad("new call before the previous one ended (no blank line)"); }
            let t: Vec<&str> = rest.split_whitespace().collect();
            // `call <n> frame <GameState+0x34> ra <hex> sp <hex>`
            if t.len() != 7 || t[1] != "frame" || t[3] != "ra" || t[5] != "sp" { return bad("bad call header"); }
            cur = Some(CallRecord {
                function: function.clone(), addr, level: level.clone(),
                n: t[0].parse().map_err(|e| format!("line {ln}: bad call number: {e}"))?,
                frame: t[2].parse().map_err(|e| format!("line {ln}: bad frame: {e}"))?,
                gpr: [0; 32], fpr: [0; 32], ins: vec![], ret: [0; 3], outs: vec![],
            });
        } else if line == "gpr" || line == "fpr" {
            return bad("register values must follow the keyword on one line");
        } else if let Some(rest) = line.strip_prefix("gpr ") {
            let cur = cur.as_mut().ok_or("gpr outside a call")?;
            let t: Vec<&str> = rest.split_whitespace().collect();
            if t.len() != 32 { return bad(&format!("expected 32 gpr values, got {}", t.len())); }
            for (k, v) in t.iter().enumerate() { cur.gpr[k] = hex_u32(v)?; }
        } else if let Some(rest) = line.strip_prefix("fpr ") {
            let cur = cur.as_mut().ok_or("fpr outside a call")?;
            let t: Vec<&str> = rest.split_whitespace().collect();
            if t.len() != 32 { return bad(&format!("expected 32 fpr values, got {}", t.len())); }
            for (k, v) in t.iter().enumerate() { cur.fpr[k] = hex_u32(v)?; }
        } else if let Some(rest) = line.strip_prefix("in ") {
            let cur = cur.as_mut().ok_or("in outside a call")?;
            cur.ins.push(parse_region(rest)?);
        } else if let Some(rest) = line.strip_prefix("out ") {
            let cur = cur.as_mut().ok_or("out outside a call")?;
            cur.outs.push(parse_region(rest)?);
        } else if let Some(rest) = line.strip_prefix("ret ") {
            let cur = cur.as_mut().ok_or("ret outside a call")?;
            let t: Vec<&str> = rest.split_whitespace().collect();
            // `ret v0 <hex> v1 <hex> f0 <hex>`
            if t.len() != 6 || t[0] != "v0" || t[2] != "v1" || t[4] != "f0" { return bad("bad ret line"); }
            cur.ret = [hex_u32(t[1])?, hex_u32(t[3])?, hex_u32(t[5])?];
        } else if line.is_empty() {
            if let Some(c) = cur.take() { calls.push(c); }
        }
    }
    if cur.is_some() { return Err("last call has no trailing blank line".into()); }
    Ok((function, addr, level, calls))
}

/// `in <expr> <address> <length hex> <bytes hex>` — the length is hex without a prefix.
fn parse_region(rest: &str) -> Result<Region, String> {
    let t: Vec<&str> = rest.split_whitespace().collect();
    if t.len() != 4 { return Err(format!("bad region line: {rest:?}")); }
    let len = usize::from_str_radix(t[2].trim_start_matches("0x"), 16).map_err(|e| format!("bad length {}: {e}", t[2]))?;
    let bytes = (0..len)
        .map(|k| u8::from_str_radix(&t[3][2 * k..2 * k + 2], 16))
        .collect::<Result<Vec<u8>, _>>()
        .map_err(|e| format!("bad bytes in region {}: {e}", t[0]))?;
    if t[3].len() != 2 * len { return Err(format!("region {}: {} byte chars for length {}", t[0], t[3].len(), len)); }
    Ok(Region { expr: t[0].to_string(), addr: hex_u32(t[1])?, bytes })
}

// ---------------------------------------------------------------------------
// Port adapters
// ---------------------------------------------------------------------------

/// What the port's version of a function produced. `None` = "the port does not reproduce this register"
/// (scratch values like PS2Sinf's `v0` are caller-visible but not part of the function's contract).
/// Out regions carry the computed bytes and a mask of the bytes the game function actually writes
/// (e.g. `Quat_QuatToMat` writes 9 of the 16 matrix floats; the rest belong to the caller).
#[derive(Debug, Default)]
struct PortResult {
    v0: Option<u32>,
    v1: Option<u32>,
    f0: Option<u32>,
    outs: Vec<(Vec<u8>, Vec<bool>)>,
}

type Adapter = fn(&CallRecord) -> Result<PortResult, String>;

fn u32_at(bytes: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(bytes[off..off + 4].try_into().unwrap())
}

fn adapters() -> Vec<(&'static str, Adapter)> {
    vec![
        ("PS2Sinf", ps2sinf),
        ("Rand_Rand", rand_rand),
        ("Quat_QuatToMat", quat_quat_to_mat),
    ]
}

/// `PS2Sinf` (0x120700): registers only — the angle in `f12`, the sine in `f0`. Port: `trig::sinf`
/// (the game's range reduction + VU0 polynomial, validated against the interpreter over ACTION.ELF).
fn ps2sinf(r: &CallRecord) -> Result<PortResult, String> {
    let x = f32::from_bits(r.fpr[12]);
    Ok(PortResult { f0: Some(nf_game::trig::sinf(x).to_bits()), ..Default::default() })
}

/// `Rand_Rand` (0x1e3738): `n` in `a0`, the shared generator state `SEED_X`/`SEED_Y` (0x30d0a0/0x30d0a4) in the
/// io regions, stepped y-then-x, return `((SEED_Y' & 0xffff) × n) >> 16` in `v0`, the new state in the io regions.
/// Port: `drone::Rng::rand` (WORKLOG 0025).
fn rand_rand(r: &CallRecord) -> Result<PortResult, String> {
    let n = r.gpr[4]; // a0
    let seed = |a: u32| -> Result<u32, String> {
        r.ins.iter().find(|g| g.addr == a).map(|g| u32_at(&g.bytes, 0)).ok_or(format!("no in region for seed {a:#x}"))
    };
    let mut rng = nf_game::drone::Rng { x: seed(0x30d0_a0)?, y: seed(0x30d0_a4)? };
    let v0 = rng.rand(n);
    let outs = r.outs.iter().map(|g| {
        let v = match g.addr { 0x30d0_a0 => rng.x, 0x30d0_a4 => rng.y, a => return Err(format!("unexpected out region {a:#x}")) };
        let bytes = v.to_le_bytes().to_vec();
        let mask = vec![true; 4];
        Ok((bytes, mask))
    }).collect::<Result<Vec<_>, String>>()?;
    Ok(PortResult { v0: Some(v0), outs, ..Default::default() })
}

/// `Quat_QuatToMat` (0x1e3e80): the quaternion (x, y, z, w) at `a0` (in region), the 3×3 rotation into the
/// row-vector MATRIX at `a1` (out region, 64 bytes; only indices 0,1,2,4,5,6,8,9,10 are written — 3,7,11 are
/// the unused fourth column and 12..15 the translation the caller owns). Operation order from the decompile,
/// EE FPU arithmetic (`ps2math`): doubling adds, chopped products, no-guard adds, `1.0 −` subtracts.
fn quat_quat_to_mat(r: &CallRecord) -> Result<PortResult, String> {
    use nf_game::ps2math as pm;
    let q = r.ins.first().ok_or("no in region for the quaternion")?;
    if q.bytes.len() < 16 { return Err("quaternion region shorter than 16 bytes".into()); }
    let (x, y, z, w) = (f32::from_bits(u32_at(&q.bytes, 0)), f32::from_bits(u32_at(&q.bytes, 4)),
        f32::from_bits(u32_at(&q.bytes, 8)), f32::from_bits(u32_at(&q.bytes, 12)));
    // The decompile's temporaries, in its order: fVar1 = y+y, fVar2 = z+z, fVar5 = x, fVar3 = y, fVar4 = z,
    // fVar7 = w, fVar6 = (x+x)·x, fVar8 = (x+x)·w.
    let (y2, z2, x2) = (pm::fpu_add(y, y), pm::fpu_add(z, z), pm::fpu_add(x, x));
    let (xx, xw) = (pm::mul(x2, x), pm::mul(x2, w));
    let entry = |a: f32, b: f32| pm::mul(a, b);
    let mut m = [0u32; 16];
    m[8] = pm::fpu_add(entry(z2, x), entry(y2, w)).to_bits();                       // 2z·x + 2y·w
    m[4] = pm::fpu_sub(entry(y2, x), entry(z2, w)).to_bits();                       // 2y·x − 2z·w
    m[9] = pm::fpu_sub(entry(z2, y), xw).to_bits();                                 // 2z·y − 2x·w
    m[10] = pm::fpu_sub(1.0, pm::fpu_add(xx, entry(y2, y))).to_bits();              // 1 − (2x² + 2y·y)
    m[0] = pm::fpu_sub(1.0, pm::fpu_add(entry(y2, y), entry(z2, z))).to_bits();     // 1 − (2y·y + 2z·z)
    m[5] = pm::fpu_sub(1.0, pm::fpu_add(xx, entry(z2, z))).to_bits();               // 1 − (2x² + 2z·z)
    m[1] = pm::fpu_add(entry(y2, x), entry(z2, w)).to_bits();                       // 2y·x + 2z·w
    m[2] = pm::fpu_sub(entry(z2, x), entry(y2, w)).to_bits();                       // 2z·x − 2y·w
    m[6] = pm::fpu_add(entry(z2, y), xw).to_bits();                                 // 2z·y + 2x·w
    let written = [0usize, 1, 2, 4, 5, 6, 8, 9, 10];
    let mut bytes = Vec::with_capacity(64);
    let mut mask = Vec::with_capacity(64);
    for k in 0..16 {
        let b = if written.contains(&k) { m[k] } else { u32_at(&r.outs[0].bytes, 4 * k) };
        bytes.extend_from_slice(&b.to_le_bytes());
        mask.extend_from_slice(&[written.contains(&k); 4]);
    }
    Ok(PortResult { outs: vec![(bytes, mask)], ..Default::default() })
}

// ---------------------------------------------------------------------------
// The ledger guard (docs/verify/ledger.md)
// ---------------------------------------------------------------------------

/// One `| 0x… | Name | Port fn | Status | Recording | …` row of the ledger.
struct LedgerRow {
    line_no: usize,
    addr: String,
    name: String,
    port: String,
    status: String,
    recording: String,
}

/// A "bit-exact on N recorded calls" claim, parsed out of a row's status.
struct BitExactClaim {
    n: u32,
}

fn parse_bit_exact(status: &str) -> Option<BitExactClaim> {
    let rest = status.strip_prefix("bit-exact on ")?
        .strip_suffix(" recorded calls")?;
    let n: u32 = rest.trim().parse().ok()?;
    (n >= 1).then_some(BitExactClaim { n })
}

/// The ledger's status grammar (docs/verify/ledger.md) — everything else is a typo and must fail loudly.
fn status_uses_the_grammar(status: &str) -> bool {
    status == "unverified"
        || status.starts_with("unverified (") && status.ends_with(')')
        || status.starts_with("within tolerance (") && status.ends_with(')')
        || status.starts_with("known divergent (") && status.ends_with(')')
        || parse_bit_exact(status).is_some()
}

fn parse_ledger(text: &str) -> Vec<LedgerRow> {
    let mut rows = Vec::new();
    for (ln, line) in text.lines().enumerate() {
        let t = line.trim_start();
        // ledger rows start with `| 0x` (or `| -` for port-only); header/separator rows don't
        if !(t.starts_with("| 0x") || t.starts_with("| - ")) { continue; }
        let cells: Vec<&str> = t.split('|').collect();
        // cells[0] is empty; we need addr, name, port, status, recording = cells[1..=5]
        if cells.len() < 7 {
            panic!("ledger line {}: a row needs at least 5 columns (Address | Name | Port fn | Status | Recording): {t:?}", ln + 1);
        }
        let clean = |s: &str| s.trim().trim_matches('`').to_string();
        rows.push(LedgerRow {
            line_no: ln + 1,
            addr: clean(cells[1]),
            name: clean(cells[2]),
            port: clean(cells[3]),
            status: clean(cells[4]),
            recording: clean(cells[5]),
        });
    }
    rows
}

/// The ledger's claims must be backed: every `bit-exact on N recorded calls` row needs the recording file (with
/// ≥ N calls, the matching `function=` header and entry address) and a registered adapter; every row must use
/// the status grammar. A stale claim (the recording shrank, the adapter was removed, a typo) fails here.
#[test]
fn ledger_claims_match_the_recordings() {
    let ledger = Path::new(&env!("CARGO_MANIFEST_DIR")).join("../../docs/verify/ledger.md");
    let text = fs::read_to_string(&ledger)
        .unwrap_or_else(|e| panic!("read {}: {e}", ledger.display()));
    let rows = parse_ledger(&text);
    assert!(rows.len() >= 400, "the ledger must keep its appendix: {} rows", rows.len());

    let table = adapters();
    let mut seen_addr_name = std::collections::HashSet::new();
    let mut claims = 0;
    for row in &rows {
        assert!(status_uses_the_grammar(&row.status),
            "ledger line {}: status {:?} is not in the grammar (docs/verify/ledger.md)", row.line_no, row.status);
        let key = (row.addr.clone(), row.name.clone());
        assert!(seen_addr_name.insert(key),
            "ledger line {}: duplicate address+name {} {}", row.line_no, row.addr, row.name);
        let Some(claim) = parse_bit_exact(&row.status) else { continue };
        claims += 1;
        assert!(row.addr != "-", "ledger line {}: a bit-exact claim needs a game address", row.line_no);
        assert!(row.port.contains("::") || row.port.starts_with("(adapter in diff_calls.rs"),
            "ledger line {}: a bit-exact claim needs a greppable port fn (or the adapter marker), got {:?}",
            row.line_no, row.port);
        assert!(!row.recording.is_empty() && row.recording != "-",
            "ledger line {}: a bit-exact claim needs a recording", row.line_no);
        let file = row.recording.rsplit('/').next().unwrap();
        let path = calls_dir().join(file);
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("ledger line {}: recording {}: {e}", row.line_no, path.display()));
        let (function, addr, _, calls) = parse_calls(&text)
            .unwrap_or_else(|e| panic!("ledger line {}: {}: {e}", row.line_no, path.display()));
        assert_eq!(function, row.name,
            "ledger line {}: the recording's function= header is {function:?}, the row says {:?}", row.line_no, row.name);
        assert_eq!(format!("{:#x}", addr), format!("{:#x}", u32::from_str_radix(row.addr.trim_start_matches("0x"), 16)
            .unwrap_or_else(|e| panic!("ledger line {}: bad address {:?}: {e}", row.line_no, row.addr))),
            "ledger line {}: the recording's entry address disagrees with the row", row.line_no);
        assert!(calls.len() as u32 >= claim.n,
            "ledger line {}: claims {} recorded calls but {} holds {}", row.line_no, claim.n, path.display(), calls.len());
        assert!(table.iter().any(|(n, _)| *n == row.name),
            "ledger line {}: {function} has no adapter registered in diff_calls.rs::adapters()", row.line_no);
    }
    assert!(claims >= 3, "the ledger's verified rows went missing: {claims} bit-exact claims");
}

/// Sister: a row that claims bit-exact without the evidence behind it must fail the guard. The forged row is
/// parsed with the real parser; today `Quat_Slerp_Acc` has neither a recording nor an adapter, so the guard
/// fails it on the missing file — and if a later task records one, the missing adapter still fails it.
#[test]
fn a_bit_exact_claim_without_a_recording_fails_the_ledger_guard() {
    let forged = "| 0x1e4188 | Quat_Slerp_Acc | `anim::slerp` | bit-exact on 40 recorded calls | `Quat_Slerp_Acc.calls` |\n";
    let rows = parse_ledger(forged);
    assert_eq!(rows.len(), 1);
    let claim = parse_bit_exact(&rows[0].status).expect("the forged status must parse as bit-exact");
    let path = calls_dir().join(format!("{}.calls", rows[0].name));
    let backed_by_file = fs::read_to_string(&path).is_ok();
    let backed_by_adapter = adapters().iter().any(|(n, _)| *n == rows[0].name);
    assert!(!(backed_by_file && backed_by_adapter),
        "the forged row must have no full backing (recording + adapter) for this sister to hold — \
         if it now has both, forge a function that has neither");
    if !backed_by_file {
        assert!(!path.exists(), "Quat_Slerp_Acc has no recording yet — the guard must fail on the forged row");
    } else {
        assert!(!backed_by_adapter, "the guard must fail the forged row on the missing adapter (claim {} calls)", claim.n);
    }
    // and a typo'd status must fail the grammar, so a stale claim can't survive as prose
    assert!(!status_uses_the_grammar("bit-exact 40 calls"));
    assert!(!status_uses_the_grammar("bit-exact on 40 calls"));
    assert!(!status_uses_the_grammar("verified"));
    assert!(status_uses_the_grammar("unverified (recording exists; no adapter)"));
    assert!(status_uses_the_grammar("known divergent (GAPS G-017)"));
    assert!(status_uses_the_grammar("within tolerance (1e-5)"));
}

/// Distance in ulps between two f32 bit patterns (NaNs compare unequal unless the bits match).
fn f32_ulp(a: u32, b: u32) -> u64 {
    let ordered = |x: u32| if x & 0x8000_0000 == 0 { x | 0x8000_0000 } else { !x };
    (ordered(a) as i64 - ordered(b) as i64).unsigned_abs()
}

fn describe_f(bits: u32) -> String { format!("{:08x} ({:+})", bits, f32::from_bits(bits)) }

/// All differences of one call: return registers, then every out region byte the function writes.
fn compare(rec: &CallRecord, port: &PortResult) -> Vec<String> {
    let mut bad = Vec::new();
    for (name, rec_v, port_v) in [("v0", rec.ret[0], port.v0), ("v1", rec.ret[1], port.v1), ("f0", rec.ret[2], port.f0)] {
        if let Some(p) = port_v {
            if p != rec_v {
                let ulp = if name == "f0" { format!(", {} ulp", f32_ulp(rec_v, p)) } else { String::new() };
                bad.push(format!("{name}: game {} vs port {}{ulp}", describe_f(rec_v), describe_f(p)));
            }
        }
    }
    for (k, out) in rec.outs.iter().enumerate() {
        let Some((bytes, mask)) = port.outs.get(k) else { bad.push(format!("out {}: the port produced no bytes", out.expr)); continue };
        if bytes.len() != out.bytes.len() { bad.push(format!("out {}: length {} vs the recorded {}", out.expr, bytes.len(), out.bytes.len())); continue; }
        for (i, (&g, &p)) in out.bytes.iter().zip(bytes.iter()).enumerate() {
            if mask[i] && g != p {
                bad.push(format!("out {} byte {i} (float {}): game {:02x} vs port {:02x}", out.expr, i / 4, g, p));
            }
        }
    }
    bad
}

fn inputs(rec: &CallRecord) -> String {
    let mut s = String::new();
    match rec.function.as_str() {
        "PS2Sinf" => s.push_str(&format!("f12 = {}", describe_f(rec.fpr[12]))),
        "Rand_Rand" => {
            s.push_str(&format!("a0 = {:#x}", rec.gpr[4]));
            for g in &rec.ins { s.push_str(&format!(", {} = {:08x}", g.expr, u32_at(&g.bytes, 0))); }
        }
        "Quat_QuatToMat" => {
            if let Some(q) = rec.ins.first() {
                let f = |k: usize| describe_f(u32_at(&q.bytes, 4 * k));
                s.push_str(&format!("quat = (x {}, y {}, z {}, w {})", f(0), f(1), f(2), f(3)));
            }
        }
        _ => s.push_str("see the recording"),
    }
    s
}

fn calls_dir() -> PathBuf { Path::new(&env!("CARGO_MANIFEST_DIR")).join("../../tests/calls") }

fn load_all() -> Vec<(String, u32, String, Vec<CallRecord>)> {
    let mut files: Vec<PathBuf> = fs::read_dir(calls_dir()).expect("tests/calls must exist")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|e| e == "calls").unwrap_or(false))
        .collect();
    files.sort();
    files.into_iter().map(|p| {
        let text = fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()));
        parse_calls(&text).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
    }).collect()
}

#[test]
fn recorded_calls_match_bit_for_bit() {
    let recordings = load_all();
    assert!(recordings.len() >= 3, "expected the committed recordings, found {}", recordings.len());
    let table = adapters();
    for (function, addr, level, calls) in &recordings {
        let Some(adapter) = table.iter().find(|(n, _)| *function == **n).map(|(_, a)| *a) else {
            println!("not covered: {function} ({addr:#x}, {} calls, level {level}) — no adapter yet", calls.len());
            continue;
        };
        let mut bad = Vec::new();
        for rec in calls {
            let port = adapter(rec).unwrap_or_else(|e| panic!("{function} call {}: {e}", rec.n));
            let diffs = compare(rec, &port);
            if !diffs.is_empty() {
                if bad.is_empty() {
                    println!("FIRST DIFFERING CALL of {function}: call {} (frame {}, level {level})", rec.n, rec.frame);
                    println!("  inputs: {}", inputs(rec));
                    for d in &diffs { println!("  {d}"); }
                }
                bad.push(rec.n);
            }
        }
        println!("{function} ({addr:#x}): {} of {} calls match", calls.len() - bad.len(), calls.len());
        assert!(bad.is_empty(), "{function}: {} of {} calls differ (first: call {})", bad.len(), calls.len(), bad[0]);
    }
}

/// Sister: the platform sine is *not* bit-identical to the game's `PS2Sinf` on these recordings — the exact
/// port (`anim::sinf`, used by `placement::object_matrix` and the pose palette) is what matches (G-017).
#[test]
fn the_host_sin_path_is_not_bit_exact() {
    let (_, _, _, calls) = load_all().into_iter().find(|(f, _, _, _)| f == "PS2Sinf").expect("PS2Sinf recording");
    let mut host_diffs = Vec::new();
    for rec in &calls {
        let x = f32::from_bits(rec.fpr[12]);
        let host = x.sin();
        if host.to_bits() != rec.ret[2] { host_diffs.push(rec.n); }
    }
    assert!(!host_diffs.is_empty(),
        "the recordings must distinguish the platform sine from the game's PS2Sinf");
    println!("platform f32 sin differs from the game's PS2Sinf on {} of {} recorded calls (first: call {})",
        host_diffs.len(), calls.len(), host_diffs[0]);
    // The exact port matches every recorded call.
    for rec in &calls {
        let port = ps2sinf(rec).unwrap();
        assert!(compare(rec, &port).is_empty(), "PS2Sinf call {}", rec.n);
    }
}

/// The VU0 guard-bit sum is load-bearing: the same polynomial with the EE FPU's no-guard sums — what a
/// "just use the FPU everywhere" port would do — differs from the game on at least one recorded call.
fn sinf_fpu_sums(mut x: f32) -> f32 {
    use nf_game::ps2math as pm;
    let (pi, tau, half) = (f32::from_bits(0x4049_0fdb), f32::from_bits(0x40c9_0fdb), f32::from_bits(0x3fc9_0fdb));
    if x >= pi { while x >= pi { x = pm::fpu_sub(x, tau); } } else { while x < -pi { x = pm::fpu_add(x, tau); } }
    if x > half { x = pm::fpu_sub(pi, x); } else if x < -half { x = pm::fpu_sub(-pi, x); }
    let (s1, s2, s3, s4) = (f32::from_bits(0xbe2a_aaa4), f32::from_bits(0x3c08_873e), f32::from_bits(0xb94f_b21f), f32::from_bits(0x362e_9c14));
    let x2 = pm::mul(x, x);
    let x4 = pm::mul(x2, x2);
    let x8 = pm::mul(x4, x4);
    let a = pm::mul(pm::mul(x, s3), x2);
    let b = pm::mul(x, s1);
    let c = pm::mul(x, s2);
    let d = pm::mul(x, s4);
    let mut acc = pm::mul(b, x2);
    acc = pm::fpu_add(acc, pm::mul(a, x4));
    acc = pm::fpu_add(acc, pm::mul(c, x4));
    acc = pm::fpu_add(acc, x);
    pm::fpu_add(acc, pm::mul(d, x8))
}

#[test]
fn the_vu0_guard_bit_sum_is_load_bearing() {
    let (_, _, _, calls) = load_all().into_iter().find(|(f, _, _, _)| f == "PS2Sinf").expect("PS2Sinf recording");
    let fpu_bad: Vec<u32> = calls.iter()
        .filter(|r| sinf_fpu_sums(f32::from_bits(r.fpr[12])).to_bits() != r.ret[2])
        .map(|r| r.n).collect();
    assert!(!fpu_bad.is_empty(), "the FPU-sum polynomial must differ from the game on at least one recorded call");
    println!("the FPU-sum polynomial differs from the game on {} of {} calls (first: call {})",
        fpu_bad.len(), calls.len(), fpu_bad[0]);
}

/// Sister: flipping one bit of a recorded output — a return register, a written matrix float — must be caught.
/// Flipping a byte the game function does not write (Quat_QuatToMat's caller-owned floats) is not a failure:
/// the mask exists exactly so the caller's memory is not held against the port.
#[test]
fn a_one_bit_change_in_a_recorded_output_fails() {
    let (_, _, _, calls) = load_all().into_iter().find(|(f, _, _, _)| f == "PS2Sinf").expect("PS2Sinf recording");
    let rec = &calls[0];
    let port = ps2sinf(rec).unwrap();
    assert!(compare(rec, &port).is_empty(), "PS2Sinf call 1 must match before the corruption");
    let mut corrupt = rec.clone();
    corrupt.ret[2] ^= 1; // one bit of the game's f0
    let diffs = compare(&corrupt, &port);
    assert_eq!(diffs.len(), 1, "the flipped f0 bit must be the only difference: {diffs:?}");
    assert!(diffs[0].starts_with("f0: game"), "{}", diffs[0]);

    let (_, _, _, calls) = load_all().into_iter().find(|(f, _, _, _)| f == "Quat_QuatToMat").expect("Quat_QuatToMat recording");
    let rec = &calls[0];
    let port = quat_quat_to_mat(rec).unwrap();
    assert!(compare(rec, &port).is_empty(), "Quat_QuatToMat call 1 must match before the corruption");
    let mut corrupt = rec.clone();
    let written_byte = 5; // inside m[1] (index 1 is written)
    corrupt.outs[0].bytes[written_byte] ^= 0x40;
    let diffs = compare(&corrupt, &port);
    assert!(!diffs.is_empty(), "a one-bit change in a written matrix float must fail");
    assert!(diffs[0].contains("out a1"), "{}", diffs[0]);
    let mut corrupt = rec.clone();
    let unwritten_byte = 0x3c; // inside m[15] (the translation row — caller-owned)
    corrupt.outs[0].bytes[unwritten_byte] ^= 0x40;
    assert!(compare(&corrupt, &port).is_empty(), "a change in caller-owned memory must not fail");

    let (_, _, _, calls) = load_all().into_iter().find(|(f, _, _, _)| f == "Rand_Rand").expect("Rand_Rand recording");
    let rec = &calls[0];
    let port = rand_rand(rec).unwrap();
    assert!(compare(rec, &port).is_empty(), "Rand_Rand call 1 must match before the corruption");
    let mut corrupt = rec.clone();
    corrupt.ret[0] ^= 1; // one bit of the game's v0
    assert!(!compare(&corrupt, &port).is_empty(), "a one-bit change in the returned draw must fail");
}
